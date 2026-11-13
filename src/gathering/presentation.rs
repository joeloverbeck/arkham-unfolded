//! Player-visible pending consequences, projected from the runtime's typed frames.
//!
//! `observe` is the normal projection. `draws::advance` and `tokens::resume`
//! also supplement `PendingView` for captured views while their continuation
//! is detached from `Game`, then call `Game::capture_public_view`. Changes to
//! resolving cards, ownership or test routing must cover those producers too.

use super::{
    CardView, Investigator,
    encounter_search::SearchDecision,
    phases::PhaseDecision,
    runtime::{Frame, Game, SkillTest, TestOperation},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingView {
    pub harm: Option<HarmView>,
    pub discard: Option<DiscardView>,
    pub resolving_cards: Vec<CardView>,
    /// The originating effect, independent of the investigator answering a nested choice.
    pub resolution_owner: Option<Investigator>,
    /// Resolving reaction cards belonging to the current skill-test token sequence.
    pub test_resolving_cards: Vec<u16>,
    /// Counts are public; unfinished simultaneous draws disclose no card identities.
    #[serde(default)]
    pub held_draws: Vec<HeldDrawView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldDrawView {
    pub investigator: Investigator,
    pub count: usize,
}

impl PendingView {
    /// Include a reveal even when its continuation was temporarily taken out of
    /// Game to advance it. Normal observations and captured completion views
    /// must expose the same owner and paid cards.
    pub(super) fn include_tokens(&mut self, tokens: &super::tokens::TokenResolution) {
        if !tokens.is_skill_test() {
            self.resolution_owner.get_or_insert(tokens.revealer);
        }
        for card in tokens.resolving_cards() {
            if tokens.is_skill_test() && !self.test_resolving_cards.contains(&card.id) {
                self.test_resolving_cards.push(card.id);
            }
            if !self
                .resolving_cards
                .iter()
                .any(|present| present.id == card.id)
            {
                self.resolving_cards.push(card);
            }
        }
    }
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

/// What a skill test attempts, as players name it (#210).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestAction {
    Fight,
    Evade,
    Investigate,
    /// Parley with a card in play: Lita Chantler.
    Parley,
    /// A test a card calls for: a treachery's revelation, a threat in play, or an
    /// event such as A Test of Will.
    Card,
}

/// What a skill test acts on (the enemy fought or evaded, the location
/// investigated, Lita for a parley) or, for a test a card calls for, that card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestTarget {
    /// The card's or the location's code.
    pub code: String,
    pub title: String,
}

pub(super) fn test_action(test: &SkillTest) -> TestAction {
    match test.operation {
        TestOperation::Fight { .. } => TestAction::Fight,
        TestOperation::Evade { .. } => TestAction::Evade,
        TestOperation::Investigate { .. } => TestAction::Investigate,
        TestOperation::LitaParley => TestAction::Parley,
        TestOperation::CancelRevelation { .. }
        | TestOperation::RottingRemains { .. }
        | TestOperation::GraspingHands { .. }
        | TestOperation::CryptChill { .. }
        | TestOperation::DiscardThreat { .. } => TestAction::Card,
    }
}

pub(super) fn test_target(game: &Game, test: &SkillTest) -> Option<TestTarget> {
    let named = |code: &str| TestTarget {
        code: code.to_owned(),
        title: super::content::title(code).unwrap_or(code).to_owned(),
    };
    let card = |card: &CardView| TestTarget {
        code: card.code.clone(),
        title: card.title.clone(),
    };
    match &test.operation {
        TestOperation::Fight { enemy, .. } | TestOperation::Evade { enemy } => game
            .scenario
            .enemies
            .iter()
            .map(|enemy| &enemy.card)
            // A weakness enemy can be fought from its owner's hand.
            .chain(game.players.iter().flat_map(|player| &player.hand))
            .find(|card| card.id == *enemy)
            .map(card),
        TestOperation::Investigate { location, .. } => Some(named(location)),
        TestOperation::LitaParley => Some(named("01117")),
        TestOperation::DiscardThreat { owner, card: id } => game.players[owner.index()]
            .threats
            .iter()
            .find(|threat| threat.id == *id)
            .map(card),
        TestOperation::CancelRevelation { .. }
        | TestOperation::RottingRemains { .. }
        | TestOperation::GraspingHands { .. }
        | TestOperation::CryptChill { .. } => test_source(game, test).as_ref().map(card),
    }
}

/// Temporary test sources otherwise held by a continuation still need their
/// actual face in Test. This projection does not move a card between rules zones.
pub(super) fn test_source(game: &Game, test: &SkillTest) -> Option<CardView> {
    test.source_card.clone().or_else(|| match &test.operation {
        TestOperation::CancelRevelation { event, .. } => Some(event.clone()),
        TestOperation::RottingRemains { encounter }
        | TestOperation::GraspingHands { encounter }
        | TestOperation::CryptChill { encounter } => {
            game.frames.iter().find_map(|frame| match frame {
                Frame::Revelation(revelation) if revelation.card.id == *encounter => {
                    Some(revelation.card.clone())
                }
                _ => None,
            })
        }
        _ => None,
    })
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
        Frame::Phase(PhaseDecision::PatriceDiscard { selected, .. }) => {
            let hand = &game.players[Investigator::Patrice.index()].hand;
            Some(DiscardView {
                owner: Investigator::Patrice,
                remaining: hand
                    .iter()
                    .filter(|card| !card.weakness && !selected.contains(&card.id))
                    .cloned()
                    .collect(),
                ordered: selected
                    .iter()
                    .map(|id| {
                        hand.iter()
                            .find(|card| card.id == *id)
                            .expect("An ordered upkeep card remains in Patrice's hand")
                            .clone()
                    })
                    .collect(),
            })
        }
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
    let mut resolution_owner = None;
    let mut held_draws = vec![];
    for frame in &game.frames {
        match frame {
            Frame::Draw(draw) => {
                if draw.held_count() > 0 {
                    held_draws.push(HeldDrawView {
                        investigator: draw.investigator,
                        count: draw.held_count(),
                    });
                }
                for card in draw.pending_revelations() {
                    add(card);
                    resolution_owner.get_or_insert(draw.investigator);
                }
            }
            Frame::HarmCard(event) => {
                add(&event.card);
                resolution_owner.get_or_insert(event.owner);
            }
            Frame::RetryEvent { owner, event } => {
                add(event);
                resolution_owner.get_or_insert(*owner);
            }
            Frame::AssetPlacement(asset) => {
                add(&asset.card);
                resolution_owner.get_or_insert(asset.actor);
            }
            Frame::Opportunity {
                investigator,
                asset,
                event,
                ..
            } => {
                if let Some(asset) = asset {
                    add(&asset.card);
                    resolution_owner.get_or_insert(*investigator);
                }
                if let Some(event) = event {
                    add(&event.card);
                    resolution_owner.get_or_insert(event.owner);
                }
            }
            Frame::EncounterSearch(
                SearchDecision::Recover { event }
                | SearchDecision::ChooseEnemy { event, .. }
                | SearchDecision::AfterSpawn { event, .. }
                | SearchDecision::FinishEvent { event },
            ) => {
                add(&event.card);
                resolution_owner.get_or_insert(event.owner);
            }
            Frame::QuarrySpawn { card, .. } => {
                add(card);
                // This frame resolves Patrice's selected Watcher from Another Dimension.
                resolution_owner.get_or_insert(Investigator::Patrice);
            }
            Frame::Revelation(revelation) => {
                resolution_owner.get_or_insert(revelation.drawn_by);
            }
            Frame::Backpack(backpack) => {
                resolution_owner.get_or_insert(backpack.owner());
            }
            _ => {}
        }
    }
    // Queued cleanup discards are independent owner groups, not ancestor
    // effects. Use the active group's owner when no enclosing card resolution
    // already owns the tray; a future queued group must never claim it.
    resolution_owner = resolution_owner.or_else(|| discard.as_ref().map(|discard| discard.owner));
    let mut pending = PendingView {
        harm,
        discard,
        resolving_cards,
        resolution_owner,
        test_resolving_cards: vec![],
        held_draws,
    };
    if let Some(tokens) = &game.tokens {
        pending.include_tokens(tokens);
    }
    pending
}
