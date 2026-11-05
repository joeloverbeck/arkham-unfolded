//! The selected Survivor events with explicit discard origins and recovery.

use super::{
    CardView, Investigator, cards,
    encounter_search::{EventEffect, PaidEvent, SearchDecision},
    runtime::{Frame, Game, Skill, TestOperation},
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerEventFixture {
    WingingHand,
    WingingDiscard,
    WingingFailure,
    Scrounge,
    ScroungeNoMatch,
    KeyringSuccess,
    KeyringFailure,
    KeyringLast,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum PlayerEventEffect {
    Winging {
        from_discard: bool,
        location: String,
        difficulty: i32,
    },
    Scrounge,
}

pub(super) fn is_action(action: &str) -> bool {
    matches!(action.split(':').nth(3), Some("04272" | "06165"))
}

pub(super) fn action_choices(game: &Game, owner: Investigator) -> Vec<Choice> {
    let player = &game.players[owner.index()];
    if player.defeated || player.resigned || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    let mut choices = vec![];
    for (from_discard, zone) in [(false, &player.hand), (true, &player.discard)] {
        for card in zone {
            let offered = match card.code.as_str() {
                "04272" => {
                    player.resources >= 1 && game.scenario.shroud(&player.location).is_some()
                }
                "06165" => !from_discard && !recovery_targets(game, owner).is_empty(),
                _ => false,
            };
            if offered {
                let origin = if card.code == "04272" {
                    if from_discard { ":discard" } else { ":hand" }
                } else {
                    ""
                };
                choices.push(Choice {
                    id: format!("play:tony:{}:{}{origin}", card.id, card.code),
                    label: format!(
                        "Play {}{}",
                        card.title,
                        if from_discard {
                            " from your discard pile"
                        } else {
                            ""
                        }
                    ),
                    cancel_faces: vec![],
                });
            }
        }
    }
    choices
}

pub(super) fn pay_action(
    game: &mut Game,
    owner: Investigator,
    action: &str,
) -> Result<PaidEvent, RuleError> {
    if !action_choices(game, owner)
        .iter()
        .any(|choice| choice.id == action)
    {
        return Err(RuleError(
            "That event or discard origin is unavailable.".into(),
        ));
    }
    let parts: Vec<_> = action.split(':').collect();
    let id: u16 = parts[2].parse().expect("Offered card");
    let from_discard = parts.get(4) == Some(&"discard");
    let location = game.players[owner.index()].location.clone();
    let difficulty = game
        .scenario
        .shroud(&location)
        .map(|shroud| (i32::from(shroud) - 1).max(0));
    let player = &mut game.players[owner.index()];
    let zone = if from_discard {
        &mut player.discard
    } else {
        &mut player.hand
    };
    let at = zone
        .iter()
        .position(|card| card.id == id)
        .expect("Offered origin");
    let card = zone.remove(at);
    let effect = if card.code == "04272" {
        player.resources -= 1;
        PlayerEventEffect::Winging {
            from_discard,
            location,
            difficulty: difficulty.expect("Investigable location"),
        }
    } else {
        PlayerEventEffect::Scrounge
    };
    Ok(PaidEvent {
        owner,
        card,
        effect: EventEffect::Survivor(effect),
    })
}

pub(super) fn resolve_action(
    game: &mut Game,
    event: PaidEvent,
    effect: PlayerEventEffect,
) -> Result<(), RuleError> {
    match effect {
        PlayerEventEffect::Winging {
            from_discard,
            location,
            difficulty,
        } => {
            let owner = event.owner;
            let source = event.card.clone();
            game.frames
                .push(Frame::EncounterSearch(SearchDecision::FinishEvent {
                    event,
                }));
            game.begin_test(
                TestOperation::Investigate {
                    location,
                    clues: if from_discard { 2 } else { 1 },
                },
                owner,
                Skill::Intellect,
                difficulty,
            )?;
            game.test.as_mut().expect("Event investigation").source_card = Some(source);
        }
        PlayerEventEffect::Scrounge => {
            if recovery_targets(game, event.owner).is_empty() {
                finish_event(game, event);
            } else {
                game.frames
                    .push(Frame::EncounterSearch(SearchDecision::Recover { event }));
            }
        }
    }
    Ok(())
}

fn recovery_targets(game: &Game, owner: Investigator) -> Vec<&CardView> {
    game.players[owner.index()]
        .discard
        .iter()
        .filter(|card| {
            cards::definition(&card.code).is_some_and(|definition| definition.level == Some(0))
        })
        .collect()
}

pub(super) fn recovery_choices(game: &Game, event: &PaidEvent) -> Vec<Choice> {
    recovery_targets(game, event.owner)
        .into_iter()
        .map(|card| Choice {
            id: format!("event:recover:{}", card.id),
            label: format!("Return {} to your hand", card.title),
            cancel_faces: vec![],
        })
        .collect()
}

pub(super) fn recover(game: &mut Game, event: PaidEvent, id: &str) -> Result<(), RuleError> {
    let target = recovery_targets(game, event.owner)
        .into_iter()
        .find(|card| id == format!("event:recover:{}", card.id))
        .ok_or_else(|| RuleError("That level-zero discard target is unavailable.".into()))?
        .id;
    let player = &mut game.players[event.owner.index()];
    let at = player
        .discard
        .iter()
        .position(|card| card.id == target)
        .expect("Offered discard");
    player.hand.push(player.discard.remove(at));
    finish_event(game, event);
    Ok(())
}

pub(super) fn finish_event(game: &mut Game, event: PaidEvent) {
    if matches!(
        event.effect,
        EventEffect::Survivor(PlayerEventEffect::Winging {
            from_discard: true,
            ..
        })
    ) {
        let owner = event.owner;
        let returned = event.card.clone();
        game.players[owner.index()].deck.push(event.card);
        for index in (1..game.players[owner.index()].deck.len()).rev() {
            let other = game.random_index(index + 1);
            game.players[owner.index()].deck.swap(index, other);
        }
        game.capture_shuffle(super::DrawSource::Investigator(owner), 0, vec![returned]);
        game.history.push(
            "Winging It played from discard resolves and shuffles into its owner's deck. [04272]"
                .into(),
        );
    } else {
        game.players[event.owner.index()].discard.push(event.card);
    }
}

pub(super) fn configure_fixture(game: &mut Game, case: PlayerEventFixture) {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        } else {
            panic!("Base encounter fixture");
        }
    }
    if matches!(
        case,
        PlayerEventFixture::KeyringSuccess
            | PlayerEventFixture::KeyringFailure
            | PlayerEventFixture::KeyringLast
    ) {
        let player = &mut game.players[Investigator::Patrice.index()];
        player.deck.append(&mut player.hand);
        player.location = "01114".into();
        let at = player
            .deck
            .iter()
            .position(|card| card.code == "60507")
            .expect("Selected Old Keyring");
        player.assets.push(super::runtime::AssetView {
            card: player.deck.remove(at),
            ready: true,
            uses: if case == PlayerEventFixture::KeyringLast {
                1
            } else {
                2
            },
            damage: 0,
            horror: 0,
        });
        if case == PlayerEventFixture::KeyringLast {
            let at = player
                .deck
                .iter()
                .position(|card| card.code == "03039")
                .expect("Actual Resourceful");
            player.hand.push(player.deck.remove(at));
        }
        game.enable_fixture_turn(Investigator::Patrice);
        game.set_fixture_tokens(vec![if case == PlayerEventFixture::KeyringFailure {
            TokenFace::MinusOne
        } else {
            TokenFace::Zero
        }]);
        game.history.clear();
        game.history.push(format!("Privileged {case:?} witness: actual controlled Keyring, stated remaining keys and the revealed Cellar's printed shroud4. No ordinary setup claim. [60507;01114]"));
        return;
    }
    let player = &mut game.players[Investigator::Patrice.index()];
    player.deck.append(&mut player.hand);
    player.location = "01113".into();
    player.ordinary_actions = 3;
    let scrounge = matches!(
        case,
        PlayerEventFixture::Scrounge | PlayerEventFixture::ScroungeNoMatch
    );
    let code = if scrounge { "06165" } else { "04272" };
    let at = player
        .deck
        .iter()
        .position(|card| card.code == code)
        .expect("Selected event");
    let card = player.deck.remove(at);
    if matches!(
        case,
        PlayerEventFixture::WingingDiscard | PlayerEventFixture::WingingFailure
    ) {
        player.discard.push(card);
    } else {
        player.hand.push(card);
    }
    if scrounge {
        let at = player
            .assets
            .iter()
            .position(|asset| asset.card.code == "06016")
            .expect("Actual Violin");
        player.discard.push(player.assets.remove(at).card);
        for code in if case == PlayerEventFixture::Scrounge {
            vec!["06017", "01080"]
        } else {
            vec!["06017"]
        } {
            let at = player
                .deck
                .iter()
                .position(|card| card.code == code)
                .expect("Actual recovery candidate");
            player.discard.push(player.deck.remove(at));
        }
    }
    game.enable_fixture_turn(Investigator::Patrice);
    game.set_fixture_tokens(vec![if case == PlayerEventFixture::WingingFailure {
        TokenFace::AutoFail
    } else {
        TokenFace::Zero
    }]);
    game.history.clear();
    game.history.push(format!("Privileged {case:?} witness: actual selected cards in the stated owned hand/discard zones, and the revealed Attic. No ordinary setup claim."));
}
