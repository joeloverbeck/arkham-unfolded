use arkham_unfolded_experiment::{
    TokenFace,
    tony::{Harm, HarmRecipient, HarmSource, Session, Stage},
};

fn choose(game: &mut Session, id: &str) {
    game.choose(game.observe().revision, id).unwrap();
}

fn test(game: &mut Session, action: &str) {
    choose(game, action);
    choose(game, "commit:none");
    choose(game, "reveal");
    choose(game, "resolve");
}

#[test]
fn opportunity_attack_is_one_recorded_harm_event_with_both_amounts() {
    let mut game = Session::with_draws(vec![]);
    choose(&mut game, "investigate");
    let view = serde_json::to_value(game.observe()).unwrap();
    assert_eq!(
        view.get("harm"),
        Some(&serde_json::json!([{
            "source": "opportunity_attack",
            "recipient": "tony",
            "damage": 1,
            "horror": 1
        }]))
    );
}

#[test]
fn a_final_tablet_fight_records_tablet_then_enemy_damage_then_injury() {
    let mut game = Session::with_draws(vec![TokenFace::Zero, TokenFace::Tablet]);
    test(&mut game, "fight");
    choose(&mut game, "resource");
    choose(&mut game, "resource");
    test(&mut game, "fight");
    let view = game.observe();
    assert_eq!(view.stage, Stage::Ended);
    assert_eq!(
        view.harm,
        Some(vec![
            Harm {
                source: HarmSource::Tablet,
                recipient: HarmRecipient::Tony,
                damage: 1,
                horror: 0,
            },
            Harm {
                source: HarmSource::BasicFight,
                recipient: HarmRecipient::Ghoul,
                damage: 1,
                horror: 0,
            },
            Harm {
                source: HarmSource::InternalInjury,
                recipient: HarmRecipient::Tony,
                damage: 1,
                horror: 0,
            },
        ])
    );
    assert_eq!((view.damage, view.horror, view.ghoul.damage), (4, 2, 2));
    assert_eq!((view.ghoul.alive, view.resources), (false, 8));
}

#[test]
fn failed_cultist_evade_records_horror_and_undo_does_not_repeat_it() {
    let mut game = Session::with_draws(vec![TokenFace::Cultist]);
    test(&mut game, "evade");
    assert_eq!(
        game.observe().harm,
        Some(vec![Harm {
            source: HarmSource::Cultist,
            recipient: HarmRecipient::Tony,
            damage: 0,
            horror: 1,
        }])
    );
    choose(&mut game, "undo");
    assert_eq!(game.observe().harm, Some(vec![]));
    assert_eq!(game.observe().horror, 0);
    game.validate().unwrap();
    choose(&mut game, "resolve");
    assert_eq!(game.observe().horror, 1);
    assert_eq!(game.observe().harm.unwrap().len(), 1);
}

#[test]
fn opportunity_preview_follows_current_offered_actions_and_enemy_readiness() {
    let mut game = Session::with_draws(vec![TokenFace::Zero, TokenFace::Zero]);
    let initial = game.observe();
    for id in ["investigate", "resource", "draw", "injury"] {
        let attack = initial.opportunity_attack(id).unwrap();
        assert_eq!((attack.damage, attack.horror), (1, 1));
        assert_eq!(attack.source, HarmSource::OpportunityAttack);
    }
    for id in ["fight", "evade", "engage", "end_turn", "play:03263", "undo"] {
        assert_eq!(initial.opportunity_attack(id), None, "{id}");
    }
    choose(&mut game, "evade");
    assert_eq!(game.observe().opportunity_attack("investigate"), None);
    choose(&mut game, "commit:none");
    choose(&mut game, "reveal");
    choose(&mut game, "resolve");
    for id in ["investigate", "resource", "draw", "injury"] {
        assert_eq!(game.observe().opportunity_attack(id), None, "{id}");
    }
    choose(&mut game, "engage");
    assert_eq!(game.observe().opportunity_attack("investigate"), None);
    let before = game.clone();
    assert!(game.choose(initial.revision, "investigate").is_err());
    assert_eq!(game, before);
}

