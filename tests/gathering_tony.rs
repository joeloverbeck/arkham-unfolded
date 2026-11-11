//! Original selected 09068/03263, Rules Reference pp3,10,18–19,23–25.
//! Source-derived expected values through the public Session decision seam.

use arkham_unfolded_experiment::gathering::{
    HostileFixture, Investigator, PlayObservation, SearchFixture, Session, TonyEffectFixture,
};

/// Whether the session performed a step itself with this history label.
fn performed(session: &Session, label: &str) -> bool {
    session
        .observe()
        .history
        .iter()
        .any(|entry| entry.strip_prefix("Automatic: ") == Some(label))
}

/// The table state presented when Tony's harm resolved during the decision
/// taken at `revision`; a lone attack of opportunity resolves by itself.
fn attacked(session: &Session, revision: u64) -> PlayObservation {
    session
        .presentation_after(revision)
        .iter()
        .find(|moment| moment.caption.starts_with("Tony Morgan resolves"))
        .map(|moment| moment.play.clone())
        .expect("Tony's harm is presented")
}

fn choose(session: &mut Session, id: &str) {
    let observation = session.observe();
    assert!(
        observation.choices.iter().any(|choice| choice.id == id),
        "Expected {id}; got {:?}",
        observation.choices
    );
    session.choose(observation.revision, id).unwrap();
}

fn play_hornet(session: &mut Session) -> u16 {
    let card = session.observe().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "09068")
        .unwrap()
        .id;
    choose(session, &format!("action:ordinary:play:tony:{card}"));
    card
}

#[test]
fn hornet_rewards_have_then_priority_before_optional_bounties() {
    for (code, health) in [
        ("01118", 4),
        ("01119", 4),
        ("01159", 1),
        ("01160", 2),
        ("01161", 3),
    ] {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HornetTargets));
        let event = play_hornet(&mut session);
        let pending = session.observe();
        let play = pending.play.unwrap();
        assert_eq!(
            play.encounter_deck_count, 26,
            "Looking does not remove cards from the encounter deck"
        );
        assert_eq!(play.searched_encounter.len(), 9);
        assert_eq!(pending.choices.len(), 5);
        assert!(
            !pending
                .choices
                .iter()
                .any(|choice| choice.id == "search:no-match")
        );
        assert_eq!(play.actions_remaining[0], 2);
        assert_eq!(play.players[0].resources, 4);
        assert!(!play.players[0].hand.iter().any(|card| card.id == event));
        assert!(!play.players[0].discard.iter().any(|card| card.id == event));
        let enemy = play
            .searched_encounter
            .iter()
            .find(|card| card.code == code)
            .unwrap()
            .id;
        if code == "01118" {
            use arkham_unfolded_experiment::gathering::continuity::{load_session, save_session};
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("hornet-target.json");
            save_session(&path, &session).unwrap();
            let reopened = load_session(&path).unwrap();
            assert_eq!(reopened.observe(), session.observe());
            session = reopened;
        }
        choose(&mut session, &format!("search:enemy:{enemy}:then"));
        let spawned = session.observe().play.unwrap();
        let target = spawned
            .scenario
            .enemies
            .iter()
            .find(|state| state.card.id == enemy)
            .unwrap();
        assert_eq!(
            target.location, "01111",
            "Even Attic/Cellar spawns enter the Study"
        );
        assert_eq!(target.engaged, Some(Investigator::Tony));
        assert_eq!(spawned.players[0].clues, 1);
        assert_eq!(
            spawned.players[0].resources,
            4 + health,
            "RR Then gives the dependent reward priority over after-entry reactions"
        );
        assert_eq!(spawned.encounter_deck_count, 25);
        let bounties = health.min(3);
        choose(&mut session, &format!("bounty:{enemy}:{bounties}"));
        assert!(performed(
            &session,
            "Shuffle the encounter deck and finish Kicking the Hornet's Nest"
        ));
        let finished = session.observe().play.unwrap();
        assert_eq!(finished.players[0].resources, 4 + health);
        assert_eq!(finished.players[0].clues, 1);
        assert_eq!(
            finished
                .scenario
                .locations
                .iter()
                .find(|location| location.code == "01111")
                .unwrap()
                .clues,
            5
        );
        assert_eq!(finished.encounter_deck_count, 25);
        assert!(finished.searched_encounter.is_empty());
        assert_eq!(
            finished.players[0]
                .discard
                .iter()
                .filter(|card| card.id == event)
                .count(),
            1
        );
        assert_eq!(
            finished
                .scenario
                .enemies
                .iter()
                .find(|state| state.card.id == enemy)
                .unwrap()
                .bounties,
            bounties
        );
        assert_eq!(
            finished.players[0]
                .assets
                .iter()
                .find(|asset| asset.card.code == "06010")
                .unwrap()
                .uses,
            6 - bounties
        );
        session.validate().unwrap();
    }
}

