//! The `worktree` command's contract against a scratch repository: each checkout it adds
//! builds into the owner's target with a `codegen-units` value no other registered
//! worktree uses, and removal takes only merged, clean checkouts and what `add` wrote.
//! With them, the shared target's free-disk floor and `target-prune`, which removes only
//! unused directories inside its `incremental` directories and unused, superseded builds of
//! the repository's own packages.

use std::{
    fs,
    io::{BufRead, BufReader, Read},
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, SystemTime},
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

/// `worktree add` with no free-disk floor, so a test does not depend on the machine's
/// free disk; the floor has tests of its own.
fn add(owner: &Path, args: &[&str]) -> Output {
    let mut all = vec!["add", "--min-free-gib", "0"];
    all.extend(args);
    worktree(owner, &all)
}

fn prune(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .arg("target-prune")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

/// An incremental cache under `target`, as rustc lays one out with a session and its lock
/// file, holding `bytes` and last used `hours` ago.
fn cache(target: &Path, relative: &str, bytes: usize, hours: u64) -> PathBuf {
    let path = target.join(relative);
    fs::create_dir_all(path.join("s-session")).unwrap();
    fs::write(path.join("s-session/query-cache.bin"), vec![7; bytes]).unwrap();
    fs::write(path.join("s-session.lock"), "").unwrap();
    let used = SystemTime::now() - Duration::from_secs(hours * 3600);
    fs::File::open(&path).unwrap().set_modified(used).unwrap();
    path
}

/// Enough unused caches, with names long enough, that their listing overfills a pipe.
fn fillers(target: &Path) -> Vec<PathBuf> {
    (0..2000)
        .map(|n| {
            let name =
                format!("debug/incremental/filler-named-at-length-to-overfill-a-pipe-{n:04}");
            cache(target, &name, 1, 240)
        })
        .collect()
}

/// `target-prune` with its output left unread after the first line, so that it waits to
/// write the rest of a listing `fillers` made too long for the pipe, before it removes
/// anything. `meanwhile` runs during that wait and gets its process ID. Returns the exit
/// code, standard output and standard error.
fn prune_held(
    owner: &Path,
    args: &[&str],
    meanwhile: impl FnOnce(u32),
) -> (Option<i32>, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .arg("target-prune")
        .args(args)
        .current_dir(owner)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut printed = String::new();
    stdout.read_line(&mut printed).unwrap();
    assert!(printed.starts_with("target "), "{printed}");
    meanwhile(child.id());
    stdout.read_to_string(&mut printed).unwrap();
    let mut error = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut error)
        .unwrap();
    (child.wait().unwrap().code(), printed, error)
}

fn worktree_count(owner: &Path) -> usize {
    git(owner, &["worktree", "list", "--porcelain"])
        .matches("worktree ")
        .count()
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
    let printed = succeeds(add(
        &owner,
        &[
            "--from",
            "main",
            "--branch",
            "agents/first",
            first.to_str().unwrap(),
        ],
    ));
    let second = directory.path().join("agents/second");
    succeeds(add(&owner, &["--from", "main", second.to_str().unwrap()]));

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

    let output = add(&owner, &["--from", "main", taken.to_str().unwrap()]);

    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(taken.join("notes.md")).unwrap(),
        "mine\n"
    );
    assert!(!taken.join(".cargo").exists());
}

#[test]
fn add_resolves_parent_components_and_links_before_checking_where_dir_is() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    fs::create_dir(directory.path().join("agents")).unwrap();
    std::os::unix::fs::symlink(&owner, directory.path().join("link")).unwrap();

    succeeds(add(&owner, &["--from", "main", "../agents/relative"]));
    for hidden in ["agents/../owner/inside", "link/inside"] {
        let hidden = directory.path().join(hidden);
        let output = add(&owner, &["--from", "main", hidden.to_str().unwrap()]);
        assert!(!output.status.success(), "{}", hidden.display());
    }

    assert!(
        directory
            .path()
            .join("agents/relative/worktree/Cargo.toml")
            .exists()
    );
    assert!(!owner.join("inside").exists());
}

