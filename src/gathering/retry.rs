//! Patrice's delayed elder sign and the completed-test Live and Learn window.
use super::{
    CardView, Investigator,
    runtime::{AssetState, Frame, Game, SkillTest},
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryFixture {
    BasicFight,
    ChargedFight,
    HandWatcher,
    ChargedDrawback,
    LethalFight,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct AfterTestState {
    pub test: SkillTest,
    pub patrice_pending: bool,
}

fn choice(id: String, label: String) -> Choice {
    Choice {
        id,
        label,
        cancel_faces: vec![],
    }
}
pub(super) fn begin(game: &mut Game, test: SkillTest) -> Result<(), RuleError> {
    let player = &game.players[test.tester.index()];
    if player.defeated || player.resigned {
        return Ok(());
    }
    let patrice_pending =
        test.tester == Investigator::Patrice && test.revealed.contains(&TokenFace::ElderSign);
    let state = AfterTestState {
        test,
        patrice_pending,
    };
    if !choices(game, &state).is_empty() {
        game.frames.push(Frame::AfterTest(state));
    }
    Ok(())
}
pub(super) fn choices(game: &Game, state: &AfterTestState) -> Vec<Choice> {
    if state.patrice_pending {
        let mut choices = vec![choice(
            "patrice-shuffle:decline".into(),
            "Decline Patrice's after-test discard shuffle".into(),
        )];
        if game.players[1].discard.len() > 1 {
            choices.extend(game.players[1].discard.iter().map(|card| {
                choice(
                    format!("patrice-shuffle:keep:{}", card.id),
                    format!(
                        "Keep {} in discard and shuffle every other discarded card into the deck",
                        card.title
                    ),
                )
            }));
        }
        return choices;
    }
    let mut choices = super::test_cards::after_test_choices(game, &state.test);
    choices.extend(super::tony_effects::after_test_choices(game, &state.test));
    let player = &game.players[state.test.tester.index()];
    if !state.test.final_result.expect("Completed outcome").1
        && !player.defeated
        && !player.resigned
        && !player.threats.iter().any(|card| card.code == "01165")
    {
        choices.extend(player.hand.iter().filter(|card|card.code=="04200").map(|card|choice(format!("live-learn:{}",card.id),"Play Live and Learn: retry this test with +2, paying no second action or activation cost".into())));
    }
    if !choices.is_empty() {
        choices.push(choice(
            "after-test:pass".into(),
            "Finish after-test opportunities".into(),
        ));
    }
    choices
}
pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::AfterTest(mut state)) = game.frames.pop() else {
        return Err(RuleError("No completed-test window is pending.".into()));
    };
    if state.patrice_pending {
        state.patrice_pending = false;
        if let Some(id) = id.strip_prefix("patrice-shuffle:keep:") {
            let id: u16 = id
                .parse()
                .map_err(|_| RuleError("Invalid retained discard card.".into()))?;
            let player = &mut game.players[1];
            let retained = player.discard.remove(
                player
                    .discard
                    .iter()
                    .position(|card| card.id == id)
                    .expect("Offered retained card"),
            );
            let recycled = player.discard.len();
            player.deck.append(&mut player.discard);
            player.discard.push(retained);
            shuffle_player(game, Investigator::Patrice);
            game.capture_shuffle(
                super::DrawSource::Investigator(Investigator::Patrice),
                recycled,
                vec![],
            );
            game.history.push("Patrice resolves the delayed elder-sign ability after ST8: keep the selected discard and shuffle all other discarded cards into the deck, before optional after-test events. [06005; RR pp3,8]".into());
        }
        game.frames.push(Frame::AfterTest(state));
    } else if let Some(id) = id.strip_prefix("live-learn:") {
        let owner = state.test.tester;
        let id: u16 = id
            .parse()
            .map_err(|_| RuleError("Invalid Live and Learn identity.".into()))?;
        let player = &mut game.players[owner.index()];
        let event = player.hand.remove(
            player
                .hand
                .iter()
                .position(|card| card.id == id)
                .expect("Offered retry event"),
        );
        if let Some(actor) = game.action_in_progress.take() {
            game.complete_action(actor);
        }
        let original = state.clone();
        game.frames.push(Frame::AfterTest(original));
        game.frames.push(Frame::RetryEvent { owner, event });
        game.begin_test(
            state.test.operation,
            owner,
            state.test.skill,
            state.test.difficulty,
        )?;
        let test = game.test.as_mut().expect("Retried test");
        test.source_card = state.test.source_card;
        test.bonus = 2;
        test.record_bonus("Live and Learn", owner, 2);
        test.is_retry = true;
        game.history.push("Live and Learn retries the completed test with +2. Previous consequences remain; commitments, token results and test-limited boosts start afresh, with no second action or charge payment. [04200; P2/P4/P5]".into());
    } else if id.starts_with("tony:") {
        let granted = super::tony_effects::choose_after_test(game, &state.test, id)?;
        game.frames.push(Frame::AfterTest(state));
        if let Some(owner) = granted {
            game.begin_granted_action(owner);
        }
    } else if id != "after-test:pass" {
        let index = game.frames.len();
        super::test_cards::choose_after_test(game, &mut state.test, id)?;
        // The concrete card may push draw/other mandatory continuations. Keep
        // this opportunity below them, not above a nested revelation.
        game.frames.insert(index, Frame::AfterTest(state));
    }
    Ok(())
}
fn shuffle_player(game: &mut Game, who: Investigator) {
    super::shuffle(&mut game.random, &mut game.players[who.index()].deck);
}
fn take(player: &mut super::runtime::PlayerState, code: &str) -> CardView {
    if let Some(index) = player.hand.iter().position(|card| card.code == code) {
        player.hand.remove(index)
    } else {
        player.deck.remove(
            player
                .deck
                .iter()
                .position(|card| card.code == code)
                .expect("Selected card"),
        )
    }
}
pub(super) fn configure_fixture(game: &mut Game, case: RetryFixture) {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    let player = &mut game.players[1];
    player.deck.append(&mut player.hand);
    for asset in player.assets.drain(..) {
        player.deck.push(asset.card);
    }
    player.damage = 0;
    player.horror = 0;
    player.resources = 3;
    player.location = "01112".into();
    let event = take(player, "04200");
    player.hand.push(event);
    if matches!(case, RetryFixture::BasicFight | RetryFixture::LethalFight) {
        let card = take(player, "04201");
        player.hand.push(card);
        for code in ["04029", "01093"] {
            let card = take(player, code);
            player.discard.push(card);
        }
        for code in ["60507", "01080"] {
            let card = take(player, code);
            player.deck.push(card);
        }
    } else {
        let code = if matches!(
            case,
            RetryFixture::ChargedFight | RetryFixture::ChargedDrawback
        ) {
            "60407"
        } else {
            "04029"
        };
        let card = take(player, code);
        player.assets.push(AssetState {
            card,
            ready: true,
            uses: 1,
            damage: 0,
            horror: 0,
        });
    }
    if case == RetryFixture::HandWatcher {
        let card = take(&mut game.players[1], "06017");
        game.players[1].hand.push(card);
    } else {
        let priest = super::scenario::advance_act_two(game).expect("Priest");
        let enemy = game
            .scenario
            .enemies
            .iter_mut()
            .find(|enemy| enemy.card.id == priest)
            .unwrap();
        enemy.location = "01112".into();
        enemy.engaged = Some(Investigator::Patrice);
    }
    game.players[0].location = "01113".into();
    game.players[2].location = "01114".into();
    if case == RetryFixture::LethalFight {
        game.players[1].damage = game.players[1].health - 2;
    }
    game.set_fixture_tokens(match case {
        RetryFixture::BasicFight | RetryFixture::LethalFight => {
            vec![TokenFace::ElderSign, TokenFace::Zero]
        }
        RetryFixture::ChargedFight => vec![TokenFace::MinusOne, TokenFace::MinusOne],
        RetryFixture::ChargedDrawback => vec![TokenFace::MinusOne, TokenFace::Zero],
        RetryFixture::HandWatcher => vec![TokenFace::Zero, TokenFace::Zero],
    });
    game.enable_fixture_turn(Investigator::Patrice);
    game.history = vec![format!(
        "Privileged source-derived {case:?} retry witness using actual selected card custody. [P2/P4/P5]"
    )];
}
