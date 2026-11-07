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
            Ok(Some(saved))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot read the remembered attempt: {e}")),
    }
}

pub(crate) fn remember(reference: &Path, save: &Path) -> Result<(), String> {
    let save = std::fs::canonicalize(save).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&save).map_err(|e| e.to_string())?;
    record_replacement::replace(
        reference,
        &bytes,
        65536,
        Error,
        |previous| {
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
    let folder = data.join("arkham-unfolded/attempts");
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
