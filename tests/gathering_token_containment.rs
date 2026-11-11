use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{
        HostileFixture, Investigator, Session, TokenFixture,
        continuity::{self},
    },
};

fn choose(session: &mut Session, id: &str) {
    let observation = session.observe();
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
    let first = session.observe();
    let premonition = first
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("premonition:"))
        .expect("Actual fast Premonition play")
        .id
        .clone();
    choose(&mut session, &premonition);
    choose(&mut session, "test:reveal");
    let before = session.observe().play.unwrap();
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
    let playing = session.observe().revision;
    choose(&mut session, &format!("action:ordinary:play:spell:{voice}"));
    // The lone attack of opportunity resolves by itself; its presented harm
    // precedes the event's resolution.
    let play = session
        .presentation_after(playing)
        .iter()
        .find(|moment| moment.caption.starts_with("Jacqueline Fine resolves"))
        .map(|moment| moment.play.clone())
        .unwrap();
    assert_eq!(play.actions_remaining[2], 2);
    assert_eq!(
        play.players[2].resources, 5,
        "Event resolution waits for opportunity attack"
    );
    assert_eq!(play.premonition, before.premonition);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("voice-opportunity.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    for current in [&mut session, &mut reopened] {
        for _ in 0..5 {
            let pending = current.observe();
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
    assert_eq!(session.observe(), reopened.observe());
    let after = session.observe().play.unwrap();
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
    let recording = session.presentation_after(0);
    let batch = recording
        .iter()
        .find(|moment| {
            moment.play.tokens.as_ref().is_some_and(|tokens| {
                tokens.revealed == [TokenFace::ElderSign, TokenFace::Skull, TokenFace::Zero]
            })
        })
        .expect("One sealed token plus two fresh tokens");
    assert_eq!(batch.play.bag_count, 13);
}
