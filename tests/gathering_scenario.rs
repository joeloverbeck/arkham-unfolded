//! Independent expectations from original 01105–01119/01159–01168,
//! Rules Reference pp9–10,17,19,21–25, Legacy FAQ2.5 §2.2 and CG pp2–3.
//! These privileged witnesses prove the named transitions, not ordinary full play.

use arkham_unfolded_experiment::gathering::{HostileFixture, Investigator, Resolution, Session};

fn choose(session: &mut Session, id: &str) {
    let observation = session.observe();
    assert!(
        observation.choices.iter().any(|choice| choice.id == id),
        "Expected {id}; choices were {:?}",
        observation.choices
    );
    session.choose(observation.revision, id).unwrap();
}

fn pay_hallway_clues(session: &mut Session) {
    choose(session, "round-end:advance-act");
    for investigator in 0..3 {
        for _ in 0..3 {
            choose(session, &format!("act:contribute:{investigator}"));
        }
    }
    choose(session, "bounty:400:0");
}

#[test]
fn act_two_when_precedes_round_end_doom_and_doom_alone_does_not_advance() {
    // S1: one Ghoul at eight doom becomes two Ghouls after the optional Act2
    // payment. The resulting ten doom must wait for an advancement permission.
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    let before = session.observe().play.unwrap();
    assert_eq!((before.act, before.agenda, before.agenda_doom), (2, 3, 8));
    assert_eq!(
        before.players.iter().map(|player| player.clues).sum::<u8>(),
        9
    );
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "round-end:ghoul-doom")
    );

    pay_hallway_clues(&mut session);
    let advanced = session.observe().play.unwrap();
    assert_eq!(advanced.act, 3);
    assert_eq!(advanced.agenda_doom, 8);
    assert!(advanced.scenario.lita_in_play);
    assert!(
        advanced
            .scenario
            .locations
            .iter()
            .find(|location| location.code == "01115")
            .unwrap()
            .revealed
    );
    let priest = advanced
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01116")
        .unwrap();
    assert_eq!(priest.location, "01112");
    assert_eq!(priest.engaged, Some(Investigator::Tony));
    assert_eq!(
        advanced
            .players
            .iter()
            .map(|player| player.clues)
            .sum::<u8>(),
        0
    );

    choose(&mut session, "round-end:ghoul-doom");
    let after = session.observe().play.unwrap();
    assert_eq!((after.act, after.agenda, after.agenda_doom), (3, 3, 10));
    assert!(after.scenario.outcome.is_none());
    assert_eq!(after.round, before.round + 1);
    session.validate().unwrap();
}

#[test]
fn declining_act_two_retains_clues_and_counts_only_the_existing_ghoul() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    choose(&mut session, "round-end:decline-act");
    choose(&mut session, "round-end:ghoul-doom");
    let after = session.observe().play.unwrap();
    assert_eq!((after.act, after.agenda, after.agenda_doom), (2, 3, 9));
    assert_eq!(
        after.players.iter().map(|player| player.clues).sum::<u8>(),
        9
    );
    assert!(!after.scenario.priest_spawned);
    assert!(!after.scenario.lita_in_play);
    assert!(after.scenario.outcome.is_none());
}

#[test]
fn paid_act_two_makes_next_mythos_a_defeat_with_trauma_and_no_resolution() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    pay_hallway_clues(&mut session);
    choose(&mut session, "round-end:ghoul-doom");
    assert_eq!(session.observe().play.unwrap().agenda_doom, 10);
    choose(&mut session, "mythos:begin");
    let after = session.observe();
    let play = after.play.unwrap();
    assert_eq!(play.agenda_doom, 0);
    let record = play.scenario.outcome.unwrap();
    assert_eq!(record.resolution, Resolution::NoResolution);
    assert_eq!(record.physical_trauma_gained, [1; 3]);
    assert!(record.killed.is_empty());
    assert_eq!(record.experience_gained, [3; 3]); // Cleared Attic1 + bonus2.
    assert!(record.ghoul_priest_alive);
    assert!(!record.house_burned);
    assert_eq!(record.lita_recipient, Some(Investigator::Tony));
    assert!(after.choices.is_empty());
    session.validate().unwrap();
}

