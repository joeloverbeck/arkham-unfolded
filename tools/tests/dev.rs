use std::{
    ffi::OsString,
    fs,
    os::unix::{fs::symlink, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
mod support;

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            // SAFETY: this test owns the live child and its new process group.
            unsafe {
                libc::kill(-(self.0.id() as i32), libc::SIGKILL);
            }
            let _ = self.0.wait();
        }
    }
}

fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        assert!(Instant::now() < deadline, "missing {}", path.display());
        thread::sleep(Duration::from_millis(10));
    }
}

/// A temporary checkout holding the real launcher, and its canonical root.
fn checkout() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    fs::create_dir(root.join("scripts")).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::copy(repository.join("scripts/dev"), root.join("scripts/dev")).unwrap();
    (directory, root)
}

/// `PATH` with `systemd-run` replaced by the fixture, which records each scope entry.
fn scope_fixture_path(bin: &Path) -> OsString {
    fs::create_dir(bin).unwrap();
    symlink(support::fixture().join("child"), bin.join("systemd-run")).unwrap();
    std::env::join_paths(
        std::iter::once(bin.to_owned())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap()
}

fn locked(root: &Path) -> bool {
    matches!(
        fs::File::open(root).unwrap().try_lock(),
        Err(fs::TryLockError::WouldBlock)
    )
}

#[test]
fn a_nested_call_runs_in_its_checkouts_scope_while_an_independent_call_still_waits() {
    let (_directory, root) = checkout();
    let path = scope_fixture_path(&root.join("bin"));
    let scopes = root.join("scopes");
    let nested = root.join("nested-started");
    let _outer = Running(
        Command::new(root.join("scripts/dev"))
            .arg("scripts/dev")
            .arg(support::fixture().join("child"))
            .args(["--fixture-record-check", "wait"])
            .env("PATH", &path)
            .env_remove("ARKHAM_DEV_CHECKOUT")
            .env("ARKHAM_FIXTURE_STARTED", &nested)
            .env("ARKHAM_FIXTURE_SCOPE_ENTERED", &scopes)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap(),
    );
    // Waiting on the outer call's own lock would take the 30-second timeout.
    wait_for(&nested);
    assert_eq!(
        fs::read_to_string(&scopes).unwrap(),
        format!("{}\n", root.display()),
        "the nested call entered a second scope"
    );
    assert!(locked(&root), "the outer call keeps the checkout lock");

    let started = root.join("independent-started");
    let independent = Command::new(root.join("scripts/dev"))
        .arg(support::fixture().join("child"))
        .args(["--fixture-record-check", "0"])
        .env("PATH", &path)
        .env("LC_ALL", "C")
        .env_remove("ARKHAM_DEV_CHECKOUT")
        .env("ARKHAM_FIXTURE_STARTED", &started)
        .output()
        .unwrap();
    assert_eq!(independent.status.code(), Some(75));
    assert!(!started.exists(), "a timed-out command must not execute");
    let diagnostic = String::from_utf8(independent.stderr).unwrap();
    assert!(
        diagnostic.contains("flock: timeout while waiting to get lock"),
        "{diagnostic}"
    );
}

#[test]
fn a_call_into_another_checkout_takes_that_checkouts_own_scope_and_lock() {
    let (_first, first) = checkout();
    let (_second, second) = checkout();
    let path = scope_fixture_path(&first.join("bin"));
    let scopes = first.join("scopes");
    let started = first.join("started");
    let _run = Running(
        Command::new(first.join("scripts/dev"))
            .arg(second.join("scripts/dev"))
            .arg(support::fixture().join("child"))
            .args(["--fixture-record-check", "wait"])
            .env("PATH", &path)
            .env_remove("ARKHAM_DEV_CHECKOUT")
            .env("ARKHAM_FIXTURE_STARTED", &started)
            .env("ARKHAM_FIXTURE_SCOPE_ENTERED", &scopes)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap(),
    );
    wait_for(&started);
    assert_eq!(
        fs::read_to_string(&scopes).unwrap(),
        format!("{}\n{}\n", first.display(), second.display())
    );
    for root in [&first, &second] {
        assert!(locked(root), "{} is unlocked", root.display());
    }
}

#[test]
fn overlapping_builds_wait_for_the_same_checkout_and_keep_the_command_exit_code() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("scripts")).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::copy(
        repository.join("scripts/dev"),
        root.path().join("scripts/dev"),
    )
    .unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    symlink(support::fixture().join("child"), bin.join("systemd-run")).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let start = |name: &str, mode: &str| {
        Running(
            Command::new(root.path().join("scripts/dev"))
                .args([
                    support::fixture().join("child").as_os_str(),
                    "--fixture-record-check".as_ref(),
                    mode.as_ref(),
                ])
                .env("PATH", &path)
                .env("ARKHAM_FIXTURE_STARTED", root.path().join(name))
                .env(
                    "ARKHAM_FIXTURE_SCOPE_ENTERED",
                    root.path().join(format!("{name}-scope")),
                )
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
                .unwrap(),
        )
    };
    let first = start("first", "wait");
    wait_for(&root.path().join("first"));
    let mut second = start("second", "42");
    wait_for(&root.path().join("second-scope"));
    thread::sleep(Duration::from_millis(150));
    assert!(
        !root.path().join("second").exists(),
        "second build started while first was running"
    );
    assert!(second.0.try_wait().unwrap().is_none());
    drop(first);
    wait_for(&root.path().join("second"));
    assert_eq!(second.0.wait().unwrap().code(), Some(42));
}

