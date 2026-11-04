use arkham_unfolded_experiment::{
    TokenFace,
    party::{
        Investigator, Session, Stage,
        continuity::{commit_choice, load_session, save_session},
    },
};

#[test]
fn act_reading_persists_payment_before_effects_and_failures_are_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("act.json");
    let mut game = Session::with_draws(vec![TokenFace::Zero; 6]);
    for who in ["tony", "patrice", "jacqueline"] {
        game.choose(game.observe().revision, &format!("select:{who}"))
            .unwrap();
        for _ in 0..2 {
            for id in ["investigate", "commit:none", "reveal", "resolve:0"] {
                game.choose(game.observe().revision, id).unwrap();
            }
        }
        if game.observe().stage == Stage::Action {
            game.choose(game.observe().revision, "end-turn").unwrap();
        }
    }
    save_session(&path, &game).unwrap();
    let unusable = directory.path().join("unusable");
    std::fs::create_dir(&unusable).unwrap();

    for (id, stage) in [
        ("advance", Stage::ActReveal),
        ("continue-act", Stage::Ended),
    ] {
        let before = game.clone();
        let revision = game.observe().revision;
        let prior_bytes = std::fs::read(&path).unwrap();
        assert!(commit_choice(&unusable, &mut game, revision, id).is_err());
        assert_eq!(game, before, "failed {id} must not install consequences");
        assert_eq!(std::fs::read(&path).unwrap(), prior_bytes);
        commit_choice(&path, &mut game, revision, id).unwrap();
        assert_eq!(game.observe().stage, stage);
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            saved["source_revision"],
            "7083162:issue-55-study-escape-v2-act-reveal"
        );
        game = load_session(&path).unwrap();
        assert_eq!(game.observe().stage, stage);
        assert!(game.observe().investigators.iter().all(|i| i.clues == 0));
        assert!(!game.observe().can_undo);
        if stage == Stage::ActReveal {
            assert_eq!(game.observe().locations.len(), 1);
            assert_eq!(game.observe().locations[0].name, "Study");
            assert!(
                game.observe()
                    .investigators
                    .iter()
                    .all(|i| i.location == "Study")
            );
        }
        let after = game.clone();
        assert!(commit_choice(&path, &mut game, revision, id).is_err());
        assert_eq!(
            game, after,
            "a repeated click cannot repeat the cost or effects"
        );
    }
}

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
    for id in ["cancel:0", "resolve:1", "reveal", "resolve:3", "resolve:2"] {
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
fn restart_preserves_the_previous_attempt() {
    use arkham_unfolded_experiment::party::continuity::restart_session;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    let mut game = Session::new().unwrap();
    save_session(&path, &game).unwrap();
    commit_choice(&path, &mut game, 0, "select:tony").unwrap();
    let original = std::fs::read(&path).unwrap();
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
}

#[test]
fn failed_automatic_cleanup_is_atomic_and_reopen_does_not_return_tokens_twice() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("party.json");
    let mut game = Session::with_draws(vec![TokenFace::Zero]);
    for id in ["select:tony", "investigate", "commit:none", "reveal"] {
        let revision = game.observe().revision;
        commit_choice(&path, &mut game, revision, id).unwrap();
    }
    let before = game.clone();
    let saved = std::fs::read(&path).unwrap();
    let unusable = directory.path().join("unusable");
    std::fs::create_dir(&unusable).unwrap();
    let revision = game.observe().revision;
    assert!(commit_choice(&unusable, &mut game, revision, "resolve:0").is_err());
    assert_eq!(game, before);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    commit_choice(&path, &mut game, revision, "resolve:0").unwrap();
    assert_eq!(game.observe().bag_count, 16);
    assert_eq!(game.observe().investigator(Investigator::Tony).clues, 1);
    let mut reopened = load_session(&path).unwrap();
    assert_eq!(reopened, game);
    let revision = reopened.observe().revision;
    assert!(commit_choice(&path, &mut reopened, revision, "finish-test").is_err());
    assert_eq!(load_session(&path).unwrap(), game);
}

#[test]
fn corrupt_and_other_experiment_records_are_refused_without_replacement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    let tony_path = directory.path().join("tony.json");
    arkham_unfolded_experiment::tony::continuity::save_session(
        &tony_path,
        &arkham_unfolded_experiment::tony::Session::new().unwrap(),
    )
    .unwrap();
    let jacqueline_path = directory.path().join("jacqueline.json");
    arkham_unfolded_experiment::continuity::save_session(
        &jacqueline_path,
        &arkham_unfolded_experiment::Session::new(arkham_unfolded_experiment::Fixture::curse()),
    )
    .unwrap();
    for bytes in [
        b"{broken save".to_vec(),
        std::fs::read(&tony_path).unwrap(),
        std::fs::read(&jacqueline_path).unwrap(),
    ] {
        std::fs::write(&path, &bytes).unwrap();
        assert!(load_session(&path).is_err());
        assert!(save_session(&path, &Session::new().unwrap()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
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
