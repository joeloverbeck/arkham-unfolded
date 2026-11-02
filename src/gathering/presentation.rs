//! Player-visible pending consequences, projected from the runtime's typed frames.

use super::{
    CardView, Investigator,
    encounter_search::SearchDecision,
    runtime::{Frame, Game},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingView {
    pub harm: Option<HarmView>,
    pub discard: Option<DiscardView>,
    pub resolving_cards: Vec<CardView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarmView {
    pub investigator: Investigator,
    pub source: String,
    pub damage: u8,
    pub horror: u8,
    pub direct: bool,
    pub allocations: Vec<HarmAllocation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarmAllocation {
    /// None assigns to the investigator; Some identifies the physical asset.
    pub asset: Option<CardView>,
    pub damage: u8,
    pub horror: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscardView {
    pub owner: Investigator,
    pub remaining: Vec<CardView>,
    pub ordered: Vec<CardView>,
}

pub(super) fn observe(game: &Game) -> PendingView {
    let harm = game.frames.iter().rev().find_map(|frame| match frame {
        Frame::Harm(harm) => Some(HarmView {
            investigator: harm.investigator,
            source: harm.source.clone(),
            damage: harm.damage,
            horror: harm.horror,
            direct: harm.direct,
            allocations: harm
                .allocations
                .iter()
                .map(|(asset, damage, horror)| HarmAllocation {
                    asset: asset.map(|id| {
                        game.players[harm.investigator.index()]
                            .assets
                            .iter()
                            .find(|asset| asset.card.id == id)
                            .expect("Pending harm's asset remains in play")
                            .card
                            .clone()
                    }),
                    damage: *damage,
                    horror: *horror,
                })
                .collect(),
        }),
        _ => None,
    });
    let discard = game.frames.iter().rev().find_map(|frame| match frame {
        Frame::DiscardCards {
            owner,
            remaining,
            ordered,
        } => Some(DiscardView {
            owner: *owner,
            remaining: remaining.clone(),
            ordered: ordered.clone(),
        }),
        _ => None,
    });
    let mut resolving_cards = vec![];
    let mut add = |card: &CardView| {
        if !resolving_cards
            .iter()
            .any(|present: &CardView| present.id == card.id)
        {
            resolving_cards.push(card.clone());
        }
    };
    for frame in &game.frames {
        match frame {
            Frame::Draw(draw) => {
                for card in draw.pending_revelations() {
                    add(card);
                }
            }
            Frame::HarmCard(event) => add(&event.card),
            Frame::RetryEvent { event, .. } => add(event),
            Frame::AssetPlacement(asset) => add(&asset.card),
            Frame::Opportunity { asset, event, .. } => {
                if let Some(asset) = asset {
                    add(&asset.card);
                }
                if let Some(event) = event {
                    add(&event.card);
                }
            }
            Frame::EncounterSearch(
                SearchDecision::Recover { event }
                | SearchDecision::ChooseEnemy { event, .. }
                | SearchDecision::AfterSpawn { event, .. }
                | SearchDecision::FinishEvent { event },
            ) => add(&event.card),
            Frame::QuarrySpawn { card, .. } => add(card),
            _ => {}
        }
    }
    if let Some(tokens) = &game.tokens {
        for card in tokens.resolving_cards() {
            add(&card);
        }
    }
    PendingView {
        harm,
        discard,
        resolving_cards,
    }
}
