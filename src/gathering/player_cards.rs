//! Asset play through the shared Investigation action transaction.

use super::{
    CardView, Investigator,
    cards::{self, CardDefinition, CardKind, Slot},
    runtime::{AssetState, Game, PlayerState},
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PreparedAsset {
    pub actor: Investigator,
    pub card: CardView,
    pub resource_cost: u8,
    replacements: Vec<u16>,
}

pub(super) fn pay_asset(
    game: &mut Game,
    actor: Investigator,
    id: &str,
) -> Result<PreparedAsset, RuleError> {
    if !play_choices(game, actor)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError("That asset play is not currently legal.".into()));
    }
    let (play, replacements) = id.split_once(":replace:").unwrap_or((id, ""));
    let card_id: u16 = play
        .strip_prefix("play:")
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| RuleError("Invalid asset identity.".into()))?;
    let replacements = if replacements.is_empty() {
        vec![]
    } else {
        replacements
            .split(',')
            .map(|id| id.parse().expect("Advertised replacement"))
            .collect()
    };
    let player = &mut game.players[actor.index()];
    let index = player
        .hand
        .iter()
        .position(|card| card.id == card_id)
        .expect("Advertised hand card");
    let card = player.hand.remove(index);
    let resource_cost = cards::definition(&card.code)
        .and_then(|definition| definition.cost)
        .expect("Playable printed cost");
    player.resources -= resource_cost;
    game.history.push(format!("{} pays {resource_cost} resources for {}; the card waits in limbo until its play effect resolves. [RR Appendix I]", actor.name(), card.title));
    Ok(PreparedAsset {
        actor,
        card,
        resource_cost,
        replacements,
    })
}

pub(super) fn resolve_asset(
    game: &mut Game,
    mut prepared: PreparedAsset,
) -> Result<Option<PreparedAsset>, RuleError> {
    let definition = cards::definition(&prepared.card.code)
        .ok_or_else(|| RuleError("The paid asset has no selected definition.".into()))?;
    let player = &game.players[prepared.actor.index()];
    prepared
        .replacements
        .retain(|id| player.assets.iter().any(|asset| asset.card.id == *id));
    let legal = replacement_choices(player, definition);
    if legal == [Vec::<u16>::new()] {
        // Losing an asset to the intervening attack can make all replacements
        // unnecessary. Unneeded assets cannot be discarded just to play a card.
        prepared.replacements.clear();
    } else if !legal.contains(&prepared.replacements) {
        return Ok(Some(prepared));
    }
    let player = &mut game.players[prepared.actor.index()];
    let mut removed_lita = None;
    for id in prepared.replacements {
        let index = player
            .assets
            .iter()
            .position(|asset| asset.card.id == id)
            .expect("Revalidated replacement");
        let discarded = player.assets.remove(index).card;
        if discarded.code == "01117" {
            removed_lita = Some(discarded);
        } else {
            player.discard.push(discarded);
        }
    }
    game.history.push(format!("{} puts {} into play after its costs and any attacks of opportunity. [RR Play Action; Slots]", prepared.actor.name(), prepared.card.title));
    player.assets.push(AssetState {
        card: prepared.card,
        ready: true,
        uses: definition.uses.map_or(0, |(_, amount)| amount),
        damage: 0,
        horror: 0,
    });
    if let Some(card) = removed_lita {
        super::scenario::remove_lita(game, card);
    }
    Ok(None)
}

pub(super) fn placement_choices(game: &Game, prepared: &PreparedAsset) -> Vec<Choice> {
    let Some(definition) = cards::definition(&prepared.card.code) else {
        return vec![];
    };
    let player = &game.players[prepared.actor.index()];
    replacement_choices(player, definition)
        .into_iter()
        .map(|ids| {
            let identities = if ids.is_empty() {
                "none".into()
            } else {
                ids.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
            };
            let names = ids
                .iter()
                .map(|id| {
                    player
                        .assets
                        .iter()
                        .find(|asset| asset.card.id == *id)
                        .expect("Legal replacement")
                        .card
                        .title
                        .as_str()
                })
                .collect::<Vec<_>>()
                .join(", ");
            Choice {
                id: format!("asset:{}:replace:{identities}", prepared.card.id),
                label: if ids.is_empty() {
                    format!(
                        "Put {} into play without discarding an asset",
                        prepared.card.title
                    )
                } else {
                    format!(
                        "Discard {names} in that order to make room for {}",
                        prepared.card.title
                    )
                },
                cancel_faces: vec![],
            }
        })
        .collect()
}

pub(super) fn choose_placement(
    game: &mut Game,
    mut prepared: PreparedAsset,
    id: &str,
) -> Result<Option<PreparedAsset>, RuleError> {
    if !placement_choices(game, &prepared)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That asset replacement is not currently legal.".into(),
        ));
    }
    let ids = id.rsplit(':').next().expect("Validated placement");
    prepared.replacements = if ids == "none" {
        vec![]
    } else {
        ids.split(',')
            .map(|id| id.parse().expect("Validated replacement identity"))
            .collect()
    };
    resolve_asset(game, prepared)
}

/// Player assets and the original Gathering story ally share slot/soak rules.
/// Lita remains scenario content and is never inserted into a selected deck.
pub(super) fn asset_definition(code: &str) -> Option<&'static CardDefinition> {
    if code == "01117" {
        Some(super::scenario::lita_definition())
    } else {
        cards::definition(code)
    }
}