#[test]
fn lock_timeout_reports_contention_without_starting_the_command_or_polluting_stdout() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("scripts")).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::copy(
        repository.join("scripts/dev"),
        root.path().join("scripts/dev"),
    )
    .unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    symlink(support::fixture().join("child"), bin.join("systemd-run")).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let started = root.path().join("command-started");
    let lock = fs::File::open(root.path()).unwrap();
    lock.lock().unwrap();
    let run = || {
        Command::new(root.path().join("scripts/dev"))
            .arg(support::fixture().join("child"))
            .args(["--fixture-record-check", "75"])
            .env("PATH", &path)
            .env("LC_ALL", "C")
            .env("ARKHAM_FIXTURE_STARTED", &started)
            .output()
            .unwrap()
    };
    let refused = run();
    assert_eq!(refused.status.code(), Some(75));
    assert!(!started.exists(), "timed-out command must not execute");
    assert!(refused.stdout.is_empty());
    let diagnostic = String::from_utf8(refused.stderr).unwrap();
    assert!(
        diagnostic.contains("flock: timeout while waiting to get lock"),
        "{diagnostic}"
    );

    drop(lock);
    let completed = run();
    assert!(started.exists());
    assert_eq!(
        completed.status.code(),
        Some(75),
        "preserve the child's own exit code"
    );
    assert!(
        String::from_utf8(completed.stdout)
            .unwrap()
            .starts_with("RECORD_CHECK_STDOUT\n")
    );
    let diagnostic = String::from_utf8(completed.stderr).unwrap();
    assert!(diagnostic.contains("RECORD_CHECK_STDERR"));
    assert!(
        !diagnostic.contains("timeout"),
        "a child exit of 75 is not lock contention"
    );
}

#[test]
fn editor_commands_use_the_protected_launcher_and_preserve_cargo_json() {
    let root = tempfile::tempdir().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::create_dir(root.path().join("scripts")).unwrap();
    fs::copy(
        repository.join("scripts/dev"),
        root.path().join("scripts/dev"),
    )
    .unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for name in ["cargo", "systemd-run"] {
        symlink(support::fixture().join("child"), bin.join(name)).unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let settings: serde_json::Value =
        serde_json::from_slice(&fs::read(repository.join(".vscode/settings.json")).unwrap())
            .unwrap();
    for key in [
        "rust-analyzer.check.overrideCommand",
        "rust-analyzer.cargo.buildScripts.overrideCommand",
    ] {
        let argv: Vec<_> = settings[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect();
        let marker = root.path().join("scope");
        let trace = root.path().join("argv");
        let result = Command::new(argv[0])
            .args(&argv[1..])
            .current_dir(root.path())
            .env("PATH", &path)
            .env("ARKHAM_FIXTURE_EDITOR", "1")
            .env("ARKHAM_FIXTURE_SCOPE_ENTERED", &marker)
            .env("ARKHAM_FIXTURE_TRACE", &trace)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        assert!(marker.exists(), "editor command bypassed the scope");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
            serde_json::json!({"reason":"build-finished", "success":true})
        );
        let args = fs::read_to_string(&trace).unwrap();
        for expected in [
            "check",
            "--locked",
            "--workspace",
            "--all-targets",
            "--message-format=json",
        ] {
            assert!(args.lines().any(|arg| arg == expected), "{args}");
        }
        fs::remove_file(marker).unwrap();
    }
}
