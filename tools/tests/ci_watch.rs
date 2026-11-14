//! `ci-watch` against scripted `gh` responses: it exits 0 only once the newest `Check` run
//! of the exact head has completed successfully, and never polls past its time limits.

use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::symlink,
    process::{Command, Output},
};
mod support;

const HEAD: &str = "24cac2a276325e230decbef29eee8951f0cb40dc";
const OTHER: &str = "0cd4ea3f4ecc0641d71231d4362955fd56a17ada";
const RUN: u64 = 43533674588;

// Public response shapes and values retained from PR #216's run, with irrelevant API
// fields omitted. The fixture never contacts GitHub.
fn run(status: &str, conclusion: Option<&str>) -> Value {
    json!({"id":RUN,"name":"Check","head_sha":HEAD,"status":status,"conclusion":conclusion,
        "event":"pull_request","run_attempt":1,
        "html_url":format!("https://github.com/joeloverbeck/arkham-unfolded/actions/runs/{RUN}")})
}

fn page(runs: &[Value]) -> Value {
    json!([{"total_count":runs.len(),"workflow_runs":runs}])
}

fn runs_args() -> Vec<String> {
    vec![
        "api".into(),
        "--hostname".into(),
        "github.com".into(),
        format!(
            "repos/joeloverbeck/arkham-unfolded/actions/workflows/check.yml/runs?head_sha={HEAD}&per_page=100"
        ),
        "--paginate".into(),
        "--slurp".into(),
    ]
}

// Scripts only the external gh process, as the merge-reviewed tests do. Every unexpected
// invocation fails, and the journal shows how often the command polled.
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

    fn poll(&mut self, response: impl ToString, status: u8) {
        for (suffix, content) in [
            ("args", runs_args().join("\n")),
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

    /// One-second polls, with the given time limits in seconds.
    fn watch(&self, appear: u64, timeout: u64) -> Output {
        self.command(&[
            "--head",
            HEAD,
            "--interval-seconds",
            "1",
            "--appear-timeout-seconds",
            &appear.to_string(),
            "--timeout-seconds",
            &timeout.to_string(),
        ])
    }

    fn command(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .arg("ci-watch")
            .args(args)
            .env("PATH", self.root.path())
            .env("ARKHAM_FIXTURE_GITHUB", self.root.path())
            .output()
            .unwrap()
    }

    fn calls(&self) -> usize {
        fs::read_to_string(self.root.path().join("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_run_that_appears_after_the_first_poll_is_followed_to_success_for_that_head() {
    let mut github = Github::new();
    github.poll(page(&[]), 0);
    github.poll(page(&[run("queued", None)]), 0);
    github.poll(page(&[run("in_progress", None)]), 0);
    github.poll(page(&[run("completed", Some("success"))]), 0);

    let output = github.watch(60, 60);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(github.calls(), 4);
    let printed = stdout(&output);
    let last = printed.lines().last().unwrap();
    assert!(
        last.contains("success") && last.contains(HEAD) && last.contains(&RUN.to_string()),
        "{printed}"
    );
    for status in ["queued", "in_progress"] {
        assert!(printed.contains(status), "{status}: {printed}");
    }
}

#[test]
fn a_run_completed_without_success_exits_one_with_its_conclusion_and_head() {
    for conclusion in ["failure", "cancelled", "timed_out", "skipped"] {
        let mut github = Github::new();
        github.poll(page(&[run("completed", Some(conclusion))]), 0);

        let output = github.watch(60, 60);

        assert_eq!(output.status.code(), Some(1), "{conclusion}: {output:?}");
        let printed = stdout(&output);
        let last = printed.lines().last().unwrap();
        assert!(
            last.contains(conclusion) && last.contains(HEAD),
            "{printed}"
        );
        assert_eq!(github.calls(), 1);
    }
}

#[test]
fn a_run_listed_for_another_head_is_never_taken_as_this_heads_result() {
    let mut other = run("completed", Some("success"));
    other["head_sha"] = json!(OTHER);
    other["id"] = json!(RUN + 1);
    for runs in [
        vec![other.clone()],
        vec![run("completed", Some("success")), other],
    ] {
        let mut github = Github::new();
        github.poll(page(&runs), 0);

        let output = github.watch(60, 60);

        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(stderr(&output).contains(OTHER), "{output:?}");
        assert!(!stdout(&output).contains("conclusion"), "{output:?}");
        assert_eq!(github.calls(), 1);
    }
}

#[test]
fn the_newest_run_of_the_head_decides_its_result() {
    let mut older = run("completed", Some("failure"));
    older["id"] = json!(RUN - 1);
    older["event"] = json!("push");
    let mut github = Github::new();
    github.poll(page(&[run("in_progress", None), older.clone()]), 0);
    github.poll(page(&[run("completed", Some("success")), older]), 0);

    let output = github.watch(60, 60);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(stdout(&output).contains("newest of 2"), "{output:?}");
}

#[test]
fn no_run_or_no_conclusion_within_the_time_limits_fails_without_a_conclusion() {
    for (appear, timeout, response, expected) in [
        (1, 60, page(&[]), "appeared within 1 seconds"),
        (
            60,
            1,
            page(&[run("in_progress", None)]),
            "still in_progress",
        ),
    ] {
        let mut github = Github::new();
        // The second poll falls at the time limit; there is no third.
        github.poll(&response, 0);
        github.poll(&response, 0);

        let output = github.watch(appear, timeout);

        assert_eq!(output.status.code(), Some(2), "{expected}: {output:?}");
        assert!(stderr(&output).contains(expected), "{output:?}");
        assert!(!stdout(&output).contains("conclusion"), "{output:?}");
        assert!((1..=2).contains(&github.calls()), "{expected}");
    }
}

#[test]
fn incomplete_malformed_or_failed_answers_stop_the_watch_without_a_conclusion() {
    let mut incomplete = page(&[run("completed", Some("success"))]);
    incomplete[0]["total_count"] = json!(2);
    let success = run("completed", Some("success"));
    let duplicated = json!([{"total_count":2,"workflow_runs":[success.clone(),success]}]);
    for (response, status) in [
        (incomplete, 0),
        (duplicated, 0),
        (json!("not JSON"), 0),
        (json!([]), 0),
        (page(&[]), 1),
    ] {
        let mut github = Github::new();
        github.poll(&response, status);

        let output = github.watch(60, 60);

        assert_eq!(output.status.code(), Some(2), "{response}: {output:?}");
        assert!(!stdout(&output).contains("conclusion"), "{output:?}");
        assert_eq!(github.calls(), 1, "{response}");
    }
}

#[test]
fn a_head_that_is_not_a_full_commit_sha_never_contacts_github() {
    let github = Github::new();
    for head in ["24cac2a", "main", &format!("{HEAD}0")] {
        let output = github.command(&["--head", head]);
        assert!(!output.status.success(), "{head}: {output:?}");
    }
    assert_eq!(github.calls(), 0);
}
