//! The two selected encounter-search events, operating on the actual deck.
//!
//! Original 09068/03263 and RR p18: a search must choose an eligible card when
//! one exists; looking does not remove cards from their original game area.

use super::runtime::{Frame, Game, PendingRevelation, Phase, Skill, TestOperation};
use super::{CardView, Investigator, cards, scenario};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

const HORNET: &str = "09068";
const HUNT: &str = "03263";
const SCENE: &str = "04103";
const INTEL: &str = "05111";
const FAVOR: &str = "05277";
const GET_OVER_HERE: &str = "60114";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchFixture {
    HornetTargets,
    HornetNoMatch,
    HornetOpportunity,
    HuntTargets,
    HuntNoMatch,
    TonyEvents,
    GetOverHereFrozen,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PaidEvent {
    pub owner: Investigator,
    pub card: CardView,
    pub effect: EventEffect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum EventEffect {
    Search,
    Uncage,
    Spell(super::spell_events::SpellEventEffect),
    Survivor(super::player_events::PlayerEventEffect),
    Scene { location: String },
    Discover { location: String, clues: u8 },
    Damage { enemy: u16, damage: u8 },
    PullFight { enemy: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum SearchDecision {
    Recover {
        event: PaidEvent,
    },
    HuntOpportunity {
        owner: Investigator,
    },
    ChooseEnemy {
        event: PaidEvent,
        searched: Vec<u16>,
    },
    AfterSpawn {
        event: PaidEvent,
        health: u8,
    },
    FinishEvent {
        event: PaidEvent,
    },
}

fn choice(id: String, label: String) -> Choice {
    Choice {
        id,
        label,
        cancel_faces: vec![],
    }
}

pub(super) fn decision_owner(decision: &SearchDecision) -> Investigator {
    match decision {
        SearchDecision::HuntOpportunity { owner } => *owner,
        SearchDecision::ChooseEnemy { event, .. }
        | SearchDecision::Recover { event }
        | SearchDecision::AfterSpawn { event, .. }
        | SearchDecision::FinishEvent { event } => event.owner,
    }
}

pub(super) fn action_choices(game: &Game, actor: Investigator) -> Vec<Choice> {
    let player = &game.players[actor.index()];
    if player.defeated || player.resigned || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    let mut choices: Vec<_> = player
        .hand
        .iter()
        .filter(|card| card.code == HORNET)
        .map(|card| {
            choice(
                format!("play:tony:{}", card.id),
                format!("Play {}: search for a non-Elite enemy", card.title),
            )
        })
        .collect();
    choices.extend(
        event_options(game, actor, true)
            .into_iter()
            .map(|(choice, _, _)| choice),
    );
    choices.extend(super::spell_events::action_choices(game, actor));
    choices.extend(super::player_events::action_choices(game, actor));
    choices
}

/// These are selected printed action designators, never a parser for card text.
pub(super) fn action_designator(action: &str) -> Option<&'static str> {
    if !action.starts_with("play:tony:") {
        return None;
    }
    Some(match action.split(':').nth(3) {
        Some(GET_OVER_HERE | "06201") => "fight",
        Some("06117" | "04272") => "investigate",
        _ => "play",
    })
}
pub(super) fn action_enemy(action: &str) -> Option<u16> {
    matches!(action.split(':').nth(3), Some(GET_OVER_HERE | "06201"))
        .then(|| action.split(':').nth(4)?.parse().ok())
        .flatten()
}
pub(super) fn provokes_opportunity(action: &str) -> Option<bool> {
    action_designator(action).map(|_| {
        !matches!(
            action.split(':').nth(3),
            Some(SCENE | GET_OVER_HERE | "06201")
        )
    })
}

fn non_elite(enemy: &scenario::EnemyState) -> bool {
    scenario::enemy_definition(&enemy.card.code)
        .map(|enemy| !enemy.elite)
        .or_else(|| cards::definition(&enemy.card.code).map(|card| !card.has_trait("Elite")))
        .unwrap_or(false)
}

fn event_options(
    game: &Game,
    actor: Investigator,
    before_action_cost: bool,
) -> Vec<(Choice, u8, EventEffect)> {
    let player = &game.players[actor.index()];
    let mut options = Vec::new();
    for card in &player.hand {
        match card.code.as_str() {
            "03033"
                if if before_action_cost {
                    super::player_cards::can_play_uncage(game, actor)
                } else {
                    !super::player_cards::uncage_targets(game, actor).is_empty()
                } =>
            {
                options.push((choice(format!("play:tony:{}:03033", card.id), "Play Uncage the Soul: play a Spell or Ritual from hand, reducing its cost by 3".into()), 0, EventEffect::Uncage));
            }
            SCENE
                if player.resources >= 2
                    && game.is_actual_turn(actor)
                    && player.actions_taken_this_turn == 0
                    && game
                        .scenario
                        .location(&player.location)
                        .is_some_and(|location| location.clues > 0) =>
            {
                options.push((choice(format!("play:tony:{}:{SCENE}", card.id), "Play Scene of the Crime for 2 resources as your first action; no attack of opportunity".into()), 2, EventEffect::Scene { location: player.location.clone() }));
            }
            INTEL | FAVOR => {
                for amount_boost in [false, true] {
                    for range_boost in [false, true] {
                        let cost = 2 + 2 * u8::from(amount_boost) + 2 * u8::from(range_boost);
                        if player.resources < cost {
                            continue;
                        }
                        let amount = 1 + u8::from(amount_boost);
                        let range = if range_boost { 2 } else { 0 };
                        if card.code == INTEL {
                            for location in &game.scenario.locations {
                                if location.clues == 0
                                    || !game
                                        .scenario
                                        .distance(&player.location, &location.code)
                                        .is_some_and(|distance| distance <= range)
                                {
                                    continue;
                                }
                                options.push((choice(format!("play:tony:{}:{INTEL}:{}:{}:{}", card.id, location.code, u8::from(amount_boost), u8::from(range_boost)), format!("Play Intel Report for {cost} resources: discover {amount} clues at {}", super::content::title(&location.code).unwrap_or(&location.code))), cost, EventEffect::Discover { location: location.code.clone(), clues: amount }));
                            }
                        } else {
                            for enemy in &game.scenario.enemies {
                                if !non_elite(enemy)
                                    || !game
                                        .scenario
                                        .distance(&player.location, &enemy.location)
                                        .is_some_and(|distance| distance <= range)
                                {
                                    continue;
                                }
                                options.push((choice(format!("play:tony:{}:{FAVOR}:{}:{}:{}", card.id, enemy.card.id, u8::from(amount_boost), u8::from(range_boost)), format!("Play Small Favor for {cost} resources: deal {amount} damage to {}", enemy.card.title)), cost, EventEffect::Damage { enemy: enemy.card.id, damage: amount }));
                            }
                        }
                    }
                }
            }
            GET_OVER_HERE if player.resources >= 2 => {
                for enemy in &game.scenario.enemies {
                    if !non_elite(enemy)
                        || !game
                            .scenario
                            .distance(&player.location, &enemy.location)
                            .is_some_and(|distance| distance <= 1)
                    {
                        continue;
                    }
                    options.push((
                        choice(
                            format!("play:tony:{}:{GET_OVER_HERE}:{}", card.id, enemy.card.id),
                            format!(
                                "Play Get over here! for 2 resources: pull, engage and fight {}",
                                enemy.card.title
                            ),
                        ),
                        2,
                        EventEffect::PullFight {
                            enemy: enemy.card.id,
                        },
                    ));
                }
            }
            _ => {}
        }
    }
    options
}

pub(super) fn pay_action(
    game: &mut Game,
    owner: Investigator,
    action: &str,
) -> Result<PaidEvent, RuleError> {
    if super::player_events::is_action(action) {
        return super::player_events::pay_action(game, owner, action);
    }
    if super::spell_events::is_action(action) {
        return super::spell_events::pay_action(game, owner, action, false);
    }
    // The shared action transaction has already spent its action component.
    // Recheck nested target affordability without reserving that action again.
    let option = event_options(game, owner, false)
        .into_iter()
        .find(|(choice, _, _)| choice.id == action);
    if option.is_none()
        && !action_choices(game, owner)
            .iter()
            .any(|choice| choice.id == action)
    {
        return Err(RuleError(
            "That selected event cannot be played now.".into(),
        ));
    }
    let card_id: u16 = action
        .split(':')
        .nth(2)
        .and_then(|id| id.parse().ok())
        .expect("Validated event identity");
    let player = &mut game.players[owner.index()];
    let index = player
        .hand
        .iter()
        .position(|card| card_id == card.id)
        .expect("Validated event");
    let card = player.hand.remove(index);
    let (cost, effect) = option
        .map(|(_, cost, effect)| (cost, effect))
        .unwrap_or((0, EventEffect::Search));
    player.resources = player
        .resources
        .checked_sub(cost)
        .ok_or_else(|| RuleError("The event's resource cost is not affordable.".into()))?;
    Ok(PaidEvent {
        owner,
        card,
        effect,
    })
}

pub(super) fn cancel_action(game: &mut Game, event: PaidEvent) {
    game.players[event.owner.index()].removed.push(event.card);
}

pub(super) fn resolve_action(game: &mut Game, event: PaidEvent) -> Result<(), RuleError> {
    if game.players[event.owner.index()].defeated || game.players[event.owner.index()].resigned {
        cancel_action(game, event);
        return Ok(());
    }
    match event.effect.clone() {
        EventEffect::Survivor(effect) => {
            return super::player_events::resolve_action(game, event, effect);
        }
        EventEffect::Spell(effect) => {
            return super::spell_events::resolve_action(game, event, effect);
        }
        EventEffect::Search => {}
        EventEffect::Uncage => {
            let owner = event.owner;
            game.frames
                .push(Frame::EncounterSearch(SearchDecision::FinishEvent {
                    event,
                }));
            super::player_cards::begin_uncage(game, owner)?;
            return Ok(());
        }
        EventEffect::Scene { location } => {
            let clues = if game
                .scenario
                .enemies
                .iter()
                .any(|enemy| enemy.location == location)
            {
                2
            } else {
                1
            };
            discover(game, event.owner, &location, clues);
            game.players[event.owner.index()].discard.push(event.card);
            return Ok(());
        }
        EventEffect::Discover { location, clues } => {
            discover(game, event.owner, &location, clues);
            game.players[event.owner.index()].discard.push(event.card);
            return Ok(());
        }
        EventEffect::Damage { enemy, damage } => {
            let owner = event.owner;
            game.frames
                .push(Frame::EncounterSearch(SearchDecision::FinishEvent {
                    event,
                }));
            if let Some(target) = game
                .scenario
                .enemies
                .iter_mut()
                .find(|target| target.card.id == enemy)
            {
                target.damage += damage;
                if target.damage
                    >= scenario::enemy_health(&target.card.code).expect("Selected enemy health")
                {
                    let defeated = scenario::remove_defeated_enemy(game, enemy)?;
                    if defeated.ends_scenario {
                        game.suspend_for_resolution();
                        game.frames
                            .push(Frame::Scenario(scenario::ScenarioDecision::Ending));
                    } else {
                        super::combat::queue_defeat_reactions(game, owner, &defeated.enemy);
                    }
                }
            }
            return Ok(());
        }
        EventEffect::PullFight { enemy } => {
            let owner = event.owner;
            let target = game
                .scenario
                .enemies
                .iter_mut()
                .find(|target| target.card.id == enemy)
                .expect("Selected enemy");
            target.location = game.players[owner.index()].location.clone();
            target.engaged = Some(owner);
            let combat = scenario::enemy_definition(&target.card.code)
                .map(|enemy| enemy.combat)
                .or_else(|| {
                    cards::definition(&target.card.code)?
                        .enemy
                        .map(|enemy| enemy.combat)
                })
                .expect("Selected enemy combat");
            let source = event.card.clone();
            game.frames
                .push(Frame::EncounterSearch(SearchDecision::FinishEvent {
                    event,
                }));
            game.begin_test(
                TestOperation::Fight {
                    enemy,
                    weapon: None,
                    skill: Skill::Combat,
                    skill_bonus: 0,
                    damage: 1,
                },
                owner,
                Skill::Combat,
                i32::from(combat),
            )?;
            game.test.as_mut().expect("Fight test").source_card = Some(source);
            return Ok(());
        }
    }
    let searched = game
        .encounter_deck
        .iter()
        .rev()
        .take(9)
        .map(|card| card.id)
        .collect();
    game.frames
        .push(Frame::EncounterSearch(SearchDecision::ChooseEnemy {
            event,
            searched,
        }));
    Ok(())
}

fn discover(game: &mut Game, owner: Investigator, location: &str, clues: u8) {
    if let Some(location) = game.scenario.location_mut(location) {
        let discovered = clues.min(location.clues);
        location.clues -= discovered;
        game.players[owner.index()].clues += discovered;
    }
}

pub(super) fn visible_cards(game: &Game) -> Vec<CardView> {
    let Some(SearchDecision::ChooseEnemy { searched, .. }) =
        game.frames.iter().rev().find_map(|frame| match frame {
            Frame::EncounterSearch(decision) => Some(decision),
            _ => None,
        })
    else {
        return vec![];
    };
    searched
        .iter()
        .filter_map(|id| {
            game.encounter_deck
                .iter()
                .find(|card| card.id == *id)
                .cloned()
        })
        .collect()
}

fn eligible(card: &CardView, event: &PaidEvent) -> bool {
    scenario::enemy_definition(&card.code)
        .is_some_and(|enemy| event.card.code == HUNT || !enemy.elite)
}

pub(super) fn choices(game: &Game, decision: &SearchDecision) -> Vec<Choice> {
    match decision {
        SearchDecision::Recover { event } => super::player_events::recovery_choices(game, event),
        SearchDecision::HuntOpportunity { owner } => {
            let mut choices = vec![choice(
                "search:decline-hunt".into(),
                "Draw the encounter card normally".into(),
            )];
            choices.extend(hunt_choices(game, *owner));
            choices
        }
        SearchDecision::FinishEvent { event } => vec![choice(
            "search:finish-event".into(),
            format!("Finish resolving {}", event.card.title),
        )],
        SearchDecision::ChooseEnemy { event, searched } => {
            let choices: Vec<_> = searched
                .iter()
                .filter_map(|id| game.encounter_deck.iter().find(|card| card.id == *id))
                .filter(|card| eligible(card, event))
                .map(|card| {
                    choice(
                        format!("search:enemy:{}", card.id),
                        format!("Spawn {} engaged with you", card.title),
                    )
                })
                .collect();
            if choices.is_empty() {
                vec![choice(
                    "search:no-match".into(),
                    if event.card.code == HUNT {
                        "No enemy: shuffle, then draw an encounter card"
                    } else {
                        "No eligible enemy: shuffle the encounter deck; gain no clue or resources"
                    }
                    .into(),
                )]
            } else {
                choices
            }
        }
        SearchDecision::AfterSpawn { event, health } => vec![choice(
            "search:finish".into(),
            if event.card.code == HUNT {
                "Shuffle the encounter deck and finish On the Hunt".into()
            } else {
                format!("Discover 1 clue, gain {health} resources, and shuffle the encounter deck")
            },
        )],
    }
}

fn shuffle(game: &mut Game) {
    for index in (1..game.encounter_deck.len()).rev() {
        let other = game.random_index(index + 1);
        game.encounter_deck.swap(index, other);
    }
}

fn finish(game: &mut Game, event: PaidEvent) {
    shuffle(game);
    game.players[event.owner.index()].discard.push(event.card);
    scenario::rebuild_empty_encounter(game);
}

fn hunt_choices(game: &Game, owner: Investigator) -> Vec<Choice> {
    let player = &game.players[owner.index()];
    if game.phase != Phase::Mythos
        || player.defeated
        || player.resigned
        || player.resources < 1
        || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    player
        .hand
        .iter()
        .filter(|card| card.code == HUNT)
        .map(|card| {
            choice(
                format!("search:play-hunt:{}", card.id),
                "Play On the Hunt for 1 resource instead of drawing this encounter card".into(),
            )
        })
        .collect()
}

fn draw_encounter(game: &mut Game, owner: Investigator) {
    scenario::rebuild_empty_encounter(game);
    if let Some(card) = game.encounter_deck.pop() {
        game.frames.push(Frame::Revelation(PendingRevelation {
            card,
            drawn_by: owner,
            started: false,
            cancelled: false,
        }));
    }
}

/// The next investigator's framework continuation is already below this draw.
/// Fallback draws create a new would-draw occurrence, still during mythos.
pub(super) fn begin_mythos_draw(game: &mut Game, owner: Investigator) -> Result<(), RuleError> {
    if game.players[owner.index()].defeated || game.players[owner.index()].resigned {
        return Ok(());
    }
    if hunt_choices(game, owner).is_empty() {
        draw_encounter(game, owner);
    } else {
        game.frames
            .push(Frame::EncounterSearch(SearchDecision::HuntOpportunity {
                owner,
            }));
    }
    Ok(())
}

pub(super) fn eliminate(game: &mut Game, owner: Investigator) {
    for frame in std::mem::take(&mut game.frames) {
        match frame {
            Frame::EncounterSearch(decision) if decision_owner(&decision) == owner => {
                match decision {
                    SearchDecision::ChooseEnemy { event, .. }
                    | SearchDecision::Recover { event }
                    | SearchDecision::AfterSpawn { event, .. }
                    | SearchDecision::FinishEvent { event } => cancel_action(game, event),
                    SearchDecision::HuntOpportunity { .. } => {}
                }
            }
            other => game.frames.push(other),
        }
    }
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::EncounterSearch(decision)) = game.frames.last().cloned() else {
        return Err(RuleError("No encounter search is pending.".into()));
    };
    if !choices(game, &decision)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError("That search choice is not available.".into()));
    }
    game.frames.pop();
    match decision {
        SearchDecision::Recover { event } => super::player_events::recover(game, event, id)?,
        SearchDecision::HuntOpportunity { owner } => {
            if id == "search:decline-hunt" {
                draw_encounter(game, owner);
            } else {
                let player = &mut game.players[owner.index()];
                let index = player
                    .hand
                    .iter()
                    .position(|card| id == format!("search:play-hunt:{}", card.id))
                    .expect("Validated Hunt");
                let card = player.hand.remove(index);
                player.resources -= 1;
                resolve_action(
                    game,
                    PaidEvent {
                        owner,
                        card,
                        effect: EventEffect::Search,
                    },
                )?;
            }
        }
        SearchDecision::FinishEvent { event } => {
            super::player_events::finish_event(game, event);
        }
        SearchDecision::ChooseEnemy { event, .. } => {
            if id == "search:no-match" {
                if event.card.code == HUNT {
                    shuffle(game);
                    let owner = event.owner;
                    game.frames
                        .push(Frame::EncounterSearch(SearchDecision::FinishEvent {
                            event,
                        }));
                    begin_mythos_draw(game, owner)?;
                } else {
                    finish(game, event);
                }
                return Ok(());
            }
            let index = game
                .encounter_deck
                .iter()
                .position(|card| id == format!("search:enemy:{}", card.id))
                .expect("Validated target");
            let card = game.encounter_deck.remove(index);
            let enemy = card.id;
            let health = scenario::enemy_health(&card.code).expect("Selected enemy health");
            game.scenario.enemies.push(scenario::EnemyState {
                card,
                location: game.players[event.owner.index()].location.clone(),
                engaged: Some(event.owner),
                exhausted: false,
                damage: 0,
                doom: 0,
                bounties: 0,
                owner: None,
            });
            game.frames
                .push(Frame::EncounterSearch(SearchDecision::AfterSpawn {
                    event,
                    health,
                }));
            game.scenario_step(scenario::ScenarioStep::EnemyEntered(enemy))?;
        }
        SearchDecision::AfterSpawn { event, health } => {
            if event.card.code == HUNT {
                finish(game, event);
                return Ok(());
            }
            let player = &mut game.players[event.owner.index()];
            let location = game
                .scenario
                .location_mut(&player.location)
                .expect("Active investigator location");
            let clues = location.clues.min(1);
            location.clues -= clues;
            player.clues += clues;
            player.resources += health;
            finish(game, event);
        }
    }
    Ok(())
}

