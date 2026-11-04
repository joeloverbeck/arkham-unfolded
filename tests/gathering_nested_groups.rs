use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{
        HostileFixture, Session, TokenFixture,
        continuity::{self},
    },
};

// Expectations are derived in issue-89-nested-tokens-followup.md, independently
// of the resolution tree. Privileged fixtures supply cards and physical draws;
// activation, declarations, choices, consequences, save and reopening use Session.

fn choose(session: &mut Session, id: &str) {
    let before = session.observe();
    assert!(
        before.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        before.choices
    );
    session.choose(before.revision, id).unwrap();
}

fn reach_reveal(session: &mut Session) {
    for _ in 0..4 {
        let before = session.observe();
        if before
            .choices
            .iter()
            .any(|choice| choice.id == "test:reveal")
        {
            return;
        }
        if before
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
        {
            choose(session, "commit:confirm");
        } else {
            choose(session, "window:pass");
        }
    }
    panic!("Reveal not reached through the empty commitment/player windows");
}

fn asset(session: &Session, code: &str) -> u16 {
    session.observe().play.unwrap().players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == code)
        .unwrap()
        .card
        .id
}

fn reopen(session: &Session) -> Session {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pending-group.json");
    continuity::save_session(&path, session).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), session.observe());
    reopened
}

fn validate_live_state(session: &Session) {
    session.validate().unwrap();
}

#[test]
fn eyes_preserves_only_the_protected_members_of_ignored_olive_groups() {
    // Study shroud 2, willpower 5, skull 0. G=[-1,-2,skull], A=[-1].
    // The outer A matches G's ignored -1. For G, choosing both numbers leaves
    // skull too: the attempted ignore is prohibited, not a third chosen token.
    for (outer, inner, total, clues) in [
        (1, None, 4, 2),
        (0, Some([0, 2]), 4, 2),
        (0, Some([1, 2]), 3, 1),
        (0, Some([0, 1]), 2, 2),
    ] {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesOliveProtected));
        let eyes = asset(&session, "11067");
        let olive = asset(&session, "04197");
        choose(
            &mut session,
            &format!("action:ordinary:investigate:spell:{eyes}"),
        );
        choose(&mut session, "window:pass");
        choose(&mut session, "commit:confirm");
        choose(&mut session, "window:pass");
        choose(&mut session, "tokens:eyes:1");
        choose(&mut session, &format!("tokens:olive:0:{olive}"));
        choose(&mut session, "test:reveal");
        let revealed = session.observe().play.unwrap();
        assert_eq!(
            revealed.tokens.unwrap().revealed,
            [
                TokenFace::MinusOne,
                TokenFace::MinusTwo,
                TokenFace::Skull,
                TokenFace::MinusOne,
            ]
        );
        assert_eq!(revealed.bag_count, 12);
        choose(&mut session, &format!("tokens:position:{outer}"));
        let mut resumed = reopen(&session);
        for active in [&mut session, &mut resumed] {
            choose(active, "tokens:confirm");
            if let Some(kept) = inner {
                choose(active, &format!("tokens:keep:{}", kept[0]));
                choose(active, &format!("tokens:keep:{}", kept[1]));
                choose(active, "tokens:confirm");
            }
        }
        assert_eq!(resumed.observe(), session.observe());
        let after = session.observe().play.unwrap();
        let result = after.completed_tests.last().unwrap();
        assert_eq!(result.total, total);
        assert!(result.success);
        assert_eq!(after.players[2].clues, clues);
        assert!(
            after.players[2]
                .threats
                .iter()
                .any(|card| card.code == "60403")
        );
        assert_eq!(after.bag_count, 16);
        assert_eq!(after.actions_remaining[2], 2);
        assert_eq!(
            after.players[2]
                .assets
                .iter()
                .find(|asset| asset.card.id == eyes)
                .unwrap()
                .uses,
            8
        );
        assert!(
            !after.players[2]
                .assets
                .iter()
                .find(|asset| asset.card.id == olive)
                .unwrap()
                .ready
        );
        validate_live_state(&resumed);
    }
}

fn jacqueline_olive_reveal() -> Session {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::JacquelineOlive));
    let olive = asset(&session, "04197");
    reach_reveal(&mut session);
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, &format!("tokens:olive:0:{olive}"));
    choose(&mut session, "test:reveal");
    assert_eq!(
        session.observe().play.unwrap().tokens.unwrap().revealed,
        [
            TokenFace::AutoFail,
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            TokenFace::Zero,
            TokenFace::PlusOne,
        ]
    );
    session
}

