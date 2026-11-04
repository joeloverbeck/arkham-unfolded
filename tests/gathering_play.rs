use arkham_unfolded_experiment::gathering::{HostileFixture, Session};

fn choose(session: &mut Session, id: &str) {
    session.choose(session.observe().revision, id).unwrap();
}

#[test]
fn committed_cards_wait_for_saved_owner_order_before_test_cleanup() {
    use arkham_unfolded_experiment::gathering::continuity;
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    let cancel = session
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap()
        .id;
    choose(&mut session, &cancel);
    choose(&mut session, "window:pass");
    let hand = session.observe().play.unwrap().players[1].hand.clone();
    let heart = hand
        .iter()
        .find(|card| card.code == "04201")
        .unwrap()
        .clone();
    let courage = hand
        .iter()
        .find(|card| card.code == "01093")
        .unwrap()
        .clone();
    choose(&mut session, &format!("commit:{}", heart.id));
    choose(&mut session, &format!("commit:{}", courage.id));
    choose(&mut session, "commit:confirm");
    choose(&mut session, "window:pass");
    choose(&mut session, "test:reveal");
    let pending = session.observe();
    let play = pending.play.as_ref().unwrap();
    assert_eq!(
        play.bag_count, 15,
        "ST8 token cleanup waits for committed-card discard order"
    );
    assert!(play.players[1].discard.is_empty());
    assert!(play.pending_revelation.as_ref().unwrap().cancelled);
    assert!(
        !play.players[1]
            .hand
            .iter()
            .any(|card| card.id == heart.id || card.id == courage.id)
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == format!("discard-card:{}", heart.id))
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("commit-order.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), pending);
    choose(&mut reopened, &format!("discard-card:{}", courage.id));
    let after = reopened.observe().play.unwrap();
    assert_eq!(
        &after.players[1].discard[..2],
        &[courage.clone(), heart.clone()]
    );
    assert_eq!(
        after.players[1].discard[2].code, "60513",
        "The event leaves limbo after its test cleanup"
    );
    assert_eq!(after.bag_count, 16);
    assert!(after.pending_revelation.is_none());
    assert_eq!(after.encounter_discard[0].code, "01166");
    choose(&mut session, &format!("discard-card:{}", heart.id));
    assert_eq!(
        &session.observe().play.unwrap().players[1].discard[..2],
        &[heart, courage]
    );
    reopened.validate().unwrap();
    reopened.validate().unwrap();
}

#[test]
fn simultaneous_asset_defeat_waits_for_saved_discard_order_before_the_action_continues() {
    use arkham_unfolded_experiment::gathering::continuity;
    let mut session = Session::with_hostile_fixture(HostileFixture::SimultaneousAssetDefeat);
    let before = session.observe().play.unwrap();
    let olive = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .clone();
    let robes = before.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .clone();
    choose(&mut session, "action:ordinary:resource");
    choose(&mut session, "opportunity:400");
    choose(&mut session, &format!("harm:damage:{}", olive.id));
    choose(&mut session, &format!("harm:damage:{}", robes.id));
    choose(&mut session, "harm:horror:investigator");
    choose(&mut session, "harm:horror:investigator");
    let pending = session.observe();
    let jacqueline = &pending.play.as_ref().unwrap().players[2];
    assert_eq!(
        jacqueline.resources, 5,
        "The interrupted resource action must wait for discard order"
    );
    assert_eq!(jacqueline.horror, before.players[2].horror + 2);
    assert!(
        !jacqueline
            .assets
            .iter()
            .any(|asset| asset.card.id == olive.id || asset.card.id == robes.id),
        "Both assets leave play simultaneously"
    );
    assert!(
        !jacqueline
            .discard
            .iter()
            .any(|card| card.id == olive.id || card.id == robes.id)
    );
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == format!("discard-card:{}", robes.id))
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("asset-order.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), pending);
    choose(&mut reopened, &format!("discard-card:{}", robes.id));
    let after = reopened.observe().play.unwrap();
    assert_eq!(after.players[2].discard, [robes.clone(), olive.clone()]);
    assert_eq!(after.players[2].resources, 6);
    assert_eq!(
        after.players[2].horror, jacqueline.horror,
        "Resuming must not repeat the attack"
    );
    assert_eq!(after.actions_remaining[2], 2);
    assert!(!after.scenario.enemies[0].exhausted);
    choose(&mut session, &format!("discard-card:{}", olive.id));
    assert_eq!(
        session.observe().play.unwrap().players[2].discard,
        [olive, robes]
    );
    reopened.validate().unwrap();
    reopened.validate().unwrap();
}

#[test]
fn actual_preparation_reaches_first_investigation_without_a_first_mythos() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    choose(&mut session, "lead:patrice");
    let hands = session
        .observe()
        .investigators
        .iter()
        .map(|player| player.hand.clone())
        .collect::<Vec<_>>();
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    let view = session.observe();
    let play = view
        .play
        .expect("Actual preparation must create the shared runtime");
    assert_eq!(play.agenda_doom, 0);
    assert!(play.pending_revelation.is_none());
    for (player, hand) in play.players.iter().zip(hands) {
        assert_eq!(player.hand, hand);
        assert_eq!(player.resources, 5);
        assert_eq!(player.location, "01111");
    }
    choose(&mut session, "turn:tony");
    choose(&mut session, "action:ordinary:resource");
    assert_eq!(session.observe().play.unwrap().players[0].resources, 6);
    session.validate().unwrap();
}

