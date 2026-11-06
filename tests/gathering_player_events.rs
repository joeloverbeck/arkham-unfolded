use arkham_unfolded_experiment::gathering::{
    HostileFixture, PlayerEventFixture, Session, continuity,
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
        let id = [
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
        .unwrap_or_else(|| panic!("Unexpected {:?}", view.choices));
        choose(session, id);
    }
    panic!("Test not finished");
}

#[test]
fn winging_it_preserves_its_origin_through_the_test_and_shuffle_destination() {
    for case in [
        PlayerEventFixture::WingingHand,
        PlayerEventFixture::WingingDiscard,
        PlayerEventFixture::WingingFailure,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::PlayerEvents(case));
        let before = session.observe().play.unwrap();
        let from_discard = case != PlayerEventFixture::WingingHand;
        let event = if from_discard {
            &before.players[1].discard
        } else {
            &before.players[1].hand
        }
        .iter()
        .find(|card| card.code == "04272")
        .unwrap()
        .id;
        choose(
            &mut session,
            &format!(
                "action:ordinary:play:tony:{event}:04272:{}",
                if from_discard { "discard" } else { "hand" }
            ),
        );
        let paid = session.observe().play.unwrap();
        assert_eq!(paid.players[1].resources, before.players[1].resources - 1);
        assert!(
            !paid.players[1]
                .discard
                .iter()
                .chain(&paid.players[1].hand)
                .any(|card| card.id == event)
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("winging-origin.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        finish_test(&mut session);
        finish_test(&mut reopened);
        assert_eq!(session.observe(), reopened.observe());
        let after = session.observe().play.unwrap();
        assert_eq!(after.completed_tests.last().unwrap().difficulty, 0);
        assert_eq!(
            after.players[1].clues,
            before.players[1].clues
                + if case == PlayerEventFixture::WingingFailure {
                    0
                } else if from_discard {
                    2
                } else {
                    1
                }
        );
        assert_eq!(
            after.players[1].deck_count,
            before.players[1].deck_count + usize::from(from_discard)
        );
        assert_eq!(
            after.players[1].discard.iter().any(|card| card.id == event),
            !from_discard
        );
        assert_eq!(after.actions_remaining[1], 2);
        session.validate().unwrap();
    }
}

#[test]
fn scrounge_targets_printed_level_zero_and_rejects_no_level_as_zero() {
    for case in [
        PlayerEventFixture::Scrounge,
        PlayerEventFixture::ScroungeNoMatch,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::PlayerEvents(case));
        let before = session.observe();
        let player = &before.play.as_ref().unwrap().players[1];
        let event = player
            .hand
            .iter()
            .find(|card| card.code == "06165")
            .unwrap()
            .id;
        let play = format!("action:ordinary:play:tony:{event}:06165");
        if case == PlayerEventFixture::ScroungeNoMatch {
            assert!(!before.choices.iter().any(|choice| choice.id == play));
            let unchanged = session.clone();
            assert!(session.choose(before.revision, &play).is_err());
            assert_eq!(session, unchanged);
            continue;
        }
        choose(&mut session, &play);
        let pending = session.observe();
        let targets: Vec<_> = pending
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("event:recover:"))
            .collect();
        assert_eq!(targets.len(), 1, "Violin and Watcher have no printed level");
        let decision = targets[0].id.clone();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("scrounge-target.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        choose(&mut session, &decision);
        choose(&mut reopened, &decision);
        assert_eq!(session.observe(), reopened.observe());
        let after = session.observe().play.unwrap();
        assert!(
            after.players[1]
                .hand
                .iter()
                .any(|card| card.code == "01080")
        );
        assert!(after.players[1].discard.iter().any(|card| card.id == event));
        session.validate().unwrap();
    }
}

#[test]
fn old_keyring_reduces_shroud_and_spends_keys_only_on_success() {
    for case in [
        PlayerEventFixture::KeyringSuccess,
        PlayerEventFixture::KeyringFailure,
        PlayerEventFixture::KeyringLast,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::PlayerEvents(case));
        let before = session.observe().play.unwrap();
        let keyring = before.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.code == "60507")
            .unwrap()
            .card
            .id;
        choose(
            &mut session,
            &format!("action:ordinary:investigate:spell:{keyring}"),
        );
        assert_eq!(
            session.observe().play.unwrap().players[1]
                .assets
                .iter()
                .find(|asset| asset.card.id == keyring)
                .unwrap()
                .uses,
            if case == PlayerEventFixture::KeyringLast {
                1
            } else {
                2
            },
            "No key is spent as an activation cost"
        );
        if case == PlayerEventFixture::KeyringLast {
            let resourceful = before.players[1]
                .hand
                .iter()
                .find(|card| card.code == "03039")
                .unwrap()
                .id;
            choose(&mut session, "window:pass");
            choose(&mut session, &format!("commit:{resourceful}"));
            choose(&mut session, "commit:confirm");
            if session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id == "window:pass")
            {
                choose(&mut session, "window:pass");
            }
            choose(&mut session, "test:reveal");
            let decision = session
                .observe()
                .choices
                .iter()
                .find(|choice| {
                    choice.id.starts_with("result:") && choice.label.contains("Old Keyring")
                })
                .unwrap()
                .id
                .clone();
            choose(&mut session, &decision);
            if let Some(choice) = session.observe().choices.iter().find(|choice| {
                choice.id.starts_with("result:") && choice.label.contains("Resourceful")
            }) {
                choose(&mut session, &choice.id);
            }
            let pending = session.observe();
            assert!(
                !pending.play.as_ref().unwrap().players[1]
                    .assets
                    .iter()
                    .any(|asset| asset.card.id == keyring)
            );
            let recover = format!("resourceful:{resourceful}:{keyring}");
            choose(&mut session, &recover);
            assert!(
                session.observe().play.unwrap().players[1]
                    .hand
                    .iter()
                    .any(|card| card.id == keyring)
            );
        } else {
            finish_test(&mut session);
            let after = session.observe().play.unwrap();
            assert_eq!(after.completed_tests.last().unwrap().difficulty, 2);
            assert_eq!(
                after.players[1]
                    .assets
                    .iter()
                    .find(|asset| asset.card.id == keyring)
                    .unwrap()
                    .uses,
                if case == PlayerEventFixture::KeyringSuccess {
                    1
                } else {
                    2
                }
            );
        }
        session.validate().unwrap();
    }
}

#[test]
fn old_keyring_last_key_is_discarded_automatically_without_resourceful() {
    let mut session = Session::with_hostile_fixture(HostileFixture::PlayerEvents(
        PlayerEventFixture::KeyringLast,
    ));
    let before = session.observe().play.unwrap();
    let keyring = before.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60507")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:investigate:spell:{keyring}"),
    );
    finish_test(&mut session);
    let after = session.observe().play.unwrap();
    assert!(
        !after.players[1]
            .assets
            .iter()
            .any(|asset| asset.card.id == keyring)
    );
    assert!(
        after.players[1]
            .discard
            .iter()
            .any(|card| card.id == keyring)
    );
    assert_eq!(after.players[1].clues, before.players[1].clues + 1);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("keyring.json");
    continuity::save_session(&path, &session).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), session.observe());
}
