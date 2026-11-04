use arkham_unfolded_experiment::gathering::{HostileFixture, Session, TestCardFixture, continuity};

#[test]
fn promise_uses_the_finite_curse_pool_and_resolved_curses_leave_at_cleanup() {
    for case in [
        TestCardFixture::PromiseCurse,
        TestCardFixture::PromiseAtLimit,
        TestCardFixture::PromiseSealedLimit,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(case));
        let initial = session.observe().play.unwrap();
        let promise = initial.players[1]
            .hand
            .iter()
            .find(|card| card.code == "07032")
            .unwrap()
            .id;
        choose(&mut session, "window:pass");
        choose(&mut session, &format!("commit:{promise}"));
        choose(&mut session, "commit:confirm");
        let committed = session.observe().play.unwrap();
        let added = case == TestCardFixture::PromiseCurse;
        assert_eq!(committed.bag_count, initial.bag_count + usize::from(added));
        assert_eq!(
            committed.players[1].horror,
            initial.players[1].horror + if added { 0 } else { 2 }
        );
        reach_reveal(&mut session);
        choose(&mut session, "test:reveal");
        if case != TestCardFixture::PromiseAtLimit {
            assert!(
                session
                    .observe()
                    .choices
                    .iter()
                    .any(|choice| choice.id == "test:reveal"),
                "A retained curse creates another reveal before calculating the result"
            );
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("curse-extra-reveal.json");
            continuity::save_session(&path, &session).unwrap();
            let mut reopened = continuity::load_session(&path).unwrap();
            assert_eq!(reopened.observe(), session.observe());
            choose(&mut session, "test:reveal");
            choose(&mut reopened, "test:reveal");
            assert_eq!(reopened.observe(), session.observe());
        }
        let after = session.observe().play.unwrap();
        assert_eq!(
            after.completed_tests.last().unwrap().total,
            if case == TestCardFixture::PromiseAtLimit {
                8
            } else {
                6
            }
        );
        assert_eq!(after.bag_count, initial.bag_count);
        assert!(
            after.players[1]
                .discard
                .iter()
                .any(|card| card.id == promise)
        );
        if case == TestCardFixture::PromiseSealedLimit {
            assert!(
                after.players[2]
                    .discard
                    .iter()
                    .any(|card| card.code == "04199")
            );
        }
        session.validate().unwrap();
    }
}

fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}

#[test]
fn tony_elder_sign_adds_a_contract_bounty_only_when_its_effect_is_resolved() {
    for case in [
        TestCardFixture::TonyElderSign,
        TestCardFixture::TonyElderSignCancelled,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(case));
        let before = session.observe().play.unwrap();
        let bounties = before.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.code == "06010")
            .unwrap()
            .uses;
        reach_reveal(&mut session);
        let cancelled = case == TestCardFixture::TonyElderSignCancelled;
        if cancelled {
            choose(&mut session, "tokens:jacqueline");
        }
        choose(&mut session, "test:reveal");
        if cancelled {
            choose(&mut session, "tokens:cancel:0");
            choose(&mut session, "tokens:cancel:2");
            choose(&mut session, "tokens:confirm");
        }
        let after = session.observe().play.unwrap();
        assert_eq!(
            after.players[0]
                .assets
                .iter()
                .find(|asset| asset.card.code == "06010")
                .unwrap()
                .uses,
            bounties + u8::from(!cancelled)
        );
        assert_eq!(
            after.completed_tests.last().unwrap().total,
            if cancelled { 2 } else { 4 }
        );
        assert_eq!(after.bag_count, 16);
        session.validate().unwrap();
    }
}