#[test]
fn hornet_no_match_spends_the_action_and_gives_no_dependent_reward() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HornetNoMatch));
    let event = play_hornet(&mut session);
    // Without an eligible enemy, the search's one outcome resolves by itself.
    assert!(performed(
        &session,
        "No eligible enemy: shuffle the encounter deck; gain no clue or resources"
    ));
    let finished = session.observe().play.unwrap();
    assert!(finished.scenario.enemies.is_empty());
    assert_eq!(finished.players[0].resources, 4);
    assert_eq!(finished.players[0].clues, 0);
    assert_eq!(finished.actions_remaining[0], 2);
    assert_eq!(finished.encounter_deck_count, 26);
    assert!(finished.searched_encounter.is_empty());
    assert!(
        finished.players[0]
            .discard
            .iter()
            .any(|card| card.id == event)
    );
    session.validate().unwrap();
}

#[test]
fn hornet_pays_before_opportunity_and_searches_only_after_the_attack_finishes() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HornetOpportunity));
    let before = session.observe().play.unwrap();
    let minion = before.scenario.enemies[0].card.id;
    let playing = session.observe().revision;
    let event = play_hornet(&mut session);
    let paid = attacked(&session, playing);
    assert_eq!(paid.actions_remaining[0], 2);
    assert!(paid.searched_encounter.is_empty());
    assert!(!paid.players[0].hand.iter().any(|card| card.id == event));
    let searched = session.observe().play.unwrap();
    assert_eq!(searched.players[0].damage, before.players[0].damage + 1);
    assert_eq!(searched.players[0].horror, before.players[0].horror + 1);
    assert_eq!(searched.searched_encounter.len(), 9);
    assert!(
        !searched
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == minion)
            .unwrap()
            .exhausted
    );
    session.validate().unwrap();
}

fn play_hunt(session: &mut Session) -> u16 {
    let card = session.observe().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "03263")
        .unwrap()
        .id;
    choose(session, "mythos:draw");
    assert!(
        session.observe().play.unwrap().pending_revelation.is_none(),
        "The original encounter has not been drawn before the replacement choice"
    );
    choose(session, &format!("search:play-hunt:{card}"));
    card
}

#[test]
fn on_the_hunt_replaces_only_tonys_mythos_draw_and_resumes_the_next_investigator() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HuntTargets));
    let event = play_hunt(&mut session);
    let searching = session.observe().play.unwrap();
    assert_eq!(
        searching.actions_remaining[0], 3,
        "Fast event spends no action"
    );
    assert_eq!(searching.players[0].resources, 3);
    assert_eq!(searching.encounter_deck_count, 26);
    assert_eq!(searching.searched_encounter.len(), 9);
    let icy = searching
        .searched_encounter
        .iter()
        .find(|card| card.code == "01119")
        .unwrap()
        .id;
    choose(&mut session, &format!("search:enemy:{icy}"));
    choose(&mut session, &format!("bounty:{icy}:0"));
    // The search finishes by itself, then Patrice's draw follows Tony's
    // replaced one.
    let history = session.observe().history;
    let finish = history
        .iter()
        .position(|entry| entry == "Automatic: Shuffle the encounter deck and finish On the Hunt")
        .unwrap();
    let patrice = history
        .iter()
        .position(|entry| entry == "Automatic: Patrice Hathaway draws an encounter card")
        .unwrap();
    assert!(finish < patrice);
    let play = session.observe().play.unwrap();
    assert_eq!(play.players[0].resources, 3);
    assert_eq!(play.players[0].clues, 0);
    assert!(play.searched_encounter.is_empty());
    assert!(play.players[0].discard.iter().any(|card| card.id == event));
    let enemy = play
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == icy)
        .unwrap();
    assert_eq!(enemy.location, "01111");
    assert_eq!(enemy.engaged, Some(Investigator::Tony));
    session.validate().unwrap();
}

