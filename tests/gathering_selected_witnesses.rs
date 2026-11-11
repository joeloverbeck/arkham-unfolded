//! Independent original-face expectations bound in issue-89-rules-scenario-tony
//! and issue-89-rules-patrice. Public Session consumers, no private engine calls.

use arkham_unfolded_experiment::gathering::{
    HostileFixture, Investigator, SelectedWitness, Session, continuity,
};

fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}

fn to_commit(session: &mut Session) {
    while session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(session, "window:pass");
    }
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm"),
        "Expected commitment: {:?}",
        session.observe().choices
    );
}

fn reveal(session: &mut Session) {
    for _ in 0..8 {
        let choices = session.observe().choices;
        if choices.iter().any(|choice| choice.id == "window:pass") {
            choose(session, "window:pass");
        } else if choices.iter().any(|choice| choice.id == "commit:confirm") {
            choose(session, "commit:confirm");
        } else {
            break;
        }
    }
    choose(session, "test:reveal");
}

fn session(case: SelectedWitness) -> Session {
    Session::with_hostile_fixture(HostileFixture::SelectedWitness(case))
}

#[test]
fn original_guts_draws_one_for_its_owner_after_success_and_rejects_a_second_copy() {
    let mut game = session(SelectedWitness::Guts);
    let before = game.observe().play.unwrap();
    let guts: Vec<_> = before.players[0]
        .hand
        .iter()
        .filter(|card| card.code == "01089")
        .map(|card| card.id)
        .collect();
    assert_eq!(guts.len(), 2);
    choose(&mut game, "revelation:resolve");
    to_commit(&mut game);
    choose(&mut game, &format!("commit:{}", guts[0]));
    let selected = game.observe();
    assert!(
        !selected
            .choices
            .iter()
            .any(|choice| choice.id == format!("commit:{}", guts[1]))
    );
    assert!(
        game.choose(selected.revision, &format!("commit:{}", guts[1]))
            .is_err()
    );
    assert_eq!(game.observe(), selected);
    reveal(&mut game);
    let after = game.observe().play.unwrap();
    assert_eq!(
        after.completed_tests.last().unwrap().total,
        4,
        "Tony willpower 2 + original Guts 2 + 0"
    );
    assert!(after.completed_tests.last().unwrap().success);
    assert_eq!(
        after.players[0].deck_count,
        before.players[0].deck_count - 1
    );
    assert!(
        after.players[0]
            .hand
            .iter()
            .any(|card| card.code == "01049")
    );
    assert!(
        after.players[0]
            .discard
            .iter()
            .any(|card| card.id == guts[0])
    );
    assert!(after.players[0].hand.iter().any(|card| card.id == guts[1]));
    assert_eq!(after.players[0].horror, 0);
    assert!(
        after
            .encounter_discard
            .iter()
            .any(|card| card.code == "01163")
    );
    assert_eq!(after.bag_count, 16);
}

#[test]
fn guts_drawn_weakness_resolves_automatically_and_the_original_test_completes() {
    // Original Guts successful draw; RR p21 resolves a drawn encounter-type
    // weakness as if drawn from the encounter deck, before resuming ST7/ST8.
    let mut game = session(SelectedWitness::GutsWeakness);
    let before = game.observe().play.unwrap();
    let original = before.pending_revelation.unwrap().card.id;
    let guts = before.players[0]
        .hand
        .iter()
        .find(|card| card.code == "01089")
        .unwrap()
        .id;
    choose(&mut game, "revelation:resolve");
    to_commit(&mut game);
    choose(&mut game, &format!("commit:{guts}"));
    reveal(&mut game);
    let completed = game.observe();
    let play = completed.play.as_ref().unwrap();
    assert!(
        play.pending_revelation.is_none(),
        "Internal Injury has no revelation choice; it automatically enters the threat area"
    );
    assert!(
        !completed
            .choices
            .iter()
            .any(|choice| choice.id == "revelation:resolve")
    );
    assert_eq!(play.players[0].deck_count, before.players[0].deck_count - 1);
    assert!(!play.players[0].hand.iter().any(|card| card.code == "02038"));
    assert_eq!(play.completed_tests.last().unwrap().total, 4);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guts-weakness.json");
    continuity::save_session(&path, &game).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), completed);
    let after = reopened.observe().play.unwrap();
    assert!(
        after.players[0]
            .threats
            .iter()
            .any(|card| card.code == "02038")
    );
    assert_eq!(
        after.players[0].damage, 0,
        "Internal Injury's damage waits until the end of its owner's turn"
    );
    assert!(
        after
            .encounter_discard
            .iter()
            .any(|card| card.id == original)
    );
    assert!(after.players[0].discard.iter().any(|card| card.id == guts));
    assert!(after.pending_revelation.is_none());
    assert!(after.test.is_none());
    assert_eq!(after.bag_count, 16);
}

