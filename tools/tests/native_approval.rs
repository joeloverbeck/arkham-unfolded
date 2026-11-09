use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture {
    root: tempfile::TempDir,
    id: String,
    question: Value,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let cwd = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        fs::write(
            root.path().join("session.jsonl"),
            format!(
                "{}\n",
                json!({
                    "type":"session_meta", "payload":{"id":"test-thread", "cwd":cwd}
                })
            ),
        )
        .unwrap();
        let mut fixture = Self {
            root,
            id: String::new(),
            question: Value::Null,
        };
        let output = fixture.command(&[
            "approval",
            "request",
            "--session-log",
            fixture.root.path().join("session.jsonl").to_str().unwrap(),
            "--minutes",
            "8",
            "--purpose",
            "check the Gathering controls using disposable saves",
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let request: Value = serde_json::from_slice(&output.stdout).unwrap();
        fixture.id = request["approval"].as_str().unwrap().into();
        fixture.question = request["request"]["questions"][0].clone();
        fixture
    }

    fn command(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .arg("native")
            .args(args)
            .env("XDG_STATE_HOME", self.root.path().join("state"))
            .env("CODEX_THREAD_ID", "test-thread")
            .env_remove("DISPLAY")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .output()
            .unwrap()
    }

    fn reply(&self, seconds_ago: f64, answer: Option<&str>, call: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let text = format!(
            "<send_user_message_question_reply>\n{}\n</send_user_message_question_reply>",
            json!([{
                "answer":answer.unwrap_or(self.question["options"][0].as_str().unwrap()),
                "question":self.question["title"],
                "questionItemId":serde_json::to_string(&json!(["request_user_input_async",call,0])).unwrap()
            }])
        );
        let events = [
            json!({"type":"response_item","payload":{"type":"function_call", "name":"request_user_input_async", "call_id":"request-1", "arguments":serde_json::to_string(&json!({"questions":[self.question]})).unwrap()}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":text}],"internal_chat_message_metadata_passthrough":{"create_time":now-seconds_ago,"content_item_metadata":[{"provenance":{"type":"user"}}]}}}),
        ];
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(self.root.path().join("session.jsonl"))
            .unwrap();
        for event in events {
            writeln!(file, "{event}").unwrap();
        }
    }

    fn status(&self) -> Output {
        self.command(&["approval", "status", "--approval", &self.id])
    }

    fn assert_desktop_refused(&self, reason: &str) {
        let path = self.root.path().join("must-not-exist");
        let path = path.to_str().unwrap();
        for args in [
            vec!["launch", "--output-dir", path, "--binary", "/absent-binary"],
            vec![
                "launch",
                "--installation",
                "normal",
                "--output-dir",
                path,
                "--binary",
                "/absent-binary",
            ],
            vec!["capture", "--pid", "1", "--window", "1", "--output", path],
            vec![
                "session",
                "--pid",
                "1",
                "--window",
                "1",
                "--output-dir",
                path,
                "--deadline-unix-seconds",
                "9999999999",
            ],
        ] {
            let mut args = args;
            args.extend(["--approval", &self.id]);
            let output = self.command(&args);
            assert_eq!(output.status.code(), Some(1));
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(reason), "{args:?}: {error}");
            assert!(!std::path::Path::new(path).exists());
        }
    }
}

#[test]
fn original_reply_deadline_survives_restart_and_cannot_be_extended() {
    let fixture = Fixture::new();
    fixture.reply(600.0, None, "request-1");
    fixture.assert_desktop_refused("approval has expired");
    fixture.assert_desktop_refused("approval has expired");
}

#[test]
fn active_approval_is_stable_and_closure_survives_process_restart() {
    let fixture = Fixture::new();
    fixture.reply(30.0, None, "request-1");
    let first = fixture.status();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let second: Value = serde_json::from_slice(&fixture.status().stdout).unwrap();
    assert_eq!(
        first["deadline_unix_seconds"],
        second["deadline_unix_seconds"]
    );
    assert_eq!(
        first["deadline_unix_seconds"].as_f64().unwrap() - first["approved_at"].as_f64().unwrap(),
        480.0
    );
    assert!(
        fixture
            .command(&["approval", "close", "--approval", &fixture.id])
            .status
            .success()
    );
    fixture.assert_desktop_refused("approval is closed");
    assert!(
        fixture
            .command(&["approval", "close", "--approval", &fixture.id])
            .status
            .success()
    );
}

#[test]
fn absent_declined_or_mismatched_reply_cannot_admit_desktop_commands() {
    for (answer, call) in [(Some("Not now"), "request-1"), (None, "old-request")] {
        let fixture = Fixture::new();
        fixture.reply(10.0, answer, call);
        fixture.assert_desktop_refused("No matching user approval");
    }
    Fixture::new().assert_desktop_refused("No matching user approval");
}

#[test]
fn changing_duration_or_bound_source_cannot_renew_an_approval() {
    let fixture = Fixture::new();
    fixture.reply(30.0, None, "request-1");
    assert!(fixture.status().status.success());
    let path = fixture
        .root
        .path()
        .join("state/arkham-unfolded/native-approvals")
        .join(&fixture.id)
        .join("request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    request["minutes"] = json!(10);
    fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    fixture.assert_desktop_refused("does not bind its ID and duration");
    request["minutes"] = json!(8);
    fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let source = fixture.root.path().join("session.jsonl");
    let mut events: Vec<Value> = fs::read_to_string(&source)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    events[2]["payload"]["internal_chat_message_metadata_passthrough"]["create_time"] = json!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            - 1.
    );
    fs::write(
        &source,
        events
            .iter()
            .map(|event| format!("{event}\n"))
            .collect::<String>(),
    )
    .unwrap();
    fixture.assert_desktop_refused("Approval evidence changed");
}