#[test]
fn on_the_hunt_no_match_draws_an_encounter_and_retains_the_event_until_it_finishes() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HuntNoMatch));
    let event = play_hunt(&mut session);
    // With no enemy to find, the fallback draw is the search's one outcome.
    assert!(performed(
        &session,
        "No enemy: shuffle, then draw an encounter card"
    ));
    let drawn = session.observe().play.unwrap();
    assert_eq!(drawn.encounter_deck_count, 25);
    assert_eq!(
        drawn.pending_revelation.as_ref().unwrap().drawn_by,
        Investigator::Tony
    );
    assert!(drawn.searched_encounter.is_empty());
    assert_eq!(drawn.players[0].resources, 3);
    assert!(!drawn.players[0].discard.iter().any(|card| card.id == event));
    for _ in 0..20 {
        let choices = session.observe().choices;
        if performed(&session, "Finish resolving On the Hunt") {
            break;
        }
        let id = [
            "revelation:resolve",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| choices.iter().any(|choice| choice.id == *id))
        .map(str::to_owned)
        .or_else(|| {
            choices
                .iter()
                .find(|choice| choice.id.starts_with("bounty:") && choice.id.ends_with(":0"))
                .map(|choice| choice.id.clone())
        })
        .unwrap_or_else(|| panic!("Unhandled fallback continuation {choices:?}"));
        choose(&mut session, &id);
    }
    // The event finishes by itself once its fallback encounter resolves.
    let play = session.observe().play.unwrap();
    assert!(play.players[0].discard.iter().any(|card| card.id == event));
    assert_eq!(play.players[0].clues, 0);
    assert_eq!(play.actions_remaining[0], 3);
    session.validate().unwrap();
}

#[test]
fn declining_on_the_hunt_draws_the_original_card_with_its_normal_spawn_rule() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HuntTargets));
    let before = session.observe().play.unwrap();
    let event = before.players[0]
        .hand
        .iter()
        .find(|card| card.code == "03263")
        .unwrap()
        .id;
    choose(&mut session, "mythos:draw");
    choose(&mut session, "search:decline-hunt");
    let drawn = session.observe().play.unwrap();
    assert!(drawn.searched_encounter.is_empty());
    let finished = session.observe().play.unwrap();
    assert_eq!(finished.players[0].resources, 4);
    assert!(finished.players[0].hand.iter().any(|card| card.id == event));
    assert!(
        !finished
            .scenario
            .enemies
            .iter()
            .any(|enemy| enemy.card.code == "01118"),
        "Flesh-Eater's normal Attic spawn cannot occur while only Study exists"
    );
    assert!(
        finished
            .encounter_discard
            .iter()
            .any(|card| card.code == "01118")
    );
    session.validate().unwrap();
}

fn tony_card(session: &Session, code: &str) -> u16 {
    session.observe().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == code)
        .unwrap()
        .id
}

fn finish_plain_test(session: &mut Session) {
    for _ in 0..12 {
        let choices = session.observe().choices;
        if session.observe().play.unwrap().test.is_none() {
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
        .unwrap_or_else(|| panic!("Unhandled plain test {choices:?}"));
        choose(session, id);
    }
    panic!("Event test did not finish");
}

#[test]
fn scene_of_the_crime_is_only_a_first_action_and_does_not_provoke_the_engaged_enemy() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::TonyEvents));
    let event = tony_card(&session, "04103");
    let before = session.observe().play.unwrap();
    choose(
        &mut session,
        &format!("action:ordinary:play:tony:{event}:04103"),
    );
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[0].resources, 6);
    assert_eq!(after.players[0].clues, 2);
    assert_eq!(after.players[0].damage, before.players[0].damage);
    assert_eq!(after.actions_remaining[0], 2);
    assert_eq!(
        after
            .scenario
            .locations
            .iter()
            .find(|location| location.code == "01113")
            .unwrap()
            .clues,
        4
    );
    assert!(after.players[0].discard.iter().any(|card| card.id == event));
    session.validate().unwrap();

    let mut late = Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::TonyEvents));
    let event = tony_card(&late, "04103");
    let rats = late
        .observe()
        .play
        .unwrap()
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01159")
        .unwrap()
        .card
        .id;
    choose(&mut late, "action:ordinary:resource");
    assert!(performed(
        &late,
        &format!("Resolve enemy {rats}'s attack of opportunity")
    ));
    assert!(
        !late
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == format!("action:ordinary:play:tony:{event}:04103"))
    );
    assert!(
        late.observe().play.unwrap().players[0]
            .hand
            .iter()
            .any(|card| card.id == event)
    );
    late.validate().unwrap();
}

