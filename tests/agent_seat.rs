#![cfg(feature = "tabletop")]
//! The agent seat's startup contract through the real executable (#153): a startup
//! problem is one refused response on standard output, the same text on standard
//! error and an unsuccessful exit. It is never a dialog and never the scenario
//! selection window. The held session's start (#175) keeps that contract and leaves
//! no socket or seat behind it. The seated table itself is covered by the protocol
//! tests beside the seat and by the ignored real-artwork runs, the held one at the end
//! of this file.

use serde_json::Value;
use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Output, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
#[path = "support/native_process.rs"]
mod native_process;

fn launch(directory: &Path, arguments: &[&str]) -> Output {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_arkham-tabletop"));
    command.args(arguments);
    native_process::isolate(&mut command, directory);
    native_process::output(&mut command)
}

/// The single refused response, after checking the exit and standard error.
fn refusal(output: &Output) -> String {
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "one response expected: {stdout}");
    let response: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(response["ok"], false);
    assert_eq!(response["mode"], "seat");
    assert_eq!(response["status"], "refused");
    let reason = response["error"].as_str().unwrap().to_owned();
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&reason),
        "the refusal must also reach standard error"
    );
    reason
}

#[test]
fn an_absent_collection_is_one_refused_response_and_an_unsuccessful_exit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("artwork");
    for existing in [false, true] {
        let save = directory.path().join(if existing {
            "existing.json"
        } else {
            "fresh.json"
        });
        let original = b"existing personal save whose bytes must not be inspected or repaired";
        if existing {
            fs::write(&save, original).unwrap();
        }
        let reason = refusal(&launch(
            directory.path(),
            &[
                "--experiment",
                "gathering",
                "--agent-seat",
                "--art-root",
                root.to_str().unwrap(),
                "--save",
                save.to_str().unwrap(),
            ],
        ));
        assert!(reason.contains("cannot start play"), "{reason}");
        assert!(reason.contains(root.to_str().unwrap()), "{reason}");
        assert!(
            reason.contains("Absent or wrong prepared collection structure"),
            "{reason}"
        );
        if existing {
            assert_eq!(fs::read(&save).unwrap(), original);
        } else {
            assert!(!save.exists());
        }
        assert!(!directory.path().join("config").exists());
    }
}

#[test]
fn refused_options_are_text_and_never_scenario_selection() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let save = save.to_str().unwrap();
    let socket = directory.path().join("held.sock");
    let socket = socket.to_str().unwrap();
    for (arguments, expected) in [
        // Native play returns a refused `--attempt` launch to scenario selection.
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--attempt",
                "new",
            ],
            "--agent-seat cannot be combined with --attempt",
        ),
        // Without the direct launch the executable would open scenario selection.
        (
            vec!["--agent-seat", "--save", save],
            "add --experiment gathering",
        ),
        (
            vec!["--experiment", "gathering", "--agent-seat"],
            "--agent-seat requires an explicit --save",
        ),
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--screenshot",
                "table.png",
            ],
            "--agent-seat cannot be combined with --attempt or --screenshot",
        ),
        // The flag swallowed as another option's value still never opens a window.
        (
            vec!["--experiment", "gathering", "--save", "--agent-seat"],
            "--agent-seat is an option of its own",
        ),
        // A held session is the agent seat's, with or without the direct launch.
        (
            vec![
                "--experiment",
                "gathering",
                "--save",
                save,
                "--socket",
                socket,
            ],
            "--socket holds an agent seat session: add --agent-seat",
        ),
        (
            vec!["--socket", socket],
            "--socket holds an agent seat session: add --agent-seat",
        ),
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--socket",
                "seat.sock",
            ],
            "--socket needs an absolute path, not seat.sock",
        ),
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--socket",
                socket,
            ],
            "--agent-seat requires an explicit --save",
        ),
        // The idle end belongs to a held session, with or without the direct launch.
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--idle-minutes",
                "5",
            ],
            "--idle-minutes ends a held session after that many minutes without a request: add --socket",
        ),
        (
            vec!["--idle-minutes", "5"],
            "--idle-minutes ends a held session after that many minutes without a request: add --socket",
        ),
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--socket",
                socket,
                "--idle-minutes",
                "0",
            ],
            "--idle-minutes needs a whole number of minutes from 1 to 1440, not 0",
        ),
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--socket",
                socket,
                "--idle-minutes",
                "1441",
            ],
            "--idle-minutes needs a whole number of minutes from 1 to 1440, not 1441",
        ),
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--socket",
                socket,
                "--idle-minutes",
            ],
            "--idle-minutes needs a whole number of minutes from 1 to 1440",
        ),
    ] {
        let reason = refusal(&launch(directory.path(), &arguments));
        assert!(reason.contains(expected), "{reason}");
        assert!(!Path::new(save).exists());
        assert!(!directory.path().join("local").exists());
        assert!(!Path::new(socket).exists());
        assert!(!directory.path().join("seat.sock").exists());
    }
}

