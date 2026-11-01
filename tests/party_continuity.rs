use arkham_unfolded_experiment::{
    TokenFace,
    party::{
        Investigator, Session, Stage,
        continuity::{commit_choice, load_session, save_session},
    },
};

fn act_fixture(compressed: &[u8]) -> Vec<u8> {
    // Exact saves made by the accepted pre-Act-reveal working tree, snapshot
    // 4fb3eb5779673aa27d4112c6ad6ee69a841887cc8a75d385ef3ecf3b07949aab.
    let mut bytes = Vec::new();
    lzma_rs::xz_decompress(&mut std::io::Cursor::new(compressed), &mut bytes).unwrap();
    bytes
}

#[test]
fn act_reading_persists_payment_before_effects_and_failures_are_atomic() {
    use arkham_unfolded_experiment::party::continuity::Replay;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("act.json");
    let old_bytes = act_fixture(include_bytes!(
        "fixtures/party-closing-before-act-reveal.session.json.xz"
    ));
    std::fs::write(&path, &old_bytes).unwrap();
    let mut game = load_session(&path).unwrap();
    let old_moments = game.recording().moments;
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
        assert!(!game.observe_for_play().can_undo);
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
    let recording = game.recording();
    assert_eq!(&recording.moments[..old_moments.len()], old_moments);
    let mut replay = Replay::new(recording).unwrap();
    while replay.next() {}
    assert_eq!(replay.current().stage, Stage::Ended);
    assert!(replay.back());
    assert_eq!(replay.current().stage, Stage::ActReveal);
    assert_eq!(replay.current().locations[0].name, "Study");
    assert!(replay.back());
    assert_eq!(replay.current().stage, Stage::Closing);
    assert_eq!(
        replay
            .current()
            .investigators
            .iter()
            .map(|i| i.clues)
            .sum::<u8>(),
        6
    );
}

#[test]
fn old_completed_act_records_remain_unchanged_without_reapplying_effects() {
    use arkham_unfolded_experiment::party::continuity::{Replay, load_recording, save_recording};

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("escaped.json");
    let bytes = act_fixture(include_bytes!(
        "fixtures/party-escaped-before-act-reveal.session.json.xz"
    ));
    std::fs::write(&path, &bytes).unwrap();
    let mut game = load_session(&path).unwrap();
    assert_eq!(game.observe().stage, Stage::Ended);
    let before = game.clone();
    for id in ["advance", "continue-act", "undo"] {
        let revision = game.observe().revision;
        assert!(commit_choice(&path, &mut game, revision, id).is_err());
        assert_eq!(game, before);
    }
    save_session(&path, &game).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let recording_path = directory.path().join("old-recording.json");
    let bytes = act_fixture(include_bytes!(
        "fixtures/party-escaped-before-act-reveal.recording.json.xz"
    ));
    std::fs::write(&recording_path, &bytes).unwrap();
    let recording = load_recording(&recording_path).unwrap();
    assert_eq!(
        recording.source_revision,
        "7083162:issue-55-study-escape-v1"
    );
    assert!(
        !recording
            .moments
            .iter()
            .any(|view| view.stage == Stage::ActReveal)
    );
    assert_eq!(recording.moments, game.recording().moments);
    let mut replay = Replay::new(recording.clone()).unwrap();
    while replay.next() {}
    assert_eq!(replay.current(), &game.observe());
    save_recording(&recording_path, &recording).unwrap();
    assert_eq!(std::fs::read(&recording_path).unwrap(), bytes);
    let exported = directory.path().join("exported-old-recording.json");
    save_recording(&exported, &recording).unwrap();
    assert_eq!(load_recording(&exported).unwrap(), recording);
    assert_eq!(std::fs::read(&exported).unwrap(), bytes);
}