#[test]
fn mauser_succeed_by_two_offers_both_rewards_and_preserves_the_paid_ammo_on_reopen() {
    for reward in ["ready", "resource"] {
        let mut game = session(SelectedWitness::Mauser);
        let before = game.observe().play.unwrap();
        let weapon = before.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.code == "60306")
            .unwrap()
            .card
            .id;
        let enemy = before.scenario.enemies[0].card.id;
        choose(
            &mut game,
            &format!("action:ordinary:fight:{enemy}:weapon:{weapon}"),
        );
        reveal(&mut game);
        let pending = game.observe();
        assert!(
            pending
                .choices
                .iter()
                .any(|choice| choice.id == format!("mauser:{weapon}:ready"))
        );
        assert!(
            pending
                .choices
                .iter()
                .any(|choice| choice.id == format!("mauser:{weapon}:resource"))
        );
        assert_eq!(
            pending.play.as_ref().unwrap().test.as_ref().unwrap().total,
            Some(4),
            "Combat 5 + Mauser 1 - 2 versus Minion combat 2: succeed by exactly 2"
        );
        assert!(
            pending
                .play
                .as_ref()
                .unwrap()
                .encounter_discard
                .iter()
                .any(|card| card.id == enemy),
            "The successful attack deals two damage before its reward is chosen"
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mauser-reward.json");
        continuity::save_session(&path, &game).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        assert_eq!(reopened.observe(), pending);
        choose(&mut reopened, &format!("mauser:{weapon}:{reward}"));
        let after = reopened.observe().play.unwrap();
        let mauser = after.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == weapon)
            .unwrap();
        assert_eq!(mauser.uses, 4);
        assert_eq!(mauser.ready, reward == "ready");
        assert_eq!(
            after.players[0].resources,
            if reward == "resource" { 2 } else { 1 }
        );
        assert_eq!(after.actions_remaining[0], 2);
        assert_eq!(after.bag_count, 16);
    }
    let mut game = session(SelectedWitness::MauserNarrow);
    let before = game.observe().play.unwrap();
    let weapon = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60306")
        .unwrap()
        .card
        .id;
    let enemy = before.scenario.enemies[0].card.id;
    choose(
        &mut game,
        &format!("action:ordinary:fight:{enemy}:weapon:{weapon}"),
    );
    reveal(&mut game);
    let after = game.observe();
    assert!(
        !after
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("mauser:")),
        "Success by one does not grant either reward"
    );
    let play = after.play.unwrap();
    assert_eq!(play.completed_tests.last().unwrap().total, 3);
    assert_eq!(play.players[0].resources, 1);
    assert!(
        !play.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == weapon)
            .unwrap()
            .ready
    );
}

