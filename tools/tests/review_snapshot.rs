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
fn all_changed_captures_both_rename_paths_and_mixed_working_changes_with_explicit_exclusions() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    git(&root, &["mv", "selected.txt", "renamed.txt"]);
    git(&root, &["commit", "--quiet", "-m", "rename"]);
    fs::write(root.join("context.txt"), "staged\n").unwrap();
    git(&root, &["add", "context.txt"]);
    fs::write(root.join("renamed.txt"), "unstaged\n").unwrap();
    fs::write(root.join("untracked space.txt"), "new\n").unwrap();
    fs::write(root.join("unrelated.txt"), "leave alone\n").unwrap();
    let index = fs::read(root.join(".git/index")).unwrap();
    let output_dir = directory.path().join("snapshot");
    let result = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .current_dir(&root)
        .args([
            "review-snapshot",
            "--base",
            &base,
            "--all-changed",
            "--exclude",
            "unrelated.txt",
            "--output-dir",
        ])
        .arg(&output_dir)
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    let record = manifest(&output_dir);
    assert_eq!(
        record["selected_paths"],
        serde_json::json!([
            "context.txt",
            "renamed.txt",
            "selected.txt",
            "untracked space.txt"
        ])
    );
    assert_eq!(
        record["excluded_paths"],
        serde_json::json!(["unrelated.txt"])
    );
    assert!(!output_dir.join("candidate/selected.txt").exists());
    assert!(!output_dir.join("candidate/unrelated.txt").exists());
    assert_eq!(
        fs::read_to_string(output_dir.join("candidate/renamed.txt")).unwrap(),
        "unstaged\n"
    );
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
}

#[test]
fn refresh_refuses_accidental_scope_contraction_but_accepts_an_explicit_exclusion() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "changed\n").unwrap();
    fs::write(root.join("context.txt"), "changed\n").unwrap();
    let first = directory.path().join("first");
    assert!(
        snapshot(&root, &base, &first, &["selected.txt", "context.txt"], None)
            .status
            .success()
    );
    let omitted = directory.path().join("omitted");
    let result = snapshot(&root, &base, &omitted, &["selected.txt"], Some(&first));
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("scope contraction"));
    assert!(!omitted.exists());
    let excluded = directory.path().join("excluded");
    let result = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .current_dir(&root)
        .args([
            "review-snapshot",
            "--base",
            &base,
            "--all-changed",
            "--exclude",
            "context.txt",
            "--previous",
        ])
        .arg(&first)
        .arg("--output-dir")
        .arg(&excluded)
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        manifest(&excluded)["excluded_paths"],
        serde_json::json!(["context.txt"])
    );
}

#[test]
fn delta_only_summary_keeps_verification_but_omits_unchanged_scope() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "first\n").unwrap();
    fs::write(root.join("context.txt"), "first\n").unwrap();
    let first = directory.path().join("first");
    assert!(
        snapshot(&root, &base, &first, &["selected.txt", "context.txt"], None)
            .status
            .success()
    );
    assert!(
        !summary(&first, &manifest_digest(&first), &["--delta-only"])
            .status
            .success()
    );
    fs::write(root.join("selected.txt"), "second\n").unwrap();
    let second = directory.path().join("second");
    assert!(
        snapshot(
            &root,
            &base,
            &second,
            &["selected.txt", "context.txt"],
            Some(&first)
        )
        .status
        .success()
    );
    let hash = manifest_digest(&second);
    let output = summary(&second, &hash, &["--delta-only"]);
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Selected paths: 2"));
    assert!(text.contains("Changes since previous snapshot: 1"));
    assert!(text.contains("modified \"selected.txt\""));
    assert!(!text.contains("context.txt"));
    fs::write(second.join("candidate/context.txt"), "tampered\n").unwrap();
    let refused = summary(&second, &hash, &["--delta-only"]);
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
}

fn manifest_digest(snapshot: &Path) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(fs::read(snapshot.join("manifest.json")).unwrap())
    )
}

fn verify_index(root: &Path, snapshot: &Path, sha256: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .current_dir(root)
        .args(["review-verify", "--snapshot"])
        .arg(snapshot)
        .args(["--sha256", sha256])
        .output()
        .unwrap()
}

fn summary(snapshot: &Path, sha256: &str, options: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .current_dir(snapshot.parent().unwrap())
        .args(["review-summary", "--snapshot"])
        .arg(snapshot)
        .args(["--sha256", sha256])
        .args(options)
        .output()
        .unwrap()
}