#[test]
fn declined_act_two_makes_next_mythos_resolution_three_instead() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    choose(&mut session, "round-end:decline-act");
    choose(&mut session, "round-end:ghoul-doom");
    assert_eq!(session.observe().play.unwrap().agenda_doom, 9);
    choose(&mut session, "mythos:begin");
    let record = session.observe().play.unwrap().scenario.outcome.unwrap();
    assert_eq!(record.resolution, Resolution::Overrun);
    assert_eq!(record.killed, Investigator::ALL);
    assert_eq!(record.physical_trauma_gained, [0; 3]);
    assert_eq!(record.experience_gained, [0; 3]);
    assert!(record.lita_recipient_pending);
    assert_eq!(record.lita_recipient, None);
}

#[test]
fn partial_group_payment_reopens_without_charging_twice() {
    use arkham_unfolded_experiment::gathering::continuity::{load_session, save_session};
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    choose(&mut session, "round-end:advance-act");
    for _ in 0..3 {
        choose(&mut session, "act:contribute:0");
    }
    // Contributions are proposed together; nobody pays before the complete,
    // validated group cost. Reopening preserves the proposal and its choices.
    assert_eq!(session.observe().play.unwrap().players[0].clues, 3);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("group-payment.json");
    save_session(&path, &session).unwrap();
    let mut reopened = load_session(&path).unwrap();
    assert_eq!(reopened, session);
    for investigator in 1..3 {
        for _ in 0..3 {
            choose(&mut reopened, &format!("act:contribute:{investigator}"));
        }
    }
    choose(&mut reopened, "bounty:400:0");
    let result = reopened.observe();
    assert!(
        result
            .play
            .as_ref()
            .unwrap()
            .players
            .iter()
            .all(|player| player.clues == 0)
    );
    assert_eq!(
        result
            .play
            .as_ref()
            .unwrap()
            .scenario
            .enemies
            .iter()
            .filter(|enemy| enemy.card.code == "01116")
            .count(),
        1
    );
}

fn defeat_priest_with_basic_fight(session: &mut Session) {
    choose(session, "action:ordinary:fight:400");
    for _ in 0..8 {
        let choices = session.observe().choices;
        if choices.iter().any(|choice| choice.id == "resolution:burn") {
            return;
        }
        let id = [
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| choices.iter().any(|choice| choice.id == *id))
        .expect("A basic fight has a pending test step");
        choose(session, id);
    }
    panic!("Priest defeat did not reach the immediate Act3 ending choice");
}

#[test]
fn priest_terminal_if_precedes_contracts_rewards_and_turn_end_harm() {
    // S4 basic fight deliberately excludes weapon-embedded conditional rewards.
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    let before = session.observe().play.unwrap().players[0].clone();
    defeat_priest_with_basic_fight(&mut session);
    let pending = session.observe();
    let play = pending.play.as_ref().unwrap();
    assert!(
        !play
            .scenario
            .enemies
            .iter()
            .any(|enemy| enemy.card.code == "01116")
    );
    assert!(
        play.scenario
            .victory
            .iter()
            .any(|card| card.code == "01116")
    );
    assert_eq!(play.players[0].resources, before.resources);
    assert_eq!(play.players[0].damage, before.damage);
    // The terminal IF interrupted ST7. ST8 has not returned the revealed token
    // or recorded a completed fight; stopped custody retains that pending test.
    assert_eq!(play.bag_count, 15);
    assert!(
        !play
            .completed_tests
            .iter()
            .any(|test| test.source == "Fight")
    );
    assert_eq!(
        pending
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["resolution:burn", "resolution:refuse"]
    );

    choose(&mut session, "resolution:burn");
    let after = session.observe();
    let play = after.play.unwrap();
    let record = play.scenario.outcome.unwrap();
    assert_eq!(record.resolution, Resolution::BurnHouse);
    assert!(record.house_burned);
    assert!(!record.ghoul_priest_alive);
    assert_eq!(record.experience_gained, [4, 4, 4]);
    assert_eq!(record.mental_trauma_gained, [1, 0, 0]);
    assert_eq!(record.lita_recipient, Some(Investigator::Tony));
    assert_eq!(play.players[0].resources, before.resources);
    assert_eq!(play.players[0].damage, before.damage);
    assert!(after.choices.is_empty());
    session.validate().unwrap();
}