#[test]
fn a_real_study_investigation_spends_one_action_and_uses_the_printed_shroud() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    choose(&mut session, "lead:tony");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    choose(&mut session, "turn:tony");
    choose(&mut session, "action:ordinary:investigate");
    let pending = session.observe().play.unwrap();
    let test = pending.test.unwrap();
    assert_eq!(test.base_skill, 3, "Tony printed intellect");
    assert_eq!(test.difficulty, 2, "Original revealed Study shroud");
    assert_eq!(pending.actions_remaining[0], 2);
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, "commit:confirm");
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, "test:reveal");
    let after = session.observe().play.unwrap();
    let result = after.completed_tests.last().unwrap();
    assert_eq!(result.source, "Investigate");
    assert_eq!(after.players[0].clues, u8::from(result.success));
    assert_eq!(
        after
            .scenario
            .locations
            .iter()
            .find(|location| location.code == "01111")
            .unwrap()
            .clues,
        6 - u8::from(result.success)
    );
    assert_eq!(after.bag_count, 16);
    assert_eq!(after.actions_remaining[0], 2);
    assert_eq!(
        after.actor,
        Some(arkham_unfolded_experiment::gathering::Investigator::Tony)
    );
    session.validate().unwrap();
}

#[test]
fn a_two_action_weakness_removal_pays_once_and_provokes_one_attack_before_its_effect() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    let initial = session.observe().play.unwrap();
    let injury = initial.players[0]
        .threats
        .iter()
        .find(|card| card.code == "02038")
        .unwrap()
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:remove-threat:tony:{injury}"),
    );
    let pending = session.observe().play.unwrap();
    assert_eq!(
        pending.actions_remaining[0], 1,
        "Two actions are one activation cost"
    );
    assert!(
        pending.players[0]
            .threats
            .iter()
            .any(|card| card.id == injury),
        "Effect follows the attack of opportunity"
    );
    choose(&mut session, "opportunity:400");
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[0].damage, initial.players[0].damage + 2);
    assert_eq!(after.players[0].horror, initial.players[0].horror + 2);
    assert!(
        after.players[0]
            .discard
            .iter()
            .any(|card| card.id == injury)
    );
    assert!(
        !after.scenario.enemies[0].exhausted,
        "Opportunity attacks do not exhaust enemies"
    );
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("opportunity:"))
    );
    session.validate().unwrap();
}

#[test]
fn tony_can_choose_his_bounty_fight_action_but_cannot_spend_it_to_evade() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    let view = session.observe();
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id == "action:tony:fight:400")
    );
    assert!(
        !view
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("action:tony:evade"))
    );
    choose(&mut session, "action:tony:fight:400");
    let pending = session.observe().play.unwrap();
    assert_eq!(pending.actions_remaining[0], 3);
    assert_eq!(pending.test.unwrap().difficulty, 4);
    session.validate().unwrap();
}

#[test]
fn a_non_test_token_reveal_belongs_to_its_revealer_instead_of_the_lead() {
    use arkham_unfolded_experiment::gathering::{Investigator, TokenFixture};
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::NestedProphecyCleanup));
    let view = session.observe();
    assert!(view.play.unwrap().test.is_none());
    assert_eq!(view.lead, Some(Investigator::Patrice));
    assert_eq!(view.decision_owner, Some(Investigator::Jacqueline));
    choose(&mut session, "tokens:jacqueline");
    assert_eq!(
        session.observe().decision_owner,
        Some(Investigator::Jacqueline)
    );
    session.validate().unwrap();
}

