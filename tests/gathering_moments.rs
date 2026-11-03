//! Public consumer checks for the presentation record of actual rules transitions.
//! Fixtures remain privileged; replay expectations come from printed effects,
//! physical custody and the play contract, not constructed replay-only states.

use arkham_unfolded_experiment::{
    TokenFace,
    gathering::{HostileFixture, Resolution, SelectedWitness, Session, TokenFixture, continuity},
};

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
    for _ in 0..8 {
        let view = session.observe_for_play();
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
    let guts = session.observe_for_play().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "01089")
        .unwrap()
        .id;
    choose(&mut session, "revelation:resolve");
    while session
        .observe_for_play()
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
    let before = session.observe_for_play();
    choose(&mut session, "test:reveal");
    let after = session.observe_for_play();
    let final_play = after.play.as_ref().unwrap();
    assert_eq!(final_play.players[0].damage, 2);
    assert_eq!(final_play.bag_count, 16);
    assert!(final_play.test.is_none());
    let moments = session.presentation_after(before.revision);
    assert!(
        !moments.is_empty(),
        "A single automatic decision must retain its transient physical reveal and test result for the real replay consumer"
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
    assert!(
        session
            .recording()
            .presentation_moments
            .iter()
            .any(|moment| moment == &moments[token])
    );
    let dir = tempfile::tempdir().unwrap();
    let mut tampered = session.recording();
    let captured = tampered
        .presentation_moments
        .iter_mut()
        .find(|moment| *moment == &moments[token])
        .unwrap();
    captured.play.players[0].damage += 1;
    assert!(
        continuity::Replay::new(tampered).is_err(),
        "A changed intermediate state is rejected even when every final decision observation remains untouched"
    );
    let path = dir.path().join("automatic-damage.session.json");
    continuity::save_session(&path, &session).unwrap();
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.presentation_after(before.revision), moments);
}

#[test]
fn replay_steps_through_real_transients_without_copying_future_hand_or_history() {
    let mut session = guts_at_reveal(SelectedWitness::Guts);
    let before = session.observe_for_play();
    choose(&mut session, "test:reveal");
    let final_view = session.observe_for_play();
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
    let recording = session.recording();
    let expected_len = recording.moments.len() + recording.presentation_moments.len();
    let mut replay = continuity::Replay::new(recording).unwrap();
    assert_eq!(
        replay.len(),
        expected_len,
        "Intermediate states are actual replay steps, not unused record metadata"
    );
    let mut found = false;
    loop {
        let view = replay.current();
        if view.revision == early.revision
            && view.prompt == early.caption
            && view.play.as_ref() == Some(&early.play)
        {
            found = true;
            assert!(
                view.choices.is_empty(),
                "A presentation step does not reopen a rules decision"
            );
            assert!(
                !view.investigators[0]
                    .hand
                    .iter()
                    .any(|card| card.code == "01049"),
                "The top-level investigator projection cannot leak the final hand"
            );
            assert!(
                !view.history.contains(late_history),
                "The earlier token frame cannot borrow final history about Guts' later draw"
            );
            let snapshot = view.clone();
            assert!(replay.next());
            assert!(replay.back());
            assert_eq!(replay.current(), &snapshot);
        }
        if !replay.next() {
            break;
        }
    }
    assert!(found, "Replay must visit the source-captured token state");
    assert_eq!(replay.current(), &final_view);
}

#[test]
fn a_guts_drawn_weakness_is_visible_while_held_before_automatic_threat_placement() {
    let mut session = guts_at_reveal(SelectedWitness::GutsWeakness);
    let before = session.observe_for_play().revision;
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
    let mut replay = continuity::Replay::new(session.recording()).unwrap();
    let mut saw_held = false;
    let mut saw_placed = false;
    loop {
        if let Some(play) = &replay.current().play {
            if play
                .pending
                .resolving_cards
                .iter()
                .any(|card| card.code == "02038")
                && !play.players[0]
                    .threats
                    .iter()
                    .any(|card| card.code == "02038")
            {
                saw_held = true;
                assert!(!saw_placed);
                assert!(replay.current().choices.is_empty());
            }
            if play.players[0]
                .threats
                .iter()
                .any(|card| card.code == "02038")
            {
                saw_placed = true;
            }
        }
        if !replay.next() {
            break;
        }
    }
    assert!(saw_held && saw_placed);
}

