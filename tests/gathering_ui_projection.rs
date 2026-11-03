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

fn omit_new_projection(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            if object.contains_key("actions_remaining") {
                object.remove("curse_pool");
            }
            if object.contains_key("assets") && object.contains_key("deck_count") {
                object.remove("presentation");
            }
            if object.contains_key("base_skill") && object.contains_key("committed") {
                object.remove("commitment");
                object.remove("resolved");
                object.remove("investigation_location");
            }
            for value in object.values_mut() {
                omit_new_projection(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                omit_new_projection(value);
            }
        }
        _ => {}
    }
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
    let mut record = serde_json::to_value(session.recording()).unwrap();
    record["moments"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["play"]["test"]["investigation_location"] = "01112".into();
    assert!(continuity::Replay::new(serde_json::from_value(record).unwrap()).is_err());
    let mut old = serde_json::to_value(session).unwrap();
    omit_new_projection(&mut old);
    let old: Session = serde_json::from_value(old).unwrap();
    old.validate().unwrap();
    continuity::Replay::new(old.recording()).unwrap();
}

#[test]
fn old_recorded_views_remain_unknown_while_live_resume_projects_authoritative_state() {
    let current = prepared();
    let mut old_json = serde_json::to_value(&current).unwrap();
    omit_new_projection(&mut old_json);
    let mut old: Session = serde_json::from_value(old_json.clone()).unwrap();
    old.validate().unwrap();
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        old_json,
        "Validation does not rewrite old records"
    );
    let mut replay = continuity::Replay::new(old.recording()).unwrap();
    while replay.next() {}
    assert!(
        serde_json::to_value(replay.current()).unwrap()["play"]["players"][0]
            .get("presentation")
            .is_none()
    );
    assert_eq!(
        old.observe().play.unwrap().players[0]
            .presentation
            .as_ref()
            .unwrap()
            .health,
        9
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("old-session.json");
    continuity::save_session(&path, &old).unwrap();
    let revision = old.observe().revision;
    continuity::commit_choice(&path, &mut old, revision, "turn:patrice").unwrap();
    assert_eq!(continuity::load_session(&path).unwrap(), old);
    continuity::Replay::new(old.recording()).unwrap();

    // Optional independent old-producer evidence: pass a genuine pre-change
    // session to this focused check. It is only read; continuation uses a copy.
    if let Some(prior) = std::env::var_os("ARKHAM_GATHERING_PRIOR_SESSION") {
        let bytes = std::fs::read(&prior).unwrap();
        let mut legacy = continuity::load_session(&prior).unwrap();
        continuity::Replay::new(legacy.recording()).unwrap();
        let copy = directory.path().join("prior-native-copy.json");
        continuity::save_session(&copy, &legacy).unwrap();
        let view = legacy.observe();
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id == "action:ordinary:resource")
            .unwrap();
        continuity::commit_choice(&copy, &mut legacy, view.revision, &choice.id).unwrap();
        assert_eq!(continuity::load_session(copy).unwrap(), legacy);
        assert_eq!(std::fs::read(prior).unwrap(), bytes);
    }
}

#[test]
fn recorded_counter_tampering_is_rejected() {
    let mut value = serde_json::to_value(prepared().recording()).unwrap();
    value["moments"].as_array_mut().unwrap().last_mut().unwrap()["play"]["players"][0]["presentation"]
        ["health"] = 99.into();
    let recording = serde_json::from_value(value).unwrap();
    assert!(continuity::Replay::new(recording).is_err());
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
    let counters = before.players[0].presentation.as_ref().unwrap();
    let bonus = counters.tony_bonus.as_ref().unwrap();
    assert!(bonus.available && bonus.eligible && !bonus.used);
    choose(&mut session, "action:tony:fight:400");
    let after = session.observe().play.unwrap();
    let counters = after.players[0].presentation.as_ref().unwrap();
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
        choose(&mut session, "turn:end");
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
        let cue = test
            .commitment
            .as_ref()
            .unwrap()
            .iter()
            .find(|cue| cue.card_id == initiative);
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
fn frozen_test_result_is_distinct_from_preview_and_old_test_tapes_still_replay() {
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
    assert_eq!(
        before
            .play
            .as_ref()
            .unwrap()
            .test
            .as_ref()
            .unwrap()
            .resolved,
        Some(false)
    );
    choose(&mut session, "test:reveal");
    let result = session
        .presentation_after(before.revision)
        .iter()
        .filter_map(|moment| moment.play.test.as_ref())
        .find(|test| test.resolved == Some(true))
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
    let mut old = serde_json::to_value(&session).unwrap();
    omit_new_projection(&mut old);
    let old: Session = serde_json::from_value(old).unwrap();
    old.validate().unwrap();
    let mut replay = continuity::Replay::new(old.recording()).unwrap();
    loop {
        if let Some(test) = replay
            .current()
            .play
            .as_ref()
            .and_then(|play| play.test.as_ref())
        {
            assert!(test.commitment.is_none() && test.resolved.is_none());
        }
        if !replay.next() {
            break;
        }
    }
}
