use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[test]
fn ordinary_player_driver_refuses_privileged_fixture_records() {
    use arkham_unfolded_experiment::gathering::{HostileFixture, Session, continuity};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("verification.json");
    let fixture = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    continuity::save_session(&path, &fixture).unwrap();
    let original = std::fs::read(&path).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .args(["--experiment", "gathering", "--save"])
        .arg(&path)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "a fixture is not an ordinary player setup"
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response["error"].as_str().unwrap().contains("verification"));
    assert_eq!(std::fs::read(path).unwrap(), original);
}

fn run(directory: &Path, args: &[&str], requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .current_dir(directory)
        .args(["--experiment", "gathering"])
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
fn fresh_process_reopens_pending_mulligan_and_replays_without_redrawing() {
    use arkham_unfolded_experiment::gathering::{Session, continuity::load_session};

    let directory = tempfile::tempdir().unwrap();
    let initial = run(
        directory.path(),
        &["--save", "setup.json"],
        &[json!({"op":"choose", "revision":0, "id":"lead:tony"})],
    );
    assert_eq!(initial[1]["ok"], true);
    let pending = &initial[1]["observation"];
    let toggle = pending["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|choice| choice["id"].as_str().unwrap().starts_with("mulligan:"))
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let selection = run(
        directory.path(),
        &["--save", "setup.json"],
        &[json!({"op":"choose", "revision":1, "id":toggle})],
    );
    assert_eq!(&selection[0]["observation"], pending);
    assert_eq!(selection[1]["ok"], true);
    let selected = &selection[1]["observation"];
    let mut expected: Session = load_session(directory.path().join("setup.json")).unwrap();
    expected.choose(2, "confirm-mulligan").unwrap();

    let resumed = run(
        directory.path(),
        &["--save", "setup.json"],
        &[
            json!({"op":"reopen"}),
            json!({"op":"choose", "revision":2, "id":"confirm-mulligan"}),
            json!({"op":"replay"}),
            json!({"op":"next"}),
            json!({"op":"next"}),
            json!({"op":"choose", "revision":2, "id":"confirm-mulligan"}),
            json!({"op":"save"}),
            json!({"op":"reopen"}),
            json!({"op":"live"}),
        ],
    );
    assert_eq!(&resumed[0]["observation"], selected);
    assert_eq!(resumed[1]["ok"], true, "{}", resumed[1]["error"]);
    assert_eq!(resumed[2]["ok"], true);
    assert_eq!(
        resumed[2]["observation"],
        serde_json::to_value(expected.observe_for_play()).unwrap()
    );
    assert_eq!(&resumed[5]["observation"], selected);
    for refusal in &resumed[6..=8] {
        assert_eq!(refusal["ok"], false);
    }
    assert_eq!(resumed[9]["observation"], resumed[2]["observation"]);
    assert_eq!(
        load_session(directory.path().join("setup.json")).unwrap(),
        expected
    );

    let visible = serde_json::to_string(&resumed).unwrap();
    for secret in [
        "\"seed\"",
        "\"random\"",
        "\"deck_order\"",
        "\"encounter_order\"",
    ] {
        assert!(!visible.contains(secret), "Player output exposes {secret}");
    }
}

#[test]
fn player_requests_cannot_control_randomness_or_modify_replay_records() {
    let directory = tempfile::tempdir().unwrap();
    let result = run(
        directory.path(),
        &["--save", "setup.json"],
        &[
            json!({"op":"choose", "revision":0, "id":"lead:patrice", "seed":1}),
            json!({"op":"choose", "revision":0, "id":"lead:patrice"}),
            json!({"op":"export", "path":"setup.recording.json"}),
            json!({"op":"replay"}),
            json!({"op":"export", "path":"forbidden.json"}),
            json!({"op":"live"}),
        ],
    );
    assert_eq!(result[1]["ok"], false);
    assert_eq!(result[1]["observation"], result[0]["observation"]);
    assert_eq!(result[2]["ok"], true);
    assert_eq!(result[3]["ok"], true);
    assert_eq!(result[5]["ok"], false);
    assert!(!directory.path().join("forbidden.json").exists());
    assert_eq!(result[6]["observation"], result[2]["observation"]);
    let recording = arkham_unfolded_experiment::gathering::continuity::load_recording(
        directory.path().join("setup.recording.json"),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(recording.moments.last().unwrap()).unwrap(),
        result[2]["observation"]
    );
}

#[test]
fn newly_started_setup_is_persisted_before_any_player_choice() {
    use arkham_unfolded_experiment::gathering::continuity::load_session;

    let directory = tempfile::tempdir().unwrap();
    let first = run(directory.path(), &["--save", "setup.json"], &[]);
    let saved = load_session(directory.path().join("setup.json")).unwrap();
    assert_eq!(
        serde_json::to_value(saved.observe_for_play()).unwrap(),
        first[0]["observation"]
    );
    let second = run(directory.path(), &["--save", "setup.json"], &[]);
    assert_eq!(first[0]["observation"], second[0]["observation"]);
}
