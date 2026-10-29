use std::{fs, process::Command};
mod support;

#[test]
fn ci_selects_the_active_override_before_removing_unused_stable() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::os::unix::fs::symlink(support::fixture().join("child"), root.join("rustup")).unwrap();
    let trace = root.join("trace");
    let run = |mode: &str| {
        Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .arg("prepare-ci-toolchain")
            .env("PATH", root)
            .env("ARKHAM_FIXTURE_RUSTUP", mode)
            .env("ARKHAM_FIXTURE_TRACE", &trace)
            .output()
            .unwrap()
    };
    let result = run("success");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(&trace).unwrap(),
        "show active-toolchain\ndefault 1.97.1-x86_64-unknown-linux-gnu\ntoolchain uninstall stable\n"
    );
    for mode in ["show-fail", "empty", "stable", "default-fail"] {
        fs::write(&trace, "").unwrap();
        assert!(!run(mode).status.success(), "{mode}");
        assert!(
            !fs::read_to_string(&trace).unwrap().contains("uninstall"),
            "{mode}"
        );
    }
}
