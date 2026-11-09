use std::{fs, process::Command};

#[test]
fn public_handoff_preflight_rejects_private_path_forms_without_echoing_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("public.md");
    fs::write(&path, "Candidate abc123; src/attempts.rs; checks passed.\n").unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .arg("check-public-handoff")
            .arg(&path)
            .output()
            .unwrap()
    };
    assert!(run().status.success());
    for private in [
        "/home/private-owner/save.json",
        "~/Pictures/private-art",
        "file:///data/private-save",
        "/tmp/private-run",
        "/run/user/123/private-socket",
        "local/verification/private-run",
        "/Users/private-owner/save",
    ] {
        fs::write(&path, format!("Candidate abc123\nEvidence: {private}\n")).unwrap();
        let result = run();
        assert!(!result.status.success(), "accepted {private}");
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains("line 2"), "{error}");
        assert!(!error.contains("private-owner") && !error.contains("private-run"));
    }
}
