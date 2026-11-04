use arkham_unfolded_experiment::gathering::{BoostFixture, HostileFixture, Session, continuity};
fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    session
        .choose(view.revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}
fn reveal(session: &mut Session) {
    for id in ["window:pass", "commit:confirm", "window:pass"] {
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == id)
        {
            choose(session, id);
        }
    }
    choose(session, "test:reveal");
}

#[test]
fn sparrow_mask_can_raise_the_secondary_willpower_of_spectral_razor() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Boosts(BoostFixture::MaskRazor));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[1];
    let mask = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "10111")
        .unwrap()
        .card
        .id;
    let action = before
        .choices
        .iter()
        .find(|choice| {
            choice.id.starts_with("action:ordinary:play:tony:") && choice.id.contains(":06201")
        })
        .unwrap()
        .id
        .clone();
    choose(&mut session, &action);
    choose(&mut session, &format!("boost:{mask}"));
    reveal(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(
        after.completed_tests[0].total, 6,
        "Combat2 + willpower4 + Mask2 - token2"
    );
    assert!(after.completed_tests[0].success);
    assert_eq!(
        after.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == mask)
            .unwrap()
            .uses,
        1
    );
}
#[test]
fn hard_knocks_pays_each_repeatable_boost_in_the_actual_test_window() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Boosts(BoostFixture::HardKnocks));
    let initial = session.observe().play.unwrap();
    let asset = initial.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "01049")
        .unwrap()
        .card
        .id;
    choose(&mut session, "action:ordinary:fight:400");
    let boost = format!("boost:{asset}");
    choose(&mut session, &boost);
    choose(&mut session, &boost);
    assert_eq!(session.observe().play.unwrap().players[0].resources, 3);
    reveal(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(after.completed_tests[0].total, 5);
    let report = &after.completed_tests[0].breakdown;
    assert_eq!(
        report
            .bonuses
            .iter()
            .map(|row| (row.label.as_str(), row.amount))
            .collect::<Vec<_>>(),
        [("Hard Knocks", 1), ("Hard Knocks", 1)]
    );
    assert!(after.completed_tests[0].success);
    assert_eq!(after.scenario.enemies[0].damage, 1);
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == boost),
        "The boost is unavailable outside a test window"
    );
}
#[test]
fn sour_mash_exhausts_and_spends_one_supply_for_a_treachery_test() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Boosts(BoostFixture::SourMash));
    let initial = session.observe().play.unwrap();
    let asset = initial.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "05117")
        .unwrap()
        .card
        .id;
    choose(&mut session, &format!("boost:{asset}"));
    let paid = session.observe();
    let mash = paid.play.as_ref().unwrap().players[0]
        .assets
        .iter()
        .find(|candidate| candidate.card.id == asset)
        .unwrap();
    assert_eq!(mash.uses, 1);
    assert!(!mash.ready);
    assert!(
        !paid
            .choices
            .iter()
            .any(|choice| choice.id == format!("boost:{asset}"))
    );
    reveal(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(after.completed_tests[0].total, 3);
    let report = &after.completed_tests[0].breakdown;
    assert!(
        report
            .bonuses
            .iter()
            .any(|row| row.label == "Tennessee Sour Mash" && row.amount == 2)
    );
    assert!(after.completed_tests[0].success);
    assert_eq!(after.players[0].horror, initial.players[0].horror);
}
#[test]
fn mask_spends_once_per_test_replenishes_once_per_attack_and_resets_on_retry() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Boosts(BoostFixture::MaskRetry));
    let initial = session.observe().play.unwrap();
    let player = &initial.players[1];
    let mask = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "10111")
        .unwrap()
        .card
        .id;
    let flame = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "60407")
        .unwrap()
        .card
        .id;
    let event = player
        .hand
        .iter()
        .find(|card| card.code == "04200")
        .unwrap()
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:fight:400:spell:{flame}"),
    );
    choose(&mut session, &format!("boost:{mask}"));
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == format!("boost:{mask}"))
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("mask.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    reveal(&mut session);
    choose(&mut session, "retaliate:complete");
    let failed = session.observe().play.unwrap();
    assert_eq!(failed.completed_tests[0].total, 2);
    assert_eq!(
        failed.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == mask)
            .unwrap()
            .uses,
        1,
        "One attack deals damage and horror together; replenish only one offering"
    );
    choose(&mut session, &format!("live-learn:{event}"));
    choose(&mut session, &format!("boost:{mask}"));
    reveal(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(
        after.completed_tests[1].total, 7,
        "The old boost expired; this is +2 Live and Learn plus one new +2 Mask boost"
    );
    assert_eq!(
        after.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == mask)
            .unwrap()
            .uses,
        0
    );
    assert_eq!(after.actions_remaining[1], 2);
}
