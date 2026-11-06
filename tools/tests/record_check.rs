use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};
mod support;

fn record(root: &Path, output: &Path, mode: &str) -> Command {
    record_options(root, output, mode, false)
}

fn record_options(root: &Path, output: &Path, mode: &str, resources: bool) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
    command
        .current_dir(root)
        .args(["record-check", "--output-dir"])
        .arg(output);
    if resources {
        command.arg("--resources");
    }
    command
        .arg("--")
        .arg(support::fixture().join("child"))
        .args(["--fixture-record-check", mode]);
    command
}

fn metadata(output: &Path) -> Value {
    serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap()
}

#[test]
fn source_fingerprints_bind_dirty_and_untracked_inputs_and_invalidate_mutation() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(root.path())
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    git(&["init", "-q"]);
    fs::write(root.path().join("tracked.rs"), "original").unwrap();
    fs::write(root.path().join(".gitignore"), "ignored/\n").unwrap();
    git(&["add", "."]);
    fs::write(root.path().join("tracked.rs"), "dirty before check").unwrap();
    fs::write(root.path().join("untracked.rs"), "new source").unwrap();
    fs::create_dir(root.path().join("ignored")).unwrap();
    for (case, change, code, expected) in [
        ("stable", None, 0, 0),
        ("tracked", Some("tracked.rs"), 0, 1),
        ("new", Some("another.rs"), 0, 1),
        ("untracked", Some("untracked.rs"), 42, 42),
        ("ignored", Some("ignored/build"), 0, 0),
    ] {
        let output = root.path().join("ignored").join(case);
        let mut command = record(root.path(), &output, &code.to_string());
        if let Some(change) = change {
            command.env("ARKHAM_FIXTURE_CHANGE_SOURCE", root.path().join(change));
        }
        let result = command.output().unwrap();
        assert_eq!(result.status.code(), Some(expected), "{case}: {result:?}");
        let data = metadata(&output);
        assert_eq!(data["exit_code"], code, "preserve the child's result");
        assert_eq!(
            data["status"],
            if case == "stable" || case == "ignored" {
                "finished"
            } else {
                "invalidated"
            }
        );
        assert_eq!(
            data["source"]["before_sha256"] == data["source"]["after_sha256"],
            expected == 0
        );
        assert_eq!(data["source"]["before_sha256"].as_str().unwrap().len(), 64);
    }
    // An unignored output directory must not invalidate itself.
    let output = root.path().join("record");
    assert!(
        record(root.path(), &output, "0")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(metadata(&output)["status"], "finished");
}

#[test]
fn nested_dev_check_is_rejected_before_spawning_with_the_correct_command() {
    let root = tempfile::tempdir().unwrap();
    let scripts = root.path().join("scripts");
    fs::create_dir(&scripts).unwrap();
    fs::create_dir(root.path().join("tools")).unwrap();
    for name in ["dev", "check"] {
        fs::copy(support::fixture().join("child"), scripts.join(name)).unwrap();
    }
    for (case, cwd, dev, check) in [
        (
            "relative",
            ".",
            PathBuf::from("scripts/dev"),
            PathBuf::from("scripts/check"),
        ),
        ("absolute", ".", scripts.join("dev"), scripts.join("check")),
        (
            "dot",
            ".",
            PathBuf::from("./scripts/dev"),
            PathBuf::from("./scripts/check"),
        ),
        (
            "from-subdirectory",
            "tools",
            PathBuf::from("../scripts/dev"),
            PathBuf::from("scripts/check"),
        ),
    ] {
        let output = root.path().join(case);
        let trace = root.path().join(format!("{case}.trace"));
        let result = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .current_dir(root.path().join(cwd))
            .args(["record-check", "--output-dir"])
            .arg(&output)
            .arg("--")
            .args([dev, check])
            .env("ARKHAM_FIXTURE_CHECK", "1")
            .env("ARKHAM_FIXTURE_TRACE", &trace)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{case}: {result:?}");
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(
            error.contains("scripts/check already runs scripts/dev"),
            "{error}"
        );
        assert!(error.contains("-- scripts/check"), "{error}");
        assert!(!trace.exists(), "the rejected command must never start");
        assert!(
            !output.exists(),
            "argument rejection must not claim a check run"
        );
    }
}

#[test]
fn check_alone_and_dev_with_another_command_still_run() {
    let root = tempfile::tempdir().unwrap();
    let scripts = root.path().join("scripts");
    fs::create_dir(&scripts).unwrap();
    for name in ["dev", "check"] {
        fs::copy(support::fixture().join("child"), scripts.join(name)).unwrap();
    }
    for (name, command) in [
        ("check", vec!["scripts/check"]),
        ("dev", vec!["scripts/dev", "cargo", "test"]),
    ] {
        let output = root.path().join(name);
        let trace = root.path().join(format!("{name}.trace"));
        let result = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .current_dir(root.path())
            .args(["record-check", "--output-dir"])
            .arg(&output)
            .arg("--")
            .args(command)
            .env("ARKHAM_FIXTURE_CHECK", "1")
            .env("ARKHAM_FIXTURE_TRACE", &trace)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        assert!(trace.exists());
        assert_eq!(metadata(&output)["exit_code"], 0);
    }
}