#[test]
fn refusing_to_burn_preserves_house_and_awards_only_lead_extra_experience() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    defeat_priest_with_basic_fight(&mut session);
    choose(&mut session, "resolution:refuse");
    let record = session.observe().play.unwrap().scenario.outcome.unwrap();
    assert_eq!(record.resolution, Resolution::RefuseToBurn);
    assert!(!record.house_burned);
    assert!(!record.ghoul_priest_alive);
    assert_eq!(record.experience_gained, [5, 4, 4]);
    assert_eq!(record.mental_trauma_gained, [0; 3]);
    assert_eq!(record.lita_recipient, None);
}

#[test]
fn ancient_evils_counts_quarry_doom_and_ends_before_remaining_encounter_draws() {
    // S3: eight agenda doom + one Quarry doom + Ancient Evils = ten now.
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringAncientEvilsTerminal);
    let before = session.observe().play.unwrap();
    assert_eq!(before.agenda_doom, 8);
    assert_eq!(before.encounter_deck_count, 25);
    assert_eq!(
        before
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.code == "06012")
            .unwrap()
            .doom,
        1
    );
    choose(&mut session, "revelation:resolve");
    let after = session.observe().play.unwrap();
    assert_eq!(after.agenda_doom, 0);
    assert!(after.scenario.enemies.iter().all(|enemy| enemy.doom == 0));
    assert_eq!(after.encounter_deck_count, 25);
    assert!(after.pending_revelation.is_none());
    let record = after.scenario.outcome.unwrap();
    assert_eq!(record.resolution, Resolution::Overrun);
    assert_eq!(record.killed, Investigator::ALL);
    assert_eq!(record.experience_gained, [0; 3]);
    assert!(record.lita_forced_to_find_others);
    assert!(record.ghoul_priest_alive);
    assert!(record.lita_recipient_pending);
    assert_eq!(record.lita_recipient, None);
    choose(&mut session, "lita:successor");
    assert!(session.observe().choices.is_empty());
    session.validate().unwrap();
}

#[test]
fn agenda_two_no_ghoul_finishes_one_pass_then_rebuilds_only_available_cards() {
    // S8 deliberately uses the documented qualified legacy no-match ruling.
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringAgendaTwoNoGhoul);
    let before = session.observe().play.unwrap();
    assert_eq!(before.encounter_deck_count, 20);
    assert_eq!(before.scenario.enemies.len(), 4);
    assert_eq!(before.scenario.victory.len(), 2);
    choose(&mut session, "agenda:search");
    let after = session.observe().play.unwrap();
    assert_eq!(after.agenda, 3);
    assert_eq!(after.encounter_deck_count, 20);
    assert!(after.encounter_discard.is_empty());
    assert_eq!(after.scenario.enemies, before.scenario.enemies);
    assert_eq!(after.scenario.victory, before.scenario.victory);
    assert!(!after.scenario.priest_spawned);
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "agenda:search")
    );
    session.validate().unwrap();
}

