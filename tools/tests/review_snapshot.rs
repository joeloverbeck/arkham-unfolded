use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn repository(root: &Path) -> String {
    fs::create_dir(root).unwrap();
    git(root, &["init", "--quiet"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "Snapshot test"]);
    fs::write(root.join("selected.txt"), "base\n").unwrap();
    fs::write(root.join("context.txt"), "unchanged context\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "--quiet", "-m", "base"]);
    String::from_utf8(git(root, &["rev-parse", "HEAD"]))
        .unwrap()
        .trim()
        .into()
}

fn snapshot(
    root: &Path,
    base: &str,
    output: &Path,
    paths: &[&str],
    previous: Option<&Path>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
    command
        .current_dir(root)
        .args(["review-snapshot", "--base", base, "--output-dir"])
        .arg(output);
    if let Some(previous) = previous {
        command.arg("--previous").arg(previous);
    }
    command.arg("--").args(paths).output().unwrap()
}

fn manifest(output: &Path) -> Value {
    serde_json::from_slice(&fs::read(output.join("manifest.json")).unwrap()).unwrap()
}

#[test]
fn captures_selected_working_bytes_with_base_context_and_leaves_the_checkout_alone() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "staged\n").unwrap();
    git(&root, &["add", "selected.txt"]);
    fs::write(root.join("selected.txt"), "working\n").unwrap();
    fs::write(root.join("context.txt"), "unrelated edit\n").unwrap();
    let index = fs::read(root.join(".git/index")).unwrap();
    let status = git(&root, &["status", "--porcelain=v1", "-z"]);
    let output = directory.path().join("review");
    let result = snapshot(&root, &base, &output, &["selected.txt"], None);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(output.join("candidate/selected.txt")).unwrap(),
        b"working\n"
    );
    assert_eq!(
        fs::read(output.join("candidate/context.txt")).unwrap(),
        b"unchanged context\n"
    );
    assert_eq!(
        fs::read(output.join("base/selected.txt")).unwrap(),
        b"base\n"
    );
    let record = manifest(&output);
    assert_eq!(record["base"], base);
    assert_eq!(record["head"], base);
    assert_eq!(
        record["changes"],
        serde_json::json!([{"path":"selected.txt","kind":"modified"}])
    );
    assert_eq!(record["files"]["selected.txt"]["mode"], "100644");
    assert_eq!(
        record["files"]["selected.txt"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    let patch = fs::read_to_string(output.join("changes.patch")).unwrap();
    assert!(patch.contains("-base\n+working\n"));
    assert!(!patch.contains("unrelated edit"));
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(git(&root, &["status", "--porcelain=v1", "-z"]), status);
    assert_eq!(
        fs::read(root.join("context.txt")).unwrap(),
        b"unrelated edit\n"
    );
    fs::write(root.join("selected.txt"), "later edit\n").unwrap();
    assert_eq!(
        fs::read(output.join("candidate/selected.txt")).unwrap(),
        b"working\n"
    );
}

#[test]
fn preserves_binary_additions_deletions_executable_modes_and_symlink_targets() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    repository(&root);
    fs::write(root.join("removed.txt"), "remove me").unwrap();
    fs::write(root.join("mode.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    symlink("context.txt", root.join("link")).unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "--quiet", "-m", "file types"]);
    let base = String::from_utf8(git(&root, &["rev-parse", "HEAD"])).unwrap();
    fs::remove_file(root.join("removed.txt")).unwrap();
    fs::set_permissions(root.join("mode.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::remove_file(root.join("link")).unwrap();
    symlink("missing target", root.join("link")).unwrap();
    fs::write(root.join("new\nfile.bin"), [0, 255, 13, 10, 0]).unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    let output = directory.path().join("review");
    let result = snapshot(
        &root.join("nested"),
        base.trim(),
        &output,
        &["removed.txt", "mode.sh", "link", "new\nfile.bin"],
        None,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!output.join("candidate/removed.txt").exists());
    assert_eq!(
        fs::read(output.join("candidate/new\nfile.bin")).unwrap(),
        [0, 255, 13, 10, 0]
    );
    assert_eq!(
        fs::metadata(output.join("candidate/mode.sh"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        fs::read_link(output.join("candidate/link")).unwrap(),
        Path::new("missing target")
    );
    let record = manifest(&output);
    assert_eq!(record["files"]["link"]["mode"], "120000");
    assert_eq!(record["files"]["mode.sh"]["mode"], "100755");
    assert_eq!(
        record["changes"],
        serde_json::json!([
            {"path":"link","kind":"modified"}, {"path":"mode.sh","kind":"modified"},
            {"path":"new\nfile.bin","kind":"added"}, {"path":"removed.txt","kind":"deleted"}
        ])
    );
    let patch = fs::read_to_string(output.join("changes.patch")).unwrap();
    assert!(patch.contains("GIT binary patch"));
    assert!(patch.contains("old mode 100644\nnew mode 100755"));
}

#[test]
fn reports_delta_from_a_verified_previous_snapshot_and_refuses_tampering() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("new.txt"), "first\n").unwrap();
    let first = directory.path().join("first");
    let result = snapshot(&root, &base, &first, &["new.txt"], None);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    fs::write(root.join("new.txt"), "second\n").unwrap();
    fs::write(root.join("selected.txt"), "changed\n").unwrap();
    let second = directory.path().join("second");
    let result = snapshot(
        &root,
        &base,
        &second,
        &["new.txt", "selected.txt"],
        Some(&first),
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let record = manifest(&second);
    assert_eq!(
        record["delta"],
        serde_json::json!([
            {"path":"new.txt","kind":"modified"}, {"path":"selected.txt","kind":"modified"}
        ])
    );
    use sha2::{Digest, Sha256};
    assert_eq!(
        record["previous_manifest_sha256"],
        format!(
            "{:x}",
            Sha256::digest(fs::read(first.join("manifest.json")).unwrap())
        )
    );
    fs::write(first.join("candidate/new.txt"), "tampered\n").unwrap();
    let rejected = directory.path().join("rejected");
    let result = snapshot(&root, &base, &rejected, &["new.txt"], Some(&first));
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("previous snapshot"));
    assert!(!rejected.exists());
}

#[test]
fn refuses_invalid_inputs_and_overwrite_without_creating_a_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join(".gitignore"), "private.txt\n").unwrap();
    fs::write(root.join("private.txt"), "private").unwrap();
    for (name, reference, paths) in [
        ("bad-base", "missing-revision", vec!["selected.txt"]),
        ("empty", base.as_str(), vec!["selected.txt"]),
        ("unknown", base.as_str(), vec!["missing.txt"]),
        ("ignored", base.as_str(), vec!["private.txt"]),
        ("escape", base.as_str(), vec!["../outside.txt"]),
        ("git", base.as_str(), vec![".git/config"]),
        ("directory", base.as_str(), vec!["."]),
    ] {
        let output = directory.path().join(name);
        let result = snapshot(&root, reference, &output, &paths, None);
        assert!(!result.status.success(), "{name} unexpectedly succeeded");
        assert!(!output.exists(), "{name} left a published output");
    }
    fs::write(root.join("selected.txt"), "changed\n").unwrap();
    let output = directory.path().join("existing");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel"), "keep\n").unwrap();
    let result = snapshot(&root, &base, &output, &["selected.txt"], None);
    assert!(!result.status.success());
    assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"keep\n");
    let inside = root.join("snapshot");
    assert!(
        !snapshot(&root, &base, &inside, &["selected.txt"], None)
            .status
            .success()
    );
    assert!(!inside.exists());
}

#[test]
fn fixed_point_keeps_committed_changes_and_excluded_base_context() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "committed after base\n").unwrap();
    fs::write(root.join("context.txt"), "unselected committed edit\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "--quiet", "-m", "later"]);
    let head = String::from_utf8(git(&root, &["rev-parse", "HEAD"])).unwrap();
    let output = directory.path().join("review");
    let result = snapshot(&root, &base, &output, &["selected.txt"], None);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(output.join("candidate/selected.txt")).unwrap(),
        b"committed after base\n"
    );
    assert_eq!(
        fs::read(output.join("candidate/context.txt")).unwrap(),
        b"unchanged context\n"
    );
    assert_eq!(manifest(&output)["head"], head.trim());
    assert_eq!(manifest(&output)["base"], base);
}