#[test]
fn boosted_intel_pays_all_costs_before_opportunity_then_discovers_remote_clues_without_investigating()
 {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::TonyEvents));
    let event = tony_card(&session, "05111");
    let before = session.observe().play.unwrap();
    let rats = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01159")
        .unwrap()
        .card
        .id;
    let playing = session.observe().revision;
    choose(
        &mut session,
        &format!("action:ordinary:play:tony:{event}:05111:01114:1:1"),
    );
    let paid = attacked(&session, playing);
    assert_eq!(paid.players[0].resources, 2);
    assert_eq!(paid.players[0].clues, 0);
    assert_eq!(
        paid.scenario
            .locations
            .iter()
            .find(|location| location.code == "01114")
            .unwrap()
            .clues,
        6
    );
    assert!(performed(
        &session,
        &format!("Resolve enemy {rats}'s attack of opportunity")
    ));
    let after = session.observe().play.unwrap();
    assert_eq!(after.players[0].damage, before.players[0].damage + 1);
    assert_eq!(after.players[0].clues, 2);
    let cellar = after
        .scenario
        .locations
        .iter()
        .find(|location| location.code == "01114")
        .unwrap();
    assert_eq!(cellar.clues, 4);
    assert!(
        cellar.fog.is_some(),
        "Testless discovery does not count as successful investigation"
    );
    assert!(after.completed_tests.is_empty());
    assert!(after.players[0].discard.iter().any(|card| card.id == event));
    session.validate().unwrap();
}

#[test]
fn boosted_small_favor_defeats_a_remote_enemy_without_a_fight_and_pays_its_bounties() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::TonyEvents));
    let event = tony_card(&session, "05277");
    let before = session.observe().play.unwrap();
    let ravenous = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01161")
        .unwrap()
        .card
        .id;
    let rats = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01159")
        .unwrap()
        .card
        .id;
    let playing = session.observe().revision;
    choose(
        &mut session,
        &format!("action:ordinary:play:tony:{event}:05277:{ravenous}:1:1"),
    );
    assert_eq!(attacked(&session, playing).players[0].resources, 2);
    assert!(performed(
        &session,
        &format!("Resolve enemy {rats}'s attack of opportunity")
    ));
    let defeated = session.observe().play.unwrap();
    assert_eq!(defeated.players[0].resources, 4);
    assert!(
        defeated
            .scenario
            .enemies
            .iter()
            .all(|enemy| enemy.card.id != ravenous)
    );
    assert!(
        defeated
            .encounter_discard
            .iter()
            .any(|card| card.id == ravenous)
    );
    assert!(defeated.completed_tests.is_empty());
    assert_eq!(
        defeated.players[2].damage, before.players[2].damage,
        "Direct damage is not a failed Fight against another investigator's enemy"
    );
    assert!(performed(&session, "Finish resolving Small Favor"));
    assert!(
        session.observe().play.unwrap().players[0]
            .discard
            .iter()
            .any(|card| card.id == event)
    );
    session.validate().unwrap();
}

#[test]
fn get_over_here_uses_the_bounty_action_and_frozen_cost_then_pulls_engages_and_fights_once() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::GetOverHereFrozen));
    let event = tony_card(&session, "60114");
    let before = session.observe().play.unwrap();
    let minion = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01160")
        .unwrap()
        .card
        .id;
    let ravenous = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01161")
        .unwrap()
        .card
        .id;
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.ends_with(&format!(":60114:{ravenous}"))),
        "Two connections is beyond this event's range"
    );
    choose(
        &mut session,
        &format!("action:tony:play:tony:{event}:60114:{minion}"),
    );
    let fighting = session.observe().play.unwrap();
    assert_eq!(
        fighting.actions_remaining[0], 2,
        "The restricted bounty action pays one of Frozen in Fear's two action costs"
    );
    assert_eq!(fighting.players[0].resources, 6);
    assert_eq!(
        fighting.players[0].damage, before.players[0].damage,
        "Fight designator exempts attacks of opportunity"
    );
    let enemy = fighting
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == minion)
        .unwrap();
    assert_eq!(enemy.location, "01113");
    assert_eq!(enemy.engaged, Some(Investigator::Tony));
    assert!(fighting.test.is_some());
    assert!(
        !fighting.players[0]
            .discard
            .iter()
            .any(|card| card.id == event)
    );
    finish_plain_test(&mut session);
    let finished_test = session.observe().play.unwrap();
    assert_eq!(finished_test.completed_tests.len(), 1);
    assert_eq!(
        finished_test
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == minion)
            .unwrap()
            .damage,
        1
    );
    // The event then finishes by itself.
    assert!(
        session
            .observe()
            .history
            .iter()
            .any(|entry| entry.starts_with("Automatic: Finish resolving "))
    );
    assert!(
        !session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("action:tony:"))
    );
    choose(&mut session, &format!("action:ordinary:fight:{minion}"));
    assert_eq!(
        session.observe().play.unwrap().actions_remaining[0],
        1,
        "Frozen in Fear charges only the first qualifying action this round"
    );
    session.validate().unwrap();
}

