use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn run(directory: &Path, args: &[&str], requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .current_dir(directory)
        .args(["--experiment", "party"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for request in requests {
        writeln!(input, "{request}").unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn party_driver_reopens_the_same_pending_cooperative_commitment() {
    let directory = tempfile::tempdir().unwrap();
    let initial = run(
        directory.path(),
        &["--save", "attempt.json"],
        &[
            json!({"op":"choose","revision":0,"id":"select:jacqueline"}),
            json!({"op":"choose","revision":1,"id":"investigate"}),
            json!({"op":"save"}),
        ],
    );
    assert_eq!(initial.last().unwrap()["ok"], true);
    let pending = &initial.last().unwrap()["observation"];
    assert!(
        pending["choices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|choice| choice["id"] == "commit:100,200")
    );
    let resumed = run(
        directory.path(),
        &["--load", "attempt.json"],
        &[json!({"op":"choose","revision":2,"id":"commit:100,200"})],
    );
    assert_eq!(&resumed[0]["observation"], pending);
    assert_eq!(resumed[1]["ok"], true);
    assert_eq!(resumed[1]["observation"]["test"]["skill_value"], 8);
    let observation = serde_json::to_string(&resumed[1]["observation"]).unwrap();
    for secret in ["rng", "draw_prefix", "forced_draws", "deck_order"] {
        assert!(
            !observation.contains(secret),
            "Observation exposed {secret}"
        );
    }
}

fn reopened_continuation_matches(
    mut game: arkham_unfolded_experiment::party::Session,
    ids: &[&str],
) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pending.json");
    arkham_unfolded_experiment::party::continuity::save_session(&path, &game).unwrap();
    let pending = serde_json::to_value(game.observe()).unwrap();
    let mut requests = Vec::new();
    let mut expected = Vec::new();
    for id in ids {
        let revision = game.observe().revision;
        requests.push(json!({"op":"choose", "revision":revision, "id":id}));
        game.choose(revision, id).unwrap();
        expected.push(serde_json::to_value(game.observe()).unwrap());
    }
    let resumed = run(directory.path(), &["--load", "pending.json"], &requests);
    assert_eq!(resumed[0]["observation"], pending);
    for (actual, expected) in resumed[1..].iter().zip(expected) {
        assert_eq!(actual["ok"], true, "{actual}");
        assert_eq!(actual["observation"], expected);
    }
}

#[test]
fn fresh_process_reopens_cancellation_and_keeps_the_unrevealed_continuation() {
    use arkham_unfolded_experiment::{TokenFace, party::Session};
    let mut game = Session::with_draws(vec![
        TokenFace::AutoFail,
        TokenFace::Curse,
        TokenFace::Zero,
        TokenFace::MinusTwo,
    ]);
    for id in [
        "select:jacqueline",
        "investigate",
        "commit:100,200",
        "use-jacqueline",
    ] {
        game.choose(game.observe().revision, id).unwrap();
    }
    reopened_continuation_matches(
        game,
        &[
            "cancel:0",
            "resolve:1",
            "reveal",
            "resolve:3",
            "resolve:2",
            "finish-test",
        ],
    );
}

#[test]
fn driver_replay_refuses_choices_and_save_and_returns_to_unchanged_live_state() {
    let directory = tempfile::tempdir().unwrap();
    let result = run(
        directory.path(),
        &["--save", "attempt.json"],
        &[
            json!({"op":"choose","revision":0,"id":"select:tony"}),
            json!({"op":"export","path":"original.recording.json"}),
            json!({"op":"replay"}),
            json!({"op":"next"}),
            json!({"op":"back"}),
            json!({"op":"choose","revision":0,"id":"select:patrice"}),
            json!({"op":"save"}),
            json!({"op":"live"}),
        ],
    );
    assert_eq!(result[6]["ok"], false);
    assert_eq!(result[7]["ok"], false);
    assert_eq!(result[8]["observation"], result[1]["observation"]);
    let saved = arkham_unfolded_experiment::party::continuity::load_session(
        directory.path().join("attempt.json"),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(saved.observe()).unwrap(),
        result[1]["observation"]
    );
    let recording = arkham_unfolded_experiment::party::continuity::load_recording(
        directory.path().join("original.recording.json"),
    )
    .unwrap();
    assert_eq!(recording, saved.recording());
}

fn choices(game: &mut arkham_unfolded_experiment::party::Session, ids: &[&str]) {
    for id in ids {
        game.choose(game.observe().revision, id).unwrap();
    }
}

#[test]
fn fresh_process_resumes_lucky_and_the_final_after_test_shuffle() {
    use arkham_unfolded_experiment::{TokenFace, party::Session};
    let mut game = Session::with_draws(vec![
        TokenFace::Zero,
        TokenFace::AutoFail,
        TokenFace::ElderSign,
        TokenFace::MinusFour,
    ]);
    choices(
        &mut game,
        &[
            "select:tony",
            "investigate",
            "commit:100",
            "reveal",
            "resolve:0",
            "finish-test",
            "end-turn",
            "select:patrice",
            "draw",
            "resource",
            "investigate",
            "commit:none",
            "use-jacqueline",
            "cancel:0",
            "resolve:1",
            "resolve:2",
        ],
    );
    reopened_continuation_matches(
        game.clone(),
        &["lucky:101", "finish-test", "shuffle:retain:100"],
    );
    choices(&mut game, &["lucky:101", "finish-test"]);
    reopened_continuation_matches(game, &["shuffle:retain:101"]);
}

#[test]
fn fresh_process_resumes_the_six_clue_spending_choice() {
    use arkham_unfolded_experiment::{TokenFace, party::Session};
    let mut game = Session::with_draws(vec![TokenFace::Zero; 6]);
    choices(&mut game, &["select:tony"]);
    for _ in 0..3 {
        choices(
            &mut game,
            &[
                "investigate",
                "commit:none",
                "reveal",
                "resolve:0",
                "finish-test",
            ],
        );
    }
    choices(&mut game, &["end-turn", "select:patrice"]);
    for _ in 0..3 {
        choices(
            &mut game,
            &[
                "investigate",
                "commit:none",
                "reveal",
                "resolve:0",
                "finish-test",
            ],
        );
    }
    reopened_continuation_matches(game.clone(), &["advance"]);
    reopened_continuation_matches(game, &["decline-advance"]);
}
