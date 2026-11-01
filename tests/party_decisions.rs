use arkham_unfolded_experiment::{
    TokenFace,
    party::{Investigator, Session, Stage},
};

fn choose(game: &mut Session, id: &str) {
    game.choose(game.observe().revision, id).unwrap();
    game.validate()
        .unwrap_or_else(|error| panic!("after {id}: {error}"));
}
fn test(game: &mut Session, commit: &str) {
    for id in ["investigate", commit, "reveal", "resolve:0"] {
        choose(game, id);
    }
}

#[test]
fn resolving_the_only_token_finishes_and_returns_it_without_an_undo_stop() {
    let mut game = Session::with_draws(vec![TokenFace::Zero]);
    for id in ["select:tony", "investigate", "commit:none", "reveal"] {
        choose(&mut game, id);
    }
    assert_eq!(game.observe().bag_count, 15);
    choose(&mut game, "resolve:0");
    let view = game.observe();
    assert_eq!(view.stage, Stage::Action);
    assert_eq!(view.bag_count, 16);
    assert!(view.held_tokens().is_empty());
    assert_eq!(view.test.as_ref().unwrap().result, Some(true));
    assert!(!view.can_undo);
    assert!(!view.choices.iter().any(|choice| choice.id == "finish-test"));
    assert!(game.choose(view.revision, "finish-test").is_err());
}

#[test]
fn automatic_cleanup_preserves_real_cancellation_and_order_undo_without_duplicate_tokens() {
    let mut game = Session::with_draws(vec![
        TokenFace::AutoFail,
        TokenFace::MinusOne,
        TokenFace::Zero,
    ]);
    for id in [
        "select:tony",
        "investigate",
        "commit:none",
        "use-jacqueline",
        "cancel:0",
    ] {
        choose(&mut game, id);
    }
    let order_choice = game.observe();
    choose(&mut game, "resolve:1");
    choose(&mut game, "resolve:2");
    assert_eq!(game.observe().bag_count, 16);
    assert!(game.observe().held_tokens().is_empty());
    choose(&mut game, "undo");
    let restored = game.observe();
    assert_eq!(restored.stage, Stage::Resolve);
    assert_eq!(restored.tokens, order_choice.tokens);
    assert_eq!(restored.bag_count, 13);
    assert_eq!(restored.held_tokens().len(), 3);
    choose(&mut game, "resolve:2");
    choose(&mut game, "resolve:1");
    assert_eq!(game.observe().bag_count, 16);
    assert_eq!(game.observe().investigator(Investigator::Tony).clues, 1);
    choose(&mut game, "undo");
    choose(&mut game, "undo");
    assert_eq!(game.observe().stage, Stage::Cancellation);
    assert_eq!(game.observe().bag_count, 13);
    assert!(
        !game.observe().can_undo,
        "the initial hidden reveal remains a barrier"
    );
}

#[test]
fn separate_turns_keep_unused_actions_and_the_exact_three_card_prefix() {
    let mut game = Session::with_draws(vec![]);
    for id in ["select:patrice", "draw", "draw", "draw"] {
        choose(&mut game, id);
    }
    let view = game.observe();
    assert_eq!(view.stage, Stage::Action);
    assert_eq!(view.investigator(Investigator::Patrice).ordinary_actions, 0);
    assert_eq!(
        view.investigator(Investigator::Patrice)
            .hand
            .iter()
            .map(|c| c.code.as_str())
            .collect::<Vec<_>>(),
        ["07032", "01080", "01080", "01093"]
    );
    assert!(!view.choices.iter().any(|c| c.id == "draw"));
    choose(&mut game, "end-turn");
    choose(&mut game, "select:tony");
    choose(&mut game, "resource");
    choose(&mut game, "end-turn");
    assert_eq!(
        game.observe()
            .investigator(Investigator::Tony)
            .ordinary_actions,
        2
    );
    assert!(!game.observe().choices.iter().any(|c| c.id == "select:tony"));
    choose(&mut game, "select:jacqueline");
    choose(&mut game, "end-turn");
    assert_eq!(game.observe().stage, Stage::Ended);
    assert_eq!(game.observe().study_clues, 6);
    game.validate().unwrap();
}

