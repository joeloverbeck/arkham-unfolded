use arkham_unfolded_experiment::gathering::{
    HostileFixture, PREDICTION_MARGIN, Session, TestCardFixture, continuity,
};

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
        let mut moments = Vec::new();
        let revealing = session.observe().revision;
        choose(&mut session, "test:reveal");
        moments.extend_from_slice(session.presentation_after(revealing));
        if let Some(result) =
            session.observe().choices.iter().find(|choice| {
                choice.id.starts_with("result:") && choice.label.contains("Resourceful")
            })
        {
            let ordering = session.observe().revision;
            choose(&mut session, &result.id);
            moments.extend_from_slice(session.presentation_after(ordering));
        }
        // Resourceful cannot recover itself or a neutral card, so its one
        // eligible target is returned by itself.
        let recovered = session
            .observe()
            .history
            .iter()
            .filter(|entry| entry.starts_with("Automatic: Return ") && entry.ends_with("'s hand"))
            .count();
        if case == TestCardFixture::ResourcefulSuccess {
            assert_eq!(
                recovered, 1,
                "Resourceful cannot recover itself or a neutral card"
            );
            let cleanup = moments
                .iter()
                .find(|moment| moment.caption.starts_with("The skill test finishes."))
                .map(|moment| &moment.play)
                .unwrap();
            assert!(
                cleanup.players[1]
                    .hand
                    .iter()
                    .any(|entry| entry.code == "04200"),
                "ST7 recovery precedes ST8 committed-card discard"
            );
            assert!(
                session.observe().play.unwrap().players[1]
                    .hand
                    .iter()
                    .any(|entry| entry.code == "04200")
            );
        } else {
            assert_eq!(recovered, 0);
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
fn observations_list_one_direct_prediction_choice_per_ready_pendulum_and_no_stepper() {
    let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalPrediction,
    ));
    let view = session.observe();
    let crystal = view.play.as_ref().unwrap().players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60411")
        .unwrap()
        .card
        .id;
    let listed: Vec<_> = view
        .choices
        .iter()
        .filter(|choice| choice.id.starts_with("test-card:predict:"))
        .collect();
    assert_eq!(listed.len(), 1, "{:?}", view.choices);
    let template = &listed[0].id;
    assert_eq!(
        template,
        &format!("test-card:predict:{crystal}:{PREDICTION_MARGIN}")
    );
    assert!(
        listed[0].label.contains("Crystal Pendulum"),
        "{}",
        listed[0].label
    );
    assert!(
        listed[0].label.contains(PREDICTION_MARGIN),
        "{}",
        listed[0].label
    );
    assert!(
        !view.choices.iter().any(|choice| {
            choice.id.starts_with("test-card:pendulum:")
                || choice.id.starts_with("test-card:number:")
        }),
        "The former stepper is gone: {:?}",
        view.choices
    );
    let before = session.clone();
    for refused in [
        template.clone(),
        format!("test-card:pendulum:{crystal}"),
        format!("test-card:predict:{crystal}:65536"),
        format!("test-card:predict:{crystal}:07"),
    ] {
        assert!(
            session.choose(view.revision, &refused).is_err(),
            "{refused}"
        );
        assert_eq!(session, before, "{refused}");
    }
    session
        .choose(view.revision, &template.replace(PREDICTION_MARGIN, "7"))
        .unwrap();
    let after = session.observe();
    assert!(
        after.history.last().unwrap().contains("predict 7"),
        "{:?}",
        after.history.last()
    );
    assert!(
        !after
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("test-card:predict:")),
        "An exhausted Pendulum offers no further prediction"
    );
    assert!(session.pendulum_prompts().is_empty());
}

#[test]
fn direct_pendulum_prediction_is_atomic_unbounded_and_undoable() {
    for number in [0u16, 3, 12, u16::MAX] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(
            TestCardFixture::CrystalPrediction,
        ));
        let before = session.clone();
        let crystal = session.observe().play.unwrap().players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "60411")
            .unwrap()
            .card
            .id;
        let revision = session.observe().revision;
        let id = format!("test-card:predict:{crystal}:{number}");
        session.choose(revision, &id).unwrap();
        let named = session.clone();
        assert_eq!(session.observe().revision, revision + 1);
        assert!(
            !session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id == "test-card:pass")
        );
        assert!(
            !session.observe().play.unwrap().players[2]
                .assets
                .iter()
                .find(|asset| asset.card.id == crystal)
                .unwrap()
                .ready
        );
        assert!(
            session.choose(revision, &id).is_err(),
            "Stale double activation"
        );
        assert!(
            session.choose(session.observe().revision, &id).is_err(),
            "Already exhausted"
        );
        assert_eq!(session, named);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("direct.json");
        continuity::save_session(&path, &session).unwrap();
        assert_eq!(continuity::load_session(&path).unwrap(), session);
        choose(&mut session, "undo");
        assert_eq!(session.observe().play, before.observe().play);
        session.validate().unwrap();
    }
}

