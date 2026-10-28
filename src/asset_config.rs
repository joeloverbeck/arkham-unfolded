//! Each experiment's persisted presentation preference: its prepared asset folder.

use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    io::Write,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct AssetConfig {
    path: PathBuf,
    default_root: PathBuf,
    saved_root: Option<PathBuf>,
    original: Option<Vec<u8>>,
    warning: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    asset_root: Option<String>,
}

#[derive(Clone, Copy)]
enum Experiment {
    Jacqueline,
    Tony,
}

impl AssetConfig {
    pub fn load() -> Result<Self, String> {
        Self::load_for(Experiment::Jacqueline)
    }

    /// Tony has a separate preference so choosing its collection preserves Jacqueline's.
    pub fn load_tony() -> Result<Self, String> {
        Self::load_for(Experiment::Tony)
    }

    fn load_for(experiment: Experiment) -> Result<Self, String> {
        let (path, root) = locations(
            experiment,
            std::env::var_os("HOME"),
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("XDG_DATA_HOME"),
        )?;
        Ok(Self::load_at(path, root))
    }

    /// Load without writing. Bad configuration remains untouched and uses the default root.
    pub fn load_at(path: PathBuf, default_root: PathBuf) -> Self {
        let mut config = Self {
            path,
            default_root,
            saved_root: None,
            original: None,
            warning: None,
        };
        match read_existing(&config.path) {
            Ok(bytes) => {
                config.original = bytes;
                if let Some(bytes) = &config.original {
                    match parse(bytes) {
                        Ok(root) => config.saved_root = root,
                        Err(error) => config.warning = Some(error),
                    }
                }
            }
            Err(error) => config.warning = Some(error),
        }
        config
    }

    pub fn root(&self) -> &Path {
        self.saved_root.as_deref().unwrap_or(&self.default_root)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    /// The caller checks the collection and obtains the user's choice before saving.
    /// CLI overrides never call this method.
    pub fn save_root(&mut self, root: &Path) -> Result<(), String> {
        if let Some(warning) = &self.warning {
            return Err(format!(
                "Settings were not saved because {} needs attention: {warning}",
                self.path.display()
            ));
        }
        let root_text = root
            .to_str()
            .filter(|_| root.is_absolute())
            .ok_or("Choose an absolute folder path containing valid UTF-8 text.")?;
        if read_existing(&self.path)? != self.original {
            return Err("Settings changed while the app was running. Restart before changing the asset folder; no settings were overwritten.".into());
        }
        let text = toml::to_string(&Settings {
            asset_root: Some(root_text.to_owned()),
        })
        .map_err(|error| format!("Could not encode settings: {error}"))?;
        let parent = self
            .path
            .parent()
            .ok_or("The settings file has no parent directory.")?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|error| format!("Could not create a settings file: {error}"))?;
        temporary
            .write_all(text.as_bytes())
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|error| format!("Could not write settings: {error}"))?;
        // Recheck after preparing the replacement so ordinary concurrent edits are preserved.
        if read_existing(&self.path)? != self.original {
            return Err("Settings changed while saving. No settings were overwritten; restart and try again.".into());
        }
        if self.original.is_none() {
            temporary
                .persist_noclobber(&self.path)
                .map_err(|error| format!("Could not save settings: {}", error.error))?;
        } else {
            temporary
                .persist(&self.path)
                .map_err(|error| format!("Could not replace settings: {}", error.error))?;
        }
        self.saved_root = Some(root.to_path_buf());
        self.original = Some(text.into_bytes());
        Ok(())
    }
}

fn locations(
    experiment: Experiment,
    home: Option<OsString>,
    config_home: Option<OsString>,
    data_home: Option<OsString>,
) -> Result<(PathBuf, PathBuf), String> {
    let home = home.map(PathBuf::from).filter(|path| path.is_absolute());
    let base = |override_path: Option<OsString>, suffix: &str| -> Result<PathBuf, String> {
        override_path
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| home.as_ref().map(|path| path.join(suffix)))
            .ok_or_else(|| "Set an absolute HOME or XDG configuration and data directories.".into())
    };
    let (settings_file, collection_folder) = match experiment {
        Experiment::Jacqueline => ("config.toml", "assets"),
        // Advance this app-owned default when Tony's prepared requirements change.
        // An explicit saved choice always takes precedence and is never migrated here.
        Experiment::Tony => ("tony.toml", "tony-assets-v3"),
    };
    Ok((
        base(config_home, ".config")?
            .join("arkham-unfolded")
            .join(settings_file),
        base(data_home, ".local/share")?
            .join("arkham-unfolded")
            .join(collection_folder),
    ))
}

