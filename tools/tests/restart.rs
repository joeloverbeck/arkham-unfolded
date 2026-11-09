#[path = "support/approval.rs"]
mod approval;
mod support;
use arkham_tools::process::{Completion, OwnedChild};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn metadata(directory: &Path) -> Value {
    serde_json::from_slice(&fs::read(directory.join("run.json")).unwrap()).unwrap()
}

fn running(directory: &Path) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if directory.join("run.json").is_file() {
            let value = metadata(directory);
            if value["status"] == "running" {
                return value;
            }
        }
        assert!(
            Instant::now() < deadline,
            "no running record: {}",
            directory.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn completed(child: &mut OwnedChild, directory: &Path) {
    fs::write(directory.join("finish"), "").unwrap();
    let result = child.wait(Duration::from_secs(5)).unwrap();
    child.cleanup().unwrap();
    assert!(matches!(result, Completion::Exited(status) if status.success()));
    fs::remove_file(directory.join("finish")).unwrap();
}

struct Initial {
    _root: tempfile::TempDir,
    directory: PathBuf,
    state: PathBuf,
    has_save: bool,
}

impl Initial {
    fn new(mode: &str, has_save: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("original");
        let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
        command
            .args(["native", "launch", "--binary"])
            .arg(support::fixture().join("child"))
            .arg("--output-dir")
            .arg(&directory)
            .args([
                "--entry",
                mode,
                "--timeout-seconds",
                "3",
                "--art-root",
                "/original-art-root",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        approval::grant(&mut command, root.path());
        if !has_save {
            command.env("ARKHAM_FIXTURE_UNSAVED", "1");
        }
        let state = command
            .get_envs()
            .find_map(|(key, value)| {
                (key == "XDG_STATE_HOME").then(|| PathBuf::from(value.unwrap()))
            })
            .unwrap();
        let mut child = OwnedChild::spawn(&mut command).unwrap();
        running(&directory);
        completed(&mut child, &directory);
        fs::write(directory.join("config/chosen-folder"), "persisted-art-root").unwrap();
        if has_save {
            fs::write(directory.join("session.json"), "pending decision").unwrap();
        }
        Self {
            _root: root,
            directory,
            state,
            has_save,
        }
    }

    fn restart(&self, name: &str) -> (Command, PathBuf) {
        let directory = self._root.path().join(name);
        let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
        command
            .args(["native", "restart", "--run"])
            .arg(self.directory.join("run.json"))
            .arg("--output-dir")
            .arg(&directory)
            .env("XDG_STATE_HOME", &self.state)
            .env(
                "ARKHAM_VERIFICATION_ATTEMPTS_DIR",
                self._root.path().join("must-not-reach-app"),
            )
            .env("ARKHAM_FIXTURE_EXPECT_RESTART", "1");
        if !self.has_save {
            command.env("ARKHAM_FIXTURE_UNSAVED", "1");
        }
        (command, directory)
    }
}

#[test]
fn restart_restores_gathering_settings_and_save_with_a_new_supervised_identity() {
    for legacy_presence in [false, true] {
        let mode = "gathering";
        let initial = Initial::new(mode, true);
        if legacy_presence {
            // Existing saves from manifests predating presence metadata still resume.
            let mut legacy = metadata(&initial.directory);
            legacy
                .as_object_mut()
                .unwrap()
                .remove("save_existed_at_start");
            legacy
                .as_object_mut()
                .unwrap()
                .remove("save_exists_at_exit");
            fs::write(
                initial.directory.join("run.json"),
                serde_json::to_vec(&legacy).unwrap(),
            )
            .unwrap();
        }
        let original = fs::read(initial.directory.join("run.json")).unwrap();
        let before = metadata(&initial.directory);
        let (mut command, directory) = initial.restart("restart");
        command
            .args(["--output-format", "summary"])
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        let mut child = OwnedChild::spawn(&mut command).unwrap();
        let restarted = running(&directory);
        assert_ne!(before["pid"], restarted["pid"]);
        for key in [
            "binary",
            "sha256",
            "cwd",
            "environment",
            "save",
            "approval",
            "approval_deadline_unix_seconds",
        ] {
            assert_eq!(before[key], restarted[key], "{mode}/{key}");
        }
        let argv = restarted["argv"].as_array().unwrap();
        assert!(!argv.iter().any(|arg| arg == "--art-root"));
        assert!(argv.iter().any(|arg| arg == "--save"));
        assert_eq!(restarted["entry_target"], "gathering");
        assert_eq!(
            restarted["restart_of"]["run"],
            initial.directory.join("run.json").to_str().unwrap()
        );
        completed(&mut child, &initial.directory);
        let final_record = metadata(&directory);
        assert_eq!(final_record["status"], "exited");
        assert_eq!(final_record["returncode"], 0);
        assert!(
            fs::read_to_string(directory.join("app.log"))
                .unwrap()
                .contains("RESTORED_SETTINGS_AND_SAVE")
        );
        assert_eq!(
            fs::read(initial.directory.join("run.json")).unwrap(),
            original
        );
    }
}

#[test]
fn restart_refuses_changed_binary_and_closed_or_expired_original_approval_before_staging() {
    use std::os::unix::fs::PermissionsExt;
    for fault in ["binary", "closed", "expired", "deadline", "running", "save"] {
        let initial = Initial::new("gathering", true);
        let mut previous = metadata(&initial.directory);
        let approval_dir = initial
            .state
            .join("arkham-unfolded/native-approvals")
            .join(previous["approval"].as_str().unwrap());
        match fault {
            "binary" => {
                let path = initial.directory.join("arkham-tabletop");
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
                fs::write(path, "changed executable").unwrap();
            }
            "closed" => fs::write(approval_dir.join("closed"), "").unwrap(),
            "expired" => {
                let request: Value =
                    serde_json::from_slice(&fs::read(approval_dir.join("request.json")).unwrap())
                        .unwrap();
                let source = Path::new(request["source"].as_str().unwrap());
                let events = fs::read_to_string(source)
                    .unwrap()
                    .lines()
                    .map(|line| {
                        let mut event: Value = serde_json::from_str(line).unwrap();
                        if let Some(time) = event.pointer_mut(
                            "/payload/internal_chat_message_metadata_passthrough/create_time",
                        ) {
                            *time = serde_json::json!(time.as_f64().unwrap() - 600.0);
                        }
                        format!("{event}\n")
                    })
                    .collect::<String>();
                fs::write(source, events).unwrap();
            }
            "deadline" => {
                previous["approval_deadline_unix_seconds"] = serde_json::json!(
                    previous["approval_deadline_unix_seconds"].as_f64().unwrap() + 60.0
                );
                fs::write(
                    initial.directory.join("run.json"),
                    serde_json::to_vec(&previous).unwrap(),
                )
                .unwrap();
            }
            "running" => {
                previous["status"] = serde_json::json!("running");
                fs::write(
                    initial.directory.join("run.json"),
                    serde_json::to_vec(&previous).unwrap(),
                )
                .unwrap();
            }
            "save" => fs::remove_file(initial.directory.join("session.json")).unwrap(),
            _ => unreachable!(),
        }
        let (mut command, directory) = initial.restart("refused");
        let output = command.output().unwrap();
        assert!(!output.status.success(), "{fault}");
        assert!(!directory.exists(), "{fault}: staged before refusal");
        let error = String::from_utf8_lossy(&output.stderr);
        let expected = match fault {
            "binary" => "executable changed",
            "closed" => "approval is closed",
            "expired" => "approval has expired",
            "deadline" => "deadline changed",
            "running" => "completed supervisor",
            "save" => "Recorded isolated save is missing",
            _ => unreachable!(),
        };
        assert!(error.contains(expected), "{fault}: {error}");
    }
}

#[test]
fn restarted_process_retains_timeout_cleanup_and_excludes_concurrent_restarts() {
    let initial = Initial::new("gathering", true);
    let (mut command, directory) = initial.restart("restart");
    command
        .env("ARKHAM_FIXTURE_DESCENDANT", "1")
        .env("ARKHAM_FIXTURE_IGNORE_TERM", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    let mut child = OwnedChild::spawn(&mut command).unwrap();
    running(&directory);
    let (mut concurrent, concurrent_directory) = initial.restart("concurrent");
    let refused = concurrent.output().unwrap();
    assert!(!refused.status.success());
    assert!(!concurrent_directory.exists());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("Another restart owns"));
    let outcome = child.wait(Duration::from_secs(8)).unwrap();
    child.cleanup().unwrap();
    assert!(matches!(outcome, Completion::Exited(status) if status.code() == Some(124)));
    let final_record = metadata(&directory);
    assert_eq!(final_record["status"], "timed-out");
    let descendant = fs::read_to_string(initial.directory.join("descendant.pid")).unwrap();
    for pid in [final_record["pid"].to_string(), descendant] {
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        assert!(
            stat.is_empty()
                || stat
                    .rsplit_once(')')
                    .unwrap()
                    .1
                    .trim_start()
                    .starts_with('Z'),
            "live process: {pid}"
        );
    }
}

#[test]
fn restart_supports_folder_persistence_before_a_game_has_been_saved() {
    for mode in ["selection", "gathering"] {
        let initial = Initial::new(mode, false);
        assert!(!initial.directory.join("session.json").exists());
        let (mut command, directory) = initial.restart("restart");
        command.stdout(Stdio::null()).stderr(Stdio::inherit());
        let mut child = OwnedChild::spawn(&mut command).unwrap();
        let restarted = running(&directory);
        assert_eq!(
            restarted["argv"]
                .as_array()
                .unwrap()
                .iter()
                .any(|arg| arg == "--save"),
            mode == "gathering"
        );
        assert_eq!(restarted["entry_target"], mode);
        completed(&mut child, &initial.directory);
        assert!(!initial.directory.join("session.json").exists());
        assert_eq!(metadata(&directory)["status"], "exited");
        assert_eq!(metadata(&directory)["save_existed_at_start"], false);
        assert_eq!(metadata(&directory)["save_exists_at_exit"], false);
        assert!(
            fs::read_to_string(directory.join("app.log"))
                .unwrap()
                .contains("RESTORED_SETTINGS_AND_SAVE")
        );
    }
}

#[test]
fn historical_or_conflicting_entry_targets_are_refused_without_staging() {
    for fault in ["missing", "retired", "conflicting"] {
        let initial = Initial::new("selection", false);
        let mut previous = metadata(&initial.directory);
        match fault {
            "missing" => {
                previous.as_object_mut().unwrap().remove("entry_target");
            }
            "retired" => previous["entry_target"] = serde_json::json!("jacqueline"),
            "conflicting" => previous["entry_target"] = serde_json::json!("gathering"),
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&previous).unwrap();
        fs::write(initial.directory.join("run.json"), &bytes).unwrap();
        let (mut command, directory) = initial.restart("refused");
        let output = command.output().unwrap();
        assert!(!output.status.success(), "{fault}");
        assert!(!directory.exists(), "{fault}");
        assert!(!initial.directory.join("session.json").exists());
        assert!(!initial.directory.join("restart.lock").exists());
        assert_eq!(fs::read(initial.directory.join("run.json")).unwrap(), bytes);
    }
}

#[test]
fn a_matching_selector_does_not_admit_an_ambiguous_invocation() {
    for mode in ["selection", "gathering"] {
        for extra in [
            vec!["--experiment", "gathering"],
            vec!["--experiment", "jacqueline"],
            vec!["--entry", "selection"],
            vec!["--entry=selection"],
            vec!["--experiment"],
        ] {
            let initial = Initial::new(mode, false);
            let mut previous = metadata(&initial.directory);
            previous["argv"]
                .as_array_mut()
                .unwrap()
                .extend(extra.iter().map(|arg| serde_json::json!(arg)));
            let bytes = serde_json::to_vec(&previous).unwrap();
            fs::write(initial.directory.join("run.json"), &bytes).unwrap();
            let (mut command, directory) = initial.restart("refused");
            let output = command.output().unwrap();
            assert!(!output.status.success(), "{mode}: {extra:?}");
            assert!(!directory.exists(), "{mode}: {extra:?}");
            assert!(!initial.directory.join("restart.lock").exists());
            assert!(!initial.directory.join("session.json").exists());
            assert_eq!(fs::read(initial.directory.join("run.json")).unwrap(), bytes);
        }
    }
}
