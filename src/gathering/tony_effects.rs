//! Selected Tony timing effects: Gregory, Cigarette Case, Chamber and Quick Thinking.
//! Original 06162/04107/09029/02229, Legacy FAQ2.5 §1.7 and taboo10.

use super::runtime::{AssetState, Frame, Game, SkillTest, TestResultEffect};
use super::{Investigator, cards};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TonyEffectFixture {
    GregoryCigarette,
    GregoryLeaves,
    Chamber,
    ChamberExpires,
    QuickThinking,
    QuickSupport,
    WatchSuccess,
    WatchTie,
    WatchFailure,
    WatchSupport,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TonyTestState {
    wagers: Vec<Wager>,
    watch: Vec<WatchWager>,
    quick_owner: Option<Investigator>,
    pub last_ammo: Option<u16>,
    pub success_window_closed: bool,
}

pub(super) fn capture_commits(test: &mut SkillTest) {
    test.tony.quick_owner = test
        .committed
        .iter()
        .find(|card| card.card.code == "02229")
        .map(|card| card.owner);
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Wager {
    owner: Investigator,
    asset: u16,
    amount: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct WatchWager {
    owner: Investigator,
    card: u16,
    amount: u8,
    paid: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TonyRoundState {
    round: u16,
    chamber: Vec<(u16, u8)>,
    quick_used: bool,
}

fn choice(id: String, label: String) -> Choice {
    Choice {
        id,
        label,
        cancel_faces: vec![],
    }
}

pub(super) fn initiation_choices(game: &Game) -> Vec<Choice> {
    let Some(test) = &game.test else {
        return vec![];
    };
    let player = &game.players[test.tester.index()];
    if player.defeated || player.resigned {
        return vec![];
    }
    player.assets.iter().filter(|asset| asset.card.code == "06162" && !test.tony.wagers.iter().any(|wager| wager.asset == asset.card.id)).flat_map(|asset| {
        (1..=asset.uses.min(3)).map(move |amount| choice(format!("tony:gregory:{}:{amount}", asset.card.id), format!("Spend {amount} resources from Gregory Gry; gain {amount} if you succeed by at least {amount}")))
    }).collect()
}

pub(super) fn choose_initiation(game: &mut Game, id: &str) -> Result<(), RuleError> {
    if !initiation_choices(game)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError("That Gregory wager is unavailable.".into()));
    }
    let mut parts = id.split(':').skip(2);
    let asset: u16 = parts
        .next()
        .expect("Asset")
        .parse()
        .expect("Asset identity");
    let amount: u8 = parts.next().expect("Amount").parse().expect("Wager");
    let test = game.test.as_mut().expect("Initiating test");
    game.players[test.tester.index()]
        .assets
        .iter_mut()
        .find(|candidate| candidate.card.id == asset)
        .expect("Controlled Gregory")
        .uses -= amount;
    test.tony.wagers.push(Wager {
        owner: test.tester,
        asset,
        amount,
    });
    Ok(())
}

pub(super) fn result_effects(game: &Game, test: &SkillTest) -> Vec<TestResultEffect> {
    let (total, success, _) = game.result(test);
    if !success {
        return vec![];
    }
    let mut effects: Vec<_> = test
        .tony
        .wagers
        .iter()
        .filter(|wager| total - test.difficulty >= i32::from(wager.amount))
        .map(|wager| TestResultEffect::Resources {
            owner: wager.owner,
            amount: wager.amount,
            source: "06162".into(),
        })
        .collect();
    if total - test.difficulty >= 1 {
        effects.extend(
            test.tony
                .watch
                .iter()
                .filter(|wager| wager.paid && wager.amount > 0)
                .map(|wager| TestResultEffect::Resources {
                    owner: wager.owner,
                    amount: wager.amount * 2,
                    source: "03233".into(),
                }),
        );
    }
    effects
}

pub(super) fn commitment_choices(game: &Game) -> Vec<Choice> {
    let Some(test) = &game.test else {
        return vec![];
    };
    if test.window != super::runtime::Window::Commitment {
        return vec![];
    }
    let player = &game.players[test.tester.index()];
    let mut choices = Vec::new();
    for card in player
        .hand
        .iter()
        .filter(|card| card.code == "03233" && test.selected.contains(&card.id))
    {
        let other_cost: u8 = test
            .tony
            .watch
            .iter()
            .filter(|wager| {
                !wager.paid && wager.card != card.id && test.selected.contains(&wager.card)
            })
            .map(|wager| wager.amount)
            .sum();
        for amount in 0..=3.min(player.resources.saturating_sub(other_cost)) {
            choices.push(choice(format!("tony:watch:{}:{amount}", card.id), format!("Set Watch this! additional commitment cost to {amount} resources; gain {} on success by at least 1", amount * 2)));
        }
    }
    choices
}

pub(super) fn choose_commitment(game: &mut Game, id: &str) -> Result<(), RuleError> {
    if !commitment_choices(game)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError("That Watch this! wager is unavailable.".into()));
    }
    let mut parts = id.split(':').skip(2);
    let card: u16 = parts.next().expect("Card").parse().expect("Card identity");
    let amount: u8 = parts.next().expect("Wager").parse().expect("Wager amount");
    let test = game.test.as_mut().expect("Commitment test");
    test.tony.watch.retain(|wager| wager.card != card);
    test.tony.watch.push(WatchWager {
        owner: test.tester,
        card,
        amount,
        paid: false,
    });
    Ok(())
}