#[test]
fn remove_takes_merged_checkouts_their_branches_and_only_what_add_wrote() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let merged = directory.path().join("merged");
    let noted = directory.path().join("noted");
    for (dir, branch) in [(&merged, "agents/merged"), (&noted, "agents/noted")] {
        succeeds(add(
            &owner,
            &["--from", "main", "--branch", branch, dir.to_str().unwrap()],
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
    // An ignored directory with nothing in it loses nothing.
    fs::create_dir_all(merged.join("worktree/local/verification")).unwrap();

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
        succeeds(add(
            &owner,
            &[
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

#[test]
fn add_reports_free_disk_and_each_incremental_cache_directory_of_the_owners_target() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    cache(&target, "debug/incremental/demo-1abc", 3 << 20, 0);
    cache(
        &target,
        "agent-tools/debug/incremental/tools-2def",
        1 << 20,
        0,
    );
    let dir = directory.path().join("agents/reported");

    let printed = succeeds(add(&owner, &["--from", "main", dir.to_str().unwrap()]));

    assert!(printed.contains("free disk"), "{printed}");
    assert!(printed.contains("debug/incremental 3.0 MiB"), "{printed}");
    assert!(
        printed.contains("agent-tools/debug/incremental 1.0 MiB"),
        "{printed}"
    );
}

#[test]
fn add_below_the_free_disk_floor_names_the_reclaimable_caches_and_creates_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    cache(
        &owner.join("target"),
        "debug/incremental/demo-1abc",
        3 << 20,
        0,
    );
    let dir = directory.path().join("agents/refused");

    // No machine running these tests has a pebibyte free.
    let output = worktree(
        &owner,
        &[
            "add",
            "--min-free-gib",
            "1048576",
            "--from",
            "main",
            "--branch",
            "agents/refused",
            dir.to_str().unwrap(),
        ],
    );

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    for expected in [
        "below the 1048576 GiB floor",
        "3.0 MiB",
        "scripts/tools target-prune",
        "--min-free-gib",
    ] {
        assert!(error.contains(expected), "{expected}: {error}");
    }
    assert!(!directory.path().join("agents").exists());
    assert_eq!(branches(&owner), "main");
    assert_eq!(worktree_count(&owner), 1);
}

#[test]
fn target_prune_lists_the_owners_unused_caches_and_removes_them_only_with_delete() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    let old = cache(&target, "debug/incremental/demo-old", 2 << 20, 240);
    let tools = cache(
        &target,
        "agent-tools/debug/incremental/tools-old",
        1 << 20,
        240,
    );
    let recent = cache(&target, "debug/incremental/demo-recent", 1 << 20, 24);
    let dir = directory.path().join("agent");
    succeeds(add(&owner, &["--from", "main", dir.to_str().unwrap()]));
    let checkout = dir.join("worktree");
    // A checkout's own target goes with it; the command reads the owner's from any checkout.
    let own = cache(
        &checkout.join("target"),
        "debug/incremental/own-old",
        1 << 20,
        240,
    );

    let listed = succeeds(prune(&checkout, &[]));

    for shown in [
        "2.0 MiB",
        "debug/incremental/demo-old",
        "agent-tools/debug/incremental/tools-old",
        "--delete",
    ] {
        assert!(listed.contains(shown), "{shown}: {listed}");
    }
    assert!(
        !listed.contains("demo-recent") && !listed.contains("own-old"),
        "{listed}"
    );
    assert!(old.exists() && tools.exists() && recent.exists());

    succeeds(prune(&checkout, &["--delete"]));

    assert!(!old.exists() && !tools.exists());
    assert!(recent.exists() && own.exists());
    assert!(target.join("debug/incremental").is_dir());

    succeeds(prune(&owner, &["--older-than", "12h", "--delete"]));

    assert!(!recent.exists());
}

#[test]
fn target_prune_removes_only_directories_inside_incremental_and_refuses_links_out() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    let unused = cache(&target, "debug/incremental/demo-old", 1 << 20, 240);
    // Old build output outside an incremental directory is not a cache.
    let deps = cache(&target, "debug/deps/demo-old", 1 << 20, 240);
    let stray = target.join("debug/incremental/stray.bin");
    fs::write(&stray, "not a cache\n").unwrap();
    // Links out of the target: one in place of a cache, one in place of an incremental
    // directory.
    let outside = cache(directory.path(), "outside/demo-old", 1 << 20, 240);
    symlink(&outside, target.join("debug/incremental/escape")).unwrap();
    fs::create_dir_all(target.join("release")).unwrap();
    symlink(
        directory.path().join("outside"),
        target.join("release/incremental"),
    )
    .unwrap();

    for invalid in ["0d", "30m", "7", "d", "-1d", "3651d"] {
        let output = prune(&owner, &["--older-than", invalid, "--delete"]);
        assert!(!output.status.success(), "{invalid}");
        assert!(unused.exists(), "{invalid}");
    }
    let output = prune(&owner, &["--older-than", "1h", "--delete"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let error = String::from_utf8_lossy(&output.stderr);
    for refused in ["debug/incremental/escape", "release/incremental"] {
        assert!(error.contains(refused), "{refused}: {error}");
    }
    assert!(!unused.exists());
    assert!(deps.exists() && stray.exists());
    assert!(outside.join("s-session/query-cache.bin").exists());
    assert!(
        fs::symlink_metadata(target.join("debug/incremental/escape"))
            .unwrap()
            .is_symlink()
    );
}

#[test]
fn target_prune_interrupted_before_a_removal_removes_nothing_more_and_exits_130() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let caches = fillers(&owner.join("target"));

    let (code, _, error) = prune_held(&owner, &["--delete"], |pid| {
        assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGINT) }, 0);
        // The handler records the interrupt on a thread of its own.
        thread::sleep(Duration::from_millis(500));
    });

    assert_eq!(code, Some(130), "{error}");
    assert!(error.contains("interrupted"), "{error}");
    assert!(caches.iter().all(|cache| cache.exists()));
}

