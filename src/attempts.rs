//! The remembered attempt is a convenience reference, never game state.
use crate::{
    asset_config::AssetConfig,
    record_replacement::{self, Replacement},
    saved_state::Error,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) fn reference_path() -> Result<PathBuf, String> {
    Ok(AssetConfig::load_gathering()?
        .path()
        .with_file_name("last-gathering.json"))
}

pub(crate) fn recalled(path: &Path) -> Result<Option<PathBuf>, String> {
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(65537)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 65536 {
                return Err("The remembered attempt reference is too large.".into());
            }
            let saved: PathBuf = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Cannot read the remembered attempt: {e}"))?;
            if !saved.is_absolute() {
                return Err("The remembered attempt needs an absolute path.".into());
            }
            if verification_path(&saved).is_err() {
                return Ok(None);
            }
            Ok(Some(saved))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot read the remembered attempt: {e}")),
    }
}

pub(crate) fn remember(reference: &Path, save: &Path) -> Result<(), String> {
    verification_path(save)?;
    let save = std::fs::canonicalize(save).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&save).map_err(|e| e.to_string())?;
    record_replacement::replace(
        reference,
        &bytes,
        65536,
        Error,
        |previous| {
            admit_verification_reference(reference, previous).map_err(Error)?;
            Ok(if previous == Some(bytes.as_slice()) {
                Replacement::Unchanged
            } else {
                Replacement::Replace
            })
        },
        |_| Ok(()),
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

// Called under the same replacement lock as an ordinary Continue write. A
// diagnostic session must never rotate a newer owner selection out of history.
fn admit_verification_reference(reference: &Path, current: Option<&[u8]>) -> Result<(), String> {
    let Some(directory) = verification_directory()? else {
        return Ok(());
    };
    #[derive(serde::Deserialize)]
    struct Snapshot {
        reference: PathBuf,
        before: Option<Vec<u8>>,
    }
    let path = directory.join(".continue-snapshot.json");
    if !std::fs::symlink_metadata(&path)
        .map_err(|e| format!("Cannot read verification Continue snapshot: {e}"))?
        .is_file()
    {
        return Err("Verification Continue snapshot is not a regular file.".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(524289).read_to_end(&mut bytes))
        .map_err(|e| format!("Cannot read verification Continue snapshot: {e}"))?;
    if bytes.len() > 524288 {
        return Err("Verification Continue snapshot exceeds 512 KiB.".into());
    }
    let snapshot: Snapshot = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Cannot read verification Continue snapshot: {e}"))?;
    if snapshot.reference != reference {
        return Err("Verification Continue snapshot refers to another configuration.".into());
    }
    let owned = current
        .and_then(|bytes| serde_json::from_slice::<PathBuf>(bytes).ok())
        .is_some_and(|path| {
            path.parent() == Some(directory.as_path())
                && std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file())
                && path.canonicalize().is_ok_and(|p| p == path)
        });
    if current != snapshot.before.as_deref() && !owned {
        return Err(
            "Continue changed outside this verification run; preserving that selection.".into(),
        );
    }
    Ok(())
}

pub(crate) fn unused_path() -> Result<PathBuf, String> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .filter(|p| p.is_absolute())
        .ok_or("Cannot find the application data directory.")?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let folder = verification_directory()?.unwrap_or_else(|| data.join("arkham-unfolded/attempts"));
    for index in 0..1000 {
        let path = folder.join(format!(
            "gathering-{stamp}-{}-{index}.json",
            std::process::id()
        ));
        match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(path),
            Err(e) => return Err(e.to_string()),
            Ok(_) => {}
        }
    }
    Err("Cannot choose an unused attempt path.".into())
}

