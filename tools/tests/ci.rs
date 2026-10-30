use std::{fs, path::Path, process::Command};
mod support;

#[test]
fn ci_selects_the_active_override_before_removing_unused_stable() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::os::unix::fs::symlink(support::fixture().join("child"), root.join("rustup")).unwrap();
    // A Cargo invocation during bootstrap must fail even if Cargo is installed.
    std::os::unix::fs::symlink(support::fixture().join("child"), root.join("cargo")).unwrap();
    let compiler = Command::new("rustup")
        .args(["which", "rustc"])
        .output()
        .unwrap();
    assert!(compiler.status.success());
    let compiler = String::from_utf8(compiler.stdout).unwrap();
    std::os::unix::fs::symlink(compiler.trim(), root.join("rustc")).unwrap();
    let path = std::env::join_paths(
        std::iter::once(root.to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let trace = root.join("trace");
    for standalone in [false, true] {
        let run = |mode: &str| {
            let mut command = if standalone {
                let mut command = Command::new("bash");
                command.arg(repository.join("scripts/prepare-ci-toolchain"));
                command
            } else {
                let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
                command.arg("prepare-ci-toolchain");
                command
            };
            command
                .env("PATH", &path)
                .env("RUNNER_TEMP", root)
                .env("ARKHAM_FIXTURE_RUSTUP", mode)
                .env("ARKHAM_FIXTURE_TRACE", &trace)
                .current_dir(root)
                .output()
                .unwrap()
        };
        fs::write(&trace, "").unwrap();
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
        assert!(!run("uninstall-fail").status.success());
    }
}
