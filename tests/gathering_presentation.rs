use arkham_unfolded_experiment::gathering::{
    HarmFixture, HostileFixture, Investigator, Session, continuity,
};

fn choose(session: &mut Session, id: &str) {
    session.choose(session.observe().revision, id).unwrap();
}

#[test]
fn assigned_harm_stays_visible_before_application_and_after_reload() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyAttack));
    let before = session.observe().play.unwrap();
    let olive = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .clone();
    let harm = before.pending.harm.unwrap();
    assert_eq!(harm.investigator, Investigator::Jacqueline);
    assert_eq!((harm.damage, harm.horror), (1, 1));
    assert!(harm.allocations.is_empty());
    choose(&mut session, &format!("harm:damage:{}", olive.id));
    choose(&mut session, "harm:horror:investigator");
    let assigned = session.observe().play.unwrap();
    assert_eq!(
        assigned.players[2].horror, before.players[2].horror,
        "Assigned harm is not applied harm"
    );
    let harm = assigned.pending.harm.as_ref().unwrap();
    assert!(
        harm.allocations
            .iter()
            .any(|allocation| allocation.asset.as_ref() == Some(&olive) && allocation.damage == 1)
    );
    assert!(
        harm.allocations
            .iter()
            .any(|allocation| allocation.asset.is_none() && allocation.horror == 1)
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("harm.json");
    continuity::save_session(&path, &session).unwrap();
    assert_eq!(
        continuity::load_session(path)
            .unwrap()
            .observe()
            .play
            .unwrap()
            .pending,
        assigned.pending
    );
}

#[test]
fn paid_ward_remains_inspectable_during_its_horror_cost() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::WardCancellation));
    let before = session.observe().play.unwrap();
    let ward = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "01065")
        .unwrap()
        .clone();
    let robes = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .id;
    choose(&mut session, &format!("ward:{}:robes:{robes}", ward.id));
    let after = session.observe().play.unwrap();
    assert!(!after.players[2].hand.contains(&ward));
    assert!(!after.players[2].discard.contains(&ward));
    assert_eq!(
        after
            .pending
            .resolving_cards
            .iter()
            .filter(|card| card.id == ward.id)
            .count(),
        1
    );
    assert!(after.pending.harm.is_some());
}
