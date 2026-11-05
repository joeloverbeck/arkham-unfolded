use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::symlink,
    process::{Command, Output},
};
mod support;

const REPO: &str = "github.com/joeloverbeck/arkham-unfolded";
const HEAD: &str = "ab12b1b141d31b401ffca8b0563ca97debeed956";
const BASE: &str = "4e610893604d05eb256a2dfe1e19dd43c8004d54";
const MERGED: &str = "dc5b7c01c9dd4870a9ea1ff668f3ecd90a006397";

// Public response shapes and values retained from the accepted PR #108, with
// irrelevant API fields omitted. The fixture never contacts GitHub.
fn pull() -> Value {
    json!({"number":108,"state":"OPEN","isDraft":false,"headRefOid":HEAD,
        "baseRefName":"main","baseRefOid":BASE,"mergeStateStatus":"CLEAN","mergeCommit":null})
}

fn checks() -> Value {
    json!([{"total_count":1,"check_runs":[{"id":127414138289_u64,"name":"Rust checks",
        "head_sha":HEAD,"status":"completed","conclusion":"success","app":{"id":15368}}]}])
}

fn merged() -> Value {
    let mut result = pull();
    result["state"] = json!("MERGED");
    result["mergeCommit"] = json!({"oid":MERGED});
    result
}

fn pull_args() -> Vec<String> {
    [
        "pr",
        "view",
        "108",
        "--repo",
        REPO,
        "--json",
        "number,state,isDraft,headRefOid,baseRefName,baseRefOid,mergeStateStatus,mergeCommit",
    ]
    .map(str::to_owned)
    .to_vec()
}

fn check_args() -> Vec<String> {
    vec![
        "api".into(),
        "--hostname".into(),
        "github.com".into(),
        format!(
            "repos/joeloverbeck/arkham-unfolded/commits/{HEAD}/check-runs?filter=latest&per_page=100"
        ),
        "--paginate".into(),
        "--slurp".into(),
    ]
}

fn merge_args() -> Vec<String> {
    [
        "pr",
        "merge",
        "108",
        "--repo",
        REPO,
        "--merge",
        "--match-head-commit",
        HEAD,
    ]
    .map(str::to_owned)
    .to_vec()
}

struct Github {
    root: tempfile::TempDir,
    steps: usize,
}

impl Github {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        symlink(support::fixture().join("child"), root.path().join("gh")).unwrap();
        Self { root, steps: 0 }
    }

    fn step(&mut self, args: Vec<String>, response: impl ToString, status: u8) {
        for (suffix, content) in [
            ("args", args.join("\n")),
            ("stdout", response.to_string()),
            ("status", status.to_string()),
        ] {
            fs::write(
                self.root.path().join(format!("{}.{suffix}", self.steps)),
                content,
            )
            .unwrap();
        }
        self.steps += 1;
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .args([
                "merge-reviewed",
                "--pr",
                "108",
                "--head",
                HEAD,
                "--base",
                BASE,
            ])
            .env("PATH", self.root.path())
            .env("ARKHAM_FIXTURE_GITHUB", self.root.path())
            .output()
            .unwrap()
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.root.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

#[test]
fn verified_head_base_and_actions_check_allow_one_head_matched_merge_then_readback() {
    let mut github = Github::new();
    github.step(pull_args(), pull(), 0);
    github.step(check_args(), checks(), 0);
    github.step(pull_args(), pull(), 0);
    github.step(merge_args(), "", 0);
    github.step(pull_args(), merged(), 0);
    let output = github.run();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains(MERGED));
    assert_eq!(github.calls().len(), 5);
    assert_eq!(github.calls()[3], merge_args().join("\t"));
}

#[test]
fn rejected_pr_or_check_preconditions_never_invoke_merge() {
    let mut cases = Vec::new();
    for (field, value) in [
        ("number", json!(109)),
        ("state", json!("CLOSED")),
        ("isDraft", json!(true)),
        ("headRefOid", json!(BASE)),
        ("baseRefOid", json!(HEAD)),
        ("baseRefName", json!("release")),
        ("mergeStateStatus", json!("BEHIND")),
        ("mergeStateStatus", json!("UNKNOWN")),
    ] {
        let mut changed = pull();
        changed[field] = value;
        cases.push((format!("PR {field}"), changed, checks(), false));
    }
    for (field, value) in [
        ("head_sha", json!(BASE)),
        ("name", json!("Other check")),
        ("app", json!({"id":42})),
        ("status", json!("in_progress")),
        ("conclusion", json!("failure")),
        ("conclusion", json!("skipped")),
        ("conclusion", Value::Null),
    ] {
        let mut changed = checks();
        changed[0]["check_runs"][0][field] = value;
        cases.push((format!("check {field}"), pull(), changed, true));
    }
    cases.push((
        "missing check".into(),
        pull(),
        json!([{"total_count":0,"check_runs":[]}]),
        true,
    ));
    let mut incomplete = checks();
    incomplete[0]["total_count"] = json!(2);
    cases.push(("incomplete checks".into(), pull(), incomplete, true));
    let mut ambiguous = checks();
    let mut pending = ambiguous[0]["check_runs"][0].clone();
    pending["status"] = json!("queued");
    ambiguous[0]["check_runs"]
        .as_array_mut()
        .unwrap()
        .push(pending);
    ambiguous[0]["total_count"] = json!(2);
    cases.push((
        "successful old check alongside queued rerun".into(),
        pull(),
        ambiguous,
        true,
    ));
    for (name, pr, checks, reaches_checks) in cases {
        let mut github = Github::new();
        github.step(pull_args(), pr, 0);
        if reaches_checks {
            github.step(check_args(), checks, 0);
        }
        let output = github.run();
        assert!(!output.status.success(), "{name}: {output:?}");
        let calls = github.calls();
        assert_eq!(
            calls.len(),
            if reaches_checks { 2 } else { 1 },
            "{name}: {calls:?}"
        );
        assert!(
            !calls.iter().any(|call| call.starts_with("pr\tmerge")),
            "{name}"
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("Unexpected gh invocation"),
            "{name}: {output:?}"
        );
    }
}