#[test]
fn patrice_can_pay_lucky_on_her_final_test_and_prevent_cultist_horror() {
    let mut game = Session::with_draws(vec![TokenFace::Cultist]);
    for id in [
        "select:patrice",
        "draw",
        "resource",
        "investigate",
        "commit:none",
        "reveal",
        "resolve:0",
    ] {
        choose(&mut game, id);
    }
    assert!(game.observe().choices.iter().any(|c| c.id == "lucky:101"));
    choose(&mut game, "lucky:101");
    let view = game.observe();
    let patrice = view.investigator(Investigator::Patrice);
    assert_eq!(
        (
            patrice.resources,
            patrice.clues,
            patrice.horror,
            patrice.ordinary_actions
        ),
        (0, 1, 0, 0)
    );
    assert_eq!(view.test.as_ref().unwrap().skill_value, 4);
    assert_eq!(view.test.as_ref().unwrap().modifier, -1);
    assert_eq!(view.test.as_ref().unwrap().result, Some(true));
    assert!(patrice.discard.iter().any(|card| card.code == "01080"));
}

#[test]
fn elders_belong_to_the_testing_investigator_and_patrice_retains_a_chosen_discard() {
    let mut game = Session::with_draws(vec![
        TokenFace::Zero,
        TokenFace::ElderSign,
        TokenFace::MinusFour,
        TokenFace::AutoFail,
    ]);
    choose(&mut game, "select:tony");
    test(&mut game, "commit:100");
    choose(&mut game, "end-turn");
    for id in [
        "select:patrice",
        "draw",
        "resource",
        "investigate",
        "commit:none",
        "use-jacqueline",
        "cancel:2",
        "resolve:0",
        "resolve:1",
        "lucky:101",
    ] {
        choose(&mut game, id);
    }
    assert!(
        game.observe()
            .choices
            .iter()
            .any(|c| c.id == "shuffle:retain:100")
    );
    assert!(
        game.observe()
            .choices
            .iter()
            .any(|c| c.id == "shuffle:retain:101")
    );
    let before = game
        .observe()
        .investigator(Investigator::Patrice)
        .deck_count;
    choose(&mut game, "shuffle:retain:100");
    let view = game.observe();
    assert_eq!(
        view.investigator(Investigator::Patrice)
            .discard
            .iter()
            .map(|c| c.code.as_str())
            .collect::<Vec<_>>(),
        ["07032"]
    );
    assert_eq!(
        view.investigator(Investigator::Patrice).deck_count,
        before + 1
    );
    assert_eq!(view.investigator(Investigator::Jacqueline).hand.len(), 1);
    assert_eq!(view.investigator(Investigator::Patrice).ordinary_actions, 0);
    assert!(!view.choices.iter().any(|c| c.id == "draw"));
    assert!(view.can_undo);
    choose(&mut game, "undo");
    assert_eq!(
        game.observe()
            .investigator(Investigator::Patrice)
            .discard
            .len(),
        2
    );
    assert_eq!(
        game.observe()
            .investigator(Investigator::Patrice)
            .deck_count,
        before
    );
}

#[test]
fn sixth_clue_on_third_action_preserves_the_turn_and_advances_the_actual_map() {
    let mut game = Session::with_draws(vec![TokenFace::Zero; 6]);
    choose(&mut game, "select:tony");
    for _ in 0..3 {
        test(&mut game, "commit:none");
    }
    choose(&mut game, "end-turn");
    choose(&mut game, "select:jacqueline");
    for _ in 0..3 {
        test(&mut game, "commit:none");
    }
    let before = game.observe();
    assert_eq!(before.stage, Stage::Closing);
    assert_eq!(
        before
            .investigator(Investigator::Jacqueline)
            .ordinary_actions,
        0
    );
    assert!(!before.investigator(Investigator::Jacqueline).turn_finished);
    assert_eq!(
        before.investigator(Investigator::Patrice).ordinary_actions,
        3
    );
    let mut declined = game.clone();
    choose(&mut declined, "decline-advance");
    assert_eq!(declined.observe().locations[0].name, "Study");
    assert_eq!(
        declined
            .observe()
            .investigators
            .iter()
            .map(|i| i.clues)
            .sum::<u8>(),
        6
    );
    choose(&mut game, "advance");
    let reveal = game.observe();
    assert!(
        reveal.endpoint.is_none(),
        "read Act 1b before applying its effects"
    );
    assert_eq!(reveal.locations[0].name, "Study");
    assert!(
        reveal
            .investigators
            .iter()
            .all(|i| i.location == "Study" && i.clues == 0)
    );
    assert_eq!(
        reveal
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["continue-act"]
    );
    assert!(!reveal.can_undo);
    assert!(game.choose(reveal.revision, "undo").is_err());
    choose(&mut game, "continue-act");
    let view = game.observe();
    assert_eq!(view.stage, Stage::Ended);
    assert_eq!(
        view.locations
            .iter()
            .map(|l| l.name.as_str())
            .collect::<Vec<_>>(),
        ["Hallway", "Cellar", "Attic", "Parlor"]
    );
    assert!(view.locations[0].revealed);
    assert!(
        view.investigators
            .iter()
            .all(|i| i.location == "Hallway" && i.clues == 0)
    );
    assert!(!view.investigator(Investigator::Jacqueline).turn_finished);
    assert_eq!(view.investigator(Investigator::Patrice).ordinary_actions, 3);
    assert!(!view.can_undo, "Continue is not an undo destination");
    assert!(game.choose(view.revision, "continue-act").is_err());
}

