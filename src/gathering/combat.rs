//! The selected Gathering's interrupted combat decisions.

use super::{
    Investigator, cards,
    runtime::{Frame, Game, TestOperation},
    scenario,
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum CombatDecision {
    LitaDamage { controller: Investigator },
    RetaliateComplete { investigator: Investigator },
    LitaAllySlot { controller: Investigator },
    DefeatReactions(DefeatWindow),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DefeatWindow {
    attacker: Investigator,
    horror: u8,
    monster: bool,
}

pub(super) fn decision_owner(decision: &CombatDecision) -> Investigator {
    match decision {
        CombatDecision::LitaDamage { controller } | CombatDecision::LitaAllySlot { controller } => {
            *controller
        }
        CombatDecision::RetaliateComplete { investigator } => *investigator,
        CombatDecision::DefeatReactions(window) => window.attacker,
    }
}

pub(super) fn queue_defeat_reactions(
    game: &mut Game,
    attacker: Investigator,
    enemy: &scenario::EnemyState,
) {
    // This Forced ability resolves before optional after-defeat reactions.
    // A terminal Priest objective never reaches this continuation.
    if attacker == Investigator::Tony {
        super::tony_cards::contracts_payout(game, attacker, enemy.bounties);
    }
    let (monster, horror) = scenario::enemy_definition(&enemy.card.code)
        .map(|definition| (definition.monster, definition.horror))
        .or_else(|| {
            cards::definition(&enemy.card.code).and_then(|definition| {
                definition
                    .enemy
                    .map(|values| (definition.has_trait("Monster"), values.attack_horror))
            })
        })
        .unwrap_or((false, 0));
    let window = DefeatWindow {
        attacker,
        horror,
        monster,
    };
    let decision = CombatDecision::DefeatReactions(window);
    if choices(game, &decision).len() > 1 {
        game.frames.push(Frame::Combat(decision));
    }
}

pub(super) fn excess_ally_slot(game: &Game, controller: Investigator) -> bool {
    game.players[controller.index()]
        .assets
        .iter()
        .filter_map(|asset| super::player_cards::asset_definition(&asset.card.code))
        .flat_map(|definition| definition.slots)
        .filter(|slot| **slot == cards::Slot::Ally)
        .count()
        > 1
}

fn choice(id: &str, label: &str) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        cancel_faces: vec![],
    }
}

pub(super) fn lita_damage_controller(
    game: &Game,
    attacker: Investigator,
    enemy: u16,
) -> Option<Investigator> {
    let controller = scenario::lita_controller(game)?;
    if game.players[controller.index()].location != game.players[attacker.index()].location {
        return None;
    }
    let enemy = game
        .scenario
        .enemies
        .iter()
        .find(|state| state.card.id == enemy)?;
    let monster = scenario::enemy_definition(&enemy.card.code)
        .map(|definition| definition.monster)
        .or_else(|| {
            cards::definition(&enemy.card.code).map(|definition| definition.has_trait("Monster"))
        })
        .unwrap_or(false);
    monster.then_some(controller)
}

pub(super) fn choices(game: &Game, decision: &CombatDecision) -> Vec<Choice> {
    match decision {
        CombatDecision::LitaDamage { controller } => {
            let mut choices = vec![choice(
                "lita:decline-damage",
                "Decline Lita Chantler's extra damage",
            )];
            if let Some(test) = &game.test
                && let TestOperation::Fight { enemy, .. } = test.operation
                && lita_damage_controller(game, test.tester, enemy) == Some(*controller)
            {
                choices.push(choice(
                    "lita:add-damage",
                    "Use Lita Chantler to deal 1 additional damage",
                ));
            }
            choices
        }
        CombatDecision::RetaliateComplete { .. } => vec![choice(
            "retaliate:complete",
            "Finish the Retaliate attack and resume the skill test",
        )],
        CombatDecision::LitaAllySlot { controller } => game.players[controller.index()]
            .assets
            .iter()
            .filter(|asset| {
                asset.card.code != "01117"
                    && super::player_cards::asset_definition(&asset.card.code)
                        .is_some_and(|definition| definition.slots.contains(&cards::Slot::Ally))
            })
            .map(|asset| {
                choice(
                    &format!("lita:discard-ally:{}", asset.card.id),
                    &format!(
                        "Discard {} to make room for Lita Chantler; afterward: Ally: 1/1",
                        asset.card.title
                    ),
                )
            })
            .collect(),
        CombatDecision::DefeatReactions(window) => {
            let mut choices = vec![choice(
                "defeat:done",
                "Finish reactions to the defeated enemy",
            )];
            let player = &game.players[window.attacker.index()];
            if player.defeated || player.resigned {
                return choices;
            }
            let can_heal = window.monster
                && window.horror > 0
                && player.resources >= 1
                && !player.threats.iter().any(|card| card.code == "01165")
                && game.players.iter().any(|other| {
                    !other.defeated
                        && !other.resigned
                        && other.location == player.location
                        && other.horror > 0
                });
            if can_heal {
                for card in player.hand.iter().filter(|card| card.code == "02225") {
                    choices.push(choice(&format!("defeat:bleeds:{}", card.id),
                        &format!("Play “If it bleeds…” for 1 resource; heal {} horror from each investigator here", window.horror)));
                }
            }
            choices
        }
    }
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Combat(decision)) = game.frames.last().cloned() else {
        return Err(RuleError("No combat decision is pending.".into()));
    };
    if !choices(game, &decision)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That combat choice is no longer available.".into(),
        ));
    }
    game.frames.pop();
    if let CombatDecision::DefeatReactions(window) = decision {
        if id == "defeat:done" {
            return Ok(());
        }
        {
            let player = &mut game.players[window.attacker.index()];
            let index = player
                .hand
                .iter()
                .position(|card| id == format!("defeat:bleeds:{}", card.id))
                .expect("Validated event");
            let card = player.hand.remove(index);
            player.resources -= 1;
            let location = player.location.clone();
            for investigator in &mut game.players {
                if !investigator.defeated
                    && !investigator.resigned
                    && investigator.location == location
                {
                    investigator.horror = investigator.horror.saturating_sub(window.horror);
                }
            }
            game.players[window.attacker.index()].discard.push(card);
        }
        game.frames
            .push(Frame::Combat(CombatDecision::DefeatReactions(window)));
        return Ok(());
    }
    if let CombatDecision::LitaAllySlot { controller } = decision {
        let assets = &mut game.players[controller.index()].assets;
        let index = assets
            .iter()
            .position(|asset| id == format!("lita:discard-ally:{}", asset.card.id))
            .expect("Validated ally");
        let card = assets.remove(index).card;
        if card.code == "01117" {
            scenario::remove_lita(game, card);
        } else {
            game.players[controller.index()].discard.push(card);
        }
        if excess_ally_slot(game, controller) {
            game.frames
                .push(Frame::Combat(CombatDecision::LitaAllySlot { controller }));
        }
        return Ok(());
    }
    if id == "lita:add-damage" {
        let test = game.test.as_mut().expect("Lita's pending attack");
        let TestOperation::Fight { damage, .. } = &mut test.operation else {
            unreachable!("Validated attack");
        };
        *damage = damage
            .checked_add(1)
            .ok_or_else(|| RuleError("Attack damage counter overflow.".into()))?;
    }
    game.finish_test()
}
