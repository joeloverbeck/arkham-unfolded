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
fn a_refused_missed_skipped_or_missing_response_fails_the_command_after_printing_the_rest() {
    let refused = r#"{"ok":false,"status":"refused","error":"Outdated"}"#;
    let missed = r#"{"ok":false,"status":"missed","error":"covered","pressed":false}"#;
    let skipped = r#"{"ok":false,"status":"skipped","error":"after a refusal"}"#;
    for answers in [
        vec![refused, OBSERVED],
        vec![missed, OBSERVED],
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

/// A stand-in seat for `seat-wait` (#218), which opens one connection per request:
/// each connection's one request line is answered with the next canned line. An
/// empty one closes the connection unanswered, and [`HOLD`] keeps it open without
/// ever answering, as a seat that stopped would. After the last, the seat keeps
/// answering it, or goes away as a quitting seat does: it stops listening and its
/// socket file is removed.
struct Hosting {
    socket: std::path::PathBuf,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    answering: thread::JoinHandle<Vec<serde_json::Value>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Then {
    Repeat,
    Vanish,
}

const HOLD: &str = "hold the connection unanswered";

fn host(socket: &Path, answers: Vec<String>, then: Then) -> Hosting {
    use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
    let listener = UnixListener::bind(socket).unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let stop = done.clone();
    let path = socket.to_owned();
    let answering = thread::spawn(move || {
        let mut received = Vec::new();
        let mut held = Vec::new();
        let mut index = 0;
        loop {
            if then == Then::Vanish && index == answers.len() {
                drop(listener);
                std::fs::remove_file(&path).unwrap();
                return received;
            }
            let (stream, _) = listener.accept().unwrap();
            if stop.load(Ordering::SeqCst) {
                return received;
            }
            let mut request = String::new();
            BufReader::new(&stream).read_line(&mut request).unwrap();
            received.push(serde_json::from_str(&request).unwrap());
            let answer = &answers[index.min(answers.len() - 1)];
            index += 1;
            if answer == HOLD {
                held.push(stream);
            } else if !answer.is_empty() {
                let _ = (&stream).write_all(format!("{answer}\n").as_bytes());
            }
        }
    });
    Hosting {
        socket: socket.to_owned(),
        done,
        answering,
    }
}

impl Hosting {
    /// The requests as they arrived, once the command has finished.
    fn received(self) -> Vec<serde_json::Value> {
        self.done.store(true, std::sync::atomic::Ordering::SeqCst);
        if !self.answering.is_finished() {
            let _ = std::os::unix::net::UnixStream::connect(&self.socket);
        }
        self.answering.join().unwrap()
    }
}

/// An answer as the seat gives it, reduced to what `seat-wait` reads: the
/// observation's revision and stage, and the table's settledness, open surface and
/// controls' labels.
fn answer(
    status: &str,
    (revision, stage): (u64, &str),
    settled: bool,
    modal: Option<&str>,
    labels: &[&str],
) -> String {
    serde_json::json!({
        "ok": true,
        "mode": "seat",
        "status": status,
        "observation": {"revision": revision, "stage": stage, "decision_owner": null},
        "table": {
            "settled": settled,
            "pending": if settled { vec![] } else { vec!["partner request"] },
            "modal": modal,
            "menu": null,
            "controls": labels.iter().map(|label| serde_json::json!({
                "choice": null, "label": label, "position": [0, 0],
                "on_screen": true, "focused": false,
            })).collect::<Vec<_>>(),
        },
    })
    .to_string()
}

/// The portrait rail and decision display while Jacqueline's partner decides, as a
/// seat-hosted run of #202 showed them. The table's side of these label shapes is
/// pinned by `a_hosted_partner_is_shown_to_seat_wait_by_take_over_and_its_cue` in
/// `src/gathering_tabletop/seat/tests.rs`.
const JACQUELINE_THINKING: &[&str] = &[
    "Focus Tony Morgan",
    "Focus Patrice Hathaway",
    "Focus Jacqueline Fine · current turn · partner · thinking",
    "Take over Jacqueline Fine",
    "Take over Jacqueline Fine",
];
/// The same table once the decision is the seat's own.
const SEATS_OWN: &[&str] = &[
    "Focus Tony Morgan · current turn",
    "Focus Patrice Hathaway",
    "Focus Jacqueline Fine · partner",
];

fn seat_wait(socket: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .arg("seat-wait")
        .arg(socket)
        .args(args)
        .output()
        .unwrap()
}

fn lines(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn seat_wait_keeps_the_table_settling_until_the_decision_is_the_seats_own() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let seat = host(
        &socket,
        vec![
            answer("observed", (3, "mulligan"), true, None, JACQUELINE_THINKING),
            answer("settled", (3, "mulligan"), true, None, JACQUELINE_THINKING),
            answer("settled", (4, "mulligan"), true, None, JACQUELINE_THINKING),
            // Still presenting: nothing awaited is shown yet, and that is no stop.
            answer("unsettled", (5, "playing"), false, None, SEATS_OWN),
            answer("settled", (5, "playing"), true, None, SEATS_OWN),
        ],
        Then::Vanish,
    );
    let output = seat_wait(&socket, &[]);
    let received = seat.received();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let observe = serde_json::json!({"op": "observe"});
    let settle = serde_json::json!({"op": "settle", "steps": 120});
    assert_eq!(
        received,
        [
            observe,
            settle.clone(),
            settle.clone(),
            settle.clone(),
            settle
        ]
    );
    assert_eq!(
        lines(&output),
        [
            serde_json::json!({"revision": 3, "stage": "mulligan", "awaiting": ["Jacqueline Fine (thinking)"]}),
            serde_json::json!({"revision": 4, "stage": "mulligan", "awaiting": ["Jacqueline Fine (thinking)"]}),
            serde_json::json!({"revision": 5, "stage": "playing", "awaiting": []}),
            serde_json::json!({"stopped": "seat", "revision": 5, "choices": 2}),
        ]
    );
}

#[test]
fn seat_wait_returns_when_a_reading_dialog_or_menu_open_over_the_table_holds_partners_choices() {
    let with_menu = {
        let mut answer: serde_json::Value = serde_json::from_str(&answer(
            "settled",
            (16, "playing"),
            true,
            None,
            JACQUELINE_THINKING,
        ))
        .unwrap();
        answer["table"]["menu"] =
            serde_json::json!({"component": "card:23", "title": "Magnifying Glass", "rows": []});
        answer.to_string()
    };
    for (held, open) in [
        (
            answer(
                "settled",
                (16, "playing"),
                true,
                Some("test result"),
                JACQUELINE_THINKING,
            ),
            "test result",
        ),
        (with_menu, "menu"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("seat.sock");
        let seat = host(
            &socket,
            vec![
                answer("observed", (16, "playing"), true, None, JACQUELINE_THINKING),
                held,
            ],
            Then::Vanish,
        );
        let output = seat_wait(&socket, &[]);
        seat.received();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert_eq!(
            lines(&output),
            [
                serde_json::json!({"revision": 16, "stage": "playing", "awaiting": ["Jacqueline Fine (thinking)"]}),
                serde_json::json!({"stopped": "seat", "revision": 16, "choices": 0, "open": open}),
            ]
        );
    }
}

#[test]
fn seat_wait_returns_at_a_revision_or_after_a_count_of_applied_choices() {
    let awaited = |revision| {
        answer(
            "settled",
            (revision, "playing"),
            true,
            None,
            JACQUELINE_THINKING,
        )
    };
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let seat = host(
        &socket,
        vec![awaited(3), awaited(4), awaited(5)],
        Then::Vanish,
    );
    let output = seat_wait(&socket, &["--revision", "5"]);
    seat.received();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        lines(&output).last().unwrap(),
        &serde_json::json!({"stopped": "revision", "revision": 5, "choices": 2})
    );

    // Choices count from the table as the command found it: the first settle
    // already applies one, and a table still presenting the second is no stop.
    std::fs::remove_file(&socket).ok();
    let seat = host(
        &socket,
        vec![
            awaited(3),
            awaited(4),
            answer(
                "unsettled",
                (5, "playing"),
                false,
                None,
                JACQUELINE_THINKING,
            ),
            awaited(5),
        ],
        Then::Vanish,
    );
    let output = seat_wait(&socket, &["--choices", "2"]);
    seat.received();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        lines(&output),
        [
            serde_json::json!({"revision": 3, "stage": "playing", "awaiting": ["Jacqueline Fine (thinking)"]}),
            serde_json::json!({"revision": 4, "stage": "playing", "awaiting": ["Jacqueline Fine (thinking)"]}),
            serde_json::json!({"revision": 5, "stage": "playing", "awaiting": ["Jacqueline Fine (thinking)"]}),
            serde_json::json!({"stopped": "choices", "revision": 5, "choices": 2}),
        ]
    );
}

#[test]
fn seat_wait_returns_with_exit_3_once_its_timeout_passes() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    // A partner that stays silent, and one that has not joined.
    let silent = [
        "Focus Tony Morgan",
        "Focus Patrice Hathaway · partner · awaiting partner",
        "Focus Jacqueline Fine · current turn · partner · silent 3 min",
        "Take over Patrice Hathaway",
        "Take over Jacqueline Fine",
        "Take over Jacqueline Fine",
    ];
    let seat = host(
        &socket,
        vec![answer("settled", (9, "playing"), true, None, &silent)],
        Then::Repeat,
    );
    let began = std::time::Instant::now();
    let output = seat_wait(&socket, &["--timeout", "1"]);
    let took = began.elapsed();
    let received = seat.received();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(took >= std::time::Duration::from_secs(1), "{took:?}");
    assert!(took < std::time::Duration::from_secs(10), "{took:?}");
    assert!(received.len() > 1, "it kept settling: {received:?}");
    assert_eq!(
        lines(&output),
        [
            serde_json::json!({"revision": 9, "stage": "playing", "awaiting": [
                "Jacqueline Fine (silent 3 min)", "Patrice Hathaway (awaiting partner)",
            ]}),
            serde_json::json!({"stopped": "timeout", "revision": 9, "choices": 0}),
        ]
    );

    // A timeout beyond an agent harness's ten-minute tool call is refused.
    let output = seat_wait(&socket, &["--timeout", "591"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
}

#[test]
fn seat_wait_returns_with_exit_4_when_the_seat_goes_away() {
    let thinking = |revision| {
        answer(
            "settled",
            (revision, "playing"),
            true,
            None,
            JACQUELINE_THINKING,
        )
    };
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let ended = answer("exited", (4, "playing"), true, None, JACQUELINE_THINKING);
    // It stops listening; it closes a connection unanswered; it ends its session.
    for last in [None, Some(String::new()), Some(ended)] {
        let mut answers = vec![thinking(3), thinking(4)];
        answers.extend(last);
        let seat = host(&socket, answers, Then::Vanish);
        let output = seat_wait(&socket, &[]);
        seat.received();
        std::fs::remove_file(&socket).ok();
        assert_eq!(output.status.code(), Some(4), "{output:?}");
        assert_eq!(
            lines(&output).last().unwrap(),
            &serde_json::json!({"stopped": "gone", "revision": 4, "choices": 1})
        );
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains(socket.to_str().unwrap()), "{error}");
        assert!(error.contains("has gone"), "{error}");
    }

    // No seat at all: the same exit, saying to start one.
    let output = seat_wait(&socket, &[]);
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    assert_eq!(
        lines(&output),
        [serde_json::json!({"stopped": "gone", "revision": null, "choices": 0})]
    );
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("No agent seat is listening"), "{error}");
    assert!(error.contains("start one"), "{error}");
}

#[test]
fn seat_wait_fails_with_exit_2_on_an_answer_that_is_neither_an_observation_nor_a_settle() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let refused = r#"{"ok":false,"status":"refused","error":"Malformed request."}"#;
    let seat = host(&socket, vec![refused.to_owned()], Then::Vanish);
    let output = seat_wait(&socket, &[]);
    seat.received();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains(refused), "{error}");
}

#[test]
fn seat_wait_fails_with_exit_2_when_the_seat_stops_answering_as_partner_wait_does() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let seat = host(
        &socket,
        vec![
            answer("observed", (9, "playing"), true, None, JACQUELINE_THINKING),
            HOLD.to_owned(),
        ],
        Then::Repeat,
    );
    let began = std::time::Instant::now();
    let output = seat_wait(&socket, &["--timeout", "1"]);
    let took = began.elapsed();
    seat.received();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(took >= std::time::Duration::from_secs(1), "{took:?}");
    // What was seen before it stopped answering is printed; no reason line follows.
    assert_eq!(
        lines(&output),
        [
            serde_json::json!({"revision": 9, "stage": "playing", "awaiting": ["Jacqueline Fine (thinking)"]})
        ]
    );
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("did not answer in time"), "{error}");
    assert!(error.contains(socket.to_str().unwrap()), "{error}");
}