#[test]
fn cancellation_test_preserves_the_original_pending_encounter() {
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    let before = session.observe();
    assert!(before.verification_fixture.is_some());
    let play = before.play.as_ref().unwrap();
    assert_eq!(play.pending_revelation.as_ref().unwrap().card.code, "01166");
    assert_eq!(play.agenda_doom, 0);
    let cancel = before
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap();
    choose(&mut session, &cancel.id);
    let during = session.observe();
    let play = during.play.as_ref().unwrap();
    assert_eq!(play.players[1].resources, 2);
    assert_eq!(play.pending_revelation.as_ref().unwrap().card.code, "01166");
    assert!(!play.pending_revelation.as_ref().unwrap().started);
    assert_eq!(play.agenda_doom, 0);
    let test = play.test.as_ref().unwrap();
    assert_eq!(test.base_skill, 4);
    assert_eq!(test.difficulty, 3);
    session.validate().unwrap();
}

fn reach_vision_failure(session: &mut Session) {
    let view = session.observe();
    let cancel = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap()
        .id
        .clone();
    choose(session, &cancel);
    let view = session.observe();
    let patrice = &view.play.as_ref().unwrap().players[1];
    let violin = patrice
        .assets
        .iter()
        .find(|asset| asset.card.code == "06016")
        .unwrap()
        .card
        .id;
    let discarded = patrice
        .hand
        .iter()
        .find(|card| card.code == "01093")
        .unwrap()
        .id;
    choose(
        session,
        &format!("violin:{violin}:discard:{discarded}:target:patrice:draw"),
    );
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    assert!(
        play.players[1]
            .threats
            .iter()
            .any(|card| card.code == "05041")
    );
    assert!(
        !play.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.code == "06016")
            .unwrap()
            .ready
    );
    assert_eq!(play.agenda_doom, 0);
    assert!(!play.pending_revelation.as_ref().unwrap().started);
    let rise = play.players[1]
        .hand
        .iter()
        .find(|card| card.code == "02192")
        .unwrap()
        .id;
    assert!(
        !view
            .choices
            .iter()
            .any(|choice| choice.id == format!("commit:{rise}"))
    );
    let heart = play.players[1]
        .hand
        .iter()
        .find(|card| card.code == "04201")
        .unwrap()
        .id;
    choose(session, &format!("commit:{heart}"));
    choose(session, "commit:confirm");
    choose(session, "test:reveal");
    let view = session.observe();
    let test = view.play.as_ref().unwrap().test.as_ref().unwrap();
    assert_eq!(test.total, Some(3));
    assert_eq!(test.would_succeed, Some(false));
    assert_eq!(test.failure_margin, Some(0));
}

#[test]
fn violin_drawn_vision_changes_the_running_test_and_lucky_cancels_without_failure_rewards() {
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    reach_vision_failure(&mut session);
    let view = session.observe();
    let lucky = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("lucky:"))
        .unwrap()
        .id
        .clone();
    choose(&mut session, &lucky);
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    assert_eq!(play.players[1].resources, 1);
    assert_eq!(play.players[1].hand.len(), 1);
    assert_eq!(play.agenda_doom, 0);
    assert!(play.pending_revelation.is_none());
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.code == "01166")
    );
    assert!(
        play.players[1]
            .threats
            .iter()
            .any(|card| card.code == "05041")
    );
    assert_eq!(play.completed_tests.len(), 1);
    assert!(play.completed_tests[0].success);
    assert_eq!(play.completed_tests[0].total, 5);
    session.validate().unwrap();
}

#[test]
fn declining_lucky_rewards_the_failed_test_before_resuming_ancient_evils() {
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    reach_vision_failure(&mut session);
    assert_eq!(session.observe().play.as_ref().unwrap().bag_count, 15);
    choose(&mut session, "lucky:decline");
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    assert_eq!(play.players[1].resources, 4);
    assert_eq!(play.players[1].hand.len(), 4);
    assert_eq!(play.players[1].horror, 0);
    assert_eq!(play.agenda_doom, 1);
    assert!(play.test.is_none());
    assert!(play.pending_revelation.is_none());
    assert_eq!(play.completed_tests.len(), 1);
    assert!(!play.completed_tests[0].success);
    assert_eq!(play.completed_tests[0].failure_margin, 0);
    assert_eq!(play.bag_count, 16);
    assert_eq!(
        play.encounter_discard
            .iter()
            .filter(|card| card.code == "01166")
            .count(),
        1
    );
    session.validate().unwrap();
}

