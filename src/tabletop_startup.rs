//! Native artwork admission and recovery. Session creation follows preflight;
//! the shared render-health gate owns admission after actual asynchronous loading.
use crate::{
    asset_config::AssetConfig,
    assets::CollectionReport,
    tabletop_health::HealthAssets,
    tabletop_support::asset_settings::{self, Collection},
};
use bevy::{app::AppExit, asset::UntypedHandle, prelude::*};
use std::{
    ffi::OsString,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::{Arc, Mutex},
};

#[derive(Clone, Resource)]
pub(crate) struct Recovery {
    root: PathBuf,
    config: AssetConfig,
    collection: Collection,
    failure: Arc<Mutex<Option<String>>>,
}

// Retain *all* required assets, including faces not visible in the opening scene.
#[derive(Resource)]
struct RequiredArtwork {
    _handles: Vec<UntypedHandle>,
}

impl Recovery {
    pub(crate) fn prepare(
        mut config: AssetConfig,
        root: PathBuf,
        collection: Collection,
    ) -> Result<(Self, CollectionReport), String> {
        let mut root = crate::asset_config::resolve_root(root)?;
        let report = require(
            &mut root,
            &mut config,
            collection,
            &mut asset_settings::zenity,
        )?;
        eprintln!(
            "{}\nArtwork root: {}\n{}",
            collection.name(),
            root.display(),
            report.summary()
        );
        Ok((
            Self {
                root,
                config,
                collection,
                failure: Arc::default(),
            },
            report,
        ))
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn config(&self) -> AssetConfig {
        self.config.clone()
    }

    pub(crate) fn install(&self, app: &mut App, report: &CollectionReport) {
        let server = app.world().resource::<AssetServer>();
        let mut handles: Vec<UntypedHandle> = report
            .verified_files()
            .map(|(role, path)| {
                if role == "font/symbols" {
                    server.load::<Font>(path.to_owned()).untyped()
                } else if role == crate::assets::CHAOS_POUCH_ROLE {
                    server
                        .load::<WorldAsset>(crate::tabletop_chaos_pouch::SCENE)
                        .untyped()
                } else {
                    server.load::<Image>(path.to_owned()).untyped()
                }
            })
            .collect();
        if report
            .verified_path(crate::assets::CHAOS_POUCH_ROLE)
            .is_some()
        {
            handles.extend(crate::tabletop_chaos_pouch::material_and_textures(server));
        }
        app.insert_resource(HealthAssets::new(handles.iter().map(UntypedHandle::id)))
            .insert_resource(RequiredArtwork { _handles: handles })
            .insert_resource(self.clone());
    }

    pub(crate) fn refuse(&self, reason: &str) {
        *self.failure.lock().expect("artwork recovery lock") = Some(reason.to_owned());
    }

    /// Winit's event loop is process-owned. Retry replaces this process after
    /// closing its window, preserving the original invocation except its effective
    /// artwork root and reopening a New attempt once its guarded creation succeeded.
    /// Fresh loading and input queues cannot inherit a failed pass.
    pub(crate) fn finish(mut self, exit: AppExit, created_attempt: bool) -> Result<(), String> {
        let reason = self.failure.lock().expect("artwork recovery lock").take();
        if let Some(reason) = reason {
            recover(
                &mut self.root,
                &mut self.config,
                self.collection,
                &reason,
                &mut asset_settings::zenity,
            )?;
            require(
                &mut self.root,
                &mut self.config,
                self.collection,
                &mut asset_settings::zenity,
            )?;
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let error = Command::new(executable)
                .args(retry_arguments(
                    std::env::args_os().skip(1),
                    &self.root,
                    created_attempt,
                ))
                .exec();
            return Err(format!("Could not retry native startup: {error}"));
        }
        crate::tabletop_health::exit_result(exit)
    }
}

fn retry_arguments(
    args: impl Iterator<Item = OsString>,
    root: &Path,
    created_attempt: bool,
) -> Vec<OsString> {
    let mut args = args;
    let mut result = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--art-root" {
            args.next();
        } else if arg == "--attempt" && created_attempt {
            let intent = args.next();
            result.push(arg);
            if let Some(intent) = intent {
                result.push(if intent == "new" {
                    OsString::from("open")
                } else {
                    intent
                });
            }
        } else {
            result.push(arg);
        }
    }
    result.extend([OsString::from("--art-root"), root.as_os_str().to_owned()]);
    result
}