#[test]
fn direct_prediction_preserves_another_reaction_and_the_following_commitment_window() {
    let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalWithGregory,
    ));
    let prompt = session.pendulum_prompts().pop().unwrap();
    session
        .choose(
            session.observe().revision,
            &format!("test-card:predict:{}:3", prompt.asset),
        )
        .unwrap();
    let view = session.observe();
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id.starts_with("tony:gregory:"))
    );
    // A window with a real opportunity is never resolved away: Gregory's
    // reaction stays beside the explicit skip.
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id == "test-card:pass")
    );
    let with_reaction = session.clone();
    assert!(
        session.choose(view.revision, "test-card:continue").is_err(),
        "No hidden continuation identity stands in for the explicit skip"
    );
    assert_eq!(session, with_reaction);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reaction.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(path).unwrap();
    let wager = session
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.id.starts_with("tony:gregory:"))
        .unwrap()
        .id;
    let reacting = session.observe();
    choose(&mut session, &wager);
    // The wager leaves no other reaction, so the emptied window closes by
    // itself as part of the wager, and Undo returns to the genuine decision.
    let after_wager = session.observe();
    assert!(
        !after_wager
            .choices
            .iter()
            .any(|choice| choice.id == "test-card:pass")
    );
    assert!(
        after_wager
            .history
            .contains(&"Automatic: Continue after test-begin reactions".to_owned())
    );
    let mut undone = session.clone();
    choose(&mut undone, "undo");
    assert_eq!(undone.observe().play, reacting.play);
    assert_eq!(undone.observe().choices, reacting.choices);
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
    );
    session.validate().unwrap();
}

#[test]
fn pendulum_matches_source_derived_twelve_and_special_failure_margins_through_saved_choices() {
    for (case, number, total, difficulty, success, failed_by) in [
        (TestCardFixture::CrystalReadSigns, 12, 14, 2, true, 0),
        (TestCardFixture::CrystalFloor, 3, 0, 3, false, 3),
        (TestCardFixture::CrystalAutoFail, 3, 0, 3, false, 3),
        (TestCardFixture::CrystalAutoFailZero, 0, 0, 0, false, 0),
        (TestCardFixture::CrystalVisionTie, 0, 2, 2, false, 0),
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::TestCards(case));
        let before = session.observe().play.unwrap();
        let crystal = before.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "60411")
            .unwrap()
            .card
            .id;
        if case == TestCardFixture::CrystalReadSigns {
            let signs = before.players[2]
                .hand
                .iter()
                .find(|card| card.code == "06117")
                .unwrap()
                .id;
            choose(
                &mut session,
                &format!("action:ordinary:play:tony:{signs}:06117"),
            );
        }
        session
            .choose(
                session.observe().revision,
                &format!("test-card:predict:{crystal}:{number}"),
            )
            .unwrap();
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "window:pass")
        {
            choose(&mut session, "window:pass");
        }
        if case == TestCardFixture::CrystalReadSigns {
            let promise = before.players[1]
                .hand
                .iter()
                .find(|card| card.code == "07032")
                .unwrap()
                .id;
            choose(&mut session, &format!("commit:{promise}"));
        }
        reach_reveal(&mut session);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("prediction-witness.json");
        continuity::save_session(&path, &session).unwrap();
        session = continuity::load_session(&path).unwrap();
        choose(&mut session, "test:reveal");
        for _ in 0..20 {
            let view = session.observe();
            if view.play.as_ref().unwrap().completed_tests.len() > before.completed_tests.len() {
                break;
            }
            let next = view
                .choices
                .iter()
                .find(|choice| choice.id == "lucky:decline")
                .or_else(|| {
                    view.choices
                        .iter()
                        .find(|choice| choice.id.starts_with("result:"))
                })
                .unwrap_or_else(|| {
                    panic!(
                        "{case:?}: unexpected remaining result choice {:?}",
                        view.choices
                    )
                });
            choose(&mut session, &next.id);
        }
        let play = session.observe().play.unwrap();
        let result = play.completed_tests.last().expect("Actual test completes");
        assert_eq!(
            (
                result.total,
                result.difficulty,
                result.success,
                result.failure_margin
            ),
            (total, difficulty, success, failed_by),
            "{case:?}"
        );
        assert_eq!(
            play.players[2].deck_count,
            before.players[2].deck_count - 1,
            "Exactly the matching prediction draws: {case:?}"
        );
        session.validate().unwrap();
    }
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
        let listed = session
            .observe()
            .choices
            .into_iter()
            .find(|choice| choice.id.starts_with("test-card:predict:"))
            .unwrap()
            .id;
        session
            .choose(
                session.observe().revision,
                &listed.replace(PREDICTION_MARGIN, &number.to_string()),
            )
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("prediction.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        assert_eq!(reopened.observe(), session.observe());
        for game in [&mut session, &mut reopened] {
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
            reach_reveal(game);
            assert!(
                !game
                    .observe()
                    .choices
                    .iter()
                    .any(|choice| choice.id.starts_with("test-card:predict:"))
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