fn continue_test(session: &mut Session) {
    let choices = session.observe().choices;
    let id = [
        "test-card:pass",
        "window:pass",
        "commit:confirm",
        "test:reveal",
        "lucky:decline",
        "success:pass",
        "revelation:resolve",
    ]
    .into_iter()
    .find(|id| choices.iter().any(|choice| choice.id == *id))
    .map(str::to_owned)
    .or_else(|| {
        choices
            .iter()
            .find(|choice| {
                choice.id.starts_with("draw:reveal:") || choice.id.starts_with("result:")
            })
            .map(|choice| choice.id.clone())
    })
    .unwrap_or_else(|| panic!("Unhandled test continuation {choices:?}"));
    choose(session, &id);
}

fn reach_tony_choice(session: &mut Session, expected: &str) {
    for _ in 0..20 {
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == expected)
        {
            return;
        }
        continue_test(session);
    }
    panic!("Did not reach {expected}");
}

#[test]
fn gregory_spends_his_own_tokens_and_cigarette_case_draws_before_attack_results() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::GregoryCigarette));
    let before = session.observe().play.unwrap();
    let gregory = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "06162")
        .unwrap()
        .card
        .id;
    let case = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04107")
        .unwrap()
        .card
        .id;
    let gun = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "06011")
        .unwrap()
        .card
        .id;
    let enemy = before.scenario.enemies[0].card.id;
    choose(
        &mut session,
        &format!("action:ordinary:fight:{enemy}:weapon:{gun}"),
    );
    choose(&mut session, &format!("tony:gregory:{gregory}:3"));
    let wagered = session.observe().play.unwrap();
    assert_eq!(wagered.players[0].resources, 5);
    assert_eq!(
        wagered.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == gregory)
            .unwrap()
            .uses,
        6
    );
    reach_tony_choice(&mut session, &format!("tony:case:{case}"));
    let successful = session.observe().play.unwrap();
    assert_eq!(successful.test.as_ref().unwrap().total, Some(5));
    assert_eq!(successful.scenario.enemies[0].damage, 0);
    assert_eq!(
        successful.players[0].resources, 5,
        "Gregory's ST7 result has not paid during the ST6 reaction"
    );
    choose(&mut session, &format!("tony:case:{case}"));
    for _ in 0..20 {
        if session.observe().play.as_ref().unwrap().test.is_none() {
            break;
        }
        continue_test(&mut session);
    }
    let finished = session.observe().play.unwrap();
    assert!(finished.test.is_none());
    assert_eq!(
        finished.players[0].resources, 9,
        "Gregory3 plus the defeated enemy's bounty1"
    );
    assert!(
        !finished.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == case)
            .unwrap()
            .ready
    );
    assert!(
        finished.players[0]
            .threats
            .iter()
            .any(|card| card.code == "02038"),
        "Cigarette Case resolves the actual drawn weakness"
    );
    assert_eq!(
        finished.players[0].deck_count,
        before.players[0].deck_count - 1
    );
    assert_eq!(finished.bag_count, 16);
    session.validate().unwrap();
}

#[test]
fn gregorys_delayed_reward_survives_his_defeat_but_still_requires_the_wagered_margin() {
    for wager in [2, 3] {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::GregoryLeaves));
        let before = session.observe().play.unwrap();
        let gregory = before.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.code == "06162")
            .unwrap()
            .card
            .id;
        let case = before.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.code == "04107")
            .unwrap()
            .card
            .id;
        let gun = before.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.code == "06011")
            .unwrap()
            .card
            .id;
        let enemy = before.scenario.enemies[0].card.id;
        choose(
            &mut session,
            &format!("action:ordinary:fight:{enemy}:weapon:{gun}"),
        );
        choose(&mut session, &format!("tony:gregory:{gregory}:{wager}"));
        reach_tony_choice(&mut session, &format!("harm:damage:{gregory}"));
        choose(&mut session, &format!("harm:damage:{gregory}"));
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "harm:confirm")
        {
            choose(&mut session, "harm:confirm");
        }
        reach_tony_choice(&mut session, &format!("tony:case:{case}"));
        let success = session.observe().play.unwrap();
        assert_eq!(success.test.as_ref().unwrap().total, Some(4));
        assert!(
            success.players[0]
                .discard
                .iter()
                .any(|card| card.id == gregory)
        );
        assert_eq!(success.scenario.enemies[0].damage, 0);
        choose(&mut session, "success:pass");
        for _ in 0..20 {
            if session.observe().play.as_ref().unwrap().test.is_none() {
                break;
            }
            continue_test(&mut session);
        }
        let finished = session.observe().play.unwrap();
        assert!(finished.test.is_none());
        assert_eq!(
            finished.players[0].resources,
            if wager == 2 { 8 } else { 6 }
        );
        assert!(
            finished.players[0]
                .assets
                .iter()
                .find(|asset| asset.card.id == case)
                .unwrap()
                .ready,
            "The ST6 reaction is optional"
        );
        session.validate().unwrap();
    }
}

