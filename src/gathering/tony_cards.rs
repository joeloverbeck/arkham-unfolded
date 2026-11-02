//! Tony's selected cards, as concrete operations on the shared Gathering game.
//!
//! Source: the bound original English identities and FAQ 2.5 / taboo 10 in
//! `docs/research/issue-89-rules-scenario-tony.md`. The runtime owns action
//! payment, timing order, test resolution, and terminal scenario interruption.

use super::{
    Investigator, cards,
    runtime::{Frame, Game, Skill, TestOperation},
    scenario::{enemy_definition, enemy_health},
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

const CONTRACTS: &str = "06010";
const LONG_COLT: &str = "06011";
const SOUR_MASH: &str = "05117";
const MAUSER: &str = "60306";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum TonyDecision {
    /// Created after forced enemy-entry consequences have completed.
    Bounties { enemy: u16 },
    /// Part of the successful Mauser attack's effect, before after-test abilities.
    MauserReward { owner: Investigator, asset: u16 },
}

fn choice(id: String, label: String) -> Choice {
    Choice {
        id,
        label,
        cancel_faces: vec![],
    }
}

fn bounty_capacity(game: &Game, enemy_id: u16) -> Option<u8> {
    let tony = &game.players[Investigator::Tony.index()];
    if tony.defeated || tony.resigned {
        return None;
    }
    let contracts = tony
        .assets
        .iter()
        .find(|asset| asset.card.code == CONTRACTS)?;
    let enemy = game
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == enemy_id)?;
    // The cap is the enemy's health value; damage does not lower that value.
    Some(
        3.min(contracts.uses)
            .min(enemy_health(&enemy.card.code)?.saturating_sub(enemy.bounties)),
    )
}

pub(super) fn bounty_choices(game: &Game, enemy_id: u16) -> Vec<Choice> {
    let Some(capacity) = bounty_capacity(game, enemy_id) else {
        return vec![];
    };
    let mut choices = vec![choice(
        format!("bounty:{enemy_id}:0"),
        "Decline Bounty Contracts".into(),
    )];
    choices.extend((1..=capacity).map(|amount| {
        choice(
            format!("bounty:{enemy_id}:{amount}"),
            format!(
                "Move {amount} {} from Bounty Contracts to the enemy",
                if amount == 1 { "bounty" } else { "bounties" }
            ),
        )
    }));
    choices
}

pub(super) fn choices(game: &Game, decision: &TonyDecision) -> Vec<Choice> {
    match *decision {
        TonyDecision::Bounties { enemy } => bounty_choices(game, enemy),
        TonyDecision::MauserReward { owner, asset } => {
            let player = &game.players[owner.index()];
            let mut choices = vec![choice(
                format!("mauser:{asset}:resource"),
                "Gain 1 resource from Mauser C96".into(),
            )];
            if player.assets.iter().any(|weapon| {
                weapon.card.id == asset && weapon.card.code == MAUSER && !weapon.ready
            }) {
                choices.push(choice(
                    format!("mauser:{asset}:ready"),
                    "Ready Mauser C96".into(),
                ));
            }
            choices
        }
    }
}

/// Consumes one actual pending opportunity, so repeated or stale reactions fail.
pub(super) fn apply(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Tony(decision)) = game.frames.last() else {
        return Err(RuleError("No Tony card decision is pending.".into()));
    };
    if !choices(game, decision).iter().any(|choice| choice.id == id) {
        return Err(RuleError(
            "That Tony card choice is not currently legal.".into(),
        ));
    }
    match *decision {
        TonyDecision::Bounties { enemy } => {
            let amount = id
                .rsplit(':')
                .next()
                .and_then(|amount| amount.parse().ok())
                .ok_or_else(|| RuleError("Invalid bounty amount.".into()))?;
            apply_bounties(game, enemy, amount)?;
        }
        TonyDecision::MauserReward { owner, asset } => {
            let player = &mut game.players[owner.index()];
            if id.ends_with(":resource") {
                player.resources += 1;
                game.history
                    .push("Mauser C96 succeeded by at least 2: gain 1 resource. [60306]".into());
            } else {
                player
                    .assets
                    .iter_mut()
                    .find(|weapon| weapon.card.id == asset)
                    .expect("Legal ready choice")
                    .ready = true;
                game.history
                    .push("Mauser C96 succeeded by at least 2: ready it. [60306]".into());
            }
        }
    }
    game.frames.pop();
    Ok(())
}

