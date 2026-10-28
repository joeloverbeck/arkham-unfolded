//! A bounded native folder-selection flow. Existing play keeps its current assets until restart.

use crate::{asset_config::AssetConfig, assets};
use bevy::prelude::Resource;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Mutex, mpsc},
};

#[derive(Resource)]
pub(crate) struct AssetSettings {
    config: AssetConfig,
    tony: bool,
    pending: Option<Mutex<mpsc::Receiver<SelectionResult>>>,
}

struct SelectionResult {
    config: AssetConfig,
    message: String,
}

impl AssetSettings {
    pub(crate) fn new(config: AssetConfig) -> Self {
        Self {
            config,
            tony: false,
            pending: None,
        }
    }

    pub(crate) fn new_tony(config: AssetConfig) -> Self {
        Self {
            tony: true,
            ..Self::new(config)
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn configured_root(&self) -> &Path {
        self.config.root()
    }

    pub(crate) fn choose(&mut self) {
        if self.is_busy() {
            return;
        }
        let config = self.config.clone();
        let tony = self.tony;
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(Mutex::new(receiver));
        std::thread::spawn(move || {
            let mut config = config;
            let message = match select_folder_with_mode(&mut config, &mut zenity, tony) {
                Ok(Some(path)) => format!(
                    "Asset folder saved: {}. Restart the app to use it. A command-line override still applies to its launch.",
                    path.display()
                ),
                Ok(None) => "Asset folder selection canceled. Your setting is unchanged.".into(),
                Err(error) => format!("Asset folder was not saved: {error}"),
            };
            let _ = sender.send(SelectionResult { config, message });
        });
    }

    pub(crate) fn poll(&mut self) -> Option<String> {
        let result = self.pending.as_ref()?.lock().ok()?.try_recv();
        match result {
            Ok(result) => {
                self.pending = None;
                self.config = result.config;
                Some(result.message)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                Some("Asset folder selection stopped before completion. Check the saved setting before retrying.".into())
            }
        }
    }
}

#[cfg(test)]
fn select_folder_with(
    config: &mut AssetConfig,
    dialog: &mut impl FnMut(&[&str]) -> Result<Output, String>,
) -> Result<Option<PathBuf>, String> {
    select_folder_with_mode(config, dialog, false)
}

fn select_folder_with_mode(
    config: &mut AssetConfig,
    dialog: &mut impl FnMut(&[&str]) -> Result<Output, String>,
    tony: bool,
) -> Result<Option<PathBuf>, String> {
    if let Some(warning) = config.warning() {
        return Err(format!(
            "Check {} before changing this setting: {warning}",
            config.path().display()
        ));
    }
    let initial = format!(
        "{}/",
        config
            .root()
            .ancestors()
            .find(|path| path.is_dir())
            .unwrap_or(Path::new("/"))
            .display()
    );
    let output = dialog(&[
        "--file-selection",
        "--directory",
        "--title=Arkham Unfolded — Choose prepared assets",
        &format!("--filename={initial}"),
    ])?;
    if canceled(&output)? {
        return Ok(None);
    }
    let selection = String::from_utf8(output.stdout)
        .map_err(|_| "The chosen path is not valid UTF-8 text.".to_owned())?;
    let path = PathBuf::from(selection.strip_suffix('\n').unwrap_or(&selection));
    if !path.is_absolute() || !path.is_dir() {
        return Err("Choose an existing folder using an absolute path.".into());
    }
    let report = if tony {
        assets::inspect_tony(&path)
    } else {
        assets::inspect(&path)
    };
    if !report.recognized {
        let message = "This folder is not a recognized prepared asset collection. Choose the folder containing the prepared collection's manifest. Your setting is unchanged.";
        let _ = dialog(&[
            "--error",
            "--no-markup",
            "--title=Arkham Unfolded — Asset folder not recognized",
            &format!("--text={message}"),
        ]);
        return Err(message.into());
    }
    let status = if report.complete {
        format!("All {} expected assets are ready.", report.expected_count())
    } else {
        format!(
            "{} of {} expected assets are ready. Missing or changed artwork will use readable substitutes. This folder is incomplete.",
            report.valid_count(),
            report.expected_count()
        )
    };
    let message = format!(
        "{}\n\n{status}\n\nSave this asset folder for the next launch? Restart the app to apply the change. Your current game is unchanged.",
        path.display()
    );
    let confirmation = dialog(&[
        "--question",
        "--no-markup",
        "--title=Arkham Unfolded — Check asset folder",
        "--ok-label=Save folder",
        "--cancel-label=Cancel",
        "--width=520",
        &format!("--text={message}"),
    ])?;
    if canceled(&confirmation)? {
        return Ok(None);
    }
    config.save_root(&path)?;
    Ok(Some(path))
}

fn zenity(arguments: &[&str]) -> Result<Output, String> {
    Command::new("zenity")
        .args(arguments)
        .output()
        .map_err(|error| {
            format!(
                "Could not open the folder dialog ({error}). Install the Ubuntu zenity package or set asset_root in the settings file."
            )
        })
}

fn canceled(output: &Output) -> Result<bool, String> {
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err("The native folder dialog could not complete. Your setting is unchanged.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::process::ExitStatusExt, process::ExitStatus};

    fn response(code: i32, text: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: text.as_bytes().to_vec(),
            stderr: vec![],
        }
    }

    fn incomplete_collection(root: &Path) {
        fs::create_dir_all(root).unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../resources/asset-selection.json")).unwrap();
        manifest
            .as_object_mut()
            .unwrap()
            .retain(|key, _| ["format_version", "collection", "assets"].contains(&key.as_str()));
        for asset in manifest["assets"].as_array_mut().unwrap() {
            let entry = asset.as_object_mut().unwrap();
            entry.retain(|key, _| ["role", "path", "sha256", "bytes"].contains(&key.as_str()));
        }
        fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(assets::inspect(root).recognized);
        assert!(!assets::inspect(root).complete);
    }

    #[test]
    fn canceling_chooser_preserves_saved_setting() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let mut config = AssetConfig::load_at(file.clone(), dir.path().join("default"));
        config.save_root(&dir.path().join("prior")).unwrap();
        let before = fs::read(&file).unwrap();
        assert!(
            select_folder_with(&mut config, &mut |_| Ok(response(1, "")))
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read(file).unwrap(), before);
    }

    #[test]
    fn incomplete_collection_warns_and_saves_only_after_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("prepared");
        incomplete_collection(&root);
        let file = dir.path().join("config.toml");
        let mut config = AssetConfig::load_at(file.clone(), dir.path().join("default"));
        for confirm in [false, true] {
            let mut round = 0;
            let result = select_folder_with(&mut config, &mut |args| {
                round += 1;
                match round {
                    1 => Ok(response(0, &format!("{}\n", root.display()))),
                    2 => {
                        let message = args.iter().find(|arg| arg.starts_with("--text=")).unwrap();
                        assert!(message.contains("This folder is incomplete"));
                        assert!(message.contains("readable substitutes"));
                        assert!(message.contains("Restart"));
                        Ok(response(if confirm { 0 } else { 1 }, ""))
                    }
                    _ => panic!("Unexpected extra dialog"),
                }
            })
            .unwrap();
            if confirm {
                assert_eq!(result, Some(root.clone()));
                assert_eq!(
                    AssetConfig::load_at(file.clone(), dir.path().into()).root(),
                    root
                );
            } else {
                assert!(result.is_none());
                assert!(!file.exists());
            }
        }
    }

    #[test]
    fn unrecognized_folder_is_rejected_without_saving() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let mut config = AssetConfig::load_at(file.clone(), dir.path().join("default"));
        let mut round = 0;
        let result = select_folder_with(&mut config, &mut |args| {
            round += 1;
            if round == 1 {
                Ok(response(0, &format!("{}\n", dir.path().display())))
            } else {
                assert!(args.contains(&"--error"));
                Ok(response(0, ""))
            }
        });
        assert!(result.is_err());
        assert!(!file.exists());
    }

    #[test]
    fn unavailable_dialog_does_not_change_the_setting() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let original = dir.path().join("original");
        let mut config = AssetConfig::load_at(file.clone(), original.clone());
        assert!(select_folder_with(&mut config, &mut |_| Err("no display".into())).is_err());
        assert_eq!(config.root(), original);
        assert!(!file.exists());
    }
}
