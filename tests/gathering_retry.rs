use arkham_unfolded_experiment::gathering::{HostileFixture, RetryFixture, Session, continuity};
fn choose(session: &mut Session, id: &str) {
    let view = session.observe();
    session.choose(view.revision, id).unwrap_or_else(|error| {
        panic!(
            "{id}: {error}; choices {:?}",
            view.choices
                .iter()
                .map(|choice| &choice.id)
                .collect::<Vec<_>>()
        )
    });
}
fn reveal(session: &mut Session) {
    pass_window(session);
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "commit:confirm")
    {
        choose(session, "commit:confirm");
    }
    pass_window(session);
    choose(session, "test:reveal");
}

#[test]
fn charged_spell_retry_keeps_the_operation_without_paying_a_second_charge_or_action() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Retry(RetryFixture::ChargedFight));
    let initial = session.observe().play.unwrap();
    let flame = initial.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60407")
        .unwrap()
        .card
        .id;
    let event = initial.players[1]
        .hand
        .iter()
        .find(|card| card.code == "04200")
        .unwrap()
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:fight:400:spell:{flame}"),
    );
    reveal(&mut session);
    choose(&mut session, "retaliate:complete");
    let first = session.observe().play.unwrap();
    assert_eq!(
        first.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == flame)
            .unwrap()
            .uses,
        0
    );
    assert_eq!((first.players[1].damage, first.players[1].horror), (2, 2));
    choose(&mut session, &format!("live-learn:{event}"));
    reveal(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(
        (
            after.completed_tests[1].total,
            after.completed_tests[1].success
        ),
        (5, true)
    );
    assert_eq!(after.scenario.enemies[0].damage, 2);
    assert_eq!(
        after.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == flame)
            .unwrap()
            .uses,
        0
    );
    assert_eq!(after.actions_remaining[1], 2);
    assert_eq!((after.players[1].damage, after.players[1].horror), (2, 2));
}
fn pass_window(session: &mut Session) {
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(session, "window:pass");
    }
}

#[test]
fn a_retry_reveals_again_and_repeats_the_originating_spell_drawback_with_event_still_in_limbo() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Retry(RetryFixture::ChargedDrawback));
    let before = session.observe().play.unwrap();
    let flame = before.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60407")
        .unwrap()
        .card
        .id;
    let event = before.players[1]
        .hand
        .iter()
        .find(|card| card.code == "04200")
        .unwrap()
        .clone();
    choose(
        &mut session,
        &format!("action:ordinary:fight:400:spell:{flame}"),
    );
    reveal(&mut session);
    choose(&mut session, "retaliate:complete");
    choose(&mut session, &format!("live-learn:{}", event.id));
    let retry = session.observe();
    let owner = &retry.play.as_ref().unwrap().players[1];
    assert!(
        !owner.hand.contains(&event) && !owner.discard.contains(&event),
        "Live and Learn stays in limbo while its test resolves"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("retry.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), retry);
    reveal(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(
        (after.players[1].damage, after.players[1].horror),
        (3, 2),
        "The new zero token triggers Azure Flame's damage again on this distinct test"
    );
    assert_eq!(after.completed_tests[1].total, 6);
    assert_eq!(after.scenario.enemies[0].damage, 2);
    assert_eq!(
        after.players[1]
            .discard
            .iter()
            .filter(|card| *card == &event)
            .count(),
        1
    );
    assert_eq!(after.actions_remaining[1], 2);
}

#[test]
fn lethal_retaliation_prevents_both_patrice_shuffle_and_a_retry() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Retry(RetryFixture::LethalFight));
    let hand = session.observe().play.unwrap().players[1].hand.clone();
    let heart = hand.iter().find(|card| card.code == "04201").unwrap().id;
    choose(&mut session, "action:ordinary:fight:400");
    pass_window(&mut session);
    choose(&mut session, &format!("commit:{heart}"));
    choose(&mut session, "commit:confirm");
    pass_window(&mut session);
    choose(&mut session, "test:reveal");
    let after = session.observe();
    let play = after.play.as_ref().unwrap();
    assert!(play.players[1].defeated);
    assert!(play.test.is_none());
    assert_eq!(play.bag_count, 16);
    assert!(
        play.players[1]
            .removed
            .iter()
            .any(|card| card.code == "04200")
    );
    assert!(
        !after
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("live-learn:")
                || choice.id.starts_with("patrice-shuffle:"))
    );
    assert!(
        after
            .choices
            .iter()
            .all(|choice| choice.id.starts_with("elimination:lead:"))
    );
    session.validate().unwrap();
}

