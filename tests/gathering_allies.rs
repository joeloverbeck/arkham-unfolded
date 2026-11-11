use arkham_unfolded_experiment::gathering::{HostileFixture, PeterFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe().revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

#[test]
fn peter_does_not_react_to_another_turn_or_to_zero_horror() {
    for case in [PeterFixture::OtherTurn, PeterFixture::Unharmed] {
        let mut session = Session::with_hostile_fixture(HostileFixture::Peter(case));
        choose(&mut session, "turn:end");
        assert!(
            !session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("peter:")),
            "{case:?}"
        );
        session.validate().unwrap();
    }
}

#[test]
fn mandatory_at_end_of_turn_test_finishes_before_peters_after_turn_reaction() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Peter(PeterFixture::ForcedBeforeReaction));
    let frozen = session.observe().play.unwrap().players[1]
        .threats
        .iter()
        .find(|card| card.code == "01164")
        .unwrap()
        .id;
    choose(&mut session, "turn:end");
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("peter:"))
    );
    // Frozen in Fear's is the only end-of-turn effect, so it begins by itself.
    assert!(
        session
            .observe()
            .history
            .contains(&format!("Automatic: Resolve end-of-turn effect {frozen}"))
    );
    for _ in 0..20 {
        let view = session.observe();
        if view.play.as_ref().unwrap().test.is_none() {
            break;
        }
        assert!(
            !view
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("peter:"))
        );
        let id = [
            "test-card:pass",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "after-test:pass",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .unwrap_or_else(|| panic!("{:?}", view.choices));
        choose(&mut session, id);
    }
    let view = session.observe();
    assert!(
        view.play.as_ref().unwrap().players[1]
            .threats
            .iter()
            .all(|card| card.id != frozen)
    );
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id.starts_with("peter:heal:"))
    );
    session.validate().unwrap();
}

#[test]
fn peters_optional_heal_waits_for_own_turn_end_and_survives_reopen() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Peter(PeterFixture::OwnTurn));
    let before = session.observe();
    let player = &before.play.as_ref().unwrap().players[1];
    let peter = player
        .assets
        .iter()
        .find(|asset| asset.card.code == "02033")
        .unwrap()
        .card
        .id;
    assert!(
        !before
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("peter:"))
    );
    choose(&mut session, "turn:end");
    let pending = session.observe();
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == format!("peter:heal:{peter}")),
        "Original Peter heals only in his optional after-own-turn reaction"
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == "peter:decline")
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("peter.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), pending);
    let mut declined = session.clone();
    choose(&mut declined, "peter:decline");
    assert_eq!(
        declined.observe().play.unwrap().players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == peter)
            .unwrap()
            .horror,
        1
    );
    choose(&mut session, &format!("peter:heal:{peter}"));
    let after = session.observe();
    assert_eq!(
        after.play.as_ref().unwrap().players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == peter)
            .unwrap()
            .horror,
        0
    );
    assert_eq!(
        after.play.as_ref().unwrap().players[1].horror,
        player.horror
    );
    let unchanged = session.clone();
    assert!(
        session
            .choose(after.revision, &format!("peter:heal:{peter}"))
            .is_err()
    );
    assert_eq!(session, unchanged);
}