fn require(
    root: &mut PathBuf,
    config: &mut AssetConfig,
    collection: Collection,
    dialog: &mut impl FnMut(&[&str]) -> Result<Output, String>,
) -> Result<CollectionReport, String> {
    loop {
        let report = collection.inspect(root);
        if report.complete && report.expected_count() > 0 {
            return Ok(report);
        }
        let reason = report.startup_failure(root);
        recover(root, config, collection, &reason, dialog)?;
    }
}

fn recover(
    root: &mut PathBuf,
    config: &mut AssetConfig,
    collection: Collection,
    reason: &str,
    dialog: &mut impl FnMut(&[&str]) -> Result<Output, String>,
) -> Result<(), String> {
    let mut outcome = String::new();
    loop {
        let message = format!(
            "{} cannot start play.\nArtwork folder: {}\n\nPrevious startup failure:\n{reason}\n\nChoose a complete prepared {} collection, or repair this folder and Retry. No game decision is committed while blocked.\n{outcome}",
            collection.name(),
            root.display(),
            collection.name()
        );
        eprintln!("{message}");
        let response = dialog(&[
            "--question",
            "--no-markup",
            "--title=Arkham Unfolded — Artwork required",
            "--width=640",
            "--ok-label=Retry",
            "--cancel-label=Quit",
            "--extra-button=Choose asset folder",
            &format!("--text={message}"),
        ])
        .map_err(|error| format!("{message}\nCannot open startup recovery: {error}"))?;
        // Zenity extra buttons return their label and status 1; Quit/close return
        // status 1 without a label. Check the explicit choice before cancellation.
        if response
            .stdout
            .strip_suffix(b"\n")
            .unwrap_or(&response.stdout)
            == b"Choose asset folder"
        {
            match asset_settings::select_folder_with_mode(config, dialog, collection) {
                Ok(Some(path)) => {
                    *root = path;
                    outcome = "Folder saved and selected for this startup. Retry checks it again. A later command-line override still takes precedence.".into();
                }
                Ok(None) => outcome = "Selection canceled. The previous preference and effective folder are unchanged.".into(),
                Err(error) => outcome = format!("Folder not selected: {error}"),
            }
        } else if response.status.success() {
            return Ok(());
        } else if response.status.code() == Some(1) && response.stderr.is_empty() {
            return Err(format!(
                "{} startup canceled; saves are unchanged.",
                collection.name()
            ));
        } else {
            return Err(format!(
                "{message}\nThe startup window could not initialize or complete."
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets;
    use std::{os::unix::process::ExitStatusExt, process::ExitStatus};

    fn response(code: i32, text: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: text.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    #[test]
    fn refused_startup_reports_root_and_offers_recovery_actions() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = AssetConfig::load_at(
            directory.path().join("settings"),
            directory.path().join("prior"),
        );
        let mut root = directory.path().join("missing");
        let expected_root = root.clone();
        let result = require(&mut root, &mut config, Collection::Gathering, &mut |args| {
            let text = args.iter().find(|arg| arg.starts_with("--text=")).unwrap();
            assert!(text.contains(expected_root.to_str().unwrap()));
            assert!(text.contains("prepared"));
            assert!(args.contains(&"--ok-label=Retry"));
            assert!(args.contains(&"--extra-button=Choose asset folder"));
            Ok(response(1, ""))
        });
        assert!(result.is_err());
    }

    #[test]
    fn retry_rechecks_external_repairs_and_never_uses_a_partial_manifest() {
        let directory = tempfile::tempdir().unwrap();
        let mut root = directory.path().join("prepared");
        let repair = root.clone();
        let mut config = AssetConfig::load_at(directory.path().join("settings"), root.clone());
        let mut calls = 0;
        let result = require(&mut root, &mut config, Collection::Gathering, &mut |args| {
            calls += 1;
            if calls == 1 {
                assets::write_gathering_manifest_without_payloads(&repair);
                Ok(response(0, ""))
            } else {
                assert_eq!(calls, 2);
                let text = args.iter().find(|arg| arg.starts_with("--text=")).unwrap();
                assert!(text.contains("structure") || text.contains("affected"));
                Ok(response(1, ""))
            }
        });
        assert!(result.is_err());
        assert_eq!(calls, 2);
        assert!(!config.path().exists());
    }

    #[test]
    fn retry_replaces_only_the_effective_root_and_preserves_the_attempt() {
        let args = [
            "--experiment",
            "gathering",
            "--art-root",
            "/wrong",
            "--save",
            "/isolated/save.json",
        ];
        let actual = retry_arguments(
            args.into_iter().map(OsString::from),
            Path::new("/chosen"),
            false,
        );
        assert_eq!(
            actual,
            [
                "--experiment",
                "gathering",
                "--save",
                "/isolated/save.json",
                "--art-root",
                "/chosen"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn retry_preserves_the_new_path_and_opens_it_only_after_guarded_creation() {
        for created in [false, true] {
            let args = [
                "--experiment",
                "gathering",
                "--attempt",
                "new",
                "--save",
                "/isolated/new.json",
            ];
            let actual = retry_arguments(
                args.into_iter().map(OsString::from),
                Path::new("/repaired"),
                created,
            );
            let expected = [
                "--experiment",
                "gathering",
                "--attempt",
                if created { "open" } else { "new" },
                "--save",
                "/isolated/new.json",
                "--art-root",
                "/repaired",
            ];
            assert_eq!(actual, expected.map(OsString::from));
        }
    }

    #[test]
    #[ignore = "requires the private Gathering collection; dialog process boundary is scripted"]
    fn private_collection_recovers_from_a_bad_override_without_losing_preferences() {
        let collection = Collection::Gathering;
        let variable = "ARKHAM_GATHERING_TEST_ASSETS";
        let chosen = PathBuf::from(std::env::var_os(variable).expect(variable));
        let directory = tempfile::tempdir().unwrap();
        let settings = directory.path().join("settings");
        let prior = directory.path().join("prior");
        let mut config = AssetConfig::load_at(settings.clone(), prior.clone());
        config.save_root(&prior).unwrap();
        let mut root = directory.path().join("bad-cli-override");
        let mut calls = 0;
        let report = require(&mut root, &mut config, collection, &mut |args| {
            calls += 1;
            Ok(match calls {
                1 => response(1, "Choose asset folder\n"),
                2 => {
                    assert!(args.contains(&"--file-selection"));
                    response(0, &format!("{}\n", chosen.display()))
                }
                3 => {
                    assert!(args.contains(&"--ok-label=Save folder"));
                    response(0, "")
                }
                4 => {
                    assert!(
                        args.iter()
                            .any(|arg| arg.contains("Folder saved and selected"))
                    );
                    response(0, "")
                }
                _ => panic!("unexpected recovery dialog"),
            })
        })
        .unwrap();
        assert!(report.complete);
        assert_eq!(calls, 4);
        assert_eq!(root, chosen);
        let saved = std::fs::read(&settings).unwrap();
        let reloaded = AssetConfig::load_at(settings.clone(), prior.clone());
        assert_eq!(reloaded.launch_root(None).unwrap(), chosen);
        assert_eq!(reloaded.launch_root(Some(prior.clone())).unwrap(), prior);

        // Cancel after selecting a valid collection: no preference write.
        let mut calls = 0;
        let canceled = asset_settings::select_folder_with_mode(
            &mut config,
            &mut |_| {
                calls += 1;
                Ok(if calls == 1 {
                    response(0, &format!("{}\n", chosen.display()))
                } else {
                    response(1, "")
                })
            },
            collection,
        )
        .unwrap();
        assert!(canceled.is_none());
        assert_eq!(std::fs::read(&settings).unwrap(), saved);

        // A concurrent settings edit makes the checked write fail without
        // overwriting either the user's new preference or our effective root.
        let mut competing = AssetConfig::load_at(settings.clone(), prior.clone());
        competing.save_root(&prior).unwrap();
        let competing_bytes = std::fs::read(&settings).unwrap();
        let mut calls = 0;
        assert!(
            asset_settings::select_folder_with_mode(
                &mut config,
                &mut |_| {
                    calls += 1;
                    Ok(if calls == 1 {
                        response(0, &format!("{}\n", chosen.display()))
                    } else {
                        response(0, "")
                    })
                },
                collection
            )
            .is_err()
        );
        assert_eq!(root, chosen);
        assert_eq!(std::fs::read(&settings).unwrap(), competing_bytes);
    }
}
