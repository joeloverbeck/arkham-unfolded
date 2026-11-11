use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::symlink,
    path::PathBuf,
    process::{Command, Output},
};
mod support;

const API: &str = "repos/joeloverbeck/arkham-unfolded";
const BODY: &str =
    "**Current handoff — #9**\n\n- `bb16632` passed: 890 tests.\n- Next: native review.\n";

fn get(path: &str) -> Vec<String> {
    ["api", "--hostname", "github.com", path]
        .map(str::to_owned)
        .to_vec()
}

fn write(method: &str, path: &str) -> Vec<String> {
    [
        "api",
        "--hostname",
        "github.com",
        "--method",
        method,
        path,
        "--input",
        "-",
    ]
    .map(str::to_owned)
    .to_vec()
}

fn comment_url(id: u64) -> String {
    format!("https://github.com/joeloverbeck/arkham-unfolded/issues/9#issuecomment-{id}")
}

// Scripts only the external gh process, as the merge-reviewed tests do. Every
// unexpected invocation fails, and the journal shows which writes happened.
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

    fn body_file(&self, text: &str) -> PathBuf {
        let path = self.root.path().join("body.md");
        fs::write(&path, text).unwrap();
        path
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .arg("publish")
            .args(args)
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

    fn sent(&self, step: usize) -> Value {
        serde_json::from_slice(&fs::read(self.root.path().join(format!("{step}.input"))).unwrap())
            .unwrap()
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_comment_is_sent_as_the_checked_bytes_and_proved_by_readback() {
    let mut github = Github::new();
    github.step(
        write("POST", &format!("{API}/issues/9/comments")),
        json!({"id": 77, "html_url": comment_url(77)}),
        0,
    );
    github.step(
        get(&format!("{API}/issues/comments/77")),
        json!({"id": 77, "body": BODY}),
        0,
    );
    let file = github.body_file(BODY);
    let output = github.run(&[
        "comment",
        "--issue",
        "9",
        "--body-file",
        file.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(github.sent(0), json!({"body": BODY}));
    assert!(String::from_utf8_lossy(&output.stdout).contains(&comment_url(77)));
    assert_eq!(github.calls().len(), 2);
}

#[test]
fn private_path_text_is_refused_before_github_is_contacted() {
    let github = Github::new();
    let file = github.body_file("Evidence kept at /home/someone/run.json\n");
    let output = github.run(&[
        "comment",
        "--issue",
        "9",
        "--body-file",
        file.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("line 1"), "{}", stderr(&output));
    assert!(!stderr(&output).contains("someone"));
    assert!(github.calls().is_empty());
}

#[test]
fn an_issue_closes_only_after_its_comment_reads_back_exactly() {
    let issue = format!("{API}/issues/9");
    let mut github = Github::new();
    github.step(get(&issue), json!({"state": "open"}), 0);
    github.step(
        write("POST", &format!("{issue}/comments")),
        json!({"id": 78, "html_url": comment_url(78)}),
        0,
    );
    github.step(
        get(&format!("{API}/issues/comments/78")),
        json!({"id": 78, "body": "@/tmp/body.md"}),
        0,
    );
    let file = github.body_file(BODY);
    let args = [
        "comment",
        "--issue",
        "9",
        "--body-file",
        file.to_str().unwrap(),
        "--close",
    ];
    let output = github.run(&args);
    assert!(!output.status.success(), "{output:?}");
    assert!(stderr(&output).contains("line 1"), "{}", stderr(&output));
    assert_eq!(
        github.calls().len(),
        3,
        "a mismatch must not close the issue"
    );

    let mut github = Github::new();
    github.step(get(&issue), json!({"state": "open"}), 0);
    github.step(
        write("POST", &format!("{issue}/comments")),
        json!({"id": 79, "html_url": comment_url(79)}),
        0,
    );
    github.step(
        get(&format!("{API}/issues/comments/79")),
        json!({"id": 79, "body": BODY}),
        0,
    );
    github.step(write("PATCH", &issue), json!({"state": "closed"}), 0);
    github.step(get(&issue), json!({"state": "closed"}), 0);
    let file = github.body_file(BODY);
    let args = [
        "comment",
        "--issue",
        "9",
        "--body-file",
        file.to_str().unwrap(),
        "--close",
    ];
    let output = github.run(&args);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        github.sent(3),
        json!({"state": "closed", "state_reason": "completed"})
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Closed #9"));
}

#[test]
fn closing_refuses_a_pull_request_before_anything_is_posted() {
    let mut github = Github::new();
    github.step(
        get(&format!("{API}/issues/9")),
        json!({"state": "open", "pull_request": {"url": "https://example.invalid"}}),
        0,
    );
    let file = github.body_file(BODY);
    let args = [
        "comment",
        "--issue",
        "9",
        "--body-file",
        file.to_str().unwrap(),
        "--close",
    ];
    let output = github.run(&args);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("pull request"),
        "{}",
        stderr(&output)
    );
    assert_eq!(github.calls().len(), 1);
}

#[test]
fn an_already_closed_issue_gets_its_comment_and_stays_closed() {
    let issue = format!("{API}/issues/9");
    let mut github = Github::new();
    github.step(get(&issue), json!({"state": "closed"}), 0);
    github.step(
        write("POST", &format!("{issue}/comments")),
        json!({"id": 80, "html_url": comment_url(80)}),
        0,
    );
    github.step(
        get(&format!("{API}/issues/comments/80")),
        json!({"id": 80, "body": BODY}),
        0,
    );
    let file = github.body_file(BODY);
    let args = [
        "comment",
        "--issue",
        "9",
        "--body-file",
        file.to_str().unwrap(),
        "--close",
    ];
    let output = github.run(&args);
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("already closed"));
    assert_eq!(github.calls().len(), 3);
}

#[test]
fn a_handoff_comment_and_a_pr_body_are_replaced_and_read_back() {
    let comment = format!("{API}/issues/comments/81");
    let mut github = Github::new();
    github.step(
        write("PATCH", &comment),
        json!({"id": 81, "html_url": comment_url(81)}),
        0,
    );
    github.step(get(&comment), json!({"id": 81, "body": BODY}), 0);
    let file = github.body_file(BODY);
    let output = github.run(&[
        "edit-comment",
        "--id",
        "81",
        "--body-file",
        file.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(github.sent(0), json!({"body": BODY}));

    let pull = format!("{API}/pulls/174");
    let url = "https://github.com/joeloverbeck/arkham-unfolded/pull/174";
    let mut github = Github::new();
    github.step(write("PATCH", &pull), json!({"html_url": url}), 0);
    github.step(get(&pull), json!({"html_url": url, "body": BODY}), 0);
    let file = github.body_file(BODY);
    let output = github.run(&[
        "pr-body",
        "--pr",
        "174",
        "--body-file",
        file.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains(url));
}

#[test]
fn a_failed_write_stops_before_any_readback() {
    let mut github = Github::new();
    github.step(write("POST", &format!("{API}/issues/9/comments")), "", 1);
    let file = github.body_file(BODY);
    let output = github.run(&[
        "comment",
        "--issue",
        "9",
        "--body-file",
        file.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert_eq!(github.calls().len(), 1);
}