fn finish_current_test(session: &mut Session) {
    for _ in 0..25 {
        if session.observe().play.as_ref().unwrap().test.is_none() {
            return;
        }
        continue_test(session);
    }
    panic!("Skill test did not finish");
}

#[test]
fn one_in_the_chamber_reloads_final_ammo_and_bonuses_only_the_next_attack_with_that_gun() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::Chamber));
    let before = session.observe().play.unwrap();
    let emptying_gun = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "06011" && asset.uses == 1)
        .unwrap()
        .card
        .id;
    let other_gun = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "06011" && asset.uses == 3)
        .unwrap()
        .card
        .id;
    let event = tony_card(&session, "09029");
    let enemies: Vec<_> = before
        .scenario
        .enemies
        .iter()
        .map(|enemy| enemy.card.id)
        .collect();
    choose(
        &mut session,
        &format!("action:ordinary:fight:{}:weapon:{emptying_gun}", enemies[0]),
    );
    finish_current_test(&mut session);
    let ended = session.observe().play.unwrap();
    assert_eq!(ended.completed_tests[0].total, 4);
    assert_eq!(
        ended.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == emptying_gun)
            .unwrap()
            .uses,
        0
    );
    choose(&mut session, &format!("tony:chamber:{event}"));
    let reloaded = session.observe().play.unwrap();
    assert_eq!(reloaded.players[0].resources, 4);
    assert_eq!(
        reloaded.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == emptying_gun)
            .unwrap()
            .uses,
        1
    );
    assert!(
        reloaded.players[0]
            .discard
            .iter()
            .any(|card| card.id == event)
    );
    choose(
        &mut session,
        &format!("action:ordinary:fight:{}:weapon:{other_gun}", enemies[1]),
    );
    finish_current_test(&mut session);
    assert_eq!(
        session.observe().play.unwrap().completed_tests[1].total,
        4,
        "The other Long Colt receives no bonus"
    );
    choose(
        &mut session,
        &format!("action:ordinary:fight:{}:weapon:{emptying_gun}", enemies[2]),
    );
    finish_current_test(&mut session);
    let finished = session.observe().play.unwrap();
    assert_eq!(finished.completed_tests[2].total, 7);
    assert_eq!(finished.actions_remaining[0], 0);
    assert_eq!(
        finished.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == other_gun)
            .unwrap()
            .uses,
        2
    );
    assert_eq!(
        finished.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == emptying_gun)
            .unwrap()
            .uses,
        0
    );
    assert!(finished.scenario.enemies.is_empty());
    session.validate().unwrap();
}

