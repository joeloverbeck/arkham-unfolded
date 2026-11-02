use arkham_unfolded_experiment::gathering::{HostileFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe_for_play().revision, id)
        .unwrap();
}

fn completed_setup() -> Session {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    session
}

#[test]
fn dissonant_voices_blocks_asset_play_without_spending_its_costs() {
    let mut session = Session::with_hostile_fixture(HostileFixture::DissonantVoicesAssetPlay);
    let before = session.observe();
    let jacqueline = &before.play.as_ref().unwrap().players[2];
    assert!(jacqueline.threats.iter().any(|card| card.code == "01165"));
    let robes = jacqueline
        .hand
        .iter()
        .find(|card| card.code == "60412")
        .unwrap();
    let id = format!("action:ordinary:play:{}", robes.id);
    assert!(
        !before.choices.iter().any(|choice| choice.id == id),
        "Dissonant Voices prohibits playing assets, including affordable Robes"
    );
    let unchanged = session.clone();
    assert!(session.choose(before.revision, &id).is_err());
    assert_eq!(
        session, unchanged,
        "A prohibited play spends neither resources nor actions"
    );
    choose(&mut session, "action:ordinary:resource");
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[2].resources, jacqueline.resources + 1);
    assert!(after.players[2].hand.iter().any(|card| card.id == robes.id));
}

#[test]
fn playing_an_asset_from_the_actual_opening_pays_once_and_preserves_custody() {
    let mut session = completed_setup();
    choose(&mut session, "turn:tony");
    let opening = session.observe_for_play().play.unwrap();
    let hard_knocks = opening.players[0]
        .hand
        .iter()
        .find(|card| card.code == "01049")
        .expect("Seed 89 opening includes Hard Knocks")
        .clone();
    let id = format!("action:ordinary:play:{}", hard_knocks.id);
    choose(&mut session, &id);
    let after = session.observe_for_play();
    let tony = &after.play.as_ref().unwrap().players[0];
    assert_eq!(
        tony.resources, 3,
        "Hard Knocks printed resource cost is 2; setup supplies 5"
    );
    assert!(!tony.hand.iter().any(|card| card.id == hard_knocks.id));
    let played = tony
        .assets
        .iter()
        .find(|asset| asset.card.id == hard_knocks.id)
        .unwrap();
    assert_eq!(played.card, hard_knocks);
    assert!(played.ready);
    assert_eq!(played.uses, 0);
    assert!(!tony.discard.iter().any(|card| card.id == hard_knocks.id));
    let original = session.clone();
    assert!(
        session.choose(after.revision, &id).is_err(),
        "The same physical hand card cannot be played again"
    );
    assert_eq!(session, original);
}

#[test]
fn a_full_ally_slot_requires_the_players_explicit_replacement_choice() {
    let mut session = completed_setup();
    choose(&mut session, "turn:jacqueline");
    let opening = session.observe_for_play().play.unwrap();
    let hand = &opening.players[2].hand;
    let olive = hand
        .iter()
        .find(|card| card.code == "04197")
        .unwrap()
        .clone();
    let katarina = hand
        .iter()
        .find(|card| card.code == "11069")
        .unwrap()
        .clone();
    choose(&mut session, &format!("action:ordinary:play:{}", olive.id));
    let after_olive = session.observe_for_play();
    let unqualified = format!("action:ordinary:play:{}", katarina.id);
    assert!(
        !after_olive
            .choices
            .iter()
            .any(|choice| choice.id == unqualified)
    );
    let original = session.clone();
    assert!(session.choose(after_olive.revision, &unqualified).is_err());
    assert_eq!(
        session, original,
        "Missing a required replacement must not spend resources/actions"
    );

    choose(
        &mut session,
        &format!("action:ordinary:play:{}:replace:{}", katarina.id, olive.id),
    );
    let after = session.observe_for_play().play.unwrap();
    let jacqueline = &after.players[2];
    assert_eq!(
        jacqueline.resources, 1,
        "Both allies have printed cost 2; setup supplies 5"
    );
    assert!(jacqueline.assets.iter().any(|asset| asset.card == katarina));
    assert!(
        !jacqueline
            .assets
            .iter()
            .any(|asset| asset.card.id == olive.id)
    );
    assert!(jacqueline.discard.iter().any(|card| card == &olive));
    assert!(
        !jacqueline
            .hand
            .iter()
            .any(|card| card.id == katarina.id || card.id == olive.id)
    );
}

#[test]
fn two_arcane_assets_fit_and_receive_their_printed_charges() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    choose(&mut session, "lead:patrice");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    choose(&mut session, "turn:patrice");
    let opening = session.observe_for_play().play.unwrap();
    let hand = &opening.players[1].hand;
    let mists = hand
        .iter()
        .find(|card| card.code == "04029")
        .unwrap()
        .clone();
    let flame = hand
        .iter()
        .find(|card| card.code == "60407")
        .unwrap()
        .clone();
    choose(&mut session, &format!("action:ordinary:play:{}", mists.id));
    choose(&mut session, &format!("action:ordinary:play:{}", flame.id));
    let after = session.observe_for_play().play.unwrap();
    let patrice = &after.players[1];
    assert_eq!(
        patrice.resources, 0,
        "Mists costs 2 and Azure Flame costs 3"
    );
    for card in [mists, flame] {
        let asset = patrice
            .assets
            .iter()
            .find(|asset| asset.card == card)
            .unwrap();
        assert_eq!(
            asset.uses, 4,
            "Both original spell identities enter with four charges"
        );
        assert!(asset.ready);
        assert_eq!((asset.damage, asset.horror), (0, 0));
    }
    assert!(
        patrice.discard.is_empty(),
        "Two arcane slots fit both spells without replacement"
    );
}

#[test]
fn asset_costs_and_limbo_precede_an_attack_of_opportunity() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    let before = session.observe_for_play().play.unwrap();
    let mauser = before.players[0]
        .hand
        .iter()
        .find(|card| card.code == "60306")
        .unwrap()
        .clone();
    choose(&mut session, &format!("action:ordinary:play:{}", mauser.id));
    let pending = session.observe_for_play();
    let tony = &pending.play.as_ref().unwrap().players[0];
    assert_eq!(
        tony.resources, 1,
        "The original Mauser costs 4, paid before the attack"
    );
    assert!(!tony.hand.iter().any(|card| card.id == mauser.id));
    assert!(
        !tony.assets.iter().any(|asset| asset.card.id == mauser.id),
        "The paid asset has not entered play during the attack"
    );
    let attack = pending
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("paid-asset.session.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(
        session.observe_for_play(),
        pending,
        "Reopening retains the paid card in limbo and the pending attack"
    );
    choose(&mut session, &attack);
    let after = session.observe_for_play().play.unwrap();
    let tony = &after.players[0];
    assert_eq!(
        (tony.damage, tony.horror, tony.resources),
        (
            before.players[0].damage + 2,
            before.players[0].horror + 2,
            1
        ),
        "The original Ghoul Priest deals 2 damage and 2 horror"
    );
    assert_eq!(
        tony.assets
            .iter()
            .find(|asset| asset.card == mauser)
            .unwrap()
            .uses,
        5
    );
    assert_eq!(after.actions_remaining[0], 2);
}
