//! Finite entry reactions for the selected Sword Cane and Long Colts.

use super::{
    Investigator, cards,
    runtime::{Frame, Game, Skill, TestOperation},
    scenario,
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum AssetEntry {
    SwordCane { actor: Investigator, asset: u16 },
    LongColt { actor: Investigator, asset: u16 },
    Uncage { actor: Investigator },
}

impl AssetEntry {
    pub(super) fn actor(&self) -> Investigator {
        match self {
            Self::SwordCane { actor, .. }
            | Self::LongColt { actor, .. }
            | Self::Uncage { actor } => *actor,
        }
    }
}

fn choice(id: String, label: String) -> Choice {
    Choice {
        id,
        label,
        cancel_faces: vec![],
    }
}

pub(super) fn after_play(game: &mut Game, actor: Investigator, asset: u16) {
    let Some(card) = game.players[actor.index()]
        .assets
        .iter()
        .find(|owned| owned.card.id == asset)
    else {
        return;
    };
    let entry = match card.card.code.as_str() {
        "07029" => AssetEntry::SwordCane { actor, asset },
        "06011" => AssetEntry::LongColt { actor, asset },
        _ => return,
    };
    if entry_choices(game, &entry).len() > 1 {
        game.frames.push(Frame::AssetEntry(entry));
    }
}

pub(super) fn frozen_surcharge(game: &Game, actor: Investigator) -> u8 {
    let player = &game.players[actor.index()];
    if player.frozen_action_used {
        0
    } else {
        player
            .threats
            .iter()
            .filter(|card| card.code == "01164")
            .count() as u8
    }
}

pub(super) fn pay_frozen_surcharge(
    game: &mut Game,
    actor: Investigator,
    source: &str,
) -> Result<u8, RuleError> {
    let surcharge = frozen_surcharge(game, actor);
    let player = &mut game.players[actor.index()];
    player.ordinary_actions = player
        .ordinary_actions
        .checked_sub(surcharge)
        .ok_or_else(|| {
            RuleError("Not enough actions remain for Frozen in Fear's additional cost.".into())
        })?;
    if surcharge > 0 {
        player.frozen_action_used = true;
        game.history.push(format!("{} pays {surcharge} additional actions for Frozen in Fear during {source}; only the base action cost is waived. [01164; FAQ2.24]",actor.name()));
    }
    Ok(surcharge)
}

pub(super) fn entry_choices(game: &Game, entry: &AssetEntry) -> Vec<Choice> {
    match entry {
        AssetEntry::Uncage { actor } => super::player_cards::uncage_targets(game, *actor),
        AssetEntry::LongColt { actor, asset } => {
            let mut choices = vec![choice(
                format!("entry:{asset}:decline"),
                "Decline the paired Long Colt play".into(),
            )];
            choices.extend(
                super::player_cards::free_colt_choices(game, *actor)
                    .into_iter()
                    .map(|mut candidate| {
                        candidate.id = format!("entry:{asset}:{}", candidate.id);
                        candidate
                    }),
            );
            choices
        }
        AssetEntry::SwordCane { actor, asset } => {
            let mut choices = vec![choice(
                format!("entry:{asset}:decline"),
                "Decline Sword Cane's entry reaction".into(),
            )];
            let surcharge = frozen_surcharge(game, *actor);
            if game.players[actor.index()].ordinary_actions >= surcharge {
                choices.extend(cane_choices(game, *actor, *asset, true).into_iter().map(
                    |mut candidate| {
                        candidate.id = format!("entry:{asset}:{}", candidate.id);
                        candidate.label =
                            format!("Trigger without its activation costs: {}", candidate.label);
                        if surcharge > 0 {
                            candidate.label.push_str(&format!(
                                "; pay {surcharge} additional actions for Frozen in Fear"
                            ));
                        }
                        candidate
                    },
                ));
            }
            choices
        }
    }
}

pub(super) fn choose_entry(game: &mut Game, entry: AssetEntry, id: &str) -> Result<(), RuleError> {
    if !entry_choices(game, &entry)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That asset entry reaction is not currently legal.".into(),
        ));
    }
    match entry {
        AssetEntry::Uncage { actor } => super::player_cards::play_uncaged(game, actor, id),
        AssetEntry::LongColt { actor, asset } => {
            if id == format!("entry:{asset}:decline") {
                return Ok(());
            }
            super::player_cards::play_free_colt(
                game,
                actor,
                id.strip_prefix(&format!("entry:{asset}:"))
                    .expect("Validated entry"),
            )
        }
        AssetEntry::SwordCane { actor, asset } => {
            if id == format!("entry:{asset}:decline") {
                return Ok(());
            }
            pay_frozen_surcharge(game, actor, "Sword Cane's entry ability")?;
            activate_cane(
                game,
                actor,
                id.strip_prefix(&format!("entry:{asset}:"))
                    .expect("Validated entry"),
                true,
            )
        }
    }
}

