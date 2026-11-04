//! The visual sidecar must describe real draws without changing saved rules.
use arkham_unfolded_experiment::gathering::{
    DrawCue, DrawFixture, DrawSource, HostileFixture, Investigator, SearchFixture, Session,
    continuity::{self},
};

fn choose(session: &mut Session, id: &str) -> Vec<DrawCue> {
    session
        .choose_with_draws(session.observe().revision, id)
        .unwrap()
}

#[test]
fn opening_mulligan_and_action_cues_survive_record_roundtrip_without_persisting_animation() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    let opening = choose(&mut session, "lead:tony");
    assert_eq!(opening.len(), 3);
    for (cue, owner) in opening.iter().zip(Investigator::ALL) {
        assert_eq!(cue.batch.source, DrawSource::Investigator(owner));
        assert_eq!(cue.batch.cards.len(), 5);
        assert!(cue.batch.cards.iter().all(|card| !card.weakness));
        assert!(cue.sequence.is_none());
    }
    let rejected = session.observe().investigators[0].hand[0].clone();
    assert!(choose(&mut session, &format!("mulligan:{}", rejected.id)).is_empty());
    let replacement = choose(&mut session, "confirm-mulligan");
    assert_eq!(
        replacement[0].batch.returned,
        std::slice::from_ref(&rejected)
    );
    assert_eq!(replacement[0].batch.cards.len(), 1);
    assert_ne!(replacement[0].batch.cards[0].id, rejected.id);
    choose(&mut session, "confirm-mulligan");
    choose(&mut session, "confirm-mulligan");
    choose(&mut session, "turn:tony");
    let action = choose(&mut session, "action:ordinary:draw");
    assert_eq!(action.len(), 1);
    assert_eq!(
        action[0].batch.source,
        DrawSource::Investigator(Investigator::Tony)
    );
    assert_eq!(action[0].batch.cards.len(), 1);
    assert!(action[0].sequence.is_some());
    session.validate().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.json");
    continuity::save_session(&path, &session).unwrap();
    assert_eq!(continuity::load_session(path).unwrap(), session);
}

#[test]
fn failed_save_or_stale_choice_returns_no_animation_or_committed_effect() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::with_seed(19);
    let before = session.clone();
    assert!(
        continuity::commit_choice_with_draws(dir.path(), &mut session, 0, "lead:tony").is_err()
    );
    assert_eq!(session, before);
    assert!(session.choose_with_draws(12, "lead:tony").is_err());
    assert_eq!(session, before);
}

#[test]
fn interrupted_draw_stays_hidden_until_completion_and_weakness_order_remains_a_choice() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Draws(DrawFixture::PatriceExhaustion));
    assert!(choose(&mut session, "result:0").is_empty());
    for _ in 0..3 {
        assert!(choose(&mut session, "harm:damage:investigator").is_empty());
    }
    let resumed = choose(&mut session, "harm:confirm");
    assert_eq!(resumed.len(), 1);
    assert_eq!(resumed[0].batch.cards.len(), 2);
    assert_eq!(
        resumed[0].batch.source,
        DrawSource::Investigator(Investigator::Patrice)
    );
    session.validate().unwrap();

    let mut session =
        Session::with_hostile_fixture(HostileFixture::Draws(DrawFixture::JacquelineWeaknessOrder));
    let cues = choose(&mut session, "result:0");
    assert_eq!(
        cues[0]
            .batch
            .cards
            .iter()
            .map(|card| card.code.as_str())
            .collect::<Vec<_>>(),
        ["60403", "02039"]
    );
    assert_eq!(session.observe().choices.len(), 2);
    session.validate().unwrap();
}

#[test]
fn encounter_draw_has_reveal_anchor_but_search_placement_is_not_a_draw() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HuntTargets));
    assert!(choose(&mut session, "mythos:draw").is_empty());
    let cues = choose(&mut session, "search:decline-hunt");
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].batch.source, DrawSource::Encounter);
    assert_eq!(cues[0].batch.cards[0].code, "01118");
    let anchor = session
        .presentation_after(cues[0].revision - 1)
        .iter()
        .find(|moment| Some(moment.sequence) == cues[0].sequence)
        .unwrap();
    assert_eq!(
        anchor.play.pending_revelation.as_ref().unwrap().card,
        cues[0].batch.cards[0]
    );
    assert!(
        session
            .observe()
            .play
            .unwrap()
            .encounter_discard
            .iter()
            .any(|card| card.code == "01118")
    );

    for fixture in [SearchFixture::HuntTargets, SearchFixture::HuntNoMatch] {
        let mut session = Session::with_hostile_fixture(HostileFixture::Search(fixture));
        let event = session.observe().play.unwrap().players[0]
            .hand
            .iter()
            .find(|card| card.code == "03263")
            .unwrap()
            .id;
        assert!(choose(&mut session, "mythos:draw").is_empty());
        assert!(choose(&mut session, &format!("search:play-hunt:{event}")).is_empty());
        if fixture == SearchFixture::HuntTargets {
            let enemy = session
                .observe()
                .play
                .unwrap()
                .searched_encounter
                .iter()
                .find(|card| card.code == "01119")
                .unwrap()
                .id;
            assert!(choose(&mut session, &format!("search:enemy:{enemy}")).is_empty());
        } else {
            assert_eq!(
                choose(&mut session, "search:no-match")[0].batch.source,
                DrawSource::Encounter
            );
        }
        session.validate().unwrap();
    }
}
