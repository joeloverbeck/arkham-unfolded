use arkham_unfolded_experiment::gathering::{
    HostileFixture, Investigator, LifecycleFixture, Session, continuity,
};
fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe().revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

#[test]
fn promise_commitment_horror_eliminates_before_any_token_is_revealed() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::PromiseDefeat));
    let before = session.observe().play.unwrap();
    let owner = &before.players[1];
    let owned = owner.hand.len()
        + owner.deck_count
        + owner.discard.len()
        + owner.assets.len()
        + owner.threats.len();
    let promise = owner
        .hand
        .iter()
        .find(|card| card.code == "07032")
        .unwrap()
        .id;
    if session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, &format!("commit:{promise}"));
    choose(&mut session, "commit:confirm");
    let after = session.observe();
    let play = after.play.as_ref().unwrap();
    assert!(play.players[1].defeated);
    assert_eq!(play.players[1].mental_trauma, 1);
    assert_eq!(play.players[1].removed.len(), owned);
    assert!(play.test.is_none() && play.tokens.is_none());
    assert_eq!(
        play.bag_count, 26,
        "All ten curses remain allocated; the interrupted test revealed no physical token"
    );
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.code == "01163")
    );
    assert!(
        after
            .choices
            .iter()
            .all(|choice| choice.id.starts_with("elimination:lead:"))
    );
    session.validate().unwrap();
}

/// Taking a resource provokes the engaged Priest's lone attack of
/// opportunity, which resolves by itself before its harm is assigned.
fn lethal_attack(session: &mut Session) {
    choose(session, "action:ordinary:resource");
    choose(session, "harm:damage:investigator");
    choose(session, "harm:damage:investigator");
}

#[test]
fn elimination_of_the_last_unfinished_turn_continues_into_the_enemy_phase() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::EndTurnDefeat));
    lethal_attack(&mut session);
    choose(&mut session, "elimination:lead:tony");
    // The enemy phase then begins by itself.
    let view = session.observe();
    assert!(
        view.history
            .iter()
            .any(|entry| entry == "Automatic: Begin enemy attacks"),
        "The surviving party must not stall with no available turns"
    );
}

#[test]
fn elimination_during_a_draw_removes_held_batch_and_commitment_without_finishing_rewards() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::DrawDefeat));
    let before = session.observe().play.unwrap();
    let player = &before.players[1];
    let owned = player.hand.len()
        + player.deck_count
        + player.discard.len()
        + player.assets.len()
        + player.threats.len()
        + before.test.as_ref().unwrap().committed.len();
    choose(&mut session, "result:0");
    for _ in 0..3 {
        choose(&mut session, "harm:damage:investigator");
    }
    // The full selected hand includes Deny Existence; decline its now-real
    // prevention opportunity before applying the lethal assigned attack.
    choose(&mut session, "harm:confirm");
    let pending = session.observe();
    let play = pending.play.as_ref().unwrap();
    let player = &play.players[1];
    assert!(player.defeated);
    assert_eq!(
        player.removed.len(),
        owned,
        "Held draw batch and Take Heart commitment remain in custody"
    );
    assert_eq!(
        player
            .removed
            .iter()
            .map(|card| card.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        owned
    );
    assert_eq!(
        (player.resources, player.deck_count, player.horror),
        (0, 0, 0),
        "No remaining draw, exhaustion horror or Take Heart resources resolve"
    );
    assert!(play.test.is_none());
    assert_eq!(play.bag_count, 16);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("draw-defeat.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    choose(&mut session, "elimination:lead:tony");
    assert_eq!(session.observe().play.unwrap().players[1].resources, 0);
}

#[test]
fn eliminating_a_premonition_controller_removes_the_event_and_returns_its_sealed_token() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::SealedDefeat));
    let before = session.observe().play.unwrap();
    let sealed = before.premonition.unwrap();
    assert_eq!(before.bag_count, 15);
    lethal_attack(&mut session);
    let after = session.observe().play.unwrap();
    assert!(after.premonition.is_none());
    assert_eq!(after.bag_count, 16);
    assert!(
        after.players[2]
            .removed
            .iter()
            .any(|card| card == &sealed.card)
    );
    session.validate().unwrap();
}

