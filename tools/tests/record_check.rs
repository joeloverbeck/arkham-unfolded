//! Arkham's record-check wiring around its own `scripts/dev` and `scripts/check`.
//! The recorder's behaviour is held in unfolded-tabletop's `dev-tools/tests/cli`.

use serde_json::Value;
use std::{fs, path::Path, process::Command};
mod support;

fn metadata(output: &Path) -> Value {
    serde_json::from_slice(&fs::read(output.join("run.json")).unwrap()).unwrap()
}

#[test]
fn check_and_dev_run_alone_or_nested() {
    let root = tempfile::tempdir().unwrap();
    let scripts = root.path().join("scripts");
    fs::create_dir(&scripts).unwrap();
    for name in ["dev", "check"] {
        fs::copy(support::fixture().join("child"), scripts.join(name)).unwrap();
    }
    // scripts/dev runs a call nested in the same checkout directly (tests/dev.rs).
    for (name, command) in [
        ("check", vec!["scripts/check"]),
        ("dev", vec!["scripts/dev", "cargo", "test"]),
        ("dev-check", vec!["scripts/dev", "scripts/check"]),
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