#[test]
fn jacqueline_cancels_a_surviving_numeric_olive_pair_as_one_non_autofail_position() {
    let mut session = jacqueline_olive_reveal();
    choose(&mut session, "tokens:keep:1");
    choose(&mut session, "tokens:keep:2");
    choose(&mut session, "tokens:confirm");
    let before = session.observe();
    assert!(session.choose(before.revision, "tokens:cancel:1").is_err());
    assert_eq!(session.observe(), before);
    choose(&mut session, "tokens:cancel-position:0");
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:confirm"),
        "A two-member non-autofail group spends only one of two cancellations"
    );
    session = reopen(&session);
    choose(&mut session, "tokens:cancel-position:1");
    choose(&mut session, "tokens:confirm");
    let after = session.observe().play.unwrap();
    // Olive ignored AF; Jacqueline cancelled G=[-1,-2] and A=[0]; B=[+1] remains.
    assert_eq!(after.completed_tests.last().unwrap().total, 6);
    assert!(after.completed_tests.last().unwrap().success);
    assert_eq!(after.bag_count, 16);
    validate_live_state(&session);
}

#[test]
fn jacqueline_can_cancel_a_mixed_autofail_olive_pair_as_one_autofail_position() {
    let mut session = jacqueline_olive_reveal();
    choose(&mut session, "tokens:keep:0");
    choose(&mut session, "tokens:keep:2");
    choose(&mut session, "tokens:confirm");
    choose(&mut session, "tokens:cancel-position:0");
    let before = session.observe();
    assert!(
        session
            .choose(before.revision, "tokens:cancel-position:1")
            .is_err()
    );
    assert_eq!(session.observe(), before);
    session = reopen(&session);
    choose(&mut session, "tokens:confirm");
    let after = session.observe().play.unwrap();
    // G=[AF,-2] was one autofail position; A=[0] and B=[+1] both remain.
    assert_eq!(after.completed_tests.last().unwrap().total, 6);
    assert!(after.completed_tests.last().unwrap().success);
    assert_eq!(after.bag_count, 16);
    validate_live_state(&session);
}

#[test]
fn prophecy_classifies_olive_by_its_surviving_symbols_before_choosing_a_position() {
    // G=[skull,-1,-2], A=[0], B=[+1], C=[-1], D=[-2]. An ignored skull
    // cannot force Prophecy to choose G; a retained skull does require G.
    for (kept, expected_positions, selected, total) in [
        ([0, 1], vec!["tokens:position:0"], 0, 4),
        (
            [1, 2],
            vec![
                "tokens:position:0",
                "tokens:position:1",
                "tokens:position:2",
                "tokens:position:3",
                "tokens:position:4",
            ],
            1,
            5,
        ),
    ] {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::ProphecyOlive));
        let olive = asset(&session, "04197");
        let prophecy = session.observe().play.unwrap().players[2]
            .hand
            .iter()
            .find(|card| card.code == "04032")
            .unwrap()
            .id;
        reach_reveal(&mut session);
        choose(&mut session, &format!("tokens:prophecy:0:{prophecy}"));
        choose(&mut session, &format!("tokens:olive:0.0:{olive}"));
        choose(&mut session, "test:reveal");
        assert_eq!(session.observe().play.unwrap().bag_count, 9);
        choose(&mut session, &format!("tokens:keep:{}", kept[0]));
        choose(&mut session, &format!("tokens:keep:{}", kept[1]));
        choose(&mut session, "tokens:confirm");
        let before = session.observe();
        let positions: Vec<_> = before
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("tokens:position:"))
            .map(|choice| choice.id.as_str())
            .collect();
        assert_eq!(positions, expected_positions);
        if selected == 0 {
            assert!(
                session
                    .choose(before.revision, "tokens:position:1")
                    .is_err()
            );
            assert_eq!(session.observe(), before);
        }
        session = reopen(&session);
        choose(&mut session, &format!("tokens:position:{selected}"));
        choose(&mut session, "tokens:confirm");
        let after = session.observe().play.unwrap();
        assert_eq!(after.completed_tests.last().unwrap().total, total);
        assert!(after.completed_tests.last().unwrap().success);
        assert_eq!(after.bag_count, 16);
        assert_eq!(after.players[2].resources, 4);
        assert!(
            after.players[2]
                .discard
                .iter()
                .any(|card| card.id == prophecy)
        );
        validate_live_state(&session);
    }
}

#[test]
fn olive_first_jacqueline_cancels_within_the_five_token_request_before_olive_keeps_two() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::JacquelineOlive));
    let olive = asset(&session, "04197");
    reach_reveal(&mut session);
    choose(&mut session, &format!("tokens:olive:0:{olive}"));
    choose(&mut session, "tokens:jacqueline:at:0");
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:cancel:0");
    choose(&mut session, "tokens:confirm");
    choose(&mut session, "tokens:keep:3");
    choose(&mut session, "tokens:keep:4");
    choose(&mut session, "tokens:confirm");
    let after = session.observe().play.unwrap();
    assert_eq!(after.completed_tests.last().unwrap().total, 6);
    assert_eq!(after.bag_count, 16);
    validate_live_state(&session);
}