fn cane_choices(game: &Game, actor: Investigator, asset_id: u16, free: bool) -> Vec<Choice> {
    let player = &game.players[actor.index()];
    if player.defeated || player.resigned || game.test.is_some() {
        return vec![];
    }
    if !player.assets.iter().any(|asset| {
        asset.card.id == asset_id && asset.card.code == "07029" && (asset.ready || free)
    }) {
        return vec![];
    }
    let mut choices = vec![];
    for enemy in &game.scenario.enemies {
        if enemy.location != player.location {
            continue;
        }
        if enemy.card.code != "06012" || enemy.engaged.is_some() {
            for skill in ["combat", "willpower"] {
                choices.push(choice(
                    format!("fight:cane:{asset_id}:{}:{skill}", enemy.card.id),
                    format!("Fight {} with Sword Cane using {skill}", enemy.card.title),
                ));
            }
        }
        if enemy.engaged == Some(actor) {
            for skill in ["agility", "willpower"] {
                choices.push(choice(
                    format!("evade:cane:{asset_id}:{}:{skill}", enemy.card.id),
                    format!("Evade {} with Sword Cane using {skill}", enemy.card.title),
                ));
            }
        }
    }
    choices
}

pub(super) fn asset_action_choices(game: &Game, actor: Investigator) -> Vec<Choice> {
    game.players[actor.index()]
        .assets
        .iter()
        .filter(|asset| asset.card.code == "07029")
        .flat_map(|asset| cane_choices(game, actor, asset.card.id, false))
        .collect()
}

pub(super) fn activate_asset_action(
    game: &mut Game,
    actor: Investigator,
    id: &str,
) -> Result<(), RuleError> {
    activate_cane(game, actor, id, false)
}

fn activate_cane(
    game: &mut Game,
    actor: Investigator,
    id: &str,
    free: bool,
) -> Result<(), RuleError> {
    let parts: Vec<_> = id.split(':').collect();
    let asset_id = parts
        .get(2)
        .and_then(|id| id.parse::<u16>().ok())
        .ok_or_else(|| RuleError("Invalid Sword Cane identity.".into()))?;
    if !cane_choices(game, actor, asset_id, free)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That Sword Cane ability is not currently legal.".into(),
        ));
    }
    let enemy_id = parts[3].parse::<u16>().expect("Validated enemy");
    let skill = match parts[4] {
        "willpower" => Skill::Willpower,
        "combat" => Skill::Combat,
        "agility" => Skill::Agility,
        _ => unreachable!("Validated skill"),
    };
    let enemy = game
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == enemy_id)
        .expect("Validated target");
    let (combat, evade) = scenario::enemy_definition(&enemy.card.code)
        .map(|enemy| (enemy.combat, enemy.evade))
        .or_else(|| {
            cards::definition(&enemy.card.code)
                .and_then(|card| card.enemy.map(|enemy| (enemy.combat, enemy.evade)))
        })
        .expect("Selected enemy values");
    let (operation, difficulty) = if parts[0] == "fight" {
        (
            TestOperation::Fight {
                enemy: enemy_id,
                weapon: Some(asset_id),
                skill,
                skill_bonus: 0,
                damage: 1,
            },
            combat,
        )
    } else {
        (TestOperation::Evade { enemy: enemy_id }, evade)
    };
    if !free {
        game.players[actor.index()]
            .assets
            .iter_mut()
            .find(|asset| asset.card.id == asset_id)
            .expect("Validated Cane")
            .ready = false;
    }
    game.begin_test(operation, actor, skill, i32::from(difficulty))?;
    if free {
        // The official-response FAQ on 01164 (2022-11-09) counts free bold
        // actions as performed; FAQ 2.19 distinguishes them from gaining an
        // action. Count at test cleanup, never while Take the Initiative applies.
        game.test
            .as_mut()
            .expect("Started Cane test")
            .completed_granted_action = Some(actor);
    }
    game.history.push(format!(
        "{} uses Sword Cane to {} using {}{}: no extra damage. [07029]",
        actor.name(),
        parts[0],
        parts[4],
        if free {
            " without paying its activation cost"
        } else {
            "; exhaust Sword Cane"
        }
    ));
    Ok(())
}
