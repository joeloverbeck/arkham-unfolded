use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn run_driver(directory: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn responses(output: &Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn empty_commitment_reopens_at_the_reaction_through_fresh_driver_processes() {
    for legacy in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let original = include_bytes!("fixtures/commitment/jacqueline-empty-v1.session.json");
        if legacy {
            std::fs::write(directory.path().join("session.json"), original).unwrap();
        }
        let args: &[&str] = if legacy {
            &["--load", "session.json"]
        } else {
            &["--patrice-away", "--save", "session.json"]
        };
        let saved = responses(&run_driver(directory.path(), args, "{\"op\":\"save\"}\n"));
        let pending = &saved[1]["observation"];
        assert_eq!(pending["stage"], "before_reveal");
        assert_eq!(pending["revision"], 0);
        assert_eq!(pending["tokens"], serde_json::json!([]));
        assert_eq!(pending["choices"][0]["id"], "use-jacqueline");
        if legacy {
            assert_eq!(
                std::fs::read(directory.path().join("session.json")).unwrap(),
                original
            );
        }

        let reopened = responses(&run_driver(
            directory.path(),
            &["--load", "session.json"],
            "{\"op\":\"choose\",\"revision\":0,\"id\":\"use-jacqueline\"}\n",
        ));
        assert_eq!(&reopened[0]["observation"], pending);
        assert_eq!(reopened[1]["ok"], true);
        assert_eq!(reopened[1]["observation"]["revision"], 1);
        assert_eq!(reopened[1]["observation"]["stage"], "cancellation");
        assert_eq!(
            reopened[1]["observation"]["tokens"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(reopened[1]["observation"]["curses_in_bag"], 0);
    }
}

#[test]
fn a_committed_choice_resumes_through_a_second_driver_process() {
    let directory = tempfile::tempdir().unwrap();
    let first = run_driver(
        directory.path(),
        &["--save", "session.json"],
        "{\"op\":\"choose\",\"revision\":0,\"id\":\"commit-promise\"}\n",
    );
    let first = responses(&first);
    assert_eq!(first.len(), 2);
    assert_eq!(first[1]["ok"], true);
    assert_eq!(first[1]["observation"]["revision"], 1);
    assert_eq!(first[1]["observation"]["skill_value"], 9);

    let second = run_driver(
        directory.path(),
        &["--load", "session.json"],
        "{\"op\":\"observe\"}\n",
    );
    let second = responses(&second);
    assert_eq!(second[1]["observation"]["stage"], "before_reveal");
    assert_eq!(second[1]["observation"]["revision"], 1);
    assert_eq!(second[1]["observation"]["curses_in_bag"], 1);
    assert_eq!(second[1]["observation"]["jacqueline_used"], false);
}

#[test]
fn replay_navigation_cannot_change_the_live_match() {
    let directory = tempfile::tempdir().unwrap();
    let output = run_driver(
        directory.path(),
        &["--save", "session.json"],
        concat!(
            "{\"op\":\"choose\",\"revision\":0,\"id\":\"commit-promise\"}\n",
            "{\"op\":\"choose\",\"revision\":1,\"id\":\"use-jacqueline\"}\n",
            "{\"op\":\"choose\",\"revision\":2,\"id\":\"cancel-0-1\"}\n",
            "{\"op\":\"replay\"}\n",
            "{\"op\":\"next\"}\n",
            "{\"op\":\"choose\",\"revision\":1,\"id\":\"use-jacqueline\"}\n",
            "{\"op\":\"save\"}\n",
            "{\"op\":\"back\"}\n",
            "{\"op\":\"live\"}\n",
        ),
    );
    let output = responses(&output);
    assert_eq!(output[4]["mode"], "replay");
    assert_eq!(output[4]["observation"]["revision"], 0);
    assert_eq!(output[5]["observation"]["revision"], 1);
    assert_eq!(output[6]["ok"], false);
    assert_eq!(output[6]["mode"], "replay");
    assert_eq!(output[7]["ok"], false);
    assert_eq!(output[7]["mode"], "replay");
    assert_eq!(output[8]["observation"]["revision"], 0);
    assert_eq!(output[9]["mode"], "live");
    assert_eq!(output[9]["observation"]["stage"], "finished");
    assert_eq!(output[9]["observation"]["revision"], 3);
    assert_eq!(output[9]["observation"]["skill_value"], 9);
    assert_eq!(output[9]["observation"]["horror_dealt"], 0);

    let reopened = run_driver(directory.path(), &["--load", "session.json"], "");
    assert_eq!(
        responses(&reopened)[0]["observation"],
        output[9]["observation"]
    );
}

#[test]
fn an_explicit_save_reopens_the_unplayed_decision() {
    let directory = tempfile::tempdir().unwrap();
    let saved = run_driver(
        directory.path(),
        &["--save", "session.json"],
        "{\"op\":\"save\"}\n",
    );
    let saved = responses(&saved);
    assert_eq!(saved[1]["ok"], true);
    assert_eq!(saved[1]["status"], "saved");
    let reopened = run_driver(directory.path(), &["--load", "session.json"], "");
    let reopened = responses(&reopened);
    assert_eq!(reopened[0]["observation"], saved[1]["observation"]);
}

#[test]
fn refused_player_commands_leave_the_current_decision_intact() {
    let directory = tempfile::tempdir().unwrap();
    let output = run_driver(
        directory.path(),
        &["--save", "session.json"],
        concat!(
            "{\"op\":\"choose\",\"revision\":0,\"id\":\"commit-promise\"}\n",
            "{\"op\":\"choose\",\"revision\":0,\"id\":\"use-jacqueline\"}\n",
            "{\"op\":\"choose\",\"revision\":1,\"id\":\"invent-token\"}\n",
            "{\"op\":\"observe\",\"fixture\":\"autofail\"}\n",
            "not-json\n",
            "{\"op\":\"set-fixture\",\"case\":\"autofail\"}\n",
            "{\"op\":\"observe\"}\n",
        ),
    );
    let output = responses(&output);
    assert_eq!(output[1]["ok"], true);
    for refusal in &output[2..7] {
        assert_eq!(refusal["ok"], false);
        assert_eq!(refusal["observation"], output[1]["observation"]);
    }
    assert_eq!(output[7]["observation"], output[1]["observation"]);
    assert!(
        output[7]["observation"]["tokens"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for response in &output {
        assert!(response.get("session").is_none());
        for hidden in [
            "fixture",
            "draw_order",
            "draw_cursor",
            "recording",
            "moments",
        ] {
            assert!(
                !response["observation"]
                    .as_object()
                    .unwrap()
                    .contains_key(hidden)
            );
        }
    }
}

#[test]
fn an_autosave_failure_refuses_the_choice_without_advancing_live_state() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("blocked"), b"retain this file").unwrap();
    let output = run_driver(
        directory.path(),
        &["--save", "blocked/session.json"],
        concat!(
            "{\"op\":\"choose\",\"revision\":0,\"id\":\"commit-promise\"}\n",
            "{\"op\":\"observe\"}\n",
        ),
    );
    let output = responses(&output);
    assert_eq!(output[1]["ok"], false);
    assert_eq!(output[1]["observation"], output[0]["observation"]);
    assert_eq!(output[2]["observation"], output[0]["observation"]);
    assert_eq!(
        std::fs::read(directory.path().join("blocked")).unwrap(),
        b"retain this file"
    );
}

#[test]
fn startup_refuses_a_corrupt_load_or_a_new_game_over_an_existing_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session.json");
    let original = b"a damaged record still belongs to its owner";
    std::fs::write(&path, original).unwrap();
    for args in [["--load", "session.json"], ["--save", "session.json"]] {
        let output = run_driver(directory.path(), &args, "");
        assert!(!output.status.success());
        let error: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(error["ok"], false);
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}