#[test]
fn every_standard_face_uses_the_testing_investigators_original_rules() {
    for (face, modifier, success, horror) in [
        (TokenFace::PlusOne, 1, true, 0),
        (TokenFace::Zero, 0, true, 0),
        (TokenFace::MinusOne, -1, true, 0),
        (TokenFace::MinusTwo, -2, false, 0),
        (TokenFace::MinusThree, -3, false, 0),
        (TokenFace::MinusFour, -4, false, 0),
        (TokenFace::Skull, 0, true, 0),
        (TokenFace::Cultist, -1, true, 0),
        (TokenFace::Tablet, -2, false, 0),
        (TokenFace::AutoFail, 0, false, 0),
        (TokenFace::ElderSign, 2, true, 0),
    ] {
        let mut game = Session::with_draws(vec![face]);
        choose(&mut game, "select:tony");
        test(&mut game, "commit:none");
        let view = game.observe();
        let test = view.test.as_ref().unwrap();
        assert_eq!(
            (test.modifier, test.result),
            (modifier, Some(success)),
            "{face:?}"
        );
        assert_eq!(view.investigator(Investigator::Tony).horror, horror);
        assert_eq!(
            view.contracts_bounties,
            if face == TokenFace::ElderSign { 7 } else { 6 }
        );
        assert_eq!(view.bag_count, 16);
        game.validate().unwrap();
    }
}

#[test]
fn jacqueline_canceled_elder_draw_is_hers_and_blocks_undo_across_the_new_card() {
    let mut game = Session::with_draws(vec![
        TokenFace::ElderSign,
        TokenFace::Zero,
        TokenFace::MinusOne,
    ]);
    for id in [
        "select:jacqueline",
        "investigate",
        "commit:none",
        "use-jacqueline",
        "cancel:0,2",
    ] {
        choose(&mut game, id);
    }
    let view = game.observe();
    assert_eq!(view.investigator(Investigator::Jacqueline).hand.len(), 2);
    assert_eq!(
        view.investigator(Investigator::Jacqueline).hand[1].code,
        "60408"
    );
    assert!(!view.can_undo);
    assert!(
        view.tokens
            .iter()
            .any(|token| token.face == TokenFace::ElderSign)
    );
    choose(&mut game, "resolve:1");
    assert_eq!(game.observe().bag_count, 16);
}

#[test]
fn later_curse_reveal_still_offers_jacqueline_and_a_poor_cancellation_still_fails() {
    let mut game = Session::with_draws(vec![
        TokenFace::Curse,
        TokenFace::AutoFail,
        TokenFace::Zero,
        TokenFace::MinusOne,
    ]);
    for id in [
        "select:tony",
        "investigate",
        "commit:100",
        "reveal",
        "resolve:0",
    ] {
        choose(&mut game, id);
    }
    assert!(
        game.observe()
            .choices
            .iter()
            .any(|choice| choice.id == "use-jacqueline")
    );
    for id in ["use-jacqueline", "cancel:2,3", "resolve:1"] {
        choose(&mut game, id);
    }
    assert_eq!(game.observe().test.as_ref().unwrap().result, Some(false));
    assert_eq!(game.observe().investigator(Investigator::Tony).clues, 0);
    assert_eq!(game.observe().bag_count, 16);
}

