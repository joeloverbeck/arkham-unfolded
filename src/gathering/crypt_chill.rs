//! Original Crypt Chill and the selected Sign Magick slot consequence.
use super::{
    CardView, Investigator,
    runtime::{AssetState, Frame, Game, HarmOrigin, PendingRevelation},
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CryptChillFixture {
    Assets,
    NoAsset,
    SignMagick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ChillDecision {
    pub owner: Investigator,
    pub arcane_overflow: bool,
}
pub(super) fn choices(game: &Game, decision: &ChillDecision) -> Vec<Choice> {
    game.players[decision.owner.index()]
        .assets
        .iter()
        .filter(|asset| {
            let definition =
                super::player_cards::asset_definition(&asset.card.code).expect("Controlled asset");
            !definition.permanent
                && (!decision.arcane_overflow
                    || definition.slots.contains(&super::cards::Slot::Arcane))
        })
        .map(|asset| Choice {
            id: format!("chill:{}", asset.card.id),
            label: format!(
                "Discard {}{}",
                asset.card.title,
                if decision.arcane_overflow {
                    " to satisfy the remaining arcane slots"
                } else {
                    " for Crypt Chill"
                }
            ),
            cancel_faces: vec![],
        })
        .collect()
}
pub(super) fn failed(game: &mut Game, owner: Investigator) -> Result<(), RuleError> {
    let decision = ChillDecision {
        owner,
        arcane_overflow: false,
    };
    if choices(game, &decision).is_empty() {
        game.queue_harm(owner, 2, 0, false, "01167", HarmOrigin::EncounterCard)?;
    } else {
        game.frames.push(Frame::CryptChill(decision));
    }
    Ok(())
}
pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::CryptChill(decision)) = game.frames.last() else {
        return Err(RuleError("No Crypt Chill asset choice is pending.".into()));
    };
    if !choices(game, decision).iter().any(|choice| choice.id == id) {
        return Err(RuleError(
            "That asset cannot be discarded for this effect.".into(),
        ));
    }
    let owner = decision.owner;
    let id: u16 = id.strip_prefix("chill:").unwrap().parse().unwrap();
    game.frames.pop();
    let player = &mut game.players[owner.index()];
    let index = player
        .assets
        .iter()
        .position(|asset| asset.card.id == id)
        .unwrap();
    let asset = player.assets.remove(index);
    game.history.push(format!(
        "{} discards {} for Crypt Chill or its required slot cleanup. [01167; RR Slots]",
        owner.name(),
        asset.card.title
    ));
    if asset.card.code == "01117" {
        super::scenario::remove_lita(game, asset.card);
    } else {
        game.players[owner.index()].discard.push(asset.card);
    }
    super::tony_effects::asset_left_play(game, id);
    let player = &game.players[owner.index()];
    let occupied = player
        .assets
        .iter()
        .map(|asset| {
            super::player_cards::asset_definition(&asset.card.code)
                .unwrap()
                .slots
                .iter()
                .filter(|slot| **slot == super::cards::Slot::Arcane)
                .count()
        })
        .sum::<usize>();
    let capacity = 2 + player
        .assets
        .iter()
        .filter(|asset| asset.card.code == "05112")
        .count();
    if occupied > capacity {
        game.frames.push(Frame::CryptChill(ChillDecision {
            owner,
            arcane_overflow: true,
        }));
    }
    Ok(())
}

fn take(game: &mut Game, who: Investigator, code: &str) -> CardView {
    let player = &mut game.players[who.index()];
    if let Some(index) = player.hand.iter().position(|card| card.code == code) {
        player.hand.remove(index)
    } else {
        player.deck.remove(
            player
                .deck
                .iter()
                .position(|card| card.code == code)
                .expect("Selected asset"),
        )
    }
}
pub(super) fn configure_fixture(game: &mut Game, case: CryptChillFixture) {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    let owner = if case == CryptChillFixture::SignMagick {
        Investigator::Jacqueline
    } else {
        Investigator::Tony
    };
    for player in &mut game.players {
        player.location = "01112".into();
    }
    let codes: &[&str] = match case {
        CryptChillFixture::Assets => &["05117", "01049"],
        CryptChillFixture::NoAsset => &[],
        CryptChillFixture::SignMagick => &["05112", "60408", "11067", "11067"],
    };
    for code in codes {
        let card = take(game, owner, code);
        let uses = super::cards::definition(code)
            .unwrap()
            .uses
            .map_or(0, |(_, n)| n);
        game.players[owner.index()].assets.push(AssetState {
            card,
            ready: true,
            uses,
            damage: 0,
            horror: 0,
        });
    }
    let index = game
        .encounter_deck
        .iter()
        .position(|card| card.code == "01167")
        .unwrap();
    let card = game.encounter_deck.remove(index);
    game.frames.push(Frame::Revelation(PendingRevelation {
        card,
        drawn_by: owner,
        started: false,
        cancelled: false,
    }));
    game.set_fixture_tokens(vec![if case == CryptChillFixture::SignMagick {
        TokenFace::MinusTwo
    } else {
        TokenFace::Zero
    }]);
    game.history=vec!["Privileged original Crypt Chill witness with actual selected assets, permanent exclusions and exact encounter custody. [01167; RR Assets/Slots]".into()];
}
