use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const PURPOSE: &str = "check the Gathering controls using disposable saves";
const CLAUDE_SESSION: &str = "0b8f2c1e-6d4a-4f7b-9c3e-5a1d2b3c4d5e";
const CLAUDE_CALL: &str = "toolu_approval";
const YES: &str = "Yes, 8 minutes now";

struct Fixture {
    root: tempfile::TempDir,
    log: PathBuf,
    session: (&'static str, Option<String>),
    id: String,
    question: Value,
}

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}

/// Independent oracle: counts whole days forward from 1970, unlike the
/// closed-form conversion under test.
fn rfc3339(unix_ms: i64) -> String {
    let leap = |year: i64| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let (mut days, day_ms) = (
        unix_ms.div_euclid(86_400_000),
        unix_ms.rem_euclid(86_400_000),
    );
    let mut year = 1970;
    while days >= if leap(year) { 366 } else { 365 } {
        days -= if leap(year) { 366 } else { 365 };
        year += 1;
    }
    let mut month = 1;
    for length in [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ] {
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }
    format!(
        "{year:04}-{month:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        days + 1,
        day_ms / 3_600_000,
        day_ms / 60_000 % 60,
        day_ms / 1000 % 60,
        day_ms % 1000
    )
}

/// A Claude Code message entry with the fields observed in real transcripts.
fn claude_entry(kind: &str, unix_ms: i64, message: Value) -> Value {
    json!({
        "type":kind, "sessionId":CLAUDE_SESSION, "cwd":repository(), "isSidechain":false,
        "timestamp":rfc3339(unix_ms), "message":message
    })
}

fn claude_call(id: &str, unix_ms: i64, input: &Value) -> Value {
    claude_entry(
        "assistant",
        unix_ms,
        json!({"role":"assistant", "content":[
            {"type":"tool_use", "id":id, "name":"AskUserQuestion", "input":input}
        ]}),
    )
}

