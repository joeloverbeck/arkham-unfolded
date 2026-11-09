#[path = "support/approval.rs"]
mod approval;
mod support;
use serde_json::Value;
use std::{fs, process::Command};

#[test]
fn normal_installation_records_actual_paths_and_restores_continue_after_timeout() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config");
    let data = root.path().join("data");
    let cwd = root.path().join("working");
    fs::create_dir_all(config.join("arkham-unfolded")).unwrap();
    fs::create_dir_all(&data).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    let reference = config.join("arkham-unfolded/last-gathering.json");
    let original = b"\"/owner/existing-attempt.json\"\n";
    fs::write(&reference, original).unwrap();
    let run = root.path().join("run");
    let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
    command
        .args([
            "native",
            "launch",
            "--installation",
            "normal",
            "--working-directory",
        ])
        .arg(&cwd)
        .arg("--binary")
        .arg(support::fixture().join("child"))
        .arg("--output-dir")
        .arg(&run)
        .args(["--timeout-seconds", "0.5"])
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_DATA_HOME", &data)
        .env("ARKHAM_FIXTURE_NORMAL_ATTEMPT", "1");
    approval::grant(&mut command, root.path());
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(124),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: Value = serde_json::from_slice(&fs::read(run.join("run.json")).unwrap()).unwrap();
    assert_eq!(manifest["cwd"], cwd.to_str().unwrap());
    assert!(
        manifest["argv"]
            .as_array()
            .unwrap()
            .iter()
            .any(|arg| arg == "--verification-attempts")
    );
    assert!(
        manifest["launch_conditions"]["environment"]
            .get("HOME")
            .is_some()
    );
    assert_eq!(
        manifest["environment"]["ARKHAM_VERIFICATION_ATTEMPTS_DIR"],
        run.join("attempts").to_str().unwrap()
    );
    assert_eq!(
        manifest["normal_installation"]["continue_restoration"],
        "restored"
    );
    assert_eq!(fs::read(&reference).unwrap(), original);
    let snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(run.join("attempts/.continue-snapshot.json")).unwrap())
            .unwrap();
    assert_eq!(snapshot["reference"], reference.to_str().unwrap());
    assert_eq!(snapshot["before"], serde_json::json!(original));
    assert_eq!(
        fs::read(run.join("attempts/test.json")).unwrap(),
        b"disposable attempt"
    );
    let log = fs::read_to_string(run.join("app.log")).unwrap();
    assert!(log.contains(&format!("XDG_CONFIG_HOME={}", config.display())));
    assert!(log.contains(&format!("XDG_DATA_HOME={}", data.display())));
    assert_eq!(
        manifest["executable"],
        run.join("arkham-tabletop").to_str().unwrap()
    );
    assert!(
        manifest["timing"]["shutdown_unix_seconds"]
            .as_f64()
            .unwrap()
            < manifest["approval_deadline_unix_seconds"].as_f64().unwrap()
    );
}

#[test]
fn normal_installation_preserves_concurrent_continue_and_refuses_symlink_snapshot() {
    for mode in [
        "concurrent",
        "concurrent-before",
        "absent",
        "symlink",
        "legacy",
    ] {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let data = root.path().join("data");
        let cwd = root.path().join("cwd");
        fs::create_dir_all(config.join("arkham-unfolded")).unwrap();
        fs::create_dir(&data).unwrap();
        fs::create_dir(&cwd).unwrap();
        let reference = config.join("arkham-unfolded/last-gathering.json");
        let owner = root.path().join("owner.json");
        fs::write(&owner, "owner save bytes").unwrap();
        if mode == "symlink" {
            std::os::unix::fs::symlink(&owner, &reference).unwrap();
        } else if mode.starts_with("concurrent") {
            fs::write(&reference, "\"/owner/earlier-attempt.json\"").unwrap();
        }
        let run = root.path().join("run");
        let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
        command
            .args(["native", "launch", "--installation", "normal", "--binary"])
            .arg(support::fixture().join("child"))
            .arg("--output-dir")
            .arg(&run)
            .arg("--working-directory")
            .arg(&cwd)
            .args(["--timeout-seconds", "0.5"])
            .env("XDG_CONFIG_HOME", &config)
            .env("XDG_DATA_HOME", &data)
            .env("ARKHAM_FIXTURE_NORMAL_ATTEMPT", mode);
        approval::grant(&mut command, root.path());
        let output = command.output().unwrap();
        assert_eq!(fs::read(&owner).unwrap(), b"owner save bytes");
        if mode == "legacy" {
            assert_eq!(output.status.code(), Some(2));
            assert!(!run.join("attempts/test.json").exists());
        } else if mode == "symlink" {
            assert_eq!(output.status.code(), Some(1));
            assert!(reference.is_symlink());
            assert!(!run.join("app.log").exists());
        } else {
            assert_eq!(
                output.status.code(),
                Some(124),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let manifest: Value =
                serde_json::from_slice(&fs::read(run.join("run.json")).unwrap()).unwrap();
            if mode == "concurrent-before" {
                assert_eq!(
                    manifest["normal_installation"]["continue_restoration"],
                    "preserved-concurrent-history"
                );
                assert_eq!(
                    fs::read(reference.with_extension("json.previous")).unwrap(),
                    b"\"/owner/newer-attempt.json\""
                );
                assert_eq!(
                    serde_json::from_slice::<std::path::PathBuf>(&fs::read(&reference).unwrap())
                        .unwrap(),
                    run.join("attempts/test.json")
                );
            } else if mode == "concurrent" {
                assert_eq!(
                    fs::read(&reference).unwrap(),
                    b"\"/owner/newer-attempt.json\""
                );
                assert_eq!(
                    manifest["normal_installation"]["continue_restoration"],
                    "preserved-concurrent-change"
                );
            } else {
                assert!(!reference.exists());
            }
        }
    }
}