#[test]
fn eyes_only_offers_surviving_positions_after_jacqueline_cancels_nested_groups() {
    // Eyes supplies two positions; Jacqueline adds two, and Olive replaces G.
    // G=[-1,-2,skull], A=[-1], B=[0], C=[+1]. Dark Future protects skull.
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesOliveJacqueline));
    let eyes = asset(&session, "11067");
    let olive = asset(&session, "04197");
    choose(
        &mut session,
        &format!("action:ordinary:investigate:spell:{eyes}"),
    );
    choose(&mut session, "window:pass");
    choose(&mut session, "commit:confirm");
    choose(&mut session, "window:pass");
    choose(&mut session, "tokens:eyes:1");
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, &format!("tokens:olive:0:{olive}"));
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:keep:0");
    choose(&mut session, "tokens:keep:2");
    choose(&mut session, "tokens:confirm");
    choose(&mut session, "tokens:cancel-position:0");
    choose(&mut session, "tokens:cancel-position:1");
    choose(&mut session, "tokens:confirm");
    let before = session.observe();
    assert!(
        !before
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:position:1")
    );
    choose(&mut session, "tokens:position:2");
    choose(&mut session, "tokens:confirm");
    let after = session.observe().play.unwrap();
    assert_eq!(after.completed_tests.last().unwrap().total, 5);
    assert_eq!(after.players[2].clues, 1);
    assert_eq!(after.bag_count, 16);
    validate_live_state(&session);
}

#[test]
fn jacqueline_group_cancellation_preserves_a_protected_symbol_inside_the_group() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Tokens(
        TokenFixture::JacquelineOliveProtected,
    ));
    let olive = asset(&session, "04197");
    reach_reveal(&mut session);
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, &format!("tokens:olive:0:{olive}"));
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:keep:0");
    choose(&mut session, "tokens:keep:1");
    choose(&mut session, "tokens:confirm");
    let offered = session.observe();
    let cancel = offered
        .choices
        .iter()
        .find(|choice| choice.id == "tokens:cancel-position:0")
        .unwrap();
    assert!(cancel.label.contains("Cancel eligible tokens in"));
    assert!(
        cancel
            .label
            .contains(&format!("Dark Future keeps {}", TokenFace::Skull.label()))
    );
    choose(&mut session, "tokens:cancel-position:0");
    choose(&mut session, "tokens:cancel-position:1");
    choose(&mut session, "tokens:confirm");
    let after = session.observe().play.unwrap();
    let result = after.completed_tests.last().unwrap();
    // G's skull survives; G's -1 and A's 0 are cancelled; B's +1 remains.
    assert_eq!(result.total, 6);
    assert_eq!(after.bag_count, 16);
    assert!(
        after.players[2]
            .threats
            .iter()
            .any(|card| card.code == "60403")
    );
    assert!(
        session
            .observe()
            .history
            .iter()
            .any(|entry| entry.contains("2 resolved, 1 ignored, 2 cancelled"))
    );
    validate_live_state(&session);
}

#[test]
fn prophecy_uses_actual_survivors_when_another_symbol_exists_or_ignore_is_prohibited() {
    for (fixture, expected, chosen, total) in [
        (
            TokenFixture::ProphecyOliveOtherSymbol,
            "tokens:position:3",
            3,
            4,
        ),
        (
            TokenFixture::ProphecyOliveProtected,
            "tokens:position:0",
            0,
            2,
        ),
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::Tokens(fixture));
        let olive = asset(&session, "04197");
        let prophecy = session.observe().play.unwrap().players[2]
            .hand
            .iter()
            .find(|card| card.code == "04032")
            .unwrap()
            .id;
        reach_reveal(&mut session);
        choose(&mut session, &format!("tokens:prophecy:0:{prophecy}"));
        choose(&mut session, &format!("tokens:olive:0.0:{olive}"));
        choose(&mut session, "test:reveal");
        choose(&mut session, "tokens:keep:1");
        choose(&mut session, "tokens:keep:2");
        choose(&mut session, "tokens:confirm");
        let before = session.observe();
        let positions: Vec<_> = before
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("tokens:position:"))
            .map(|choice| choice.id.as_str())
            .collect();
        assert_eq!(positions, [expected]);
        choose(&mut session, &format!("tokens:position:{chosen}"));
        choose(&mut session, "tokens:confirm");
        if fixture == TokenFixture::ProphecyOliveProtected {
            choose(&mut session, "harm:horror:investigator");
        }
        let after = session.observe().play.unwrap();
        assert_eq!(after.completed_tests.last().unwrap().total, total);
        assert_eq!(after.bag_count, 16);
        validate_live_state(&session);
    }
}