#[test]
fn mists_retry_targets_the_watcher_now_in_play_instead_of_replaying_hand_discard() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Retry(RetryFixture::HandWatcher));
    let initial = session.observe().play.unwrap();
    let watcher = initial.players[1]
        .hand
        .iter()
        .find(|card| card.code == "06017")
        .unwrap()
        .clone();
    let event = initial.players[1]
        .hand
        .iter()
        .find(|card| card.code == "04200")
        .unwrap()
        .id;
    let mists = initial.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04029")
        .unwrap()
        .card
        .id;
    choose(
        &mut session,
        &format!("action:ordinary:evade:{}:spell:{mists}", watcher.id),
    );
    reveal(&mut session);
    let failed = session.observe().play.unwrap();
    let enemy = failed
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == watcher.id)
        .expect("Failed hand evasion spawns the actual Watcher");
    assert_eq!(
        enemy.engaged,
        Some(arkham_unfolded_experiment::gathering::Investigator::Patrice)
    );
    assert_eq!(
        (failed.players[1].damage, failed.players[1].horror),
        (0, 0),
        "Watcher has no Alert attack"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("spawned-watcher.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == format!("bounty:{}:0", watcher.id))
    {
        choose(&mut session, &format!("bounty:{}:0", watcher.id));
    }
    choose(&mut session, &format!("live-learn:{event}"));
    reveal(&mut session);
    let succeeded = session.observe();
    let play = succeeded.play.as_ref().unwrap();
    let enemy = play
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == watcher.id)
        .unwrap();
    assert!(enemy.exhausted && enemy.engaged.is_none());
    assert_eq!(enemy.damage, 0);
    assert!(
        !play.players[1]
            .discard
            .iter()
            .any(|card| card.id == watcher.id)
    );
    assert_eq!(
        play.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.id == mists)
            .unwrap()
            .uses,
        0
    );
    assert_eq!(play.actions_remaining[1], 2);
    assert!(succeeded.choices.iter().any(|choice|choice.id.starts_with("spell-move:") && choice.id!="spell-move:decline"),"Successful retry retains Mists' movement opportunity");
}
#[test]
fn patrice_elder_cleanup_precedes_live_and_learn_after_reward_and_retaliation() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Retry(RetryFixture::BasicFight));
    let initial = session.observe().play.unwrap();
    let heart = initial.players[1]
        .hand
        .iter()
        .find(|card| card.code == "04201")
        .unwrap()
        .clone();
    let event = initial.players[1]
        .hand
        .iter()
        .find(|card| card.code == "04200")
        .unwrap()
        .clone();
    choose(&mut session, "action:ordinary:fight:400");
    pass_window(&mut session);
    choose(&mut session, &format!("commit:{}", heart.id));
    choose(&mut session, "commit:confirm");
    pass_window(&mut session);
    choose(&mut session, "test:reveal");
    choose(&mut session, "retaliate:complete");
    let pending = session.observe();
    let play = pending.play.as_ref().unwrap();
    assert_eq!(
        (
            play.players[1].damage,
            play.players[1].horror,
            play.players[1].resources
        ),
        (2, 2, 5)
    );
    assert_eq!(play.bag_count, 16);
    assert!(play.players[1].discard.iter().any(|card| card == &heart));
    assert!(
        pending
            .choices
            .iter()
            .any(|choice| choice.id == "patrice-shuffle:decline"),
        "The resolved elder sign has forced timing before optional after-test events"
    );
    assert!(
        !pending
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("live-learn:"))
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("elder-shuffle.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    choose(&mut session, &format!("patrice-shuffle:keep:{}", heart.id));
    assert_eq!(session.observe().play.unwrap().players[1].discard, [heart]);
    choose(&mut session, &format!("live-learn:{}", event.id));
    reveal(&mut session);
    let after = session.observe().play.unwrap();
    assert_eq!(after.completed_tests.len(), 2);
    assert_eq!(
        (
            after.completed_tests[0].total,
            after.completed_tests[0].success
        ),
        (3, false)
    );
    assert_eq!(
        (
            after.completed_tests[1].total,
            after.completed_tests[1].success
        ),
        (4, true)
    );
    assert_eq!(after.scenario.enemies[0].damage, 1);
    assert_eq!(
        (
            after.players[1].damage,
            after.players[1].horror,
            after.players[1].resources
        ),
        (2, 2, 5)
    );
    assert_eq!(after.actions_remaining[1], 2);
    assert!(after.players[1].discard.iter().any(|card| card == &event));
    session.validate().unwrap();
}
