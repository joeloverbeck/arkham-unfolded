use arkham_unfolded_experiment::gathering::{
    HostileFixture, Session, SpellEventFixture, continuity,
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

fn finish_test(session: &mut Session) {
    for _ in 0..12 {
        let view = session.observe();
        if view.play.as_ref().unwrap().completed_tests.len() == 1
            && !view.choices.iter().any(|choice| {
                matches!(
                    choice.id.as_str(),
                    "after-test:pass" | "search:finish-event"
                )
            })
        {
            return;
        }
        let decision = [
            "test-card:pass",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "after-test:pass",
            "search:finish-event",
        ]
        .iter()
        .find(|&&id| view.choices.iter().any(|choice| choice.id == id))
        .copied()
        .unwrap_or_else(|| panic!("Unexpected test decision: {:?}", view.choices));
        choose(session, decision);
    }
    panic!("Test did not finish");
}

#[test]
fn read_the_signs_adds_current_willpower_and_keeps_the_paid_event_until_test_cleanup() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::SpellEvents(SpellEventFixture::ReadSigns));
    let before = session.observe().play.unwrap();
    let event = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "06117")
        .unwrap()
        .id;
    let robes = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:play:tony:{event}:06117:robes:{robes}"),
    );
    let paid = session.observe().play.unwrap();
    assert_eq!(paid.players[2].resources, before.players[2].resources - 1);
    assert_eq!(paid.actions_remaining[2], 2);
    assert!(!paid.players[2].hand.iter().any(|card| card.id == event));
    assert!(!paid.players[2].discard.iter().any(|card| card.id == event));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("read-signs-test.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    finish_test(&mut session);
    finish_test(&mut reopened);
    assert_eq!(session.observe(), reopened.observe());
    let after = session.observe().play.unwrap();
    assert_eq!(
        after.completed_tests.last().unwrap().total,
        9,
        "Intellect3 + willpower5 + Crystal Pendulum1"
    );
    assert_eq!(after.players[2].clues, before.players[2].clues + 2);
    assert!(after.players[2].discard.iter().any(|card| card.id == event));
    session.validate().unwrap();
}

#[test]
fn spectral_razor_engages_before_fighting_and_distinguishes_elite_damage_and_hand_watcher() {
    for case in [
        SpellEventFixture::Razor,
        SpellEventFixture::RazorElite,
        SpellEventFixture::RazorWatcher,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::SpellEvents(case));
        let before = session.observe().play.unwrap();
        let owner = if case == SpellEventFixture::RazorWatcher {
            1
        } else {
            2
        };
        let card = before.players[owner]
            .hand
            .iter()
            .find(|card| card.code == "06201")
            .unwrap()
            .id;
        let enemy = if owner == 1 {
            before.players[owner]
                .hand
                .iter()
                .find(|card| card.code == "06017")
                .unwrap()
                .id
        } else {
            before.scenario.enemies[0].card.id
        };
        let suffix = if owner == 1 { "stay" } else { "engage" };
        choose(
            &mut session,
            &format!("action:ordinary:play:tony:{card}:06201:{enemy}:{suffix}"),
        );
        let paid = session.observe().play.unwrap();
        assert_eq!(
            paid.players[owner].resources,
            before.players[owner].resources - 2
        );
        assert_eq!(
            paid.players[owner].damage, before.players[owner].damage,
            "Fight designator avoids opportunity attacks"
        );
        if owner == 2 {
            assert_eq!(
                paid.scenario.enemies[0].engaged,
                Some(arkham_unfolded_experiment::gathering::Investigator::Jacqueline)
            );
        }
        finish_test(&mut session);
        let after = session.observe().play.unwrap();
        assert_eq!(
            after.completed_tests.last().unwrap().total,
            if owner == 1 { 6 } else { 8 }
        );
        if case == SpellEventFixture::RazorElite {
            assert_eq!(after.scenario.enemies[0].damage, 2);
        } else if owner == 1 {
            assert!(after.players[1].discard.iter().any(|card| card.id == enemy));
        } else {
            assert!(after.encounter_discard.iter().any(|card| card.id == enemy));
        }
        assert!(
            after.players[owner]
                .discard
                .iter()
                .any(|entry| entry.id == card)
        );
        session.validate().unwrap();
    }
}