#[test]
fn reveal_records_require_the_new_known_protocol_identity() {
    use arkham_unfolded_experiment::party::continuity::{load_recording, save_recording};
    use sha2::{Digest, Sha256};

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("act.json");
    let bytes = act_fixture(include_bytes!(
        "fixtures/party-closing-before-act-reveal.session.json.xz"
    ));
    std::fs::write(&path, bytes).unwrap();
    let mut game = load_session(&path).unwrap();
    let revision = game.observe().revision;
    commit_choice(&path, &mut game, revision, "advance").unwrap();
    let valid: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for source in [
        "7083162:issue-55-study-escape-v1",
        "unrecognized-party-protocol",
    ] {
        let mut altered = valid.clone();
        altered["source_revision"] = source.into();
        // A valid checksum ensures rejection is specifically the version boundary.
        let content = serde_json::to_vec(&(
            &altered["format_version"],
            &altered["kind"],
            &altered["source_revision"],
            &altered["app_version"],
            &altered["payload"],
        ))
        .unwrap();
        altered["checksum"] = format!("{:x}", Sha256::digest(content)).into();
        let bytes = serde_json::to_vec(&altered).unwrap();
        let refused = directory.path().join("refused.json");
        std::fs::write(&refused, &bytes).unwrap();
        assert!(load_session(&refused).is_err());
        assert!(save_session(&refused, &game).is_err());
        assert_eq!(std::fs::read(&refused).unwrap(), bytes);
    }
    let mut recording = game.recording();
    recording.source_revision = "7083162:issue-55-study-escape-v1".into();
    assert!(arkham_unfolded_experiment::party::continuity::Replay::new(recording).is_err());

    let recording = game.recording();
    let recording_path = directory.path().join("mismatched-recording.json");
    save_recording(&recording_path, &recording).unwrap();
    let mut altered: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&recording_path).unwrap()).unwrap();
    altered["source_revision"] = "7083162:issue-55-study-escape-v1".into();
    let content = serde_json::to_vec(&(
        &altered["format_version"],
        &altered["kind"],
        &altered["source_revision"],
        &altered["app_version"],
        &altered["payload"],
    ))
    .unwrap();
    altered["checksum"] = format!("{:x}", Sha256::digest(content)).into();
    let bytes = serde_json::to_vec(&altered).unwrap();
    std::fs::write(&recording_path, &bytes).unwrap();
    assert!(load_recording(&recording_path).is_err());
    assert!(save_recording(&recording_path, &recording).is_err());
    assert_eq!(std::fs::read(&recording_path).unwrap(), bytes);
}

#[test]
fn legacy_finished_undo_skips_cleanup_and_forced_resolution_preserving_real_choices() {
    // Both fixtures are exact ef6daf2 saves, compressed without changing bytes.
    for (compressed, meaningful) in [
        (
            include_bytes!("fixtures/party-finished-ef6daf2.session.json.xz").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/party-finished-order-ef6daf2.session.json.xz").as_slice(),
            true,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy.json");
        let mut original = Vec::new();
        lzma_rs::xz_decompress(&mut std::io::Cursor::new(compressed), &mut original).unwrap();
        std::fs::write(&path, &original).unwrap();
        let mut game = load_session(&path).unwrap();
        let before = game.clone();
        let recording = game.recording();
        assert_eq!(game.observe().stage, Stage::Action);
        assert_eq!(game.observe_for_play().can_undo, meaningful);
        assert_eq!(
            game.observe_for_play()
                .choices
                .iter()
                .any(|choice| choice.id == "undo"),
            meaningful
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let revision = game.observe().revision;
        if meaningful {
            commit_choice(&path, &mut game, revision, "undo").unwrap();
            assert_eq!(game.observe().stage, Stage::Resolve);
            assert_eq!(game.observe().bag_count, 13);
            assert_eq!(
                game.observe()
                    .choices
                    .iter()
                    .filter(|choice| choice.id.starts_with("resolve:"))
                    .count(),
                2
            );
            assert_eq!(game.observe().investigator(Investigator::Tony).clues, 0);
            assert_eq!(
                &game.recording().moments[..recording.moments.len()],
                recording.moments
            );
            let revision = game.observe().revision;
            commit_choice(&path, &mut game, revision, "undo").unwrap();
            assert_eq!(game.observe().stage, Stage::Cancellation);
            assert!(!game.observe().can_undo);
            assert_eq!(load_session(&path).unwrap(), game);
        } else {
            assert!(commit_choice(&path, &mut game, revision, "undo").is_err());
            assert_eq!(game, before);
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
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
fn original_result_save_and_recording_remain_readable_without_rewriting_history() {
    // Generated using the unmodified ef6daf2 baseline: Tony commits On the Hunt
    // and resolves a zero, leaving its former explicit Result boundary pending.
    let original = include_bytes!("fixtures/party-result-ef6daf2.session.json");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    std::fs::write(&path, original).unwrap();
    let mut game = load_session(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(game.observe().stage, Stage::Result);
    assert_eq!(game.observe().bag_count, 15);
    let recording = game.recording();
    let mut replay =
        arkham_unfolded_experiment::party::continuity::Replay::new(recording.clone()).unwrap();
    while replay.next() {}
    assert_eq!(replay.current().stage, Stage::Result);
    assert_eq!(replay.current().bag_count, 15);
    let revision = game.observe().revision;
    commit_choice(&path, &mut game, revision, "finish-test").unwrap();
    assert_eq!(game.observe().bag_count, 16);
    assert_eq!(game.observe().test.as_ref().unwrap().total, Some(4));
    assert!(game.observe().held_tokens().is_empty());
    assert!(!game.observe().can_undo);
    assert_eq!(
        &game.recording().moments[..recording.moments.len()],
        recording.moments
    );
    assert_eq!(load_session(&path).unwrap(), game);
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
