use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{HostileFixture, Session},
};

#[test]
fn pending_token_cancellation_reopens_with_the_same_physical_tokens() {
    use arkham_unfolded_experiment::gathering::continuity::{self, Replay};
    let mut session = Session::with_hostile_fixture(HostileFixture::JacquelineCancellation);
    reach_reveal(&mut session);
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:cancel:0");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("token-choice.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe_for_play(), session.observe_for_play());
    choose(&mut reopened, "tokens:confirm");
    choose(&mut session, "tokens:confirm");
    assert_eq!(reopened.observe_for_play(), session.observe_for_play());
    let mut replay = Replay::new(reopened.recording()).unwrap();
    while replay.next() {}
    assert_eq!(replay.current(), &session.observe_for_play());
}

fn choose(session: &mut Session, id: &str) {
    let view = session.observe_for_play();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}

fn reach_reveal(session: &mut Session) {
    for _ in 0..4 {
        let choices = session.observe_for_play().choices;
        if choices.iter().any(|choice| choice.id == "test:reveal") {
            return;
        }
        if choices.iter().any(|choice| choice.id == "commit:confirm") {
            choose(session, "commit:confirm");
        } else {
            choose(session, "window:pass");
        }
    }
    panic!("Reveal was not reached through the empty player/commitment windows");
}

#[test]
fn eyes_can_ignore_the_whole_olive_group_before_olive_selects_any_tokens() {
    use arkham_unfolded_experiment::gathering::{TokenFixture, continuity};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesOlive));
    let play = session.observe_for_play().play.unwrap();
    let eyes = play.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "11067")
        .unwrap()
        .card
        .id;
    let olive = play.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .id;
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
    let outer = session.observe_for_play();
    let play = outer.play.as_ref().unwrap();
    assert_eq!(
        play.tokens.as_ref().unwrap().revealed,
        [
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            TokenFace::Skull,
            TokenFace::MinusOne
        ]
    );
    assert_eq!(play.bag_count, 12);
    assert!(
        !outer
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:keep:")),
        "An inner physical-token choice must not precede the Eyes position choice"
    );
    assert!(session.choose(outer.revision, "tokens:keep:0").is_err());
    assert_eq!(
        session.observe_for_play(),
        outer,
        "A forged flattened choice cannot mutate the group or pay again"
    );
    choose(&mut session, "tokens:position:1");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("eyes-outer.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe_for_play(), session.observe_for_play());
    choose(&mut session, "tokens:confirm");
    choose(&mut reopened, "tokens:confirm");
    assert_eq!(reopened.observe_for_play(), session.observe_for_play());
    let after = session.observe_for_play().play.unwrap();
    assert_eq!(after.completed_tests.last().unwrap().total, 4);
    assert_eq!(
        after.players[2].clues, 2,
        "The selected ordinary -1 matches an ignored member of Olive's entire group"
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
}

#[test]
fn eyes_chooses_olive_before_its_two_survivors_and_matches_only_the_retained_members() {
    use arkham_unfolded_experiment::gathering::{
        TokenFixture,
        continuity::{self, Replay},
    };
    // Independent source-derived oracle: Study shroud 2, willpower 5,
    // G=[-1,-2,skull(0)] and A=[-1]. FAQ p17 preserves G until Eyes chooses it;
    // §2.5 and p22 give one extra clue only if a surviving member matches.
    for (position, kept, outer_ignored, expected_total, expected_clues) in [
        (0, [0, 2], 3, 4, 2),
        (0, [1, 2], 3, 3, 1),
        // With Olive second, only the later retained member (-1) matches.
        (1, [1, 3], 0, 2, 2),
    ] {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesOlive));
        let play = session.observe_for_play().play.unwrap();
        let eyes = play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "11067")
            .unwrap()
            .card
            .id;
        let olive = play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "04197")
            .unwrap()
            .card
            .id;
        choose(
            &mut session,
            &format!("action:ordinary:investigate:spell:{eyes}"),
        );
        choose(&mut session, "window:pass");
        choose(&mut session, "commit:confirm");
        choose(&mut session, "window:pass");
        choose(&mut session, "tokens:eyes:1");
        choose(&mut session, &format!("tokens:olive:{position}:{olive}"));
        assert!(
            !session
                .observe_for_play()
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("tokens:jacqueline")
                    || choice.id.starts_with("tokens:prophecy:"))
        );
        choose(&mut session, "test:reveal");
        choose(&mut session, &format!("tokens:position:{position}"));
        choose(&mut session, "tokens:confirm");
        let inner = session.observe_for_play();
        let tokens = inner.play.as_ref().unwrap().tokens.as_ref().unwrap();
        assert_eq!(
            tokens.revealed,
            [
                TokenFace::MinusOne,
                TokenFace::MinusTwo,
                TokenFace::Skull,
                TokenFace::MinusOne
            ]
        );
        assert_eq!(
            tokens.ignored,
            [outer_ignored],
            "Eyes ignores A before Olive chooses its survivors"
        );
        assert!(
            tokens.cancelled.is_empty(),
            "Ignoring does not become cancellation"
        );
        assert_eq!(inner.play.as_ref().unwrap().bag_count, 12);
        assert!(
            !inner
                .choices
                .iter()
                .any(|choice| choice.id == format!("tokens:keep:{outer_ignored}")
                    || choice.id.starts_with("tokens:position:"))
        );
        choose(&mut session, &format!("tokens:keep:{}", kept[0]));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eyes-inner.json");
        continuity::save_session(&path, &session).unwrap();
        let mut reopened = continuity::load_session(&path).unwrap();
        assert_eq!(reopened.observe_for_play(), session.observe_for_play());
        for active in [&mut session, &mut reopened] {
            choose(active, &format!("tokens:keep:{}", kept[1]));
            choose(active, "tokens:confirm");
        }
        assert_eq!(reopened.observe_for_play(), session.observe_for_play());
        let after = reopened.observe_for_play().play.unwrap();
        assert_eq!(after.completed_tests.last().unwrap().total, expected_total);
        assert_eq!(after.players[2].clues, expected_clues);
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
        let mut replay = Replay::new(reopened.recording()).unwrap();
        while replay.next() {}
        assert_eq!(replay.current(), &reopened.observe_for_play());
    }
}