/// Processes whose arguments name `socket`: a seat started on it.
fn seats_on(socket: &Path) -> Vec<String> {
    let wanted = socket.as_os_str().as_encoded_bytes();
    fs::read_dir("/proc")
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let arguments = fs::read(entry.path().join("cmdline")).ok()?;
            arguments
                .split(|byte| *byte == 0)
                .any(|argument| argument == wanted)
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect()
}

#[test]
fn a_held_start_that_cannot_admit_is_refused_and_leaves_no_socket_or_seat() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("artwork");
    let save = directory.path().join("attempt.json");
    let socket = directory.path().join("seat.sock");
    let reason = refusal(&launch(
        directory.path(),
        &[
            "--experiment",
            "gathering",
            "--agent-seat",
            "--art-root",
            root.to_str().unwrap(),
            "--save",
            save.to_str().unwrap(),
            "--socket",
            socket.to_str().unwrap(),
        ],
    ));
    assert!(reason.contains("cannot start play"), "{reason}");
    assert!(
        reason.contains("Absent or wrong prepared collection structure"),
        "{reason}"
    );
    assert!(!socket.exists(), "no socket is left");
    assert_eq!(
        seats_on(&socket),
        Vec::<String>::new(),
        "no seat is left running"
    );
    assert!(!save.exists());
    // The started seat's diagnostics went to the log beside the socket.
    let log = fs::read_to_string(directory.path().join("seat.sock.log")).unwrap();
    assert!(log.contains(&reason), "{log}");
}

/// Run `command` as an agent harness does: its standard output and error are pipes, and
/// it is finished only when both reach their end. A detached seat holding either would
/// keep the start command from finishing.
fn finish(command: &mut Command, limit: Duration) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let read = |mut pipe: Box<dyn Read + Send>| {
        let (done, ended) = mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).unwrap();
            done.send(bytes).unwrap();
        });
        ended
    };
    let stdout = read(Box::new(child.stdout.take().unwrap()));
    let stderr = read(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + limit;
    let mut end = |pipe: mpsc::Receiver<Vec<u8>>| {
        pipe.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_else(|_| {
                let _ = child.kill();
                panic!("the command's output did not end within {limit:?}")
            })
    };
    let (stdout, stderr) = (end(stdout), end(stderr));
    Output {
        status: child.wait().unwrap(),
        stdout,
        stderr,
    }
}

/// Ends a held seat that a failed assertion left running: quit over its socket.
struct Held<'a>(&'a Path);

impl Drop for Held<'_> {
    fn drop(&mut self) {
        use std::io::Write;
        if let Ok(mut last) = std::os::unix::net::UnixStream::connect(self.0) {
            let _ = writeln!(last, r#"{{"op":"quit"}}"#);
            let _ = last.read_to_end(&mut Vec::new());
        }
    }
}

fn tool(socket: &Path, requests: &[&str]) -> (Output, Vec<Value>) {
    let output = Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/tools"))
        .arg("seat")
        .arg(socket)
        .args(requests)
        .env("CARGO_NET_OFFLINE", "true")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let responses = String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (output, responses)
}

/// The one response a successful start command printed.
fn started(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    let lines: Vec<&str> = std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .collect();
    assert_eq!(lines.len(), 1, "one response: {lines:?}");
    serde_json::from_str(lines[0]).unwrap()
}

