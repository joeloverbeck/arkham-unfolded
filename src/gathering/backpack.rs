//! Original Backpack: a finite private search and facedown playable attachments.
use super::{
    CardView, Investigator, cards,
    option_labels::counted,
    runtime::{AssetState, Frame, Game, PendingRevelation},
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackpackFixture {
    Search,
    NoMatch,
    ShortDeck,
    PairedColts,
    Chill,
    Defeat,
}

/// Faces are inspectable by the controller (FAQ 2.5, p17), but remain attached,
/// outside the hand; this is the single human's complete party observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachedCards {
    pub owner: Investigator,
    pub asset: u16,
    pub cards: Vec<CardView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum BackpackDecision {
    Entry {
        owner: Investigator,
        asset: u16,
    },
    Search {
        owner: Investigator,
        asset: u16,
        searched: Vec<u16>,
        selected: Vec<u16>,
    },
}
impl BackpackDecision {
    pub(super) fn owner(&self) -> Investigator {
        match self {
            Self::Entry { owner, .. } | Self::Search { owner, .. } => *owner,
        }
    }
}
fn choice(id: impl Into<String>, label: impl Into<String>) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        cancel_faces: vec![],
    }
}
pub(super) fn after_play(game: &mut Game, owner: Investigator, asset: u16) {
    game.frames
        .push(Frame::Backpack(BackpackDecision::Entry { owner, asset }));
}
pub(super) fn searched_cards(game: &Game) -> Vec<CardView> {
    let Some(Frame::Backpack(BackpackDecision::Search {
        owner, searched, ..
    })) = game.frames.last()
    else {
        return vec![];
    };
    searched
        .iter()
        .map(|id| {
            game.players[owner.index()]
                .deck
                .iter()
                .find(|card| card.id == *id)
                .expect("Searched card remains in deck")
                .clone()
        })
        .collect()
}
pub(super) fn choices(game: &Game, decision: &BackpackDecision) -> Vec<Choice> {
    match decision {
        BackpackDecision::Entry { owner, .. } => {
            let mut choices = vec![choice(
                "backpack:decline",
                "Decline Backpack's search; discard the empty Backpack",
            )];
            if !game.players[owner.index()].deck.is_empty() {
                choices.push(choice(
                    "backpack:search",
                    "Search the top 6 cards with Backpack",
                ));
            }
            choices
        }
        BackpackDecision::Search {
            owner,
            searched,
            selected,
            ..
        } => {
            let mut choices = vec![choice(
                "backpack:attach",
                format!(
                    "Attach {} facedown; shuffle the deck",
                    counted(selected.len(), "selected card", "selected cards")
                ),
            )];
            for id in searched {
                let card = game.players[owner.index()]
                    .deck
                    .iter()
                    .find(|card| card.id == *id)
                    .expect("Searched card remains in deck");
                let definition = cards::definition(&card.code).expect("Selected card definition");
                let chosen = selected.contains(id);
                if !definition.weakness
                    && (definition.has_trait("Item") || definition.has_trait("Supply"))
                    && (chosen || selected.len() < 3)
                {
                    choices.push(choice(
                        format!("backpack:select:{id}"),
                        format!(
                            "{} {}",
                            if chosen { "Unselect" } else { "Select" },
                            card.title
                        ),
                    ));
                }
            }
            choices
        }
    }
}
pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Backpack(decision)) = game.frames.last() else {
        return Err(RuleError("No Backpack decision is pending.".into()));
    };
    if !choices(game, decision).iter().any(|choice| choice.id == id) {
        return Err(RuleError("That Backpack choice is not available.".into()));
    }
    let Some(Frame::Backpack(decision)) = game.frames.pop() else {
        unreachable!()
    };
    match decision {
        BackpackDecision::Entry { owner, asset } => {
            if id == "backpack:decline" {
                discard_empty(game, owner, asset);
            } else {
                let searched = game.players[owner.index()]
                    .deck
                    .iter()
                    .rev()
                    .take(6)
                    .map(|card| card.id)
                    .collect();
                game.frames.push(Frame::Backpack(BackpackDecision::Search {
                    owner,
                    asset,
                    searched,
                    selected: vec![],
                }));
            }
        }
        BackpackDecision::Search {
            owner,
            asset,
            searched,
            mut selected,
        } => {
            if id == "backpack:attach" {
                let mut attached = vec![];
                for id in selected {
                    let deck = &mut game.players[owner.index()].deck;
                    let index = deck
                        .iter()
                        .position(|card| card.id == id)
                        .expect("Searched selected card");
                    attached.push(deck.remove(index));
                }
                for index in (1..game.players[owner.index()].deck.len()).rev() {
                    let other = game.random_index(index + 1);
                    game.players[owner.index()].deck.swap(index, other);
                }
                game.history.push(format!("{} finishes Backpack's search, attaches {} cards facedown, and shuffles the remaining deck. [04037; RR p18]",owner.name(),attached.len()));
                if attached.is_empty() {
                    discard_empty(game, owner, asset);
                } else {
                    game.backpacks.push(AttachedCards {
                        owner,
                        asset,
                        cards: attached,
                    });
                }
                game.capture_shuffle(super::DrawSource::Investigator(owner), 0, vec![]);
            } else {
                let id: u16 = id
                    .strip_prefix("backpack:select:")
                    .unwrap()
                    .parse()
                    .unwrap();
                if let Some(index) = selected.iter().position(|candidate| *candidate == id) {
                    selected.remove(index);
                } else {
                    selected.push(id);
                }
                game.frames.push(Frame::Backpack(BackpackDecision::Search {
                    owner,
                    asset,
                    searched,
                    selected,
                }));
            }
        }
    }
    Ok(())
}