#[test]
fn eyes_olive_keeps_the_dark_future_partial_group_boundary_without_spending_olive() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesOliveProtected));
    let play = session.observe_for_play().play.unwrap();
    let eyes = play.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "11067")
        .unwrap()
        .card
        .id;
    let olive = play.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:investigate:spell:{eyes}"),
    );
    choose(&mut session, "window:pass");
    choose(&mut session, "commit:confirm");
    choose(&mut session, "window:pass");
    choose(&mut session, "tokens:eyes:1");
    let before = session.observe_for_play();
    assert!(
        !before
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:olive:"))
    );
    assert!(
        session
            .choose(before.revision, &format!("tokens:olive:0:{olive}"))
            .is_err()
    );
    assert_eq!(session.observe_for_play(), before);
    let play = before.play.unwrap();
    assert!(
        play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == olive)
            .unwrap()
            .ready
    );
    assert_eq!(
        play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.id == eyes)
            .unwrap()
            .uses,
        8
    );
    assert_eq!(play.actions_remaining[2], 2);
    assert_eq!(play.bag_count, 16);
}

#[test]
fn robes_cannot_reduce_dark_prophecy_because_its_printed_trait_is_only_augury() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    // Bound original face df42c9255e4bf557adb016fad3af22b8229e7ffb83becdd0056dd9480df7540f.
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::RobesProphecy));
    reach_reveal(&mut session);
    let before = session.observe_for_play();
    assert!(
        !before
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:prophecy:"))
    );
    let play = before.play.as_ref().unwrap();
    let robes = play.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap();
    let event = play.players[2]
        .hand
        .iter()
        .find(|card| card.code == "04032")
        .unwrap();
    assert!(robes.ready);
    assert_eq!(play.players[2].resources, 0);
    assert!(
        session
            .choose(
                before.revision,
                &format!("tokens:prophecy:0:{}:robes:{}", event.id, robes.card.id)
            )
            .is_err()
    );
    assert_eq!(session.observe_for_play(), before);
    choose(&mut session, "test:reveal");
    session.validate().unwrap();
}

#[test]
fn voice_of_ra_counts_curse_and_elder_sign_without_skill_test_effects() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::VoiceRaCurse));
    assert_eq!(
        session.observe_for_play().play.as_ref().unwrap().players[2].resources,
        6
    );
    choose(&mut session, "test:reveal");
    let play = session.observe_for_play().play.unwrap();
    assert_eq!(
        play.players[2].resources, 10,
        "Starting5 plusVoiceinitial1 plus2 for eachofCurse/ElderSign"
    );
    assert_eq!(
        play.bag_count, 17,
        "Non-testcurse returns to thebag without an extra draw"
    );
    assert_eq!(play.players[2].horror, 2);
    assert!(
        play.players[2]
            .threats
            .iter()
            .any(|card| card.code == "60403")
    );
    assert!(play.players[2].hand.is_empty());
    assert!(
        play.players[2]
            .discard
            .iter()
            .any(|card| card.code == "60416")
    );
    assert!(play.completed_tests.is_empty());
}