fn apply_bounties(game: &mut Game, enemy_id: u16, amount: u8) -> Result<(), RuleError> {
    let capacity = bounty_capacity(game, enemy_id).ok_or_else(|| {
        RuleError("Bounty Contracts has no eligible enemy-entry opportunity.".into())
    })?;
    if amount > capacity {
        return Err(RuleError(
            "That placement exceeds the bounty supply or enemy health cap.".into(),
        ));
    }
    if amount != 0 {
        let contracts = game.players[Investigator::Tony.index()]
            .assets
            .iter_mut()
            .find(|asset| asset.card.code == CONTRACTS)
            .expect("Validated Contracts");
        contracts.uses -= amount;
        game.scenario
            .enemies
            .iter_mut()
            .find(|enemy| enemy.card.id == enemy_id)
            .expect("Validated enemy")
            .bounties += amount;
        game.history.push(format!(
            "Move {amount} bounties from Bounty Contracts to enemy {enemy_id}. [06010]"
        ));
    }
    Ok(())
}

/// Validate the source and target before the runtime offers action-payment
/// choices. Paying an ordinary/restricted/granted action remains its decision.
pub(super) fn fight_operation(
    game: &Game,
    actor: Investigator,
    asset_id: u16,
    enemy_id: u16,
) -> Result<TestOperation, RuleError> {
    let player = &game.players[actor.index()];
    if player.defeated || player.resigned || game.test.is_some() {
        return Err(RuleError(
            "This investigator cannot start an attack now.".into(),
        ));
    }
    let asset = player
        .assets
        .iter()
        .find(|asset| asset.card.id == asset_id)
        .ok_or_else(|| RuleError("That weapon is not controlled by the attacker.".into()))?;
    let enemy = game
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == enemy_id)
        .ok_or_else(|| RuleError("The attacked enemy is no longer in play.".into()))?;
    if enemy.location != player.location || (enemy.card.code == "06012" && enemy.engaged.is_none())
    {
        return Err(RuleError("The enemy is not a legal attack target.".into()));
    }
    let (skill_bonus, damage) = match asset.card.code.as_str() {
        LONG_COLT if asset.uses > 0 => (i32::from(enemy.bounties), 2),
        MAUSER if asset.uses > 0 && asset.ready => (1, 2),
        SOUR_MASH => (3, 1),
        _ => return Err(RuleError("That asset cannot initiate this attack.".into())),
    };
    Ok(TestOperation::Fight {
        enemy: enemy_id,
        weapon: Some(asset_id),
        skill: Skill::Combat,
        skill_bonus,
        damage,
    })
}

/// Called inside the runtime's validated, atomic action transaction. The
/// runtime has selected and paid the action and must resolve its test normally.
pub(super) fn activate_fight(
    game: &mut Game,
    actor: Investigator,
    asset_id: u16,
    enemy_id: u16,
) -> Result<(), RuleError> {
    let operation = fight_operation(game, actor, asset_id, enemy_id)?;
    let enemy = game
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == enemy_id)
        .expect("Validated enemy");
    let combat = enemy_definition(&enemy.card.code)
        .map(|definition| definition.combat)
        .or_else(|| {
            cards::definition(&enemy.card.code)
                .and_then(|definition| definition.enemy.map(|values| values.combat))
        })
        .ok_or_else(|| RuleError("No supported enemy fight value is available.".into()))?;
    game.begin_test(operation, actor, Skill::Combat, i32::from(combat))?;
    let player = &mut game.players[actor.index()];
    let index = player
        .assets
        .iter()
        .position(|asset| asset.card.id == asset_id)
        .expect("Validated weapon");
    match player.assets[index].card.code.as_str() {
        SOUR_MASH => {
            let asset = player.assets.remove(index);
            player.discard.push(asset.card);
        }
        MAUSER => {
            player.assets[index].uses -= 1;
            player.assets[index].ready = false;
        }
        LONG_COLT => {
            player.assets[index].uses -= 1;
        }
        _ => unreachable!("Validated weapon"),
    }
    Ok(())
}

/// This reward is part of the Long Colt attack effect. Call before opening
/// after-defeat reactions; a scenario-ending interrupt still belongs to core.
pub(super) fn long_colt_defeat_reward(
    game: &mut Game,
    actor: Investigator,
    weapon_code: &str,
    bounties: u8,
) {
    let player = &mut game.players[actor.index()];
    // The effect remembers its source even if the weapon leaves play during
    // the attack. The runtime supplies the source captured at initiation.
    if bounties > 0
        && weapon_code == LONG_COLT
        && let Some(contracts) = player
            .assets
            .iter_mut()
            .find(|asset| asset.card.code == CONTRACTS)
    {
        contracts.uses += 1;
        game.history.push("The Long Colt attack defeated a marked enemy: place 1 bounty on Bounty Contracts. [06011]".into());
    }
}

/// Forced after-defeat payout. Core must not call after terminal resolution.
pub(super) fn contracts_payout(game: &mut Game, actor: Investigator, bounties: u8) {
    let player = &mut game.players[actor.index()];
    if bounties > 0
        && player
            .assets
            .iter()
            .any(|asset| asset.card.code == CONTRACTS)
    {
        player.resources += bounties;
        game.history.push(format!("Bounty Contracts moves {bounties} defeated-enemy bounties to {}'s resource pool. [06010]", actor.name()));
    }
}
