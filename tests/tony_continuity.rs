use arkham_unfolded_experiment::tony::{
    Session,
    continuity::{commit_choice, load_session, save_session},
};

#[test]
fn legacy_empty_commitment_resumes_at_reveal_without_rewriting_its_record() {
    use arkham_unfolded_experiment::tony::{
        Stage,
        continuity::{load_recording, previous_path},
    };
    use sha2::{Digest, Sha256};
    // Captured through the unmodified Tony driver at cfd57848aba9cd8a74f527b8cd15ab334cd26ac0.
    let original = include_bytes!("fixtures/tony-empty-commit/pending.session.json");
    let recorded = include_bytes!("fixtures/tony-empty-commit/pending.recording.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(original)),
        "e5b23ca1947bbece8c797e9269cb2d919d72da13a1a1d1d8f70149152e05ebf7"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(recorded)),
        "e5bbb0594e7f84ce2a895f029b6131de8d5881c172a34e66181d5fb3bbbee480"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.session.json");
    let record_path = directory.path().join("legacy.recording.json");
    std::fs::write(&path, original).unwrap();
    std::fs::write(&record_path, recorded).unwrap();
    let mut game = load_session(&path).unwrap();
    let old_record = load_recording(&record_path).unwrap();
    assert_eq!(old_record.moments.last().unwrap().stage, Stage::Commit);
    assert_eq!(game.recording(), old_record);
    assert_eq!(game.observe().stage, Stage::Reveal);
    assert_eq!(
        game.observe()
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["reveal", "undo"]
    );
    save_session(&path, &game).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let revision = game.observe().revision;
    commit_choice(&path, &mut game, revision, "reveal").unwrap();
    assert_eq!(game.observe().stage, Stage::Resolve);
    assert!(!game.observe().can_undo);
    assert!(game.recording().moments.starts_with(&old_record.moments));
    assert_eq!(load_session(&path).unwrap(), game);
    assert_eq!(std::fs::read(previous_path(&path)).unwrap(), original);
    assert_eq!(std::fs::read(&record_path).unwrap(), recorded);
}

#[test]
fn legacy_empty_commitment_undo_returns_to_the_action_before_and_after_declining() {
    use arkham_unfolded_experiment::tony::Stage;
    use sha2::{Digest, Sha256};
    let pending = include_bytes!("fixtures/tony-empty-commit/pending.session.json");
    let declined = include_bytes!("fixtures/tony-empty-commit/declined.session.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(declined)),
        "a971b268341ce060adb2f0f18fcaf35559a4699ce199b3f6750b3731ec9e7e89"
    );
    for original in [pending.as_slice(), declined.as_slice()] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy.session.json");
        std::fs::write(&path, original).unwrap();
        let mut game = load_session(&path).unwrap();
        let history = game.recording().moments;
        let revision = game.observe().revision;
        commit_choice(&path, &mut game, revision, "undo").unwrap();
        let restored = game.observe();
        assert_eq!(restored.stage, Stage::Action);
        assert_eq!(
            (restored.ordinary_actions, restored.bonus_action),
            (3, true)
        );
        assert_eq!((restored.damage, restored.horror), (0, 0));
        assert_eq!(restored.test, None);
        assert!(!restored.can_undo);
        assert!(game.recording().moments.starts_with(&history));
        assert_eq!(load_session(&path).unwrap(), game);
    }
}

#[test]
fn empty_commitment_save_failure_and_reopening_keep_the_same_action_and_randomness() {
    use arkham_unfolded_experiment::tony::Stage;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session.json");
    let blocked = directory.path().join("blocked");
    std::fs::write(&blocked, b"owner bytes").unwrap();
    let mut game = Session::new().unwrap();
    let before = game.clone();
    assert!(commit_choice(blocked.join("session.json"), &mut game, 0, "evade").is_err());
    assert_eq!(game, before);
    commit_choice(&path, &mut game, 0, "evade").unwrap();
    assert_eq!(game.observe().stage, Stage::Reveal);
    let mut resumed = load_session(&path).unwrap();
    assert_eq!(resumed, game);
    let mut control = before;
    control.choose(0, "evade").unwrap();
    control.choose(1, "reveal").unwrap();
    assert!(commit_choice(blocked.join("session.json"), &mut resumed, 1, "reveal").is_err());
    assert_eq!(resumed, game);
    commit_choice(&path, &mut resumed, 1, "reveal").unwrap();
    assert_eq!(resumed, control);
    assert_eq!(load_session(&path).unwrap(), resumed);
    assert_eq!(std::fs::read(&blocked).unwrap(), b"owner bytes");
}