#[test]
fn summary_identifies_the_selected_candidate_and_delta_without_dumping_context() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "first\n").unwrap();
    let first = directory.path().join("first");
    assert!(
        snapshot(&root, &base, &first, &["selected.txt"], None)
            .status
            .success()
    );
    fs::write(root.join("selected.txt"), "second\n").unwrap();
    fs::write(root.join("new.txt"), "new\n").unwrap();
    let second = directory.path().join("second");
    assert!(
        snapshot(
            &root,
            &base,
            &second,
            &["new.txt", "selected.txt"],
            Some(&first)
        )
        .status
        .success()
    );
    let hash = manifest_digest(&second);
    let index = fs::read(root.join(".git/index")).unwrap();
    let record = manifest(&second);
    let result = summary(&second, &hash, &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains(&format!("Manifest SHA256: {hash}")));
    for label in ["Fixed point", "Base", "Head"] {
        assert!(text.contains(&format!("{label}: {base}")));
    }
    assert!(text.contains(record["patch_sha256"].as_str().unwrap()));
    assert!(text.contains(&manifest_digest(&first)));
    assert!(text.contains("Selected paths: 2"));
    assert!(text.contains("Changes since previous snapshot: 2"));
    assert!(text.contains("added \"new.txt\""));
    assert!(text.contains("modified \"selected.txt\""));
    assert!(!text.contains("context.txt"));
    assert!(text.ends_with("END\n"));
    assert!(text.len() <= 6000);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(manifest_digest(&second), hash);
}

#[test]
fn summary_pages_keep_every_selected_path_under_the_byte_budget() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    let paths: Vec<_> = (0..40)
        .map(|index| format!("{index:02} café with spaces and a long review path.txt"))
        .collect();
    for path in &paths {
        fs::write(root.join(path), "candidate\n").unwrap();
    }
    let output = directory.path().join("review");
    assert!(
        snapshot(
            &root,
            &base,
            &output,
            &paths.iter().map(String::as_str).collect::<Vec<_>>(),
            None
        )
        .status
        .success()
    );
    let hash = manifest_digest(&output);
    for budget in [512, 1024, 6000] {
        let mut start = 1;
        let mut complete = String::new();
        loop {
            let result = summary(
                &output,
                &hash,
                &[
                    "--start",
                    &start.to_string(),
                    "--max-bytes",
                    &budget.to_string(),
                ],
            );
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(result.stdout.len() <= budget);
            let page = String::from_utf8(result.stdout).unwrap();
            complete.push_str(&page);
            if let Some(next) = page.lines().find(|line| line.starts_with("NEXT ")) {
                assert!(next.contains(&hash));
                let following: usize = next.split_whitespace().nth(2).unwrap().parse().unwrap();
                assert!(
                    following > start && following < 100,
                    "pagination must advance"
                );
                start = following;
            } else {
                assert!(page.ends_with("END\n"));
                break;
            }
        }
        for path in &paths {
            assert_eq!(complete.matches(&format!("added {path:?}")).count(), 1);
        }
        assert!(!complete.contains("context.txt"));
        assert!(complete.contains("no previous snapshot"));
    }
}

#[test]
fn summary_refuses_changed_identity_or_candidate_without_emitting_a_page() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "reviewed\n").unwrap();
    let output = directory.path().join("review");
    assert!(
        snapshot(&root, &base, &output, &["selected.txt"], None)
            .status
            .success()
    );
    let hash = manifest_digest(&output);
    let wrong = summary(&output, &"0".repeat(64), &[]);
    assert!(!wrong.status.success());
    assert!(wrong.stdout.is_empty());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("recorded SHA256"));

    let manifest_path = output.join("manifest.json");
    let original = fs::read(&manifest_path).unwrap();
    let mut changed = original.clone();
    changed.push(b'\n');
    fs::write(&manifest_path, changed).unwrap();
    let continuation = summary(&output, &hash, &["--start", "4", "--max-bytes", "512"]);
    assert!(!continuation.status.success());
    assert!(continuation.stdout.is_empty());
    assert!(String::from_utf8_lossy(&continuation.stderr).contains("recorded SHA256"));

    fs::write(&manifest_path, original).unwrap();
    fs::write(output.join("candidate/selected.txt"), "tampered\n").unwrap();
    let tampered = summary(&output, &hash, &[]);
    assert!(!tampered.status.success());
    assert!(tampered.stdout.is_empty());
    assert!(String::from_utf8_lossy(&tampered.stderr).contains("candidate files"));
}

#[test]
fn verifies_index_contents_despite_patch_formatting_without_changing_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "reviewed\n").unwrap();
    let output = directory.path().join("review");
    assert!(
        snapshot(&root, &base, &output, &["selected.txt"], None)
            .status
            .success()
    );
    let hash = manifest_digest(&output);
    git(&root, &["add", "selected.txt"]);
    let patch = fs::read(output.join("changes.patch")).unwrap();
    assert_ne!(patch, git(&root, &["diff", "--cached", "--binary", &base]));
    // Only the staged candidate is being checked; later working edits stay local.
    fs::write(root.join("selected.txt"), "later unstaged edit\n").unwrap();
    let index = fs::read(root.join(".git/index")).unwrap();
    let result = verify_index(&root, &output, &hash);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("Index matches reviewed candidate"));
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(
        fs::read(root.join("selected.txt")).unwrap(),
        b"later unstaged edit\n"
    );
    assert_eq!(fs::read(output.join("changes.patch")).unwrap(), patch);
    assert_eq!(manifest_digest(&output), hash);
}