#[test]
fn recorded_harm_refuses_split_attacks_missing_effects_and_reordered_sources() {
    use arkham_unfolded_experiment::tony::validate_recording;
    let mut attack = Session::with_draws(vec![]);
    choose(&mut attack, "investigate");
    let mut split = attack.recording();
    split.moments[1].harm = Some(vec![
        Harm {
            source: HarmSource::OpportunityAttack,
            recipient: HarmRecipient::Tony,
            damage: 1,
            horror: 0,
        },
        Harm {
            source: HarmSource::OpportunityAttack,
            recipient: HarmRecipient::Tony,
            damage: 0,
            horror: 1,
        },
    ]);
    assert!(validate_recording(&split).is_err());
    let mut missing = attack.recording();
    missing.moments[1].harm = Some(vec![]);
    assert!(validate_recording(&missing).is_err());

    let mut fight = Session::with_draws(vec![TokenFace::Zero, TokenFace::Tablet]);
    test(&mut fight, "fight");
    test(&mut fight, "fight");
    let mut reordered = fight.recording();
    reordered
        .moments
        .last_mut()
        .unwrap()
        .harm
        .as_mut()
        .unwrap()
        .reverse();
    assert!(validate_recording(&reordered).is_err());
}

// Issue #22's independent witness: Tony/Contracts/Injury original faces,
// Rules Reference pp5,12,24–26 and FAQ2.5 §1.10.
#[test]
fn two_successful_fights_then_injury_use_the_bonus_and_three_ordinary_actions() {
    let mut game = Session::with_draws(vec![TokenFace::Zero, TokenFace::Zero]);
    test(&mut game, "fight");
    assert_eq!(
        (game.observe().ordinary_actions, game.observe().bonus_action),
        (3, false)
    );
    test(&mut game, "fight");
    assert!(!game.observe().ghoul.alive);
    assert_eq!(game.observe().resources, 6);
    choose(&mut game, "injury");
    let view = game.observe();
    assert_eq!(
        (
            view.stage,
            view.ordinary_actions,
            view.injury,
            view.damage,
            view.horror
        ),
        (Stage::Ended, 0, false, 0, 0)
    );
}

#[test]
fn removing_injury_first_provokes_exactly_one_opportunity_attack_for_two_actions() {
    let mut game = Session::with_draws(vec![TokenFace::Zero, TokenFace::Zero]);
    choose(&mut game, "injury");
    assert_eq!(
        (
            game.observe().damage,
            game.observe().horror,
            game.observe().ordinary_actions
        ),
        (1, 1, 1)
    );
    test(&mut game, "fight");
    test(&mut game, "fight");
    assert_eq!(
        (
            game.observe().stage,
            game.observe().damage,
            game.observe().horror,
            game.observe().resources
        ),
        (Stage::Ended, 1, 1, 6)
    );
}

#[test]
fn three_resources_leave_the_bonus_fight_and_early_end_applies_injury_only_once() {
    let mut game = Session::with_draws(vec![]);
    for _ in 0..3 {
        choose(&mut game, "resource");
    }
    let view = game.observe();
    assert_eq!(
        (
            view.stage,
            view.resources,
            view.damage,
            view.horror,
            view.ordinary_actions
        ),
        (Stage::Action, 8, 3, 3, 0)
    );
    assert!(view.choices.iter().any(|c| c.id == "fight"));
    let before = game.clone();
    assert!(game.choose(view.revision, "resource").is_err());
    assert_eq!(game, before);
    choose(&mut game, "end_turn");
    assert_eq!(
        (
            game.observe().stage,
            game.observe().damage,
            game.observe().horror
        ),
        (Stage::Ended, 4, 3)
    );
    let before = game.clone();
    assert!(game.choose(game.observe().revision, "end_turn").is_err());
    assert_eq!(game, before);
}

