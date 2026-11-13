//! The `partner` command's contract against a minimal listener standing in for the
//! table's partner socket (#205): the request line each subcommand sends, the answer
//! printed unchanged, the exit codes, and argument refusals.

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::Path,
    process::{Command, Output},
    thread,
};

/// A stand-in table answering one connection's one request with one canned line.
fn listen(socket: &Path, answer: &'static str) -> thread::JoinHandle<String> {
    let listener = UnixListener::bind(socket).unwrap();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        BufReader::new(&stream).read_line(&mut request).unwrap();
        (&stream)
            .write_all(format!("{answer}\n").as_bytes())
            .unwrap();
        request.trim_end().to_owned()
    })
}

fn partner(socket: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .arg("partner")
        .arg("--socket")
        .arg(socket)
        .args(args)
        .env_remove("ARKHAM_PARTNER_SOCKET")
        .output()
        .unwrap()
}

/// What the stand-in received, as JSON, and the command's output.
fn exchange(args: &[&str], answer: &'static str) -> (serde_json::Value, Output) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("partners.sock");
    let table = listen(&socket, answer);
    let output = partner(&socket, args);
    let received = table.join().unwrap();
    (serde_json::from_str(&received).unwrap(), output)
}

// Spacing and field order the tool must not normalise.
const JOINED: &str = r#"{"ok":true,  "status":"joined","mode":"partner","partner":"a1b2"}"#;

#[test]
fn each_subcommand_sends_its_protocol_request_and_prints_the_answer_unchanged() {
    let (sent, output) = exchange(&["join"], JOINED);
    assert_eq!(sent, serde_json::json!({"op": "join", "investigators": []}));
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{JOINED}\n")
    );

    let cases: [(&[&str], serde_json::Value); 8] = [
        (
            &["join", "patrice", "jacqueline"],
            serde_json::json!({"op": "join", "investigators": ["patrice", "jacqueline"]}),
        ),
        (
            &["wait", "a1b2"],
            serde_json::json!({"op": "wait", "partner": "a1b2", "timeout_seconds": 240}),
        ),
        (
            &["wait", "a1b2", "--timeout", "30"],
            serde_json::json!({"op": "wait", "partner": "a1b2", "timeout_seconds": 30}),
        ),
        (
            &["observe", "a1b2"],
            serde_json::json!({"op": "observe", "partner": "a1b2"}),
        ),
        (
            &["observe", "a1b2", "--full"],
            serde_json::json!({"op": "observe", "partner": "a1b2", "full": true}),
        ),
        (
            &["inspect", "a1b2", "01113"],
            serde_json::json!({"op": "inspect", "partner": "a1b2", "card": "01113"}),
        ),
        (
            &["choose", "a1b2", "17", "action:ordinary:resource"],
            serde_json::json!({"op": "choose", "partner": "a1b2", "revision": 17, "id": "action:ordinary:resource"}),
        ),
        (
            &["leave", "a1b2"],
            serde_json::json!({"op": "leave", "partner": "a1b2"}),
        ),
    ];
    for (args, expected) in cases {
        let (sent, output) = exchange(args, r#"{"ok":true,"status":"observed"}"#);
        assert_eq!(sent, expected, "{args:?}");
        assert!(output.status.success(), "{args:?}: {output:?}");
    }
}

#[test]
fn a_refusal_exits_1_and_a_wait_with_nothing_yet_exits_3_after_printing_the_answer() {
    for (answer, code) in [
        (r#"{"ok":false,"status":"refused","error":"stale"}"#, 1),
        (r#"{"ok":true,"status":"nothing_yet"}"#, 3),
        (r#"{"ok":true,"status":"decision"}"#, 0),
        (r#"{"ok":true,"status":"closed"}"#, 0),
        ("not json", 1),
    ] {
        let (_, output) = exchange(&["wait", "a1b2", "--timeout", "1"], answer);
        assert_eq!(output.status.code(), Some(code), "{answer}: {output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{answer}\n")
        );
    }
}

#[test]
fn malformed_arguments_are_refused_before_connecting() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("partners.sock");
    for args in [
        &["wait", "a1b2", "--timeout", "0"][..],
        &["wait", "a1b2", "--timeout", "591"],
        &["join", "roland"],
        &["choose", "a1b2", "seventeen", "turn:end"],
        &["choose", "a1b2", "17"],
        &["observe"],
        &["observe", "a1b2", "--all-cards"],
        &["inspect", "a1b2"],
        &["save", "a1b2"],
    ] {
        let output = partner(&socket, args);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn no_table_listening_names_the_path_and_says_how_partners_are_enabled() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("partners.sock");
    let output = partner(&socket, &["join"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains(&socket.display().to_string()), "{error}");
    assert!(error.contains("Agent partners"), "{error}");
}

#[test]
fn the_default_socket_follows_the_environment_and_help_teaches_the_loop() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("elsewhere.sock");
    let table = listen(&socket, JOINED);
    let output = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .args(["partner", "join"])
        .env("ARKHAM_PARTNER_SOCKET", &socket)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let sent: serde_json::Value = serde_json::from_str(&table.join().unwrap()).unwrap();
    assert_eq!(sent["op"], "join");

    let help = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .args(["partner", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    for step in [
        "partner join",
        "partner wait PARTNER",
        "partner choose PARTNER REVISION ID",
        "partner inspect PARTNER CODE",
        "observe PARTNER --full",
        "changes",
        "nothing_yet",
        "Exit codes",
    ] {
        assert!(help.contains(step), "{step} in {help}");
    }
}