#[test]
fn legacy_projection_cannot_replace_the_last_saved_observation() {
    use sha2::{Digest, Sha256};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session.json");
    let original = include_bytes!("fixtures/tony-empty-commit/pending.session.json");
    std::fs::write(&path, original).unwrap();
    let game = load_session(&path).unwrap();
    let mut invalid: serde_json::Value = serde_json::from_slice(original).unwrap();
    invalid["payload"]["moments"][1] = serde_json::to_value(game.observe()).unwrap();
    let checksum = Sha256::digest(
        serde_json::to_vec(&(
            &invalid["format_version"],
            &invalid["kind"],
            &invalid["source_revision"],
            &invalid["app_version"],
            &invalid["payload"],
        ))
        .unwrap(),
    );
    invalid["checksum"] = serde_json::json!(format!("{checksum:x}"));
    let bytes = serde_json::to_vec(&invalid).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(load_session(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn replay_shows_one_combined_attack_before_the_injury_action_finishes() {
    use arkham_unfolded_experiment::tony::continuity::Replay;
    let mut game = Session::new().unwrap();
    game.choose(0, "injury").unwrap();
    let settled = game.observe();
    let mut replay = Replay::new(game.recording()).unwrap();
    assert_eq!(replay.len(), 3);
    assert!(replay.next());
    assert_eq!((replay.current().damage, replay.current().horror), (1, 1));
    assert!(
        replay.current().injury,
        "the attack precedes removing Injury"
    );
    assert!(replay.current().choices.is_empty());
    assert_eq!(replay.current().ordinary_actions, 1);
    assert!(replay.next());
    assert_eq!(replay.current(), &settled);
    assert!(!replay.next());
    assert!(replay.back());
    assert_eq!((replay.current().damage, replay.current().horror), (1, 1));
    assert_eq!(game.observe(), settled);
}

#[test]
fn failed_save_preserves_the_pending_test_and_future_random_draw() {
    let directory = tempfile::tempdir().unwrap();
    let mut game = Session::new().unwrap();
    game.choose(0, "fight").unwrap();
    game.choose(1, "commit:none").unwrap();
    let before = game.clone();
    let blocked = directory.path().join("blocked");
    std::fs::write(&blocked, b"owner bytes").unwrap();
    assert!(commit_choice(blocked.join("session.json"), &mut game, 2, "reveal").is_err());
    assert_eq!(game, before);
    let path = directory.path().join("session.json");
    save_session(&path, &game).unwrap();
    commit_choice(&path, &mut game, 2, "reveal").unwrap();
    let mut control = before;
    control.choose(2, "reveal").unwrap();
    assert_eq!(game.observe(), control.observe());
    assert_eq!(load_session(&path).unwrap(), game);
    assert_eq!(std::fs::read(&blocked).unwrap(), b"owner bytes");
}

#[test]
fn undo_and_replay_keep_the_append_only_record_and_refuse_stale_saves() {
    use arkham_unfolded_experiment::tony::continuity::{Replay, load_recording, save_recording};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let record_path = directory.path().join("record.json");
    let mut game = Session::new().unwrap();
    save_session(&path, &game).unwrap();
    let stale = game.clone();
    commit_choice(&path, &mut game, 0, "injury").unwrap();
    let after_injury = game.observe();
    save_recording(&record_path, &game.recording()).unwrap();
    let exported = std::fs::read(&record_path).unwrap();
    commit_choice(&path, &mut game, 1, "undo").unwrap();
    let restored = game.observe();
    assert_eq!(
        (
            restored.ordinary_actions,
            restored.damage,
            restored.horror,
            restored.injury
        ),
        (3, 0, 0, true)
    );
    assert_eq!(restored.revision, 2);
    assert_eq!(game.recording().moments[1], after_injury);
    assert!(save_session(&path, &stale).is_err());
    assert!(save_recording(&record_path, &game.recording()).is_err());
    assert_eq!(std::fs::read(&record_path).unwrap(), exported);
    let mut replay = Replay::new(load_recording(&record_path).unwrap()).unwrap();
    assert!(replay.next());
    assert!(replay.harm().is_some());
    assert!(replay.next());
    assert_eq!(replay.current(), &after_injury);
    assert!(replay.back());
    assert_eq!(load_session(&path).unwrap().observe(), restored);
}

#[test]
fn original_tony_records_keep_aggregate_history_and_continue_without_reapplying_harm() {
    use arkham_unfolded_experiment::tony::continuity::{Replay, load_recording, previous_path};
    let directory = tempfile::tempdir().unwrap();
    let session_path = directory.path().join("old.session.json");
    let recording_path = directory.path().join("old.recording.json");
    let session_bytes = include_bytes!("fixtures/tony-v1/injury.session.json");
    let recording_bytes = include_bytes!("fixtures/tony-v1/injury.recording.json");
    std::fs::write(&session_path, session_bytes).unwrap();
    std::fs::write(&recording_path, recording_bytes).unwrap();
    let mut session = load_session(&session_path).unwrap();
    let mut replay = Replay::new(load_recording(&recording_path).unwrap()).unwrap();
    assert_eq!(replay.len(), 2, "old history remains aggregate");
    assert!(replay.next());
    assert_eq!(replay.current(), &session.observe());
    assert!(replay.harm().is_none());
    save_session(&session_path, &session).unwrap();
    assert_eq!(std::fs::read(&session_path).unwrap(), session_bytes);
    commit_choice(&session_path, &mut session, 1, "resource").unwrap();
    assert_eq!((session.observe().damage, session.observe().horror), (2, 2));
    assert_eq!(load_session(&session_path).unwrap(), session);
    assert_eq!(
        std::fs::read(previous_path(&session_path)).unwrap(),
        session_bytes
    );
    assert_eq!(std::fs::read(&recording_path).unwrap(), recording_bytes);
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&session_path).unwrap()).unwrap();
    assert_eq!(written["format_version"], 2);
    let mut resumed_replay = Replay::new(session.recording()).unwrap();
    assert_eq!(resumed_replay.len(), 4);
    assert!(resumed_replay.next());
    assert!(resumed_replay.harm().is_none());
    assert!(resumed_replay.next());
    let hit = resumed_replay.harm().unwrap();
    assert_eq!((hit.damage, hit.horror), (1, 1));
}

#[test]
fn presentation_keeps_the_draw_hidden_until_the_combined_attack_has_been_shown() {
    use arkham_unfolded_experiment::tony::presentation::stages;
    let mut game = Session::new().unwrap();
    let before = game.observe();
    game.choose(0, "draw").unwrap();
    let after = game.observe();
    let shown = stages(&before, &after);
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[0].view.hand, before.hand);
    assert_eq!(shown[0].view.deck_count, 32);
    assert!(shown[0].view.choices.is_empty());
    assert!(!shown[0].view.can_undo);
    assert!(
        !shown[0]
            .view
            .history
            .iter()
            .any(|line| line.contains("Guts"))
    );
    assert_eq!((shown[0].view.damage, shown[0].view.horror), (1, 1));
    assert_eq!(shown[1].view, after);
    assert_eq!(shown[1].view.hand.len(), 2);
    assert_eq!(shown[1].view.deck_count, 31);
    assert!(!game.observe().can_undo);
}

