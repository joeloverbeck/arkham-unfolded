//! `arkham-partner`'s contract against a minimal listener standing in for the
//! table's partner socket (#205): the request line each subcommand sends, the answer
//! printed unchanged, the exit codes, and argument refusals; and the partner folder
//! (Cuba Libre #202).

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::Path,
    process::{Command, Output},
    thread,
};
use unfolded_agent_players::folder::CODEX_WAIT;

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
    Command::new(env!("CARGO_BIN_EXE_arkham-partner"))
        .arg("--socket")
        .arg(socket)
        .args(args)
        .env_remove("ARKHAM_PARTNER_SOCKET")
        // Whatever tool runs this test.
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("CODEX_THREAD_ID")
        .output()
        .unwrap()
}

/// Cuba Libre #201, its correction 18: a join carries the identifiers the tool gives its
/// session's commands, and no other request does.
#[test]
fn a_join_carries_the_tools_identifiers_of_the_session() {
    for (variable, value) in [
        ("CODEX_THREAD_ID", "019e-thread"),
        ("CLAUDE_CODE_SESSION_ID", "3f2b-session"),
    ] {
        for (args, joins) in [(&["join", "patrice"][..], true), (&["wait", "a1b2"], false)] {
            let directory = tempfile::tempdir().unwrap();
            let socket = directory.path().join("partners.sock");
            let table = listen(&socket, JOINED);
            let output = Command::new(env!("CARGO_BIN_EXE_arkham-partner"))
                .arg("--socket")
                .arg(&socket)
                .args(args)
                .env_remove("CLAUDE_CODE_SESSION_ID")
                .env_remove("CODEX_THREAD_ID")
                .env(variable, value)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
            let sent: serde_json::Value = serde_json::from_str(&table.join().unwrap()).unwrap();
            let expected = serde_json::json!(format!("{variable}={value}"));
            assert_eq!(sent.get("session"), joins.then_some(&expected), "{sent}");
        }
    }
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
    let (sent, output) = exchange(&["join", "patrice"], JOINED);
    assert_eq!(
        sent,
        serde_json::json!({"op": "join", "investigators": ["patrice"]})
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{JOINED}\n")
    );

    let cases: [(&[&str], serde_json::Value); 16] = [
        // The table refuses these two, naming the investigators open (#295).
        (
            &["join"],
            serde_json::json!({"op": "join", "investigators": []}),
        ),
        (
            &["join", "patrice", "jacqueline"],
            serde_json::json!({"op": "join", "investigators": ["patrice", "jacqueline"]}),
        ),
        (
            &["join", "patrice", "--partner", "a1b2"],
            serde_json::json!({"op": "join", "investigators": ["patrice"], "partner": "a1b2"}),
        ),
        (
            &["wait", "a1b2"],
            serde_json::json!({"op": "wait", "partner": "a1b2"}),
        ),
        (
            &["wait", "a1b2", "--timeout", "30"],
            serde_json::json!({"op": "wait", "partner": "a1b2", "timeout_seconds": 30}),
        ),
        (
            &["wait", "a1b2", "--timeout", "18446744073709551615"],
            serde_json::json!({"op": "wait", "partner": "a1b2", "timeout_seconds": u64::MAX}),
        ),
        (
            &["wait", "a1b2", "--listen"],
            serde_json::json!({"op": "wait", "partner": "a1b2", "listen": true}),
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
            &["ready", "a1b2", "17"],
            serde_json::json!({"op": "ready", "partner": "a1b2", "revision": 17}),
        ),
        (
            &["choose", "a1b2", "17", "turn:end", "--wait"],
            serde_json::json!({"op": "choose", "partner": "a1b2", "revision": 17, "id": "turn:end", "wait": true}),
        ),
        (
            &[
                "choose",
                "a1b2",
                "17",
                "turn:end",
                "--wait",
                "--timeout",
                "900",
            ],
            serde_json::json!({"op": "choose", "partner": "a1b2", "revision": 17, "id": "turn:end", "wait": true, "timeout_seconds": 900}),
        ),
        (
            &["ready", "a1b2", "17", "--wait"],
            serde_json::json!({"op": "ready", "partner": "a1b2", "revision": 17, "wait": true}),
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
        // A wait its session replaced with a newer one.
        (r#"{"ok":true,"status":"replaced"}"#, 0),
        // Review fix G1: a held choice's outcome is news, not a refusal.
        (r#"{"ok":true,"status":"chosen"}"#, 0),
        (r#"{"ok":true,"status":"released","reason":"moved on"}"#, 0),
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

/// Review fix G1: a choice the turn's player window holds is answered `held` at
/// once, which the command prints and exits 0 for. How its outcome arrives is the
/// partner instructions' to say (tested below, #218). So is one that waits for the
/// table, answered `settling` (Cuba Libre #201).
#[test]
fn a_held_choice_exits_0() {
    for answer in [
        r#"{"ok":true,"status":"held"}"#,
        r#"{"ok":true,"status":"settling"}"#,
    ] {
        let (_, output) = exchange(&["choose", "a1b2", "17", "turn:end"], answer);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
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
        &["choose", "a1b2", "17", "turn:end", "--timeout", "30"],
        &["ready", "a1b2", "17", "--wait", "--timeout", "0"],
        &["join", "roland"],
        &["choose", "a1b2", "seventeen", "turn:end"],
        &["choose", "a1b2", "17"],
        &["observe"],
        &["observe", "a1b2", "--all-cards"],
        &["inspect", "a1b2"],
        &["ready", "a1b2"],
        &["ready", "a1b2", "seventeen"],
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
    let output = partner(&socket, &["join", "patrice"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains(&socket.display().to_string()), "{error}");
    assert!(error.contains("Agent partners"), "{error}");
}

/// #218: help lists the commands and points to the partner instructions, which
/// alone state the protocol's rules: what a wait answers and when, which choices
/// are held, table talk's limits and the exit codes.
#[test]
fn the_default_socket_follows_the_environment_and_help_lists_the_commands() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("elsewhere.sock");
    let table = listen(&socket, JOINED);
    let output = Command::new(env!("CARGO_BIN_EXE_arkham-partner"))
        .args(["join", "patrice"])
        .env("ARKHAM_PARTNER_SOCKET", &socket)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let sent: serde_json::Value = serde_json::from_str(&table.join().unwrap()).unwrap();
    assert_eq!(sent["op"], "join");

    let help = help();
    for step in [
        "the partner instructions",
        "arkham-partner join INVESTIGATOR",
        "arkham-partner wait PARTNER",
        "wait PARTNER --listen",
        "arkham-partner choose PARTNER REVISION ID",
        "arkham-partner choose PARTNER REVISION ID --wait",
        "arkham-partner inspect PARTNER CODE",
        "arkham-partner ready PARTNER REVISION",
        "observe PARTNER --full",
    ] {
        assert!(help.contains(step), "{step} in {help}");
    }
    for rule in [
        "\"held\"",
        "\"released\"",
        "three messages",
        "six in an exchange",
        "Exit codes",
    ] {
        assert!(
            !help.contains(rule),
            "{rule} is the instructions' to state, not help's: {help}"
        );
    }
}

/// The partner instructions, which the partner folder holds and the pasted line and
/// `--help` point to (#211, Cuba Libre #202).
const INSTRUCTIONS: &str = "docs/agents/partner-instructions.md";

fn help() -> String {
    let help = Command::new(env!("CARGO_BIN_EXE_arkham-partner"))
        .arg("--help")
        .output()
        .unwrap();
    String::from_utf8(help.stdout).unwrap()
}

/// #211: the instructions describe every partner request and every status the
/// table answers with (`src/partners.rs`), the waiting loop of Claude Code and of
/// Codex, table talk's limits (#209), and notes kept against context compaction.
#[test]
fn the_partner_instructions_describe_every_request_status_and_waiting_loop() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let instructions = std::fs::read_to_string(repository.join(INSTRUCTIONS)).unwrap();
    let requests = [
        "arkham-partner join INVESTIGATOR",
        "join patrice",
        "join patrice --partner PARTNER",
        "arkham-partner wait PARTNER",
        "wait PARTNER --listen",
        "arkham-partner observe PARTNER",
        "observe PARTNER --full",
        "arkham-partner inspect PARTNER CODE",
        "arkham-partner choose PARTNER REVISION ID",
        "choose PARTNER REVISION ID --wait",
        "ready PARTNER REVISION --wait",
        "arkham-partner ready PARTNER REVISION",
        "arkham-partner say PARTNER MESSAGE",
        "arkham-partner leave PARTNER",
        "decision.undo",
        "before_action_of",
        "before_decision_of",
        "waits_for_passes",
        "--socket",
    ];
    let statuses = [
        "joined",
        "decision",
        "group_choice",
        "shared",
        "table_talk",
        "closed",
        "nothing_yet",
        "observed",
        "inspected",
        "chosen",
        "held",
        "released",
        "settling",
        "replaced",
        "ready",
        "said",
        "left",
        "refused",
    ];
    let waiting = [
        "Claude Code",
        "run_in_background",
        "Codex",
        "without a time limit",
        "without a timeout",
        "{CODEX_WAIT}",
    ];
    let talk = [
        "280 characters",
        "three messages in a row",
        "six messages",
        "peril",
    ];
    let notes = ["compacted", "notes/PARTNER.md"];
    // #218: what help and the in-protocol hints no longer restate.
    let moved = [
        "`changes`",
        "pass by marking yourself ready",
        "exactly the requirement",
        "no partner plays yet is waited on too",
        "## Exit codes",
    ];
    for taught in requests
        .into_iter()
        .chain(waiting)
        .chain(talk)
        .chain(notes)
        .chain(moved)
    {
        assert!(instructions.contains(taught), "{taught} in {INSTRUCTIONS}");
    }
    for status in statuses {
        assert!(
            instructions.contains(&format!("`{status}`")),
            "status {status} in {INSTRUCTIONS}"
        );
    }
}

/// #209: `say` sends one message, its words joined as the player typed them, and a
/// refused message exits 1; help names the command. Table talk's limits are the
/// partner instructions' alone (tested above, #211, #218).
#[test]
fn say_sends_one_message_and_help_names_it() {
    let (sent, output) = exchange(
        &["say", "a1b2", "I'll", "take the", "Attic."],
        r#"{"ok":true,"status":"said"}"#,
    );
    assert_eq!(
        sent,
        serde_json::json!({"op": "say", "partner": "a1b2", "text": "I'll take the Attic."})
    );
    assert!(output.status.success(), "{output:?}");
    let (_, output) = exchange(
        &["say", "a1b2", "Again."],
        r#"{"ok":false,"status":"refused","error":"chatter limit"}"#,
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");

    let directory = tempfile::tempdir().unwrap();
    let output = partner(&directory.path().join("partners.sock"), &["say", "a1b2"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "A message is required: {output:?}"
    );

    let help = help();
    assert!(
        help.contains("arkham-partner say PARTNER MESSAGE"),
        "{help}"
    );
}

/// The partner folders written under `data`, or what refused them.
fn partner_folders(data: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .args(["partner-folder", "--built"])
        .arg(env!("CARGO_BIN_EXE_arkham-partner"))
        .env("XDG_DATA_HOME", data)
        .output()
        .unwrap()
}

/// Cuba Libre #202: each tool has a partner folder outside the checkout, which holds the
/// partner instructions, naming the installed client program by its path and no other
/// way, and that tool's permission for that program, which lies outside the folders and
/// runs. Neither folder holds a file the other tool reads.
#[test]
fn each_tool_has_a_partner_folder_and_the_permission_for_the_installed_client() {
    let data = tempfile::tempdir().unwrap();
    let output = partner_folders(data.path());
    assert!(output.status.success(), "{output:?}");
    let root = data.path().join("arkham-unfolded");
    let client = root.join("client/arkham-partner");
    let path = client.to_str().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let instructions = std::fs::read_to_string(repository.join(INSTRUCTIONS)).unwrap();
    for (file, permission, allowed, other) in [
        (
            "claude-code/CLAUDE.md",
            "claude-code/.claude/settings.json",
            format!("Bash({path}:*)"),
            ["claude-code/AGENTS.md", "claude-code/.codex"],
        ),
        (
            "codex/AGENTS.md",
            "codex/.codex/rules/player.rules",
            format!("[\"{path}\"]"),
            ["codex/CLAUDE.md", "codex/.claude"],
        ),
    ] {
        let read = |name: &str| std::fs::read_to_string(root.join("partner").join(name)).unwrap();
        let written = read(file);
        // #306: each tool's folder is named by its full path, in both folders.
        let claude_code = root.join("partner/claude-code");
        let codex = root.join("partner/codex");
        for folder in [&claude_code, &codex] {
            assert!(
                written.contains(&format!("`{}`", folder.display())),
                "{file} names {folder:?}"
            );
        }
        assert_eq!(
            written
                .replace(path, "arkham-partner")
                .replace(&claude_code.display().to_string(), "{CLAUDE_CODE_FOLDER}")
                .replace(&codex.display().to_string(), "{CODEX_FOLDER}")
                .replace(CODEX_WAIT, "{CODEX_WAIT}"),
            instructions
        );
        assert!(!written.replace(path, "").contains("arkham-partner"));
        assert!(written.contains(CODEX_WAIT));
        assert!(!written.contains("](../"), "No link leaves the folder");
        assert!(read(permission).contains(&allowed));
        for other in other {
            assert!(!root.join("partner").join(other).exists(), "{other}");
        }
    }
    let help = Command::new(&client).arg("--help").output().unwrap();
    assert!(help.status.success(), "{help:?}");
    assert!(String::from_utf8(help.stdout).unwrap().contains("wait"));

    let planted = root.join("partner/codex/.mcp.json");
    std::fs::write(&planted, "{}").unwrap();
    let output = partner_folders(data.path());
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains(planted.to_str().unwrap()), "{error}");
    std::fs::remove_file(planted).unwrap();
    assert!(partner_folders(data.path()).status.success());
}