#[test]
fn optional_resource_capture_keeps_measurements_and_original_failure_status() {
    let root = tempfile::tempdir().unwrap();
    for code in [0, 42] {
        let directory = root.path().join(format!("measured-{code}"));
        let result = record_options(root.path(), &directory, &code.to_string(), true)
            .env("ARKHAM_FIXTURE_DESCENDANT", "1")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(code), "{result:?}");
        let completed = metadata(&directory);
        assert_eq!(completed["status"], "finished");
        assert_eq!(completed["exit_code"], code);
        let descendant = fs::read_to_string(root.path().join("descendant.pid"))
            .unwrap()
            .parse()
            .unwrap();
        assert!(!process_is_alive(descendant));
        assert!(completed["resources"]["error"].is_null());
        assert!(completed["resources"]["finished_unix_ms"].is_u64());
        let samples: Vec<Value> = fs::read_to_string(directory.join("resources.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(samples.len() >= 2, "capture must include start and finish");
        assert_eq!(completed["resources"]["samples"], samples.len());
        for sample in &samples {
            assert!(sample["unix_ms"].is_u64());
            assert!(
                sample["host"]["/proc/meminfo"]["text"]
                    .as_str()
                    .unwrap()
                    .contains("MemAvailable:")
            );
            assert!(
                sample["host"]["/proc/stat"]["text"]
                    .as_str()
                    .unwrap()
                    .starts_with("cpu ")
            );
        }
        assert!(
            samples
                .windows(2)
                .all(|pair| pair[0]["unix_ms"].as_u64() <= pair[1]["unix_ms"].as_u64())
        );
    }
}

#[test]
fn complete_logs_and_metadata_preserve_success_and_original_failure_codes() {
    let root = tempfile::tempdir().unwrap();
    for code in [0, 42, 255] {
        let directory = root.path().join(format!("run-{code}"));
        let result = record(root.path(), &directory, &code.to_string())
            .env("ARKHAM_FIXTURE_RECORD_ENV", "inherited value")
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let log = fs::read_to_string(directory.join("check.log")).unwrap();
        for expected in [
            "RECORD_CHECK_STDOUT",
            "RECORD_CHECK_STDERR",
            "env=inherited value",
            &format!("cwd={}", root.path().display()),
        ] {
            assert!(log.contains(expected), "{log}");
        }
        let completed = metadata(&directory);
        assert!(completed.get("resources").is_none());
        assert!(!directory.join("resources.jsonl").exists());
        assert_eq!(completed["status"], "finished");
        assert_eq!(completed["exit_code"], code);
        assert_eq!(completed["signal"], Value::Null);
        assert_eq!(completed["error"], Value::Null);
        assert_eq!(completed["cwd"], root.path().to_str().unwrap());
        assert_eq!(
            completed["command"],
            json!([
                support::fixture().join("child"),
                "--fixture-record-check",
                code.to_string()
            ])
        );
        assert!(completed["pid"].as_u64().unwrap() > 0);
        let start = completed["started_unix_ms"].as_u64().unwrap();
        let finish = completed["finished_unix_ms"].as_u64().unwrap();
        assert!(start > 0 && finish >= start);
    }
}

#[test]
fn launch_failure_is_incomplete_and_never_reports_a_successful_exit() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("failed");
    let result = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .current_dir(root.path())
        .args(["record-check", "--resources", "--output-dir"])
        .arg(&directory)
        .arg("--")
        .arg(root.path().join("missing-program"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    let failed = metadata(&directory);
    assert_eq!(failed["status"], "incomplete");
    assert!(failed["exit_code"].is_null());
    assert!(failed["pid"].is_null());
    assert!(failed["signal"].is_null());
    assert!(!failed["error"].as_str().unwrap().is_empty());
    assert!(directory.join("check.log").is_file());
}

#[test]
fn output_directory_must_be_fresh_and_its_parent_must_exist() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("existing");
    fs::create_dir(&directory).unwrap();
    for (name, contents) in [("run.json", "prior metadata"), ("check.log", "prior log")] {
        fs::write(directory.join(name), contents).unwrap();
    }
    let refused = record(root.path(), &directory, "0").output().unwrap();
    assert!(!refused.status.success());
    assert_eq!(
        fs::read_to_string(directory.join("run.json")).unwrap(),
        "prior metadata"
    );
    assert_eq!(
        fs::read_to_string(directory.join("check.log")).unwrap(),
        "prior log"
    );
    let absent = root.path().join("missing-parent");
    let refused = record(root.path(), &absent.join("run"), "0")
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(!absent.exists());
}

fn process_is_alive(pid: u64) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        !stat
            .rsplit_once(')')
            .unwrap()
            .1
            .trim_start()
            .starts_with('Z')
    })
}

struct Running {
    child: Child,
    directory: PathBuf,
    pid: u64,
    descendant: Option<u64>,
}

