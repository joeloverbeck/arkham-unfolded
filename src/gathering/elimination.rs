//! Original RR elimination and the remaining party's continuation.
use super::{
    CardView, Investigator,
    runtime::{AssetState, Frame, Game},
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleFixture {
    LeadDefeat,
    SimultaneousDefeat,
    LastDefeat,
    Resignation,
    DrawDefeat,
    SealedDefeat,
    EndTurnDefeat,
    PromiseDefeat,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Cause {
    Harm,
    Resignation,
    Agenda,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum EliminationDecision {
    Trauma(Investigator),
    Lead,
}

pub(super) fn choices(game: &Game, decision: &EliminationDecision) -> Vec<Choice> {
    match decision {
        EliminationDecision::Trauma(_) => [
            ("elimination:physical", "Suffer 1 physical trauma"),
            ("elimination:mental", "Suffer 1 mental trauma"),
        ]
        .into_iter()
        .map(|(id, label)| Choice {
            id: id.into(),
            label: label.into(),
            cancel_faces: vec![],
        })
        .collect(),
        EliminationDecision::Lead => game
            .players
            .iter()
            .filter(|p| !p.defeated && !p.resigned)
            .map(|p| Choice {
                id: format!("elimination:lead:{}", p.investigator.id()),
                label: format!(
                    "Choose {} as the new lead investigator",
                    p.investigator.name()
                ),
                cancel_faces: vec![],
            })
            .collect(),
    }
}
pub(super) fn begin(game: &mut Game, who: Investigator, cause: Cause) -> Result<(), RuleError> {
    let player = &mut game.players[who.index()];
    if player.defeated || player.resigned {
        return Ok(());
    }
    player.defeated = cause != Cause::Resignation;
    player.resigned = cause == Cause::Resignation;
    player.turn_finished = true;
    let physical = cause == Cause::Agenda || cause == Cause::Harm && player.damage >= player.health;
    let mental = cause == Cause::Harm && player.horror >= player.sanity;
    if physical && mental {
        game.frames
            .push(Frame::Elimination(EliminationDecision::Trauma(who)));
        return Ok(());
    }
    player.physical_trauma += u8::from(physical);
    player.mental_trauma += u8::from(mental);
    complete(game, who)
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Elimination(decision)) = game.frames.pop() else {
        return Err(RuleError("No elimination decision is pending.".into()));
    };
    match decision {
        EliminationDecision::Trauma(who) => {
            if id == "elimination:physical" {
                game.players[who.index()].physical_trauma += 1;
            } else if id == "elimination:mental" {
                game.players[who.index()].mental_trauma += 1;
            } else {
                return Err(RuleError("Invalid trauma choice.".into()));
            }
            complete(game, who)?;
        }
        EliminationDecision::Lead => {
            game.lead = Investigator::ALL
                .into_iter()
                .find(|who| id == format!("elimination:lead:{}", who.id()))
                .ok_or_else(|| RuleError("Invalid lead investigator.".into()))?;
            game.history.push(format!(
                "{} becomes the lead investigator after elimination. [RR p10]",
                game.lead.name()
            ));
        }
    }
    Ok(())
}

fn complete(game: &mut Game, who: Investigator) -> Result<(), RuleError> {
    // None of this party's six selected weaknesses has a when-game-ends
    // ability. In-play owned weaknesses leave first, including signature enemies.
    let mut index = 0;
    while index < game.scenario.enemies.len() {
        if game.scenario.enemies[index].owner == Some(who) {
            let enemy = game.scenario.enemies.remove(index);
            game.players[who.index()].removed.push(enemy.card);
        } else {
            index += 1;
        }
    }
    let player = &mut game.players[who.index()];
    player.removed.append(&mut player.hand);
    player.removed.append(&mut player.deck);
    player.removed.append(&mut player.discard);
    let departed_assets: Vec<_> = player.assets.iter().map(|asset| asset.card.id).collect();
    let mut lita = None;
    for asset in player.assets.drain(..) {
        if asset.card.code == "01117" {
            lita = Some(asset.card);
        } else {
            player.removed.push(asset.card);
        }
    }
    let clues = std::mem::take(&mut player.clues);
    player.resources = 0;
    let location = player.location.clone();
    for card in player.threats.drain(..) {
        if card.weakness {
            player.removed.push(card);
        } else {
            game.encounter_discard.push(card);
        }
    }
    if let Some(card) = lita {
        super::scenario::remove_lita(game, card);
    }
    for asset in departed_assets {
        super::tony_effects::asset_left_play(game, asset);
    }
    game.scenario
        .location_mut(&location)
        .expect("Investigator location")
        .clues += clues;
    let mut disengaged = vec![];
    for enemy in &mut game.scenario.enemies {
        if enemy.engaged == Some(who) {
            enemy.engaged = None;
            enemy.location = location.clone();
            disengaged.push(enemy.card.id);
        }
    }
    super::draws::eliminate(game, who);
    super::backpack::eliminate(game, who);
    super::harm_cards::eliminate(game, who);
    super::phases::eliminate(game, who);
    super::encounter_search::eliminate(game, who);
    game.clear_eliminated_continuations(who)?;
    if game.actor == Some(who) {
        game.actor = None;
    }
    if game.action_in_progress == Some(who) {
        game.action_in_progress = None;
    }
    game.history.push(format!("{} is eliminated: owned zones are removed, clues drop at the current location, resources return to the pool, and engaged enemies remain there. [RR p10]",who.name()));
    if game
        .players
        .iter()
        .all(|player| player.defeated || player.resigned)
    {
        super::scenario::finish_resolution(game, super::scenario::Resolution::NoResolution);
        return Ok(());
    }
    for enemy in disengaged {
        game.check_engagement(enemy);
    }
    if game.lead == who {
        game.frames
            .push(Frame::Elimination(EliminationDecision::Lead));
    }
    Ok(())
}
fn take(cards: &mut Vec<CardView>, code: &str) -> CardView {
    cards.remove(
        cards
            .iter()
            .position(|card| card.code == code)
            .expect("Selected physical card"),
    )
}

pub(super) fn configure_fixture(game: &mut Game, fixture: LifecycleFixture) {
    if fixture == LifecycleFixture::PromiseDefeat {
        super::test_cards::configure_fixture(
            game,
            super::test_cards::TestCardFixture::PromiseAtLimit,
        );
        game.players[1].horror = game.players[1].sanity - 2;
        game.lead = Investigator::Patrice;
        return;
    }
    if fixture == LifecycleFixture::DrawDefeat {
        super::draws::configure_fixture(game, super::draws::DrawFixture::PatriceExhaustion)
            .expect("Draw interruption fixture");
        game.players[1].damage = game.players[1].health - 3;
        game.lead = Investigator::Patrice;
        return;
    }
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    for player in &mut game.players {
        player.location = "01112".into();
    }
    game.scenario.location_mut("01113").unwrap().clues = 1;
    game.scenario.location_mut("01114").unwrap().clues = 0;
    let priest = super::scenario::advance_act_two(game).expect("Priest");
    game.scenario
        .enemies
        .iter_mut()
        .find(|enemy| enemy.card.id == priest)
        .unwrap()
        .engaged = Some(Investigator::Jacqueline);
    game.lead = Investigator::Jacqueline;
    let voices = take(&mut game.encounter_deck, "01165");
    let player = &mut game.players[2];
    let chronophobia = take(&mut player.deck, "02039");
    player.threats.extend([voices, chronophobia]);
    let robes = if player.hand.iter().any(|card| card.code == "60412") {
        take(&mut player.hand, "60412")
    } else {
        take(&mut player.deck, "60412")
    };
    player.assets.push(AssetState {
        card: robes,
        ready: true,
        uses: 0,
        damage: 0,
        horror: 0,
    });
    player.clues = 2;
    player.damage = player.health - 2;
    player.horror = if fixture == LifecycleFixture::SimultaneousDefeat {
        player.sanity - 2
    } else {
        2
    };
    if fixture == LifecycleFixture::LastDefeat {
        for index in [0, 1] {
            begin(game, Investigator::ALL[index], Cause::Resignation)
                .expect("Earlier resignations");
        }
    }
    if fixture == LifecycleFixture::EndTurnDefeat {
        for index in [0, 1] {
            game.players[index].turn_finished = true;
        }
    }
    if fixture == LifecycleFixture::Resignation {
        game.players[2].location = "01115".into();
        game.scenario.enemies[0].location = "01115".into();
    }
    if fixture == LifecycleFixture::SealedDefeat {
        let player = &mut game.players[2];
        let card = if player.hand.iter().any(|card| card.code == "04199") {
            take(&mut player.hand, "04199")
        } else {
            take(&mut player.deck, "04199")
        };
        let index = game
            .bag
            .iter()
            .position(|face| *face == crate::TokenFace::Zero)
            .expect("Standard zero");
        let face = game.bag.remove(index);
        game.premonition = Some(super::tokens::SealedPremonition {
            controller: Investigator::Jacqueline,
            card,
            faces: vec![face],
        });
    }
    game.enable_fixture_turn(Investigator::Jacqueline);
    game.history = vec!["Privileged elimination witness with selected cards, a ready Priest, and two held clues. No claim that ordinary preparation reached this position. [RR pp5,10]".into()];
}
