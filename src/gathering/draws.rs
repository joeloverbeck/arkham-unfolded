//! Simultaneous player draws and their nested exhaustion/revelation effects.

use super::{
    CardView, Investigator, cards,
    runtime::{
        AssetState, CommittedCard, Frame, Game, PendingRevelation, Skill, TestOperation,
        TestResultEffect, Window,
    },
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawFixture {
    PatriceExhaustion,
    JacquelineWeaknessOrder,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DrawState {
    pub investigator: Investigator,
    remaining: usize,
    drawn: Vec<CardView>,
    revelations: Vec<CardView>,
    exhaustion_horror: u8,
    collected: bool,
}

pub(super) fn begin(
    game: &mut Game,
    investigator: Investigator,
    count: usize,
) -> Result<(), RuleError> {
    if count == 0
        || game.players[investigator.index()].defeated
        || game.players[investigator.index()].resigned
    {
        return Ok(());
    }
    game.frames.push(Frame::Draw(DrawState {
        investigator,
        remaining: count,
        drawn: vec![],
        revelations: vec![],
        exhaustion_horror: 0,
        collected: false,
    }));
    Ok(())
}

pub(super) fn advance(game: &mut Game) -> Result<(), RuleError> {
    let Some(Frame::Draw(mut draw)) = game.frames.pop() else {
        return Err(RuleError("No player draw is waiting to continue.".into()));
    };
    let index = draw.investigator.index();
    if game.players[index].defeated || game.players[index].resigned {
        // The interrupted draw retains physical custody even when its owner
        // is eliminated before the simultaneous batch can finish.
        game.players[index].removed.append(&mut draw.drawn);
        game.players[index].removed.append(&mut draw.revelations);
        return Ok(());
    }
    while !draw.collected && draw.remaining > 0 {
        if game.players[index].deck.is_empty() {
            if game.players[index].discard.is_empty() {
                game.history.push(format!("{} cannot draw the remaining {} cards: both deck and discard pile are empty. No empty-pile shuffle is performed. [RR Drawing Cards; Discard Piles]", draw.investigator.name(), draw.remaining));
                draw.remaining = 0;
                break;
            }
            game.players[index].deck = std::mem::take(&mut game.players[index].discard);
            for upper in (1..game.players[index].deck.len()).rev() {
                let other = game.random_index(upper + 1);
                game.players[index].deck.swap(upper, other);
            }
            draw.exhaustion_horror += 1;
            game.history.push(format!("{} resets the empty deck from the discard pile and continues the same draw. Its horror waits until the draw resolves. [RR Drawing Cards]", draw.investigator.name()));
        }
        let card = game.players[index].deck.pop().expect("Available draw card");
        draw.drawn.push(card);
        draw.remaining -= 1;
        if game.players[index].deck.is_empty()
            && let Some(watcher) = game.players[index]
                .hand
                .iter()
                .find(|card| card.code == "06017")
        {
            let watcher_id = watcher.id;
            let investigator = draw.investigator;
            let values = cards::definition("06017")
                .and_then(|definition| definition.enemy)
                .expect("Selected Watcher values");
            game.frames.push(Frame::Draw(draw));
            game.history.push("Patrice's deck has just run out while Watcher is already in her hand. Its forced attack interrupts the draw before any reset or later draw. [06017; FAQ 1.4]".into());
            game.queue_harm(
                investigator,
                values.attack_damage,
                values.attack_horror,
                false,
                "Watcher from Another Dimension [06017]",
                Some(watcher_id),
            )?;
            return Ok(());
        }
    }
    if !draw.collected {
        for card in draw.drawn.drain(..) {
            if card.weakness {
                draw.revelations.push(card);
            } else {
                game.players[index].hand.push(card);
            }
        }
        draw.collected = true;
        game.history.push(format!("{} completes the simultaneous card draw. Resolve any drawn revelations in the investigator's chosen order. [FAQ 2.5 p17]", draw.investigator.name()));
    }
    if draw.revelations.len() > 1 {
        game.frames.push(Frame::Draw(draw));
        return Ok(());
    }
    if let Some(card) = draw.revelations.pop() {
        let investigator = draw.investigator;
        game.frames.push(Frame::Draw(draw));
        queue_revelation(game, investigator, card);
        return Ok(());
    }
    if draw.exhaustion_horror > 0 {
        let amount = std::mem::take(&mut draw.exhaustion_horror);
        let investigator = draw.investigator;
        game.frames.push(Frame::Draw(draw));
        game.queue_harm(
            investigator,
            0,
            amount,
            false,
            "Player-deck exhaustion [RR Drawing Cards]",
            None,
        )?;
    }
    Ok(())
}

pub(super) fn choices(game: &Game) -> Vec<Choice> {
    let Some(Frame::Draw(draw)) = game.frames.last() else {
        return vec![];
    };
    if !draw.collected || draw.revelations.len() < 2 {
        return vec![];
    }
    draw.revelations
        .iter()
        .map(|card| Choice {
            id: format!("draw:reveal:{}", card.id),
            label: format!("Resolve {} next from the simultaneous draw", card.title),
            cancel_faces: vec![],
        })
        .collect()
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    if !choices(game).iter().any(|choice| choice.id == id) {
        return Err(RuleError(
            "That drawn revelation is not currently available.".into(),
        ));
    }
    let card_id: u16 = id
        .strip_prefix("draw:reveal:")
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| RuleError("Invalid drawn card identity.".into()))?;
    let Some(Frame::Draw(draw)) = game.frames.last_mut() else {
        unreachable!("Validated draw decision");
    };
    let card = draw.revelations.remove(
        draw.revelations
            .iter()
            .position(|card| card.id == card_id)
            .expect("Offered revelation"),
    );
    let investigator = draw.investigator;
    queue_revelation(game, investigator, card);
    Ok(())
}

fn queue_revelation(game: &mut Game, investigator: Investigator, card: CardView) {
    game.frames.push(Frame::Revelation(PendingRevelation {
        card,
        drawn_by: investigator,
        started: false,
        cancelled: false,
    }));
}

fn take(pool: &mut Vec<CardView>, code: &str) -> CardView {
    pool.remove(
        pool.iter()
            .position(|card| card.code == code)
            .expect("Selected fixture identity"),
    )
}

/// A named prepared result, not a claim that an ordinary game reached it.
/// Every physical card remains in its own actual selected inventory.
pub(super) fn configure_fixture(game: &mut Game, fixture: DrawFixture) -> Result<(), RuleError> {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    game.test = None;
    if fixture == DrawFixture::JacquelineWeaknessOrder {
        return configure_weakness_order(game);
    }
    let player = &mut game.players[Investigator::Patrice.index()];
    let mut pool = std::mem::take(&mut player.hand);
    pool.append(&mut player.deck);
    pool.append(&mut player.discard);
    pool.append(&mut player.threats);
    pool.extend(player.assets.drain(..).map(|asset| asset.card));
    let heart = take(&mut pool, "04201");
    let watcher = take(&mut pool, "06017");
    let vision = take(&mut pool, "05041");
    let peter = take(&mut pool, "02033");
    player.deck.push(take(&mut pool, "60507"));
    player.discard.push(take(&mut pool, "04029"));
    player.discard.push(take(&mut pool, "01093"));
    pool.push(watcher);
    player.hand = pool;
    player.threats.push(vision);
    player.assets.push(AssetState {
        card: peter,
        ready: true,
        uses: 0,
        damage: 0,
        horror: 0,
    });
    player.resources = 3;
    player.damage = 0;
    player.horror = 0;
    let location = player.location.clone();
    game.begin_test(
        TestOperation::Investigate { location, clues: 1 },
        Investigator::Patrice,
        Skill::Intellect,
        4,
    )?;
    let minus_one = game
        .bag
        .iter()
        .position(|face| *face == TokenFace::MinusOne)
        .expect("Standard bag minus one");
    game.bag.remove(minus_one);
    let test = game.test.as_mut().expect("Prepared failed test");
    test.committed.push(CommittedCard {
        card: heart,
        owner: Investigator::Patrice,
    });
    test.revealed.push(TokenFace::MinusOne);
    test.modifier = -1;
    test.window = Window::Results;
    test.results_started = true;
    game.frames
        .push(Frame::TestResults(vec![TestResultEffect::TakeHeart]));
    game.history.push("Privileged draw witness: Patrice's failed Cellar investigation has a committed Take Heart awaiting its result. Watcher is already in hand; one card remains in deck, two ordinary cards in discard, and Peter offers only ordinary harm assignment. This is not an ordinary starting game. [04201; 06017; RR Drawing Cards]".into());
    Ok(())
}

fn configure_weakness_order(game: &mut Game) -> Result<(), RuleError> {
    let player = &mut game.players[Investigator::Jacqueline.index()];
    let mut pool = std::mem::take(&mut player.hand);
    pool.append(&mut player.deck);
    pool.append(&mut player.discard);
    pool.append(&mut player.threats);
    player.deck.push(take(&mut pool, "02039"));
    player.deck.push(take(&mut pool, "60403"));
    player.hand = pool;
    player.location = super::scenario::CELLAR.into();
    player.resources = 3;
    let heart = take(
        &mut game.players[Investigator::Patrice.index()].hand,
        "04201",
    );
    game.begin_test(
        TestOperation::Investigate {
            location: super::scenario::CELLAR.into(),
            clues: 1,
        },
        Investigator::Jacqueline,
        Skill::Intellect,
        4,
    )?;
    let minus_one = game
        .bag
        .iter()
        .position(|face| *face == TokenFace::MinusOne)
        .expect("Standard bag minus one");
    game.bag.remove(minus_one);
    let test = game.test.as_mut().expect("Prepared failed test");
    test.committed.push(CommittedCard {
        card: heart,
        owner: Investigator::Patrice,
    });
    test.revealed.push(TokenFace::MinusOne);
    test.modifier = -1;
    test.window = Window::Results;
    test.results_started = true;
    game.frames
        .push(Frame::TestResults(vec![TestResultEffect::TakeHeart]));
    game.history.push("Privileged draw witness: Jacqueline's failed Cellar investigation has Patrice's committed Take Heart awaiting its result. Her two remaining deck cards are Chronophobia and Dark Future; their simultaneous revelation order belongs to Jacqueline. This is not an ordinary starting game. [04201; FAQ 2.5 p17]".into());
    Ok(())
}
