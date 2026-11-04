use arkham_unfolded_experiment::gathering::{Investigator, Session, Stage};

fn choose(session: &mut Session, id: &str) {
    session.choose(session.observe().revision, id).unwrap();
}

#[test]
fn real_opening_hands_follow_lead_selection_and_preserve_fresh_campaign_inputs() {
    let mut session = Session::with_seed(89);
    let initial = session.observe();
    assert_eq!(initial.stage, Stage::ChooseLead);
    assert!(initial.investigators.iter().all(|i| i.hand.is_empty()));
    choose(&mut session, "lead:jacqueline");
    let view = session.observe();
    assert_eq!(view.stage, Stage::Mulligan);
    assert_eq!(view.lead, Some(Investigator::Jacqueline));
    assert_eq!(view.decision_owner, Some(Investigator::Jacqueline));
    for investigator in &view.investigators {
        assert_eq!(investigator.resources, 5);
        assert_eq!(investigator.hand.len(), 5);
        assert!(investigator.hand.iter().all(|card| !card.weakness));
        assert!(investigator.hand.iter().all(|card| !card.title.is_empty()));
    }
    assert_eq!(view.investigators[0].bounties, 6);
    assert_eq!(view.investigators[0].permanents[0].code, "06010");
    assert_eq!(view.investigators[2].mental_trauma, 2);
    assert_eq!(view.investigators[2].horror, 2);
    assert_eq!(view.investigators[2].permanents.len(), 2);
    assert!(view.scenario.is_none());
    session.validate().unwrap();
}

#[test]
fn starting_resources_are_collected_after_choosing_lead() {
    // RR p27: choose lead (step 3), then collect resources (step 7).
    let mut session = Session::with_seed(15);
    assert!(
        session
            .observe()
            .investigators
            .iter()
            .all(|i| i.resources == 0)
    );
    choose(&mut session, "lead:tony");
    assert!(
        session
            .observe()
            .investigators
            .iter()
            .all(|i| i.resources == 5)
    );
}

#[test]
fn mulligan_keeps_removed_physical_cards_out_of_replacements_and_follows_player_order() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:jacqueline");
    let opening = session.observe();
    let replace = opening.investigators[2].hand[0].id;
    let foreign = opening.investigators[0].hand[0].id;
    let unchanged = session.clone();
    assert!(
        session
            .choose(opening.revision, &format!("mulligan:{foreign}"))
            .is_err()
    );
    assert_eq!(session, unchanged);
    choose(&mut session, &format!("mulligan:{replace}"));
    let selected = session.observe();
    assert_eq!(
        selected.investigators[2].hand,
        opening.investigators[2].hand
    );
    assert_eq!(selected.investigators[2].selected_mulligan, vec![replace]);
    let unchanged = session.clone();
    assert!(
        session
            .choose(opening.revision, "confirm-mulligan")
            .is_err()
    );
    assert_eq!(session, unchanged);
    choose(&mut session, "confirm-mulligan");
    let replaced = session.observe();
    assert_eq!(replaced.investigators[2].hand.len(), 5);
    assert!(
        replaced.investigators[2]
            .hand
            .iter()
            .all(|card| card.id != replace && !card.weakness)
    );
    for kept in &opening.investigators[2].hand[1..] {
        assert!(replaced.investigators[2].hand.contains(kept));
    }
    assert_eq!(replaced.decision_owner, Some(Investigator::Tony));
    assert!(replaced.investigators[2].mulligan_complete);
    assert!(!replaced.choices.iter().any(|choice| choice.id == "undo"));
    choose(&mut session, "confirm-mulligan");
    assert_eq!(
        session.observe().decision_owner,
        Some(Investigator::Patrice)
    );
    choose(&mut session, "confirm-mulligan");
    assert_eq!(session.observe().stage, Stage::Playing);
    session.validate().unwrap();
}

#[test]
fn weaknesses_stay_out_through_all_mulligans_and_final_deck_counts_include_them() {
    // Inspect finite deterministic witnesses through the player seam. Lower deck
    // counts show weaknesses set aside without disclosing their hidden identities.
    let mut witnessed_set_aside = false;
    for seed in [0, 1, 7, 15, 89] {
        let mut session = Session::with_seed(seed);
        choose(&mut session, "lead:tony");
        let before = session.observe();
        witnessed_set_aside |= before
            .investigators
            .iter()
            .zip([29, 40, 28])
            .any(|(i, final_count)| i.deck_count < final_count);
        for owner in 0..3 {
            let view = session.observe();
            for card in &view.investigators[owner].hand {
                choose(&mut session, &format!("mulligan:{}", card.id));
            }
            choose(&mut session, "confirm-mulligan");
            assert!(
                session
                    .observe()
                    .investigators
                    .iter()
                    .all(|i| i.hand.iter().all(|card| !card.weakness))
            );
            if owner < 2 {
                assert!(session.observe().investigators[owner].deck_count <= [29, 40, 28][owner]);
            }
        }
        let ready = session.observe();
        assert_eq!(
            ready
                .investigators
                .iter()
                .map(|i| i.deck_count)
                .collect::<Vec<_>>(),
            [29, 40, 28]
        );
        assert!(
            ready
                .investigators
                .iter()
                .all(|i| i.hand.len() == 5 && i.mulligan_complete)
        );
        session.validate().unwrap();
    }
    assert!(
        witnessed_set_aside,
        "the fixtures must exercise a replaced opening weakness"
    );
}

