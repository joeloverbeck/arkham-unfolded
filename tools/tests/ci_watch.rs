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

fn jobs_args(attempt: u64) -> Vec<String> {
    vec![
        "api".into(),
        "--hostname".into(),
        "github.com".into(),
        format!(
            "repos/joeloverbeck/arkham-unfolded/actions/runs/{RUN}/attempts/{attempt}/jobs?per_page=100"
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
        self.response(runs_args(), response, status);
    }

    fn jobs(&mut self, attempt: u64, jobs: &[Value]) {
        self.response(
            jobs_args(attempt),
            json!([{"total_count":jobs.len(),"jobs":jobs}]),
            0,
        );
    }

    fn response(&mut self, args: Vec<String>, response: impl ToString, status: u8) {
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

fn job(step: &str) -> Value {
    // The job identity and fields follow the real #213 run's API response.
    json!({"id":132529203201_u64,"run_id":RUN,"run_attempt":1,"head_sha":HEAD,
        "name":"Rust checks","status":"in_progress",
        "steps":[{"number":11,"name":step,"status":"in_progress"}]})
}

#[test]
fn progress_reports_changed_steps_once_with_elapsed_time_and_run_identity() {
    let mut github = Github::new();
    for step in ["Clippy", "Clippy", "Application tests"] {
        github.poll(page(&[run("in_progress", None)]), 0);
        github.jobs(1, &[job(step)]);
    }
    github.poll(page(&[run("completed", Some("success"))]), 0);

    let output = github.watch(60, 60);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let printed = stdout(&output);
    assert_eq!(
        printed.matches("Rust checks: Clippy").count(),
        1,
        "{printed}"
    );
    assert_eq!(
        printed.matches("Rust checks: Application tests").count(),
        1,
        "{printed}"
    );
    for line in printed.lines().filter(|line| line.contains("Rust checks:")) {
        assert!(
            line.contains("elapsed ") && line.contains(&format!("run {RUN} attempt 1")),
            "{line}"
        );
    }
    assert_eq!(github.calls(), 7);
}

#[test]
fn reruns_report_the_same_step_again_under_the_new_attempt() {
    let mut github = Github::new();
    for attempt in [1, 2] {
        let mut running = run("in_progress", None);
        running["run_attempt"] = json!(attempt);
        let mut current = job("Application tests");
        current["run_attempt"] = json!(attempt);
        github.poll(page(&[running]), 0);
        github.jobs(attempt, &[current]);
    }
    let mut finished = run("completed", Some("success"));
    finished["run_attempt"] = json!(2);
    github.poll(page(&[finished]), 0);

    let output = github.watch(60, 60);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let printed = stdout(&output);
    for attempt in [1, 2] {
        assert!(
            printed.contains(&format!(
                "run {RUN} attempt {attempt}: Rust checks: Application tests"
            )),
            "{printed}"
        );
    }
    assert_eq!(github.calls(), 5);
}

#[test]
fn parallel_jobs_are_stable_across_page_order_and_never_supply_the_run_conclusion() {
    let mut github = Github::new();
    let first = job("Application tests");
    let mut second = job("Renderer tests");
    second["id"] = json!(132529203202_u64);
    second["name"] = json!("Renderer");
    for jobs in [
        vec![first.clone(), second.clone()],
        vec![second.clone(), first.clone()],
    ] {
        github.poll(page(&[run("in_progress", None)]), 0);
        github.response(
            jobs_args(1),
            json!([
                {"total_count":2,"jobs":[jobs[0]]},
                {"total_count":2,"jobs":[jobs[1]]}
            ]),
            0,
        );
    }
    github.poll(page(&[run("in_progress", None)]), 0);
    let mut completed = first;
    completed["status"] = json!("completed");
    second["status"] = json!("completed");
    github.jobs(1, &[completed, second]);
    github.poll(page(&[run("completed", Some("failure"))]), 0);

    let output = github.watch(60, 60);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let printed = stdout(&output);
    assert_eq!(
        printed
            .matches("Rust checks: Application tests; Renderer: Renderer tests")
            .count(),
        1,
        "{printed}"
    );
    assert!(
        printed.contains("jobs completed; waiting for run conclusion"),
        "{printed}"
    );
    assert!(
        printed
            .lines()
            .last()
            .unwrap()
            .contains("conclusion failure"),
        "{printed}"
    );
    assert_eq!(github.calls(), 7);
}

#[test]
fn untrusted_jobs_stop_without_printing_their_progress_or_claiming_a_conclusion() {
    let current = job("Application tests");
    let mut responses = vec![
        (json!([{"total_count":2,"jobs":[current.clone()]}]), 0),
        (
            json!([{"total_count":2,"jobs":[current.clone(),current.clone()]}]),
            0,
        ),
        (json!([]), 0),
        (json!("not a jobs response"), 0),
        (json!([]), 1),
    ];
    for (field, wrong) in [
        ("head_sha", json!(OTHER)),
        ("run_id", json!(RUN + 1)),
        ("run_attempt", json!(2)),
    ] {
        let mut mismatched = current.clone();
        mismatched[field] = wrong;
        responses.push((json!([{"total_count":1,"jobs":[mismatched]}]), 0));
    }
    for (response, status) in responses {
        let mut github = Github::new();
        github.poll(page(&[run("in_progress", None)]), 0);
        github.response(jobs_args(1), response, status);

        let output = github.watch(60, 60);

        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(!stdout(&output).contains("Application tests"), "{output:?}");
        assert!(!stdout(&output).contains("conclusion"), "{output:?}");
        assert_eq!(github.calls(), 2);
    }
}

#[test]
fn a_run_that_appears_after_the_first_poll_is_followed_to_success_for_that_head() {
    let mut github = Github::new();
    github.poll(page(&[]), 0);
    github.poll(page(&[run("queued", None)]), 0);
    github.jobs(1, &[]);
    github.poll(page(&[run("in_progress", None)]), 0);
    github.jobs(1, &[job("Application tests")]);
    github.poll(page(&[run("completed", Some("success"))]), 0);

    let output = github.watch(60, 60);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(github.calls(), 6);
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
    github.jobs(1, &[job("Application tests")]);
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
        if timeout == 1 {
            github.jobs(1, &[]);
        }
        github.poll(&response, 0);
        if timeout == 1 {
            github.jobs(1, &[]);
        }

        let output = github.watch(appear, timeout);

        assert_eq!(output.status.code(), Some(2), "{expected}: {output:?}");
        assert!(stderr(&output).contains(expected), "{output:?}");
        assert!(!stdout(&output).contains("conclusion"), "{output:?}");
        let calls_per_poll = if timeout == 1 { 2 } else { 1 };
        assert!(
            (calls_per_poll..=2 * calls_per_poll).contains(&github.calls()),
            "{expected}"
        );
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
