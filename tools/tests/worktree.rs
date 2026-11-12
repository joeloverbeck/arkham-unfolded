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

/// A worktree added by hand beside `owner`, with its own Cargo configuration file.
fn manual_worktree(owner: &Path, name: &str, file: &str, config: &str) -> PathBuf {
    let manual = owner.parent().unwrap().join(name);
    fs::create_dir_all(manual.join(".cargo")).unwrap();
    fs::write(manual.join(".cargo").join(file), config).unwrap();
    let checkout = manual.join("worktree");
    git(
        owner,
        &[
            "worktree",
            "add",
            "--quiet",
            "--detach",
            checkout.to_str().unwrap(),
        ],
    );
    manual
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
    fs::write(owner.join(".gitignore"), "/target/\n/local/\n").unwrap();
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
    // Worktrees laid out by hand, as before this command, hold the first free values,
    // one in the configuration file name Cargo still reads without an extension.
    let units = |value| format!("[profile.dev.package.demo]\ncodegen-units = {value}\n");
    manual_worktree(&owner, "manual", "config.toml", &units(249));
    manual_worktree(&owner, "legacy", "config", &units(248));

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
        assert!((1..248).contains(&value), "{value}");
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
fn add_resolves_parent_components_before_checking_where_dir_is() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    fs::create_dir(directory.path().join("agents")).unwrap();

    succeeds(worktree(
        &owner,
        &["add", "--from", "main", "../agents/relative"],
    ));
    let hidden = directory.path().join("agents/../owner/inside");
    let output = worktree(&owner, &["add", "--from", "main", hidden.to_str().unwrap()]);

    assert!(
        directory
            .path()
            .join("agents/relative/worktree/Cargo.toml")
            .exists()
    );
    assert!(!output.status.success());
    assert!(!owner.join("inside").exists());
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
    // A copy staged for a README run is build output, which goes with the checkout.
    fs::create_dir_all(merged.join("worktree/target/debug")).unwrap();
    fs::write(merged.join("worktree/target/debug/demo"), "build\n").unwrap();

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
fn remove_keeps_checkouts_that_fail_a_check_and_layouts_it_did_not_write() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let dir = |name: &str| directory.path().join(name);
    for name in ["unmerged", "dirty", "evidence", "own"] {
        let branch = format!("agents/{name}");
        succeeds(worktree(
            &owner,
            &[
                "add",
                "--from",
                "main",
                "--branch",
                &branch,
                dir(name).to_str().unwrap(),
            ],
        ));
    }
    commit_in(&dir("unmerged").join("worktree"), "unmerged");
    fs::write(dir("dirty").join("worktree/untracked.txt"), "work\n").unwrap();
    // Ignored, so `git worktree remove` alone would delete it.
    fs::create_dir_all(dir("evidence").join("worktree/local/verification")).unwrap();
    fs::write(
        dir("evidence").join("worktree/local/verification/record.txt"),
        "evidence\n",
    )
    .unwrap();
    manual_worktree(&owner, "manual", "config.toml", "[build]\njobs = 1\n");
    let main_commit = git(&owner, &["rev-parse", "main"]);

    let attempts = [
        ("unmerged", "main"),
        ("dirty", "main"),
        ("evidence", "main"),
        ("manual", "main"),
        // Its own branch would be deleted, and a bare commit keeps nothing reachable.
        ("own", "agents/own"),
        ("own", main_commit.as_str()),
    ];
    for (name, merged_into) in attempts {
        let output = worktree(
            &owner,
            &[
                "remove",
                "--merged-into",
                merged_into,
                dir(name).to_str().unwrap(),
            ],
        );
        assert!(!output.status.success(), "{name} {merged_into}");
        assert!(dir(name).join("worktree/Cargo.toml").exists(), "{name}");
        assert!(dir(name).join(".cargo/config.toml").exists(), "{name}");
    }
    assert!(
        dir("evidence")
            .join("worktree/local/verification/record.txt")
            .exists()
    );
    assert_eq!(
        branches(&owner),
        "agents/dirty\nagents/evidence\nagents/own\nagents/unmerged\nmain"
    );
}
