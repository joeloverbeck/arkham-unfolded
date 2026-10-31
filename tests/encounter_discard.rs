use arkham_unfolded_experiment::{TokenFace, tony};
use serde_json::Value;
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn driver(directory: &Path, args: &[&str], input: &str) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-driver"))
        .current_dir(directory)
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
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn legacy_defeat_continuation_is_atomic_and_retains_its_historical_prefix() {
    use tony::continuity::{
        Replay, commit_choice, load_recording, load_session, save_recording, save_session,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let bytes = include_bytes!("fixtures/encounter-discard/before-defeat.session.json");
    std::fs::write(&path, bytes).unwrap();
    let mut game = load_session(&path).unwrap();
    let prior = game.clone();
    let historical = game.recording();
    save_session(&path, &game).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let blocked = directory.path().join("blocked");
    std::fs::create_dir(&blocked).unwrap();
    assert!(commit_choice(&blocked, &mut game, prior.observe().revision, "resolve").is_err());
    assert_eq!(game, prior);
    assert!(game.observe().encounter_discard().is_empty());
    commit_choice(&path, &mut game, prior.observe().revision, "resolve").unwrap();
    assert_eq!(game.observe().encounter_discard()[0].code, "01160");
    assert!(game.recording().moments.starts_with(&historical.moments));
    assert_eq!(load_session(&path).unwrap(), game);
    let recorded = game.recording();
    let record_path = directory.path().join("recording.json");
    save_recording(&record_path, &recorded).unwrap();
    let mut replay = Replay::new(load_recording(&record_path).unwrap()).unwrap();
    while replay.next() {}
    assert_eq!(
        replay.current().encounter_discard(),
        game.observe().encounter_discard()
    );
    assert!(replay.back());
    assert!(
        replay.current().encounter_discard().is_empty(),
        "replay retains the final damage stage before transfer"
    );
    assert_eq!(replay.recording(), &recorded);
}

#[test]
fn fresh_process_and_replay_expose_discards_without_changing_legacy_observations() {
    let directory = tempfile::tempdir().unwrap();
    for (name, bytes, args, expected) in [
        (
            "tony.json",
            include_bytes!("fixtures/encounter-discard/after-defeat.session.json").as_slice(),
            vec!["--experiment", "tony"],
            "01160",
        ),
        (
            "jacqueline.json",
            include_bytes!("fixtures/result-explanation/late.session.json").as_slice(),
            vec![],
            "01163",
        ),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        let original: Value = serde_json::from_slice(bytes).unwrap();
        let mut args = args;
        args.extend(["--load", path.to_str().unwrap()]);
        let live = driver(
            directory.path(),
            &args,
            "{\"op\":\"observe\"}\n{\"op\":\"save\"}\n{\"op\":\"quit\"}\n",
        );
        assert_eq!(live[0]["encounter_discard"][0]["code"], expected);
        assert_eq!(
            live[0]["observation"],
            *original["payload"]["moments"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let replay = driver(
            directory.path(),
            &args,
            "{\"op\":\"replay\"}\n{\"op\":\"next\"}\n{\"op\":\"back\"}\n{\"op\":\"live\"}\n{\"op\":\"quit\"}\n",
        );
        assert_eq!(replay[1]["encounter_discard"], serde_json::json!([]));
        assert_eq!(replay[3]["encounter_discard"], serde_json::json!([]));
        assert_eq!(replay[4]["encounter_discard"][0]["code"], expected);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    let path = directory.path().join("continued-tony.json");
    let bytes = include_bytes!("fixtures/encounter-discard/before-defeat.session.json");
    std::fs::write(&path, bytes).unwrap();
    let args = ["--experiment", "tony", "--load", path.to_str().unwrap()];
    let continued = driver(
        directory.path(),
        &args,
        "{\"op\":\"choose\",\"revision\":7,\"id\":\"resolve\"}\n{\"op\":\"quit\"}\n",
    );
    assert_eq!(continued[0]["encounter_discard"], serde_json::json!([]));
    assert_eq!(continued[1]["encounter_discard"][0]["code"], "01160");
    let reopened = driver(directory.path(), &args, "{\"op\":\"quit\"}\n");
    assert_eq!(
        reopened[0]["encounter_discard"],
        continued[1]["encounter_discard"]
    );
    assert_eq!(reopened[0]["observation"], continued[1]["observation"]);
}

#[test]
fn rotting_remains_enters_encounter_discard_only_after_its_revelation_finishes() {
    use arkham_unfolded_experiment::{Fixture, Session, Stage};
    for (fixture, cancel, expected_horror) in [
        (Fixture::curse(), "cancel-0-1", 0),
        (Fixture::auto_fail(), "cancel-1-2", 3),
    ] {
        let mut game = Session::new(fixture);
        assert!(game.observe().encounter_discard().is_empty());
        for id in ["commit-promise", "use-jacqueline"] {
            game.choose(game.observe().revision, id).unwrap();
            assert!(game.observe().encounter_discard().is_empty());
        }
        let mut abandoned = game.clone();
        abandoned
            .choose(abandoned.observe().revision, "end")
            .unwrap();
        assert_eq!(abandoned.observe().stage, Stage::Ended);
        assert!(abandoned.observe().encounter_discard().is_empty());

        game.choose(game.observe().revision, cancel).unwrap();
        let done = game.observe();
        assert_eq!(done.stage, Stage::Finished);
        assert_eq!(done.horror_dealt, expected_horror);
        assert_eq!(
            done.encounter_discard()
                .iter()
                .map(|c| c.code)
                .collect::<Vec<_>>(),
            ["01163"]
        );
        assert_eq!(done.encounter_discard()[0].title, "Rotting Remains");
    }
}

#[test]
fn defeating_the_ghoul_retains_it_in_encounter_discard_after_the_final_damage_stage() {
    let mut game = tony::Session::with_draws(vec![TokenFace::Zero, TokenFace::Zero]);
    assert!(game.observe().encounter_discard().is_empty());
    for id in [
        "fight",
        "commit:none",
        "reveal",
        "resolve",
        "fight",
        "commit:none",
        "reveal",
    ] {
        game.choose(game.observe().revision, id).unwrap();
        assert!(game.observe().encounter_discard().is_empty());
    }
    let before = game.observe();
    game.choose(before.revision, "resolve").unwrap();
    let defeated = game.observe();
    assert_eq!(
        defeated
            .encounter_discard()
            .iter()
            .map(|c| c.code)
            .collect::<Vec<_>>(),
        ["01160"]
    );
    assert_eq!(defeated.encounter_discard()[0].title, "Ghoul Minion");
    assert_eq!(defeated.resources, 6, "the bounty is paid once");
    assert_eq!(defeated.bag_count, 16, "the token returns at test end");
    assert!(
        defeated.discard.is_empty(),
        "the enemy is not Tony's discard"
    );
    let stages = tony::presentation::stages(&before, &defeated);
    assert!(
        stages[0].view.encounter_discard().is_empty(),
        "show final damage before the transfer"
    );
    assert_eq!(
        stages.last().unwrap().view.encounter_discard(),
        defeated.encounter_discard()
    );

    game.choose(defeated.revision, "injury").unwrap();
    let done = game.observe();
    assert_eq!(
        done.discard
            .iter()
            .map(|c| c.code.as_str())
            .collect::<Vec<_>>(),
        ["02038"]
    );
    assert_eq!(done.encounter_discard(), defeated.encounter_discard());
}