#[test]
fn failed_commands_and_malformed_responses_stop_before_merge() {
    for step in 0..3 {
        for (body, code) in [("{}", 1), ("not JSON", 0), ("{}", 0)] {
            let mut github = Github::new();
            if step >= 1 {
                github.step(pull_args(), pull(), 0);
            }
            if step >= 2 {
                github.step(check_args(), checks(), 0);
            }
            github.step(
                if step == 1 { check_args() } else { pull_args() },
                body,
                code,
            );
            let output = github.run();
            assert!(!output.status.success(), "step {step}: {output:?}");
            assert_eq!(github.calls().len(), step + 1);
        }
    }
}

#[test]
fn head_or_base_moving_during_checks_stops_before_merge() {
    for field in ["headRefOid", "baseRefOid"] {
        let mut github = Github::new();
        github.step(pull_args(), pull(), 0);
        github.step(check_args(), checks(), 0);
        let mut changed = pull();
        changed[field] = json!(MERGED);
        github.step(pull_args(), changed, 0);
        let output = github.run();
        assert!(!output.status.success(), "{field}: {output:?}");
        assert_eq!(github.calls().len(), 3);
    }
}

#[test]
fn merge_failure_is_read_back_and_never_retried() {
    let mut github = Github::new();
    github.step(pull_args(), pull(), 0);
    github.step(check_args(), checks(), 0);
    github.step(pull_args(), pull(), 0);
    github.step(merge_args(), "", 1);
    github.step(pull_args(), merged(), 0);
    let output = github.run();
    assert!(!output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("not retried"));
    assert!(String::from_utf8_lossy(&output.stderr).contains(MERGED));
    assert_eq!(github.calls().len(), 5);
}

#[test]
fn all_check_pages_are_verified_before_merging() {
    let mut github = Github::new();
    github.step(pull_args(), pull(), 0);
    let required = checks()[0]["check_runs"][0].clone();
    let others: Vec<_> = (1..=100)
        .map(|id| {
            let mut check = required.clone();
            check["id"] = json!(id);
            check["name"] = json!(format!("Additional check {id}"));
            check
        })
        .collect();
    github.step(
        check_args(),
        json!([
            {"total_count":101,"check_runs":others},
            {"total_count":101,"check_runs":[required]}
        ]),
        0,
    );
    github.step(pull_args(), pull(), 0);
    github.step(merge_args(), "", 0);
    github.step(pull_args(), merged(), 0);
    let output = github.run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(github.calls().len(), 5);
}

#[test]
fn unsuccessful_or_unconfirmed_merge_readback_never_claims_completion() {
    for (body, code) in [
        (pull().to_string(), 0),
        ("{}".into(), 1),
        ("not JSON".into(), 0),
    ] {
        let mut github = Github::new();
        github.step(pull_args(), pull(), 0);
        github.step(check_args(), checks(), 0);
        github.step(pull_args(), pull(), 0);
        github.step(merge_args(), "", 0);
        github.step(pull_args(), body, code);
        let output = github.run();
        assert!(!output.status.success(), "{output:?}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("Merged"));
        assert_eq!(github.calls().len(), 5);
    }
}

#[test]
fn duplicate_check_ids_cannot_disguise_an_incomplete_response() {
    let mut github = Github::new();
    github.step(pull_args(), pull(), 0);
    let required = checks()[0]["check_runs"][0].clone();
    let mut other = required.clone();
    other["id"] = json!(1);
    other["name"] = json!("Other check");
    github.step(
        check_args(),
        json!([{"total_count":3,"check_runs":[required,other,other]}]),
        0,
    );
    let output = github.run();
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(
        github.calls().len(),
        2,
        "no PR refresh or merge after incomplete evidence"
    );
}

#[test]
fn invalid_revision_arguments_do_not_contact_github() {
    let github = Github::new();
    for (pr, head, base) in [
        ("0", HEAD, BASE),
        ("108", "main", BASE),
        ("108", HEAD, "4e61089"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .args(["merge-reviewed", "--pr", pr, "--head", head, "--base", base])
            .env("PATH", github.root.path())
            .env("ARKHAM_FIXTURE_GITHUB", github.root.path())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert!(github.calls().is_empty());
    }
}