fn read_existing(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not inspect {}: {error}", path.display())),
        Ok(metadata) if !metadata.file_type().is_file() => Err(format!(
            "{} is not a regular settings file; it will not be replaced.",
            path.display()
        )),
        Ok(_) => fs::read(path)
            .map(Some)
            .map_err(|error| format!("Could not read {}: {error}", path.display())),
    }
}

fn parse(bytes: &[u8]) -> Result<Option<PathBuf>, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "The settings file does not contain valid UTF-8 text.".to_owned())?;
    let settings: Settings =
        toml::from_str(text).map_err(|error| format!("Could not read settings: {error}"))?;
    settings
        .asset_root
        .map(|value| {
            let root = PathBuf::from(value);
            if root.is_absolute() {
                Ok(root)
            } else {
                Err("The saved asset_root must be an absolute folder path.".into())
            }
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_locations_use_absolute_overrides_and_ignore_relative_ones() {
        let home = Some(OsString::from("/home/player"));
        assert_eq!(
            locations(Experiment::Jacqueline, home.clone(), None, None).unwrap(),
            (
                PathBuf::from("/home/player/.config/arkham-unfolded/config.toml"),
                PathBuf::from("/home/player/.local/share/arkham-unfolded/assets")
            )
        );
        assert_eq!(
            locations(
                Experiment::Jacqueline,
                home.clone(),
                Some("relative".into()),
                Some("/disk/data".into())
            )
            .unwrap(),
            (
                PathBuf::from("/home/player/.config/arkham-unfolded/config.toml"),
                PathBuf::from("/disk/data/arkham-unfolded/assets")
            )
        );
        assert_eq!(
            locations(
                Experiment::Tony,
                home,
                Some("/disk/settings".into()),
                Some("relative".into())
            )
            .unwrap(),
            (
                PathBuf::from("/disk/settings/arkham-unfolded/tony.toml"),
                PathBuf::from("/home/player/.local/share/arkham-unfolded/tony-assets-v3")
            )
        );
    }

    #[test]
    fn experiment_preferences_are_independent_and_saved_roots_outrank_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let load = |experiment| {
            let (path, root) = locations(
                experiment,
                Some(dir.path().as_os_str().to_owned()),
                None,
                None,
            )
            .unwrap();
            AssetConfig::load_at(path, root)
        };
        let mut jacqueline = load(Experiment::Jacqueline);
        let jacqueline_root = dir.path().join("chosen-jacqueline");
        jacqueline.save_root(&jacqueline_root).unwrap();
        let legacy_bytes = fs::read(jacqueline.path()).unwrap();

        let mut tony = load(Experiment::Tony);
        assert_eq!(
            tony.root(),
            dir.path()
                .join(".local/share/arkham-unfolded/tony-assets-v3")
        );
        assert!(!tony.path().exists(), "Loading must not create settings");
        let tony_root = dir.path().join("chosen-tony");
        tony.save_root(&tony_root).unwrap();
        assert_eq!(load(Experiment::Tony).root(), tony_root);
        assert_eq!(load(Experiment::Jacqueline).root(), jacqueline_root);
        assert_eq!(fs::read(jacqueline.path()).unwrap(), legacy_bytes);
    }

    #[test]
    fn selected_root_survives_reload_and_escapes_toml_characters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings/config.toml");
        let fallback = dir.path().join("default");
        let selected = dir.path().join("art \"quoted\" \\ folder");
        let mut config = AssetConfig::load_at(path.clone(), fallback.clone());
        assert_eq!(config.root(), fallback);
        config.save_root(&selected).unwrap();
        let reopened = AssetConfig::load_at(path, fallback);
        assert!(reopened.warning().is_none());
        assert_eq!(reopened.root(), selected);
    }

    #[test]
    fn malformed_unknown_or_changed_settings_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let fallback = dir.path().join("default");
        for original in ["asset_root = [", "other = 42", "asset_root = 'relative'"] {
            fs::write(&path, original).unwrap();
            let mut config = AssetConfig::load_at(path.clone(), fallback.clone());
            assert!(config.warning().is_some());
            assert!(config.save_root(&dir.path().join("selected")).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
        }
        fs::write(&path, "asset_root = '/before'").unwrap();
        let mut config = AssetConfig::load_at(path.clone(), fallback);
        fs::write(&path, "asset_root = '/changed'").unwrap();
        assert!(config.save_root(&dir.path().join("selected")).is_err());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "asset_root = '/changed'"
        );
    }

    #[test]
    fn failed_write_keeps_preference_and_existing_data() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not-a-directory");
        fs::write(&blocker, "owner data").unwrap();
        let fallback = dir.path().join("default");
        let mut config = AssetConfig::load_at(blocker.join("config.toml"), fallback.clone());
        assert!(config.save_root(&dir.path().join("selected")).is_err());
        assert_eq!(config.root(), fallback);
        assert_eq!(fs::read_to_string(blocker).unwrap(), "owner data");
    }
}