#[test]
fn uncage_plays_read_the_signs_from_zero_resources_and_finishes_both_owned_events() {
    let mut session = Session::with_hostile_fixture(HostileFixture::SpellEvents(
        SpellEventFixture::UncageReadSigns,
    ));
    let before = session.observe().play.unwrap();
    let event = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "06117")
        .unwrap()
        .id;
    let uncage = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "03033")
        .unwrap()
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:play:tony:{uncage}:03033"),
    );
    // Read the Signs is the only eligible Spell event, so Uncage plays it by itself.
    assert!(
        session
            .observe()
            .history
            .iter()
            .any(|entry| entry.starts_with("Automatic: ") && entry.contains("Read the Signs"))
    );
    assert_eq!(session.observe().play.unwrap().players[2].resources, 0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("uncage-read-signs.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    finish_test(&mut session);
    finish_test(&mut reopened);
    assert_eq!(session.observe(), reopened.observe());
    let after = session.observe().play.unwrap();
    assert_eq!(after.actions_remaining[2], 2);
    assert_eq!(after.players[2].resources, 0);
    assert_eq!(after.players[2].clues, before.players[2].clues + 2);
    for id in [event, uncage] {
        assert_eq!(
            after.players[2]
                .discard
                .iter()
                .filter(|card| card.id == id)
                .count(),
            1
        );
    }
    assert!(
        after.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "60412")
            .unwrap()
            .ready,
        "Uncage's reduction already makes the Spell free"
    );
    session.validate().unwrap();
}

#[test]
fn read_signs_pays_before_opportunity_and_resumes_without_repayment() {
    let mut session = Session::with_hostile_fixture(HostileFixture::SpellEvents(
        SpellEventFixture::ReadSignsOpportunity,
    ));
    let before = session.observe().play.unwrap();
    let event = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "06117")
        .unwrap()
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:play:tony:{event}:06117"),
    );
    let paid = session.observe();
    assert_eq!(
        paid.play.as_ref().unwrap().players[2].resources,
        before.players[2].resources - 2
    );
    assert!(
        !paid.play.as_ref().unwrap().players[2]
            .hand
            .iter()
            .any(|card| card.id == event)
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("read-signs-paid.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    choose(&mut session, "harm:damage:investigator");
    finish_test(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[2].resources, before.players[2].resources - 2);
    assert_eq!(after.players[2].damage, before.players[2].damage + 1);
    assert_eq!(after.players[2].horror, before.players[2].horror + 1);
    assert!(after.players[2].discard.iter().any(|card| card.id == event));
    session.validate().unwrap();
}

#[test]
fn uncage_preserves_frozen_additional_action_costs_and_marks_the_first_qualifying_action() {
    for case in [
        SpellEventFixture::UncageRazorFrozen,
        SpellEventFixture::UncageRazorStacked,
        SpellEventFixture::UncageRazorShort,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::SpellEvents(case));
        let before = session.observe();
        let player = &before.play.as_ref().unwrap().players[2];
        let uncage = player
            .hand
            .iter()
            .find(|card| card.code == "03033")
            .unwrap()
            .id;
        let razor = player
            .hand
            .iter()
            .find(|card| card.code == "06201")
            .unwrap()
            .id;
        let outer = format!("action:ordinary:play:tony:{uncage}:03033");
        if case == SpellEventFixture::UncageRazorShort {
            assert!(
                !before.choices.iter().any(|choice| choice.id == outer),
                "The outer action must leave enough actions for a playable target"
            );
            let unchanged = session.clone();
            assert!(session.choose(before.revision, &outer).is_err());
            assert_eq!(session, unchanged);
            continue;
        }
        choose(&mut session, &outer);
        choose(
            &mut session,
            &format!("uncage:event:{razor}:06201:400:engage"),
        );
        assert_eq!(
            session.observe().play.unwrap().actions_remaining[2],
            if case == SpellEventFixture::UncageRazorStacked {
                0
            } else {
                1
            },
            "Uncage's reduction does not waive Frozen's one additional action per copy"
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("uncage-frozen-paid.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        finish_test(&mut session);
        finish_test(&mut reopened);
        assert_eq!(session.observe(), reopened.observe());
        let after = session.observe().play.unwrap();
        assert_eq!(after.players[2].resources, player.resources);
        if case == SpellEventFixture::UncageRazorFrozen {
            let second = after.players[2]
                .hand
                .iter()
                .find(|card| card.code == "06201")
                .unwrap()
                .id;
            choose(
                &mut session,
                &format!("action:ordinary:play:tony:{second}:06201:400:stay"),
            );
            assert_eq!(
                session.observe().play.unwrap().actions_remaining[2],
                0,
                "The second Fight does not pay Frozen again"
            );
        }
        session.validate().unwrap();
    }
}

#[test]
fn razor_labels_identify_enemy_copies_and_the_hidden_hand_target() {
    let session =
        Session::with_hostile_fixture(HostileFixture::SpellEvents(SpellEventFixture::RazorTargets));
    let view = session.observe();
    let labels: Vec<_> = view
        .choices
        .iter()
        .filter(|choice| {
            choice.id.starts_with("action:ordinary:play:tony:") && choice.id.contains(":06201:")
        })
        .map(|choice| choice.label.as_str())
        .collect();
    assert_eq!(labels.len(), 4);
    assert_eq!(
        labels
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4,
        "Physical copies need distinct recognizable labels"
    );
    assert!(labels.iter().any(|label| label.contains("Swarm of Rats")));
    assert!(labels.iter().any(|label|label.contains("Watcher from Another Dimension")&&label.contains("hand")));
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.contains("Ghoul Minion"))
            .count(),
        2
    );
}