#[test]
fn look_what_i_found_uses_the_settled_failure_margin_before_st7() {
    for case in [
        TestCardFixture::LookFound,
        TestCardFixture::LookFoundTooFar,
        TestCardFixture::LookFoundAutoFail,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(case));
        let initial = session.observe().play.unwrap();
        let event = initial.players[1]
            .hand
            .iter()
            .find(|card| card.code == "01079")
            .unwrap()
            .id;
        reach_reveal(&mut session);
        choose(&mut session, "test:reveal");
        let decision = format!("test-card:found:{event}");
        let pending = session.observe();
        if case == TestCardFixture::LookFoundTooFar {
            assert!(!pending.choices.iter().any(|choice| choice.id == decision));
            assert_eq!(
                pending.play.unwrap().players[1].resources,
                initial.players[1].resources
            );
            continue;
        }
        assert!(pending.choices.iter().any(|choice| choice.id == decision));
        assert!(
            pending.play.as_ref().unwrap().completed_tests.is_empty(),
            "After-failure reactions happen at ST6 before results and cleanup"
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("found-failed-test.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        choose(&mut session, &decision);
        choose(&mut reopened, &decision);
        assert_eq!(session.observe(), reopened.observe());
        let after = session.observe().play.unwrap();
        assert_eq!(after.players[1].clues, initial.players[1].clues + 2);
        assert_eq!(after.players[1].resources, initial.players[1].resources - 2);
        assert!(after.players[1].discard.iter().any(|card| card.id == event));
        assert!(
            !after.completed_tests.last().unwrap().success,
            "Finding clues does not turn the test into a success"
        );
        assert_eq!(
            after.completed_tests.last().unwrap().failure_margin,
            if case == TestCardFixture::LookFoundAutoFail {
                1
            } else {
                2
            }
        );
        session.validate().unwrap();
    }
}

#[test]
fn resourceful_recovers_only_an_eligible_discard_at_st7_and_has_a_no_match_outcome() {
    for case in [
        TestCardFixture::ResourcefulSuccess,
        TestCardFixture::ResourcefulFailure,
        TestCardFixture::ResourcefulNoMatch,
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(case));
        let before = session.observe().play.unwrap();
        let card = before.players[1]
            .hand
            .iter()
            .find(|card| card.code == "03039")
            .unwrap()
            .id;
        choose(&mut session, "window:pass");
        choose(&mut session, &format!("commit:{card}"));
        choose(&mut session, "commit:confirm");
        reach_reveal(&mut session);
        choose(&mut session, "test:reveal");
        if let Some(result) =
            session.observe().choices.iter().find(|choice| {
                choice.id.starts_with("result:") && choice.label.contains("Resourceful")
            })
        {
            choose(&mut session, &result.id);
        }
        let pending = session.observe();
        let recovery: Vec<_> = pending
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("resourceful:"))
            .collect();
        if case == TestCardFixture::ResourcefulSuccess {
            assert_eq!(
                recovery.len(),
                1,
                "Resourceful cannot recover itself or a neutral card"
            );
            assert!(
                !pending.play.as_ref().unwrap().players[1]
                    .discard
                    .iter()
                    .any(|entry| entry.id == card),
                "ST7 recovery precedes ST8 committed-card discard"
            );
            let decision = recovery[0].id.clone();
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("resourceful.json");
            continuity::save_session(&path, &session).unwrap();
            let mut reopened = continuity::load_session(&path).unwrap();
            choose(&mut session, &decision);
            choose(&mut reopened, &decision);
            assert_eq!(session.observe(), reopened.observe());
            assert!(
                session.observe().play.unwrap().players[1]
                    .hand
                    .iter()
                    .any(|entry| entry.code == "04200")
            );
        } else {
            assert!(recovery.is_empty());
        }
        assert!(
            session.observe().play.unwrap().players[1]
                .discard
                .iter()
                .any(|entry| entry.id == card)
        );
        session.validate().unwrap();
    }
}

#[test]
fn a_cancelled_curse_returns_to_the_bag_without_an_additional_reveal() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::TestCards(TestCardFixture::PromiseCancelled));
    let promise = session.observe().play.unwrap().players[1]
        .hand
        .iter()
        .find(|card| card.code == "07032")
        .unwrap()
        .id;
    choose(&mut session, "window:pass");
    choose(&mut session, &format!("commit:{promise}"));
    choose(&mut session, "commit:confirm");
    reach_reveal(&mut session);
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:cancel:0");
    choose(&mut session, "tokens:cancel:2");
    choose(&mut session, "tokens:confirm");
    let play = session.observe().play.unwrap();
    assert_eq!(play.completed_tests.last().unwrap().total, 8);
    assert_eq!(
        play.bag_count, 17,
        "Cancelled curse returns to the bag; only resolved curses return to the pool"
    );
    session.validate().unwrap();
}

fn reach_reveal(session: &mut Session) {
    for _ in 0..4 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            return;
        }
        choose(
            session,
            if view
                .choices
                .iter()
                .any(|choice| choice.id == "commit:confirm")
            {
                "commit:confirm"
            } else {
                "window:pass"
            },
        );
    }
    panic!("Expected the actual test's reveal window");
}