#[test]
fn target_prune_keeps_a_cache_whose_rustc_session_lock_is_held() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    let locked = cache(&target, "debug/incremental/demo-locked", 1 << 20, 240);
    let unlocked = cache(&target, "debug/incremental/demo-unlocked", 1 << 20, 240);
    // As a rustc compiling the crate holds it.
    let lock = fs::File::open(locked.join("s-session.lock")).unwrap();
    lock.lock().unwrap();

    let printed = succeeds(prune(&owner, &["--delete"]));

    assert!(
        printed.contains("kept debug/incremental/demo-locked: a rustc session holds its lock"),
        "{printed}"
    );
    assert!(locked.join("s-session/query-cache.bin").exists());
    assert!(!unlocked.exists());
    assert!(
        printed.contains("removed debug/incremental/demo-unlocked"),
        "{printed}"
    );
    drop(lock);
}

#[test]
fn target_prune_reports_a_cache_already_gone_and_a_failed_removal_with_what_is_left() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    fillers(&target);
    let gone = cache(&target, "debug/incremental/demo-gone", 1 << 20, 240);
    let stuck = cache(&target, "debug/incremental/demo-stuck", 1 << 20, 240);
    // Its session's files cannot be unlinked, except by root, which removes it whole.
    let session = stuck.join("s-session");
    fs::set_permissions(&session, fs::Permissions::from_mode(0o500)).unwrap();
    let root = unsafe { libc::geteuid() } == 0;

    let (code, printed, error) = prune_held(&owner, &["--delete"], |_| {
        fs::remove_dir_all(&gone).unwrap()
    });
    fs::set_permissions(&session, fs::Permissions::from_mode(0o700)).unwrap();

    assert!(
        printed.contains("already gone debug/incremental/demo-gone"),
        "{printed}"
    );
    assert!(
        printed.contains(&format!("removed {} caches", 2000 + usize::from(root))),
        "{printed}"
    );
    if !root {
        assert_eq!(code, Some(1), "{error}");
        assert!(
            error.contains(
                "removal of debug/incremental/demo-stuck failed, 1.0 MiB of 1.0 MiB left"
            ),
            "{error}"
        );
        assert!(session.join("query-cache.bin").exists());
    }
}