#[test]
fn a_paid_voice_of_ra_does_not_resolve_after_lethal_opportunity_damage() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session = Session::with_hostile_fixture(HostileFixture::Tokens(
        TokenFixture::VoiceRaLethalOpportunity,
    ));
    let before = session.observe_for_play().play.unwrap();
    let event = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "60416")
        .unwrap()
        .id;
    choose(&mut session, &format!("action:ordinary:play:spell:{event}"));
    let pending = session.observe_for_play();
    assert_eq!(pending.play.as_ref().unwrap().players[2].resources, 5);
    assert!(
        !pending.play.as_ref().unwrap().players[2]
            .hand
            .iter()
            .any(|card| card.id == event)
    );
    let attack = pending
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("opportunity:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &attack);
    let play = session.observe_for_play().play.unwrap();
    assert!(play.players[2].defeated);
    assert!(play.players[2].removed.iter().any(|card| card.id == event));
    assert!(
        play.players[2].resources <= 5,
        "Voice of Ra's effect never begins"
    );
    assert!(play.tokens.is_none());
}

#[test]
fn olive_seals_two_premonition_tokens_and_both_resolve_with_one_spell_drawback() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::PremonitionOlive));
    let olive = session
        .observe_for_play()
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("tokens:olive:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &olive);
    assert!(
        !session.observe_for_play().choices.iter().any(|choice| {
            choice.id.starts_with("tokens:jacqueline") || choice.id.starts_with("tokens:prophecy:")
        }),
        "Unimplemented nested group choices must not flatten Olive's two-token result"
    );
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:keep:0");
    choose(&mut session, "tokens:keep:1");
    choose(&mut session, "tokens:confirm");
    let sealed = session.observe_for_play().play.unwrap();
    assert_eq!(
        sealed.premonition.as_ref().unwrap().faces,
        [TokenFace::ElderSign, TokenFace::PlusOne]
    );
    assert_eq!(sealed.bag_count, 14);
    let clairvoyance = sealed.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60408")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:investigate:spell:{clairvoyance}"),
    );
    reach_reveal(&mut session);
    choose(&mut session, "test:reveal");
    choose(&mut session, "harm:horror:investigator");
    let play = session.observe_for_play().play.unwrap();
    assert_eq!(play.completed_tests.last().unwrap().total, 7);
    assert_eq!(play.players[2].clues, 2);
    assert_eq!(
        play.players[2].horror, 3,
        "Matching drawback triggers once despite two positive tokens"
    );
    assert_eq!(play.bag_count, 16);
    assert!(play.premonition.is_none());
}

#[test]
fn eyes_with_two_premonition_tokens_is_withheld_without_spending_an_action_or_charges() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::PremonitionEyes));
    let olive = session
        .observe_for_play()
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
    let before = session.observe_for_play();
    let play = before.play.as_ref().unwrap();
    assert_eq!(
        play.premonition.as_ref().unwrap().faces,
        [TokenFace::ElderSign, TokenFace::PlusOne]
    );
    assert_eq!(play.bag_count, 14);
    let eyes = play.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "11067")
        .unwrap();
    assert_eq!(eyes.uses, 9);
    assert_eq!(play.actions_remaining[2], 3);
    let unavailable = format!("action:ordinary:investigate:spell:{}", eyes.card.id);
    assert!(
        !before.choices.iter().any(|choice| choice.id == unavailable),
        "The unresolved Eyes/Premonition group composition must be gated before payment, not reduced to a zero-charge physical-token selection"
    );
    assert!(
        before
            .choices
            .iter()
            .any(|choice| choice.id == "action:ordinary:investigate")
    );
    assert!(session.choose(before.revision, &unavailable).is_err());
    assert_eq!(
        session.observe_for_play(),
        before,
        "A forged unsupported activation cannot spend an action, resource, charge, or sealed token"
    );
    session.validate().unwrap();
}