#[test]
fn ordinary_preparation_runs_a_full_round_and_all_following_mythos_draws() {
    let mut session = Session::with_seed(89);
    assert!(!session.is_verification_fixture());
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    let before = session.observe().play.unwrap();
    // Deliberately choose a turn order different from seated player order.
    for investigator in ["tony", "jacqueline", "patrice"] {
        choose(&mut session, &format!("turn:{investigator}"));
        choose(&mut session, "turn:end");
    }
    for _ in 0..48 {
        let observation = session.observe();
        if observation
            .choices
            .iter()
            .any(|choice| choice.id == "mythos:begin")
        {
            break;
        }
        let choice = observation
            .choices
            .iter()
            .find(|choice| {
                matches!(
                    choice.id.as_str(),
                    "enemy:attacks"
                        | "enemy:skip-empty"
                        | "enemy:next-investigator"
                        | "enemy:attacks-complete"
                        | "enemy:end"
                        | "upkeep:ready"
                        | "upkeep:draw"
                        | "upkeep:draws-complete"
                        | "upkeep:patrice-refill"
                        | "upkeep:resources"
                        | "upkeep:next-hand"
                        | "upkeep:hand-limits-complete"
                        | "round:end"
                        | "phase-window:pass"
                        | "revelation:resolve"
                ) || choice.id.starts_with("upkeep:patrice-discard:")
                    || (choice.id.starts_with("bounty:") && choice.id.ends_with(":0"))
            })
            .unwrap_or_else(|| panic!("Unexpected first-round step: {:?}", observation.choices));
        choose(&mut session, &choice.id);
    }
    let upkeep = session.observe().play.unwrap();
    assert_eq!(upkeep.round, 2);
    assert_eq!(upkeep.agenda_doom, 0);
    assert_eq!(
        upkeep.players[0].deck_count,
        before.players[0].deck_count - 1
    );
    assert_eq!(
        upkeep.players[1].deck_count,
        before.players[1].deck_count - 5
    );
    assert_eq!(
        upkeep.players[2].deck_count,
        before.players[2].deck_count - 1
    );
    for card in &before.players[1].hand {
        assert!(
            upkeep.players[1]
                .discard
                .iter()
                .any(|discarded| discarded.id == card.id)
        );
    }
    assert!(upkeep.players.iter().all(|player| player.resources == 6));
    assert_eq!(upkeep.actions_remaining, [3; 3]);
    choose(&mut session, "mythos:begin");
    let mythos = session.observe().play.unwrap();
    assert_eq!(mythos.agenda_doom, 1);
    assert_eq!(mythos.encounter_deck_count, 26);
    assert!(mythos.pending_revelation.is_none());
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "mythos:draw")
    );
    let mut draws = 0;
    for _ in 0..100 {
        let view = session.observe();
        if view.play.as_ref().unwrap().phase
            == arkham_unfolded_experiment::gathering::Phase::Investigation
        {
            break;
        }
        let id = [
            "mythos:draw",
            "mythos:draws-complete",
            "investigation:begin",
            "phase-window:pass",
            "revelation:resolve",
            "test-card:pass",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
            "success:pass",
            "after-test:pass",
            "harm:damage:investigator",
            "harm:horror:investigator",
            "harm:confirm",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .map(str::to_owned)
        .or_else(|| {
            view.choices
                .iter()
                .find(|choice| {
                    choice.id.contains("decline")
                        || choice.id.ends_with(":pass")
                        || choice.id.starts_with("result:")
                        || choice.id.starts_with("discard-card:")
                        || (choice.id.starts_with("bounty:") && choice.id.ends_with(":0"))
                })
                .map(|choice| choice.id.clone())
        })
        .unwrap_or_else(|| panic!("Unexpected mythos choice: {:?}", view.choices));
        draws += usize::from(id == "mythos:draw");
        choose(&mut session, &id);
    }
    let after = session.observe();
    assert_eq!(
        draws, 3,
        "Every investigator takes their next-round encounter draw"
    );
    assert_eq!(
        after.play.as_ref().unwrap().phase,
        arkham_unfolded_experiment::gathering::Phase::Investigation
    );
    assert_eq!(after.play.as_ref().unwrap().round, 2);
    assert_eq!(after.play.as_ref().unwrap().encounter_deck_count, 23);
    assert!(after.play.as_ref().unwrap().pending_revelation.is_none());
    assert!(
        after
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("turn:"))
    );
    session.validate().unwrap();
}