#[test]
fn evaded_ghoul_can_be_engaged_or_fought_directly_and_stays_exhausted() {
    for engage in [false, true] {
        let mut game = Session::with_draws(vec![TokenFace::Zero, TokenFace::Zero]);
        test(&mut game, "evade");
        assert_eq!(
            (
                game.observe().ghoul.ready,
                game.observe().ghoul.engaged,
                game.observe().damage
            ),
            (false, false, 0)
        );
        assert!(game.observe().choices.iter().any(|c| c.id == "engage"));
        if engage {
            choose(&mut game, "engage");
            assert_eq!(
                (
                    game.observe().bonus_action,
                    game.observe().ghoul.engaged,
                    game.observe().ghoul.ready
                ),
                (false, true, false)
            );
        }
        test(&mut game, "fight");
        assert_eq!(
            (
                game.observe().ghoul.damage,
                game.observe().ghoul.ready,
                game.observe().damage,
                game.observe().horror
            ),
            (1, false, 0, 0)
        );
    }
}

#[test]
fn on_the_hunt_changes_minus_two_investigation_from_failure_to_success() {
    for commit in [false, true] {
        let mut game = Session::with_draws(vec![TokenFace::MinusTwo]);
        choose(&mut game, "investigate");
        assert_eq!((game.observe().damage, game.observe().horror), (1, 1));
        choose(&mut game, if commit { "commit:0" } else { "commit:none" });
        assert_eq!(game.observe().committed.len(), usize::from(commit));
        choose(&mut game, "reveal");
        assert_eq!(
            (game.observe().held_token, game.observe().bag_count),
            (Some(TokenFace::MinusTwo), 15)
        );
        choose(&mut game, "resolve");
        let view = game.observe();
        assert_eq!(view.test.unwrap().success, Some(commit));
        assert_eq!(view.clues, u8::from(commit));
        assert_eq!(
            (
                view.committed.len(),
                view.discard.len(),
                view.bag_count,
                view.held_token
            ),
            (0, usize::from(commit), 16, None)
        );
    }
}

#[test]
fn only_the_three_card_prefix_is_reachable_and_guts_cannot_commit_to_combat() {
    let mut game = Session::with_draws(vec![TokenFace::Zero]);
    assert_eq!(game.observe().deck_count, 32);
    let initial = serde_json::to_string(&game.observe()).unwrap();
    assert!(!initial.contains("Guts"));
    assert!(!initial.contains("random"));
    for (count, title) in [(1, "Guts"), (2, "Guts"), (3, "On the Hunt")] {
        choose(&mut game, "draw");
        assert_eq!(game.observe().hand.last().unwrap().title, title);
        assert_eq!(game.observe().deck_count, 32 - count);
    }
    let before = game.clone();
    assert!(game.choose(game.observe().revision, "draw").is_err());
    assert_eq!(game, before);
    choose(&mut game, "fight");
    let view = game.observe();
    assert!(view.choices.iter().any(|c| c.id == "commit:0,3"));
    let before = game.clone();
    assert!(game.choose(view.revision, "commit:1").is_err());
    assert_eq!(game, before);
    choose(&mut game, "commit:0,3");
    assert_eq!(game.observe().test.unwrap().skill_value, 7);
    choose(&mut game, "reveal");
    choose(&mut game, "resolve");
    assert_eq!(
        game.observe()
            .hand
            .iter()
            .map(|c| c.title.as_str())
            .collect::<Vec<_>>(),
        ["Guts", "Guts"]
    );
}

#[test]
fn ordinary_session_randomness_is_owned_by_the_session_and_hidden_from_observations() {
    let mut game = Session::new().unwrap();
    let mut control = game.clone();
    assert_eq!(game.observe(), Session::new().unwrap().observe());
    for run in [&mut game, &mut control] {
        choose(run, "fight");
        choose(run, "commit:none");
        choose(run, "reveal");
    }
    assert_eq!(game.observe(), control.observe());
    assert_eq!(game.observe().bag_count, 15);
    assert_ne!(game.observe().held_token, Some(TokenFace::Curse));
    assert!(game.observe().held_token.is_some());
}