#[test]
fn original_act_advancement_and_ending_are_recorded_when_their_story_state_changes() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    assert!(
        session.recording().presentation_moments.is_empty(),
        "Privileged fixture construction is not a recorded player transition"
    );
    choose(&mut session, "round-end:advance-act");
    for investigator in 0..3 {
        for _ in 0..3 {
            choose(&mut session, &format!("act:contribute:{investigator}"));
        }
    }
    let record = session.recording();
    let advanced = record
        .presentation_moments
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
    assert!(ending.recording().presentation_moments.is_empty());
    choose(&mut ending, "action:ordinary:fight:400");
    reach_reveal(&mut ending);
    choose(&mut ending, "test:reveal");
    assert!(
        ending
            .observe_for_play()
            .choices
            .iter()
            .any(|choice| choice.id == "resolution:burn")
    );
    let before = ending.observe_for_play().revision;
    let prior = ending.recording().presentation_moments;
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
    assert_eq!(
        &ending.recording().presentation_moments[..prior.len()],
        prior.as_slice()
    );
    let final_view = ending.observe_for_play();
    let end_history = final_view
        .history
        .iter()
        .find(|line| line.contains("The Gathering ends:"))
        .expect("Actual terminal narration");
    let mut replay = continuity::Replay::new(ending.recording()).unwrap();
    loop {
        let view = replay.current();
        if view
            .play
            .as_ref()
            .is_none_or(|play| play.scenario.outcome.is_none())
        {
            assert!(
                !view.history.contains(end_history),
                "Before the ending, projected replay history cannot reveal the future resolution"
            );
        }
        if !replay.next() {
            break;
        }
    }
    assert_eq!(replay.current(), &final_view);
}

#[test]
fn a_pending_mists_move_reopens_and_undo_preserves_earlier_physical_snapshots() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::SelectedWitness(SelectedWitness::Mists));
    let play = session.observe_for_play().play.unwrap();
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
    let before_reveal = session.observe_for_play().revision;
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
    let raw_view = session.observe_presentation(&raw);
    let prefix = session.recording().presentation_moments;
    choose(&mut session, "spell-move:decline");
    choose(&mut session, "undo");
    assert_eq!(
        &session.recording().presentation_moments[..prefix.len()],
        prefix.as_slice(),
        "Undo restores the live choice without rewriting chronological presentation evidence"
    );
    let pending = session.observe_for_play();
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
    assert_eq!(reopened.observe_for_play(), pending);
    assert_eq!(
        reopened.recording().presentation_moments,
        session.recording().presentation_moments
    );
    choose(&mut reopened, "spell-move:01114");
    let moved = reopened.observe_for_play().play.unwrap();
    assert_eq!(moved.players[1].location, "01114");
    assert_eq!(moved.players[1].damage, 1);
    assert!(
        moved
            .scenario
            .locations
            .iter()
            .any(|location| location.code == "01114" && location.revealed && location.clues == 6)
    );
    assert_eq!(
        reopened.observe_presentation(&raw),
        raw_view,
        "Later movement, room information, harm and cleanup cannot mutate the earlier token state or projected history"
    );
    assert_eq!(
        &reopened.recording().presentation_moments[..prefix.len()],
        prefix.as_slice()
    );
}

#[test]
fn voice_of_ra_records_its_physical_batch_without_a_skill_test_callback() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::VoiceRaCurse));
    let before = session.observe_for_play();
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
    let after = session.observe_for_play();
    let play = after.play.as_ref().unwrap();
    assert_eq!(play.bag_count, 17);
    assert!(play.test.is_none());
    assert_eq!(
        play.players[2].resources,
        before.play.as_ref().unwrap().players[2].resources + 4,
        "Taboo Voice counts curse and elder sign, without a curse skill-test extra reveal"
    );
    let mut replay = continuity::Replay::new(session.recording()).unwrap();
    let mut found = false;
    loop {
        if replay.current().play.as_ref() == Some(&batch.play)
            && replay.current().prompt == batch.caption
        {
            found = true;
            assert!(replay.current().choices.is_empty());
        }
        if !replay.next() {
            break;
        }
    }
    assert!(found);
    assert_eq!(replay.current(), &after);
}
