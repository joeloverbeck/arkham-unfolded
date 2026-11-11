//! The `seat` command's contract against a minimal listener standing in for a held
//! agent seat: what crosses the socket, what reaches standard output, and the exit.

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::Path,
    process::{Command, Output, Stdio},
    thread,
};

/// A stand-in seat answering one connection with one canned line per request line.
struct Listening<'a> {
    socket: &'a Path,
    answering: thread::JoinHandle<Vec<String>>,
}

fn listen<'a>(socket: &'a Path, answers: Vec<&'static str>) -> Listening<'a> {
    let listener = UnixListener::bind(socket).unwrap();
    let answering = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut received = Vec::new();
        let mut answers = answers.into_iter();
        for line in BufReader::new(&stream).lines() {
            received.push(line.unwrap());
            let Some(answer) = answers.next() else { break };
            (&stream)
                .write_all(format!("{answer}\n").as_bytes())
                .unwrap();
        }
        received
    });
    Listening { socket, answering }
}

impl Listening<'_> {
    /// The request lines exactly as they arrived, once the command has finished. A
    /// command that never connected leaves an empty list, not a waiting listener.
    fn received(self) -> Vec<String> {
        if !self.answering.is_finished() {
            let _ = std::os::unix::net::UnixStream::connect(self.socket);
        }
        self.answering.join().unwrap()
    }
}

fn seat(socket: &Path, requests: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tools"));
    command
        .arg("seat")
        .arg(socket)
        .args(requests)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(stdin.unwrap_or("").as_bytes()).unwrap();
    drop(input);
    child.wait_with_output().unwrap()
}

const OBSERVE: &str = r#"{"op":"observe"}"#;
const SETTLE: &str = r#"{"op": "settle", "steps": 3}"#;
// Spacing and field order the tool must not normalise.
const OBSERVED: &str = r#"{"ok":true,  "status":"observed","mode":"seat"}"#;
const SETTLED: &str = r#"{"status": "settled", "ok": true, "steps": 3}"#;

#[test]
fn requests_cross_one_connection_and_responses_print_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");

    let listener = listen(&socket, vec![OBSERVED, SETTLED]);
    let output = seat(&socket, &[OBSERVE, SETTLE], None);
    assert_eq!(listener.received(), [OBSERVE, SETTLE]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{OBSERVED}\n{SETTLED}\n")
    );

    // The same requests as standard-input lines; a blank line is not a request.
    std::fs::remove_file(&socket).unwrap();
    let listener = listen(&socket, vec![OBSERVED, SETTLED]);
    let output = seat(&socket, &[], Some(&format!("{OBSERVE}\n\n{SETTLE}\n")));
    assert_eq!(listener.received(), [OBSERVE, SETTLE]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{OBSERVED}\n{SETTLED}\n")
    );
}

#[test]
fn a_refused_skipped_or_missing_response_fails_the_command_after_printing_the_rest() {
    let refused = r#"{"ok":false,"status":"refused","error":"Outdated"}"#;
    let skipped = r#"{"ok":false,"status":"skipped","error":"after a refusal"}"#;
    for answers in [
        vec![refused, OBSERVED],
        vec![OBSERVED, skipped],
        // The session ended before answering the second request.
        vec![OBSERVED],
    ] {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("seat.sock");
        let expected: String = answers.iter().map(|answer| format!("{answer}\n")).collect();
        let listener = listen(&socket, answers);
        let output = seat(&socket, &[OBSERVE, OBSERVE], None);
        listener.received();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
fn no_listening_seat_names_the_path_and_says_to_start_one() {
    let directory = tempfile::tempdir().unwrap();
    let absent = directory.path().join("absent.sock");
    // A socket file nobody listens on any more, as a crashed seat leaves it.
    let left = directory.path().join("left.sock");
    drop(UnixListener::bind(&left).unwrap());
    assert!(left.exists());
    for socket in [absent, left] {
        let output = seat(&socket, &[OBSERVE], None);
        assert!(!output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(socket.to_str().unwrap()), "{error}");
        assert!(error.contains("No agent seat is listening"), "{error}");
        assert!(error.contains("start one"), "{error}");
    }
}
