//! Selected original Spell events use the shared paid-event and test lifecycle.

use super::{
    Investigator, cards,
    encounter_search::{EventEffect, PaidEvent, SearchDecision},
    runtime::{AssetView, Frame, Game, Skill, TestOperation},
    scenario,
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpellEventFixture {
    ReadSigns,
    ReadSignsOpportunity,
    Razor,
    RazorWatcher,
    RazorElite,
    UncageReadSigns,
    UncageRazorFrozen,
    UncageRazorStacked,
    UncageRazorShort,
    RazorTargets,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SpellEventEffect {
    operation: TestOperation,
    skill: Skill,
    difficulty: i32,
    engage: Option<u16>,
    granted: bool,
}

struct EventOption {
    choice: Choice,
    card: u16,
    robes: Option<u16>,
    effect: SpellEventEffect,
}

pub(super) fn is_action(action: &str) -> bool {
    matches!(action.split(':').nth(3), Some("06117" | "06201" | "06164"))
}

fn options(
    game: &Game,
    owner: Investigator,
    uncaged: bool,
    reserved_actions: u8,
) -> Vec<EventOption> {
    let player = &game.players[owner.index()];
    if player.defeated || player.resigned || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    let mut options = vec![];
    for card in &player.hand {
        if !matches!(card.code.as_str(), "06117" | "06201" | "06164") {
            continue;
        }
        if card.code == "06201" && super::ethereal_form::is_ethereal(game, owner) {
            continue;
        }
        let surcharge = if uncaged && matches!(card.code.as_str(), "06201" | "06164") {
            super::asset_entry::frozen_surcharge(game, owner)
        } else {
            0
        };
        if uncaged && player.ordinary_actions.saturating_sub(reserved_actions) < surcharge {
            continue;
        }
        let payments = if uncaged {
            vec![super::player_cards::SpellPayment {
                robes: None,
                cost: 0,
            }]
        } else {
            super::player_cards::spell_cost_choices(game, owner, &card.code)
        };
        let mut effects = vec![];
        if card.code == "06117" {
            if let Some(shroud) = game.scenario.shroud(&player.location) {
                effects.push((
                    String::new(),
                    SpellEventEffect {
                        operation: TestOperation::Investigate {
                            location: player.location.clone(),
                            clues: 2,
                        },
                        skill: Skill::Intellect,
                        difficulty: i32::from(shroud),
                        engage: None,
                        granted: uncaged,
                    },
                ));
            }
        } else if card.code == "06164" {
            for enemy in game
                .scenario
                .enemies
                .iter()
                .filter(|enemy| enemy.engaged == Some(owner))
            {
                let difficulty = scenario::enemy_definition(&enemy.card.code)
                    .map(|definition| definition.evade)
                    .or_else(|| {
                        cards::definition(&enemy.card.code)?
                            .enemy
                            .map(|values| values.evade)
                    })
                    .expect("Selected enemy evade");
                effects.push((
                    format!(":{}", enemy.card.id),
                    SpellEventEffect {
                        operation: TestOperation::Evade {
                            enemy: enemy.card.id,
                        },
                        skill: Skill::Agility,
                        difficulty: i32::from(difficulty),
                        engage: None,
                        granted: uncaged,
                    },
                ));
            }
            for watcher in player.hand.iter().filter(|card| card.code == "06017") {
                effects.push((
                    format!(":{}", watcher.id),
                    SpellEventEffect {
                        operation: TestOperation::Evade { enemy: watcher.id },
                        skill: Skill::Agility,
                        difficulty: 5,
                        engage: None,
                        granted: uncaged,
                    },
                ));
            }
        } else {
            for enemy in game
                .scenario
                .enemies
                .iter()
                .filter(|enemy| enemy.location == player.location)
            {
                let printed = scenario::enemy_definition(&enemy.card.code);
                let definition = cards::definition(&enemy.card.code);
                let combat = printed
                    .map(|enemy| enemy.combat)
                    .or_else(|| definition?.enemy.map(|enemy| enemy.combat))
                    .expect("Selected enemy");
                let elite = printed
                    .map(|enemy| enemy.elite)
                    .unwrap_or_else(|| definition.expect("Player enemy").has_trait("Elite"));
                for engage in [false, true] {
                    if engage && enemy.engaged == Some(owner) {
                        continue;
                    }
                    // An aloof enemy can only be attacked after this optional engagement.
                    if !engage && enemy.card.code == "06012" && enemy.engaged.is_none() {
                        continue;
                    }
                    effects.push((
                        format!(
                            ":{}:{}",
                            enemy.card.id,
                            if engage { "engage" } else { "stay" }
                        ),
                        SpellEventEffect {
                            operation: TestOperation::Fight {
                                enemy: enemy.card.id,
                                weapon: None,
                                skill: Skill::Combat,
                                skill_bonus: 0,
                                damage: if elite { 2 } else { 3 },
                            },
                            skill: Skill::Combat,
                            difficulty: i32::from(combat),
                            engage: engage.then_some(enemy.card.id),
                            granted: uncaged,
                        },
                    ));
                }
            }
            for watcher in player.hand.iter().filter(|card| card.code == "06017") {
                effects.push((
                    format!(":{}:stay", watcher.id),
                    SpellEventEffect {
                        operation: TestOperation::Fight {
                            enemy: watcher.id,
                            weapon: None,
                            skill: Skill::Combat,
                            skill_bonus: 0,
                            damage: 3,
                        },
                        skill: Skill::Combat,
                        difficulty: 5,
                        engage: None,
                        granted: uncaged,
                    },
                ));
            }
        }
        for payment in payments {
            for (suffix, effect) in &effects {
                let target = if let TestOperation::Fight { enemy, .. } = effect.operation {
                    if let Some(target) = game
                        .scenario
                        .enemies
                        .iter()
                        .find(|target| target.card.id == enemy)
                    {
                        format!(
                            "; attack {} (copy {enemy}){}",
                            target.card.title,
                            if effect.engage.is_some() {
                                ", engage it before the attack"
                            } else {
                                ""
                            }
                        )
                    } else {
                        format!(
                            "; attack Watcher from Another Dimension in your hand (copy {enemy})"
                        )
                    }
                } else if let TestOperation::Evade { enemy } = effect.operation {
                    let title = game
                        .scenario
                        .enemies
                        .iter()
                        .find(|target| target.card.id == enemy)
                        .map(|target| target.card.title.as_str())
                        .unwrap_or("Watcher from Another Dimension in your hand");
                    format!("; evade {title} (copy {enemy})")
                } else {
                    String::new()
                };
                let additional = if surcharge > 0 {
                    format!("; pay {surcharge} additional actions for Frozen in Fear")
                } else {
                    String::new()
                };
                let mut id = format!(
                    "{}:{}:{}{suffix}",
                    if uncaged { "uncage:event" } else { "play:tony" },
                    card.id,
                    card.code
                );
                if let Some(robes) = payment.robes {
                    id.push_str(&format!(":robes:{robes}"));
                }
                options.push(EventOption {
                    choice: Choice {
                        id,
                        label: format!(
                            "Play {} for {} resources{}{target}{additional}",
                            card.title,
                            payment.cost,
                            if payment.robes.is_some() {
                                " (exhaust Robes)"
                            } else {
                                ""
                            }
                        ),
                        cancel_faces: vec![],
                    },
                    card: card.id,
                    robes: payment.robes,
                    effect: effect.clone(),
                });
            }
        }
    }
    options
}

pub(super) fn action_choices(game: &Game, owner: Investigator) -> Vec<Choice> {
    options(game, owner, false, 0)
        .into_iter()
        .map(|option| option.choice)
        .collect()
}

pub(super) fn uncage_choices(game: &Game, owner: Investigator) -> Vec<Choice> {
    options(game, owner, true, 0)
        .into_iter()
        .map(|option| option.choice)
        .collect()
}

pub(super) fn uncage_choices_after_play_cost(game: &Game, owner: Investigator) -> Vec<Choice> {
    // The selected owners initiate this non-Fast Spirit event with an ordinary
    // Play action; reserve it when checking that its nested effect is playable.
    options(game, owner, true, 1)
        .into_iter()
        .map(|option| option.choice)
        .collect()
}

pub(super) fn pay_action(
    game: &mut Game,
    owner: Investigator,
    action: &str,
    uncaged: bool,
) -> Result<PaidEvent, RuleError> {
    let option = options(game, owner, uncaged, 0)
        .into_iter()
        .find(|option| option.choice.id == action)
        .ok_or_else(|| RuleError("That Spell event target or payment is unavailable.".into()))?;
    let code = game.players[owner.index()]
        .hand
        .iter()
        .find(|card| card.id == option.card)
        .expect("Offered event")
        .code
        .clone();
    if !uncaged {
        super::player_cards::pay_spell_cost(game, owner, &code, option.robes)?;
    } else if matches!(code.as_str(), "06201" | "06164") {
        super::asset_entry::pay_frozen_surcharge(game, owner, "Uncage's fight or evade event")?;
    }
    let player = &mut game.players[owner.index()];
    let at = player
        .hand
        .iter()
        .position(|card| card.id == option.card)
        .expect("Offered event");
    Ok(PaidEvent {
        owner,
        card: player.hand.remove(at),
        effect: EventEffect::Spell(option.effect),
    })
}

pub(super) fn resolve_action(
    game: &mut Game,
    event: PaidEvent,
    effect: SpellEventEffect,
) -> Result<(), RuleError> {
    if let Some(enemy) = effect.engage {
        game.scenario
            .enemies
            .iter_mut()
            .find(|target| target.card.id == enemy)
            .expect("Attacked enemy")
            .engaged = Some(event.owner);
    }
    let owner = event.owner;
    let source = event.card.clone();
    game.frames
        .push(Frame::EncounterSearch(SearchDecision::FinishEvent {
            event,
        }));
    game.begin_test(effect.operation, owner, effect.skill, effect.difficulty)?;
    game.test.as_mut().expect("Spell event test").source_card = Some(source);
    if effect.granted {
        game.test
            .as_mut()
            .expect("Nested event test")
            .completed_granted_action = Some(owner);
    }
    Ok(())
}

pub(super) fn configure_fixture(game: &mut Game, case: SpellEventFixture) {
    if matches!(
        case,
        SpellEventFixture::UncageRazorFrozen
            | SpellEventFixture::UncageRazorStacked
            | SpellEventFixture::UncageRazorShort
    ) {
        configure_fixture(game, SpellEventFixture::RazorElite);
        let player = &mut game.players[Investigator::Jacqueline.index()];
        let at = player
            .deck
            .iter()
            .position(|card| card.code == "03033")
            .expect("Selected Uncage");
        player.hand.push(player.deck.remove(at));
        let at = player
            .deck
            .iter()
            .position(|card| card.code == "06201")
            .expect("Second actual Razor");
        player.hand.push(player.deck.remove(at));
        for _ in 0..if case == SpellEventFixture::UncageRazorStacked {
            2
        } else {
            1
        } {
            let at = game
                .encounter_deck
                .iter()
                .position(|card| card.code == "01164")
                .expect("Actual Frozen in Fear");
            player.threats.push(game.encounter_deck.remove(at));
        }
        if case == SpellEventFixture::UncageRazorShort {
            player.ordinary_actions = 1;
        }
        return;
    }
    if case == SpellEventFixture::RazorTargets {
        configure_fixture(game, SpellEventFixture::RazorWatcher);
        for code in ["01160", "01160", "01159"] {
            let at = game
                .encounter_deck
                .iter()
                .position(|card| card.code == code)
                .expect("Actual encounter target");
            game.scenario.enemies.push(scenario::EnemyState {
                card: game.encounter_deck.remove(at),
                location: game.players[1].location.clone(),
                engaged: Some(Investigator::Patrice),
                exhausted: false,
                damage: 0,
                doom: 0,
                bounties: 0,
                owner: None,
            });
        }
        return;
    }
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        } else {
            panic!("Base encounter fixture");
        }
    }
    let who = if case == SpellEventFixture::RazorWatcher {
        Investigator::Patrice
    } else {
        Investigator::Jacqueline
    };
    let player = &mut game.players[who.index()];
    player.deck.append(&mut player.hand);
    player.location = "01112".into();
    player.ordinary_actions = 3;
    player.completed_actions_this_phase = 0;
    player.actions_taken_this_turn = 0;
    let event = if matches!(
        case,
        SpellEventFixture::ReadSigns
            | SpellEventFixture::ReadSignsOpportunity
            | SpellEventFixture::UncageReadSigns
    ) {
        "06117"
    } else {
        "06201"
    };
    let at = player
        .deck
        .iter()
        .position(|card| card.code == event)
        .expect("Selected Spell event");
    player.hand.push(player.deck.remove(at));
    if case == SpellEventFixture::UncageReadSigns {
        let at = player
            .deck
            .iter()
            .position(|card| card.code == "03033")
            .expect("Selected Uncage");
        player.hand.push(player.deck.remove(at));
        player.resources = 0;
    }
    if who == Investigator::Jacqueline {
        for code in ["60411", "60412"] {
            let at = player
                .deck
                .iter()
                .position(|card| card.code == code)
                .expect("Selected asset");
            player.assets.push(AssetView {
                card: player.deck.remove(at),
                ready: true,
                uses: 0,
                damage: 0,
                horror: 0,
            });
        }
    }
    if case == SpellEventFixture::RazorWatcher {
        let at = player
            .deck
            .iter()
            .position(|card| card.code == "06017")
            .expect("Actual Watcher");
        player.hand.push(player.deck.remove(at));
    } else if matches!(
        case,
        SpellEventFixture::Razor
            | SpellEventFixture::RazorElite
            | SpellEventFixture::ReadSignsOpportunity
    ) {
        let code = if case == SpellEventFixture::RazorElite {
            "01116"
        } else {
            "01160"
        };
        let card = if case == SpellEventFixture::RazorElite {
            let id = scenario::advance_act_two(game)
                .expect("Original Barrier places the set-aside Priest");
            let at = game
                .scenario
                .enemies
                .iter()
                .position(|enemy| enemy.card.id == id)
                .expect("Spawned Priest");
            game.scenario.enemies.remove(at).card
        } else {
            let at = game
                .encounter_deck
                .iter()
                .position(|card| card.code == code)
                .expect("Actual Ghoul Minion");
            game.encounter_deck.remove(at)
        };
        game.scenario.enemies.push(scenario::EnemyState {
            card,
            location: "01112".into(),
            engaged: if case == SpellEventFixture::ReadSignsOpportunity {
                Some(who)
            } else {
                Some(Investigator::Tony)
            },
            exhausted: false,
            damage: 0,
            doom: 0,
            bounties: 0,
            owner: None,
        });
        if case != SpellEventFixture::ReadSignsOpportunity {
            game.players[Investigator::Tony.index()].location = "01112".into();
        }
    }
    // Attic is revealed in the source-derived base fixture and has shroud 1.
    if matches!(
        case,
        SpellEventFixture::ReadSigns
            | SpellEventFixture::ReadSignsOpportunity
            | SpellEventFixture::UncageReadSigns
    ) {
        game.players[who.index()].location = "01113".into();
        for enemy in &mut game.scenario.enemies {
            enemy.location = "01113".into();
        }
    }
    game.enable_fixture_turn(who);
    game.set_fixture_tokens(vec![TokenFace::Zero]);
    game.history.clear();
    game.history.push(format!("Privileged {case:?} witness uses the actual selected event, paid asset state, revealed location and actual enemy identity. This is not ordinary play."));
}