#[test]
fn final_draw_is_visible_before_the_later_internal_injury_stage() {
    use arkham_unfolded_experiment::{
        TokenFace,
        tony::{HarmSource, presentation::stages},
    };
    let mut game = Session::with_draws(vec![TokenFace::Zero]);
    for choice in ["fight", "commit:none", "reveal", "resolve", "draw", "draw"] {
        game.choose(game.observe().revision, choice).unwrap();
    }
    let before = game.observe();
    game.choose(before.revision, "draw").unwrap();
    let after = game.observe();
    let shown = stages(&before, &after);
    assert_eq!(shown.len(), 3);
    assert_eq!(shown[0].harm.unwrap().source, HarmSource::OpportunityAttack);
    assert_eq!(shown[0].view.hand.len(), 3);
    assert_eq!(shown[0].view.deck_count, 30);
    assert_eq!((shown[0].view.damage, shown[0].view.horror), (3, 3));
    assert_eq!(shown[1].harm.unwrap().source, HarmSource::InternalInjury);
    assert_eq!(shown[1].view.hand, after.hand);
    assert_eq!(shown[1].view.hand.len(), 4);
    assert_eq!(shown[1].view.deck_count, 29);
    assert_eq!((shown[1].view.damage, shown[1].view.horror), (4, 3));
    assert!(shown[1].view.choices.is_empty());
    assert!(!shown[1].view.can_undo);
    assert_eq!(shown[2].view, after);
    assert_eq!(game.observe(), after);
}