#[test]
fn exhausted_nonhunter_ghoul_moves_at_enemy_end_then_readies_without_another_attack() {
    // S2 distinguishes the agenda's movement from Hunter and from enemy attacks.
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringGhoulMovement);
    let before = session.observe().play.unwrap();
    let minion = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap();
    assert_eq!(minion.location, "01113");
    assert!(minion.exhausted);
    assert_eq!(minion.engaged, None);
    choose(&mut session, "enemy:end");
    let moved = session.observe().play.unwrap();
    let minion = moved
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap();
    assert_eq!(minion.location, "01112");
    assert!(minion.exhausted);
    assert_eq!(minion.engaged, None);
    assert_eq!(moved.players[0].damage, before.players[0].damage);
    assert_eq!(moved.players[0].horror, before.players[0].horror);

    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "phase-window:pass")
    {
        choose(&mut session, "phase-window:pass");
    }
    choose(&mut session, "upkeep:ready");
    let ready = session.observe().play.unwrap();
    let minion = ready
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap();
    assert!(!minion.exhausted);
    assert_eq!(minion.engaged, Some(Investigator::Tony));
    assert_eq!(ready.players[0].damage, before.players[0].damage);
    assert_eq!(ready.players[0].horror, before.players[0].horror);
    assert_eq!(ready.agenda_doom, 0);
    assert_eq!(ready.round, before.round);
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("enemy:attack:"))
    );
    session.validate().unwrap();
}

#[test]
fn lead_orders_voices_and_ghoul_doom_after_the_act_when_opportunity() {
    use arkham_unfolded_experiment::gathering::continuity::{load_session, save_session};
    let mut completed = Vec::new();
    for voices_first in [true, false] {
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEndVoices);
        let before = session.observe();
        let before_play = before.play.unwrap();
        let voices = before_play.players[1]
            .threats
            .iter()
            .find(|card| card.code == "01165")
            .unwrap()
            .id;
        let voices_choice = format!("round-end:voices:{voices}");
        assert!(
            !before
                .choices
                .iter()
                .any(|choice| choice.id == voices_choice || choice.id == "round-end:ghoul-doom")
        );
        choose(&mut session, "round-end:decline-act");
        let at = session.observe();
        assert_eq!(at.decision_owner, Some(Investigator::Tony));
        assert_eq!(
            at.choices
                .iter()
                .filter(|choice| choice.id != "undo")
                .count(),
            2
        );
        let (first, second) = if voices_first {
            (voices_choice.as_str(), "round-end:ghoul-doom")
        } else {
            ("round-end:ghoul-doom", voices_choice.as_str())
        };
        choose(&mut session, first);
        let between = session.observe();
        assert_eq!(between.play.as_ref().unwrap().round, before_play.round);
        assert_eq!(
            between
                .choices
                .iter()
                .filter(|choice| choice.id != "undo")
                .map(|choice| choice.id.as_str())
                .collect::<Vec<_>>(),
            [second]
        );
        assert_eq!(
            between.play.as_ref().unwrap().agenda_doom,
            if voices_first { 8 } else { 9 }
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("remaining-forced-effect.json");
        save_session(&path, &session).unwrap();
        let mut reopened = load_session(&path).unwrap();
        assert_eq!(reopened, session);
        choose(&mut reopened, second);
        let after = reopened.observe().play.unwrap();
        assert_eq!(after.agenda_doom, 9);
        assert_eq!(after.round, before_play.round + 1);
        assert!(
            !after.players[1]
                .threats
                .iter()
                .any(|card| card.code == "01165")
        );
        assert!(after.encounter_discard.iter().any(|card| card.id == voices));
        completed.push(after);
    }
    assert_eq!(completed[0], completed[1]);
}

