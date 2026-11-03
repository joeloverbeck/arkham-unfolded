//! Privileged source-bound positions for selected existing rules consumers.
//! These positions do not expose randomness or bypass choices in ordinary play.

use super::{
    CardView, Investigator,
    runtime::{AssetState, Frame, Game, PendingRevelation},
    scenario::{self, ScenarioState},
};
use crate::TokenFace;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectedWitness {
    Guts,
    GutsWeakness,
    Mauser,
    MauserNarrow,
    Mists,
    Initiative,
    GraspingHands,
    GraspingSuccess,
    ObscuringFog,
    ObscuringFogDuplicate,
    ObscuringFogNoClues,
}

fn take_player(game: &mut Game, who: Investigator, code: &str) -> CardView {
    let player = &mut game.players[who.index()];
    let index = player
        .deck
        .iter()
        .position(|card| card.code == code)
        .expect("Actual selected card");
    player.deck.remove(index)
}

fn take_encounter(game: &mut Game, code: &str) -> CardView {
    let index = game
        .encounter_deck
        .iter()
        .position(|card| card.code == code)
        .expect("Actual Gathering card");
    game.encounter_deck.remove(index)
}

fn equip(game: &mut Game, who: Investigator, code: &str, uses: u8) {
    let card = take_player(game, who, code);
    game.players[who.index()].assets.push(AssetState {
        card,
        ready: true,
        uses,
        damage: 0,
        horror: 0,
    });
}

fn revelation(game: &mut Game, code: &str) {
    let card = take_encounter(game, code);
    game.frames.push(Frame::Revelation(PendingRevelation {
        card,
        drawn_by: Investigator::Tony,
        started: false,
        cancelled: false,
    }));
}

pub(super) fn configure_fixture(game: &mut Game, case: SelectedWitness) {
    for frame in std::mem::take(&mut game.frames) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        } else {
            panic!("Expected the baseline encounter frame");
        }
    }
    game.scenario = ScenarioState::new();
    game.act = 1;
    game.agenda = 1;
    game.agenda_doom = 0;
    game.lead = Investigator::Tony;
    for player in &mut game.players {
        player.deck.append(&mut player.hand);
        player.location = scenario::STUDY.into();
        player.clues = 0;
        player.damage = 0;
        player.horror = 0;
    }
    game.enable_fixture_turn(Investigator::Tony);
    game.history.push(format!("Privileged selected-card witness {case:?}: actual cards retain physical identities; this is not ordinary setup."));
    game.set_fixture_tokens(vec![TokenFace::Zero]);
    match case {
        SelectedWitness::Guts | SelectedWitness::GutsWeakness => {
            for _ in 0..2 {
                let card = take_player(game, Investigator::Tony, "01089");
                game.players[0].hand.push(card);
            }
            let drawn = take_player(
                game,
                Investigator::Tony,
                if case == SelectedWitness::GutsWeakness {
                    "02038"
                } else {
                    "01049"
                },
            );
            game.players[0].deck.push(drawn);
            revelation(game, "01163");
        }
        SelectedWitness::Mauser | SelectedWitness::MauserNarrow => {
            equip(game, Investigator::Tony, "60306", 5);
            game.players[0].resources = 1;
            let card = take_encounter(game, "01160");
            scenario::spawn_encounter_enemy(game, card, Investigator::Tony).expect("Minion spawn");
            game.set_fixture_tokens(vec![if case == SelectedWitness::Mauser {
                TokenFace::MinusTwo
            } else {
                TokenFace::MinusThree
            }]);
        }
        SelectedWitness::Mists => {
            game.scenario = ScenarioState::gathering_after_act_one();
            game.act = 2;
            for player in &mut game.players {
                player.location = scenario::HALLWAY.into();
            }
            equip(game, Investigator::Patrice, "04029", 4);
            game.players[1].resources = 3;
            let card = take_encounter(game, "01160");
            scenario::spawn_encounter_enemy(game, card, Investigator::Patrice)
                .expect("Minion spawn");
            let card = take_encounter(game, "01161");
            game.scenario.enemies.push(scenario::EnemyState {
                card,
                location: scenario::CELLAR.into(),
                engaged: None,
                exhausted: false,
                damage: 0,
                doom: 0,
                bounties: 0,
                owner: None,
            });
            game.enable_fixture_turn(Investigator::Patrice);
        }
        SelectedWitness::Initiative => {
            let card = take_player(game, Investigator::Tony, "04150");
            game.players[0].hand.push(card);
            game.enable_fixture_turn(Investigator::Patrice);
            game.set_fixture_tokens(vec![TokenFace::Zero, TokenFace::Zero]);
        }
        SelectedWitness::GraspingHands | SelectedWitness::GraspingSuccess => {
            revelation(game, "01162");
            game.set_fixture_tokens(vec![if case == SelectedWitness::GraspingHands {
                TokenFace::MinusOne
            } else {
                TokenFace::PlusOne
            }]);
        }
        SelectedWitness::ObscuringFog
        | SelectedWitness::ObscuringFogDuplicate
        | SelectedWitness::ObscuringFogNoClues => {
            if case == SelectedWitness::ObscuringFogDuplicate {
                let existing = take_encounter(game, "01168");
                game.scenario.locations[0].fog = Some(existing);
            }
            if case == SelectedWitness::ObscuringFogNoClues {
                game.scenario.locations[0].clues = 0;
                game.players[1].clues = 6;
            }
            revelation(game, "01168");
            game.set_fixture_tokens(vec![TokenFace::Zero, TokenFace::PlusOne, TokenFace::Zero]);
        }
    }
}