// The native supervisor supplies a fresh directory only for normal-installation
// checks. Artwork configuration remains ordinary; personal attempts stay outside
// the diagnostic route, including Continue and explicitly selected Open paths.
pub(crate) fn verification_directory() -> Result<Option<PathBuf>, String> {
    std::env::var_os("ARKHAM_VERIFICATION_ATTEMPTS_DIR")
        .map(|path| {
            let path = PathBuf::from(path);
            if path.is_absolute()
                && path.is_dir()
                && path.canonicalize().ok().as_ref() == Some(&path)
            {
                Ok(path)
            } else {
                Err("Verification attempts directory is not an existing absolute directory.".into())
            }
        })
        .transpose()
}

pub(crate) fn verification_path(path: &Path) -> Result<(), String> {
    if let Some(directory) = verification_directory()? {
        if path.parent() != Some(directory.as_path())
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("This verification run opens only its disposable attempts.".into());
        }
        if let Ok(metadata) = std::fs::symlink_metadata(path)
            && (!metadata.is_file() || path.canonicalize().ok().as_deref() != Some(path))
        {
            return Err("The disposable attempt is not a regular local file.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_verification_limits_new_continue_and_open_to_disposable_attempts() {
        if std::env::var_os("ARKHAM_ATTEMPTS_TEST_CHILD").is_some() {
            let directory = verification_directory().unwrap().unwrap();
            let owner = directory.parent().unwrap().join("owner.json");
            let reference = directory.parent().unwrap().join("reference.json");
            let bytes = std::fs::read(&owner).unwrap();
            assert!(unused_path().unwrap().starts_with(&directory));
            assert!(recalled(&reference).unwrap().is_none());
            assert!(verification_path(&owner).is_err());
            let save = directory.join("new.json");
            verification_path(&save).unwrap();
            std::fs::write(&save, "disposable").unwrap();
            remember(&reference, &save).unwrap();
            assert_eq!(recalled(&reference).unwrap(), Some(save.clone()));
            // A normal owner write between diagnostic admissions must survive
            // both the next attempt and any later retry of its Remember action.
            let newer_owner = directory.parent().unwrap().join("newer-owner.json");
            std::fs::write(&newer_owner, "newer owner attempt").unwrap();
            let newer = serde_json::to_vec(&newer_owner).unwrap();
            std::fs::write(&reference, &newer).unwrap();
            let prior = reference.with_extension("json.previous");
            std::fs::write(&prior, serde_json::to_vec(&save).unwrap()).unwrap();
            let next = directory.join("next.json");
            std::fs::write(&next, "second disposable").unwrap();
            for _ in 0..2 {
                assert!(
                    remember(&reference, &next)
                        .unwrap_err()
                        .contains("Continue changed")
                );
                assert_eq!(std::fs::read(&reference).unwrap(), newer);
                assert_eq!(
                    serde_json::from_slice::<PathBuf>(&std::fs::read(&prior).unwrap()).unwrap(),
                    save
                );
            }
            let link = directory.join("linked.json");
            std::os::unix::fs::symlink(&owner, &link).unwrap();
            assert!(verification_path(&link).is_err());
            std::fs::remove_file(directory.join(".continue-snapshot.json")).unwrap();
            assert!(
                remember(&reference, &next)
                    .unwrap_err()
                    .contains("snapshot")
            );
            assert_eq!(std::fs::read(&reference).unwrap(), newer);
            assert_eq!(std::fs::read(owner).unwrap(), bytes);
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("attempts");
        std::fs::create_dir(&directory).unwrap();
        let owner = root.path().join("owner.json");
        std::fs::write(&owner, "existing personal attempt").unwrap();
        std::fs::write(
            root.path().join("reference.json"),
            serde_json::to_vec(&owner).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join(".continue-snapshot.json"),
            serde_json::to_vec(&serde_json::json!({
                "reference": root.path().join("reference.json"),
                "before": serde_json::to_vec(&owner).unwrap(),
            }))
            .unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "attempts::tests::normal_verification_limits_new_continue_and_open_to_disposable_attempts", "--nocapture"])
            .env("ARKHAM_ATTEMPTS_TEST_CHILD", "1")
            .env("ARKHAM_VERIFICATION_ATTEMPTS_DIR", directory)
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