fn reach_choice(session: &mut Session, expected: &str) {
    for _ in 0..10 {
        let choices = session.observe().choices;
        if choices.iter().any(|choice| choice.id == expected) {
            return;
        }
        let id = [
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| choices.iter().any(|choice| choice.id == *id))
        .unwrap_or_else(|| panic!("Expected progression toward {expected}, got {choices:?}"));
        choose(session, id);
    }
    panic!("Did not reach {expected}");
}

#[test]
fn lita_controller_chooses_extra_monster_damage_before_the_fight_result() {
    for use_lita in [false, true] {
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringLitaDamage);
        choose(&mut session, "action:ordinary:fight:400");
        reach_choice(&mut session, "lita:decline-damage");
        let when = session.observe();
        assert_eq!(when.decision_owner, Some(Investigator::Patrice));
        let play = when.play.unwrap();
        assert_eq!(
            play.scenario
                .enemies
                .iter()
                .find(|enemy| enemy.card.id == 400)
                .unwrap()
                .damage,
            13
        );
        assert_eq!(play.test.as_ref().unwrap().total, Some(5)); // Tony5 + Lita1 − token1.
        assert_eq!(play.bag_count, 15);
        choose(
            &mut session,
            if use_lita {
                "lita:add-damage"
            } else {
                "lita:decline-damage"
            },
        );
        let after = session.observe();
        let play = after.play.unwrap();
        if use_lita {
            assert!(
                !play
                    .scenario
                    .enemies
                    .iter()
                    .any(|enemy| enemy.card.id == 400)
            );
            assert!(
                after
                    .choices
                    .iter()
                    .any(|choice| choice.id == "resolution:burn")
            );
            assert_eq!(play.bag_count, 15); // Terminal IF interrupts ST7, no cleanup.
        } else {
            assert_eq!(
                play.scenario
                    .enemies
                    .iter()
                    .find(|enemy| enemy.card.id == 400)
                    .unwrap()
                    .damage,
                14
            );
            assert!(
                !after
                    .choices
                    .iter()
                    .any(|choice| choice.id == "resolution:burn")
            );
            assert_eq!(play.bag_count, 16);
            assert!(play.test.is_none());
        }
        session.validate().unwrap();
    }
}

#[test]
fn failed_tie_retaliates_before_test_cleanup_and_keeps_the_priest_ready() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRetaliate);
    let before = session.observe().play.unwrap();
    choose(&mut session, "action:ordinary:fight:400");
    reach_choice(&mut session, "retaliate:complete");
    let pending = session.observe().play.unwrap();
    assert_eq!(pending.test.as_ref().unwrap().failure_margin, Some(0));
    assert_eq!(pending.test.as_ref().unwrap().would_succeed, Some(false));
    assert_eq!(pending.bag_count, 15);
    // Tony has no eligible soak asset, so the forced assignment resolves
    // without introducing a choice between nonexistent alternatives.
    let attacked = session.observe().play.unwrap();
    assert_eq!(attacked.players[0].damage, before.players[0].damage + 2);
    assert_eq!(attacked.players[0].horror, before.players[0].horror + 2);
    assert_eq!(attacked.bag_count, 15);
    assert!(
        !attacked
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == 400)
            .unwrap()
            .exhausted
    );
    choose(&mut session, "retaliate:complete");
    let finished = session.observe().play.unwrap();
    assert!(finished.test.is_none());
    assert_eq!(finished.bag_count, 16);
    assert_eq!(finished.completed_tests.last().unwrap().failure_margin, 0);
    let priest = finished
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == 400)
        .unwrap();
    assert!(!priest.exhausted);
    assert_eq!(priest.damage, 14);
    session.validate().unwrap();
}