#[test]
fn final_tablet_fight_and_injury_are_distinct_stages_before_the_settled_turn() {
    use arkham_unfolded_experiment::{
        TokenFace,
        tony::{HarmSource, Stage, presentation::stages},
    };
    let mut game = Session::with_draws(vec![TokenFace::Zero, TokenFace::Tablet]);
    for choice in [
        "fight",
        "commit:none",
        "reveal",
        "resolve",
        "resource",
        "resource",
        "fight",
        "commit:none",
        "reveal",
    ] {
        game.choose(game.observe().revision, choice).unwrap();
    }
    let before = game.observe();
    game.choose(before.revision, "resolve").unwrap();
    let settled = game.observe();
    let shown = stages(&before, &settled);
    assert_eq!(shown.len(), 4);
    assert_eq!(shown[0].harm.unwrap().source, HarmSource::Tablet);
    assert_eq!((shown[0].view.damage, shown[0].view.ghoul.damage), (3, 1));
    assert_eq!(shown[0].view.test.as_ref().unwrap().success, None);
    assert_eq!(shown[0].view.test.as_ref().unwrap().modifier, None);
    assert_eq!(shown[0].view.held_token, Some(TokenFace::Tablet));
    assert_eq!(shown[0].view.bag_count, 15);
    assert_eq!(shown[1].harm.unwrap().source, HarmSource::BasicFight);
    assert_eq!(shown[1].view.ghoul.damage, 2);
    assert_eq!(shown[1].view.test.as_ref().unwrap().success, Some(true));
    assert_eq!(shown[1].view.test.as_ref().unwrap().modifier, Some(-2));
    assert_eq!(shown[1].view.held_token, Some(TokenFace::Tablet));
    assert_eq!(shown[1].view.bag_count, 15);
    assert!(
        shown[1].view.ghoul.alive,
        "retain the card for its final impact"
    );
    assert_eq!(shown[2].harm.unwrap().source, HarmSource::InternalInjury);
    assert_eq!(shown[2].view.damage, 4);
    assert_eq!(shown[2].view.test.as_ref().unwrap().success, Some(true));
    assert_eq!(shown[2].view.stage, Stage::Ended);
    assert_eq!(shown[2].view.held_token, None);
    assert_eq!(shown[2].view.bag_count, 16);
    assert!(!shown[2].view.ghoul.alive);
    assert_eq!(
        shown[2].view.resources, 8,
        "the earlier defeat paid its bounty"
    );
    assert!(shown[2].harm.unwrap().is_direct());
    assert!(shown[3].harm.is_none());
    assert_eq!(shown[3].view, settled);
    assert!(!settled.ghoul.alive);
    assert_eq!(settled.stage, Stage::Ended);
    assert_eq!(game.recording().moments.last(), Some(&settled));
}

#[test]
fn cultist_horror_shows_the_failed_test_before_returning_its_token() {
    use arkham_unfolded_experiment::{
        TokenFace,
        tony::{HarmSource, presentation::stages},
    };
    let mut game = Session::with_draws(vec![TokenFace::Cultist]);
    for choice in ["evade", "reveal"] {
        game.choose(game.observe().revision, choice).unwrap();
    }
    let before = game.observe();
    game.choose(before.revision, "resolve").unwrap();
    let settled = game.observe();
    let shown = stages(&before, &settled);
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[0].harm.unwrap().source, HarmSource::Cultist);
    assert_eq!(shown[0].view.test.as_ref().unwrap().success, Some(false));
    assert_eq!(shown[0].view.test.as_ref().unwrap().modifier, Some(-1));
    assert_eq!(shown[0].view.held_token, Some(TokenFace::Cultist));
    assert_eq!(shown[0].view.bag_count, 15);
    assert_eq!(shown[1].view, settled);
    assert_eq!(shown[1].view.held_token, None);
    assert_eq!(shown[1].view.bag_count, 16);
}

