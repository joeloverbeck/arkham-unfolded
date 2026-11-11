//! Finite source-derived oracles: issue-89-premonition-followup.md P1–P7.
use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{
        HostileFixture, Session, TokenFixture,
        continuity::{self},
    },
};

fn choose(session: &mut Session, id: &str) {
    let before = session.observe();
    assert!(
        before.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        before.choices
    );
    session.choose(before.revision, id).unwrap();
}

fn seal_two(case: TokenFixture) -> Session {
    let mut session = Session::with_hostile_fixture(HostileFixture::Tokens(case));
    let olive = session
        .observe()
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("tokens:olive:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &olive);
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:keep:0");
    choose(&mut session, "tokens:keep:1");
    choose(&mut session, "tokens:confirm");
    let play = session.observe().play.unwrap();
    assert_eq!(play.premonition.unwrap().faces.len(), 2);
    assert_eq!(play.bag_count, 14);
    session
}

fn validate_live_state(session: &Session) {
    session.validate().unwrap();
}

#[test]
fn mandatory_dark_future_uses_the_sealed_group_and_four_fresh_positions() {
    // 5 requested positions => [-1,-2] plus four fresh tokens, never five
    // fresh tokens or the sealed group alone. Only the elder-sign case clears.
    for (case, fresh, removed) in [
        (
            TokenFixture::PremonitionDarkFuture,
            TokenFace::ElderSign,
            true,
        ),
        (
            TokenFixture::PremonitionDarkFutureAbsent,
            TokenFace::Cultist,
            false,
        ),
    ] {
        let mut session = seal_two(case);
        let sealed = session.observe().play.unwrap().premonition.unwrap();
        choose(&mut session, "turn:end");
        // Dark Future is the only end-of-turn effect, so it begins by itself.
        assert!(
            session
                .observe()
                .history
                .iter()
                .any(|entry| entry.starts_with("Automatic: Resolve end-of-turn effect "))
        );
        let pending = session.observe();
        assert!(
            pending
                .choices
                .iter()
                .any(|choice| choice.id == "test:reveal"),
            "{:?}",
            pending.choices
        );
        assert!(
            !pending
                .choices
                .iter()
                .any(|choice| choice.id == "tokens:jacqueline:at:0")
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dark-future.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        choose(&mut session, "test:reveal");
        choose(&mut reopened, "test:reveal");
        assert_eq!(session.observe(), reopened.observe());
        let expected = vec![
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            fresh,
            TokenFace::Zero,
            TokenFace::PlusOne,
            TokenFace::Skull,
        ];
        let moments = session.presentation_after(pending.revision);
        let physical = moments
            .iter()
            .find(|moment| {
                moment
                    .play
                    .tokens
                    .as_ref()
                    .is_some_and(|tokens| tokens.revealed == expected)
            })
            .expect("Six held physical tokens");
        assert_eq!(physical.play.bag_count, 10);
        let after = session.observe().play.unwrap();
        assert_eq!(after.bag_count, 16);
        assert!(after.premonition.is_none());
        assert_eq!(
            after.players[2]
                .threats
                .iter()
                .any(|card| card.code == "60403"),
            !removed
        );
        assert_eq!(
            after.players[2]
                .discard
                .iter()
                .filter(|card| card.id == sealed.card.id)
                .count(),
            1
        );
        assert!(after.completed_tests.is_empty());
        validate_live_state(&session);
    }
}

#[test]
fn voice_counts_all_sealed_symbols_and_exactly_two_fresh_positions() {
    let mut session = seal_two(TokenFixture::PremonitionVoice);
    let before = session.observe().play.unwrap();
    let voice = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "60416")
        .unwrap()
        .id;
    let hand_count = before.players[2].hand.len();
    choose(&mut session, &format!("action:ordinary:play:spell:{voice}"));
    let pending = session.observe();
    assert_eq!(pending.play.as_ref().unwrap().actions_remaining[2], 2);
    assert_eq!(pending.play.as_ref().unwrap().players[2].resources, 6);
    assert!(
        !pending
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:jacqueline"
                || choice.id == "tokens:jacqueline:at:0"
                || choice.id.starts_with("tokens:prophecy:0:"))
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:jacqueline:at:1")
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:prophecy:1:"))
    );
    let immutable = session.clone();
    assert!(
        session
            .choose(pending.revision, "tokens:jacqueline:at:0")
            .is_err()
    );
    assert_eq!(session, immutable);
    choose(&mut session, "test:reveal");
    let expected = [
        TokenFace::Skull,
        TokenFace::ElderSign,
        TokenFace::Cultist,
        TokenFace::Zero,
    ];
    let moments = session.presentation_after(pending.revision);
    let physical = moments
        .iter()
        .find(|moment| {
            moment
                .play
                .tokens
                .as_ref()
                .is_some_and(|tokens| tokens.revealed == expected)
        })
        .unwrap();
    assert_eq!(physical.play.bag_count, 12);
    let after = session.observe().play.unwrap();
    assert_eq!(
        after.players[2].resources, 12,
        "Taboo10: 1 + 2*3 symbols, including sealed elder sign"
    );
    assert_eq!(
        after.players[2].hand.len(),
        hand_count - 1,
        "Non-test elder sign draws no card"
    );
    assert_eq!(
        after.players[2]
            .discard
            .iter()
            .filter(|card| card.id == voice)
            .count(),
        1
    );
    assert_eq!(after.bag_count, 16);
    assert!(after.completed_tests.is_empty());
    validate_live_state(&session);
}

