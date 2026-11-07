use arkham_unfolded_experiment::gathering::{EtherealFixture, HostileFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe().revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}; {:?}", session.observe().choices));
}

fn finish_test(session: &mut Session) {
    for _ in 0..20 {
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
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .unwrap_or_else(|| panic!("Unexpected test choice: {:?}", view.choices));
        choose(session, id);
    }
    panic!("Test did not finish");
}

#[test]
fn ethereal_form_evades_one_enemy_disengages_others_and_preserves_its_paid_test() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Ethereal(EtherealFixture::Evade));
    let before = session.observe().play.unwrap();
    let event = before.players[1]
        .hand
        .iter()
        .find(|card| card.code == "06164")
        .unwrap()
        .id;
    let target = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap()
        .card
        .id;
    let id = format!("action:ordinary:play:tony:{event}:06164:{target}");
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == id),
        "Actual Ethereal Form must offer its engaged enemy as an evade target"
    );
    choose(&mut session, &id);
    let paid = session.observe();
    let play = paid.play.as_ref().unwrap();
    assert_eq!(play.players[1].resources, 3);
    assert_eq!(play.actions_remaining[1], 2);
    assert!(
        play.test.is_some(),
        "Evade-designated play does not provoke attacks from either engaged enemy"
    );
    assert!(play.players[1].discard.iter().all(|card| card.id != event));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ethereal-test.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), paid);
    finish_test(&mut session);
    let after = session.observe();
    let play = after.play.as_ref().unwrap();
    assert_eq!(
        play.completed_tests.last().unwrap().total,
        6,
        "Patrice's agility2 + currentwillpower4 + zero token"
    );
    assert!(
        play.scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == target)
            .unwrap()
            .exhausted
    );
    let other = play
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id != target)
        .unwrap();
    assert!(
        !other.exhausted,
        "Other enemies disengage; they are not evaded"
    );
    assert!(
        play.scenario
            .enemies
            .iter()
            .all(|enemy| enemy.engaged.is_none())
    );
    assert_eq!(
        play.ethereal,
        vec![arkham_unfolded_experiment::gathering::Investigator::Patrice]
    );
    assert_eq!(
        play.players[1].location, before.players[1].location,
        "Original level0 Form grants no move"
    );
    assert!(
        !after
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("action:ordinary:fight:")
                || choice.id.starts_with("action:ordinary:engage:")
                || choice.id.starts_with("spell-move:"))
    );
    assert!(play.players[1].discard.iter().any(|card| card.id == event));
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(
        session.observe(),
        after,
        "The active round-duration effect survives reopen"
    );
    for forbidden in [
        format!("action:ordinary:fight:{target}"),
        format!("action:ordinary:engage:{target}"),
    ] {
        let unchanged = session.clone();
        assert!(
            session
                .choose(session.observe().revision, &forbidden)
                .is_err()
        );
        assert_eq!(
            session, unchanged,
            "An ethereal action refusal must preserve all costs and custody"
        );
    }
    session.validate().unwrap();
}

fn play_form(session: &mut Session, target_code: &str) {
    let play = session.observe().play.unwrap();
    let event = play.players[1]
        .hand
        .iter()
        .find(|card| card.code == "06164")
        .unwrap()
        .id;
    let target = play
        .scenario
        .enemies
        .iter()
        .map(|enemy| &enemy.card)
        .chain(play.players[1].hand.iter())
        .find(|card| card.code == target_code)
        .unwrap()
        .id;
    choose(
        session,
        &format!("action:ordinary:play:tony:{event}:06164:{target}"),
    );
    finish_test(session);
}

#[test]
fn a_failed_form_pays_the_event_without_disengaging_or_becoming_ethereal() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Ethereal(EtherealFixture::FailedEvade));
    play_form(&mut session, "01160");
    let play = session.observe().play.unwrap();
    assert!(!play.completed_tests[0].success);
    assert!(play.ethereal.is_empty());
    assert!(play.scenario.enemies.iter().all(|enemy| !enemy.exhausted
        && enemy.engaged == Some(arkham_unfolded_experiment::gathering::Investigator::Patrice)));
    assert_eq!(play.players[1].resources, 3);
    assert!(
        play.players[1]
            .discard
            .iter()
            .any(|card| card.code == "06164")
    );
}

#[test]
fn other_disengaged_enemies_can_immediately_engage_another_investigator() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Ethereal(EtherealFixture::OtherInvestigator));
    play_form(&mut session, "01160");
    let play = session.observe().play.unwrap();
    let rat = play
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01159")
        .unwrap();
    assert_eq!(
        rat.engaged,
        Some(arkham_unfolded_experiment::gathering::Investigator::Tony)
    );
    assert!(!rat.exhausted);
    assert_eq!(
        play.ethereal,
        vec![arkham_unfolded_experiment::gathering::Investigator::Patrice]
    );
}

#[test]
fn form_can_evade_the_hand_watcher_and_then_disengage_both_other_enemies() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Ethereal(EtherealFixture::Watcher));
    play_form(&mut session, "06017");
    let play = session.observe().play.unwrap();
    assert!(play.completed_tests[0].success);
    assert!(
        play.players[1]
            .discard
            .iter()
            .any(|card| card.code == "06017")
    );
    assert!(
        play.scenario
            .enemies
            .iter()
            .all(|enemy| enemy.engaged.is_none() && !enemy.exhausted)
    );
    assert_eq!(play.ethereal.len(), 1);
}

#[test]
fn ethereal_does_not_prevent_the_watchers_forced_attack_from_hand() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Ethereal(EtherealFixture::WatcherAttack));
    play_form(&mut session, "01160");
    let before = session.observe().play.unwrap().players[1].damage;
    choose(&mut session, "action:ordinary:draw");
    let play = session.observe().play.unwrap();
    assert_eq!(play.players[1].damage, before + 3);
    assert_eq!(play.ethereal.len(), 1);
    assert!(play.players[1].hand.iter().any(|card| card.code == "06017"));
    session.validate().unwrap();
}

#[test]
fn ethereal_expires_before_at_round_end_abilities_and_ready_enemies_reengage() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Ethereal(EtherealFixture::Expiry));
    let round = session.observe().play.unwrap().round;
    choose(&mut session, "round:end");
    assert_eq!(
        session.observe().play.unwrap().ethereal.len(),
        1,
        "The when-round-ends window is still in the old round"
    );
    choose(&mut session, "round-end:decline-act");
    let play = session.observe().play.unwrap();
    assert_eq!(
        play.round, round,
        "AT-end Voices still must resolve before the next round"
    );
    assert!(play.ethereal.is_empty());
    assert!(
        play.scenario.enemies.iter().all(|enemy| enemy.engaged
            == Some(arkham_unfolded_experiment::gathering::Investigator::Patrice))
    );
    let voices = session
        .observe()
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("round-end:voices:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &voices);
    assert_eq!(session.observe().play.unwrap().round, round + 1);
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "mythos:begin")
    );
    session.validate().unwrap();
}