pub(super) fn configure_fixture(game: &mut Game, fixture: SearchFixture) -> Result<(), RuleError> {
    for frame in std::mem::take(&mut game.frames) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    game.scenario = scenario::ScenarioState::new();
    game.act = 1;
    game.lead = Investigator::Tony;
    game.agenda = 1;
    game.agenda_doom = 0;
    game.phase = Phase::Investigation;
    game.actor = Some(Investigator::Tony);
    game.enable_fixture_turn(Investigator::Tony);
    game.test = None;
    for player in &mut game.players {
        player.location = scenario::STUDY.into();
    }
    if matches!(
        fixture,
        SearchFixture::TonyEvents | SearchFixture::GetOverHereFrozen
    ) {
        return configure_event_fixture(game, fixture == SearchFixture::GetOverHereFrozen);
    }
    let tony = &mut game.players[Investigator::Tony.index()];
    tony.resources = 4;
    let hunt = matches!(
        fixture,
        SearchFixture::HuntTargets | SearchFixture::HuntNoMatch
    );
    let event_code = if hunt { HUNT } else { HORNET };
    let index = tony
        .deck
        .iter()
        .position(|card| card.code == event_code)
        .expect("Fixed selected search event");
    tony.hand.push(tony.deck.remove(index));
    if fixture == SearchFixture::HornetOpportunity {
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == "01160")
            .expect("Actual Minion");
        let card = game.encounter_deck.remove(index);
        scenario::spawn_encounter_enemy(game, card, Investigator::Tony)?;
    }
    let codes: &[&str] = if matches!(
        fixture,
        SearchFixture::HornetNoMatch | SearchFixture::HuntNoMatch
    ) {
        &[
            "01163", "01163", "01163", "01164", "01164", "01165", "01165", "01167", "01168",
        ]
    } else {
        &[
            "01118", "01119", "01159", "01160", "01161", "01163", "01164", "01165", "01167",
        ]
    };
    let mut top = Vec::new();
    for code in codes {
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == *code)
            .expect("Actual fixed encounter copy");
        top.push(game.encounter_deck.remove(index));
    }
    top.reverse();
    game.encounter_deck.append(&mut top);
    if hunt {
        game.phase = Phase::Mythos;
        game.actor = None;
        game.frames
            .push(Frame::Phase(super::phases::PhaseDecision::MythosDraw {
                order: scenario::player_order(game),
                index: 0,
            }));
    }
    Ok(())
}

