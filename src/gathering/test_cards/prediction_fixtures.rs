//! Source-derived #137 witnesses, reached only by the hostile-fixture constructor.
use super::*;
use crate::gathering::{CardView, Session};

fn take(game: &mut Game, who: Investigator, code: &str) -> CardView {
    let player = &mut game.players[who.index()];
    if let Some(at) = player.hand.iter().position(|card| card.code == code) {
        player.hand.remove(at)
    } else {
        let at = player
            .deck
            .iter()
            .position(|card| card.code == code)
            .expect("Actual selected card");
        player.deck.remove(at)
    }
}

pub(super) fn configure(game: &mut Game, case: TestCardFixture) {
    let mut session = Session::with_seed(9);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    *game = session.runtime.unwrap();
    let crystal = take(game, Investigator::Jacqueline, "60411");
    let crystal_id = crystal.id;
    game.players[2].hand.push(crystal);
    game.choose("turn:jacqueline").unwrap();
    game.choose(&format!("action:ordinary:play:{crystal_id}"))
        .unwrap();
    // Specify an ordinary next draw so unrelated weaknesses do not obscure the
    // independent margin witness. All selected card identities remain intact.
    let at = game.players[2]
        .deck
        .iter()
        .position(|card| !card.weakness)
        .unwrap();
    let card = game.players[2].deck.remove(at);
    game.players[2].deck.push(card);
    match case {
        TestCardFixture::CrystalWithGregory => {
            game.choose("turn:end").unwrap();
            game.choose("turn:tony").unwrap();
            let gregory = take(game, Investigator::Tony, "06162");
            let id = gregory.id;
            game.players[0].hand.push(gregory);
            game.choose(&format!("action:ordinary:play:{id}")).unwrap();
            game.choose("action:ordinary:investigate").unwrap();
        }
        TestCardFixture::CrystalReadSigns => {
            let signs = take(game, Investigator::Jacqueline, "06117");
            let promise = take(game, Investigator::Patrice, "07032");
            game.players[2].hand.push(signs);
            game.players[1].hand.push(promise);
            game.set_fixture_tokens(vec![TokenFace::PlusOne]);
        }
        TestCardFixture::CrystalFloor | TestCardFixture::CrystalAutoFail => {
            game.actor = None;
            game.phase = super::super::runtime::Phase::Mythos;
            game.frames.clear();
            let at = game
                .encounter_deck
                .iter()
                .position(|card| card.code == "01162")
                .unwrap();
            let card = game.encounter_deck.remove(at);
            game.frames.push(Frame::Revelation(PendingRevelation {
                card,
                drawn_by: Investigator::Patrice,
                started: false,
                cancelled: false,
            }));
            game.choose("revelation:resolve").unwrap();
            game.set_fixture_tokens(vec![if case == TestCardFixture::CrystalFloor {
                TokenFace::MinusFour
            } else {
                TokenFace::AutoFail
            }]);
        }
        TestCardFixture::CrystalAutoFailZero | TestCardFixture::CrystalVisionTie => {
            game.choose("turn:end").unwrap();
            game.choose("turn:patrice").unwrap();
            if case == TestCardFixture::CrystalAutoFailZero {
                let keyring = take(game, Investigator::Patrice, "60507");
                let id = keyring.id;
                game.players[1].hand.push(keyring);
                game.choose(&format!("action:ordinary:play:{id}")).unwrap();
                let action = game
                    .choices()
                    .into_iter()
                    .find(|choice| {
                        choice.id.starts_with("action:ordinary:investigate:")
                            && choice.id.ends_with(&format!(":{id}"))
                    })
                    .expect("Old Keyring investigation");
                game.choose(&action.id).unwrap();
                game.set_fixture_tokens(vec![TokenFace::AutoFail]);
            } else {
                let vision = take(game, Investigator::Patrice, "05041");
                game.players[1].threats.push(vision);
                game.choose("action:ordinary:investigate").unwrap();
                game.set_fixture_tokens(vec![TokenFace::Zero]);
            }
        }
        _ => unreachable!("Named prediction witnesses only"),
    }
    game.presentation.clear();
    game.history.push(format!("Privileged #137 {case:?} witness using actual selected cards and normal paid actions/test operations. Read the Signs: 3+5+1+4+1 versus Study 2 gives margin 12. Grasping Hands: agility 2, -4 or automatic failure versus 3 fails by 3. Old Keyring reduces Study to zero; Vision makes an intellect-2 tie fail by zero. [60411; 06117; 07032; 01111; 01162; 60507; 05041; RR ST6]"));
}
