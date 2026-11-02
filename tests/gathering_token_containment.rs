use arkham_unfolded_experiment::gathering::{HostileFixture, Investigator, Session, TokenFixture};

fn choose(session: &mut Session, id: &str) {
    let observation = session.observe_for_play();
    assert!(observation.choices.iter().any(|choice| choice.id == id));
    session.choose(observation.revision, id).unwrap();
}

#[test]
fn sealed_premonition_withholds_voice_before_action_payment_and_opportunity() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Tokens(
        TokenFixture::PremonitionVoiceOpportunity,
    ));
    let first = session.observe_for_play();
    let premonition = first
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("premonition:"))
        .expect("Actual fast Premonition play")
        .id
        .clone();
    choose(&mut session, &premonition);
    choose(&mut session, "test:reveal");

    let before = session.observe_for_play();
    let play = before.play.as_ref().unwrap();
    let jacqueline = &play.players[2];
    let voice = jacqueline
        .hand
        .iter()
        .find(|card| card.code == "60416")
        .unwrap();
    let unavailable = format!("action:ordinary:play:spell:{}", voice.id);
    assert_eq!(play.actions_remaining[2], 3);
    assert_eq!(jacqueline.resources, 5);
    assert_eq!(play.bag_count, 15);
    assert_eq!(play.premonition.as_ref().unwrap().faces.len(), 1);
    assert!(play.scenario.enemies.iter().any(|enemy| {
        enemy.card.code == "01116"
            && enemy.engaged == Some(Investigator::Jacqueline)
            && !enemy.exhausted
    }));
    assert!(
        !before.choices.iter().any(|choice| choice.id == unavailable),
        "The unresolved multi-token composition must be withheld before action payment"
    );
    let unchanged = session.clone();
    assert!(session.choose(before.revision, &unavailable).is_err());
    assert_eq!(
        session, unchanged,
        "Refusal preserves all state and recorded decisions"
    );
    assert!(
        !session
            .observe_for_play()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("opportunity:")),
        "Refusal must not queue the engaged enemy's opportunity attack"
    );
    session.validate().unwrap();

    // Positive control: the same prepared enemy really provokes an opportunity
    // attack when a supported ordinary action is paid.
    choose(&mut session, "action:ordinary:resource");
    let pending = session.observe_for_play();
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("opportunity:"))
    );
    let pending_play = pending.play.as_ref().unwrap();
    assert_eq!(pending_play.actions_remaining[2], 2);
    assert_eq!(pending_play.players[2].resources, 5);
    assert_eq!(pending_play.premonition, play.premonition);
}