#[test]
fn mists_finishes_evasion_then_moves_reveals_and_applies_the_original_room_entry() {
    for (destination, damage, horror) in [("01114", 1, 0), ("01113", 0, 1), ("decline", 0, 0)] {
        let mut game = session(SelectedWitness::Mists);
        let before = game.observe().play.unwrap();
        let mists = before.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.code == "04029")
            .unwrap()
            .card
            .id;
        let evaded = before
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.code == "01160")
            .unwrap()
            .card
            .id;
        choose(
            &mut game,
            &format!("action:ordinary:evade:{evaded}:spell:{mists}"),
        );
        reveal(&mut game);
        let pending = game.observe();
        assert!(
            !pending
                .choices
                .iter()
                .any(|choice| choice.id == "spell-move:01115"),
            "Unrevealed Parlor is not a legal destination"
        );
        let enemy = pending
            .play
            .as_ref()
            .unwrap()
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == evaded)
            .unwrap();
        assert!(enemy.exhausted);
        assert_eq!(enemy.engaged, None);
        assert_eq!(enemy.location, "01112");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mists-move.json");
        continuity::save_session(&path, &game).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        assert_eq!(reopened.observe(), pending);
        choose(&mut reopened, &format!("spell-move:{destination}"));
        let after = reopened.observe().play.unwrap();
        assert_eq!(
            after.players[1].location,
            if destination == "decline" {
                "01112"
            } else {
                destination
            }
        );
        assert_eq!(after.players[1].damage, damage);
        assert_eq!(after.players[1].horror, horror);
        if destination != "decline" {
            let room = after
                .scenario
                .locations
                .iter()
                .find(|location| location.code == destination)
                .unwrap();
            assert!(room.revealed);
            assert_eq!(room.clues, 6);
        }
        let waiting = after
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.code == "01161")
            .unwrap();
        assert_eq!(
            waiting.engaged,
            if destination == "01114" {
                Some(Investigator::Patrice)
            } else {
                None
            }
        );
        assert_eq!(
            after.players[1]
                .assets
                .iter()
                .find(|asset| asset.card.id == mists)
                .unwrap()
                .uses,
            3
        );
        assert_eq!(after.actions_remaining[1], 2);
        assert_eq!(after.bag_count, 16);
    }
}

#[test]
fn initiative_loses_icons_for_completed_actions_by_the_other_investigator_in_this_phase() {
    for completed in 0..=3 {
        let mut game = session(SelectedWitness::Initiative);
        let initiative = game.observe().play.unwrap().players[0]
            .hand
            .iter()
            .find(|card| card.code == "04150")
            .unwrap()
            .id;
        if completed == 1 {
            choose(&mut game, "action:ordinary:investigate");
            while game
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id == "window:pass")
            {
                choose(&mut game, "window:pass");
            }
            let other_test = game.observe();
            assert!(
                !other_test
                    .choices
                    .iter()
                    .any(|choice| choice.id == format!("commit:{initiative}")),
                "Tony's Initiative cannot support Patrice's test"
            );
            assert!(
                game.choose(other_test.revision, &format!("commit:{initiative}"))
                    .is_err()
            );
            assert_eq!(game.observe(), other_test);
            reveal(&mut game);
        } else {
            for _ in 0..completed {
                choose(&mut game, "action:ordinary:resource");
            }
        }
        // With no action left and nothing else playable, the turn ends by itself.
        if game
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "turn:end")
        {
            choose(&mut game, "turn:end");
        }
        choose(&mut game, "turn:tony");
        choose(&mut game, "action:ordinary:investigate");
        while game
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "window:pass")
        {
            choose(&mut game, "window:pass");
        }
        if completed < 3 {
            choose(&mut game, &format!("commit:{initiative}"));
        } else {
            let no_icons = game.observe();
            assert!(
                !no_icons
                    .choices
                    .iter()
                    .any(|choice| choice.id == format!("commit:{initiative}"))
            );
            assert!(
                game.choose(no_icons.revision, &format!("commit:{initiative}"))
                    .is_err()
            );
            assert_eq!(game.observe(), no_icons);
        }
        reveal(&mut game);
        let after = game.observe().play.unwrap();
        assert_eq!(
            after.completed_tests.last().unwrap().total,
            6 - completed,
            "Tony's current investigation is not completed yet; only Patrice's earlier actions reduce the three wild icons"
        );
        assert_eq!(after.actions_remaining[0], 2);
        assert_eq!(
            after.players[0]
                .hand
                .iter()
                .any(|card| card.id == initiative),
            completed == 3
        );
    }
}

