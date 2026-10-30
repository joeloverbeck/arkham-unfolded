use std::{fs, process::Command};

#[test]
fn expired_control_deadline_refuses_before_desktop_access_or_evidence_creation() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("expired");
    let output = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .args([
            "native",
            "session",
            "--pid",
            "1",
            "--window",
            "1",
            "--output-dir",
        ])
        .arg(&directory)
        .args(["--deadline-unix-seconds", "1"])
        .env_remove("DISPLAY")
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("Desktop-control deadline has expired"),
        "{error}"
    );
    assert!(fs::symlink_metadata(directory).is_err());
}

#[test]
fn insufficient_control_time_refuses_before_desktop_access() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let soon = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 20)
        .to_string();
    let later = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 600)
        .to_string();
    for flags in [
        vec!["--timeout-seconds", "20"],
        vec!["--deadline-unix-seconds", soon.as_str()],
        vec![
            "--timeout-seconds",
            "20",
            "--deadline-unix-seconds",
            later.as_str(),
        ],
    ] {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("too-short");
        let output = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .args([
                "native",
                "session",
                "--pid",
                "1",
                "--window",
                "1",
                "--output-dir",
            ])
            .arg(&directory)
            .args(flags)
            .env_remove("DISPLAY")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(
            error.contains("Insufficient desktop-control time"),
            "{error}"
        );
        assert!(!directory.exists());
    }
}
