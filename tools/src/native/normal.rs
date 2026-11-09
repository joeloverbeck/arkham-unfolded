//! Normal artwork settings with disposable attempts and conditional Continue repair.
use super::*;

pub(super) fn prepare(directory: &Path) -> Result<Value> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .filter(|p| p.is_absolute())
        .context("Cannot find normal configuration directory")?;
    let reference = config.join("arkham-unfolded/last-gathering.json");
    fs::create_dir_all(reference.parent().unwrap())?;
    let _lock = lock(&reference)?;
    let before = read(&reference)?;
    let previous = read(&reference.with_extension("json.previous"))?;
    let attempts = directory.join("attempts");
    fs::create_dir(&attempts)?;
    fs::write(
        attempts.join(".continue-snapshot.json"),
        serde_json::to_vec(&json!({"reference": reference, "before": before}))?,
    )?;
    Ok(
        json!({"reference": reference, "before": before, "previous_before": previous,
        "attempts": attempts, "continue_restoration": "pending"}),
    )
}

fn lock(reference: &Path) -> Result<File> {
    let path = reference.with_extension("json.lock");
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        ensure!(metadata.is_file(), "Continue lock is not a regular file");
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.try_lock()
        .context("Continue is being updated by another process")?;
    Ok(file)
}

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(metadata.is_file(), "Continue record is not a regular file");
            let mut bytes = Vec::new();
            File::open(path)?.take(65537).read_to_end(&mut bytes)?;
            ensure!(bytes.len() <= 65536, "Continue record exceeds 64 KiB");
            Ok(Some(bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write(path: &Path, bytes: Option<&[u8]>) -> Result<()> {
    if let Some(bytes) = bytes {
        let mut next = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        next.write_all(bytes)?;
        next.as_file().sync_all()?;
        next.persist(path)?;
    } else if path.try_exists()? {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn owned(bytes: &[u8], attempts: &Path) -> bool {
    let Ok(path) = serde_json::from_slice::<PathBuf>(bytes) else {
        return false;
    };
    path.parent() == Some(attempts)
        && fs::symlink_metadata(&path).is_ok_and(|m| m.is_file())
        && path.canonicalize().is_ok_and(|p| p == path)
}

pub(super) fn restore(record: &mut Value) -> Result<()> {
    let reference = PathBuf::from(
        record["reference"]
            .as_str()
            .context("Missing Continue reference")?,
    );
    let attempts = PathBuf::from(
        record["attempts"]
            .as_str()
            .context("Missing disposable attempts")?,
    );
    let before: Option<Vec<u8>> = serde_json::from_value(record["before"].clone())?;
    let previous_before: Option<Vec<u8>> =
        serde_json::from_value(record["previous_before"].clone())?;
    let _lock = lock(&reference)?;
    let current = read(&reference)?;
    record["continue_restoration"] = json!(if current == before {
        "unchanged"
    } else if current
        .as_deref()
        .is_some_and(|bytes| owned(bytes, &attempts))
    {
        let previous_path = reference.with_extension("json.previous");
        let previous = read(&previous_path)?;
        // A foreign predecessor means the prelaunch snapshot is stale. Preserve
        // both current and previous records for inspection; do not label an old
        // selection as restored merely because this run wrote most recently.
        if previous != before
            && !previous
                .as_deref()
                .is_some_and(|bytes| owned(bytes, &attempts))
        {
            record["continue_restoration"] = json!("preserved-concurrent-history");
            return Ok(());
        }
        write(&previous_path, previous_before.as_deref())?;
        write(&reference, before.as_deref())?;
        "restored"
    } else {
        "preserved-concurrent-change"
    });
    Ok(())
}