pub(super) fn reset_commitment(game: &mut Game, card: u16) {
    if let Some(test) = game.test.as_mut() {
        test.tony
            .watch
            .retain(|wager| wager.card != card || wager.paid);
    }
}

/// Called before moving the selected cards out of hand. Selection and wager
/// editing are proposals; the additional costs are paid together on confirmation.
pub(super) fn pay_commitment_costs(game: &mut Game) -> Result<(), RuleError> {
    let test = game.test.as_mut().expect("Confirming commitment");
    let player = &mut game.players[test.tester.index()];
    let selected: Vec<_> = player
        .hand
        .iter()
        .filter(|card| card.code == "03233" && test.selected.contains(&card.id))
        .map(|card| card.id)
        .collect();
    let cost: u8 = test
        .tony
        .watch
        .iter()
        .filter(|wager| !wager.paid && selected.contains(&wager.card))
        .map(|wager| wager.amount)
        .sum();
    player.resources = player
        .resources
        .checked_sub(cost)
        .ok_or_else(|| RuleError("The additional commitment costs are not affordable.".into()))?;
    test.tony
        .watch
        .retain(|wager| wager.paid || selected.contains(&wager.card));
    for card in selected {
        if let Some(wager) = test.tony.watch.iter_mut().find(|wager| wager.card == card) {
            wager.paid = true;
        } else {
            test.tony.watch.push(WatchWager {
                owner: test.tester,
                card,
                amount: 0,
                paid: true,
            });
        }
    }
    Ok(())
}

pub(super) fn success_choices(game: &Game) -> Vec<Choice> {
    let Some(test) = &game.test else {
        return vec![];
    };
    if test.tony.success_window_closed {
        return vec![];
    }
    let (total, success, _) = game.result(test);
    if !success || total - test.difficulty < 2 {
        return vec![];
    }
    let player = &game.players[test.tester.index()];
    if player.defeated || player.resigned {
        return vec![];
    }
    player
        .assets
        .iter()
        .filter(|asset| asset.card.code == "04107" && asset.ready)
        .map(|asset| {
            choice(
                format!("tony:case:{}", asset.card.id),
                "Exhaust Lucky Cigarette Case to draw 1 card after succeeding by at least 2".into(),
            )
        })
        .collect()
}

pub(super) fn choose_success(game: &mut Game, id: &str) -> Result<(), RuleError> {
    if !success_choices(game).iter().any(|choice| choice.id == id) {
        return Err(RuleError(
            "That Cigarette Case reaction is unavailable.".into(),
        ));
    }
    let owner = game.test.as_ref().expect("Successful test").tester;
    let asset: u16 = id
        .rsplit(':')
        .next()
        .expect("Asset")
        .parse()
        .expect("Asset identity");
    game.players[owner.index()]
        .assets
        .iter_mut()
        .find(|candidate| candidate.card.id == asset)
        .expect("Ready Cigarette Case")
        .ready = false;
    game.draw_player(owner, 1)
}

pub(super) fn close_success_window(game: &mut Game) {
    if let Some(test) = game.test.as_mut() {
        test.tony.success_window_closed = true;
    }
}

pub(super) fn attack_bonus(game: &Game, asset: u16) -> i32 {
    if game.tony.round != game.round {
        return 0;
    }
    game.tony
        .chamber
        .iter()
        .filter(|(id, _)| *id == asset)
        .map(|(_, bonus)| i32::from(*bonus))
        .sum()
}