#[test]
fn premonition_seals_then_forces_the_next_clairvoyance_reveal() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::Premonition));
    choose(&mut session, "test:reveal");
    let sealed = session.observe_for_play().play.unwrap();
    assert_eq!(
        sealed
            .premonition
            .as_ref()
            .expect("Premonition remains in play")
            .faces,
        [TokenFace::ElderSign]
    );
    assert_eq!(sealed.bag_count, 15);
    let clairvoyance = sealed.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60408")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:investigate:spell:{clairvoyance}"),
    );
    reach_reveal(&mut session);
    let forced = session.observe_for_play();
    assert!(
        !forced
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:jacqueline")
                || choice.id.starts_with("tokens:olive")
                || choice.id.starts_with("tokens:prophecy"))
    );
    choose(&mut session, "test:reveal");
    let play = session.observe_for_play().play.unwrap();
    assert!(play.premonition.is_none());
    assert_eq!(play.bag_count, 16);
    assert_eq!(play.completed_tests.last().unwrap().total, 6);
    assert_eq!(play.players[2].clues, 2);
    assert_eq!(play.players[2].horror, 3);
    assert!(
        play.players[2]
            .threats
            .iter()
            .any(|card| card.code == "60403")
    );
    assert!(
        play.players[2]
            .discard
            .iter()
            .any(|card| card.code == "04199")
    );
    assert!(play.players[2].hand.is_empty());
}

#[test]
fn eyes_katarina_and_confidence_preserve_protected_tokens_and_count_all_reveals() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesKatarina));
    let before = session.observe_for_play().play.unwrap();
    let confidence = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "11097")
        .unwrap()
        .id;
    let katarina = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "11069")
        .unwrap()
        .card
        .id;
    while !session
        .observe_for_play()
        .choices
        .iter()
        .any(|choice| choice.id == format!("commit:{confidence}"))
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, &format!("commit:{confidence}"));
    choose(&mut session, "commit:confirm");
    while session
        .observe_for_play()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, "tokens:eyes:1");
    choose(&mut session, &format!("tokens:katarina:0:{katarina}"));
    choose(&mut session, "test:reveal");
    let revealed = session.observe_for_play();
    assert_eq!(
        revealed
            .play
            .as_ref()
            .unwrap()
            .tokens
            .as_ref()
            .unwrap()
            .revealed
            .len(),
        4
    );
    assert!(
        !revealed
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:keep:0" || choice.id == "tokens:keep:1")
    );
    choose(&mut session, "tokens:keep:2");
    choose(&mut session, "tokens:confirm");
    let play = session.observe_for_play().play.unwrap();
    assert_eq!(play.completed_tests.last().unwrap().total, 10);
    assert_eq!(play.players[2].clues, 1);
    assert!(
        play.players[2]
            .threats
            .iter()
            .any(|card| card.code == "60403")
    );
    assert!(play.players[2].hand.is_empty());
    assert_eq!(
        play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "11067")
            .unwrap()
            .uses,
        8
    );
    assert!(
        !play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == "11069")
            .unwrap()
            .ready
    );
    assert_eq!(play.bag_count, 16);
}

#[test]
fn eyes_matches_a_number_ignored_by_its_nested_katarina_reveal() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::EyesIgnoredMatch));
    let before = session.observe_for_play().play.unwrap();
    let confidence = before.players[2]
        .hand
        .iter()
        .find(|card| card.code == "11097")
        .unwrap()
        .id;
    let katarina = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "11069")
        .unwrap()
        .card
        .id;
    while session
        .observe_for_play()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, &format!("commit:{confidence}"));
    choose(&mut session, "commit:confirm");
    while session
        .observe_for_play()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, "tokens:eyes:1");
    choose(&mut session, &format!("tokens:katarina:0:{katarina}"));
    choose(&mut session, "test:reveal");
    choose(&mut session, "tokens:keep:3");
    choose(&mut session, "tokens:confirm");
    let play = session.observe_for_play().play.unwrap();
    assert_eq!(play.completed_tests.last().unwrap().total, 8);
    assert_eq!(
        play.players[2].clues, 2,
        "The chosen -1 matches both earlier Katarina-ignored -1 tokens; Eyes grants one extra clue"
    );
    assert_eq!(play.bag_count, 16);
}

