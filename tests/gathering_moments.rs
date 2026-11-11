//! Public consumer checks for the presentation record of actual rules transitions.
//! Fixtures remain privileged; live presentation expectations come from printed effects,
//! physical custody and the play contract, not constructed presentation-only states.

use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{HostileFixture, Resolution, SelectedWitness, Session, TokenFixture, continuity},
};

fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "Missing {id}: {:?}",
        view.choices
    );
    session.choose(view.revision, id).unwrap();
}

/// The Barrier's group clue payment, offered as one listed choice, paid with the
/// Hallway's nine clues as the direct choice naming that allocation.
fn pay_hallway(session: &mut Session) {
    let view = session.observe();
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id == "act:pay:<contributions>"),
        "The group clue payment is offered: {:?}",
        view.choices
    );
    session.choose(view.revision, "act:pay:3:3:3").unwrap();
}

fn reach_reveal(session: &mut Session) {
    for _ in 0..8 {
        let view = session.observe();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            return;
        }
        let next = ["window:pass", "commit:confirm"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .expect("An empty test can advance to its reveal");
        choose(session, next);
    }
    panic!("The real test did not reach its reveal");
}

fn guts_at_reveal(case: SelectedWitness) -> Session {
    let mut session = Session::with_hostile_fixture(HostileFixture::SelectedWitness(case));
    let guts = session.observe().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "01089")
        .unwrap()
        .id;
    choose(&mut session, "revelation:resolve");
    while session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, &format!("commit:{guts}"));
    reach_reveal(&mut session);
    session
}