pub(super) fn playable_cards(game: &Game, owner: Investigator) -> Vec<&CardView> {
    game.players[owner.index()]
        .hand
        .iter()
        .chain(
            game.backpacks
                .iter()
                .filter(|pack| pack.owner == owner)
                .flat_map(|pack| &pack.cards),
        )
        .collect()
}
pub(super) fn take_for_play(game: &mut Game, owner: Investigator, id: u16) -> CardView {
    if let Some(index) = game.players[owner.index()]
        .hand
        .iter()
        .position(|card| card.id == id)
    {
        return game.players[owner.index()].hand.remove(index);
    }
    let index = game
        .backpacks
        .iter()
        .position(|pack| pack.owner == owner && pack.cards.iter().any(|card| card.id == id))
        .expect("Advertised attached card");
    let position = game.backpacks[index]
        .cards
        .iter()
        .position(|card| card.id == id)
        .unwrap();
    let card = game.backpacks[index].cards.remove(position);
    if game.backpacks[index].cards.is_empty() {
        let pack = game.backpacks.remove(index);
        discard_empty(game, owner, pack.asset);
    }
    card
}
fn discard_empty(game: &mut Game, owner: Investigator, asset: u16) {
    let player = &mut game.players[owner.index()];
    let index = player
        .assets
        .iter()
        .position(|card| card.card.id == asset)
        .expect("Controlled Backpack");
    let card = player.assets.remove(index).card;
    player.discard.push(card);
    game.history
        .push("Backpack has no attached cards and is discarded. [04037]".into());
}
/// Called with the host already removed. RR p14 makes its attachments leave
/// simultaneously; the owner orders all these cards together (RR p9).
pub(super) fn discard_host(game: &mut Game, owner: Investigator, host: CardView) {
    discard_replaced(game, owner, vec![host]);
}
pub(super) fn discard_replaced(game: &mut Game, owner: Investigator, hosts: Vec<CardView>) {
    let mut discarded = vec![];
    let mut has_attachments = false;
    for host in hosts {
        if let Some(index) = game.backpacks.iter().position(|pack| pack.asset == host.id) {
            has_attachments = true;
            discarded.extend(game.backpacks.remove(index).cards);
        }
        discarded.push(host);
    }
    if has_attachments && discarded.len() > 1 {
        game.frames.push(Frame::DiscardCards {
            owner,
            remaining: discarded,
            ordered: vec![],
        });
    } else {
        game.players[owner.index()].discard.extend(discarded);
    }
}
pub(super) fn eliminate(game: &mut Game, owner: Investigator) {
    let mut index = 0;
    while index < game.backpacks.len() {
        if game.backpacks[index].owner == owner {
            let pack = game.backpacks.remove(index);
            game.players[owner.index()].removed.extend(pack.cards);
        } else {
            index += 1;
        }
    }
    game.frames
        .retain(|frame| !matches!(frame,Frame::Backpack(decision) if decision.owner()==owner));
}

fn take(game: &mut Game, code: &str) -> CardView {
    let player = &mut game.players[Investigator::Tony.index()];
    if let Some(index) = player.hand.iter().position(|card| card.code == code) {
        player.hand.remove(index)
    } else {
        let index = player
            .deck
            .iter()
            .position(|card| card.code == code)
            .expect("Selected physical card");
        player.deck.remove(index)
    }
}
pub(super) fn configure_fixture(game: &mut Game, case: BackpackFixture) {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    let owner = Investigator::Tony;
    let player = &mut game.players[owner.index()];
    player.deck.append(&mut player.hand);
    player.resources = 10;
    player.ordinary_actions = 3;
    let pack = take(game, "04037");
    if matches!(case, BackpackFixture::Chill | BackpackFixture::Defeat) {
        let mut attached = vec![];
        for code in ["04107", "05117", "60306"] {
            attached.push(take(game, code));
        }
        game.backpacks.push(AttachedCards {
            owner,
            asset: pack.id,
            cards: attached,
        });
        game.players[0].assets.push(AssetState {
            card: pack,
            ready: true,
            uses: 0,
            damage: 0,
            horror: 0,
        });
        if case == BackpackFixture::Chill {
            let index = game
                .encounter_deck
                .iter()
                .position(|card| card.code == "01167")
                .unwrap();
            let card = game.encounter_deck.remove(index);
            game.frames.push(Frame::Revelation(PendingRevelation {
                drawn_by: owner,
                card,
                started: false,
                cancelled: false,
            }));
            game.set_fixture_tokens(vec![crate::TokenFace::MinusOne]);
        } else {
            let index = game
                .encounter_deck
                .iter()
                .position(|card| card.code == "01160")
                .unwrap();
            let card = game.encounter_deck.remove(index);
            super::scenario::spawn_encounter_enemy(game, card, owner).unwrap();
            game.players[0].damage = game.players[0].health - 1;
        }
    } else {
        game.players[0].hand.push(pack);
        let codes: &[&str] = match case {
            BackpackFixture::NoMatch => &["01089", "02038", "02225", "02229", "03233", "03263"],
            BackpackFixture::ShortDeck => &["04107", "01089"],
            BackpackFixture::PairedColts => &["06011", "06011", "04107", "01089", "02038", "02225"],
            _ => &["04107", "05117", "06011", "60306", "01089", "02038"],
        };
        let mut top: Vec<_> = codes.iter().map(|code| take(game, code)).collect();
        if case == BackpackFixture::ShortDeck {
            let player = &mut game.players[0];
            player.discard.append(&mut player.deck);
        }
        top.reverse();
        game.players[0].deck.extend(top);
    }
    game.enable_fixture_turn(owner);
}
