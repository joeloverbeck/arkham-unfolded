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
fn defeat_continuation_is_atomic_and_retains_action_history() {
    use tony::continuity::{commit_choice, load_session, save_session};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let mut game = tony::Session::with_draws(vec![TokenFace::Zero; 2]);
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
    }
    tony::continuity::save_session(&path, &game).unwrap();
    let prior = game.clone();
    let historical = game.observe().history;
    save_session(&path, &game).unwrap();
    let blocked = directory.path().join("blocked");
    std::fs::create_dir(&blocked).unwrap();
    assert!(commit_choice(&blocked, &mut game, prior.observe().revision, "resolve").is_err());
    assert_eq!(game, prior);
    assert!(game.observe().encounter_discard().is_empty());
    commit_choice(&path, &mut game, prior.observe().revision, "resolve").unwrap();
    assert_eq!(game.observe().encounter_discard()[0].code, "01160");
    assert!(game.observe().history.starts_with(&historical));
    assert_eq!(load_session(&path).unwrap(), game);
}

#[test]
fn fresh_process_exposes_the_saved_encounter_discard() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("continued-tony.json");
    let mut game = tony::Session::with_draws(vec![TokenFace::Zero; 2]);
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
    }
    tony::continuity::save_session(&path, &game).unwrap();
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
