use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn build(source: &Path, output: &Path, reference: &Path, inherited: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .args(["native", "build", "--source"])
        .arg(source)
        .arg("--output-dir")
        .arg(output)
        .arg("--reference")
        .arg(reference)
        .env("CARGO_TARGET_DIR", inherited)
        .env("CARGO_BUILD_BUILD_DIR", inherited.join("intermediates"))
        .output()
        .unwrap()
}

#[test]
fn real_cargo_build_is_isolated_despite_inherited_targets_and_preserves_reference_on_failure() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("Cargo.toml"), "[package]\nname=\"native-diagnostic-fixture\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[features]\ntabletop=[]\n[[bin]]\nname=\"arkham-tabletop\"\npath=\"main.rs\"\n").unwrap();
    fs::write(
        source.join("main.rs"),
        "fn main() { println!(\"isolated diagnostic\"); }\n",
    )
    .unwrap();
    assert!(
        Command::new("cargo")
            .args(["generate-lockfile", "--offline", "--manifest-path"])
            .arg(source.join("Cargo.toml"))
            .output()
            .unwrap()
            .status
            .success()
    );
    let normal = root.path().join("normal-target");
    fs::create_dir_all(normal.join("debug")).unwrap();
    let reference = normal.join("debug/arkham-tabletop");
    fs::write(&reference, "preserved original executable").unwrap();
    let directory = root.path().join("diagnostic");
    let result = build(&source, &directory, &reference, &normal);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    let binary = report["binary"].as_str().unwrap();
    assert_eq!(
        Command::new(binary).output().unwrap().stdout,
        b"isolated diagnostic\n"
    );
    assert_eq!(
        fs::read_to_string(&reference).unwrap(),
        "preserved original executable"
    );
    assert!(!normal.join("intermediates").exists());
    assert_eq!(fs::read_dir(&normal).unwrap().count(), 1);
    assert!(
        !build(&source, &source.join("target/new"), &reference, &normal)
            .status
            .success()
    );
    fs::write(source.join("main.rs"), "this is not valid Rust\n").unwrap();
    let failure = root.path().join("failed-build");
    assert!(
        !build(&source, &failure, &reference, &normal)
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(&reference).unwrap(),
        "preserved original executable"
    );
    assert!(!fs::read(failure.join("build.log")).unwrap().is_empty());
}