#[test]
fn an_initial_weakness_is_not_returned_after_its_owners_mulligan() {
    let mut session = (0..64)
        .find_map(|seed| {
            let mut candidate = Session::with_seed(seed);
            choose(&mut candidate, "lead:tony");
            (candidate.observe().investigators[0].deck_count < 29).then_some(candidate)
        })
        .expect("A finite opening-weakness witness");
    let count_with_weakness_out = session.observe().investigators[0].deck_count;
    choose(&mut session, "confirm-mulligan");
    assert_eq!(
        session.observe().investigators[0].deck_count,
        count_with_weakness_out
    );
    choose(&mut session, "confirm-mulligan");
    assert_eq!(
        session.observe().investigators[0].deck_count,
        count_with_weakness_out
    );
    choose(&mut session, "confirm-mulligan");
    assert_eq!(session.observe().investigators[0].deck_count, 29);
}

#[test]
fn scenario_assembly_matches_the_independent_publisher_inventory_in_each_destination() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    let view = session.observe();
    let scenario = view.scenario.as_ref().unwrap();
    // #89 independently derives this literal from publisher LTP p8 and the
    // printed sets, rather than from the constructor's membership table.
    let actual: Vec<_> = scenario
        .encounter_inventory
        .iter()
        .map(|entry| (entry.code.as_str(), entry.quantity))
        .collect();
    assert_eq!(
        actual,
        [
            ("01118", 1),
            ("01119", 1),
            ("01159", 3),
            ("01160", 3),
            ("01161", 1),
            ("01162", 3),
            ("01163", 3),
            ("01164", 2),
            ("01165", 2),
            ("01166", 3),
            ("01167", 2),
            ("01168", 2),
        ]
    );
    assert_eq!(scenario.encounter_deck_count, 26);
    assert_eq!(scenario.reference, "01104");
    assert_eq!(scenario.agenda, ["01105", "01106", "01107"]);
    assert_eq!(scenario.act, ["01108", "01109", "01110"]);
    assert_eq!(scenario.locations, ["01111"]);
    assert_eq!(
        scenario.set_aside,
        ["01112", "01113", "01114", "01115", "01116", "01117"]
    );
    assert_eq!(scenario.study_clues, 6);
    assert!(scenario.skip_first_mythos);
    assert!(
        view.investigators
            .iter()
            .all(|i| i.location.as_deref() == Some("01111"))
    );
    assert_eq!(view.stage, Stage::Playing);
    assert!(view.choices.iter().any(|choice| choice.id == "turn:tony"));
    assert!(!session.is_verification_fixture());
    let unchanged = session.clone();
    assert!(session.choose(view.revision, "investigate").is_err());
    assert_eq!(session, unchanged);
}

#[test]
fn source_bindings_and_observations_reveal_no_seed_or_hidden_draw_order() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:patrice");
    let view = session.observe();
    assert_eq!(view.source.taboo_id, 10);
    assert_eq!(
        view.source
            .decks
            .iter()
            .map(|d| (d.deck_id, d.revision.as_str(), d.basic_weakness.as_str()))
            .collect::<Vec<_>>(),
        [
            (5392133, "0.5", "02038"),
            (5391062, "0.7", "05041"),
            (5389418, "0.9", "02039"),
        ]
    );
    assert!(
        view.source
            .decks
            .iter()
            .all(|d| !d.inventory.iter().any(|card| card.code == "01000"))
    );
    assert!(view.source.decks.iter().all(|d| {
        d.inventory
            .iter()
            .find(|card| card.code == d.basic_weakness)
            .unwrap()
            .quantity
            == 1
    }));
    let serialized = serde_json::to_value(view).unwrap();
    assert!(serialized.get("seed").is_none());
    assert!(serialized.get("random").is_none());
    assert!(serialized.get("encounter_deck").is_none());
    assert!(serialized.get("decks").is_none());
    assert!(serialized.get("excluded_weaknesses").is_none());
    let mut different = Session::with_seed(90);
    choose(&mut different, "lead:patrice");
    assert_ne!(
        session.observe().investigators[1].hand,
        different.observe().investigators[1].hand
    );
}

#[test]
fn save_validation_refuses_states_that_differ_from_recorded_setup_decisions() {
    use arkham_unfolded_experiment::gathering::continuity::save_session;
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    let mut wrong_resources = serde_json::to_value(&session).unwrap();
    wrong_resources["investigators"][0]["resources"] = 255.into();
    let mut wrong_scenario = serde_json::to_value(&session).unwrap();
    wrong_scenario["scenario"]["study_clues"] = 99.into();
    let directory = tempfile::tempdir().unwrap();
    for state in [wrong_resources, wrong_scenario] {
        let forged: Session = serde_json::from_value(state).unwrap();
        assert!(forged.validate().is_err());
        let path = directory.path().join("invalid.recording.json");
        assert!(save_session(&path, &forged).is_err());
        assert!(!path.exists());
    }
    session.validate().unwrap();
}
