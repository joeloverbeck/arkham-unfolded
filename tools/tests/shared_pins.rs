use std::{fs, path::Path, process::Command};
mod support;

#[test]
fn the_launcher_compiles_the_check_without_cargo_and_names_both_revisions() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for path in [
        "scripts/check-shared-pins",
        "tools/src/bin/check-shared-pins.rs",
        "Cargo.toml",
        "tools/Cargo.toml",
        "rust-toolchain.toml",
    ] {
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::copy(repository.join(path), root.join(path)).unwrap();
    }
    // A stale lockfile stops every locked Cargo command, so the check must not need Cargo.
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    std::os::unix::fs::symlink(support::fixture().join("child"), bin.join("cargo")).unwrap();
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let run = || {
        Command::new("bash")
            .arg(root.join("scripts/check-shared-pins"))
            .env("PATH", &path)
            // The fixture's bootstrap mode, in which a Cargo call fails with CARGO_MUST_NOT_RUN.
            .env("ARKHAM_FIXTURE_RUSTUP", "success")
            .current_dir(root)
            .output()
            .unwrap()
    };

    let tools = fs::read_to_string(root.join("tools/Cargo.toml")).unwrap();
    let revision = tools.split_once("rev = \"").unwrap().1[..40].to_owned();
    let aligned = run();
    let stdout = String::from_utf8_lossy(&aligned.stdout);
    assert!(
        aligned.status.success(),
        "{}",
        String::from_utf8_lossy(&aligned.stderr)
    );
    assert!(stdout.contains(&revision), "{stdout}");

    let other = "3121c60b39744f8cd92e46ce3d634d7674897ebb";
    fs::write(
        root.join("tools/Cargo.toml"),
        tools.replacen(&revision, other, 1),
    )
    .unwrap();
    let drifted = run();
    let stderr = String::from_utf8_lossy(&drifted.stderr);
    assert_eq!(drifted.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("name 2 revisions"), "{stderr}");
    assert!(
        stderr.contains(&format!("  {revision}\n")) && stderr.contains(&format!("  {other}\n")),
        "{stderr}"
    );
    assert!(!stderr.contains("CARGO_MUST_NOT_RUN"), "{stderr}");
}
