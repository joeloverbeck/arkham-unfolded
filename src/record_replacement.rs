//! Locked file-replacement lifecycle. Adapters decide record
//! compatibility and replacement policy; this module owns filesystem sequencing.

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) enum Replacement {
    Unchanged,
    Replace,
    Create,
    Archive,
}

pub(crate) fn previous_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".previous");
    PathBuf::from(name)
}

fn check_size<E>(bytes: &[u8], limit: u64, error: fn(String) -> E) -> Result<(), E> {
    if bytes.len() as u64 > limit {
        return Err(error(
            "This file exceeds the bounded experiment's record limit.".into(),
        ));
    }
    Ok(())
}

pub(crate) fn read<E: From<std::io::Error>>(
    path: &Path,
    limit: u64,
    error: fn(String) -> E,
) -> Result<Vec<u8>, E> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    check_size(&bytes, limit, error)?;
    Ok(bytes)
}

/// Admission and any recovery validation run while the persistent sidecar is
/// locked. Original bytes, the lock, and every staged file stay owned here.
pub(crate) fn replace<E: From<std::io::Error>>(
    path: &Path,
    bytes: &[u8],
    limit: u64,
    error: fn(String) -> E,
    admit: impl FnOnce(Option<&[u8]>) -> Result<Replacement, E>,
    validate_recovery: impl FnOnce(&Path) -> Result<(), E>,
) -> Result<Option<PathBuf>, E> {
    check_size(bytes, limit, error)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    // Never truncate or remove the sidecar: cooperating writers need its inode.
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(PathBuf::from(lock_path))?;
    lock.try_lock()
        .map_err(|cause| error(format!("The save is busy: {cause}")))?;
    let existing = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Some(read(path, limit, error)?),
        Ok(_) => {
            return Err(error(
                "The record destination is not a regular file.".into(),
            ));
        }
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => None,
        Err(cause) => return Err(cause.into()),
    };
    let replacement = admit(existing.as_deref())?;
    if matches!(replacement, Replacement::Unchanged) {
        return Ok(None);
    }
    let previous = previous_path(path);
    if existing.is_some() && previous.try_exists()? {
        validate_recovery(&previous)?;
    }
    // Preserve the existing partial effects: this is not a multi-file
    // transaction. A later failure may leave an archive or rotated recovery.
    let mut next = tempfile::NamedTempFile::new_in(parent)?;
    next.write_all(bytes)?;
    next.as_file().sync_all()?;
    let mut archived_path = None;
    if let Some(existing) = existing {
        let mut backup = tempfile::NamedTempFile::new_in(parent)?;
        backup.write_all(&existing)?;
        backup.as_file().sync_all()?;
        if matches!(replacement, Replacement::Archive) {
            let prefix = format!(
                "{}.archived-",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            let mut archive = tempfile::Builder::new()
                .prefix(&prefix)
                .suffix(".json")
                .tempfile_in(parent)?;
            archive.write_all(&existing)?;
            archive.as_file().sync_all()?;
            let (_, path) = archive.keep().map_err(|cause| E::from(cause.error))?;
            archived_path = Some(path);
        }
        backup
            .persist(&previous)
            .map_err(|cause| E::from(cause.error))?;
    }
    if matches!(replacement, Replacement::Create) {
        next.persist_noclobber(path)
            .map_err(|cause| E::from(cause.error))?;
    } else {
        next.persist(path).map_err(|cause| E::from(cause.error))?;
    }
    Ok(archived_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusive_creation_preserves_a_file_that_appears_after_admission() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("attempt.json");
        let result = replace(
            &path,
            b"new attempt",
            1024,
            std::io::Error::other,
            |previous| {
                assert!(previous.is_none());
                fs::write(&path, b"competing attempt")?;
                Ok(Replacement::Create)
            },
            |_| Ok(()),
        );
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"competing attempt");
        assert!(!previous_path(&path).exists());
    }

    #[test]
    fn oversized_candidate_is_refused_before_creating_directories_or_replacing_bytes() {
        let dir = tempfile::tempdir().unwrap();
        for path in [
            dir.path().join("missing/record"),
            dir.path().join("existing"),
        ] {
            if path.file_name().unwrap() == "existing" {
                fs::write(&path, b"keep").unwrap();
            }
            let result = replace(
                &path,
                b"too large",
                4,
                std::io::Error::other,
                |_| panic!("Oversized candidate reached admission"),
                |_| Ok(()),
            );
            assert_eq!(
                result.unwrap_err().to_string(),
                "This file exceeds the bounded experiment's record limit."
            );
        }
        assert!(!dir.path().join("missing").exists());
        assert_eq!(fs::read(dir.path().join("existing")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