fn configure_event_fixture(game: &mut Game, frozen: bool) -> Result<(), RuleError> {
    fn encounter(game: &mut Game, code: &str) -> CardView {
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == code)
            .expect("Fixed encounter copy");
        game.encounter_deck.remove(index)
    }
    game.scenario = scenario::ScenarioState::gathering_after_act_one();
    game.scenario.reveal(scenario::ATTIC);
    game.scenario.reveal(scenario::CELLAR);
    game.act = 2;
    for (who, location) in [
        (Investigator::Tony, scenario::ATTIC),
        (Investigator::Patrice, scenario::HALLWAY),
        (Investigator::Jacqueline, scenario::CELLAR),
    ] {
        game.players[who.index()].location = location.into();
    }
    let tony = &mut game.players[Investigator::Tony.index()];
    tony.resources = 8;
    for code in [SCENE, INTEL, FAVOR, GET_OVER_HERE] {
        if tony.hand.iter().any(|card| card.code == code) {
            continue;
        }
        let index = tony
            .deck
            .iter()
            .position(|card| card.code == code)
            .expect("Selected Tony event");
        tony.hand.push(tony.deck.remove(index));
    }
    tony.assets
        .iter_mut()
        .find(|asset| asset.card.code == "06010")
        .expect("Contracts")
        .uses = 3;
    for (code, who, damage, bounties) in [
        ("01159", Investigator::Tony, 0, 0),
        ("01160", Investigator::Patrice, 0, 1),
        ("01161", Investigator::Jacqueline, 1, 2),
    ] {
        let card = encounter(game, code);
        let id = card.id;
        scenario::spawn_encounter_enemy(game, card, who)?;
        let enemy = game
            .scenario
            .enemies
            .iter_mut()
            .find(|enemy| enemy.card.id == id)
            .expect("Spawned enemy");
        enemy.damage = damage;
        enemy.bounties = bounties;
    }
    let fog = encounter(game, "01168");
    game.scenario
        .location_mut(scenario::CELLAR)
        .expect("Cellar")
        .fog = Some(fog);
    if frozen {
        let card = encounter(game, "01164");
        game.players[Investigator::Tony.index()].threats.push(card);
    }
    Ok(())
}
