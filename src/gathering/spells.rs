//! The selected investigation, fight and evasion asset abilities.
//! Costs precede opportunity attacks; captured ability text survives its asset.

use super::{
    CardView, Investigator, cards,
    runtime::{Game, Skill, TestOperation},
    scenario,
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum SpellActivation {
    Test {
        investigator: Investigator,
        operation: TestOperation,
        skill: Skill,
        difficulty: i32,
        source: CardView,
    },
    VoiceRa {
        investigator: Investigator,
        event: CardView,
    },
}

fn choice(id: String, label: String) -> Choice {
    Choice {
        id,
        label,
        cancel_faces: vec![],
    }
}

pub(super) fn action_choices(game: &Game, who: Investigator) -> Vec<Choice> {
    let player = &game.players[who.index()];
    let mut choices = vec![];
    // Premonition's replacement scope for Voice's three-token request remains
    // unresolved (issue-89-token-compositions.md). Withhold it before action
    // payment and opportunity attacks; pay_action validates this same list.
    if game.premonition.is_none() && !player.threats.iter().any(|card| card.code == "01165") {
        for card in &player.hand {
            if card.code == "60416" {
                choices.push(choice(
                    format!("play:spell:{}", card.id),
                    "Play Voice of Ra".into(),
                ));
            }
        }
    }
    for asset in &player.assets {
        let code = asset.card.code.as_str();
        // Eyes still has a choose-one instruction at zero charges. Flattening
        // two Olive-sealed Premonition tokens into that choice would silently
        // choose an unresolved group interpretation (FAQ2.5 §2.5/p17). Keep
        // this composition unavailable before action payment; pay_action uses
        // this same list, so a forged activation cannot bypass the boundary.
        if code == "11067"
            && game
                .premonition
                .as_ref()
                .is_some_and(|sealed| sealed.faces.len() > 1)
        {
            continue;
        }
        if !matches!(code, "11067" | "60408" | "60407" | "04029" | "60507")
            || (code != "11067" && asset.uses == 0)
        {
            continue;
        }
        if matches!(code, "11067" | "60408" | "60507") {
            if game.scenario.shroud(&player.location).is_some() {
                choices.push(choice(
                    format!("investigate:spell:{}", asset.card.id),
                    format!(
                        "Investigate with {}{}",
                        asset.card.title,
                        if code == "60408" {
                            " (spend 1 charge)"
                        } else {
                            ""
                        }
                    ),
                ));
            }
        } else {
            for enemy in &game.scenario.enemies {
                if enemy.location != player.location {
                    continue;
                }
                let action = match code {
                    "60407" if enemy.card.code != "06012" || enemy.engaged.is_some() => "fight",
                    "04029" if enemy.engaged == Some(who) => "evade",
                    _ => continue,
                };
                choices.push(choice(
                    format!("{action}:{}:spell:{}", enemy.card.id, asset.card.id),
                    format!(
                        "{} {} with {} (spend 1 charge)",
                        if action == "fight" { "Fight" } else { "Evade" },
                        enemy.card.title,
                        asset.card.title
                    ),
                ));
            }
            for watcher in player.hand.iter().filter(|card| card.code == "06017") {
                let action = if code == "60407" { "fight" } else { "evade" };
                choices.push(choice(
                    format!("{action}:{}:spell:{}", watcher.id, asset.card.id),
                    format!(
                        "{} Watcher in your hand with {} (spend 1 charge)",
                        if action == "fight" { "Fight" } else { "Evade" },
                        asset.card.title
                    ),
                ));
            }
        }
    }
    choices
}

/// The shared action transaction invokes this before opportunity attacks. The
/// returned value owns the paid operation, not another copy in a card zone.
pub(super) fn pay_action(
    game: &mut Game,
    who: Investigator,
    action: &str,
) -> Result<SpellActivation, RuleError> {
    if !action_choices(game, who)
        .iter()
        .any(|choice| choice.id == action)
    {
        return Err(RuleError("That spell activation is unavailable.".into()));
    }
    let parts: Vec<_> = action.split(':').collect();
    if parts[0] == "play" {
        let id: u16 = parts[2].parse().expect("Offered event");
        let player = &mut game.players[who.index()];
        let index = player
            .hand
            .iter()
            .position(|card| card.id == id)
            .expect("Owned event");
        return Ok(SpellActivation::VoiceRa {
            investigator: who,
            event: player.hand.remove(index),
        });
    }
    let asset_id: u16 = parts
        .last()
        .expect("Spell identity")
        .parse()
        .expect("Offered spell");
    let player = &mut game.players[who.index()];
    let asset = player
        .assets
        .iter_mut()
        .find(|asset| asset.card.id == asset_id)
        .expect("Offered asset");
    let source = asset.card.clone();
    if !matches!(source.code.as_str(), "11067" | "60507") {
        asset.uses -= 1;
    }
    let (operation, difficulty) = if parts[0] == "investigate" {
        let location = player.location.clone();
        let difficulty = (i32::from(game.scenario.shroud(&location).expect("Offered location"))
            - if source.code == "60507" { 2 } else { 0 })
        .max(0);
        (
            TestOperation::Investigate {
                location,
                clues: if source.code == "60408" { 2 } else { 1 },
            },
            difficulty,
        )
    } else {
        let enemy_id: u16 = parts[1].parse().expect("Offered enemy");
        let enemy = game
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == enemy_id)
            .map(|enemy| &enemy.card)
            .or_else(|| {
                player
                    .hand
                    .iter()
                    .find(|card| card.id == enemy_id && card.code == "06017")
            })
            .expect("Offered enemy");
        let printed = scenario::enemy_definition(&enemy.code);
        let weakness = cards::definition(&enemy.code).and_then(|card| card.enemy);
        if parts[0] == "fight" {
            (
                TestOperation::Fight {
                    enemy: enemy_id,
                    weapon: Some(asset_id),
                    skill: Skill::Willpower,
                    skill_bonus: 0,
                    damage: 2,
                },
                i32::from(
                    printed
                        .map(|enemy| enemy.combat)
                        .or(weakness.map(|enemy| enemy.combat))
                        .expect("Enemy combat"),
                ),
            )
        } else {
            (
                TestOperation::Evade { enemy: enemy_id },
                i32::from(
                    printed
                        .map(|enemy| enemy.evade)
                        .or(weakness.map(|enemy| enemy.evade))
                        .expect("Enemy evade"),
                ),
            )
        }
    };
    Ok(SpellActivation::Test {
        investigator: who,
        operation,
        skill: if source.code == "60507" {
            Skill::Intellect
        } else {
            Skill::Willpower
        },
        difficulty,
        source,
    })
}