#[test]
fn dark_prophecy_uses_taboo_preference_and_cannot_be_declared_after_reveal() {
    use arkham_unfolded_experiment::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::DarkProphecy));
    let card = session.observe_for_play().play.unwrap().players[2]
        .hand
        .iter()
        .find(|card| card.code == "04032")
        .unwrap()
        .id;
    reach_reveal(&mut session);
    let declaration = format!("tokens:prophecy:0:{card}");
    choose(&mut session, &declaration);
    choose(&mut session, "test:reveal");
    let pending = session.observe_for_play();
    assert_eq!(pending.play.as_ref().unwrap().bag_count, 11);
    assert!(
        !pending
            .choices
            .iter()
            .any(|choice| choice.id == "tokens:keep:0")
    );
    assert!(session.choose(pending.revision, &declaration).is_err());
    assert_eq!(session.observe_for_play(), pending);
    choose(&mut session, "tokens:keep:1");
    choose(&mut session, "tokens:confirm");
    let play = session.observe_for_play().play.unwrap();
    assert_eq!(play.completed_tests.last().unwrap().total, 5);
    assert!(play.completed_tests.last().unwrap().success);
    assert_eq!(play.players[2].resources, 4);
    assert_eq!(
        play.players[2].hand.len(),
        1,
        "Ignored Elder Sign draws one card for Jacqueline's own test"
    );
    assert!(
        play.players[2]
            .discard
            .iter()
            .any(|entry| entry.id == card && entry.code == "04032")
    );
    assert_eq!(play.bag_count, 16);
}

#[test]
fn nested_prophecy_cleanup_reveals_the_declared_batch_before_selection() {
    use arkham_unfolded_experiment::gathering::{TokenFixture, continuity};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::NestedProphecyCleanup));
    let prophecy = session.observe_for_play().play.unwrap().players[2].hand[0].id;
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, &format!("tokens:prophecy:0:{prophecy}"));
    choose(&mut session, "test:reveal");
    let nested = session.observe_for_play();
    assert_eq!(
        nested
            .play
            .as_ref()
            .unwrap()
            .tokens
            .as_ref()
            .unwrap()
            .revealed
            .len(),
        11
    );
    assert_eq!(nested.play.as_ref().unwrap().bag_count, 5);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested-tokens.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe_for_play(), nested);
    choose(&mut session, "tokens:keep:1");
    choose(&mut session, "tokens:confirm");
    let outer = session.observe_for_play();
    let play = outer.play.as_ref().unwrap();
    assert_eq!(play.tokens.as_ref().unwrap().revealed.len(), 11);
    assert_eq!(play.tokens.as_ref().unwrap().ignored, [3, 4]);
    assert_eq!(play.bag_count, 5);
    assert!(
        play.players[2]
            .threats
            .iter()
            .any(|card| card.code == "60403")
    );
    assert!(session.choose(outer.revision, "tokens:cancel:0").is_err());
    assert_eq!(session.observe_for_play(), outer);
    choose(&mut session, "tokens:cancel:5");
    choose(&mut session, "tokens:cancel:6");
    choose(&mut session, "tokens:confirm");
    let finished = session.observe_for_play().play.unwrap();
    assert_eq!(finished.bag_count, 16);
    assert!(finished.completed_tests.is_empty());
    assert_eq!(finished.players[2].resources, 4);
    assert_eq!(finished.players[2].horror, 2);
    assert!(finished.players[2].hand.is_empty());
    assert!(
        finished.players[2]
            .discard
            .iter()
            .any(|card| card.code == "60403")
    );
}

#[test]
fn jacqueline_cancels_the_autofail_and_resolves_both_remaining_numbers() {
    let mut session = Session::with_hostile_fixture(HostileFixture::JacquelineCancellation);
    reach_reveal(&mut session);
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, "test:reveal");
    let play = session.observe_for_play().play.unwrap();
    assert_eq!(
        play.tokens.as_ref().unwrap().revealed,
        [TokenFace::AutoFail, TokenFace::Zero, TokenFace::MinusOne]
    );
    assert_eq!(play.bag_count, 13);
    choose(&mut session, "tokens:cancel:0");
    choose(&mut session, "tokens:confirm");
    let play = session.observe_for_play().play.unwrap();
    let result = play.completed_tests.last().unwrap();
    assert!(result.success);
    assert_eq!(result.total, 4);
    assert_eq!(play.bag_count, 16);
}

#[test]
fn dark_future_allows_extra_reveals_but_protects_every_symbol_in_the_witness() {
    let mut session = Session::with_hostile_fixture(HostileFixture::JacquelineProtectedTokens);
    reach_reveal(&mut session);
    choose(&mut session, "tokens:jacqueline");
    choose(&mut session, "test:reveal");
    choose(&mut session, "result:0");
    let play = session.observe_for_play().play.unwrap();
    assert!(!play.completed_tests.last().unwrap().success);
    assert_eq!(play.players[2].horror, 6);
    assert_eq!(play.bag_count, 16);
    assert!(
        play.players[2]
            .threats
            .iter()
            .any(|card| card.code == "60403")
    );
    assert!(
        !session
            .observe_for_play()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tokens:cancel:"))
    );
}