fn claude_opening() -> Vec<Value> {
    vec![
        json!({"type":"mode", "mode":"default", "sessionId":CLAUDE_SESSION}),
        claude_entry(
            "user",
            now_ms() - 120_000,
            json!({"role":"user", "content":"Check the Gathering controls."}),
        ),
    ]
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let log = root.path().join("session.jsonl");
        fs::write(
            &log,
            format!(
                "{}\n",
                json!({
                    "type":"session_meta", "payload":{"id":"test-thread", "cwd":repository()}
                })
            ),
        )
        .unwrap();
        let mut fixture = Self {
            root,
            log,
            session: ("CODEX_THREAD_ID", Some("test-thread".into())),
            id: String::new(),
            question: Value::Null,
        };
        fixture.admit_request();
        fixture
    }

    /// An active Claude Code transcript, named by its session, before any request.
    fn claude_transcript(entries: &[Value]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let log = root.path().join(format!("{CLAUDE_SESSION}.jsonl"));
        let fixture = Self {
            root,
            log,
            session: ("CLAUDE_CODE_SESSION_ID", Some(CLAUDE_SESSION.into())),
            id: String::new(),
            question: Value::Null,
        };
        fs::write(&fixture.log, "").unwrap();
        fixture.append(entries);
        fixture
    }

    fn claude() -> Self {
        let mut fixture = Self::claude_transcript(&claude_opening());
        fixture.admit_request();
        fixture
    }

    fn request(&self) -> Output {
        self.command(&[
            "approval",
            "request",
            "--session-log",
            self.log.to_str().unwrap(),
            "--minutes",
            "8",
            "--purpose",
            PURPOSE,
        ])
    }

    fn admit_request(&mut self) {
        let output = self.request();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let request: Value = serde_json::from_slice(&output.stdout).unwrap();
        self.id = request["approval"].as_str().unwrap().into();
        self.question = request["request"]["questions"][0].clone();
    }

    fn command(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
        command
            .arg("native")
            .args(args)
            .env("XDG_STATE_HOME", self.root.path().join("state"))
            .env_remove("CODEX_THREAD_ID")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env_remove("DISPLAY")
            .env_remove("DBUS_SESSION_BUS_ADDRESS");
        if let (name, Some(value)) = &self.session {
            command.env(name, value);
        }
        command.output().unwrap()
    }

    fn append(&self, entries: &[Value]) {
        let mut file = fs::OpenOptions::new().append(true).open(&self.log).unwrap();
        for entry in entries {
            writeln!(file, "{entry}").unwrap();
        }
    }

    /// The exact `AskUserQuestion` input emitted by `request`.
    fn asked(&self) -> Value {
        json!({"questions":[self.question]})
    }

    fn claude_result(&self, unix_ms: i64, answer: &str, annotations: Value) -> Value {
        let mut entry = claude_entry(
            "user",
            unix_ms,
            json!({"role":"user", "content":[
                {"type":"tool_result", "tool_use_id":CLAUDE_CALL, "content":"The user answered."}
            ]}),
        );
        entry["toolUseResult"] = json!({
            "questions":[self.question],
            "answers":{self.question["question"].as_str().unwrap(): answer},
            "annotations":annotations
        });
        entry
    }

    /// Record the unchanged question and the owner's answer five seconds later.
    fn claude_answer(&self, answered_ms: i64, answer: &str) {
        self.append(&[
            claude_call(CLAUDE_CALL, answered_ms - 5000, &self.asked()),
            self.claude_result(answered_ms, answer, json!({})),
        ]);
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
        self.append(&events);
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

#[test]
fn timestamp_oracle_matches_known_instants() {
    // Verified independently with GNU date.
    assert_eq!(rfc3339(1_794_357_995_969), "2026-11-11T00:46:35.969Z");
    assert_eq!(rfc3339(951_782_400_000), "2000-02-29T00:00:00.000Z");
}

#[test]
fn claude_request_asks_one_bound_ask_user_question() {
    let fixture = Fixture::claude();
    assert_eq!(
        fixture.question,
        json!({
            "question":format!(
                "May I use your desktop for 8 minutes now to {PURPOSE}? Approval request: {}.",
                fixture.id
            ),
            "header":"Desktop",
            "multiSelect":false,
            "options":[
                {"label":YES, "description":"Allow desktop use for this interval, starting when you answer."},
                {"label":"Not now", "description":"Decline. No desktop use."}
            ]
        })
    );
}

#[test]
fn claude_answer_sets_a_stable_deadline_and_closure_survives_restart() {
    let fixture = Fixture::claude();
    let answered = now_ms() - 30_000;
    fixture.claude_answer(answered, YES);
    let first = fixture.status();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    let approved_at = first["approved_at"].as_f64().unwrap();
    assert!((approved_at - answered as f64 / 1000.).abs() < 0.0005);
    assert_eq!(
        first["deadline_unix_seconds"].as_f64().unwrap() - approved_at,
        480.0
    );
    fixture.append(&[claude_entry(
        "assistant",
        now_ms(),
        json!({"role":"assistant", "content":[{"type":"text", "text":"Approved."}]}),
    )]);
    let second: Value = serde_json::from_slice(&fixture.status().stdout).unwrap();
    assert_eq!(first, second);
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
fn expired_claude_answer_cannot_admit_desktop_commands() {
    let fixture = Fixture::claude();
    fixture.claude_answer(now_ms() - 600_000, YES);
    fixture.assert_desktop_refused("approval has expired");
}

#[test]
fn declined_free_text_or_annotated_claude_answers_cannot_admit() {
    for answer in ["Not now", "Yes, but only for two minutes"] {
        let fixture = Fixture::claude();
        fixture.claude_answer(now_ms() - 10_000, answer);
        fixture.assert_desktop_refused("No matching user approval");
    }
    let fixture = Fixture::claude();
    let question = fixture.question["question"].as_str().unwrap();
    fixture.append(&[
        claude_call(CLAUDE_CALL, now_ms() - 15_000, &fixture.asked()),
        fixture.claude_result(
            now_ms() - 10_000,
            YES,
            json!({question: {"notes":"only the left monitor"}}),
        ),
    ]);
    fixture.assert_desktop_refused("Approval answer carries notes");
}

#[test]
fn only_the_unchanged_main_conversation_question_can_be_approved() {
    let answered = now_ms() - 10_000;
    let fixture = Fixture::claude();
    let mut prefilled = fixture.asked();
    prefilled["answers"] = json!({fixture.question["question"].as_str().unwrap(): YES});
    fixture.append(&[
        claude_call(CLAUDE_CALL, answered - 5000, &prefilled),
        fixture.claude_result(answered, YES, json!({})),
    ]);
    fixture.assert_desktop_refused("No matching user approval");

    let fixture = Fixture::claude();
    let mut entries = [
        claude_call(CLAUDE_CALL, answered - 5000, &fixture.asked()),
        fixture.claude_result(answered, YES, json!({})),
    ];
    for entry in &mut entries {
        entry["isSidechain"] = json!(true);
    }
    fixture.append(&entries);
    fixture.assert_desktop_refused("No matching user approval");

    let fixture = Fixture::claude();
    fixture.append(&[claude_call(
        "toolu_earlier",
        answered - 9000,
        &fixture.asked(),
    )]);
    fixture.claude_answer(answered, YES);
    fixture.assert_desktop_refused("Ambiguous repeated approval question");

    let fixture = Fixture::claude();
    let mut call = claude_call(CLAUDE_CALL, answered - 5000, &fixture.asked());
    call["cwd"] = json!(fixture.root.path());
    fixture.append(&[call, fixture.claude_result(answered, YES, json!({}))]);
    fixture.assert_desktop_refused("asked in another repository");
}

#[test]
fn claude_request_requires_the_active_session_of_this_repository() {
    let refused = |fixture: &Fixture, reason: &str| {
        let output = fixture.request();
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(reason), "{error}");
        assert!(!fixture.root.path().join("state").exists());
    };
    let mut fixture = Fixture::claude_transcript(&claude_opening());
    fixture.session.1 = None;
    refused(&fixture, "Request requires CLAUDE_CODE_SESSION_ID");
    fixture.session.1 = Some("another-session".into());
    refused(&fixture, "not the active Claude Code session");

    let mut opening = claude_opening();
    opening[1]["sessionId"] = json!("another-session");
    refused(
        &Fixture::claude_transcript(&opening),
        "belongs to another Claude Code session",
    );

    let fixture = Fixture::claude_transcript(&claude_opening());
    let mut elsewhere = claude_entry("user", now_ms() - 60_000, json!({"role":"user"}));
    elsewhere["cwd"] = json!(fixture.root.path());
    fixture.append(&[elsewhere]);
    refused(&fixture, "belongs to another repository");

    refused(
        &Fixture::claude_transcript(&[json!({"type":"note", "text":"no session"})]),
        "Unrecognized session log",
    );
}

#[test]
fn missing_repeated_or_future_claude_results_cannot_admit() {
    let fixture = Fixture::claude();
    fixture.append(&[claude_call(CLAUDE_CALL, now_ms() - 5000, &fixture.asked())]);
    fixture.assert_desktop_refused("No matching user approval");

    let fixture = Fixture::claude();
    fixture.append(&[
        claude_call(CLAUDE_CALL, now_ms() - 15_000, &fixture.asked()),
        fixture.claude_result(now_ms() - 10_000, "Not now", json!({})),
        fixture.claude_result(now_ms() - 5000, YES, json!({})),
    ]);
    fixture.assert_desktop_refused("Repeated answer to the approval question");

    let fixture = Fixture::claude();
    fixture.append(&[
        claude_call(CLAUDE_CALL, now_ms() - 5000, &fixture.asked()),
        fixture.claude_result(now_ms() + 60_000, YES, json!({})),
    ]);
    fixture.assert_desktop_refused("Approval timestamp is in the future");
}

#[test]
fn admission_rechecks_the_claude_session_identity() {
    let fixture = Fixture::claude();
    fixture.claude_answer(now_ms() - 30_000, YES);
    assert!(fixture.status().status.success());
    let mut foreign = claude_entry("user", now_ms(), json!({"role":"user", "content":"Hi."}));
    foreign["sessionId"] = json!("another-session");
    fixture.append(&[foreign]);
    fixture.assert_desktop_refused("belongs to another Claude Code session");
}

#[test]
fn changed_or_malformed_claude_evidence_cannot_admit() {
    let fixture = Fixture::claude();
    fixture.claude_answer(now_ms() - 30_000, YES);
    assert!(fixture.status().status.success());
    let mut entries: Vec<Value> = fs::read_to_string(&fixture.log)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    entries.last_mut().unwrap()["timestamp"] = json!(rfc3339(now_ms() - 1000));
    fs::write(&fixture.log, "").unwrap();
    fixture.append(&entries);
    fixture.assert_desktop_refused("Approval evidence changed");

    for timestamp in [
        "2026-11-11T00:46:35.969+00:00",
        "2026-11-11 00:46:35.969Z",
        "2026-02-30T00:46:35.969Z",
        "2026-11-11T00:46:35.Z",
    ] {
        let fixture = Fixture::claude();
        let mut result = fixture.claude_result(now_ms() - 10_000, YES, json!({}));
        result["timestamp"] = json!(timestamp);
        fixture.append(&[
            claude_call(CLAUDE_CALL, now_ms() - 15_000, &fixture.asked()),
            result,
        ]);
        fixture.assert_desktop_refused("Invalid approval timestamp");
    }

    let fixture = Fixture::claude();
    fixture.append(&[
        claude_call(CLAUDE_CALL, now_ms() - 10_000, &fixture.asked()),
        fixture.claude_result(now_ms() - 15_000, YES, json!({})),
    ]);
    fixture.assert_desktop_refused("Approval answer precedes its question");
}