#[test]
fn automatic_grasping_failure_records_the_physical_token_and_result_before_damage_and_cleanup() {
    // Printed Grasping Hands: agility(3), one damage per failed point.
    // Tony agility2 and -1 => total1, two damage, then ST8 returns the token.
    let mut session = Session::with_hostile_fixture(HostileFixture::SelectedWitness(
        SelectedWitness::GraspingHands,
    ));
    choose(&mut session, "revelation:resolve");
    reach_reveal(&mut session);
    let before = session.observe();
    choose(&mut session, "test:reveal");
    let after = session.observe();
    let final_play = after.play.as_ref().unwrap();
    assert_eq!(final_play.players[0].damage, 2);
    assert_eq!(final_play.bag_count, 16);
    assert!(final_play.test.is_none());
    let moments = session.presentation_after(before.revision);
    assert!(
        !moments.is_empty(),
        "A single automatic decision must retain its transient physical reveal and test result for the live presentation consumer"
    );
    let token = moments
        .iter()
        .position(|moment| {
            moment.play.bag_count == 15
                && (moment
                    .play
                    .tokens
                    .as_ref()
                    .is_some_and(|tokens| tokens.revealed == [TokenFace::MinusOne])
                    || moment
                        .play
                        .test
                        .as_ref()
                        .is_some_and(|test| test.revealed == [TokenFace::MinusOne]))
        })
        .expect("The physical -1 must remain inspectable before ST8 returns it");
    let result = moments
        .iter()
        .position(|moment| {
            moment
                .play
                .test
                .as_ref()
                .is_some_and(|test| test.total == Some(1) && test.would_succeed == Some(false))
                && serde_json::to_value(moment.play.window).unwrap() == "results"
                && moment.play.players[0].damage == 0
        })
        .expect(
            "The frozen result is visible at its results boundary before its two damage is applied",
        );
    assert!(token <= result);
    assert!(
        moments
            .iter()
            .all(|moment| moment.revision == after.revision && !moment.caption.trim().is_empty())
    );
    assert!(
        moments
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
    assert!(session.presentation_after(after.revision).is_empty());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("automatic-damage.session.json");
    continuity::save_session(&path, &session).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert!(reopened.presentation_after(0).is_empty());
    assert_eq!(reopened.observe(), session.observe());
}

#[test]
fn live_transients_do_not_copy_future_hand_or_history() {
    let mut session = guts_at_reveal(SelectedWitness::Guts);
    let before = session.observe();
    choose(&mut session, "test:reveal");
    let final_view = session.observe();
    assert!(
        final_view.play.as_ref().unwrap().players[0]
            .hand
            .iter()
            .any(|card| card.code == "01049"),
        "Successful Guts actually draws the next Hard Knocks"
    );
    let early = session
        .presentation_after(before.revision)
        .iter()
        .find(|moment| {
            moment
                .play
                .tokens
                .as_ref()
                .is_some_and(|tokens| tokens.revealed == [TokenFace::Zero])
        })
        .expect("The real revealed zero precedes Guts' draw")
        .clone();
    assert!(
        !early.play.players[0]
            .hand
            .iter()
            .any(|card| card.code == "01049")
    );
    let late_history = final_view
        .history
        .iter()
        .find(|line| line.contains("completes the simultaneous card draw"))
        .expect("The actual later draw has a history entry");

    let view = session.observe_presentation(&early);
    assert!(view.choices.is_empty());
    assert!(
        !view.investigators[0]
            .hand
            .iter()
            .any(|card| card.code == "01049")
    );
    assert!(!view.history.contains(late_history));
    assert_eq!(session.observe(), final_view);
}

#[test]
fn a_guts_drawn_weakness_is_visible_while_held_before_automatic_threat_placement() {
    let mut session = guts_at_reveal(SelectedWitness::GutsWeakness);
    let before = session.observe().revision;
    choose(&mut session, "test:reveal");
    let moments = session.presentation_after(before);
    let drawn = moments.iter().position(|moment| {
        moment.play.pending.resolving_cards.iter().any(|card| card.code == "02038")
            && !moment.play.players[0].threats.iter().any(|card| card.code == "02038")
    }).expect("The physical Internal Injury drawn by Guts is visible before its revelation auto-places it");
    assert!(
        !moments[drawn].play.players[0]
            .hand
            .iter()
            .any(|card| card.code == "02038"),
        "A held encounter-type weakness is not falsely presented as an ordinary hand card"
    );
    let placed = moments
        .iter()
        .position(|moment| {
            moment.play.players[0]
                .threats
                .iter()
                .any(|card| card.code == "02038")
        })
        .expect("Its resolved threat placement is a later recorded state");
    assert!(drawn < placed);
    assert_eq!(moments[drawn].play.players[0].damage, 0);
    assert_eq!(
        moments[placed].play.players[0].damage, 0,
        "Internal Injury's damage waits until the owner's turn ends"
    );

    assert!(
        session
            .observe_presentation(&moments[drawn])
            .choices
            .is_empty()
    );
}

#[test]
fn original_act_advancement_and_ending_are_recorded_when_their_story_state_changes() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    assert!(
        session.presentation_after(0).is_empty(),
        "Privileged fixture construction is not a recorded player transition"
    );
    pay_hallway(&mut session);
    let record = session.presentation_after(0);
    let advanced = record
        .iter()
        .find(|moment| moment.play.act == 3)
        .expect("The paid original Act2 reverse has its own presentation state");
    assert!(advanced.play.scenario.lita_in_play);
    assert!(
        advanced
            .play
            .scenario
            .locations
            .iter()
            .any(|location| location.code == "01115" && location.revealed)
    );
    assert!(
        advanced
            .play
            .scenario
            .enemies
            .iter()
            .any(|enemy| enemy.card.code == "01116" && enemy.location == "01112")
    );
    assert_eq!(
        advanced
            .play
            .players
            .iter()
            .map(|player| player.clues)
            .sum::<u8>(),
        0
    );
    assert!(advanced.play.scenario.outcome.is_none());
    assert!(!advanced.caption.trim().is_empty());

    let mut ending = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    assert!(ending.presentation_after(0).is_empty());
    choose(&mut ending, "action:ordinary:fight:400");
    reach_reveal(&mut ending);
    choose(&mut ending, "test:reveal");
    assert!(
        ending
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "resolution:burn")
    );
    let before = ending.observe().revision;
    let prior = ending.presentation_after(0).to_vec();
    assert!(
        prior
            .iter()
            .all(|moment| moment.play.scenario.outcome.is_none())
    );
    choose(&mut ending, "resolution:burn");
    let ended = ending
        .presentation_after(before)
        .iter()
        .find(|moment| {
            moment
                .play
                .scenario
                .outcome
                .as_ref()
                .is_some_and(|outcome| outcome.resolution == Resolution::BurnHouse)
        })
        .expect("Actual resolution and narration survive as a distinct presentation state");
    assert!(!ended.caption.trim().is_empty());
    assert!(ended.play.scenario.outcome.as_ref().unwrap().house_burned);
    assert_eq!(
        ended
            .play
            .scenario
            .outcome
            .as_ref()
            .unwrap()
            .mental_trauma_gained,
        [1, 0, 0]
    );
    let final_view = ending.observe();
    let end_history = final_view
        .history
        .iter()
        .find(|line| line.contains("The Gathering ends:"))
        .expect("Actual terminal narration");

    for moment in ending.presentation_after(before) {
        let view = ending.observe_presentation(moment);
        if view
            .play
            .as_ref()
            .is_none_or(|play| play.scenario.outcome.is_none())
        {
            assert!(!view.history.contains(end_history));
        }
    }
}

