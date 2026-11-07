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
    collection: Collection,
    pending: Option<Mutex<mpsc::Receiver<SelectionResult>>>,
}

#[derive(Clone, Copy)]
pub(crate) enum Collection {
    Gathering,
}

impl Collection {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Gathering => "Gathering",
        }
    }
    pub(crate) fn inspect(self, path: &Path) -> assets::CollectionReport {
        match self {
            Self::Gathering => assets::inspect_gathering(path),
        }
    }
}

struct SelectionResult {
    config: AssetConfig,
    message: String,
}

/// Keeps a simulated external operation connected until the test releases it.
#[cfg(test)]
pub(crate) struct PendingSelection {
    _sender: mpsc::Sender<SelectionResult>,
}

impl AssetSettings {
    pub(crate) fn new(config: AssetConfig) -> Self {
        Self {
            config,
            collection: Collection::Gathering,
            pending: None,
        }
    }

    pub(crate) fn new_gathering(config: AssetConfig) -> Self {
        Self {
            collection: Collection::Gathering,
            ..Self::new(config)
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Hold the external folder-selection seam without opening a desktop dialog.
    #[cfg(test)]
    pub(crate) fn with_pending_selection(mut self) -> (Self, PendingSelection) {
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(Mutex::new(receiver));
        (self, PendingSelection { _sender: sender })
    }

    pub(crate) fn choose(&mut self) {
        if self.is_busy() {
            return;
        }
        let config = self.config.clone();
        let collection = self.collection;
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(Mutex::new(receiver));
        std::thread::spawn(move || {
            let mut config = config;
            let message = match select_folder_with_mode(&mut config, &mut zenity, collection) {
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
    select_folder_with_mode(config, dialog, Collection::Gathering)
}

pub(crate) fn select_folder_with_mode(
    config: &mut AssetConfig,
    dialog: &mut impl FnMut(&[&str]) -> Result<Output, String>,
    collection: Collection,
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
    let report = collection.inspect(&path);
    if !report.complete {
        let message = format!(
            "{}\n{}\nYour setting is unchanged.",
            path.display(),
            report.startup_failure(&path)
        );
        let _ = dialog(&[
            "--error",
            "--no-markup",
            "--title=Arkham Unfolded — Asset folder not recognized",
            &format!("--text={message}"),
        ]);
        return Err(message);
    }
    let status = format!(
        "All {} expected files verified. Images and font must still load before play.",
        report.expected_count()
    );
    let message = format!(
        "{}\n\n{status}\n\nSave this asset folder? Return to startup and Retry, or restart an open game to apply it. Your current game is unchanged.",
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

pub(crate) fn zenity(arguments: &[&str]) -> Result<Output, String> {
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
        assets::write_gathering_manifest_without_payloads(root);
        assert!(assets::inspect_gathering(root).recognized);
        assert!(!assets::inspect_gathering(root).complete);
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
    fn incomplete_collection_is_refused_and_preserves_the_saved_preference() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("prepared");
        incomplete_collection(&root);
        let file = dir.path().join("config.toml");
        let mut config = AssetConfig::load_at(file.clone(), dir.path().join("default"));
        config.save_root(&dir.path().join("prior")).unwrap();
        let before = fs::read(&file).unwrap();
        for confirm in [false, true] {
            let mut round = 0;
            let result = select_folder_with(&mut config, &mut |args| {
                round += 1;
                match round {
                    1 => Ok(response(0, &format!("{}\n", root.display()))),
                    2 => {
                        let message = args.iter().find(|arg| arg.starts_with("--text=")).unwrap();
                        assert!(args.contains(&"--error"));
                        assert!(message.contains("unchanged"));
                        Ok(response(if confirm { 0 } else { 1 }, ""))
                    }
                    _ => panic!("Unexpected extra dialog"),
                }
            });
            assert!(result.is_err());
            assert_eq!(fs::read(&file).unwrap(), before);
        }
    }

    #[test]
    fn gathering_folder_selection_refuses_missing_payloads_and_preserves_settings() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("gathering-art");
        assets::write_gathering_manifest_without_payloads(&root);
        let file = directory.path().join("gathering.toml");
        let prior = directory.path().join("prior");
        let mut config = AssetConfig::load_at(file.clone(), prior.clone());
        config.save_root(&prior).unwrap();
        let before = fs::read(&file).unwrap();
        let mut settings = AssetSettings::new_gathering(config);
        for confirm in [false, true] {
            let mut round = 0;
            let result = select_folder_with_mode(
                &mut settings.config,
                &mut |args| {
                    round += 1;
                    match round {
                        1 => Ok(response(0, &format!("{}\n", root.display()))),
                        2 => {
                            assert!(
                                args.contains(&"--error"),
                                "A manifest without its required payloads cannot be selected"
                            );
                            let message =
                                args.iter().find(|arg| arg.starts_with("--text=")).unwrap();
                            assert!(message.contains("unchanged"));
                            Ok(response(if confirm { 0 } else { 1 }, ""))
                        }
                        _ => panic!("Unexpected additional dialog"),
                    }
                },
                settings.collection,
            );
            assert!(result.is_err());
            assert_eq!(fs::read(&file).unwrap(), before);
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
