//! Peter Sylvestre's original after-own-turn reaction.

use super::{
    Investigator,
    runtime::{AssetState, Frame, Game},
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeterFixture {
    OwnTurn,
    OtherTurn,
    Unharmed,
    ForcedBeforeReaction,
}

pub(super) fn configure_fixture(game: &mut Game, case: PeterFixture) {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    let player = &mut game.players[Investigator::Patrice.index()];
    let card = if let Some(index) = player.hand.iter().position(|card| card.code == "02033") {
        player.hand.remove(index)
    } else {
        let index = player
            .deck
            .iter()
            .position(|card| card.code == "02033")
            .expect("Selected Peter");
        player.deck.remove(index)
    };
    player.assets.push(AssetState {
        card,
        ready: true,
        uses: 0,
        damage: 0,
        horror: u8::from(case != PeterFixture::Unharmed),
    });
    game.enable_fixture_turn(if case == PeterFixture::OtherTurn {
        Investigator::Tony
    } else {
        Investigator::Patrice
    });
    if case == PeterFixture::ForcedBeforeReaction {
        let at = game
            .encounter_deck
            .iter()
            .position(|card| card.code == "01164")
            .expect("Selected Frozen in Fear");
        let card = game.encounter_deck.remove(at);
        game.players[Investigator::Patrice.index()]
            .threats
            .push(card);
        game.set_fixture_tokens(vec![crate::TokenFace::Zero]);
    }
    game.history = vec!["Privileged Peter witness: Patrice controls her actual original Peter, with the named horror and current turn. His optional reaction belongs after his controller's turn, not another investigator's turn. [02033; official response 2017-04-16]".into()];
}

pub(super) fn begin(game: &mut Game, owner: Investigator) -> bool {
    if choices(game, owner).len() <= 1 {
        return false;
    }
    game.frames.push(Frame::Peter { owner });
    true
}
pub(super) fn choices(game: &Game, owner: Investigator) -> Vec<Choice> {
    let player = &game.players[owner.index()];
    if player.defeated || player.resigned {
        return vec![];
    }
    let mut choices: Vec<_> = player
        .assets
        .iter()
        .filter(|asset| asset.card.code == "02033" && asset.horror > 0)
        .map(|asset| Choice {
            id: format!("peter:heal:{}", asset.card.id),
            label: "After your turn: heal 1 horror from Peter Sylvestre".into(),
            cancel_faces: vec![],
        })
        .collect();
    choices.push(Choice {
        id: "peter:decline".into(),
        label: "Do not use Peter Sylvestre's after-turn reaction".into(),
        cancel_faces: vec![],
    });
    choices
}
pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Peter { owner }) = game.frames.last().cloned() else {
        return Err(RuleError("No Peter reaction is pending.".into()));
    };
    if !choices(game, owner).iter().any(|choice| choice.id == id) {
        return Err(RuleError("That Peter reaction is unavailable.".into()));
    }
    game.frames.pop();
    if let Some(asset) = id.strip_prefix("peter:heal:") {
        let asset: u16 = asset.parse().expect("Offered Peter");
        game.players[owner.index()]
            .assets
            .iter_mut()
            .find(|candidate| candidate.card.id == asset)
            .expect("Offered Peter")
            .horror -= 1;
        game.history.push(format!(
            "After {}'s turn ends, Peter Sylvestre heals 1 horror from himself. [02033]",
            owner.name()
        ));
    }
    finish_turn_end(game);
    Ok(())
}

pub(super) fn finish_turn_end(game: &mut Game) {
    if game
        .players
        .iter()
        .all(|player| player.turn_finished || player.defeated || player.resigned)
    {
        super::phases::begin_enemy(game);
    }
}