#[test]
fn every_standard_face_uses_the_original_gathering_and_tony_effects() {
    use TokenFace::*;
    // Source-derived independent outcomes at combat 5 / agility 2 / intellect 3,
    // each against difficulty 2. Ghoul is present at ST.4 for every row here.
    for (face, modifier, outcomes) in [
        (PlusOne, 1, [true, true, true]),
        (Zero, 0, [true, true, true]),
        (MinusOne, -1, [true, false, true]),
        (MinusTwo, -2, [true, false, false]),
        (MinusThree, -3, [true, false, false]),
        (MinusFour, -4, [false, false, false]),
        (Skull, -1, [true, false, true]),
        (Cultist, -1, [true, false, true]),
        (Tablet, -2, [true, false, false]),
        (AutoFail, 0, [false, false, false]),
        (ElderSign, 2, [true, true, true]),
    ] {
        for (index, action) in ["fight", "evade", "investigate"].into_iter().enumerate() {
            let mut game = Session::with_draws(vec![face]);
            test(&mut game, action);
            let view = game.observe();
            let outcome = view.test.unwrap();
            assert_eq!(
                (outcome.modifier, outcome.success),
                (Some(modifier), Some(outcomes[index])),
                "{face:?} {action}"
            );
            let opportunity = u8::from(action == "investigate");
            assert_eq!(
                view.damage,
                opportunity + u8::from(face == Tablet),
                "{face:?} {action}"
            );
            assert_eq!(
                view.horror,
                opportunity + u8::from(face == Cultist && action == "evade"),
                "{face:?} {action}"
            );
            assert_eq!(
                view.contracts_bounties,
                if face == ElderSign { 6 } else { 5 }
            );
            assert_eq!(view.resources, 5);
            assert_eq!((view.bag_count, view.held_token), (16, None));
        }
    }
}

#[test]
fn undo_restores_whole_deterministic_choice_and_keeps_append_only_history() {
    let mut game = Session::with_draws(vec![TokenFace::Zero]);
    let initial = game.observe();
    choose(&mut game, "injury");
    let saved_record = game.recording();
    assert!(game.observe().can_undo);
    choose(&mut game, "undo");
    let view = game.observe();
    assert_eq!(
        (
            view.ordinary_actions,
            view.damage,
            view.horror,
            view.injury,
            view.discard
        ),
        (3, 0, 0, true, vec![])
    );
    assert_eq!(view.hand, initial.hand);
    assert_eq!(view.revision, 2);
    assert!(game.recording().moments.starts_with(&saved_record.moments));
    assert!(view.history.last().unwrap().contains("Undid"));
    choose(&mut game, "fight");
    choose(&mut game, "commit:0");
    choose(&mut game, "undo");
    assert_eq!(
        (
            game.observe().stage,
            game.observe().hand.len(),
            game.observe().committed.len()
        ),
        (Stage::Commit, 1, 0)
    );
    choose(&mut game, "undo");
    assert_eq!(
        (game.observe().stage, game.observe().bonus_action),
        (Stage::Action, true)
    );
}

#[test]
fn committed_revelations_are_undo_barriers_even_before_their_presentation() {
    let mut game = Session::with_draws(vec![TokenFace::Tablet]);
    choose(&mut game, "draw");
    assert!(!game.observe().can_undo);
    let after_draw = game.clone();
    assert!(game.choose(game.observe().revision, "undo").is_err());
    assert_eq!(game, after_draw);
    choose(&mut game, "fight");
    choose(&mut game, "commit:0");
    choose(&mut game, "reveal");
    let revealed = game.observe();
    assert!(!revealed.can_undo);
    let before = game.clone();
    assert!(game.choose(revealed.revision, "undo").is_err());
    assert_eq!(game, before);
    choose(&mut game, "resolve");
    choose(&mut game, "undo");
    let undone = game.observe();
    assert_eq!(
        (
            undone.stage,
            undone.held_token,
            undone.damage,
            undone.committed
        ),
        (
            revealed.stage,
            revealed.held_token,
            revealed.damage,
            revealed.committed
        )
    );
    assert!(!undone.can_undo);
    choose(&mut game, "resolve");
    assert_eq!(game.observe().damage, 2); // one draw opportunity, one tablet
}

