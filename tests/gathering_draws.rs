use arkham_unfolded_experiment::gathering::{DrawFixture, HostileFixture, Session, continuity};

fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe_for_play().revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error:?}"));
}

#[test]
fn watcher_interrupts_the_batch_before_reshuffle_and_take_heart_rewards_wait() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Draws(DrawFixture::PatriceExhaustion));
    let before = session.observe_for_play().play.unwrap();
    let hand_count = before.players[1].hand.len();
    assert_eq!(before.players[1].deck_count, 1);
    choose(&mut session, "result:0");
    let interrupted = session.observe_for_play();
    let patrice = &interrupted.play.as_ref().unwrap().players[1];
    assert_eq!(
        patrice.hand.len(),
        hand_count,
        "The simultaneous two-card draw has not completed"
    );
    assert_eq!(patrice.deck_count, 0);
    assert_eq!(
        patrice.resources, 3,
        "Take Heart's resource reward cannot precede its interrupted draw"
    );
    assert!(
        interrupted
            .choices
            .iter()
            .any(|choice| choice.id == "harm:damage:investigator")
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("interrupted-draw.session.json");
    continuity::save_session(&path, &session).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(
        reopened, session,
        "The held first card and remaining draw survive reopening"
    );
    session = reopened;
    for _ in 0..3 {
        choose(&mut session, "harm:damage:investigator");
    }
    let after_draw = session.observe_for_play();
    let patrice = &after_draw.play.as_ref().unwrap().players[1];
    assert_eq!(
        patrice.damage, 3,
        "Watcher attacks once at the original empty-deck transition"
    );
    assert_eq!(patrice.hand.len(), hand_count + 2);
    assert_eq!(
        patrice.deck_count, 1,
        "The two-card discard is reset and supplies only the missing draw"
    );
    assert_eq!(patrice.resources, 3);
    assert!(
        after_draw
            .choices
            .iter()
            .any(|choice| choice.id == "harm:horror:investigator")
    );
    choose(&mut session, "harm:horror:investigator");
    let after = session.observe_for_play().play.unwrap();
    let patrice = &after.players[1];
    assert_eq!(
        (patrice.damage, patrice.horror, patrice.resources),
        (3, 1, 5)
    );
    assert!(patrice.hand.iter().any(|card| card.code == "06017"));
    assert!(patrice.discard.iter().any(|card| card.code == "04201"));
    assert_eq!(
        after.bag_count,
        before.bag_count + 1,
        "The completed test returns its held token once"
    );
}

#[test]
fn an_ordinary_draw_uses_the_actual_post_mulligan_deck_and_one_action() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    choose(&mut session, "turn:tony");
    let before = session.observe_for_play().play.unwrap();
    choose(&mut session, "action:ordinary:draw");
    let after = session.observe_for_play().play.unwrap();
    assert_eq!(
        after.players[0].hand.len(),
        before.players[0].hand.len() + 1
    );
    assert_eq!(
        after.players[0].deck_count + 1,
        before.players[0].deck_count
    );
    assert!(
        after.players[0]
            .hand
            .iter()
            .any(|card| card.code == "06011"),
        "The actual seed89 post-mulligan deck supplies Tony's Long Colt"
    );
    assert_eq!(after.players[0].resources, before.players[0].resources);
    assert_eq!(after.actions_remaining[0], 2);
    assert_eq!(after.players[0].damage, 0);
    assert_eq!(after.players[0].horror, 0);
    assert_eq!(after.actor, before.actor);
}

#[test]
fn simultaneous_weaknesses_offer_an_order_and_finish_before_the_testers_reward() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Draws(DrawFixture::JacquelineWeaknessOrder));
    let before = session.observe_for_play().play.unwrap();
    choose(&mut session, "result:0");
    let pending = session.observe_for_play();
    assert_eq!(pending.play.as_ref().unwrap().players[2].resources, 3);
    assert_eq!(
        pending.choices.len(),
        2,
        "Both newly drawn weaknesses are genuine order choices"
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.label.contains("Chronophobia"))
    );
    let dark = pending
        .choices
        .iter()
        .find(|choice| choice.label.contains("Dark Future"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &dark);
    let after = session.observe_for_play().play.unwrap();
    let jacqueline = &after.players[2];
    assert!(jacqueline.threats.iter().any(|card| card.code == "60403"));
    assert!(jacqueline.threats.iter().any(|card| card.code == "02039"));
    assert_eq!(
        jacqueline.resources, 5,
        "Take Heart rewards the tester, not the committing investigator"
    );
    assert_eq!(jacqueline.hand.len(), before.players[2].hand.len());
    assert_eq!(jacqueline.deck_count, 0);
    assert_eq!(
        jacqueline.horror, before.players[2].horror,
        "Ending with an empty deck does not reset it when no more cards are requested"
    );
    assert!(
        after.players[1]
            .discard
            .iter()
            .any(|card| card.code == "04201")
    );
    assert_eq!(after.players[1].resources, before.players[1].resources);
    let original = session.clone();
    assert!(
        session
            .choose(session.observe_for_play().revision, &dark)
            .is_err()
    );
    assert_eq!(session, original);
}
