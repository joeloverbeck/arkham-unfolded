mod support;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

struct Target {
    root: tempfile::TempDir,
    binary: PathBuf,
    child: Child,
}
impl Target {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let binary = root.path().join("candidate");
        // Link the finished fixture: concurrent fork/exec tests must not inherit
        // another thread's writable descriptor to an executable being staged.
        fs::hard_link(support::fixture().join("child"), &binary).unwrap();
        let child = Command::new(&binary)
            .arg("--fixture-descendant")
            .spawn()
            .unwrap();
        for name in ["sudo", "timeout", "perf", "sleep"] {
            symlink(support::fixture().join("child"), root.path().join(name)).unwrap();
        }
        Self {
            root,
            binary,
            child,
        }
    }
    fn base_command(&self, mode: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
        command
            .args([
                "profile-native",
                "--pid",
                &self.child.id().to_string(),
                "--executable",
            ])
            .arg(&self.binary)
            .arg("--output-dir")
            .arg(self.root.path().join("capture"))
            .env("PATH", self.root.path())
            .env("ARKHAM_FIXTURE_PROFILE", mode)
            .env("ARKHAM_FIXTURE_TRACE", self.root.path().join("trace"))
            .env("ARKHAM_FIXTURE_TARGET_BINARY", &self.binary);
        command
    }
    fn command(&self, mode: &str) -> Command {
        let mut command = self.base_command(mode);
        command.args(["--seconds", "1", "--countdown", "1"]);
        command
    }
}
impl Drop for Target {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn profiling_authenticates_before_countdown_and_uses_bounded_capture_with_private_outputs() {
    let target = Target::new();
    let result = target.command("success").output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output = String::from_utf8(result.stdout).unwrap();
    let interaction = output.find("Interaction:").unwrap();
    let auth = output.find("FIXTURE_AUTH").unwrap();
    let countdown = output.find("recording in 1").unwrap();
    let recording = output.find("RECORDING").unwrap();
    let complete = output.find("Capture complete:").unwrap();
    assert!(
        interaction < auth && auth < countdown && countdown < recording && recording < complete,
        "{output}"
    );
    let capture = target.root.path().join("capture");
    assert_eq!(
        fs::metadata(&capture).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::read(capture.join("perf.data")).unwrap(),
        b"captured samples"
    );
    assert_eq!(
        fs::read_to_string(capture.join("perf.log")).unwrap(),
        "perf statistics\n"
    );
    let trace = fs::read_to_string(target.root.path().join("trace")).unwrap();
    let args: Vec<_> = trace.lines().collect();
    assert_eq!(&args[2..5], ["--signal=INT", "--kill-after=3", "6"]);
    assert_eq!(
        &args[6..12],
        [
            "record",
            "-F",
            "99",
            "-g",
            "-p",
            &target.child.id().to_string()
        ]
    );
    assert_eq!(args.last(), Some(&"1"));
}

#[test]
fn bad_identity_and_existing_output_refuse_before_authentication() {
    let mut target = Target::new();
    let capture = target.root.path().join("capture");
    fs::create_dir(&capture).unwrap();
    fs::write(capture.join("perf.data"), "previous evidence").unwrap();
    let result = target.command("success").output().unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&result.stdout).contains("FIXTURE_AUTH"));
    assert_eq!(
        fs::read_to_string(capture.join("perf.data")).unwrap(),
        "previous evidence"
    );
    target.binary = PathBuf::from("/bin/true");
    let result = target.command("success").output().unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&result.stdout).contains("FIXTURE_AUTH"));
    assert!(String::from_utf8_lossy(&result.stderr).contains("selected executable"));
}

#[test]
fn failed_authentication_or_changed_target_prevents_recording() {
    for mode in ["auth-fail", "replace-before"] {
        let target = Target::new();
        let result = target.command(mode).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        let output = String::from_utf8_lossy(&result.stdout);
        assert!(!output.contains("RECORDING"), "{mode}: {output}");
        assert!(!output.contains("Capture complete:"));
        if mode == "auth-fail" {
            assert!(!output.contains("recording in"));
        }
        assert!(!target.root.path().join("trace").exists());
    }
}

#[test]
fn failed_empty_missing_or_changed_target_capture_never_claims_completion() {
    for mode in ["fail", "empty", "missing", "replace-after"] {
        let target = Target::new();
        let result = target.command(mode).output().unwrap();
        assert_eq!(result.status.code(), Some(1), "{mode}");
        assert!(!String::from_utf8_lossy(&result.stdout).contains("Capture complete:"));
        let capture = target.root.path().join("capture");
        assert!(capture.join("perf.log").is_file());
        if mode == "fail" || mode == "replace-after" {
            assert_eq!(
                fs::read(capture.join("perf.data")).unwrap(),
                b"captured samples"
            );
        }
    }
}

#[test]
fn interrupted_capture_retains_partial_data_and_reports_failure() {
    let target = Target::new();
    let child = target
        .command("hang")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !target.root.path().join("capture/perf.data").exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    // SAFETY: this is the live supervisor spawned by this test.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Capture complete:"));
    assert_eq!(
        fs::read(target.root.path().join("capture/perf.data")).unwrap(),
        b"captured samples"
    );
}

#[test]
fn cli_rejects_unbounded_capture_and_countdown_before_side_effects() {
    let target = Target::new();
    for (flag, value) in [
        ("--seconds", "0"),
        ("--seconds", "61"),
        ("--countdown", "0"),
        ("--countdown", "11"),
    ] {
        let result = target
            .base_command("success")
            .args([flag, value])
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(!target.root.path().join("capture").exists());
    }
}
