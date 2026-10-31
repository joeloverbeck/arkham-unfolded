//! Characterization at Tony and Party's public continuity boundaries (#62).

use std::{fs, path::Path};

fn archives(path: &Path) -> Vec<std::path::PathBuf> {
    let prefix = format!("{}.archived-", path.file_name().unwrap().to_string_lossy());
    let mut paths: Vec<_> = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&prefix)
        })
        .collect();
    paths.sort();
    paths
}

fn lock_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".lock");
    name.into()
}

fn assert_unlocked(path: &Path) {
    fs::File::open(lock_path(path)).unwrap().try_lock().unwrap();
}

fn assert_no_staging_files(directory: &Path) {
    for entry in fs::read_dir(directory).unwrap() {
        assert!(
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".tmp")
        );
    }
}

macro_rules! continuity_cases {
    ($experiment:ident, $choice:literal, $other_choice:literal, $limit:expr) => {
        mod $experiment {
            use super::*;
            use arkham_unfolded_experiment::$experiment::{Session, continuity::*};

            #[test]
            fn continuation_and_restart_keep_original_bytes_and_durable_archives() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("nested/session.json");
                let mut game = Session::new().unwrap();
                save_session(&path, &game).unwrap();
                assert_eq!(load_session(&path).unwrap(), game);
                // Valid historical formatting must be copied, not regenerated.
                let mut original = fs::read(&path).unwrap();
                original.extend_from_slice(b"\n \t\n");
                fs::write(&path, &original).unwrap();
                commit_choice(&path, &mut game, 0, $choice).unwrap();
                assert_eq!(load_session(&path).unwrap(), game);
                assert_eq!(fs::read(previous_path(&path)).unwrap(), original);
                let old_active = fs::read(&path).unwrap();
                let mut fresh = Session::new().unwrap();
                restart_session(&path, &game, &fresh).unwrap();
                assert_eq!(load_session(&path).unwrap(), fresh);
                assert_eq!(fs::read(previous_path(&path)).unwrap(), old_active);
                let retained = archives(&path);
                assert_eq!(retained.len(), 1);
                assert_eq!(fs::read(&retained[0]).unwrap(), old_active);
                let restarted = fs::read(&path).unwrap();
                commit_choice(&path, &mut fresh, 0, $choice).unwrap();
                assert_eq!(fs::read(previous_path(&path)).unwrap(), restarted);
                assert_eq!(fs::read(&retained[0]).unwrap(), old_active);
                restart_session(&path, &fresh, &Session::new().unwrap()).unwrap();
                assert_eq!(archives(&path).len(), 2);
                assert_eq!(fs::read(&retained[0]).unwrap(), old_active);
                assert_unlocked(&path);
                assert_no_staging_files(path.parent().unwrap());
            }

            #[test]
            fn identical_saves_restarts_and_exports_return_before_recovery_validation() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("session.json");
                let game = Session::new().unwrap();
                save_session(&path, &game).unwrap();
                let mut original = fs::read(&path).unwrap();
                original.extend_from_slice(b"\n \t\n");
                fs::write(&path, &original).unwrap();
                fs::write(previous_path(&path), b"damaged recovery").unwrap();
                save_session(&path, &game).unwrap();
                // Equality precedes even a wrong expected restart session.
                let wrong = Session::with_draws(vec![arkham_unfolded_experiment::TokenFace::Zero]);
                assert_ne!(wrong, game);
                restart_session(&path, &wrong, &game).unwrap();
                assert_eq!(fs::read(&path).unwrap(), original);
                assert_eq!(fs::read(previous_path(&path)).unwrap(), b"damaged recovery");
                assert!(archives(&path).is_empty());
                assert_unlocked(&path);

                let export = directory.path().join("recording.json");
                save_recording(&export, &game.recording()).unwrap();
                let value: serde_json::Value = serde_json::from_slice(&fs::read(&export).unwrap()).unwrap();
                let historical = serde_json::to_vec(&value).unwrap();
                fs::write(&export, &historical).unwrap();
                fs::write(previous_path(&export), b"damaged export recovery").unwrap();
                save_recording(&export, &game.recording()).unwrap();
                assert_eq!(load_recording(&export).unwrap(), game.recording());
                let mut changed = game.clone();
                changed.choose(0, $choice).unwrap();
                assert_eq!(save_recording(&export, &changed.recording()).unwrap_err().to_string(),
                    "A different recording already occupies this path. Recordings are immutable; choose a new path.");
                assert_eq!(fs::read(&export).unwrap(), historical);
                assert_eq!(fs::read(previous_path(&export)).unwrap(), b"damaged export recovery");
                assert_unlocked(&export);
                assert_no_staging_files(directory.path());
            }

            #[test]
            fn damaged_recovery_blocks_replacement_but_is_ignored_for_a_fresh_path() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("session.json");
                fs::write(previous_path(&path), b"damaged recovery").unwrap();
                let mut game = Session::new().unwrap();
                save_session(&path, &game).unwrap();
                assert_eq!(load_session(&path).unwrap(), game);
                let original = fs::read(&path).unwrap();
                let before = game.clone();
                assert!(commit_choice(&path, &mut game, 0, $choice).is_err());
                assert_eq!(game, before);
                assert!(restart_session(&path, &game, &Session::new().unwrap()).is_err());
                assert_eq!(fs::read(&path).unwrap(), original);
                assert_eq!(fs::read(previous_path(&path)).unwrap(), b"damaged recovery");
                assert!(archives(&path).is_empty());
                assert_unlocked(&path);
                assert_no_staging_files(directory.path());
            }

            #[test]
            fn restart_requires_the_expected_active_session_and_a_fresh_candidate() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("nested/session.json");
                let game = Session::new().unwrap();
                let fresh = Session::with_draws(vec![arkham_unfolded_experiment::TokenFace::Zero]);
                assert_ne!(fresh, game);
                assert_eq!(restart_session(&path, &game, &fresh).unwrap_err().to_string(),
                    "The previous turn is no longer at this path. Reopen before starting another turn.");
                assert!(!path.exists());
                assert!(lock_path(&path).exists());
                save_session(&path, &game).unwrap();
                let original = fs::read(&path).unwrap();
                assert_eq!(restart_session(&path, &fresh, &fresh).unwrap_err().to_string(),
                    "A different or later turn is already saved. Reopen it before starting another turn.");
                let mut progressed = game.clone();
                progressed.choose(0, $choice).unwrap();
                assert_eq!(restart_session(&path, &game, &progressed).unwrap_err().to_string(),
                    "Restart requires a fresh example.");
                assert_eq!(fs::read(&path).unwrap(), original);
                assert!(!previous_path(&path).exists());
                assert!(archives(&path).is_empty());
                assert_unlocked(&path);
            }

            #[test]
            fn illegal_choices_and_non_regular_destinations_preserve_custody() {
                let directory = tempfile::tempdir().unwrap();
                let absent = directory.path().join("absent/session.json");
                let mut game = Session::new().unwrap();
                let before = game.clone();
                assert!(commit_choice(&absent, &mut game, 0, "not-an-offered-choice").is_err());
                assert_eq!(game, before);
                assert!(!absent.parent().unwrap().exists());
                let path = directory.path().join("directory");
                fs::create_dir(&path).unwrap();
                fs::write(path.join("owned"), b"keep").unwrap();
                assert_eq!(save_session(&path, &game).unwrap_err().to_string(),
                    "The record destination is not a regular file.");
                assert_eq!(fs::read(path.join("owned")).unwrap(), b"keep");
                #[cfg(unix)]
                {
                    let link = directory.path().join("symlink");
                    std::os::unix::fs::symlink(path.join("owned"), &link).unwrap();
                    assert_eq!(save_session(&link, &game).unwrap_err().to_string(),
                        "The record destination is not a regular file.");
                    assert_eq!(fs::read(&link).unwrap(), b"keep");
                    assert!(fs::symlink_metadata(link).unwrap().is_symlink());
                }
            }

            #[test]
            fn candidate_and_existing_record_limits_preserve_files_and_directory_order() {
                const LIMIT: usize = $limit;
                const ERROR: &str = "This file exceeds the bounded experiment's record limit.";
                let directory = tempfile::tempdir().unwrap();
                let game = Session::new().unwrap();
                let mut oversized = game.recording();
                // A valid observation with oversized presentation text tests the
                // byte limit, independently of the session's bounded rules state.
                oversized.moments[0].history.push("x".repeat(LIMIT));
                let absent = directory.path().join("absent/export.json");
                assert_eq!(save_recording(&absent, &oversized).unwrap_err().to_string(), ERROR);
                assert!(!absent.parent().unwrap().exists());

                let path = directory.path().join("session.json");
                save_session(&path, &game).unwrap();
                let mut bytes = fs::read(&path).unwrap();
                bytes.resize(LIMIT, b' ');
                fs::write(&path, &bytes).unwrap();
                assert_eq!(load_session(&path).unwrap(), game);
                save_session(&path, &game).unwrap();
                assert_eq!(fs::read(&path).unwrap(), bytes);
                bytes.push(b' ');
                fs::write(&path, &bytes).unwrap();
                assert_eq!(load_session(&path).unwrap_err().to_string(), ERROR);
                assert_eq!(save_session(&path, &game).unwrap_err().to_string(), ERROR);
                assert_eq!(fs::read(&path).unwrap(), bytes);
                assert!(!previous_path(&path).exists());
                assert_unlocked(&path);
            }

            #[test]
            fn incompatible_active_envelopes_are_refused_before_recovery_or_replacement() {
                use sha2::{Digest, Sha256};
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("session.json");
                let game = Session::new().unwrap();
                save_session(&path, &game).unwrap();
                let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
                fs::write(previous_path(&path), b"damaged recovery").unwrap();
                for (field, value) in [
                    ("format_version", serde_json::json!(999)),
                    ("kind", serde_json::json!("other-session")),
                    ("source_revision", serde_json::json!("other-source")),
                    ("app_version", serde_json::json!("other-version")),
                ] {
                    let mut invalid = original.clone();
                    invalid[field] = value;
                    let checksum = Sha256::digest(serde_json::to_vec(&serde_json::json!([
                        invalid["format_version"], invalid["kind"], invalid["source_revision"],
                        invalid["app_version"], invalid["payload"]
                    ])).unwrap());
                    invalid["checksum"] = serde_json::json!(format!("{checksum:x}"));
                    let bytes = serde_json::to_vec(&invalid).unwrap();
                    fs::write(&path, &bytes).unwrap();
                    assert_eq!(save_session(&path, &game).unwrap_err().to_string(),
                        "This record requires a different compatible application/source version.");
                    assert_eq!(fs::read(&path).unwrap(), bytes);
                    assert_eq!(fs::read(previous_path(&path)).unwrap(), b"damaged recovery");
                    assert_unlocked(&path);
                }
            }

            #[test]
            fn competing_lock_refuses_the_choice_and_release_rechecks_the_current_record() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("session.json");
                let mut game = Session::new().unwrap();
                save_session(&path, &game).unwrap();
                let original = fs::read(&path).unwrap();
                fs::write(previous_path(&path), &original).unwrap();
                fs::write(lock_path(&path), b"persistent lock contents").unwrap();
                let lock = fs::OpenOptions::new().read(true).write(true).open(lock_path(&path)).unwrap();
                lock.try_lock().unwrap();
                let before = game.clone();
                assert!(commit_choice(&path, &mut game, 0, $choice).unwrap_err().to_string().starts_with("The save is busy: "));
                assert_eq!(game, before);
                assert_eq!(fs::read(&path).unwrap(), original);
                assert_eq!(fs::read(previous_path(&path)).unwrap(), original);
                let mut competing = before.clone();
                competing.choose(0, $other_choice).unwrap();
                let other_path = directory.path().join("other.json");
                save_session(&other_path, &competing).unwrap();
                let then_current = fs::read(&other_path).unwrap();
                // A cooperating holder changes the active bytes before releasing
                // the actual sidecar. The next writer must reread that version.
                fs::write(&path, &then_current).unwrap();
                drop(lock);
                assert_eq!(commit_choice(&path, &mut game, 0, $choice).unwrap_err().to_string(),
                    "A different or later decision is already saved. Reopen it before continuing.");
                assert_eq!(game, before);
                assert_eq!(fs::read(&path).unwrap(), then_current);
                assert_eq!(fs::read(previous_path(&path)).unwrap(), original);
                assert_eq!(fs::read(lock_path(&path)).unwrap(), b"persistent lock contents");
                assert_unlocked(&path);
            }

            #[cfg(unix)]
            #[test]
            fn recovery_install_failure_cleans_staging_and_retains_only_a_restart_archive() {
                use std::{io::Write, os::unix::fs::OpenOptionsExt, time::{Duration, Instant}};
                for restart in [false, true] {
                    let directory = tempfile::tempdir().unwrap();
                    let path = directory.path().join("session.json");
                    let game = Session::new().unwrap();
                    save_session(&path, &game).unwrap();
                    let original = fs::read(&path).unwrap();
                    let previous = previous_path(&path);
                    rustix::fs::mkfifoat(rustix::fs::CWD, &previous, rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR).unwrap();
                    let child_path = path.clone();
                    let before = game.clone();
                    let worker = std::thread::spawn(move || {
                        let mut live = game;
                        let result = if restart {
                            restart_session(&child_path, &live, &Session::new().unwrap())
                        } else {
                            commit_choice(&child_path, &mut live, 0, $choice)
                        };
                        (result, live)
                    });
                    // Opening a nonblocking FIFO writer succeeds only once the
                    // production reader is inside recovery validation. No sleeps
                    // or fake filesystem decide when to inject the persist error.
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let mut writer = loop {
                        match fs::OpenOptions::new().write(true).custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32).open(&previous) {
                            Ok(writer) => break writer,
                            Err(error) if error.raw_os_error() == Some(rustix::io::Errno::NXIO.raw_os_error()) => {
                                assert!(!worker.is_finished(), "writer returned before reading recovery");
                                assert!(Instant::now() < deadline, "recovery reader did not arrive");
                                std::thread::sleep(Duration::from_millis(1));
                            }
                            Err(error) => panic!("open recovery FIFO: {error}"),
                        }
                    };
                    assert!(fs::File::open(lock_path(&path)).unwrap().try_lock().is_err());
                    // The already-open reader gets valid bytes; installation
                    // subsequently encounters a real directory at .previous.
                    fs::remove_file(&previous).unwrap();
                    fs::create_dir(&previous).unwrap();
                    fs::write(previous.join("owned"), b"keep").unwrap();
                    // Switch the writer back to blocking for records larger than
                    // pipe capacity; the production reader drains it concurrently.
                    rustix::fs::fcntl_setfl(&writer, rustix::fs::OFlags::empty()).unwrap();
                    writer.write_all(&original).unwrap();
                    drop(writer);
                    let (result, live) = worker.join().unwrap();
                    assert!(result.is_err());
                    assert_eq!(live, before);
                    assert_eq!(fs::read(&path).unwrap(), original);
                    assert_eq!(fs::read(previous.join("owned")).unwrap(), b"keep");
                    let retained = archives(&path);
                    assert_eq!(retained.len(), usize::from(restart));
                    if restart {
                        assert_eq!(fs::read(&retained[0]).unwrap(), original);
                    }
                    assert_unlocked(&path);
                    assert_no_staging_files(directory.path());
                    fs::remove_dir_all(&previous).unwrap();
                    save_session(&path, &live).unwrap();
                }
            }
        }
    };
}