#[test]
fn the_original_rotting_remains_starts_its_own_test_after_failed_cancellation_cleanup() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::PatriceCancellationRottingRemains);
    reach_vision_failure(&mut session);
    choose(&mut session, "lucky:decline");
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    assert_eq!(play.completed_tests.len(), 1);
    assert_eq!(play.players[1].resources, 4);
    assert_eq!(play.players[1].horror, 0);
    assert_eq!(play.pending_revelation.as_ref().unwrap().card.code, "01163");
    assert!(play.pending_revelation.as_ref().unwrap().started);
    assert!(play.test.as_ref().unwrap().revealed.is_empty());
    assert!(play.test.as_ref().unwrap().committed.is_empty());
    assert!(
        !view
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("test-of-will:"))
    );
    if view
        .choices
        .iter()
        .any(|choice| choice.id == "commit:confirm")
    {
        choose(&mut session, "commit:confirm");
    }
    choose(&mut session, "test:reveal");
    assert_eq!(
        session
            .observe()
            .play
            .as_ref()
            .unwrap()
            .test
            .as_ref()
            .unwrap()
            .total,
        Some(2)
    );
    choose(&mut session, "lucky:decline");
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    assert_eq!(play.completed_tests.len(), 2);
    assert_eq!(play.completed_tests[1].source, "Rotting Remains");
    assert_eq!(play.players[1].horror, 1);
    assert_eq!(play.players[1].resources, 4);
    assert!(play.test.is_none());
    assert_eq!(
        play.encounter_discard
            .iter()
            .filter(|card| card.code == "01163")
            .count(),
        1
    );
    session.validate().unwrap();
}

#[test]
fn declining_violin_keeps_an_ordinary_tie_successful_and_preserves_both_player_windows() {
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    let cancel = session
        .observe()
        .choices
        .into_iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap()
        .id;
    choose(&mut session, &cancel);
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("violin:"))
    );
    choose(&mut session, "window:pass");
    let view = session.observe();
    let heart = view.play.as_ref().unwrap().players[1]
        .hand
        .iter()
        .find(|card| card.code == "04201")
        .unwrap()
        .id;
    choose(&mut session, &format!("commit:{heart}"));
    choose(&mut session, "commit:confirm");
    assert!(
        session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("violin:"))
    );
    choose(&mut session, "window:pass");
    choose(&mut session, "test:reveal");
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    assert!(play.completed_tests[0].success);
    assert_eq!(play.completed_tests[0].total, 3);
    assert_eq!(play.players[1].resources, 2);
    assert!(play.players[1].threats.is_empty());
    assert!(
        play.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.code == "06016")
            .unwrap()
            .ready
    );
    assert_eq!(play.agenda_doom, 0);
}

#[test]
fn pending_lucky_reopens_without_repeating_costs_draws_or_doom() {
    use arkham_unfolded_experiment::gathering::continuity::{
        commit_choice, load_session, save_session,
    };
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("p1.session.json");
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    reach_vision_failure(&mut session);
    save_session(&path, &session).unwrap();
    let pending = session.clone();
    let mut resumed = load_session(&path).unwrap();
    assert_eq!(resumed, pending);
    assert!(resumed.is_verification_fixture());
    let unusable = directory.path().join("not-a-file");
    std::fs::create_dir(&unusable).unwrap();
    let revision = resumed.observe().revision;
    assert!(commit_choice(&unusable, &mut resumed, revision, "lucky:decline").is_err());
    assert_eq!(resumed, pending);
    commit_choice(&path, &mut resumed, revision, "lucky:decline").unwrap();
    assert_eq!(load_session(&path).unwrap(), resumed);
    let result = resumed.observe();
    let play = result.play.as_ref().unwrap();
    assert_eq!(play.players[1].resources, 4);
    assert_eq!(play.agenda_doom, 1);

    assert_eq!(load_session(&path).unwrap().observe(), result);
    for moment in resumed.presentation_after(pending.observe().revision) {
        assert!(resumed.observe_presentation(moment).choices.is_empty());
    }
}
