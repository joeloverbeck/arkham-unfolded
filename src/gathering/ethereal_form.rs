//! Original Ethereal Form: an evade and its round-duration restrictions.
use super::{
    Investigator,
    runtime::{Frame, Game},
    scenario,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EtherealFixture {
    Evade,
    FailedEvade,
    OtherInvestigator,
    Watcher,
    WatcherAttack,
    Expiry,
}

pub(super) fn is_ethereal(game: &Game, owner: Investigator) -> bool {
    game.ethereal.contains(&owner)
}

pub(super) fn succeeded(game: &mut Game, owner: Investigator) {
    if !is_ethereal(game, owner) {
        game.ethereal.push(owner);
    }
    let mut disengaged = Vec::new();
    for enemy in &mut game.scenario.enemies {
        if enemy.engaged == Some(owner) {
            enemy.engaged = None;
            disengaged.push(enemy.card.id);
        }
    }
    game.history.push(format!("{} becomes ethereal for the remainder of this round: other engaged enemies disengage without exhausting; enemies cannot engage them, and they cannot attack or deal damage to enemies. No movement is granted. [06164]", owner.name()));
    for enemy in disengaged {
        game.check_engagement(enemy);
    }
}

pub(super) fn expire(game: &mut Game) {
    if game.ethereal.is_empty() {
        return;
    }
    game.ethereal.clear();
    for enemy in game
        .scenario
        .enemies
        .iter()
        .map(|enemy| enemy.card.id)
        .collect::<Vec<_>>()
    {
        game.check_engagement(enemy);
    }
}

/// Resolution of WHEN-round-ends abilities stays above this continuation;
/// AT-round-end abilities stay below it. Lasting effects expire at the timing
/// point, before the AT abilities initiate (RR p14).
pub(super) fn queue_expiry(game: &mut Game) {
    if !game.ethereal.is_empty() {
        game.frames.push(Frame::ExpireEthereal);
    }
}

pub(super) fn configure_fixture(game: &mut Game, case: EtherealFixture) {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    game.scenario = scenario::ScenarioState::gathering_after_act_one();
    game.scenario.reveal(scenario::ATTIC);
    game.scenario.reveal(scenario::CELLAR);
    game.act = 2;
    for player in &mut game.players {
        player.location = scenario::ATTIC.into();
    }
    let owner = Investigator::Patrice;
    let player = &mut game.players[owner.index()];
    player.location = scenario::HALLWAY.into();
    player.resources = 5;
    let card = if let Some(index) = player.hand.iter().position(|card| card.code == "06164") {
        player.hand.remove(index)
    } else {
        let index = player
            .deck
            .iter()
            .position(|card| card.code == "06164")
            .expect("Selected Ethereal Form");
        player.deck.remove(index)
    };
    player.hand.push(card);
    for code in ["01160", "01159"] {
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == code)
            .expect("Selected encounter");
        let card = game.encounter_deck.remove(index);
        scenario::spawn_encounter_enemy(game, card, owner).expect("Actual enemy spawn");
    }
    game.enable_fixture_turn(owner);
    if case == EtherealFixture::OtherInvestigator {
        game.players[Investigator::Tony.index()].location = scenario::HALLWAY.into();
    }
    if matches!(
        case,
        EtherealFixture::Watcher | EtherealFixture::WatcherAttack
    ) {
        let player = &mut game.players[owner.index()];
        if !player.hand.iter().any(|card| card.code == "06017") {
            let at = player
                .deck
                .iter()
                .position(|card| card.code == "06017")
                .expect("Selected Watcher");
            player.hand.push(player.deck.remove(at));
        }
        if case == EtherealFixture::WatcherAttack {
            let at = player
                .deck
                .iter()
                .position(|card| !card.weakness)
                .expect("Ordinary remaining draw");
            let last = player.deck.remove(at);
            player.discard.append(&mut player.deck);
            player.deck.push(last);
        }
    }
    if case == EtherealFixture::Expiry {
        game.ethereal.push(owner);
        for enemy in &mut game.scenario.enemies {
            enemy.engaged = None;
        }
        game.actor = None;
        game.phase = super::runtime::Phase::RoundEnd;
        game.frames
            .push(Frame::Phase(super::phases::PhaseDecision::RoundEnd));
        let at = game
            .encounter_deck
            .iter()
            .position(|card| card.code == "01165")
            .expect("Selected Voices");
        game.players[owner.index()]
            .threats
            .push(game.encounter_deck.remove(at));
    }
    game.set_fixture_tokens(vec![if case == EtherealFixture::FailedEvade {
        crate::TokenFace::AutoFail
    } else {
        crate::TokenFace::Zero
    }]);
    game.history = vec![format!(
        "Privileged Ethereal Form witness {case:?}: Patrice has her actual event and original encounter enemies in Hallway. The named case supplies a hand Watcher/last draw, another investigator, or an existing round-duration effect at RoundEnd. A chosen zero/autofail is a verification token, not ordinary player control. [06164; 06017; RR Evade and lasting effects]"
    )];
}
