use arkham_unfolded_experiment::tony::{
    Session,
    continuity::{commit_choice, load_session, save_session},
};

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
fn undo_keeps_history_and_refuses_stale_saves() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut game = Session::new().unwrap();
    save_session(&path, &game).unwrap();
    let stale = game.clone();
    commit_choice(&path, &mut game, 0, "injury").unwrap();
    let after_injury = game.observe();
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
    assert!(game.observe().history.starts_with(&after_injury.history));
    assert!(save_session(&path, &stale).is_err());
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
    assert_eq!(game.observe(), settled);
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
    after.harm = vec![Harm {
        source: HarmSource::Tablet,
        recipient: HarmRecipient::Tony,
        damage: 2,
        horror: 0,
    }];
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