pub(super) fn consume_attack_bonus(game: &mut Game, asset: u16) {
    game.tony.chamber.retain(|(id, _)| *id != asset);
}

pub(super) fn asset_left_play(game: &mut Game, asset: u16) {
    consume_attack_bonus(game, asset);
}

fn current_round(game: &mut Game) {
    if game.tony.round != game.round {
        game.tony = TonyRoundState {
            round: game.round,
            ..Default::default()
        };
    }
}

pub(super) fn after_test_choices(game: &Game, test: &SkillTest) -> Vec<Choice> {
    let mut choices = Vec::new();
    let player = &game.players[test.tester.index()];
    if let Some(asset) = test.tony.last_ammo
        && !player.defeated
        && !player.resigned
        && player.resources >= 1
        && !player.threats.iter().any(|card| card.code == "01165")
        && player.assets.iter().any(|candidate| {
            candidate.card.id == asset
                && cards::definition(&candidate.card.code)
                    .is_some_and(|card| card.has_trait("Firearm"))
        })
    {
        for card in player.hand.iter().filter(|card| card.code == "09029") {
            choices.push(choice(format!("tony:chamber:{}", card.id), "Play One in the Chamber for 1 resource: add 1 ammo; your next attack with this firearm this round gets +3 skill".into()));
        }
    }
    let (total, success, _) = game.result(test);
    if success
        && total - test.difficulty >= 2
        && (game.tony.round != game.round || !game.tony.quick_used)
        && let Some(owner) = test.tony.quick_owner
        && !game.players[owner.index()].defeated
        && !game.players[owner.index()].resigned
    {
        choices.push(choice(
            format!("tony:quick-thinking:{}", owner.id()),
            "Use Quick Thinking to take an immediate additional action (once per round)".into(),
        ));
    }
    choices
}

/// The consumer owns the granted-action continuation. Returning an investigator
/// requests that one concrete action, after the taboo allowance has been paid.
pub(super) fn choose_after_test(
    game: &mut Game,
    test: &SkillTest,
    id: &str,
) -> Result<Option<Investigator>, RuleError> {
    if !after_test_choices(game, test)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That Tony after-test effect is unavailable.".into(),
        ));
    }
    current_round(game);
    if id.starts_with("tony:quick-thinking:") {
        game.tony.quick_used = true;
        let owner = test.tony.quick_owner.expect("Committed Quick Thinking");
        return Ok(Some(owner));
    }
    let player = &mut game.players[test.tester.index()];
    let index = player
        .hand
        .iter()
        .position(|card| id == format!("tony:chamber:{}", card.id))
        .expect("Selected Chamber event");
    let card = player.hand.remove(index);
    player.resources -= 1;
    let asset = test.tony.last_ammo.expect("Attack spent final ammo");
    player
        .assets
        .iter_mut()
        .find(|candidate| candidate.card.id == asset)
        .expect("Controlled Firearm")
        .uses += 1;
    player.discard.push(card);
    game.tony.chamber.push((asset, 3));
    Ok(None)
}