#[test]
fn eyes_keeps_a_sealed_group_indivisible_at_zero_or_one_charge() {
    // Study shroud2; willpower5. Whole group [-1,-2] => total2. Fresh
    // [-1] => total4. With one charge either choice matches an ignored -1.
    for (charges, position, total, clues) in [(0, 0, 2, 1), (1, 0, 2, 2), (1, 1, 4, 2)] {
        let mut session = seal_two(TokenFixture::PremonitionEyes);
        let before = session.observe().play.unwrap();
        let sealed_id = before.premonition.as_ref().unwrap().card.id;
        let eyes = before.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "11067")
            .unwrap()
            .card
            .id;
        choose(
            &mut session,
            &format!("action:ordinary:investigate:spell:{eyes}"),
        );
        for id in ["window:pass", "commit:confirm", "window:pass"] {
            choose(&mut session, id);
        }
        choose(&mut session, &format!("tokens:eyes:{charges}"));
        let declaration = session.observe();
        assert!(
            !declaration
                .choices
                .iter()
                .any(|choice| choice.id == "tokens:jacqueline:at:0")
        );
        choose(&mut session, "test:reveal");
        let outer = session.observe();
        let held = outer.play.as_ref().unwrap();
        assert_eq!(held.bag_count, 14 - charges as usize);
        assert_eq!(
            held.tokens.as_ref().unwrap().revealed.len(),
            2 + charges as usize
        );
        assert!(
            !outer
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("tokens:keep:"))
        );
        let immutable = session.clone();
        assert!(session.choose(outer.revision, "tokens:keep:0").is_err());
        assert_eq!(session, immutable);
        choose(&mut session, &format!("tokens:position:{position}"));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eyes-selected.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        choose(&mut session, "tokens:confirm");
        choose(&mut reopened, "tokens:confirm");
        assert_eq!(session.observe(), reopened.observe());
        let after = session.observe().play.unwrap();
        assert_eq!(after.completed_tests.last().unwrap().total, total);
        assert_eq!(after.players[2].clues, clues);
        assert_eq!(after.actions_remaining[2], 2);
        assert_eq!(
            after.players[2]
                .assets
                .iter()
                .find(|asset| asset.card.id == eyes)
                .unwrap()
                .uses,
            9 - charges
        );
        assert_eq!(
            after.players[2]
                .discard
                .iter()
                .filter(|card| card.id == sealed_id)
                .count(),
            1
        );
        assert_eq!(after.bag_count, 16);
        validate_live_state(&session);
    }
}