#[test]
fn successful_lita_parley_resolves_the_actual_ally_slot_before_test_cleanup() {
    use arkham_unfolded_experiment::gathering::continuity::{load_session, save_session};
    {
        let keep_lita = true;
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringLitaParley);
        let before = session.observe().play.unwrap();
        let patrice = &before.players[Investigator::Patrice.index()];
        let peter = patrice
            .assets
            .iter()
            .find(|asset| asset.card.code == "02033")
            .unwrap()
            .card
            .id;
        let rise = patrice
            .hand
            .iter()
            .find(|card| card.code == "02192")
            .unwrap()
            .id;
        choose(&mut session, "action:ordinary:parley:lita");
        let commit = format!("commit:{rise}");
        reach_choice(&mut session, &commit);
        choose(&mut session, &commit);
        let discard = format!("lita:discard-ally:{}", if keep_lita { peter } else { 401 });
        reach_choice(&mut session, &discard);
        let pending = session.observe();
        assert_eq!(pending.decision_owner, Some(Investigator::Patrice));
        assert!(
            !pending
                .choices
                .iter()
                .any(|choice| choice.id == "lita:discard-ally:401"),
            "The incoming Lita cannot be discarded to make room for herself"
        );
        let play = pending.play.unwrap();
        assert_eq!(play.bag_count, 15);
        assert_eq!(play.test.as_ref().unwrap().would_succeed, Some(true));
        assert!(
            play.players[Investigator::Patrice.index()]
                .assets
                .iter()
                .any(|asset| asset.card.code == "01117")
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lita-ally-slot.json");
        save_session(&path, &session).unwrap();
        session = load_session(&path).unwrap();
        choose(&mut session, &discard);
        let finished = session.observe().play.unwrap();
        assert!(finished.test.is_none());
        assert_eq!(finished.bag_count, 16);
        let patrice = &finished.players[Investigator::Patrice.index()];
        assert_eq!(
            patrice
                .assets
                .iter()
                .any(|asset| asset.card.code == "01117"),
            keep_lita
        );
        assert_eq!(
            patrice.assets.iter().any(|asset| asset.card.id == peter),
            !keep_lita
        );
        assert_eq!(
            patrice.discard.iter().any(|card| card.id == peter),
            keep_lita
        );
        assert!(!patrice.discard.iter().any(|card| card.code == "01117"));
        assert_eq!(
            finished
                .scenario
                .removed
                .iter()
                .any(|card| card.code == "01117"),
            !keep_lita
        );
        assert_eq!(finished.scenario.lita_in_play, keep_lita);
        session.validate().unwrap();
    }
}

#[test]
fn nonterminal_defeat_pays_forced_bounties_before_optional_printed_horror_healing() {
    for play_bleeds in [false, true] {
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringNonterminalDefeat);
        let before = session.observe().play.unwrap();
        let minion = before
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.code == "01160")
            .unwrap()
            .card
            .id;
        let event = before.players[0]
            .hand
            .iter()
            .find(|card| card.code == "02225")
            .unwrap()
            .id;
        choose(&mut session, &format!("action:ordinary:fight:{minion}"));
        reach_choice(&mut session, "defeat:done");
        let after_defeat = session.observe().play.unwrap();
        assert_eq!(
            after_defeat.players[0].resources,
            before.players[0].resources + 2
        );
        assert_eq!(after_defeat.bag_count, 15);
        assert!(
            after_defeat
                .encounter_discard
                .iter()
                .any(|card| card.id == minion)
        );
        assert!(
            !after_defeat
                .scenario
                .enemies
                .iter()
                .any(|enemy| enemy.card.id == minion)
        );
        assert_eq!(
            after_defeat.players[0]
                .assets
                .iter()
                .find(|asset| asset.card.code == "06010")
                .unwrap()
                .uses,
            4
        );
        if play_bleeds {
            choose(&mut session, &format!("defeat:bleeds:{event}"));
            let healed = session.observe().play.unwrap();
            assert_eq!(healed.players[0].resources, before.players[0].resources + 1);
            assert_eq!(healed.players[0].horror, 1);
            assert_eq!(healed.players[1].horror, 2);
            assert_eq!(healed.players[2].horror, 2);
            assert!(
                healed.players[0]
                    .discard
                    .iter()
                    .any(|card| card.id == event)
            );
        }
        choose(&mut session, "defeat:done");
        let finished = session.observe().play.unwrap();
        assert_eq!(finished.bag_count, 16);
        assert!(finished.test.is_none());
        if !play_bleeds {
            assert_eq!(finished.players[0].horror, 2);
            assert_eq!(finished.players[1].horror, 3);
            assert!(finished.players[0].hand.iter().any(|card| card.id == event));
        }
        session.validate().unwrap();
    }
}