#[test]
fn a_pending_mists_move_reopens_and_undo_restores_the_live_choice() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::SelectedWitness(SelectedWitness::Mists));
    let play = session.observe().play.unwrap();
    let mists = play.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04029")
        .unwrap()
        .card
        .id;
    let enemy = play
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:evade:{enemy}:spell:{mists}"),
    );
    reach_reveal(&mut session);
    let before_reveal = session.observe().revision;
    choose(&mut session, "test:reveal");
    let raw = session
        .presentation_after(before_reveal)
        .iter()
        .find(|moment| {
            moment
                .play
                .tokens
                .as_ref()
                .is_some_and(|tokens| tokens.revealed == [TokenFace::Zero])
        })
        .expect("The physical token precedes Mists' optional movement")
        .clone();
    assert_eq!(raw.play.bag_count, 15);
    choose(&mut session, "spell-move:decline");
    choose(&mut session, "undo");
    let pending = session.observe();
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == "spell-move:01114")
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pending-mists.session.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), pending);
    assert!(reopened.presentation_after(0).is_empty());
    choose(&mut reopened, "spell-move:01114");
    let moved = reopened.observe().play.unwrap();
    assert_eq!(moved.players[1].location, "01114");
    assert_eq!(moved.players[1].damage, 1);
    assert!(
        moved
            .scenario
            .locations
            .iter()
            .any(|location| location.code == "01114" && location.revealed && location.clues == 6)
    );
}

#[test]
fn voice_of_ra_records_its_physical_batch_without_a_skill_test_callback() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::VoiceRaCurse));
    let before = session.observe();
    assert_eq!(before.play.as_ref().unwrap().bag_count, 17);
    assert!(before.play.as_ref().unwrap().test.is_none());
    choose(&mut session, "test:reveal");
    let moments = session.presentation_after(before.revision);
    let batch = moments
        .iter()
        .find(|moment| {
            moment.play.tokens.as_ref().is_some_and(|tokens| {
                tokens.revealed == [TokenFace::Curse, TokenFace::ElderSign, TokenFace::MinusOne]
            })
        })
        .expect(
            "A non-test reveal needs its own physical batch record before it returns all tokens",
        );
    assert!(batch.play.test.is_none());
    assert_eq!(batch.play.bag_count, 14);
    let after = session.observe();
    let play = after.play.as_ref().unwrap();
    assert_eq!(play.bag_count, 17);
    assert!(play.test.is_none());
    assert_eq!(
        play.players[2].resources,
        before.play.as_ref().unwrap().players[2].resources + 4,
        "Taboo Voice counts curse and elder sign, without a curse skill-test extra reveal"
    );

    let view = session.observe_presentation(batch);
    assert!(view.choices.is_empty());
    assert_eq!(view.play.as_ref(), Some(&batch.play));
}
