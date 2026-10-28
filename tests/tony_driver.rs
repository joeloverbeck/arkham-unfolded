use serde_json::Value;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn run(directory: &Path, args: &[&str], input: &str) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .current_dir(directory)
        .args(["--experiment", "tony"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

#[test]
fn fresh_process_reopens_a_pending_test_and_uses_the_saved_random_sequence() {
    use std::io::{BufRead, BufReader};
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .current_dir(directory.path())
        .args(["--experiment", "tony", "--save", "control.json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    fn receive(output: &mut impl BufRead) -> Value {
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    assert_eq!(receive(&mut output)["ok"], true);
    for (revision, id) in [(0, "fight"), (1, "commit:none")] {
        writeln!(
            input,
            "{}",
            serde_json::json!({"op":"choose", "revision":revision, "id":id})
        )
        .unwrap();
        assert_eq!(receive(&mut output)["ok"], true);
    }
    std::fs::copy(
        directory.path().join("control.json"),
        directory.path().join("pending.json"),
    )
    .unwrap();
    writeln!(
        input,
        "{}",
        serde_json::json!({"op":"choose", "revision":2, "id":"reveal"})
    )
    .unwrap();
    let revealed = receive(&mut output);
    assert_eq!(revealed["ok"], true);
    std::fs::copy(
        directory.path().join("control.json"),
        directory.path().join("held.json"),
    )
    .unwrap();
    writeln!(
        input,
        "{}",
        serde_json::json!({"op":"choose", "revision":3, "id":"resolve"})
    )
    .unwrap();
    let uninterrupted = receive(&mut output);
    assert_eq!(uninterrupted["ok"], true);
    drop(input);
    assert!(child.wait().unwrap().success());
    let resumed = run(
        directory.path(),
        &["--load", "pending.json"],
        concat!(
            "{\"op\":\"choose\",\"revision\":2,\"id\":\"reveal\"}\n",
            "{\"op\":\"choose\",\"revision\":3,\"id\":\"resolve\"}\n"
        ),
    );
    assert_eq!(resumed[1], revealed);
    assert_eq!(resumed[2], uninterrupted);
    let held = run(
        directory.path(),
        &["--load", "held.json"],
        "{\"op\":\"choose\",\"revision\":3,\"id\":\"resolve\"}\n",
    );
    assert_eq!(held[0]["observation"], revealed["observation"]);
    assert_eq!(held[1], uninterrupted);
    assert_eq!(held[1]["observation"]["ordinary_actions"], 3);
    assert_eq!(held[1]["observation"]["bonus_action"], false);
}

#[test]
fn replay_cannot_save_or_choose_and_player_commands_do_not_expose_hidden_state() {
    let directory = tempfile::tempdir().unwrap();
    let responses = run(
        directory.path(),
        &["--save", "turn.json"],
        concat!(
            "{\"op\":\"choose\",\"revision\":0,\"id\":\"injury\"}\n",
            "{\"op\":\"replay\"}\n",
            "{\"op\":\"next\"}\n",
            "{\"op\":\"back\"}\n",
            "{\"op\":\"save\"}\n",
            "{\"op\":\"choose\",\"revision\":0,\"id\":\"fight\"}\n",
            "{\"op\":\"export\",\"path\":\"past.json\"}\n",
            "{\"op\":\"live\"}\n",
            "{\"op\":\"observe\",\"draws\":[\"elder_sign\"]}\n",
            "{\"op\":\"export\",\"path\":\"record.json\"}\n"
        ),
    );
    assert_eq!(responses[1]["ok"], true);
    assert_eq!(responses[3]["observation"], responses[1]["observation"]);
    assert_eq!(responses[4]["observation"], responses[0]["observation"]);
    for index in [5, 6, 7, 9] {
        assert_eq!(responses[index]["ok"], false);
    }
    assert!(!directory.path().join("past.json").exists());
    assert_eq!(responses[8]["observation"], responses[1]["observation"]);
    let reopened = run(directory.path(), &["--load", "turn.json"], "");
    assert_eq!(reopened[0]["observation"], responses[1]["observation"]);
    assert_eq!(responses[10]["ok"], true);
    for response in &responses {
        let observation = response["observation"].as_object().unwrap();
        for hidden in ["seed", "random", "draws", "deck", "fixed_draws", "state"] {
            assert!(!observation.contains_key(hidden));
        }
    }
}