#[test]
fn target_prune_reports_a_failed_removal_whose_remainder_cannot_be_measured_as_failed() {
    // Root reads and removes the session whole.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    fillers(&target);
    let unreadable = cache(&target, "debug/incremental/demo-unreadable", 1 << 20, 240);
    let session = unreadable.join("s-session");

    // Made unreadable after the scan measured it: neither the removal nor the measurement of
    // what is left can then list the session.
    let (code, _, error) = prune_held(&owner, &["--delete"], |_| {
        fs::set_permissions(&session, fs::Permissions::from_mode(0o300)).unwrap()
    });
    fs::set_permissions(&session, fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(code, Some(1), "{error}");
    assert!(
        error.contains("removal of debug/incremental/demo-unreadable failed")
            && error.contains("could not be measured"),
        "{error}"
    );
    assert!(!error.contains("kept"), "{error}");
    assert!(session.join("query-cache.bin").exists());
}

/// One build of a target of `package` as Cargo leaves it in the profile directory `profile`
/// of `target`: its fingerprint directory, and `files` in that profile directory with `hash`
/// in place of `{}`, each of `bytes`, all last written `hours` ago. Returns the files and
/// then the fingerprint directory.
fn build(
    target: &Path,
    profile: &str,
    package: &str,
    hash: &str,
    files: &[&str],
    bytes: usize,
    hours: u64,
) -> Vec<PathBuf> {
    let profile = target.join(profile);
    let written = SystemTime::now() - Duration::from_secs(hours * 3600);
    let mut paths: Vec<_> = files
        .iter()
        .map(|file| {
            let path = profile.join(file.replace("{}", hash));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, vec![7; bytes]).unwrap();
            fs::File::open(&path)
                .unwrap()
                .set_modified(written)
                .unwrap();
            path
        })
        .collect();
    let fingerprint = profile.join(format!(".fingerprint/{package}-{hash}"));
    fs::create_dir_all(&fingerprint).unwrap();
    let timestamp = fingerprint.join("invoked.timestamp");
    fs::write(&timestamp, "").unwrap();
    fs::File::open(&timestamp)
        .unwrap()
        .set_modified(written)
        .unwrap();
    fs::File::open(&fingerprint)
        .unwrap()
        .set_modified(written)
        .unwrap();
    paths.push(fingerprint);
    paths
}

const SUITE: [&str; 2] = ["deps/suite-{}", "deps/suite-{}.d"];

