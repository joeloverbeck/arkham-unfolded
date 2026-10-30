use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn build(source: &Path, output: &Path, reference: &Path, inherited: &Path) -> Output {
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

fn succeeds(output: Output) -> Output {
    assert!(
        output.status.success(),
        "status: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn build_normal(source: &Path, target: &Path) {
    succeeds(
        Command::new("cargo")
            .args(["build", "--locked", "--offline", "--manifest-path"])
            .arg(source.join("Cargo.toml"))
            .args([
                "--features",
                "tabletop",
                "--bin",
                "arkham-tabletop",
                "--target-dir",
            ])
            .arg(target)
            .env("CARGO_BUILD_BUILD_DIR", target.join("intermediates"))
            .current_dir(source)
            .output()
            .unwrap(),
    );
}

fn build_files(root: &Path) -> BTreeMap<PathBuf, String> {
    let mut files = BTreeMap::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                directories.push(path);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().into(),
                    format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
                );
            }
        }
    }
    files
}

#[test]
fn real_cargo_build_is_isolated_despite_inherited_targets_and_preserves_reference_on_failure() {
    let root = tempfile::tempdir().unwrap();
    let normal_source = root.path().join("normal-source");
    fs::create_dir(&normal_source).unwrap();
    fs::write(
        normal_source.join("Cargo.toml"),
        "[package]\nname=\"native-diagnostic-fixture\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[features]\ntabletop=[]\n[lib]\npath=\"lib.rs\"\n[[bin]]\nname=\"arkham-tabletop\"\npath=\"main.rs\"\nrequired-features=[\"tabletop\"]\n",
    )
    .unwrap();
    fs::write(
        normal_source.join("main.rs"),
        "fn main() { println!(\"{}\", native_diagnostic_fixture::message()); }\n",
    )
    .unwrap();
    fs::write(
        normal_source.join("lib.rs"),
        "pub fn message() -> &'static str { \"normal library\" }\n",
    )
    .unwrap();
    succeeds(
        Command::new("cargo")
            .args(["generate-lockfile", "--offline", "--manifest-path"])
            .arg(normal_source.join("Cargo.toml"))
            .output()
            .unwrap(),
    );
    let normal = root.path().join("normal-target");
    build_normal(&normal_source, &normal);
    let reference = normal.join("debug/arkham-tabletop");
    assert_eq!(
        succeeds(Command::new(&reference).output().unwrap()).stdout,
        b"normal library\n"
    );
    let original = fs::read(&reference).unwrap();
    let artifacts = build_files(&normal);

    // A source copy keeps the package and library identities. Instrumenting its
    // library must not affect the normal executable, even on a later rebuild.
    let source = root.path().join("diagnostic-source");
    fs::create_dir(&source).unwrap();
    for name in ["Cargo.toml", "Cargo.lock", "main.rs", "lib.rs"] {
        fs::copy(normal_source.join(name), source.join(name)).unwrap();
    }
    fs::write(
        source.join("lib.rs"),
        "pub fn message() -> &'static str { \"isolated diagnostic library\" }\n",
    )
    .unwrap();
    let directory = root.path().join("diagnostic");
    let result = succeeds(build(&source, &directory, &reference, &normal));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    let binary = report["binary"].as_str().unwrap();
    assert_eq!(
        succeeds(Command::new(binary).output().unwrap()).stdout,
        b"isolated diagnostic library\n"
    );
    assert!(
        fs::read(&reference).unwrap() == original,
        "the diagnostic build changed the normal executable"
    );
    assert_eq!(build_files(&normal), artifacts);
    build_normal(&normal_source, &normal);
    assert_eq!(
        succeeds(Command::new(&reference).output().unwrap()).stdout,
        b"normal library\n"
    );
    let artifacts = build_files(&normal);
    assert!(
        !build(&source, &source.join("target/new"), &reference, &normal)
            .status
            .success()
    );
    fs::write(source.join("lib.rs"), "this is not valid Rust\n").unwrap();
    let failure = root.path().join("failed-build");
    assert!(
        !build(&source, &failure, &reference, &normal)
            .status
            .success()
    );
    assert!(
        fs::read(&reference).unwrap() == original,
        "the failed diagnostic build changed the normal executable"
    );
    assert_eq!(build_files(&normal), artifacts);
    assert!(!fs::read(failure.join("build.log")).unwrap().is_empty());
}
