use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{
        HostileFixture, Investigator, Session, TokenFixture,
        continuity::{self, Replay},
    },
};

fn choose(session: &mut Session, id: &str) {
    let observation = session.observe_for_play();
    assert!(
        observation.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        observation.choices
    );
    session.choose(observation.revision, id).unwrap();
}

#[test]
fn sealed_premonition_voice_pays_once_then_resumes_after_opportunity() {
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
    let before = session.observe_for_play().play.unwrap();
    let voice = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "60416")
        .unwrap()
        .id;
    assert_eq!(before.actions_remaining[2], 3);
    assert_eq!(before.players[2].resources, 5);
    assert_eq!(before.bag_count, 15);
    assert!(
        before
            .scenario
            .enemies
            .iter()
            .any(|enemy| enemy.card.code == "01116"
                && enemy.engaged == Some(Investigator::Jacqueline)
                && !enemy.exhausted)
    );
    choose(&mut session, &format!("action:ordinary:play:spell:{voice}"));
    let paid = session.observe_for_play();
    let play = paid.play.as_ref().unwrap();
    assert_eq!(play.actions_remaining[2], 2);
    assert_eq!(
        play.players[2].resources, 5,
        "Event resolution waits for opportunity attack"
    );
    assert_eq!(play.premonition, before.premonition);
    let attack = paid
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("voice-opportunity.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    for current in [&mut session, &mut reopened] {
        choose(current, &attack);
        for _ in 0..5 {
            let pending = current.observe_for_play();
            if pending
                .choices
                .iter()
                .any(|choice| choice.id == "test:reveal")
            {
                break;
            }
            let assignment = pending
                .choices
                .iter()
                .find(|choice| {
                    choice.id == "harm:damage:investigator"
                        || choice.id == "harm:horror:investigator"
                })
                .expect("Investigator harm assignment")
                .id
                .clone();
            choose(current, &assignment);
        }
        choose(current, "test:reveal");
        current.validate().unwrap();
    }
    assert_eq!(session.observe_for_play(), reopened.observe_for_play());
    let after = session.observe_for_play().play.unwrap();
    assert_eq!(after.actions_remaining[2], 2);
    assert!(after.players[2].damage > before.players[2].damage);
    assert_eq!(
        after.players[2].resources, 10,
        "Voice adds 1 + 2*2 symbols after the attack"
    );
    assert_eq!(
        after.players[2]
            .discard
            .iter()
            .filter(|card| card.id == voice)
            .count(),
        1
    );
    assert!(after.premonition.is_none());
    assert_eq!(after.bag_count, 16);
    let recording = session.recording();
    let batch = recording
        .presentation_moments
        .iter()
        .find(|moment| {
            moment.play.tokens.as_ref().is_some_and(|tokens| {
                tokens.revealed == [TokenFace::ElderSign, TokenFace::Skull, TokenFace::Zero]
            })
        })
        .expect("One sealed token plus two fresh tokens");
    assert_eq!(batch.play.bag_count, 13);
    let mut replay = Replay::new(session.recording()).unwrap();
    while replay.next() {}
    assert_eq!(replay.current(), &session.observe_for_play());
}