#[test]
fn target_prune_lists_superseded_builds_of_own_packages_and_removes_them_only_with_delete() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    let library = [
        "deps/demo-{}.d",
        "deps/libdemo-{}.rlib",
        "deps/libdemo-{}.rmeta",
    ];
    let tool = ["deps/member_tool-{}", "deps/member_tool-{}.d"];
    let example = ["examples/sample-{}", "examples/sample-{}.d"];
    let third_party = ["deps/serde-{}.d", "deps/libserde-{}.rlib"];
    let mib = 1 << 20;
    let superseded = [
        build(
            &target,
            "debug",
            "demo",
            "a000000000000001",
            &SUITE,
            mib,
            240,
        ),
        build(
            &target,
            "debug",
            "demo",
            "b000000000000001",
            &library,
            mib,
            240,
        ),
        build(
            &target,
            "debug",
            "demo-member",
            "c000000000000001",
            &tool,
            mib,
            240,
        ),
        build(
            &target,
            "debug",
            "demo",
            "d000000000000001",
            &example,
            mib,
            240,
        ),
        build(
            &target,
            "agent-tools/debug",
            "demo",
            "e000000000000001",
            &SUITE,
            mib,
            240,
        ),
    ];
    let recent = build(
        &target,
        "debug",
        "demo",
        "a000000000000002",
        &SUITE,
        mib,
        24,
    );
    let kept = [
        // The newest build of each target in each profile directory.
        build(&target, "debug", "demo", "a000000000000003", &SUITE, mib, 1),
        build(
            &target,
            "debug",
            "demo",
            "b000000000000002",
            &library,
            mib,
            2,
        ),
        build(
            &target,
            "debug",
            "demo-member",
            "c000000000000002",
            &tool,
            mib,
            3,
        ),
        build(
            &target,
            "debug",
            "demo",
            "d000000000000002",
            &example,
            mib,
            1,
        ),
        build(
            &target,
            "agent-tools/debug",
            "demo",
            "e000000000000002",
            &SUITE,
            mib,
            1,
        ),
        // The library's unit-test executable is a target of its own, newest though old.
        build(
            &target,
            "debug",
            "demo",
            "b000000000000003",
            &["deps/demo-{}", "deps/demo-{}.d"],
            mib,
            240,
        ),
        // A dependency's builds are not the repository's.
        build(
            &target,
            "debug",
            "serde",
            "f000000000000001",
            &third_party,
            mib,
            240,
        ),
        build(
            &target,
            "debug",
            "serde",
            "f000000000000002",
            &third_party,
            mib,
            1,
        ),
    ];
    // Cargo's unhashed copy of the newest build.
    let copy = target.join("debug/suite");
    fs::write(&copy, "").unwrap();

    let listed = succeeds(prune(&owner, &[]));

    for shown in [
        "debug/deps/suite-a000000000000001",
        "debug/deps/demo-b000000000000001",
        "debug/deps/member_tool-c000000000000001",
        "debug/examples/sample-d000000000000001",
        "agent-tools/debug/deps/suite-e000000000000001",
        "5 superseded builds",
        "--delete",
    ] {
        assert!(listed.contains(shown), "{shown}: {listed}");
    }
    for hidden in [
        "a000000000000002",
        "a000000000000003",
        "b000000000000002",
        "b000000000000003",
        "serde",
    ] {
        assert!(!listed.contains(hidden), "{hidden}: {listed}");
    }
    let all = || superseded.iter().chain(&kept).chain([&recent]).flatten();
    assert!(all().all(|path| path.exists()));

    let removed = succeeds(prune(&owner, &["--delete"]));

    assert!(
        removed.contains("removed 0 caches and 5 builds"),
        "{removed}"
    );
    assert!(superseded.iter().flatten().all(|path| !path.exists()));
    assert!(
        kept.iter()
            .chain([&recent])
            .flatten()
            .all(|path| path.exists())
    );
    assert!(copy.exists());

    succeeds(prune(&owner, &["--older-than", "12h", "--delete"]));

    assert!(recent.iter().all(|path| !path.exists()));
    assert!(kept.iter().flatten().all(|path| path.exists()));
}

#[test]
fn target_prune_keeps_the_builds_of_a_profile_directory_whose_cargo_build_lock_is_held() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    let held = build(&target, "debug", "demo", "a000000000000001", &SUITE, 1, 240);
    build(&target, "debug", "demo", "a000000000000002", &SUITE, 1, 1);
    let free = build(
        &target,
        "release",
        "demo",
        "a000000000000001",
        &SUITE,
        1,
        240,
    );
    build(&target, "release", "demo", "a000000000000002", &SUITE, 1, 1);
    // As a Cargo build into `debug` holds it.
    let lock = fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(target.join("debug/.cargo-build-lock"))
        .unwrap();
    lock.lock().unwrap();

    let printed = succeeds(prune(&owner, &["--delete"]));

    assert!(
        printed.contains(
            "kept debug/deps/suite-a000000000000001: a Cargo build holds the lock on its directory"
        ),
        "{printed}"
    );
    assert!(held.iter().all(|path| path.exists()));
    assert!(free.iter().all(|path| !path.exists()));
    drop(lock);

    succeeds(prune(&owner, &["--delete"]));

    assert!(held.iter().all(|path| !path.exists()));
}