#[test]
fn crystal_prediction_is_saved_before_reveal_and_only_the_exact_margin_draws() {
    for number in [2, 3] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(
            TestCardFixture::CrystalPrediction,
        ));
        let before = session.observe().play.unwrap();
        let crystal = before.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "60411")
            .unwrap()
            .card
            .id;
        choose(&mut session, &format!("test-card:pendulum:{crystal}"));
        for _ in 0..number {
            choose(&mut session, "test-card:number:up");
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("prediction.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        assert_eq!(reopened.observe(), session.observe());
        for game in [&mut session, &mut reopened] {
            choose(game, "test-card:number:confirm");
            let named = game.observe().play.unwrap();
            assert!(
                !named.players[2]
                    .assets
                    .iter()
                    .find(|asset| asset.card.id == crystal)
                    .unwrap()
                    .ready
            );
            assert_eq!(named.players[2].deck_count, before.players[2].deck_count);
            choose(game, "test-card:pass");
            reach_reveal(game);
            assert!(
                !game
                    .observe()
                    .choices
                    .iter()
                    .any(|choice| choice.id.starts_with("test-card:number:"))
            );
            choose(game, "test:reveal");
        }
        assert_eq!(reopened.observe(), session.observe());
        let play = session.observe().play.unwrap();
        let result = play
            .completed_tests
            .last()
            .expect("Rotting Remains completed");
        assert_eq!(
            result.total, 6,
            "Printed willpower 5 plus Crystal's static modifier"
        );
        assert!(result.success);
        assert_eq!(
            play.players[2].deck_count,
            before.players[2].deck_count - usize::from(number == 3)
        );
        assert_eq!(
            play.players[2]
                .threats
                .iter()
                .any(|card| card.code == "02039"),
            number == 3,
            "The exact prediction draws and resolves the actual top-deck Chronophobia before continuing"
        );
        assert_eq!(play.players[2].horror, 2);
        assert_eq!(play.players[2].resources, 3);
        session.validate().unwrap();
    }
}

#[test]
fn prescient_names_before_reveal_and_recovers_only_a_spell_after_cleanup() {
    for (case, recover) in [
        (TestCardFixture::PrescientResolved, true),
        (TestCardFixture::PrescientResolved, false),
        (TestCardFixture::PrescientIgnoredOnly, false),
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(case));
        let initial = session.observe().play.unwrap();
        let prescient = initial.players[2]
            .hand
            .iter()
            .find(|card| card.code == "60419")
            .unwrap()
            .id;
        let voice = initial.players[2]
            .discard
            .iter()
            .find(|card| card.code == "60416")
            .unwrap()
            .id;
        choose(&mut session, "window:pass");
        choose(&mut session, &format!("commit:{prescient}"));
        choose(&mut session, "commit:confirm");
        assert!(
            !session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id == "test:reveal")
        );
        choose(&mut session, "test-card:prescient:symbol");
        choose(&mut session, "window:pass");
        if case == TestCardFixture::PrescientIgnoredOnly {
            let olive = session
                .observe()
                .choices
                .iter()
                .find(|choice| choice.id.starts_with("tokens:olive:"))
                .unwrap()
                .id
                .clone();
            choose(&mut session, &olive);
        }
        choose(&mut session, "test:reveal");
        if case == TestCardFixture::PrescientIgnoredOnly {
            choose(&mut session, "tokens:keep:1");
            choose(&mut session, "tokens:keep:2");
            choose(&mut session, "tokens:confirm");
            assert!(
                !session
                    .observe()
                    .choices
                    .iter()
                    .any(|choice| choice.id.starts_with("prescient:return:")),
                "Qualified legacy interpretation: an ignored-only symbol does not qualify"
            );
            continue;
        }
        let after = session.observe();
        let choices: Vec<_> = after
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("prescient:return:"))
            .collect();
        assert_eq!(
            choices.len(),
            2,
            "Only Clairvoyance and Voice of Ra are Spell targets, not Arbiter or Premonition"
        );
        assert_eq!(after.play.as_ref().unwrap().bag_count, 16);
        assert!(
            after.play.as_ref().unwrap().players[2]
                .discard
                .iter()
                .any(|card| card.id == prescient),
            "Prescient is already discarded at its after-test window"
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("prescient-after-test.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        assert_eq!(reopened.observe(), after);
        let id = if recover {
            format!("prescient:return:{prescient}:{voice}")
        } else {
            "after-test:pass".into()
        };
        choose(&mut session, &id);
        choose(&mut reopened, &id);
        assert_eq!(reopened.observe(), session.observe());
        assert_eq!(
            session.observe().play.unwrap().players[2]
                .hand
                .iter()
                .any(|card| card.id == voice),
            recover
        );
        session.validate().unwrap();
    }
}