#[test]
fn impossible_targets_wrong_timing_costs_and_stale_choices_are_atomic_refusals() {
    let mut game = Session::with_draws(vec![TokenFace::Zero]);
    for id in [
        "engage",
        "fight:patrice",
        "play:03263",
        "move",
        "advance_act",
        "reveal",
    ] {
        let before = game.clone();
        assert!(game.choose(game.observe().revision, id).is_err(), "{id}");
        assert_eq!(game, before);
    }
    choose(&mut game, "resource");
    let before = game.clone();
    assert!(game.choose(0, "resource").is_err());
    assert_eq!(game, before);
    choose(&mut game, "resource");
    let before = game.clone();
    assert!(game.choose(game.observe().revision, "injury").is_err());
    assert_eq!(game, before); // one ordinary + bonus cannot fund Injury
    choose(&mut game, "evade");
    for id in ["end_turn", "commit:0", "resource"] {
        let before = game.clone();
        assert!(game.choose(game.observe().revision, id).is_err(), "{id}");
        assert_eq!(game, before);
    }
}

#[test]
fn symbols_count_exhausted_ghouls_but_not_defeated_ghouls() {
    for face in [TokenFace::Skull, TokenFace::Tablet] {
        let mut exhausted = Session::with_draws(vec![TokenFace::Zero, face]);
        test(&mut exhausted, "evade");
        test(&mut exhausted, "investigate");
        assert_eq!(
            exhausted.observe().test.unwrap().modifier,
            Some(if face == TokenFace::Skull { -1 } else { -2 })
        );
        assert_eq!(
            exhausted.observe().damage,
            u8::from(face == TokenFace::Tablet)
        );

        let mut defeated = Session::with_draws(vec![TokenFace::Zero, TokenFace::Zero, face]);
        test(&mut defeated, "fight");
        test(&mut defeated, "fight");
        test(&mut defeated, "investigate");
        assert_eq!(
            defeated.observe().test.unwrap().modifier,
            Some(if face == TokenFace::Skull { 0 } else { -2 })
        );
        assert_eq!(defeated.observe().damage, 0);
    }
}

#[test]
fn tablet_damages_before_the_second_fight_defeats_the_ghoul() {
    let mut game = Session::with_draws(vec![TokenFace::Zero, TokenFace::Tablet]);
    test(&mut game, "fight");
    test(&mut game, "fight");
    assert_eq!(
        (
            game.observe().ghoul.alive,
            game.observe().damage,
            game.observe().resources
        ),
        (false, 1, 6)
    );
}

#[test]
fn source_bounds_allow_eight_damage_and_contracts_more_than_six_bounties() {
    let mut harmful = Session::with_draws(vec![TokenFace::Tablet; 4]);
    test(&mut harmful, "fight");
    for _ in 0..3 {
        test(&mut harmful, "investigate");
    }
    assert_eq!(
        (
            harmful.observe().stage,
            harmful.observe().damage,
            harmful.observe().horror
        ),
        (Stage::Ended, 8, 3)
    );

    let mut bounties = Session::with_draws(vec![TokenFace::ElderSign; 4]);
    test(&mut bounties, "fight");
    for _ in 0..3 {
        test(&mut bounties, "investigate");
    }
    assert_eq!(
        (
            bounties.observe().contracts_bounties,
            bounties.observe().resources,
            bounties.observe().clues,
            bounties.observe().study_clues
        ),
        (9, 5, 3, 3)
    );
    assert!(bounties.observe().ghoul.alive);
}

#[test]
fn early_end_stops_before_other_turns_and_preserves_companion_state() {
    let mut game = Session::with_draws(vec![]);
    choose(&mut game, "end_turn");
    let view = game.observe();
    assert_eq!(
        (view.ordinary_actions, view.damage, view.horror, view.stage),
        (3, 1, 0, Stage::Ended)
    );
    assert_eq!(view.investigators[1].horror, 0);
    assert_eq!(view.investigators[2].horror, 2);
    assert_eq!(
        (view.jacqueline_used, view.jacqueline_arcane_research),
        (true, 2)
    );
    assert!(view.ghoul.ready && view.ghoul.engaged);
}