#[test]
fn one_in_the_chamber_unused_bonus_expires_through_the_actual_round_transition() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::ChamberExpires));
    let before = session.observe().play.unwrap();
    let gun = before.players[0]
        .assets
        .iter()
        .find(|asset| asset.card.code == "06011" && asset.uses == 1)
        .unwrap()
        .card
        .id;
    let event = tony_card(&session, "09029");
    let first_enemy = before.scenario.enemies[0].card.id;
    choose(
        &mut session,
        &format!("action:ordinary:fight:{first_enemy}:weapon:{gun}"),
    );
    finish_current_test(&mut session);
    choose(&mut session, &format!("tony:chamber:{event}"));
    choose(&mut session, "turn:end");
    choose(&mut session, "turn:patrice");
    choose(&mut session, "turn:end");
    // Jacqueline's is then the only remaining turn and begins by itself.
    assert!(performed(&session, "Begin Jacqueline Fine's turn"));
    choose(&mut session, "turn:end");
    for _ in 0..90 {
        let observation = session.observe();
        if observation
            .choices
            .iter()
            .any(|choice| choice.id == "turn:tony")
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
                        | "enemy:attack-complete"
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
                        | "round-end:decline-act"
                        | "round-end:finish"
                        | "mythos:begin"
                        | "mythos:encounters"
                        | "mythos:draw"
                        | "mythos:draws-complete"
                        | "revelation:resolve"
                        | "investigation:begin"
                        | "search:decline-hunt"
                ) || choice.id.starts_with("enemy:attack:")
                    || choice.id.starts_with("upkeep:patrice-discard:")
                    || choice.id.starts_with("upkeep:discard:")
            })
            .unwrap_or_else(|| {
                panic!(
                    "Unexpected round-transition decision: {:?}",
                    observation.choices
                )
            });
        choose(&mut session, &choice.id);
    }
    let next_round = session.observe().play.unwrap();
    assert_eq!(next_round.round, before.round + 1);
    assert_eq!(next_round.agenda_doom, 1);
    assert_eq!(next_round.players[0].damage, before.players[0].damage + 2);
    assert_eq!(next_round.players[0].horror, before.players[0].horror + 2);
    choose(&mut session, "turn:tony");
    let enemy = next_round.scenario.enemies[0].card.id;
    choose(
        &mut session,
        &format!("action:ordinary:fight:{enemy}:weapon:{gun}"),
    );
    finish_current_test(&mut session);
    let finished = session.observe().play.unwrap();
    assert_eq!(
        finished.completed_tests[1].total, 4,
        "Tony's combat 5 and the fixed -1 token apply; the unused previous-round +3 has expired"
    );
    assert_eq!(
        finished.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.id == gun)
            .unwrap()
            .uses,
        0
    );
    session.validate().unwrap();
}

#[test]
fn quick_thinking_retains_its_owner_after_cleanup_and_grants_one_action_without_spending_the_normal_pool()
 {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::QuickThinking));
    let before = session.observe().play.unwrap();
    let quick: Vec<_> = before.players[0]
        .hand
        .iter()
        .filter(|card| card.code == "02229")
        .map(|card| card.id)
        .collect();
    let enemies: Vec<_> = before
        .scenario
        .enemies
        .iter()
        .map(|enemy| enemy.card.id)
        .collect();
    assert_eq!(
        quick.len(),
        1,
        "Tony's actual selected deck contains one copy"
    );
    choose(
        &mut session,
        &format!("action:ordinary:fight:{}", enemies[0]),
    );
    reach_tony_choice(&mut session, &format!("commit:{}", quick[0]));
    choose(&mut session, &format!("commit:{}", quick[0]));
    finish_current_test(&mut session);
    let completed = session.observe().play.unwrap();
    assert_eq!(completed.bag_count, 16);
    assert_eq!(completed.actions_remaining[0], 2);
    assert!(
        completed.players[0]
            .discard
            .iter()
            .any(|card| card.id == quick[0])
    );
    choose(&mut session, "tony:quick-thinking:tony");
    choose(&mut session, &format!("granted:fight:{}", enemies[1]));
    finish_current_test(&mut session);
    let finished = session.observe();
    assert!(
        !finished
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("tony:quick-thinking:")
                || choice.id.starts_with("granted:"))
    );
    let play = finished.play.unwrap();
    assert_eq!(
        play.actions_remaining[0], 2,
        "Only the original action reduces the normal pool"
    );
    assert_eq!(play.completed_tests.len(), 2);
    assert!(play.scenario.enemies.is_empty());
    for id in quick {
        assert!(play.players[0].discard.iter().any(|card| card.id == id));
    }
    session.validate().unwrap();
}

