use serde_json::{Value, json};
use std::{
    io::{Seek, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::SystemTime,
};

fn directory_state(directory: &Path) -> Vec<(PathBuf, Vec<u8>, SystemTime)> {
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into(),
                std::fs::read(entry.path()).unwrap(),
                entry.metadata().unwrap().modified().unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

#[test]
fn inspection_reports_one_observation_without_writes_or_player_actions() {
    use arkham_unfolded_experiment::gathering::{Session, continuity};

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pending.json");
    let mut session = Session::with_seed(89);
    session.choose(0, "lead:tony").unwrap();
    continuity::save_session(&path, &session).unwrap();
    std::fs::remove_file(directory.path().join("pending.json.lock")).unwrap();
    // Leave a recovery copy as well: inspection must preserve every sibling.
    std::fs::copy(&path, continuity::previous_path(&path)).unwrap();
    let before = directory_state(directory.path());
    let mut input = tempfile::tempfile().unwrap();
    writeln!(
        input,
        "{}",
        json!({"op":"choose", "revision":1, "id":"confirm-mulligan"})
    )
    .unwrap();
    writeln!(input, "{}", json!({"op":"save"})).unwrap();
    input.rewind().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .current_dir(directory.path())
        .args(["--experiment", "gathering", "--inspect", "pending.json"])
        .stdin(input)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    // Parsing one JSON value also rejects extra live-protocol responses.
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["ok"], true);
    assert_eq!(response["mode"], "inspection");
    assert_eq!(response["status"], "inspected");
    assert_eq!(
        response["observation"],
        serde_json::to_value(session.observe()).unwrap()
    );
    assert_eq!(directory_state(directory.path()), before);
}

#[test]
fn inspection_refuses_missing_invalid_and_privileged_saves_without_writes() {
    use arkham_unfolded_experiment::gathering::{HostileFixture, Session, continuity};

    for kind in ["missing", "malformed", "unsupported", "fixture"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("input.json");
        match kind {
            "missing" => {}
            "malformed" => std::fs::write(&path, b"{broken").unwrap(),
            "unsupported" => {
                continuity::save_session(&path, &Session::with_seed(89)).unwrap();
                let mut record: Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                record["format_version"] = json!(0);
                std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
            }
            "fixture" => continuity::save_session(
                &path,
                &Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision),
            )
            .unwrap(),
            _ => unreachable!(),
        }
        let before = directory_state(directory.path());
        let output = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
            .current_dir(directory.path())
            .args(["--inspect", "input.json"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{kind}: {output:?}");
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["ok"], false);
        assert_eq!(response["status"], "error");
        assert_eq!(directory_state(directory.path()), before, "{kind}");
    }
}

#[test]
fn inspection_rejects_live_paths_in_either_argument_order() {
    for live_flag in ["--load", "--save"] {
        for inspect_first in [true, false] {
            let directory = tempfile::tempdir().unwrap();
            let inspect = ["--inspect", "missing.json"];
            let live = [live_flag, "live.json"];
            let args = if inspect_first {
                [inspect, live]
            } else {
                [live, inspect]
            };
            let output = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
                .current_dir(directory.path())
                .args(args.into_iter().flatten())
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(!output.status.success());
            let response: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(
                response["error"]
                    .as_str()
                    .unwrap()
                    .contains("cannot be combined")
            );
            assert!(directory_state(directory.path()).is_empty());
        }
    }
}

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
fn fresh_process_reopens_pending_mulligan_without_redrawing() {
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
        serde_json::to_value(expected.observe()).unwrap()
    );
    for index in [3, 4, 5, 6, 9] {
        assert_eq!(resumed[index]["ok"], false);
    }
    for index in [7, 8] {
        assert_eq!(resumed[index]["ok"], true);
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
fn player_requests_cannot_control_randomness_or_export_recordings() {
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
    for response in &result[3..=6] {
        assert_eq!(response["ok"], false);
    }
    assert_eq!(result[5]["ok"], false);
    assert!(!directory.path().join("forbidden.json").exists());
    assert_eq!(result[6]["observation"], result[2]["observation"]);

    assert!(!directory.path().join("setup.recording.json").exists());
}

#[test]
fn newly_started_setup_is_persisted_before_any_player_choice() {
    use arkham_unfolded_experiment::gathering::continuity::load_session;

    let directory = tempfile::tempdir().unwrap();
    let first = run(directory.path(), &["--save", "setup.json"], &[]);
    let saved = load_session(directory.path().join("setup.json")).unwrap();
    assert_eq!(
        serde_json::to_value(saved.observe()).unwrap(),
        first[0]["observation"]
    );
    let second = run(directory.path(), &["--save", "setup.json"], &[]);
    assert_eq!(first[0]["observation"], second[0]["observation"]);
}

#[test]
fn ordinary_setup_round_and_mythos_resume_through_fresh_player_processes() {
    use arkham_unfolded_experiment::gathering::{Phase, Session, continuity};

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ordinary.json");
    let mut expected = Session::with_seed(89);
    assert!(!expected.is_verification_fixture());
    continuity::save_session(&path, &expected).unwrap();
    let mut requests = Vec::new();
    for _ in 0..180 {
        let view = expected.observe();
        if view
            .play
            .as_ref()
            .is_some_and(|play| play.round == 2 && play.phase == Phase::Investigation)
        {
            break;
        }
        // A modest, legal player policy exercises the ordinary production
        // entry. Its choices establish continuity, not a strategy/rules oracle.
        let preferred = [
            "lead:tony",
            "confirm-mulligan",
            "phase-window:pass",
            "window:pass",
            "test-card:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
            "success:pass",
            "after-test:pass",
            "harm:damage:investigator",
            "harm:horror:investigator",
            "harm:confirm",
            "action:ordinary:resource",
            "turn:end",
            "enemy:attacks",
            "enemy:skip-empty",
            "round:end",
            "round-end:decline-act",
            "mythos:begin",
            "mythos:encounters",
            "revelation:resolve",
            "investigation:begin",
        ];
        let id = preferred
            .into_iter()
            .find_map(|id| view.choices.iter().find(|choice| choice.id == id))
            .or_else(|| {
                view.choices.iter().find(|choice| {
                    choice.id.contains("decline")
                        || choice.id.ends_with(":pass")
                        || [
                            "turn:",
                            "upkeep:",
                            "enemy:",
                            "mythos:",
                            "result:",
                            "turn-end:",
                            "round-end:",
                            "revelation-order:",
                            "discard-card:",
                        ]
                        .iter()
                        .any(|prefix| choice.id.starts_with(prefix))
                        || (choice.id.starts_with("bounty:") && choice.id.ends_with(":0"))
                })
            })
            .unwrap_or_else(|| panic!("Unexpected ordinary choice: {:?}", view.choices))
            .id
            .clone();
        requests.push(json!({"op":"choose", "revision":view.revision, "id":id}));
        expected.choose(view.revision, &id).unwrap();
    }
    let pending = expected.observe();
    // The session performs every mythos draw itself and records each.
    assert_eq!(
        pending
            .history
            .iter()
            .filter(|entry| entry.starts_with("Automatic: ")
                && entry.ends_with(" draws an encounter card"))
            .count(),
        3
    );
    assert_eq!(pending.play.as_ref().unwrap().round, 2);
    assert_eq!(pending.play.as_ref().unwrap().phase, Phase::Investigation);
    assert_eq!(pending.play.as_ref().unwrap().encounter_deck_count, 23);
    assert!(pending.verification_fixture.is_none());

    let first = run(directory.path(), &["--save", "ordinary.json"], &requests);
    assert!(first.iter().all(|response| response["ok"] == true));
    assert_eq!(
        first.last().unwrap()["observation"],
        serde_json::to_value(&pending).unwrap()
    );
    assert_eq!(continuity::load_session(&path).unwrap(), expected);
    let original = std::fs::read(&path).unwrap();
    let resumed = run(
        directory.path(),
        &["--save", "ordinary.json"],
        &[
            json!({"op":"replay"}),
            json!({"op":"next"}),
            json!({"op":"back"}),
            json!({"op":"live"}),
            json!({"op":"export", "path":"ordinary.recording.json"}),
        ],
    );
    assert!(resumed[1..].iter().all(|response| response["ok"] == false));
    assert_eq!(
        resumed[0]["observation"],
        serde_json::to_value(&pending).unwrap()
    );
    assert_eq!(resumed[4]["observation"], resumed[0]["observation"]);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(!directory.path().join("ordinary.recording.json").exists());
    let visible = serde_json::to_string(&first).unwrap();
    for secret in [
        "\"seed\"",
        "\"random\"",
        "\"fixture_tokens\"",
        "\"deck_order\"",
        "\"encounter_order\"",
    ] {
        assert!(!visible.contains(secret), "Player output exposes {secret}");
    }
}

#[test]
fn ordinary_player_pays_trapped_once_from_the_observed_group_clue_payment() {
    use arkham_unfolded_experiment::gathering::{Session, continuity};

    // Ordinary seeded play: investigate until the party holds Trapped's clues.
    let mut expected = Session::with_seed(89);
    for _ in 0..200 {
        let view = expected.observe();
        if view.group_clue_payment.is_some() {
            break;
        }
        let id = [
            "lead:tony",
            "confirm-mulligan",
            "turn:tony",
            "turn:patrice",
            "turn:jacqueline",
            "action:ordinary:investigate",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
            "success:pass",
            "turn:end",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .or_else(|| view.choices.first().map(|choice| choice.id.as_str()))
        .unwrap()
        .to_owned();
        expected.choose(view.revision, &id).unwrap();
    }
    let view = expected.observe();
    let payment = view.group_clue_payment.clone().expect("Trapped is payable");
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id == payment.choice)
    );
    assert_eq!(payment.required, 6);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ordinary.json");
    continuity::save_session(&path, &expected).unwrap();
    std::fs::remove_file(directory.path().join("ordinary.json.lock")).ok();

    // The driver decides from the observation's holdings, not from labels.
    let observed = run(directory.path(), &["--save", "ordinary.json"], &[]);
    let observed = &observed[0]["observation"]["group_clue_payment"];
    let mut allocation = Vec::new();
    let mut remaining = observed["required"].as_u64().unwrap();
    for contributor in observed["contributors"].as_array().unwrap() {
        assert!(contributor["ineligible"].is_null());
        let paid = contributor["clues"].as_u64().unwrap().min(remaining);
        remaining -= paid;
        allocation.push(paid.to_string());
    }
    let id = format!("act:pay:{}", allocation.join(":"));
    let responses = run(
        directory.path(),
        &["--save", "ordinary.json"],
        &[json!({"op": "choose", "revision": view.revision, "id": id})],
    );
    let chosen = responses.last().unwrap();
    assert_eq!(chosen["status"], "chosen", "{chosen}");
    assert_eq!(chosen["observation"]["play"]["act"], 2);
    expected.choose(view.revision, &id).unwrap();
    assert_eq!(continuity::load_session(&path).unwrap(), expected);
    assert_eq!(
        chosen["observation"],
        serde_json::to_value(expected.observe()).unwrap()
    );
}