#[test]
fn target_prune_refuses_links_among_builds_and_follows_none() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    let outside = directory.path().join("outside");
    let outer = build(
        &outside,
        "debug",
        "demo",
        "a000000000000009",
        &SUITE,
        1,
        240,
    );
    let superseded = build(&target, "debug", "demo", "a000000000000001", &SUITE, 1, 240);
    build(&target, "debug", "demo", "a000000000000002", &SUITE, 1, 1);
    // An own build's file, a fingerprint directory, and a profile's `deps` and
    // `.fingerprint` directories, each a link out of the target.
    let fingerprint = target.join("debug/.fingerprint/demo-a000000000000008");
    fs::create_dir_all(&fingerprint).unwrap();
    let file = target.join("debug/deps/suite-a000000000000008");
    symlink(&outer[0], &file).unwrap();
    let linked_fingerprint = target.join("debug/.fingerprint/demo-a000000000000009");
    symlink(
        outside.join("debug/.fingerprint/demo-a000000000000009"),
        &linked_fingerprint,
    )
    .unwrap();
    build(&target, "release", "demo", "a000000000000001", &[], 1, 1);
    let deps = target.join("release/deps");
    symlink(outside.join("debug/deps"), &deps).unwrap();
    fs::create_dir_all(target.join("profile")).unwrap();
    let fingerprints = target.join("profile/.fingerprint");
    symlink(outside.join("debug/.fingerprint"), &fingerprints).unwrap();

    let output = prune(&owner, &["--older-than", "1h", "--delete"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let error = String::from_utf8_lossy(&output.stderr);
    for refused in [&file, &linked_fingerprint, &deps, &fingerprints] {
        let refused = refused.strip_prefix(&target).unwrap().to_str().unwrap();
        assert!(error.contains(refused), "{refused}: {error}");
    }
    assert!(superseded.iter().all(|path| !path.exists()));
    assert!(outer.iter().all(|path| path.exists()));
    for link in [&file, &linked_fingerprint, &deps, &fingerprints] {
        assert!(fs::symlink_metadata(link).unwrap().is_symlink(), "{link:?}");
    }
}

#[test]
fn target_prune_interrupted_before_a_build_removal_removes_nothing_more_and_exits_130() {
    let directory = tempfile::tempdir().unwrap();
    let owner = repository(directory.path());
    let target = owner.join("target");
    // Enough superseded builds, with names long enough, that their listing overfills a pipe.
    let fillers: Vec<_> = (0..2000)
        .map(|n| {
            let hash = format!("{n:016x}");
            build(
                &target,
                "debug",
                "demo",
                &hash,
                &["deps/filler-named-at-length-to-overfill-a-pipe-{}"],
                1,
                240,
            )
        })
        .collect();
    build(
        &target,
        "debug",
        "demo",
        "ffffffffffffffff",
        &["deps/filler-named-at-length-to-overfill-a-pipe-{}"],
        1,
        1,
    );

    let (code, _, error) = prune_held(&owner, &["--delete"], |pid| {
        assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGINT) }, 0);
        // The handler records the interrupt on a thread of its own.
        thread::sleep(Duration::from_millis(500));
    });

    assert_eq!(code, Some(130), "{error}");
    assert!(error.contains("interrupted"), "{error}");
    assert!(fillers.iter().flatten().all(|path| path.exists()));
}
