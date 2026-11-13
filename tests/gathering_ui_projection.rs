//! Public presentation facts are supplied by the same Session that offers choices.
use arkham_unfolded_experiment::gathering::{HostileFixture, SelectedWitness, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    assert!(view.choices.iter().any(|choice| choice.id == id), "{id}");
    session.choose(view.revision, id).unwrap();
}

fn prepared() -> Session {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    session
}

#[test]
fn ordinary_action_counters_distinguish_spending_from_ending_early() {
    let mut session = prepared();
    choose(&mut session, "turn:patrice");
    choose(&mut session, "action:ordinary:resource");
    let view = serde_json::to_value(session.observe()).unwrap();
    let counters = &view["play"]["players"][1]["presentation"];
    assert_eq!(counters["health"], 7);
    assert_eq!(counters["sanity"], 7);
    assert_eq!(counters["ordinary_action_allowance"], 3);
    assert_eq!(counters["ordinary_actions_spent"], 1);
    assert_eq!(counters["turn_finished"], false);
    assert_eq!(view["play"]["actions_remaining"][1], 2);
    choose(&mut session, "turn:end");
    let view = serde_json::to_value(session.observe()).unwrap();
    let counters = &view["play"]["players"][1]["presentation"];
    assert_eq!(counters["turn_finished"], true);
    assert_eq!(counters["ordinary_actions_spent"], 1);
    assert_eq!(view["play"]["actions_remaining"][1], 2);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("session.json");
    continuity::save_session(&path, &session).unwrap();
    assert_eq!(
        continuity::load_session(path).unwrap().observe(),
        session.observe()
    );
}

#[test]
fn commitment_cues_follow_legal_additions_and_keep_selected_cards_removable() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::SelectedWitness(SelectedWitness::Guts));
    choose(&mut session, "revelation:resolve");
    while session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    let view = session.observe();
    let guts: Vec<_> = view.play.as_ref().unwrap().players[0]
        .hand
        .iter()
        .filter(|card| card.code == "01089")
        .map(|card| card.id)
        .collect();
    assert_eq!(guts.len(), 2);
    let before = serde_json::to_value(&view).unwrap();
    let cues = before["play"]["test"]["commitment"]
        .as_array()
        .expect("Typed commitment cues");
    for id in &guts {
        let cue = cues.iter().find(|cue| cue["card_id"] == *id).unwrap();
        assert_eq!(cue["contribution"], 2);
        assert_eq!(cue["selected"], false);
    }
    choose(&mut session, &format!("commit:{}", guts[0]));
    let selected = serde_json::to_value(session.observe()).unwrap();
    let cues = selected["play"]["test"]["commitment"].as_array().unwrap();
    assert_eq!(
        cues.iter().find(|cue| cue["card_id"] == guts[0]).unwrap()["selected"],
        true
    );
    assert!(!cues.iter().any(|cue| cue["card_id"] == guts[1]));
    choose(&mut session, "commit:confirm");
    let confirmed = serde_json::to_value(session.observe()).unwrap();
    assert_eq!(
        confirmed["play"]["test"]["commitment"],
        serde_json::json!([])
    );
}

#[test]
fn discovery_motion_uses_the_actual_investigation_operation_and_rejects_forgery() {
    let mut session = prepared();
    choose(&mut session, "turn:patrice");
    choose(&mut session, "action:ordinary:investigate");
    let view = session.observe();
    assert_eq!(
        view.play
            .unwrap()
            .test
            .unwrap()
            .investigation_location
            .as_deref(),
        Some("01111")
    );

    let mut state = serde_json::to_value(&session).unwrap();
    state["runtime"]["test"]["operation"]["Investigate"]["location"] = "01112".into();
    let forged: Session = serde_json::from_value(state).unwrap();
    assert!(forged.validate().is_err());
}

