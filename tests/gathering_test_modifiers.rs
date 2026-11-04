use arkham_unfolded_experiment::gathering::{HostileFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    let view = session.observe_for_play();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}

#[test]
fn modifiers_remain_frozen_when_result_damage_defeats_the_static_bonus_asset() {
    let mut session = Session::with_hostile_fixture(HostileFixture::TestModifierFreeze);
    choose(&mut session, "revelation:resolve");
    let before = session.observe_for_play().play.unwrap();
    let peter = before.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "02033")
        .unwrap()
        .card
        .id;
    let test = before.test.unwrap();
    assert_eq!(test.base_skill, 2);
    let report = serde_json::to_value(&test).unwrap();
    assert_eq!(
        report["breakdown"]["bonuses"][0]["label"],
        "Peter Sylvestre"
    );
    assert_eq!(report["breakdown"]["bonuses"][0]["amount"], 1);
    assert_eq!(
        (
            test.modifiers.cards,
            test.modifiers.other,
            test.modifiers.chaos
        ),
        (0, 1, None)
    );
    choose(&mut session, "window:pass");
    for code in ["04201", "01093"] {
        let card = before.players[1]
            .hand
            .iter()
            .find(|card| card.code == code)
            .unwrap();
        choose(&mut session, &format!("commit:{}", card.id));
    }
    choose(&mut session, "commit:confirm");
    let pending = session.observe_for_play().play.unwrap().test.unwrap();
    assert_eq!(
        (
            pending.modifiers.cards,
            pending.modifiers.other,
            pending.modifiers.chaos
        ),
        (2, 1, None)
    );
    choose(&mut session, "window:pass");
    choose(&mut session, "test:reveal");
    let revealed = session.observe_for_play().play.unwrap().test.unwrap();
    assert_eq!(
        (
            revealed.modifiers.cards,
            revealed.modifiers.other,
            revealed.modifiers.chaos
        ),
        (2, 1, Some(-4))
    );
    assert!(!revealed.modifiers.automatic_failure);
    assert_eq!(revealed.total, Some(1));
    choose(&mut session, "lucky:decline");
    let results = session.observe_for_play();
    let before_damage_revision = results.revision;
    let damage = results
        .choices
        .iter()
        .find(|choice| choice.label.contains("damage"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &damage);
    choose(&mut session, &format!("harm:damage:{peter}"));
    let after = session.observe_for_play();
    let play = after.play.as_ref().unwrap();
    assert!(
        !play.players[1]
            .assets
            .iter()
            .any(|asset| asset.card.id == peter)
    );
    assert!(play.players[1].discard.iter().any(|card| card.id == peter));
    assert_eq!(play.players[1].damage, before.players[1].damage + 1);
    let damage_moment = session
        .presentation_after(before_damage_revision)
        .iter()
        .find(|moment| {
            moment.play.test.is_some()
                && !moment.play.players[1]
                    .assets
                    .iter()
                    .any(|asset| asset.card.id == peter)
                && moment.play.players[1]
                    .discard
                    .iter()
                    .any(|card| card.id == peter)
        })
        .expect(
            "The actual ST7 damage consequence is recorded before remaining results and cleanup",
        );
    let frozen = damage_moment.play.test.as_ref().unwrap();
    assert_eq!(frozen.total, Some(1));
    assert_eq!(
        frozen.breakdown, revealed.breakdown,
        "Named contributions remain frozen after Peter leaves play"
    );
    let detail = frozen.breakdown.as_ref().unwrap();
    assert_eq!(detail.cards.iter().map(|row| row.amount).sum::<i32>(), 2);
    assert!(
        detail
            .cards
            .iter()
            .all(|row| row.owner == arkham_unfolded_experiment::gathering::Investigator::Patrice)
    );
    assert_eq!(detail.tokens.iter().map(|row| row.amount).sum::<i32>(), -4);
    assert_eq!(
        frozen.modifiers, revealed.modifiers,
        "ST7 asset loss must not rewrite ST6 arithmetic"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("frozen-modifiers.json");
    continuity::save_session(&path, &session).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe_for_play(), after);
    assert_eq!(
        reopened.presentation_after(before_damage_revision),
        session.presentation_after(before_damage_revision)
    );
}

#[test]
fn automatic_failure_is_explicit_and_unresolved_chaos_is_not_shown_as_zero() {
    let mut session = Session::with_hostile_fixture(HostileFixture::JacquelineProtectedTokens);
    let pending = session.observe_for_play().play.unwrap().test.unwrap();
    assert_eq!(pending.base_skill, 5);
    assert_eq!(pending.modifiers.chaos, None);
    assert!(!pending.modifiers.automatic_failure);
    for _ in 0..4 {
        let choices = session.observe_for_play().choices;
        if choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        choose(
            &mut session,
            if choices.iter().any(|choice| choice.id == "commit:confirm") {
                "commit:confirm"
            } else {
                "window:pass"
            },
        );
    }
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, "test:reveal");
    let resolved = session.observe_for_play().play.unwrap().test.unwrap();
    assert_eq!(
        (
            resolved.modifiers.cards,
            resolved.modifiers.other,
            resolved.modifiers.chaos
        ),
        (0, 0, Some(-1))
    );
    assert!(resolved.modifiers.automatic_failure);
    assert_eq!(resolved.total, Some(0));
    assert_eq!(resolved.would_succeed, Some(false));
    session.validate().unwrap();
}