#[test]
fn quick_thinking_committed_by_tony_preserves_patrices_turn_and_uses_tonys_action() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::QuickSupport));
    let before = session.observe().play.unwrap();
    let quick = tony_card(&session, "02229");
    let scene = tony_card(&session, "04103");
    choose(
        &mut session,
        &format!(
            "action:ordinary:fight:{}",
            before.scenario.enemies[0].card.id
        ),
    );
    reach_tony_choice(&mut session, &format!("commit:{quick}"));
    choose(&mut session, &format!("commit:{quick}"));
    finish_current_test(&mut session);
    choose(&mut session, "tony:quick-thinking:tony");
    let granted = session.observe();
    assert_eq!(granted.decision_owner, Some(Investigator::Tony));
    assert!(
        granted
            .choices
            .iter()
            .any(|choice| choice.id == "granted:resource")
    );
    assert!(
        !granted
            .choices
            .iter()
            .any(|choice| choice.id.contains(&format!("play:tony:{scene}:04103"))),
        "Scene of the Crime requires Tony's actual turn, not an action as if it were his turn"
    );
    choose(&mut session, "granted:resource");
    // The remaining engaged Minion makes a real attack of opportunity before
    // Tony receives the resource, then the interrupted turn resumes.
    assert!(performed(
        &session,
        &format!(
            "Resolve enemy {}'s attack of opportunity",
            before.scenario.enemies[1].card.id
        )
    ));
    let finished = session.observe();
    let play = finished.play.unwrap();
    assert_eq!(play.players[0].resources, before.players[0].resources + 1);
    assert_eq!(play.players[0].damage, before.players[0].damage + 1);
    assert_eq!(play.players[0].horror, before.players[0].horror + 1);
    assert_eq!(play.actions_remaining, [3, 2, 3]);
    assert_eq!(play.actor, Some(Investigator::Patrice));
    assert_eq!(finished.decision_owner, Some(Investigator::Patrice));
    assert!(play.players[0].discard.iter().any(|card| card.id == quick));
    assert!(play.players[0].hand.iter().any(|card| card.id == scene));
    session.validate().unwrap();
}

#[test]
fn watch_this_proposes_reversible_cost_then_pays_once_and_rewards_only_success_by_one() {
    for (fixture, final_resources) in [
        (TonyEffectFixture::WatchSuccess, 8),
        (TonyEffectFixture::WatchTie, 2),
        (TonyEffectFixture::WatchFailure, 2),
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::Tony(fixture));
        let card = tony_card(&session, "03233");
        let enemy = session.observe().play.unwrap().scenario.enemies[0].card.id;
        choose(&mut session, &format!("action:ordinary:fight:{enemy}"));
        reach_tony_choice(&mut session, &format!("commit:{card}"));
        choose(&mut session, &format!("commit:{card}"));
        choose(&mut session, &format!("tony:watch:{card}:3"));
        assert_eq!(session.observe().play.unwrap().players[0].resources, 5);
        choose(&mut session, &format!("commit:{card}"));
        assert!(
            !session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("tony:watch:"))
        );
        assert_eq!(session.observe().play.unwrap().players[0].resources, 5);
        choose(&mut session, &format!("commit:{card}"));
        choose(&mut session, &format!("tony:watch:{card}:3"));
        if fixture == TonyEffectFixture::WatchSuccess {
            use arkham_unfolded_experiment::gathering::continuity::{load_session, save_session};
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("watch-wager.json");
            save_session(&path, &session).unwrap();
            session = load_session(&path).unwrap();
        }
        choose(&mut session, "commit:confirm");
        let paid = session.observe().play.unwrap();
        assert_eq!(paid.players[0].resources, 2);
        assert!(
            paid.test
                .as_ref()
                .unwrap()
                .committed
                .iter()
                .any(|committed| committed.card.id == card)
        );
        finish_current_test(&mut session);
        let finished = session.observe().play.unwrap();
        assert_eq!(finished.players[0].resources, final_resources);
        assert!(
            finished.players[0]
                .discard
                .iter()
                .any(|discarded| discarded.id == card)
        );
        assert_eq!(finished.completed_tests.len(), 1);
        if fixture == TonyEffectFixture::WatchSuccess {
            assert_eq!(
                finished.completed_tests[0].total, 3,
                "Margin1 pays the full six-resource reward for a three-resource wager"
            );
        }
        if fixture == TonyEffectFixture::WatchTie {
            assert!(finished.completed_tests[0].success);
            assert_eq!(finished.completed_tests[0].total, 2);
        }
        if fixture == TonyEffectFixture::WatchFailure {
            assert!(!finished.completed_tests[0].success);
        }
        session.validate().unwrap();
    }
}

#[test]
fn watch_this_cannot_be_committed_to_another_investigators_test() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::WatchSupport));
    let watch = tony_card(&session, "03233");
    let support = tony_card(&session, "60114");
    let enemy = session.observe().play.unwrap().scenario.enemies[0].card.id;
    choose(&mut session, &format!("action:ordinary:fight:{enemy}"));
    reach_tony_choice(&mut session, &format!("commit:{support}"));
    let choices = session.observe().choices;
    assert!(
        !choices
            .iter()
            .any(|choice| choice.id == format!("commit:{watch}"))
    );
    assert!(
        choices
            .iter()
            .any(|choice| choice.id == format!("commit:{support}")),
        "Same-location support is otherwise available"
    );
    session.validate().unwrap();
}