#[test]
fn rejects_changed_index_contents_and_unreviewed_staged_files() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "reviewed\n").unwrap();
    let output = directory.path().join("review");
    assert!(
        snapshot(&root, &base, &output, &["selected.txt"], None)
            .status
            .success()
    );
    let hash = manifest_digest(&output);
    fs::write(root.join("selected.txt"), "different staged bytes\n").unwrap();
    git(&root, &["add", "selected.txt"]);
    fs::write(root.join("selected.txt"), "reviewed\n").unwrap();
    let index = fs::read(root.join(".git/index")).unwrap();
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("selected.txt"));
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(root.join("selected.txt")).unwrap(), b"reviewed\n");

    git(&root, &["add", "selected.txt"]);
    assert!(verify_index(&root, &output, &hash).status.success());
    fs::write(root.join("extra.txt"), "outside the review\n").unwrap();
    git(&root, &["add", "extra.txt"]);
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("extra.txt"));

    git(&root, &["rm", "--cached", "extra.txt"]);
    git(&root, &["config", "diff.ignoreSubmodules", "all"]);
    git(
        &root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000",
            &base,
            "external",
        ],
    );
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unsupported index mode"));
}

#[test]
fn refuses_changed_snapshot_identity_files_and_patch() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "reviewed\n").unwrap();
    let output = directory.path().join("review");
    assert!(
        snapshot(&root, &base, &output, &["selected.txt"], None)
            .status
            .success()
    );
    let hash = manifest_digest(&output);
    git(&root, &["add", "selected.txt"]);
    let index = fs::read(root.join(".git/index")).unwrap();
    let manifest_path = output.join("manifest.json");
    let original = fs::read(&manifest_path).unwrap();
    let mut changed = original.clone();
    changed.push(b'\n');
    fs::write(&manifest_path, changed).unwrap();
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("manifest SHA-256"));
    fs::write(manifest_path, original).unwrap();

    fs::write(output.join("candidate/selected.txt"), "tampered\n").unwrap();
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("candidate files"));
    fs::write(output.join("candidate/selected.txt"), "reviewed\n").unwrap();
    fs::write(output.join("changes.patch"), "tampered\n").unwrap();
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("patch does not match"));
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
}

#[test]
fn intent_to_add_does_not_count_as_staging_an_empty_reviewed_file() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("empty.txt"), "").unwrap();
    let output = directory.path().join("review");
    assert!(
        snapshot(&root, &base, &output, &["empty.txt"], None)
            .status
            .success()
    );
    let hash = manifest_digest(&output);
    git(&root, &["add", "--intent-to-add", "empty.txt"]);
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("empty.txt"));
    git(&root, &["add", "empty.txt"]);
    assert!(verify_index(&root, &output, &hash).status.success());
}

#[test]
fn refuses_an_unresolved_index() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let base = repository(&root);
    fs::write(root.join("selected.txt"), "reviewed\n").unwrap();
    let output = directory.path().join("review");
    assert!(
        snapshot(&root, &base, &output, &["selected.txt"], None)
            .status
            .success()
    );
    let hash = manifest_digest(&output);
    git(&root, &["checkout", "-b", "ours"]);
    git(&root, &["add", "selected.txt"]);
    git(&root, &["commit", "--quiet", "-m", "ours"]);
    git(&root, &["checkout", "-b", "theirs", &base]);
    fs::write(root.join("selected.txt"), "conflicting\n").unwrap();
    git(&root, &["add", "selected.txt"]);
    git(&root, &["commit", "--quiet", "-m", "theirs"]);
    git(&root, &["checkout", "ours"]);
    let merge = Command::new("git")
        .current_dir(&root)
        .args(["merge", "--no-edit", "theirs"])
        .output()
        .unwrap();
    assert!(!merge.status.success());
    let index = fs::read(root.join(".git/index")).unwrap();
    let result = verify_index(&root, &output, &hash);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("resolve index conflicts"));
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
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

    let hash = manifest_digest(&output);
    git(&root, &["add", "--all"]);
    assert!(
        verify_index(&root.join("nested"), &output, &hash)
            .status
            .success()
    );
    // Each omitted staged change must be detected, including modes and link targets.
    for path in ["removed.txt", "mode.sh", "link", "new\nfile.bin"] {
        git(&root, &["reset", "--quiet", base.trim(), "--", path]);
        let result = verify_index(&root, &output, &hash);
        assert!(!result.status.success(), "missed staged change at {path:?}");
        assert!(String::from_utf8_lossy(&result.stderr).contains("index differs"));
        git(&root, &["add", "--all", "--", path]);
        assert!(verify_index(&root, &output, &hash).status.success());
    }
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
