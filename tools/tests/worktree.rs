//! The `worktree` command's contract against a scratch repository: each checkout it adds
//! builds into the owner's target with a `codegen-units` value no other registered
//! worktree uses, and removal takes only merged, clean checkouts and what `add` wrote.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

/// A repository whose workspace has two packages, like this one.
fn repository(root: &Path) -> PathBuf {
    let owner = root.join("owner");
    fs::create_dir_all(owner.join("member")).unwrap();
    git(&owner, &["init", "--quiet", "-b", "main"]);
    git(&owner, &["config", "user.email", "test@example.invalid"]);
    git(&owner, &["config", "user.name", "Worktree test"]);
    fs::write(
        owner.join("Cargo.toml"),
        "[package]\nname = \"demo\"\n\n[workspace]\nmembers = [\"member\"]\n",
    )
    .unwrap();
    fs::write(
        owner.join("member/Cargo.toml"),
        "[package]\nname = \"demo-member\"\n",
    )
    .unwrap();
    git(&owner, &["add", "."]);
    git(&owner, &["commit", "--quiet", "-m", "base"]);
    owner
}

fn worktree(owner: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .arg("worktree")
        .args(args)
        .current_dir(owner)
        .output()
        .unwrap()
}

fn succeeds(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn config(dir: &Path) -> toml::Table {
    fs::read_to_string(dir.join(".cargo/config.toml"))
        .unwrap()
        .parse()
        .unwrap()
}

/// The value `add` gave both packages, which must agree.
fn codegen_units(dir: &Path) -> i64 {
    let config = config(dir);
    let packages = &config["profile"]["dev"]["package"];
    let value = packages["demo"]["codegen-units"].as_integer().unwrap();
    assert_eq!(
        packages["demo-member"]["codegen-units"].as_integer(),
        Some(value)
    );
    value
}

fn branches(owner: &Path) -> String {
    git(owner, &["branch", "--format=%(refname:short)"])
}

fn commit_in(checkout: &Path, file: &str) {
    fs::write(checkout.join(file), "work\n").unwrap();
    git(checkout, &["add", file]);
    git(checkout, &["commit", "--quiet", "-m", file]);
}

#[test]
fn added_checkouts_share_the_owners_target_with_values_no_other_worktree_uses() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    // A worktree laid out by hand, as before this command, holds the first free value.
    let manual = directory.path().join("manual");
    fs::create_dir_all(manual.join(".cargo")).unwrap();
    fs::write(
        manual.join(".cargo/config.toml"),
        "[profile.dev.package.demo]\ncodegen-units = 249\n",
    )
    .unwrap();
    git(
        &owner,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            "../manual/worktree",
        ],
    );

    let first = directory.path().join("agents/first");
    let printed = succeeds(worktree(
        &owner,
        &[
            "add",
            "--from",
            "main",
            "--branch",
            "agents/first",
            first.to_str().unwrap(),
        ],
    ));
    let second = directory.path().join("agents/second");
    succeeds(worktree(
        &owner,
        &["add", "--from", "main", second.to_str().unwrap()],
    ));

    assert!(
        printed.contains(first.join("worktree").to_str().unwrap()),
        "{printed}"
    );
    assert_eq!(
        git(&first.join("worktree"), &["branch", "--show-current"]),
        "agents/first"
    );
    assert_eq!(
        git(&second.join("worktree"), &["branch", "--show-current"]),
        ""
    );
    assert_eq!(
        config(&first)["build"]["target-dir"].as_str(),
        Some(owner.join("target").to_str().unwrap())
    );
    let values = [codegen_units(&first), codegen_units(&second)];
    assert_ne!(values[0], values[1]);
    for value in values {
        // 256 is Cargo's default, which the owner keeps; the baseline recipe passes 250.
        assert!((1..250).contains(&value) && value != 249, "{value}");
    }
}

#[test]
fn add_refuses_an_existing_directory_and_leaves_it_alone() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let taken = directory.path().join("taken");
    fs::create_dir(&taken).unwrap();
    fs::write(taken.join("notes.md"), "mine\n").unwrap();

    let output = worktree(&owner, &["add", "--from", "main", taken.to_str().unwrap()]);

    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(taken.join("notes.md")).unwrap(),
        "mine\n"
    );
    assert!(!taken.join(".cargo").exists());
}

#[test]
fn remove_takes_merged_checkouts_their_branches_and_only_what_add_wrote() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let merged = directory.path().join("merged");
    let noted = directory.path().join("noted");
    for (dir, branch) in [(&merged, "agents/merged"), (&noted, "agents/noted")] {
        succeeds(worktree(
            &owner,
            &[
                "add",
                "--from",
                "main",
                "--branch",
                branch,
                dir.to_str().unwrap(),
            ],
        ));
        commit_in(&dir.join("worktree"), branch.trim_start_matches("agents/"));
    }
    git(
        &owner,
        &[
            "merge",
            "--quiet",
            "--no-edit",
            "agents/merged",
            "agents/noted",
        ],
    );
    fs::write(noted.join("notes.md"), "kept\n").unwrap();

    succeeds(worktree(
        &owner,
        &[
            "remove",
            "--merged-into",
            "main",
            merged.to_str().unwrap(),
            noted.to_str().unwrap(),
        ],
    ));

    assert!(!merged.exists());
    assert_eq!(
        fs::read_to_string(noted.join("notes.md")).unwrap(),
        "kept\n"
    );
    assert!(!noted.join("worktree").exists() && !noted.join(".cargo").exists());
    assert_eq!(branches(&owner), "main");
    assert_eq!(
        git(&owner, &["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count(),
        1
    );
}

#[test]
fn remove_keeps_unmerged_or_dirty_checkouts_and_layouts_it_did_not_write() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let unmerged = directory.path().join("unmerged");
    let dirty = directory.path().join("dirty");
    for (dir, branch) in [(&unmerged, "agents/unmerged"), (&dirty, "agents/dirty")] {
        succeeds(worktree(
            &owner,
            &[
                "add",
                "--from",
                "main",
                "--branch",
                branch,
                dir.to_str().unwrap(),
            ],
        ));
    }
    commit_in(&unmerged.join("worktree"), "unmerged");
    fs::write(dirty.join("worktree/untracked.txt"), "work\n").unwrap();
    let manual = directory.path().join("manual");
    fs::create_dir_all(manual.join(".cargo")).unwrap();
    fs::write(manual.join(".cargo/config.toml"), "[build]\njobs = 1\n").unwrap();
    git(
        &owner,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            "../manual/worktree",
        ],
    );

    for dir in [&unmerged, &dirty, &manual] {
        let output = worktree(
            &owner,
            &["remove", "--merged-into", "main", dir.to_str().unwrap()],
        );
        assert!(!output.status.success(), "{}", dir.display());
        assert!(
            dir.join("worktree/Cargo.toml").exists(),
            "{}",
            dir.display()
        );
        assert!(dir.join(".cargo/config.toml").exists(), "{}", dir.display());
    }
    assert_eq!(branches(&owner), "agents/dirty\nagents/unmerged\nmain");
}
