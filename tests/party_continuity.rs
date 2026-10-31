use arkham_unfolded_experiment::{
    TokenFace,
    party::{
        Investigator, Session, Stage,
        continuity::{commit_choice, load_session, save_session},
    },
};

#[test]
fn cooperation_and_curse_cancellation_resume_through_the_saved_choice_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("party.session.json");
    let mut game = Session::with_draws(vec![
        TokenFace::AutoFail,
        TokenFace::Curse,
        TokenFace::Zero,
        TokenFace::MinusTwo,
    ]);
    save_session(&path, &game).unwrap();
    for id in [
        "select:jacqueline",
        "investigate",
        "commit:100,200",
        "use-jacqueline",
    ] {
        let revision = game.observe().revision;
        commit_choice(&path, &mut game, revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let pending = game.observe();
    assert_eq!(pending.stage, Stage::Cancellation);
    assert_eq!(pending.test.as_ref().unwrap().skill_value, 8);
    assert_eq!(pending.bag_count, 14);
    assert_eq!(pending.tokens.len(), 3);
    let mut reopened = load_session(&path).unwrap();
    assert_eq!(reopened.observe(), pending);
    for id in [
        "cancel:0",
        "resolve:1",
        "reveal",
        "resolve:3",
        "resolve:2",
        "finish-test",
    ] {
        let revision = reopened.observe().revision;
        commit_choice(&path, &mut reopened, revision, id).unwrap();
    }
    let settled = reopened.observe();
    assert_eq!(settled.test.as_ref().unwrap().modifier, -4);
    assert_eq!(settled.test.as_ref().unwrap().result, Some(true));
    assert_eq!(settled.investigator(Investigator::Jacqueline).clues, 1);
    assert_eq!(
        settled
            .investigator(Investigator::Jacqueline)
            .ordinary_actions,
        2
    );
    assert_eq!(
        settled.investigator(Investigator::Patrice).discard[0].code,
        "07032"
    );
    assert_eq!(
        settled.investigator(Investigator::Jacqueline).discard[0].code,
        "60408"
    );
    assert_eq!(
        (
            settled.study_clues,
            settled.bag_count,
            settled.curses_in_pool
        ),
        (5, 16, 10)
    );
    assert_eq!(load_session(&path).unwrap().observe(), settled);
}

#[test]
fn failed_save_leaves_the_live_choice_and_random_continuation_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let mut game = Session::with_draws(vec![TokenFace::MinusTwo]);
    let path = directory.path().join("saved.json");
    for id in ["select:tony", "investigate", "commit:none"] {
        let revision = game.observe().revision;
        commit_choice(&path, &mut game, revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let before = game.clone();
    let unusable = directory.path().join("unusable");
    std::fs::create_dir(&unusable).unwrap();
    let revision = game.observe().revision;
    assert!(commit_choice(&unusable, &mut game, revision, "reveal").is_err());
    assert_eq!(game, before);
    commit_choice(&path, &mut game, revision, "reveal").unwrap();
    let mut control = before;
    control.choose(revision, "reveal").unwrap();
    assert_eq!(game, control);
}

#[test]
fn recording_replay_is_stored_and_restart_preserves_the_previous_attempt() {
    use arkham_unfolded_experiment::party::continuity::{
        Replay, load_recording, restart_session, save_recording,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    let recording_path = directory.path().join("recording.json");
    let mut game = Session::new().unwrap();
    save_session(&path, &game).unwrap();
    commit_choice(&path, &mut game, 0, "select:tony").unwrap();
    let original = std::fs::read(&path).unwrap();
    save_recording(&recording_path, &game.recording()).unwrap();
    let mut replay = Replay::new(load_recording(&recording_path).unwrap()).unwrap();
    assert_eq!(replay.current().revision, 0);
    assert!(replay.next());
    assert_eq!(replay.current(), &game.observe());
    assert!(replay.back());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let fresh = Session::new().unwrap();
    restart_session(&path, &game, &fresh).unwrap();
    assert_eq!(load_session(&path).unwrap(), fresh);
    let archived: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".archived-")
        })
        .collect();
    assert_eq!(archived.len(), 1);
    assert_eq!(std::fs::read(&archived[0]).unwrap(), original);
    let recording_bytes = std::fs::read(&recording_path).unwrap();
    assert!(save_recording(&recording_path, &fresh.recording()).is_err());
    assert_eq!(std::fs::read(&recording_path).unwrap(), recording_bytes);
}

#[test]
fn corrupt_and_other_experiment_records_are_refused_without_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    for bytes in [
        b"{broken record".as_slice(),
        include_bytes!("fixtures/tony-v1/injury.session.json").as_slice(),
        include_bytes!("fixtures/commitment/jacqueline-empty-v1.session.json").as_slice(),
    ] {
        std::fs::write(&path, bytes).unwrap();
        assert!(load_session(&path).is_err());
        assert!(save_session(&path, &Session::new().unwrap()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let tony_path = directory.path().join("tony.json");
    std::fs::write(
        &tony_path,
        include_bytes!("fixtures/tony-v1/injury.session.json"),
    )
    .unwrap();
    assert!(arkham_unfolded_experiment::tony::continuity::load_session(&tony_path).is_ok());
    let jacqueline_path = directory.path().join("jacqueline.json");
    std::fs::write(
        &jacqueline_path,
        include_bytes!("fixtures/commitment/jacqueline-empty-v1.session.json"),
    )
    .unwrap();
    assert!(arkham_unfolded_experiment::continuity::load_session(&jacqueline_path).is_ok());
}

#[test]
fn a_competing_saved_decision_cannot_be_overwritten_by_a_stale_session() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    let mut first = Session::new().unwrap();
    save_session(&path, &first).unwrap();
    let mut stale = load_session(&path).unwrap();
    commit_choice(&path, &mut first, 0, "select:tony").unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let before = stale.clone();
    assert!(commit_choice(&path, &mut stale, 0, "select:patrice").is_err());
    assert_eq!(stale, before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn final_shuffle_failure_retry_and_undo_preserve_the_next_random_reveal() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    // Privileged deterministic seed selects the worked final-shuffle case,
    // while using the ordinary physical-bag and shuffle random generator.
    let mut game = Session::with_seed(12932);
    for id in [
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
        "lucky:101",
        "finish-test",
    ] {
        let revision = game.observe().revision;
        commit_choice(&path, &mut game, revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let before = game.clone();
    let unusable = directory.path().join("unusable");
    std::fs::create_dir(&unusable).unwrap();
    let revision = game.observe().revision;
    assert!(commit_choice(&unusable, &mut game, revision, "shuffle:retain:100").is_err());
    assert_eq!(game, before);
    commit_choice(&path, &mut game, revision, "shuffle:retain:100").unwrap();
    let mut control = before;
    control.choose(revision, "shuffle:retain:100").unwrap();
    assert_eq!(game, control);
    let revision = game.observe().revision;
    commit_choice(&path, &mut game, revision, "undo").unwrap();
    assert_eq!(game.observe().stage, Stage::AfterTest);
    game = load_session(&path).unwrap();
    let revision = game.observe().revision;
    commit_choice(&path, &mut game, revision, "shuffle:retain:100").unwrap();
    for id in [
        "end-turn",
        "select:jacqueline",
        "investigate",
        "commit:none",
        "reveal",
    ] {
        game.choose(game.observe().revision, id).unwrap();
        control.choose(control.observe().revision, id).unwrap();
    }
    assert_eq!(game.observe().tokens, control.observe().tokens);
    assert!(!game.observe().can_undo);
}