/// The held session at the real table, with real artwork and a real renderer: the start
/// command returns once admitted, separate tool commands share one table, and `quit`
/// ends the seat. A later start resumes the attempt, a second seat on its path is
/// refused, and the seat ends itself without requests. Run on the owner's machine after
/// preparing the collection.
#[test]
#[ignore = "requires ARKHAM_GATHERING_TEST_ASSETS pointing at a complete prepared collection and a graphics adapter"]
fn the_real_table_is_held_between_tool_commands_until_quit() {
    let root = std::env::var("ARKHAM_GATHERING_TEST_ASSETS")
        .expect("set ARKHAM_GATHERING_TEST_ASSETS to the prepared collection");
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let socket = directory.path().join("seat.sock");
    let start = |extra: &[&str], limit| {
        finish(
            Command::new(env!("CARGO_BIN_EXE_arkham-tabletop"))
                .args(["--experiment", "gathering", "--agent-seat", "--save"])
                .arg(&save)
                .arg("--art-root")
                .arg(&root)
                .arg("--socket")
                .arg(&socket)
                .args(extra)
                .current_dir(directory.path())
                .env("XDG_CONFIG_HOME", directory.path().join("config"))
                .env("XDG_DATA_HOME", directory.path().join("data")),
            limit,
        )
    };
    let first = start(&[], Duration::from_secs(240));
    let _held = Held(&socket);
    let ready = started(&first);
    assert_eq!(ready["status"], "ready", "{ready}");
    assert!(ready["adapter"]["name"].is_string(), "{}", ready["adapter"]);
    assert_eq!(ready["table"]["settled"], true, "{}", ready["table"]);
    assert_eq!(
        seats_on(&socket).len(),
        1,
        "the seat runs on after its start"
    );

    // One command opens Tony's menu through mesh picking; the next still finds it open.
    let (selected, responses) = tool(
        &socket,
        &[r#"{"op":"activate","revision":0,"component":"investigator:tony"}"#],
    );
    assert!(selected.status.success(), "{selected:?}");
    assert_eq!(responses[0]["status"], "activated", "{}", responses[0]);
    let menu = responses[0]["table"]["menu"].clone();
    assert_eq!(menu["component"], "investigator:tony", "{menu}");
    let (chosen, responses) = tool(
        &socket,
        &[
            r#"{"op":"observe"}"#,
            r#"{"op":"activate","revision":0,"choice":"lead:tony"}"#,
            r#"{"op":"settle"}"#,
        ],
    );
    assert!(chosen.status.success(), "{chosen:?}");
    assert_eq!(responses[0]["table"]["menu"], menu);
    assert_eq!(responses[1]["status"], "activated", "{}", responses[1]);
    assert_eq!(responses[2]["status"], "settled", "{}", responses[2]);
    assert_eq!(responses[2]["observation"]["lead"], "tony");
    assert!(save.exists(), "the committed choice autosaved");

    // A refusal fails the command, and the session continues.
    let (stale, responses) = tool(
        &socket,
        &[r#"{"op":"activate","revision":0,"choice":"lead:tony"}"#],
    );
    assert_eq!(stale.status.code(), Some(1), "{stale:?}");
    assert_eq!(responses[0]["status"], "refused");

    let (quit, responses) = tool(&socket, &[r#"{"op":"quit"}"#]);
    assert!(quit.status.success(), "{quit:?}");
    assert_eq!(responses[0]["status"], "bye");
    assert_eq!(responses[0]["observation"]["revision"], 1);
    assert!(!socket.exists(), "quit removes the socket");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !seats_on(&socket).is_empty() {
        assert!(Instant::now() < deadline, "the seat did not end after quit");
        std::thread::sleep(Duration::from_millis(100));
    }
    let log = fs::read_to_string(directory.path().join("seat.sock.log")).unwrap();
    assert!(log.contains("Artwork root:"), "{log}");
    assert!(log.contains("The agent seat is listening on"), "{log}");
    assert!(log.contains("The agent seat's session ended."), "{log}");

    // A start on the same path resumes the attempt. A second start there is refused
    // while the seat runs on; without requests the seat ends itself as quit does.
    let restarted = start(&["--idle-minutes", "1"], Duration::from_secs(240));
    let _restarted = Held(&socket);
    let ready = started(&restarted);
    assert_eq!(ready["status"], "ready", "{ready}");
    assert_eq!(ready["observation"]["revision"], 1, "the attempt resumed");
    let reason = refusal(&start(&[], Duration::from_secs(60)));
    assert!(reason.contains("already answers"), "{reason}");
    assert_eq!(seats_on(&socket).len(), 1, "the first seat runs on alone");
    let (observed, responses) = tool(&socket, &[r#"{"op":"observe"}"#]);
    let asked = Instant::now();
    assert!(observed.status.success(), "{observed:?}");
    assert_eq!(responses[0]["observation"]["revision"], 1);
    let attempt = fs::read(&save).unwrap();
    let deadline = asked + Duration::from_secs(120);
    while !seats_on(&socket).is_empty() {
        assert!(Instant::now() < deadline, "the idle seat did not end");
        std::thread::sleep(Duration::from_millis(500));
    }
    assert!(
        asked.elapsed() >= Duration::from_secs(55),
        "the idle period counts from the last request: {:?}",
        asked.elapsed()
    );
    assert!(!socket.exists(), "the idle end removes the socket");
    assert_eq!(
        fs::read(&save).unwrap(),
        attempt,
        "the attempt is as it was"
    );
    let log = fs::read_to_string(directory.path().join("seat.sock.log")).unwrap();
    assert!(
        log.contains("Artwork root:"),
        "the refused start left the log: {log}"
    );
    assert!(log.contains("no request arrived for 1 minute"), "{log}");
    assert!(log.contains("The agent seat's session ended."), "{log}");
}