pub(super) fn play_choices(game: &Game, actor: Investigator) -> Vec<Choice> {
    let player = &game.players[actor.index()];
    if player.defeated || player.resigned || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    player
        .hand
        .iter()
        .flat_map(|card| {
            let Some(definition) = cards::definition(&card.code) else {
                return vec![];
            };
            let Some(cost) = definition.cost else {
                return vec![];
            };
            if definition.kind != CardKind::Asset
                || definition.permanent
                || definition.weakness
                // These play/entry operations have distinct unfinished timing
                // consumers. Never silently spend an action on a Fast asset or
                // skip a player's entry reaction.
                || matches!(card.code.as_str(), "05112" | "04037" | "06011" | "07029")
                || cost > player.resources
                || definition.unique
                    && game
                        .players
                        .iter()
                        .flat_map(|p| &p.assets)
                        .any(|asset| asset.card.title == card.title)
            {
                return vec![];
            }
            if definition.has_trait("Mask")
                && player.assets.iter().any(|asset| {
                    cards::definition(&asset.card.code)
                        .is_some_and(|existing| existing.has_trait("Mask"))
                })
            {
                return vec![];
            }
            replacement_choices(player, definition)
                .into_iter()
                .map(|discarded| {
                    let mut id = format!("play:{}", card.id);
                    let mut label = format!("Play {} for {cost} resources", card.title);
                    if !discarded.is_empty() {
                        id.push_str(":replace:");
                        id.push_str(
                            &discarded
                                .iter()
                                .map(u16::to_string)
                                .collect::<Vec<_>>()
                                .join(","),
                        );
                        let names: Vec<_> = discarded
                            .iter()
                            .map(|id| {
                                player
                                    .assets
                                    .iter()
                                    .find(|asset| asset.card.id == *id)
                                    .expect("Owned replacement")
                                    .card
                                    .title
                                    .as_str()
                            })
                            .collect();
                        label.push_str(&format!(
                            "; discard {} in that order to make room",
                            names.join(", ")
                        ));
                    }
                    Choice {
                        id,
                        label,
                        cancel_faces: vec![],
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn replacement_choices(player: &PlayerState, incoming: &CardDefinition) -> Vec<Vec<u16>> {
    if slots_fit(player, incoming, &[]) {
        return vec![vec![]];
    }
    // The selected inventory has a fixed, small number of occupied slots. All
    // legal minimal replacement sets are player choices; none is auto-selected.
    let mut subsets = vec![vec![]];
    for asset in &player.assets {
        let Some(definition) = asset_definition(&asset.card.code) else {
            continue;
        };
        if definition.permanent || definition.slots.is_empty() {
            continue;
        }
        let added: Vec<_> = subsets
            .iter()
            .map(|subset| {
                let mut next = subset.clone();
                next.push(asset.card.id);
                next
            })
            .collect();
        subsets.extend(added);
    }
    let mut choices = vec![];
    for subset in subsets {
        if !slots_fit(player, incoming, &subset) {
            continue;
        }
        if subset.iter().any(|id| {
            slots_fit(
                player,
                incoming,
                &subset
                    .iter()
                    .copied()
                    .filter(|other| other != id)
                    .collect::<Vec<_>>(),
            )
        }) {
            continue;
        }
        discard_orders(&subset, &mut vec![], &mut choices);
    }
    choices
}

fn discard_orders(remaining: &[u16], prefix: &mut Vec<u16>, orders: &mut Vec<Vec<u16>>) {
    if remaining.is_empty() {
        orders.push(prefix.clone());
        return;
    }
    for (index, id) in remaining.iter().enumerate() {
        prefix.push(*id);
        let rest: Vec<_> = remaining
            .iter()
            .enumerate()
            .filter_map(|(other, id)| (other != index).then_some(*id))
            .collect();
        discard_orders(&rest, prefix, orders);
        prefix.pop();
    }
}

fn slots_fit(player: &PlayerState, incoming: &CardDefinition, discarded: &[u16]) -> bool {
    let mut definitions: Vec<_> = player
        .assets
        .iter()
        .filter(|asset| !discarded.contains(&asset.card.id))
        .filter_map(|asset| asset_definition(&asset.card.code))
        .collect();
    definitions.push(incoming);
    let extra_arcane = definitions
        .iter()
        .filter(|definition| definition.code == "05112")
        .count();
    for slot in [
        Slot::Hand,
        Slot::Arcane,
        Slot::Ally,
        Slot::Accessory,
        Slot::Body,
    ] {
        let used = definitions
            .iter()
            .flat_map(|definition| definition.slots)
            .filter(|occupied| **occupied == slot)
            .count();
        let limit = match slot {
            Slot::Hand => 2,
            Slot::Arcane => 2 + extra_arcane,
            _ => 1,
        };
        if used > limit {
            return false;
        }
    }
    let unrestricted_arcane = definitions
        .iter()
        .filter(|definition| !definition.has_trait("Spell") && !definition.has_trait("Ritual"))
        .flat_map(|definition| definition.slots)
        .filter(|slot| **slot == Slot::Arcane)
        .count();
    unrestricted_arcane <= 2
}