#[test]
fn one_two_damage_application_uses_one_presentation_stage() {
    use arkham_unfolded_experiment::tony::{Harm, HarmRecipient, HarmSource, presentation::stages};
    // Presentation-only witness: no new two-damage action is claimed for Tony.
    let before = Session::new().unwrap().observe();
    let mut after = before.clone();
    after.damage = 2;
    after.harm = Some(vec![Harm {
        source: HarmSource::Tablet,
        recipient: HarmRecipient::Tony,
        damage: 2,
        horror: 0,
    }]);
    let shown = stages(&before, &after);
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[0].harm.unwrap().damage, 2);
    assert_eq!(shown[0].view.damage, 2);
    assert_eq!(shown[1].view, after);
}

#[test]
fn restart_archives_previous_run_and_bad_records_preserve_their_bytes() {
    use arkham_unfolded_experiment::tony::continuity::restart_session;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut old = Session::new().unwrap();
    old.choose(0, "injury").unwrap();
    save_session(&path, &old).unwrap();
    let fresh = Session::new().unwrap();
    restart_session(&path, &old, &fresh).unwrap();
    let archives: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".archived-")
        })
        .collect();
    assert_eq!(archives.len(), 1);
    assert_eq!(load_session(&archives[0]).unwrap(), old);
    assert_eq!(load_session(&path).unwrap(), fresh);
    let original = std::fs::read(&path).unwrap();
    let mut envelope: serde_json::Value = serde_json::from_slice(&original).unwrap();
    envelope["source_revision"] = serde_json::json!("unsupported-source");
    let bad = serde_json::to_vec(&envelope).unwrap();
    std::fs::write(&path, &bad).unwrap();
    assert!(load_session(&path).is_err());
    assert!(save_session(&path, &fresh).is_err());
    assert!(restart_session(&path, &old, &Session::new().unwrap()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bad);
}

#[test]
fn baseline_jacqueline_records_reopen_only_through_the_retained_compatible_mode() {
    use arkham_unfolded_experiment::continuity as legacy;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let original = include_bytes!("fixtures/result-explanation/early.session.json");
    std::fs::write(&path, original).unwrap();
    let session = legacy::load_session(&path).unwrap();
    assert!(legacy::Replay::new(session.recording()).is_ok());
    assert!(load_session(&path).is_err());
    assert!(save_session(&path, &Session::new().unwrap()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn malformed_or_incompatible_records_with_valid_checksums_are_refused_without_mutation() {
    use sha2::{Digest, Sha256};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    save_session(&path, &Session::new().unwrap()).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for (pointer, value) in [
        ("/payload/state/bag", serde_json::json!([])),
        ("/payload/state/clues", serde_json::json!(255)),
        ("/source_revision", serde_json::json!("unsupported-source")),
        ("/format_version", serde_json::json!(1)),
    ] {
        let mut invalid = original.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        let checksum = Sha256::digest(
            serde_json::to_vec(&serde_json::json!([
                invalid["format_version"],
                invalid["kind"],
                invalid["source_revision"],
                invalid["app_version"],
                invalid["payload"]
            ]))
            .unwrap(),
        );
        invalid["checksum"] = serde_json::json!(format!("{checksum:x}"));
        let bytes = serde_json::to_vec(&invalid).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(load_session(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn stale_restart_cannot_replace_a_newer_active_turn() {
    use arkham_unfolded_experiment::tony::continuity::restart_session;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut old = Session::new().unwrap();
    old.choose(0, "end_turn").unwrap();
    save_session(&path, &old).unwrap();
    let mut current = Session::new().unwrap();
    restart_session(&path, &old, &current).unwrap();
    commit_choice(&path, &mut current, 0, "injury").unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(restart_session(&path, &old, &Session::new().unwrap()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(load_session(&path).unwrap(), current);
}
