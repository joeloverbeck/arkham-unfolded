//! Synthetic approval source for headless process fixtures only.
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn grant(command: &mut Command, parent: &Path) {
    let directory = tempfile::Builder::new()
        .prefix("approval-fixture-")
        .tempdir_in(parent)
        .unwrap()
        .keep();
    let source = directory.join("session.jsonl");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::write(
        &source,
        format!(
            "{}\n",
            json!({"type":"session_meta", "payload":{"id":"headless-fixture", "cwd":repository}})
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .args(["native", "approval", "request", "--session-log"])
        .arg(&source)
        .args([
            "--minutes",
            "8",
            "--purpose",
            "exercise the headless process fixture",
        ])
        .env("CODEX_THREAD_ID", "headless-fixture")
        .env("XDG_STATE_HOME", &directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let request: Value = serde_json::from_slice(&output.stdout).unwrap();
    let question = &request["request"]["questions"][0];
    let reply = format!(
        "<send_user_message_question_reply>\n{}\n</send_user_message_question_reply>",
        json!([{
            "answer":question["options"][0], "question":question["title"],
            "questionItemId":"[\"request_user_input_async\",\"fixture-call\",0]"
        }])
    );
    let events = [
        json!({"type":"response_item","payload":{"type":"function_call","name":"request_user_input_async","call_id":"fixture-call","arguments":serde_json::to_string(&request["request"]).unwrap()}}),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":reply}],"internal_chat_message_metadata_passthrough":{"create_time":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64(),"content_item_metadata":[{"provenance":{"type":"user"}}]}}}),
    ];
    let mut file = fs::OpenOptions::new().append(true).open(source).unwrap();
    for event in events {
        writeln!(file, "{event}").unwrap();
    }
    command
        .args(["--approval", request["approval"].as_str().unwrap()])
        .env("XDG_STATE_HOME", directory);
}
