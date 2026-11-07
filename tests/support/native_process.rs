use arkham_tools::process::{Completion, OwnedChild};
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
    time::Duration,
};

pub fn isolate(command: &mut Command, directory: &Path) {
    command
        .current_dir(directory)
        .env("XDG_CONFIG_HOME", directory.join("config"))
        .env("XDG_DATA_HOME", directory.join("data"))
        .env("XDG_RUNTIME_DIR", directory.join("no-runtime"))
        .env("GDK_BACKEND", "x11")
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}", directory.join("no-bus").display()),
        )
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET");
}

pub fn capture(command: &mut Command, timeout: Duration) -> (Completion, Output) {
    let logs = tempfile::tempdir().unwrap();
    let stdout = logs.path().join("stdout.log");
    let stderr = logs.path().join("stderr.log");
    command
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout).unwrap())
        .stderr(fs::File::create(&stderr).unwrap());
    let mut child = OwnedChild::spawn(command).unwrap();
    let completion = child.wait(timeout);
    let status = child.cleanup();
    if !matches!(completion, Ok(Completion::Exited(_))) || status.is_err() {
        eprintln!(
            "Native subprocess failed to finish cleanly; logs: {}",
            logs.keep().display()
        );
    }
    let completion = completion.unwrap();
    let status = status.unwrap().unwrap();
    let output = Output {
        status,
        stdout: fs::read(&stdout).unwrap(),
        stderr: fs::read(&stderr).unwrap(),
    };
    (completion, output)
}

pub fn output(command: &mut Command) -> Output {
    let (completion, output) = capture(command, Duration::from_secs(10));
    assert!(
        matches!(completion, Completion::Exited(_)),
        "native subprocess did not finish within 10 seconds\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