continuity_cases!(tony, "injury", "resource", 2 * 1024 * 1024);
continuity_cases!(party, "select:tony", "select:patrice", 8 * 1024 * 1024);

#[test]
fn legacy_tony_envelopes_remain_byte_identical_when_recovery_is_damaged() {
    use arkham_unfolded_experiment::tony::continuity::*;
    let directory = tempfile::tempdir().unwrap();
    let session = directory.path().join("session.json");
    let recording = directory.path().join("recording.json");
    let session_bytes = include_bytes!("fixtures/tony-v1/injury.session.json");
    let recording_bytes = include_bytes!("fixtures/tony-v1/injury.recording.json");
    fs::write(&session, session_bytes).unwrap();
    fs::write(&recording, recording_bytes).unwrap();
    for path in [&session, &recording] {
        fs::write(previous_path(path), b"damaged recovery").unwrap();
    }
    save_session(&session, &load_session(&session).unwrap()).unwrap();
    save_recording(&recording, &load_recording(&recording).unwrap()).unwrap();
    assert_eq!(fs::read(&session).unwrap(), session_bytes);
    assert_eq!(fs::read(&recording).unwrap(), recording_bytes);
    for path in [&session, &recording] {
        assert_eq!(fs::read(previous_path(path)).unwrap(), b"damaged recovery");
        assert_unlocked(path);
    }
}