pub(super) fn configure_fixture(game: &mut Game, fixture: TonyEffectFixture) {
    fn take(game: &mut Game, code: &str) -> super::CardView {
        let player = &mut game.players[Investigator::Tony.index()];
        for pile in [&mut player.hand, &mut player.deck, &mut player.discard] {
            if let Some(index) = pile.iter().position(|card| card.code == code) {
                return pile.remove(index);
            }
        }
        panic!("Selected Tony card missing: {code}");
    }
    fn equip(game: &mut Game, code: &str, uses: u8) {
        let card = take(game, code);
        game.players[0].assets.push(AssetState {
            card,
            ready: true,
            uses,
            damage: 0,
            horror: 0,
        });
    }
    for frame in std::mem::take(&mut game.frames) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    game.enable_fixture_turn(Investigator::Tony);
    game.lead = Investigator::Tony;
    game.scenario = super::scenario::ScenarioState::gathering_after_act_one();
    game.act = 2;
    game.agenda = 1;
    game.agenda_doom = 0;
    game.test = None;
    game.tony = Default::default();
    game.players[0].resources = 5;
    for player in &mut game.players {
        player.location = super::scenario::HALLWAY.into();
    }
    let enemies = if matches!(
        fixture,
        TonyEffectFixture::Chamber | TonyEffectFixture::ChamberExpires
    ) {
        3
    } else if matches!(
        fixture,
        TonyEffectFixture::QuickThinking | TonyEffectFixture::QuickSupport
    ) {
        2
    } else {
        1
    };
    for _ in 0..enemies {
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == "01160")
            .expect("Actual Minion");
        let card = game.encounter_deck.remove(index);
        let id = card.id;
        super::scenario::spawn_encounter_enemy(game, card, Investigator::Tony)
            .expect("Minion spawn");
        let enemy = game
            .scenario
            .enemies
            .iter_mut()
            .find(|enemy| enemy.card.id == id)
            .expect("Spawned Minion");
        if matches!(
            fixture,
            TonyEffectFixture::QuickThinking
                | TonyEffectFixture::QuickSupport
                | TonyEffectFixture::WatchSuccess
                | TonyEffectFixture::WatchTie
                | TonyEffectFixture::WatchFailure
                | TonyEffectFixture::WatchSupport
        ) {
            enemy.damage = 1;
        }
        if matches!(
            fixture,
            TonyEffectFixture::GregoryCigarette | TonyEffectFixture::GregoryLeaves
        ) {
            enemy.bounties = 1;
        }
    }
    match fixture {
        TonyEffectFixture::GregoryCigarette | TonyEffectFixture::GregoryLeaves => {
            equip(game, "06162", 9);
            equip(game, "04107", 0);
            equip(game, "06011", 3);
            game.players[0]
                .assets
                .iter_mut()
                .find(|asset| asset.card.code == "06010")
                .expect("Contracts")
                .uses = 5;
            let injury = take(game, "02038");
            game.players[0].deck.push(injury);
            game.set_fixture_tokens(vec![if fixture == TonyEffectFixture::GregoryLeaves {
                TokenFace::Tablet
            } else {
                TokenFace::MinusOne
            }]);
        }
        TonyEffectFixture::Chamber | TonyEffectFixture::ChamberExpires => {
            equip(game, "06011", 1);
            equip(game, "06011", 3);
            let chamber = take(game, "09029");
            game.players[0].hand.push(chamber);
            game.set_fixture_tokens(vec![
                TokenFace::MinusOne,
                TokenFace::MinusOne,
                TokenFace::MinusOne,
            ]);
            if fixture == TonyEffectFixture::ChamberExpires {
                // This source witness crosses a real phase boundary. Keep
                // actual benign draws fixed so unrelated weaknesses do not
                // replace the next-round bonus-expiry question.
                for player in &mut game.players {
                    let count = if player.investigator == Investigator::Patrice {
                        5
                    } else {
                        1
                    };
                    let mut top = Vec::new();
                    for _ in 0..count {
                        let index = player
                            .deck
                            .iter()
                            .position(|card| !card.weakness)
                            .expect("Actual ordinary draw");
                        top.push(player.deck.remove(index));
                    }
                    top.reverse();
                    player.deck.extend(top);
                }
                let mut encounters = Vec::new();
                for code in ["01165", "01165", "01168"] {
                    let index = game
                        .encounter_deck
                        .iter()
                        .position(|card| card.code == code)
                        .expect("Actual encounter copy");
                    encounters.push(game.encounter_deck.remove(index));
                }
                encounters.reverse();
                game.encounter_deck.extend(encounters);
            }
        }
        TonyEffectFixture::QuickThinking | TonyEffectFixture::QuickSupport => {
            let quick = take(game, "02229");
            game.players[0].hand.push(quick);
            if fixture == TonyEffectFixture::QuickSupport {
                let scene = take(game, "04103");
                game.players[0].hand.push(scene);
                game.enable_fixture_turn(Investigator::Patrice);
                game.set_fixture_tokens(vec![TokenFace::PlusOne]);
            } else {
                game.set_fixture_tokens(vec![TokenFace::MinusOne, TokenFace::MinusOne]);
            }
        }
        TonyEffectFixture::WatchSuccess
        | TonyEffectFixture::WatchTie
        | TonyEffectFixture::WatchFailure
        | TonyEffectFixture::WatchSupport => {
            let watch = take(game, "03233");
            game.players[0].hand.push(watch);
            game.set_fixture_tokens(vec![match fixture {
                TonyEffectFixture::WatchSuccess => TokenFace::MinusThree,
                TonyEffectFixture::WatchTie => TokenFace::MinusFour,
                TonyEffectFixture::WatchFailure => TokenFace::AutoFail,
                _ => TokenFace::MinusOne,
            }]);
            if fixture == TonyEffectFixture::WatchSupport {
                let other = take(game, "60114");
                game.players[0].hand.push(other);
                game.enable_fixture_turn(Investigator::Patrice);
            }
        }
    }
}