#[test]
fn hunters_use_nearest_prey_and_attacks_exhaust_after_their_actual_harm() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringHunters);
    let before = session.observe().play.unwrap();
    let rats = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01159" && !enemy.exhausted)
        .unwrap()
        .card
        .id;
    let exhausted = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01159" && enemy.exhausted)
        .unwrap()
        .card
        .id;
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == format!("enemy:hunter:{exhausted}"))
    );
    choose(&mut session, "enemy:hunter:400");
    let priest_moved = session.observe().play.unwrap();
    let priest = priest_moved
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == 400)
        .unwrap();
    assert_eq!(priest.location, "01115");
    assert_eq!(priest.engaged, Some(Investigator::Tony));
    assert!(!priest.exhausted);
    choose(&mut session, &format!("enemy:hunter:{rats}"));
    let choices = session.observe().choices;
    for location in ["01113", "01114", "01115"] {
        assert!(
            choices
                .iter()
                .any(|choice| choice.id == format!("hunter:move:{rats}:{location}"))
        );
    }
    choose(&mut session, &format!("hunter:move:{rats}:01115"));
    choose(&mut session, "enemy:attacks");
    let mut windows = 0;
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "phase-window:pass")
    {
        choose(&mut session, "phase-window:pass");
        windows += 1;
    }
    assert_eq!(session.observe().decision_owner, Some(Investigator::Tony));
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "enemy:attack:400")
    );
    choose(&mut session, &format!("enemy:attack:{rats}"));
    let hurt = session.observe().play.unwrap();
    assert_eq!(hurt.players[0].damage, before.players[0].damage + 1);
    assert!(
        !hurt
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == rats)
            .unwrap()
            .exhausted
    );
    choose(&mut session, "enemy:attack-complete");
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == format!("enemy:attack:{rats}"))
    );
    choose(&mut session, "enemy:attack:400");
    let hurt = session.observe().play.unwrap();
    assert_eq!(hurt.players[0].damage, before.players[0].damage + 3);
    assert_eq!(hurt.players[0].horror, before.players[0].horror + 2);
    choose(&mut session, "enemy:attack-complete");
    for _ in 0..12 {
        let choices = session.observe().choices;
        if choices.iter().any(|choice| choice.id == "enemy:end") {
            break;
        }
        let id = [
            "phase-window:pass",
            "enemy:next-investigator",
            "enemy:attacks-complete",
        ]
        .into_iter()
        .find(|id| choices.iter().any(|choice| choice.id == *id))
        .unwrap();
        if id == "phase-window:pass" {
            windows += 1;
        }
        choose(&mut session, id);
    }
    assert_eq!(
        windows, 4,
        "One window before each investigator's attacks and one after the attack step"
    );
    let ended = session.observe().play.unwrap();
    for id in [400, rats, exhausted] {
        assert!(
            ended
                .scenario
                .enemies
                .iter()
                .find(|enemy| enemy.card.id == id)
                .unwrap()
                .exhausted
        );
    }
    choose(&mut session, "enemy:end");
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "phase-window:pass")
    {
        choose(&mut session, "phase-window:pass");
    }
    choose(&mut session, "upkeep:ready");
    let readied = session.observe().play.unwrap();
    assert_eq!(readied.players[0].damage, before.players[0].damage + 3);
    assert_eq!(readied.players[0].horror, before.players[0].horror + 2);
    assert!(
        readied
            .scenario
            .enemies
            .iter()
            .all(|enemy| !enemy.exhausted)
    );
    assert_eq!(
        readied
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == exhausted)
            .unwrap()
            .location,
        "01112"
    );
    session.validate().unwrap();
}
