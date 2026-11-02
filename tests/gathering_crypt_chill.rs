use arkham_unfolded_experiment::gathering::{
    CryptChillFixture, HostileFixture, Session, continuity,
};
fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    session
        .choose(view.revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}
fn reveal(session: &mut Session) {
    for _ in 0..8 {
        let view = session.observe();
        let id = if view.choices.iter().any(|choice| choice.id == "window:pass") {
            "window:pass"
        } else if view
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
        {
            "commit:confirm"
        } else {
            break;
        };
        choose(session, id);
    }
    choose(session, "test:reveal");
}
#[test]
fn crypt_chill_keeps_revelation_pending_until_a_saved_legal_asset_choice_finishes() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::CryptChill(CryptChillFixture::Assets));
    choose(&mut session, "revelation:resolve");
    assert_eq!(session.observe().play.unwrap().test.unwrap().difficulty, 4);
    reveal(&mut session);
    let view = session.observe();
    let player = &view.play.as_ref().unwrap().players[0];
    let contracts = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "06010")
        .unwrap()
        .card
        .id;
    let knocks = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "01049")
        .unwrap()
        .card
        .id;
    assert!(
        !view
            .choices
            .iter()
            .any(|choice| choice.id == format!("chill:{contracts}"))
    );
    assert!(view.play.as_ref().unwrap().test.is_some());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chill.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    choose(&mut reopened, &format!("chill:{knocks}"));
    let play = reopened.observe().play.unwrap();
    assert!(play.players[0].discard.iter().any(|card| card.id == knocks));
    assert_eq!(play.players[0].damage, 0);
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.code == "01167")
    );
    assert!(play.test.is_none());
}

#[test]
fn crypt_chill_takes_two_damage_when_only_a_permanent_is_controlled() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::CryptChill(CryptChillFixture::NoAsset));
    let before = session.observe().play.unwrap();
    choose(&mut session, "revelation:resolve");
    reveal(&mut session);
    let play = session.observe().play.unwrap();
    assert_eq!(play.players[0].damage, before.players[0].damage + 2);
    assert_eq!(play.players[0].assets, before.players[0].assets);
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.code == "01167")
    );
    assert!(play.test.is_none());
}

#[test]
fn sour_mash_can_pass_crypt_chill_without_asset_loss_or_damage() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::CryptChill(CryptChillFixture::Assets));
    let before = session.observe().play.unwrap();
    let mash = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "05117")
        .unwrap()
        .card
        .id;
    choose(&mut session, "revelation:resolve");
    choose(&mut session, &format!("boost:{mash}"));
    reveal(&mut session);
    let play = session.observe().play.unwrap();
    assert_eq!(play.players[0].assets.len(), before.players[0].assets.len());
    assert_eq!(play.players[0].damage, before.players[0].damage);
    let mash = play.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.id == mash)
        .unwrap();
    assert_eq!(mash.uses, 1);
    assert!(!mash.ready);
    assert!(play.completed_tests.last().unwrap().success);
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.code == "01167")
    );
}

#[test]
fn losing_sign_magick_requires_a_saved_arcane_slot_choice_before_test_cleanup() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::CryptChill(CryptChillFixture::SignMagick));
    let before = session.observe().play.unwrap();
    let sign = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "05112")
        .unwrap()
        .card
        .id;
    let eyes = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "11067")
        .unwrap()
        .card
        .id;
    choose(&mut session, "revelation:resolve");
    reveal(&mut session);
    choose(&mut session, &format!("chill:{sign}"));
    let pending = session.observe();
    assert_eq!(
        pending
            .choices
            .iter()
            .filter(|choice| choice.id != "undo")
            .count(),
        3
    );
    assert!(pending.play.as_ref().unwrap().test.is_some());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("slot.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    choose(&mut reopened, &format!("chill:{eyes}"));
    let play = reopened.observe().play.unwrap();
    assert!(play.players[2].discard.iter().any(|card| card.id == sign));
    assert!(play.players[2].discard.iter().any(|card| card.id == eyes));
    assert!(play.test.is_none());
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.code == "01167")
    );
}