/// #210, tests name their action and target: every test's view says what it
/// attempts and on what, or which card calls for it, from the rules session's own
/// operation. The driver's observation does not change.
#[test]
fn every_skill_test_names_its_operation_and_target() {
    use arkham_unfolded_experiment::gathering::{CryptChillFixture, SearchFixture, TestAction::*};
    fn named(session: &Session) -> (TestAction, String, String) {
        let test = session.observe().play.unwrap().test.expect("A skill test");
        let target = test.target.expect("Every test here names its target");
        (
            test.operation.expect("A live view names it"),
            target.code,
            target.title,
        )
    }
    use arkham_unfolded_experiment::gathering::TestAction;
    let frozen =
        || Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::GetOverHereFrozen));
    let swarm = |session: &Session| {
        session
            .observe()
            .play
            .unwrap()
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.code == "01159")
            .unwrap()
            .card
            .id
    };

    let mut investigation = prepared();
    choose(&mut investigation, "turn:patrice");
    choose(&mut investigation, "action:ordinary:investigate");
    assert_eq!(
        named(&investigation),
        (Investigate, "01111".into(), "Study".into())
    );

    let mut fight = frozen();
    let rats = swarm(&fight);
    choose(&mut fight, &format!("action:ordinary:fight:{rats}"));
    assert_eq!(
        named(&fight),
        (Fight, "01159".into(), "Swarm of Rats".into())
    );
    let mut evade = frozen();
    let rats = swarm(&evade);
    choose(&mut evade, &format!("action:ordinary:evade:{rats}"));
    assert_eq!(
        named(&evade),
        (Evade, "01159".into(), "Swarm of Rats".into())
    );

    // Card-caused and treachery tests name the card that calls for them.
    let mut chill =
        Session::with_hostile_fixture(HostileFixture::CryptChill(CryptChillFixture::Assets));
    choose(&mut chill, "revelation:resolve");
    assert_eq!(named(&chill), (Card, "01167".into(), "Crypt Chill".into()));
    let mut parley = Session::with_hostile_fixture(HostileFixture::GatheringLitaParley);
    choose(&mut parley, "action:ordinary:parley:lita");
    assert_eq!(
        named(&parley),
        (Parley, "01117".into(), "Lita Chantler".into())
    );

    let observed = serde_json::to_value(fight.observe()).unwrap();
    let test = observed["play"]["test"].as_object().unwrap();
    assert!(
        !test.contains_key("operation") && !test.contains_key("target"),
        "The driver's observation is unchanged: {test:?}"
    );
}

#[test]
fn setup_capacities_and_tonys_restricted_action_are_authoritative() {
    let setup = Session::with_seed(89).observe();
    assert_eq!(
        setup
            .investigators
            .iter()
            .map(|player| (player.health(), player.sanity()))
            .collect::<Vec<_>>(),
        vec![(9, 5), (7, 7), (6, 9)]
    );
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    let before = session.observe().play.unwrap();
    let counters = &before.players[0].presentation;
    let bonus = counters.tony_bonus.as_ref().unwrap();
    assert!(bonus.available && bonus.eligible && !bonus.used);
    choose(&mut session, "action:tony:fight:400");
    let after = session.observe().play.unwrap();
    let counters = &after.players[0].presentation;
    let bonus = counters.tony_bonus.as_ref().unwrap();
    assert!(!bonus.available && !bonus.eligible && bonus.used);
    assert_eq!(after.actions_remaining[0], 3);
    assert_eq!(counters.ordinary_actions_spent, 0);
}

#[test]
fn unused_curse_pool_has_its_own_physical_count() {
    let view = serde_json::to_value(prepared().observe()).unwrap();
    assert_eq!(view["play"]["curse_pool"], 10);
}

#[test]
fn conditional_icons_use_effective_contribution_instead_of_printed_wild_icons() {
    for completed in [0, 2, 3] {
        let mut session = Session::with_hostile_fixture(HostileFixture::SelectedWitness(
            SelectedWitness::Initiative,
        ));
        let initiative = session.observe().play.unwrap().players[0]
            .hand
            .iter()
            .find(|card| card.code == "04150")
            .unwrap()
            .id;
        for _ in 0..completed {
            choose(&mut session, "action:ordinary:resource");
        }
        // With no action left and nothing else playable, the turn ends by itself.
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "turn:end")
        {
            choose(&mut session, "turn:end");
        }
        choose(&mut session, "turn:tony");
        choose(&mut session, "action:ordinary:investigate");
        while session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "window:pass")
        {
            choose(&mut session, "window:pass");
        }
        let view = session.observe();
        let test = view.play.unwrap().test.unwrap();
        let cue = test.commitment.iter().find(|cue| cue.card_id == initiative);
        assert_eq!(
            cue.map(|cue| cue.contribution),
            if completed == 3 {
                None
            } else {
                Some(3 - completed)
            }
        );
    }
}

#[test]
fn frozen_test_result_is_distinct_from_preview() {
    let mut session = Session::with_hostile_fixture(HostileFixture::SelectedWitness(
        SelectedWitness::GraspingHands,
    ));
    choose(&mut session, "revelation:resolve");
    for _ in 0..8 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "window:pass") {
            choose(&mut session, "window:pass");
        } else if view
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
        {
            choose(&mut session, "commit:confirm");
        } else {
            break;
        }
    }
    let before = session.observe();
    assert!(
        !before
            .play
            .as_ref()
            .unwrap()
            .test
            .as_ref()
            .unwrap()
            .resolved
    );
    choose(&mut session, "test:reveal");
    let result = session
        .presentation_after(before.revision)
        .iter()
        .filter_map(|moment| moment.play.test.as_ref())
        .find(|test| test.resolved)
        .unwrap();
    assert_eq!(
        (
            result.base_skill,
            result.modifiers.cards,
            result.modifiers.other,
            result.modifiers.chaos
        ),
        (2, 0, 0, Some(-1))
    );
    assert_eq!(
        (result.total, result.would_succeed, result.difficulty),
        (Some(1), Some(false), 3)
    );
    assert_eq!(
        session
            .observe()
            .play
            .unwrap()
            .completed_tests
            .last()
            .unwrap()
            .total,
        1
    );
}