impl Running {
    fn start(root: &Path) -> Self {
        let directory = root.join("run");
        let child = record_options(root, &directory, "wait", true)
            .env("ARKHAM_FIXTURE_DESCENDANT", "1")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut running = Self {
            child,
            directory,
            pid: 0,
            descendant: None,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(bytes) = fs::read(running.directory.join("run.json")) {
                let current: Value =
                    serde_json::from_slice(&bytes).expect("metadata must always be complete JSON");
                if let Some(pid) = current["pid"].as_u64() {
                    running.pid = pid;
                    if fs::read_to_string(running.directory.join("check.log"))
                        .is_ok_and(|log| log.contains("RECORD_CHECK_READY"))
                    {
                        assert_eq!(current["status"], "running");
                        assert!(current["exit_code"].is_null());
                        assert!(current["finished_unix_ms"].is_null());
                        running.descendant = Some(
                            fs::read_to_string(root.join("descendant.pid"))
                                .unwrap()
                                .parse()
                                .unwrap(),
                        );
                        return running;
                    }
                }
            }
            assert!(
                running.child.try_wait().unwrap().is_none(),
                "runner exited before fixture was ready"
            );
            assert!(
                Instant::now() < deadline,
                "runner failed to publish running evidence"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "runner failed to stop");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // SAFETY: clean only the recorded fixture process group created by this test,
        // including after a failed assertion or deliberately killed supervisor.
        let group = self.pid as i32;
        let member = |pid: u64| unsafe { libc::getpgid(pid as i32) } == group;
        if self.pid != 0 && (member(self.pid) || self.descendant.is_some_and(member)) {
            unsafe {
                libc::kill(-(self.pid as i32), libc::SIGKILL);
            }
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn controlled_interruption_records_the_outcome_and_cleans_the_owned_group() {
    for signal in [libc::SIGINT, libc::SIGTERM] {
        let root = tempfile::tempdir().unwrap();
        let mut running = Running::start(root.path());
        let descendant = running.descendant.unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while fs::read_to_string(running.directory.join("resources.jsonl"))
            .unwrap()
            .lines()
            .filter(|line| serde_json::from_str::<Value>(line).is_ok())
            .count()
            < 3
        {
            assert!(
                Instant::now() < deadline,
                "periodic resource sampling did not advance"
            );
            thread::sleep(Duration::from_millis(20));
        }
        // SAFETY: the supervisor is still owned and running in this test.
        assert_eq!(unsafe { libc::kill(running.child.id() as i32, signal) }, 0);
        assert_eq!(running.wait().code(), Some(130));
        let completed = metadata(&running.directory);
        assert_eq!(completed["status"], "interrupted");
        assert!(completed["finished_unix_ms"].is_u64());
        assert!(!process_is_alive(running.pid));
        assert!(!process_is_alive(descendant));
        assert!(completed["resources"]["error"].is_null());
        assert!(completed["resources"]["finished_unix_ms"].is_u64());
        let measurements = fs::read(running.directory.join("resources.jsonl")).unwrap();
        thread::sleep(Duration::from_millis(100));
        assert_eq!(
            fs::read(running.directory.join("resources.jsonl")).unwrap(),
            measurements
        );
    }
}

#[test]
fn a_signaled_child_is_interrupted_and_retains_the_actual_signal() {
    let root = tempfile::tempdir().unwrap();
    let mut running = Running::start(root.path());
    // SAFETY: signal only the fixture child identified by this test's live record.
    assert_eq!(unsafe { libc::kill(running.pid as i32, libc::SIGTERM) }, 0);
    assert!(!running.wait().success());
    let completed = metadata(&running.directory);
    assert_eq!(completed["status"], "interrupted");
    assert_eq!(completed["signal"], libc::SIGTERM);
    assert!(completed["exit_code"].is_null());
    assert!(!process_is_alive(running.descendant.unwrap()));
}

#[test]
fn killed_supervisor_leaves_running_metadata_as_incomplete_evidence() {
    use std::os::unix::process::ExitStatusExt;

    let root = tempfile::tempdir().unwrap();
    let mut running = Running::start(root.path());
    let before = fs::read(running.directory.join("run.json")).unwrap();
    // SAFETY: deliberately kill only the owned supervisor; Running cleans its group.
    assert_eq!(
        unsafe { libc::kill(running.child.id() as i32, libc::SIGKILL) },
        0
    );
    assert_eq!(running.wait().signal(), Some(libc::SIGKILL));
    let after = fs::read(running.directory.join("run.json")).unwrap();
    assert_eq!(after, before);
    let stale: Value = serde_json::from_slice(&after).unwrap();
    assert_eq!(stale["status"], "running");
    assert!(stale["finished_unix_ms"].is_null());
    assert!(stale["exit_code"].is_null());
    assert!(stale["resources"]["finished_unix_ms"].is_null());
    let measurements = fs::read(running.directory.join("resources.jsonl")).unwrap();
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        fs::read(running.directory.join("resources.jsonl")).unwrap(),
        measurements
    );
}
