mod support;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn launch(binary: &Path, output: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
    command
        .args(["native", "launch", "--binary"])
        .arg(binary)
        .arg("--output-dir")
        .arg(output);
    command
}

fn running(directory: &Path) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(bytes) = fs::read(directory.join("run.json")) {
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            if value["status"] == "running" {
                return value;
            }
        }
        assert!(
            Instant::now() < deadline,
            "no running metadata at {}",
            directory.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn launch_stages_fixed_binary_and_autosaves_only_to_its_isolated_copy() {
    let root = tempfile::tempdir().unwrap();
    let binary = root.path().join("candidate");
    fs::copy(support::fixture().join("child"), &binary).unwrap();
    let original = fs::read(&binary).unwrap();
    let save = root.path().join("owner.session.json");
    fs::write(&save, "owner pending decision").unwrap();
    let directory = root.path().join("run");
    let child = launch(&binary, &directory)
        .args(["--experiment", "tony", "--load"])
        .arg(&save)
        .arg("--art-root")
        .arg(root.path().join("art"))
        .args([
            "--screenshot",
            "render.png",
            "--quit-after-screenshot",
            "--initial-view",
            "scenario",
            "--timeout-seconds",
            "5",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let metadata = running(&directory);
    assert_eq!(metadata["initial_view"], "scenario");
    let process_exe =
        fs::read_link(format!("/proc/{}/exe", metadata["pid"].as_u64().unwrap())).unwrap();
    assert_eq!(process_exe, directory.join("arkham-tabletop"));
    assert_eq!(metadata["executable"], process_exe.to_str().unwrap());
    fs::write(root.path().join("replacement"), "new build bytes").unwrap();
    fs::rename(root.path().join("replacement"), &binary).unwrap();
    fs::write(directory.join("finish"), "").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let completed: Value =
        serde_json::from_slice(&fs::read(directory.join("run.json")).unwrap()).unwrap();
    assert_eq!(completed["status"], "exited");
    assert_eq!(completed["initial_view"], "scenario");
    assert_eq!(
        fs::read(directory.join("arkham-tabletop")).unwrap(),
        original
    );
    assert_eq!(fs::read_to_string(save).unwrap(), "owner pending decision");
    assert_eq!(
        fs::read_to_string(directory.join("session.json")).unwrap(),
        "autosaved copy"
    );
    let log = fs::read_to_string(directory.join("app.log")).unwrap();
    for (name, folder) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
    ] {
        assert!(
            log.contains(&format!("{name}={}", directory.join(folder).display())),
            "{log}"
        );
    }
    assert!(log.ends_with("original executable\n"));
    assert_eq!(
        completed["argv"],
        serde_json::json!([
            directory.join("arkham-tabletop"),
            "--experiment",
            "tony",
            "--load",
            directory.join("session.json"),
            "--art-root",
            root.path().join("art"),
            "--screenshot",
            directory.join("render.png"),
            "--quit-after-screenshot",
            "--initial-view",
            "scenario"
        ])
    );
}

#[test]
fn launch_preserves_existing_paths_and_refuses_escape_or_unbounded_timeout() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("run");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("app.log"), "prior evidence").unwrap();
    let binary = support::fixture().join("child");
    let existing = launch(&binary, &directory).output().unwrap();
    assert_eq!(existing.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(directory.join("app.log")).unwrap(),
        "prior evidence"
    );
    for flags in [
        ["--screenshot", "../escape.png"],
        ["--timeout-seconds", "NaN"],
        ["--timeout-seconds", "0"],
        ["--timeout-seconds", "3601"],
        ["--initial-view", "missing"],
    ] {
        let fresh = root.path().join("fresh");
        let result = launch(&binary, &fresh).args(flags).output().unwrap();
        assert!(!result.status.success());
        assert!(!fresh.exists());
    }
    let symlink = root.path().join("existing-link");
    std::os::unix::fs::symlink(&directory, &symlink).unwrap();
    assert!(!launch(&binary, &symlink).output().unwrap().status.success());
}