pub(super) fn resolve_action(game: &mut Game, paid: SpellActivation) -> Result<(), RuleError> {
    match paid {
        SpellActivation::Test {
            operation,
            investigator,
            skill,
            difficulty,
            source,
        } => {
            game.begin_test(operation, investigator, skill, difficulty)?;
            game.test.as_mut().expect("Spell test").source_card = Some(source);
            Ok(())
        }
        SpellActivation::VoiceRa {
            investigator,
            event,
        } => super::tokens::begin_voice_ra(game, investigator, event),
    }
}

pub(super) fn cancel_action(game: &mut Game, paid: SpellActivation) {
    if let SpellActivation::VoiceRa {
        investigator,
        event,
    } = paid
    {
        game.players[investigator.index()].removed.push(event);
    }
}

pub(super) struct Drawback {
    pub damage: u8,
    pub horror: u8,
    pub discard: bool,
    pub source: &'static str,
}

/// Legacy FAQ 2.5 p12 §2.5: one conditional card effect even when multiple
/// resolved tokens match. The p17 Shrivelling answer places it at ST.3.
pub(super) fn reveal_drawback(game: &Game, resolved: &[TokenFace]) -> Option<Drawback> {
    let source = game.test.as_ref()?.source_card.as_ref()?.code.as_str();
    let positive = resolved.iter().any(|face| {
        matches!(
            face,
            TokenFace::PlusOne | TokenFace::Zero | TokenFace::ElderSign
        )
    });
    match source {
        "60408" if positive => Some(Drawback {
            damage: 0,
            horror: 1,
            discard: false,
            source: "60408",
        }),
        "60407" if positive => Some(Drawback {
            damage: 1,
            horror: 0,
            discard: false,
            source: "60407",
        }),
        "04029"
            if resolved.iter().any(|face| {
                matches!(
                    face,
                    TokenFace::Skull | TokenFace::Cultist | TokenFace::Tablet | TokenFace::AutoFail
                )
            }) =>
        {
            Some(Drawback {
                damage: 0,
                horror: 0,
                discard: true,
                source: "04029",
            })
        }
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum SpellDecision {
    MistsDiscard(Investigator),
    MistsMove(Investigator),
}

pub(super) enum SpellStep {
    Done,
    Move {
        investigator: Investigator,
        location: String,
    },
}

pub(super) fn choices(game: &Game, decision: &SpellDecision) -> Vec<Choice> {
    match decision {
        SpellDecision::MistsDiscard(who) => game.players[who.index()]
            .hand
            .iter()
            .filter(|card| !card.weakness)
            .map(|card| {
                choice(
                    format!("spell-discard:{}", card.id),
                    format!("Discard {} for Mists of R'lyeh", card.title),
                )
            })
            .collect(),
        SpellDecision::MistsMove(who) => {
            let mut result = vec![choice(
                "spell-move:decline".into(),
                "Decline Mists of R'lyeh movement".into(),
            )];
            for location in &game.scenario.locations {
                if game
                    .scenario
                    .can_move(&game.players[who.index()].location, &location.code)
                {
                    result.push(choice(
                        format!("spell-move:{}", location.code),
                        format!("Move to {} after evading", location.code),
                    ));
                }
            }
            result
        }
    }
}

pub(super) fn choose(
    game: &mut Game,
    decision: SpellDecision,
    id: &str,
) -> Result<SpellStep, RuleError> {
    if !choices(game, &decision)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError("That spell consequence is unavailable.".into()));
    }
    match decision {
        SpellDecision::MistsDiscard(who) => {
            let id: u16 = id
                .strip_prefix("spell-discard:")
                .expect("Offered discard")
                .parse()
                .expect("Card identity");
            let player = &mut game.players[who.index()];
            let index = player
                .hand
                .iter()
                .position(|card| card.id == id)
                .expect("Owned hand card");
            player.discard.push(player.hand.remove(index));
            Ok(SpellStep::Done)
        }
        SpellDecision::MistsMove(investigator) if id != "spell-move:decline" => {
            Ok(SpellStep::Move {
                investigator,
                location: id
                    .strip_prefix("spell-move:")
                    .expect("Offered location")
                    .into(),
            })
        }
        SpellDecision::MistsMove(_) => Ok(SpellStep::Done),
    }
}