#[test]
fn excess_support_and_unaffordable_assets_are_refused_without_mutation() {
    let mut game = Session::with_draws(vec![]);
    for id in [
        "select:jacqueline",
        "draw",
        "end-turn",
        "select:tony",
        "investigate",
    ] {
        choose(&mut game, id);
    }
    let before = game.clone();
    let error = game
        .choose(game.observe().revision, "commit:200,201")
        .unwrap_err();
    assert!(error.0.contains("one card"));
    assert_eq!(game, before);
    let mut fresh = Session::with_draws(vec![]);
    choose(&mut fresh, "select:jacqueline");
    assert!(
        fresh
            .choose(fresh.observe().revision, "play:200")
            .unwrap_err()
            .0
            .contains("resources")
    );
    assert!(
        fresh
            .choose(fresh.observe().revision, "fight")
            .unwrap_err()
            .0
            .contains("enemy")
    );
    assert!(
        fresh
            .choose(fresh.observe().revision, "move")
            .unwrap_err()
            .0
            .contains("destination")
    );
}

#[test]
fn canceled_curse_returns_to_the_bag_and_resolution_order_preserves_held_multiplicity() {
    let mut game =
        Session::with_draws(vec![TokenFace::AutoFail, TokenFace::Curse, TokenFace::Zero]);
    for id in [
        "select:tony",
        "investigate",
        "commit:100",
        "use-jacqueline",
        "cancel:1,2",
        "resolve:0",
    ] {
        choose(&mut game, id);
    }
    assert_eq!(
        (game.observe().bag_count, game.observe().curses_in_pool),
        (17, 9)
    );
    assert_eq!(game.observe().test.as_ref().unwrap().total, Some(0));
    assert!(game.observe().history.last().unwrap().contains("auto-fail"));
    for order in [["resolve:1", "resolve:2"], ["resolve:2", "resolve:1"]] {
        let mut game = Session::with_draws(vec![
            TokenFace::AutoFail,
            TokenFace::Cultist,
            TokenFace::ElderSign,
        ]);
        for id in [
            "select:tony",
            "investigate",
            "commit:none",
            "use-jacqueline",
            "cancel:0",
        ] {
            choose(&mut game, id);
        }
        choose(&mut game, order[0]);
        assert_eq!(
            game.observe().bag_count,
            13,
            "All three revealed physical tokens stay held through ST.4"
        );
        choose(&mut game, order[1]);
        assert_eq!(game.observe().test.as_ref().unwrap().total, Some(4));
        assert_eq!(game.observe().contracts_bounties, 7);
        assert_eq!(game.observe().bag_count, 16);
    }
}

#[test]
fn each_prefix_and_support_limit_remain_playable_without_revealing_the_hidden_tail() {
    for (actor, prefix) in [
        ("tony", ["01089", "01089", "03263"]),
        ("jacqueline", ["60408", "11067", "11067"]),
    ] {
        let mut game = Session::with_draws(vec![]);
        choose(&mut game, &format!("select:{actor}"));
        for _ in 0..3 {
            choose(&mut game, "draw");
        }
        let view = game.observe();
        let investigator = view.investigator(if actor == "tony" {
            Investigator::Tony
        } else {
            Investigator::Jacqueline
        });
        assert_eq!(
            investigator.hand[1..]
                .iter()
                .map(|card| card.code.as_str())
                .collect::<Vec<_>>(),
            prefix
        );
        let before = game.clone();
        assert!(game.choose(view.revision, "draw").is_err());
        assert_eq!(game, before);
    }
}

#[test]
fn unlucky_cultist_and_auto_fail_remain_valid_patrice_choices() {
    for face in [TokenFace::Cultist, TokenFace::AutoFail] {
        let mut game = Session::with_draws(vec![face]);
        for id in [
            "select:patrice",
            "draw",
            "resource",
            "investigate",
            "commit:none",
            "reveal",
            "resolve:0",
        ] {
            choose(&mut game, id);
        }
        assert!(
            game.observe()
                .choices
                .iter()
                .any(|choice| choice.id == "lucky:101")
        );
        choose(
            &mut game,
            if face == TokenFace::AutoFail {
                "lucky:101"
            } else {
                "decline-lucky"
            },
        );
        let view = game.observe();
        assert_eq!(view.test.as_ref().unwrap().result, Some(false));
        assert_eq!(
            view.investigator(Investigator::Patrice).horror,
            u8::from(face == TokenFace::Cultist)
        );
        assert_eq!(
            view.investigator(Investigator::Patrice).resources,
            u8::from(face == TokenFace::Cultist)
        );
    }
}