#[test]
fn grasping_hands_uses_agility_and_only_failed_margin_becomes_damage() {
    for (case, total, damage) in [
        (SelectedWitness::GraspingHands, 1, 2),
        (SelectedWitness::GraspingSuccess, 3, 0),
    ] {
        let mut game = session(case);
        let encounter = game
            .observe()
            .play
            .unwrap()
            .pending_revelation
            .unwrap()
            .card
            .id;
        choose(&mut game, "revelation:resolve");
        let test = game.observe().play.unwrap().test.unwrap();
        assert_eq!(serde_json::to_value(test.skill).unwrap(), "agility");
        assert_eq!(test.difficulty, 3);
        reveal(&mut game);
        let after = game.observe().play.unwrap();
        assert_eq!(after.completed_tests.last().unwrap().total, total);
        assert_eq!(after.players[0].damage, damage);
        assert_eq!(after.players[0].horror, 0);
        assert!(
            after
                .encounter_discard
                .iter()
                .any(|card| card.id == encounter)
        );
        assert!(after.pending_revelation.is_none());
        assert!(after.test.is_none());
        assert_eq!(after.bag_count, 16);
    }
}

#[test]
fn fog_attaches_once_modifies_the_real_investigation_and_leaves_only_after_success() {
    let mut duplicate_game = session(SelectedWitness::ObscuringFogDuplicate);
    let before = duplicate_game.observe().play.unwrap();
    let existing = before.scenario.locations[0].fog.as_ref().unwrap().id;
    let duplicate = before.pending_revelation.unwrap().card.id;
    choose(&mut duplicate_game, "revelation:resolve");
    let limited = duplicate_game.observe().play.unwrap();
    assert_eq!(
        limited.scenario.locations[0].fog.as_ref().unwrap().id,
        existing
    );
    assert!(
        limited
            .encounter_discard
            .iter()
            .any(|card| card.id == duplicate)
    );
    choose(&mut duplicate_game, "action:ordinary:investigate");
    assert_eq!(
        duplicate_game
            .observe()
            .play
            .unwrap()
            .test
            .unwrap()
            .difficulty,
        4
    );

    for (case, has_clues) in [
        (SelectedWitness::ObscuringFog, true),
        (SelectedWitness::ObscuringFogNoClues, false),
    ] {
        let mut game = session(case);
        let original = game
            .observe()
            .play
            .unwrap()
            .pending_revelation
            .unwrap()
            .card
            .id;
        choose(&mut game, "revelation:resolve");
        let attached = game.observe();
        assert_eq!(
            attached.play.as_ref().unwrap().scenario.locations[0]
                .fog
                .as_ref()
                .unwrap()
                .id,
            original
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fog-attachment.json");
        continuity::save_session(&path, &game).unwrap();
        game = continuity::load_session(&path).unwrap();
        assert_eq!(game.observe(), attached);
        choose(&mut game, "action:ordinary:investigate");
        assert_eq!(
            game.observe().play.unwrap().test.unwrap().difficulty,
            4,
            "Study 2 + one Fog 2, never two attachments"
        );
        reveal(&mut game);
        let failed = game.observe().play.unwrap();
        assert_eq!(failed.completed_tests.last().unwrap().total, 3);
        assert_eq!(failed.players[0].clues, 0);
        assert_eq!(
            failed.scenario.locations[0].fog.as_ref().unwrap().id,
            original
        );
        choose(&mut game, "action:ordinary:investigate");
        reveal(&mut game);
        let success = game.observe().play.unwrap();
        assert_eq!(success.completed_tests.last().unwrap().total, 4);
        assert_eq!(success.players[0].clues, u8::from(has_clues));
        assert!(success.scenario.locations[0].fog.is_none());
        assert!(
            success
                .encounter_discard
                .iter()
                .any(|card| card.id == original)
        );
        choose(&mut game, "action:ordinary:investigate");
        assert_eq!(game.observe().play.unwrap().test.unwrap().difficulty, 2);
        reveal(&mut game);
        assert_eq!(
            game.observe().play.unwrap().players[0].clues,
            if has_clues { 2 } else { 0 }
        );
    }
}
