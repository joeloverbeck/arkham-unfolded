use std::{
    ffi::OsStr,
    fs,
    os::unix::ffi::OsStrExt,
    path::Path,
    process::{Command, Output},
};
mod support;

fn git(root: &Path, args: &[&OsStr]) {
    let result = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn check(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .arg("check-tracked-files")
        .current_dir(root)
        .output()
        .unwrap()
}

#[test]
fn index_rejects_force_added_cache_paths_even_when_absent_with_raw_names() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    git(root, &[OsStr::new("init"), OsStr::new("--quiet")]);
    fs::write(
        root.join(".gitignore"),
        "__pycache__/\n*.py[cod]\n.pytest_cache/\n",
    )
    .unwrap();
    let names = [
        b"scripts/__pycache__/generated.pyc".as_slice(),
        b"cache.pyo",
        b"cache.pyd",
        b".pytest_cache/v/cache/nodeids",
        b"scripts/__pycache__/odd\nname.txt",
        b"scripts/__pycache__/nonutf8\xff.txt",
    ];
    for name in names {
        let name = OsStr::from_bytes(name);
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "fixture\n").unwrap();
        git(
            root,
            &[
                OsStr::new("add"),
                OsStr::new("--force"),
                OsStr::new("--"),
                name,
            ],
        );
        fs::remove_file(path).unwrap();
    }
    fs::create_dir(root.join("child")).unwrap();
    let result = check(&root.join("child"));
    assert_eq!(result.status.code(), Some(1));
    assert!(result.stdout.is_empty());
    let errors = String::from_utf8(result.stderr).unwrap();
    assert!(errors.contains("Generated files are tracked"), "{errors}");
    assert!(errors.contains(r"odd\nname.txt"));
    assert!(errors.contains(r"nonutf8\xff.txt"));
    assert_eq!(
        errors.lines().filter(|line| line.starts_with("  ")).count(),
        names.len()
    );
}

#[test]
fn untracked_caches_and_tracked_rust_are_allowed_but_git_failure_is_honest() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let outside = check(root);
    assert_eq!(outside.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&outside.stderr).contains("Could not inspect tracked files"));
    git(root, &[OsStr::new("init"), OsStr::new("--quiet")]);
    fs::write(root.join("cache.pyc"), "untracked").unwrap();
    fs::write(root.join("tool.rs"), "fn main() {}\n").unwrap();
    git(root, &[OsStr::new("add"), OsStr::new("tool.rs")]);
    assert!(check(root).status.success());
}

#[test]
fn tracked_non_rust_sources_and_templates_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    git(root, &[OsStr::new("init"), OsStr::new("--quiet")]);
    fs::create_dir(root.join("scripts")).unwrap();
    for name in [
        "tool.py",
        "scripts/native_verify.py",
        "inline.js",
        "helper.ts",
        "wizard.template.sh",
        "old.rb",
        "child.py.template",
    ] {
        fs::write(root.join(name), "fixture\n").unwrap();
        git(root, &[OsStr::new("add"), OsStr::new(name)]);
    }
    let result = check(root);
    assert_eq!(result.status.code(), Some(1));
    let errors = String::from_utf8(result.stderr).unwrap();
    assert!(
        errors.contains("Non-Rust executable sources are tracked"),
        "{errors}"
    );
    assert_eq!(
        errors.lines().filter(|line| line.starts_with("  ")).count(),
        7
    );
}

#[test]
fn actual_repository_gate_stops_at_a_forbidden_index_before_later_checks() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    git(root, &[OsStr::new("init"), OsStr::new("--quiet")]);
    fs::create_dir(root.join("scripts")).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::copy(repository.join("scripts/check"), root.join("scripts/check")).unwrap();
    fs::write(root.join("generated.pyc"), "fixture").unwrap();
    git(root, &[OsStr::new("add"), OsStr::new("generated.pyc")]);
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    std::os::unix::fs::symlink(support::fixture().join("child"), bin.join("cargo")).unwrap();
    let result = Command::new("bash")
        .arg(root.join("scripts/check"))
        .env(
            "PATH",
            std::env::join_paths(
                std::iter::once(bin)
                    .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
            )
            .unwrap(),
        )
        .env("ARKHAM_FIXTURE_TOOLS", env!("CARGO_BIN_EXE_arkham-tools"))
        .current_dir(root)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let errors = String::from_utf8(result.stderr).unwrap();
    assert!(errors.contains("Generated files are tracked"), "{errors}");
    assert!(!errors.contains("UNEXPECTED_LATER_CHECK"), "{errors}");
}