#[test]
fn failed_executable_copy_removes_partial_bytes_and_preserves_sources_and_prior_runs() {
    use std::os::unix::process::CommandExt;

    let root = tempfile::tempdir().unwrap();
    let binary = support::fixture().join("child");
    let original = fs::read(&binary).unwrap();
    assert!(original.len() > 64 * 1024);
    let previous = root.path().join("previous.log");
    fs::write(&previous, "prior evidence").unwrap();
    let directory = root.path().join("failed");
    let mut command = launch(&binary, &directory);
    // SAFETY: these async-signal-safe calls affect only the new test subprocess.
    unsafe {
        command.pre_exec(|| {
            libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
            let limit = libc::rlimit {
                rlim_cur: 64 * 1024,
                rlim_max: 64 * 1024,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        fs::read_dir(&directory).unwrap().next().is_none(),
        "a failed staging copy must leave no partial executable or temporary file"
    );
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains(directory.join("arkham-tabletop").to_str().unwrap()),
        "{error}"
    );
    assert!(
        error.contains(&format!("{} bytes", original.len())),
        "{error}"
    );
    assert!(error.contains("File too large"), "{error}");
    assert_eq!(fs::read(binary).unwrap(), original);
    assert_eq!(fs::read_to_string(previous).unwrap(), "prior evidence");
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

#[test]
fn timeout_cleans_resistant_owned_group_and_leaves_unrelated_process_alive() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("run");
    let binary = support::fixture().join("child");
    let mut unrelated = Command::new(&binary)
        .arg("--fixture-descendant")
        .spawn()
        .unwrap();
    let output = launch(&binary, &directory)
        .args(["--timeout-seconds", "0.3", "--no-art"])
        .env("ARKHAM_FIXTURE_IGNORE_TERM", "1")
        .env("ARKHAM_FIXTURE_DESCENDANT", "1")
        .output()
        .unwrap();
    let unrelated_alive = unrelated.try_wait().unwrap().is_none();
    unrelated.kill().unwrap();
    unrelated.wait().unwrap();
    assert!(unrelated_alive);
    assert_eq!(
        output.status.code(),
        Some(124),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let completed: Value =
        serde_json::from_slice(&fs::read(directory.join("run.json")).unwrap()).unwrap();
    assert_eq!(completed["status"], "timed-out");
    assert!(!process_is_alive(completed["pid"].as_u64().unwrap()));
    let descendant: u64 = fs::read_to_string(directory.join("descendant.pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(!process_is_alive(descendant));
    assert_eq!(
        fs::read_to_string(directory.join("session.json")).unwrap(),
        "autosaved copy"
    );
}

#[test]
fn interruption_cleans_owned_child_and_records_honest_status() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("run");
    let child = launch(&support::fixture().join("child"), &directory)
        .args(["--timeout-seconds", "5"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let metadata = running(&directory);
    assert!(metadata.get("initial_view").is_some_and(Value::is_null));
    assert!(
        metadata["argv"]
            .as_array()
            .unwrap()
            .iter()
            .all(|arg| arg != "--initial-view")
    );
    // SAFETY: signal only the supervisor spawned and still owned by this test.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130));
    assert!(!process_is_alive(metadata["pid"].as_u64().unwrap()));
    let completed: Value =
        serde_json::from_slice(&fs::read(directory.join("run.json")).unwrap()).unwrap();
    assert_eq!(completed["status"], "interrupted-or-failed");
}

#[test]
fn timeout_cleans_the_owned_group_after_its_leader_executes_another_program() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("run");
    let child = launch(&support::fixture().join("child"), &directory)
        .args(["--timeout-seconds", "0.5"])
        .env("ARKHAM_FIXTURE_EXEC_HANDOFF", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let metadata = running(&directory);
    let pid = metadata["pid"].as_u64().unwrap();
    let output = child.wait_with_output().unwrap();
    let leaked = process_is_alive(pid);
    if leaked {
        // SAFETY: clean the exact test-owned group even on the expected red run.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    assert_eq!(
        output.status.code(),
        Some(124),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!leaked, "the exec handoff escaped timeout cleanup");
    let completed: Value =
        serde_json::from_slice(&fs::read(directory.join("run.json")).unwrap()).unwrap();
    assert_eq!(completed["status"], "timed-out");
}