#[test]
fn simultaneous_damage_and_horror_defeat_has_a_saved_choice_of_one_trauma() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Lifecycle(
        LifecycleFixture::SimultaneousDefeat,
    ));
    lethal_attack(&mut session);
    let pending = session.observe();
    assert_eq!(
        pending
            .choices
            .iter()
            .filter(|choice| choice.id != "undo")
            .count(),
        2
    );
    assert_eq!(pending.play.as_ref().unwrap().players[2].physical_trauma, 0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("trauma.json");
    continuity::save_session(&path, &session).unwrap();
    let mut reopened = continuity::load_session(&path).unwrap();
    choose(&mut session, "elimination:physical");
    choose(&mut reopened, "elimination:mental");
    let physical = session.observe().play.unwrap();
    let mental = reopened.observe().play.unwrap();
    assert_eq!(
        (
            physical.players[2].physical_trauma,
            physical.players[2].mental_trauma
        ),
        (1, 2)
    );
    assert_eq!(
        (
            mental.players[2].physical_trauma,
            mental.players[2].mental_trauma
        ),
        (0, 3)
    );
    assert_eq!(physical.players[2].removed, mental.players[2].removed);
}

#[test]
fn resignation_uses_the_parlor_action_without_an_attack_or_defeat_trauma() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::Resignation));
    let before = session.observe().play.unwrap();
    choose(&mut session, "action:ordinary:resign");
    let after = session.observe();
    let player = &after.play.as_ref().unwrap().players[2];
    assert!(player.resigned && !player.defeated);
    assert_eq!(
        (player.damage, player.horror),
        (before.players[2].damage, before.players[2].horror)
    );
    assert_eq!((player.physical_trauma, player.mental_trauma), (0, 2));
    assert_eq!(
        after.play.as_ref().unwrap().scenario.enemies[0].engaged,
        None
    );
    assert_eq!(
        after.play.as_ref().unwrap().scenario.enemies[0].location,
        "01115"
    );
    assert!(
        after
            .choices
            .iter()
            .all(|choice| choice.id.starts_with("elimination:lead:"))
    );
}

#[test]
fn the_last_elimination_finishes_after_clue_drop_and_owned_zone_cleanup() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::LastDefeat));
    lethal_attack(&mut session);
    let after = session.observe();
    assert!(after.choices.is_empty());
    let play = after.play.unwrap();
    let result = play.scenario.outcome.as_ref().unwrap();
    assert_eq!(
        result.resolution,
        arkham_unfolded_experiment::gathering::Resolution::NoResolution
    );
    assert_eq!(
        result.experience_gained, [3; 3],
        "Only cleared Cellar contributes victory1 before the two scenario XP"
    );
    assert!(play.players.iter().all(|player| player.resources == 0
        && player.hand.is_empty()
        && player.deck_count == 0
        && player.assets.is_empty()));
    assert_eq!(
        play.scenario
            .locations
            .iter()
            .find(|location| location.code == "01112")
            .unwrap()
            .clues,
        2
    );
    assert_eq!(play.players[2].physical_trauma, 1);
    session.validate().unwrap();
}
#[test]
fn defeated_lead_drops_clues_removes_owned_cards_and_resumes_with_a_new_lead() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::LeadDefeat));
    let before = session.observe().play.unwrap();
    let owned = before.players[2].deck_count
        + before.players[2].hand.len()
        + before.players[2].assets.len()
        + 1;
    choose(&mut session, "action:ordinary:resource");
    choose(&mut session, "harm:damage:investigator");
    choose(&mut session, "harm:damage:investigator");
    let pending = session.observe();
    let player = &pending.play.as_ref().unwrap().players[2];
    assert!(player.defeated);
    assert_eq!(player.physical_trauma, 1);
    assert_eq!(player.mental_trauma, 2);
    assert_eq!((player.clues, player.resources), (0, 0));
    assert!(
        player.hand.is_empty()
            && player.assets.is_empty()
            && player.threats.is_empty()
            && player.discard.is_empty()
    );
    assert_eq!(player.deck_count, 0);
    assert_eq!(player.removed.len(), owned);
    let play = pending.play.as_ref().unwrap();
    assert_eq!(
        play.scenario
            .locations
            .iter()
            .find(|location| location.code == "01112")
            .unwrap()
            .clues,
        2
    );
    assert!(
        play.encounter_discard
            .iter()
            .any(|card| card.code == "01165")
    );
    assert_eq!(play.scenario.enemies[0].engaged, Some(Investigator::Tony));
    assert!(!play.scenario.enemies[0].exhausted);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("lead.json");
    continuity::save_session(&path, &session).unwrap();
    session = continuity::load_session(&path).unwrap();
    assert_eq!(session.observe(), pending);
    choose(&mut session, "elimination:lead:patrice");
    let after = session.observe();
    assert_eq!(after.play.as_ref().unwrap().lead, Investigator::Patrice);
    assert!(
        !after
            .choices
            .iter()
            .any(|choice| choice.id == "turn:jacqueline")
    );
    assert_eq!(
        after.play.as_ref().unwrap().players[2].resources,
        0,
        "The interrupted action cannot resolve for an eliminated investigator"
    );
    session.validate().unwrap();
}
