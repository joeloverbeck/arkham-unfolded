//! Selected test initiation, prediction and commitment effects.
//! Expectations come from J7/P2–P6 and Legacy FAQ 2.5 §1.7, not from card-text parsing.

use super::{
    Investigator,
    runtime::{
        AssetView, Frame, Game, HarmOrigin, PendingRevelation, Skill, SkillTest, TestOperation,
        TestResultEffect,
    },
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

mod prediction_fixtures;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestCardFixture {
    CrystalPrediction,
    CrystalReadSigns,
    CrystalFloor,
    CrystalAutoFail,
    CrystalAutoFailZero,
    CrystalVisionTie,
    CrystalWithGregory,
    PrescientResolved,
    PrescientIgnoredOnly,
    PromiseCurse,
    PromiseAtLimit,
    PromiseSealedLimit,
    PromiseCancelled,
    ResourcefulSuccess,
    ResourcefulFailure,
    ResourcefulNoMatch,
    LookFound,
    LookFoundTooFar,
    LookFoundAutoFail,
    TonyElderSign,
    TonyElderSignCancelled,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct TestCardState {
    predictions: Vec<Prediction>,
    prescient: Vec<Prescient>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Prediction {
    owner: Investigator,
    asset: u16,
    number: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum TokenClass {
    Even,
    Odd,
    Symbol,
}

impl TokenClass {
    fn name(self) -> &'static str {
        match self {
            Self::Even => "even",
            Self::Odd => "odd",
            Self::Symbol => "symbol",
        }
    }
    fn matches(self, token: TokenFace) -> bool {
        match self {
            Self::Even => matches!(
                token,
                TokenFace::Zero | TokenFace::MinusTwo | TokenFace::MinusFour
            ),
            Self::Odd => matches!(
                token,
                TokenFace::PlusOne | TokenFace::MinusOne | TokenFace::MinusThree
            ),
            Self::Symbol => matches!(
                token,
                TokenFace::Skull
                    | TokenFace::Cultist
                    | TokenFace::Tablet
                    | TokenFace::AutoFail
                    | TokenFace::ElderSign
                    | TokenFace::Curse
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Prescient {
    owner: Investigator,
    card: u16,
    class: TokenClass,
    used: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum TestCardDecision {
    Initiation,
    Resourceful { owner: Investigator, card: u16 },
    CommitEffects { pending: Vec<CommitEffect> },
    Prescient { owner: Investigator, card: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CommitEffect {
    owner: Investigator,
    card: u16,
    code: String,
}

fn choice(id: impl Into<String>, label: impl Into<String>) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        cancel_faces: vec![],
    }
}

fn pendulums(game: &Game) -> Vec<(Investigator, u16)> {
    let Some(test) = &game.test else {
        return vec![];
    };
    let location = &game.players[test.tester.index()].location;
    game.players
        .iter()
        .filter(|player| !player.defeated && !player.resigned && player.location == *location)
        .flat_map(|player| {
            player
                .assets
                .iter()
                .filter(|asset| {
                    asset.card.code == "60411"
                        && asset.ready
                        && !test
                            .card_effects
                            .predictions
                            .iter()
                            .any(|prediction| prediction.asset == asset.card.id)
                })
                .map(|asset| (player.investigator, asset.card.id))
        })
        .collect()
}

/// A current legal source for the native number picker. This is derived from
/// the running decision; opening or editing a picker never changes game state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendulumPrompt {
    pub owner: Investigator,
    pub asset: u16,
}

/// Stands for the named number in the one listed prediction choice per ready
/// Pendulum. A player chooses that identity with a whole number in its place.
pub const PREDICTION_MARGIN: &str = "<margin>";

pub(super) fn prediction_prompts(game: &Game) -> Vec<PendulumPrompt> {
    match game.frames.last() {
        Some(Frame::TestCard(TestCardDecision::Initiation)) => pendulums(game)
            .into_iter()
            .map(|(owner, asset)| PendulumPrompt { owner, asset })
            .collect(),
        _ => vec![],
    }
}

fn prediction_choice(owner: Investigator, asset: u16) -> Choice {
    choice(
        format!("test-card:predict:{asset}:{PREDICTION_MARGIN}"),
        format!(
            "{}: exhaust Crystal Pendulum and predict {PREDICTION_MARGIN} (a whole number from 0 to {})",
            owner.name(),
            u16::MAX
        ),
    )
}

fn direct_prediction(game: &Game, id: &str) -> Option<(PendulumPrompt, u16)> {
    let (asset, number) = id.strip_prefix("test-card:predict:")?.split_once(':')?;
    let (asset, number) = (asset.parse::<u16>().ok()?, number.parse::<u16>().ok()?);
    // One canonical journal spelling, including zero and the full u16 range.
    if id != format!("test-card:predict:{asset}:{number}") {
        return None;
    }
    prediction_prompts(game)
        .into_iter()
        .find(|prompt| prompt.asset == asset)
        .map(|prompt| (prompt, number))
}

pub(super) fn direct_choice(game: &Game, id: &str) -> Option<Choice> {
    if id == "test-card:continue" {
        if !matches!(
            game.frames.last(),
            Some(Frame::TestCard(TestCardDecision::Initiation))
        ) {
            return None;
        }
        let remaining = choices(game);
        return (remaining.len() == 1 && remaining[0].id == "test-card:pass")
            .then(|| choice(id, "Continue automatically after test-begin reactions"));
    }
    let (prompt, number) = direct_prediction(game, id)?;
    Some(choice(
        id,
        format!(
            "{}: exhaust Crystal Pendulum and predict {number}",
            prompt.owner.name()
        ),
    ))
}

fn predict(game: &mut Game, owner: Investigator, asset: u16, number: u16) {
    game.players[owner.index()]
        .assets
        .iter_mut()
        .find(|held| held.card.id == asset)
        .expect("Eligible Pendulum")
        .ready = false;
    game.test
        .as_mut()
        .expect("Current test")
        .card_effects
        .predictions
        .push(Prediction {
            owner,
            asset,
            number,
        });
    game.history.push(format!("{} exhausts Crystal Pendulum and predicts a margin of {number} before commitments or token revelation. [60411]", owner.name()));
}

pub(super) fn begin_test(game: &mut Game) -> Result<(), RuleError> {
    if !pendulums(game).is_empty() || !super::tony_effects::initiation_choices(game).is_empty() {
        game.frames
            .push(Frame::TestCard(TestCardDecision::Initiation));
    }
    Ok(())
}

pub(super) fn committed(game: &mut Game) -> Result<(), RuleError> {
    let pending = game
        .test
        .as_ref()
        .expect("Commitment test")
        .committed
        .iter()
        .filter(|entry| matches!(entry.card.code.as_str(), "60419" | "07032"))
        .map(|entry| CommitEffect {
            owner: entry.owner,
            card: entry.card.id,
            code: entry.card.code.clone(),
        })
        .collect();
    continue_commit_effects(game, pending)
}

fn continue_commit_effects(
    game: &mut Game,
    mut pending: Vec<CommitEffect>,
) -> Result<(), RuleError> {
    match pending.len() {
        0 => Ok(()),
        1 => apply_commit_effect(game, pending.remove(0)),
        _ => {
            game.frames
                .push(Frame::TestCard(TestCardDecision::CommitEffects { pending }));
            Ok(())
        }
    }
}

fn apply_commit_effect(game: &mut Game, effect: CommitEffect) -> Result<(), RuleError> {
    if effect.code == "60419" {
        game.frames
            .push(Frame::TestCard(TestCardDecision::Prescient {
                owner: effect.owner,
                card: effect.card,
            }));
    } else if super::tokens::add_curse(game) {
        game.history.push(format!(
            "{} commits Promise of Power and adds one available curse token. [07032]",
            effect.owner.name()
        ));
    } else {
        game.history.push(format!("{} commits Promise of Power; all ten curse tokens are held, so its controller takes 2 horror. [07032; Innsmouth p1]", effect.owner.name()));
        game.queue_harm(effect.owner, 0, 2, false, "07032", HarmOrigin::PlayerCard)?;
    }
    Ok(())
}

pub(super) fn decision_owner(game: &Game) -> Option<Investigator> {
    match game.frames.last()? {
        Frame::TestCard(
            TestCardDecision::Prescient { owner, .. } | TestCardDecision::Resourceful { owner, .. },
        ) => Some(*owner),
        Frame::TestCard(TestCardDecision::Initiation) => game.test.as_ref().map(|test| test.tester),
        Frame::TestCard(TestCardDecision::CommitEffects { .. }) => Some(game.lead),
        _ => None,
    }
}

pub(super) fn choices(game: &Game) -> Vec<Choice> {
    match game.frames.last() {
        Some(Frame::TestCard(TestCardDecision::Resourceful { owner, card })) => {
            resourceful_targets(game, *owner)
                .into_iter()
                .map(|target| {
                    choice(
                        format!("resourceful:{card}:{}", target.id),
                        format!("Return {} to {}'s hand", target.title, owner.name()),
                    )
                })
                .collect()
        }
        Some(Frame::TestCard(TestCardDecision::CommitEffects { pending })) => pending
            .iter()
            .map(|effect| {
                choice(
                    format!("test-card:commit-effect:{}", effect.card),
                    format!(
                        "Resolve {}'s {} commitment effect next",
                        effect.owner.name(),
                        if effect.code == "07032" {
                            "Promise of Power"
                        } else {
                            "Prescient"
                        }
                    ),
                )
            })
            .collect(),
        Some(Frame::TestCard(TestCardDecision::Prescient { .. })) => {
            [TokenClass::Even, TokenClass::Odd, TokenClass::Symbol]
                .into_iter()
                .map(|class| {
                    choice(
                        format!("test-card:prescient:{}", class.name()),
                        format!("Prescient: name {} before revealing tokens", class.name()),
                    )
                })
                .collect()
        }
        Some(Frame::TestCard(TestCardDecision::Initiation)) => {
            let mut choices = vec![choice(
                "test-card:pass",
                "Continue after test-begin reactions",
            )];
            choices.extend(
                pendulums(game)
                    .into_iter()
                    .map(|(owner, asset)| prediction_choice(owner, asset)),
            );
            choices.extend(super::tony_effects::initiation_choices(game));
            choices
        }
        _ => vec![],
    }
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    if id == "test-card:continue" {
        if direct_choice(game, id).is_none() {
            return Err(RuleError(
                "A real test-begin reaction remains available.".into(),
            ));
        }
        return choose(game, "test-card:pass");
    }
    if id.ends_with(&format!(":{PREDICTION_MARGIN}")) {
        return Err(RuleError(format!(
            "Name the predicted margin: replace {PREDICTION_MARGIN} with a whole number from 0 to {}.",
            u16::MAX
        )));
    }
    if let Some((prompt, number)) = direct_prediction(game, id) {
        game.frames.pop();
        predict(game, prompt.owner, prompt.asset, number);
        // Keep genuine remaining reactions. An empty window closes in this
        // same transaction, so Undo returns to the meaningful prediction.
        return begin_test(game);
    }
    if !choices(game).iter().any(|choice| choice.id == id) {
        return Err(RuleError("This test-card decision is unavailable.".into()));
    }
    let Some(Frame::TestCard(decision)) = game.frames.pop() else {
        unreachable!()
    };
    match decision {
        TestCardDecision::Resourceful { owner, card } => {
            let target = resourceful_targets(game, owner)
                .into_iter()
                .find(|target| id == format!("resourceful:{card}:{}", target.id))
                .expect("Offered recovery")
                .id;
            let player = &mut game.players[owner.index()];
            let at = player
                .discard
                .iter()
                .position(|held| held.id == target)
                .expect("Discard target");
            let recovered = player.discard.remove(at);
            game.history.push(format!(
                "{} returns {} with Resourceful at ST7. [03039]",
                owner.name(),
                recovered.title
            ));
            player.hand.push(recovered);
        }
        TestCardDecision::CommitEffects { mut pending } => {
            let at = pending
                .iter()
                .position(|effect| id == format!("test-card:commit-effect:{}", effect.card))
                .expect("Offered commitment effect");
            let effect = pending.remove(at);
            // Keep the remaining triggers below this one's choices and harm.
            if !pending.is_empty() {
                game.frames
                    .push(Frame::TestCard(TestCardDecision::CommitEffects { pending }));
            }
            apply_commit_effect(game, effect)?;
        }
        TestCardDecision::Prescient { owner, card } => {
            let class = [TokenClass::Even, TokenClass::Odd, TokenClass::Symbol]
                .into_iter()
                .find(|class| id == format!("test-card:prescient:{}", class.name()))
                .expect("Offered token class");
            game.test
                .as_mut()
                .expect("Commitment test")
                .card_effects
                .prescient
                .push(Prescient {
                    owner,
                    card,
                    class,
                    used: false,
                });
            game.history.push(format!("{} commits Prescient and names {} before revelation. After-test qualification uses retained resolved tokens; the ignored-only case follows the explicitly qualified legacy interpretation. [60419; J7]",owner.name(),class.name()));
        }
        TestCardDecision::Initiation => {
            if id.starts_with("tony:gregory:") {
                game.frames
                    .push(Frame::TestCard(TestCardDecision::Initiation));
                super::tony_effects::choose_initiation(game, id)?;
            }
            // Otherwise the offered pass closes the test-start window.
        }
    }
    Ok(())
}

pub(super) fn base_skill(game: &Game, who: Investigator, skill: Skill) -> i32 {
    i32::from(game.players[who.index()].base_skills[skill.index()])
}

pub(super) fn skill_modifier(game: &Game, test: &SkillTest) -> i32 {
    super::test_report::static_bonuses(game, test)
        .iter()
        .map(|row| row.amount)
        .sum()
}

pub(super) fn result_effects(game: &Game, test: &SkillTest) -> Vec<TestResultEffect> {
    let Some((total, success, failed_by)) = test.final_result else {
        return vec![];
    };
    let margin = if success {
        (total - test.difficulty).max(0)
    } else {
        failed_by
    };
    let mut effects: Vec<_> = test
        .card_effects
        .predictions
        .iter()
        .filter(|prediction| {
            i32::from(prediction.number) == margin
                && !game.players[prediction.owner.index()].defeated
                && !game.players[prediction.owner.index()].resigned
        })
        .map(|prediction| TestResultEffect::Draw {
            owner: prediction.owner,
        })
        .collect();
    if success {
        if let Some(source) = &test.source_card
            && source.code == "60507"
        {
            effects.push(TestResultEffect::OldKeyring {
                owner: test.tester,
                asset: source.id,
            });
        }
        effects.extend(
            test.committed
                .iter()
                .filter(|entry| entry.card.code == "03039")
                .map(|entry| TestResultEffect::Resourceful {
                    owner: entry.owner,
                    card: entry.card.id,
                }),
        );
    }
    effects
}

pub(super) fn failure_choices(game: &Game) -> Vec<Choice> {
    let Some(test) = &game.test else {
        return vec![];
    };
    if !matches!(test.final_result, Some((_, false, 0..=2)))
        || !matches!(test.operation, TestOperation::Investigate { .. })
    {
        return vec![];
    }
    let player = &game.players[test.tester.index()];
    if player.defeated
        || player.resigned
        || player.resources < 2
        || player.threats.iter().any(|card| card.code == "01165")
        || !game
            .scenario
            .location(&player.location)
            .is_some_and(|location| location.clues > 0)
    {
        return vec![];
    }
    player
        .hand
        .iter()
        .filter(|card| card.code == "01079")
        .map(|card| {
            choice(
                format!("test-card:found:{}", card.id),
                "Pay 2 for Look what I found!: discover 2 clues after this failed investigation",
            )
        })
        .collect()
}

pub(super) fn spend_key(game: &mut Game, owner: Investigator, asset: u16) -> Result<(), RuleError> {
    let player = &mut game.players[owner.index()];
    if let Some(at) = player.assets.iter().position(|held| held.card.id == asset) {
        let keyring = &mut player.assets[at];
        keyring.uses = keyring.uses.saturating_sub(1);
        if keyring.uses == 0 {
            player.discard.push(player.assets.remove(at).card);
        }
        game.history.push(format!("{} removes a key after the successful investigation, discarding Old Keyring if it has no keys. [60507]",owner.name()));
    }
    Ok(())
}
pub(super) fn choose_failure(game: &mut Game, id: &str) -> Result<(), RuleError> {
    if !failure_choices(game).iter().any(|choice| choice.id == id) {
        return Err(RuleError("That failed-test event is not available.".into()));
    }
    let owner = game.test.as_ref().expect("Failed test").tester;
    let player = &mut game.players[owner.index()];
    let at = player
        .hand
        .iter()
        .position(|card| id == format!("test-card:found:{}", card.id))
        .expect("Offered event");
    let event = player.hand.remove(at);
    player.resources -= 2;
    let location = game
        .scenario
        .location_mut(&player.location)
        .expect("Current location");
    let clues = location.clues.min(2);
    location.clues -= clues;
    player.clues += clues;
    player.discard.push(event);
    game.history.push(format!("{} pays 2 and discovers {clues} clues with Look what I found! at ST6; the settled failed-test result is unchanged. [01079; FAQ1.7]",owner.name()));
    Ok(())
}

fn resourceful_targets(game: &Game, owner: Investigator) -> Vec<&super::CardView> {
    let player = &game.players[owner.index()];
    if player.defeated || player.resigned {
        return vec![];
    }
    player
        .discard
        .iter()
        .filter(|card| {
            card.code != "03039"
                && super::cards::definition(&card.code).is_some_and(|definition| {
                    definition
                        .factions
                        .contains(&super::cards::Faction::Survivor)
                })
        })
        .collect()
}

pub(super) fn start_resourceful(
    game: &mut Game,
    owner: Investigator,
    card: u16,
) -> Result<(), RuleError> {
    if resourceful_targets(game, owner).is_empty() {
        game.history.push(format!(
            "{} has no eligible Survivor card for Resourceful at ST7. [03039]",
            owner.name()
        ));
    } else {
        game.frames
            .push(Frame::TestCard(TestCardDecision::Resourceful {
                owner,
                card,
            }));
    }
    Ok(())
}

pub(super) fn after_test_choices(game: &Game, test: &SkillTest) -> Vec<Choice> {
    let mut choices = vec![];
    for prediction in &test.card_effects.prescient {
        let player = &game.players[prediction.owner.index()];
        // This localized predicate intentionally follows the qualified J7
        // interpretation; raw ignored/cancelled reveals are not conflated with it.
        if prediction.used
            || player.defeated
            || player.resigned
            || !test
                .revealed
                .iter()
                .any(|&face| prediction.class.matches(face))
        {
            continue;
        }
        for card in &player.discard {
            if super::cards::definition(&card.code).is_some_and(|card| card.has_trait("Spell")) {
                choices.push(choice(
                    format!("prescient:return:{}:{}", prediction.card, card.id),
                    format!(
                        "{}: use Prescient to return {} to hand",
                        player.investigator.name(),
                        card.title
                    ),
                ));
            }
        }
    }
    choices
}
pub(super) fn choose_after_test(
    game: &mut Game,
    test: &mut SkillTest,
    id: &str,
) -> Result<(), RuleError> {
    if !after_test_choices(game, test)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That after-test card effect is unavailable.".into(),
        ));
    }
    let parts: Vec<_> = id.split(':').collect();
    let source: u16 = parts[2].parse().expect("Offered Prescient");
    let target: u16 = parts[3].parse().expect("Offered Spell");
    let prediction = test
        .card_effects
        .prescient
        .iter_mut()
        .find(|prediction| prediction.card == source)
        .expect("Recorded prediction");
    prediction.used = true;
    let owner = &mut game.players[prediction.owner.index()];
    let at = owner
        .discard
        .iter()
        .position(|card| card.id == target)
        .expect("Available target");
    let card = owner.discard.remove(at);
    game.history.push(format!(
        "{} resolves Prescient after test cleanup and returns {} from discard to hand. [60419; J7]",
        owner.investigator.name(),
        card.title
    ));
    owner.hand.push(card);
    Ok(())
}

pub(super) fn configure_fixture(game: &mut Game, case: TestCardFixture) {
    assert!(game.test.is_none());
    if matches!(
        case,
        TestCardFixture::CrystalReadSigns
            | TestCardFixture::CrystalFloor
            | TestCardFixture::CrystalAutoFail
            | TestCardFixture::CrystalAutoFailZero
            | TestCardFixture::CrystalVisionTie
            | TestCardFixture::CrystalWithGregory
    ) {
        prediction_fixtures::configure(game, case);
        return;
    }
    game.history.clear();
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        } else {
            panic!("Expected base fixture encounter only");
        }
    }
    if matches!(
        case,
        TestCardFixture::TonyElderSign | TestCardFixture::TonyElderSignCancelled
    ) {
        let player = &mut game.players[Investigator::Tony.index()];
        player.deck.append(&mut player.hand);
        if case == TestCardFixture::TonyElderSignCancelled {
            game.players[2].location = game.players[0].location.clone();
        }
        let at = game
            .encounter_deck
            .iter()
            .position(|card| card.code == "01163")
            .expect("Actual Rotting Remains");
        let card = game.encounter_deck.remove(at);
        let encounter = card.id;
        game.frames.push(Frame::Revelation(PendingRevelation {
            card,
            drawn_by: Investigator::Tony,
            started: true,
            cancelled: false,
        }));
        game.begin_test(
            TestOperation::RottingRemains { encounter },
            Investigator::Tony,
            Skill::Willpower,
            3,
        )
        .expect("Real Tony test");
        game.set_fixture_tokens(if case == TestCardFixture::TonyElderSign {
            vec![TokenFace::ElderSign]
        } else {
            vec![TokenFace::ElderSign, TokenFace::Zero, TokenFace::MinusOne]
        });
        game.history.push(format!("Privileged {case:?} witness: original Tony elder sign on an actual encounter test and his actual Bounty Contracts. No ordinary setup claim. [06003]"));
        return;
    }
    if matches!(
        case,
        TestCardFixture::LookFound
            | TestCardFixture::LookFoundTooFar
            | TestCardFixture::LookFoundAutoFail
    ) {
        let owner = &mut game.players[Investigator::Patrice.index()];
        owner.deck.append(&mut owner.hand);
        let at = owner
            .deck
            .iter()
            .position(|card| card.code == "01079")
            .expect("Selected Look What I Found");
        owner.hand.push(owner.deck.remove(at));
        let location = if case == TestCardFixture::LookFoundAutoFail {
            "01113"
        } else {
            "01114"
        };
        owner.location = location.into();
        let difficulty = i32::from(game.scenario.shroud(location).expect("Revealed location"));
        game.begin_test(
            TestOperation::Investigate {
                location: location.into(),
                clues: 1,
            },
            Investigator::Patrice,
            Skill::Intellect,
            difficulty,
        )
        .expect("Actual investigate");
        game.set_fixture_tokens(vec![match case {
            TestCardFixture::LookFoundTooFar => TokenFace::MinusOne,
            TestCardFixture::LookFoundAutoFail => TokenFace::AutoFail,
            _ => TokenFace::Zero,
        }]);
        game.history.push("Privileged Look What I Found witness: actual Patrice, event, location shroud and unsuccessful investigation. No ordinary setup claim. [01079]".into());
        return;
    }
    if matches!(
        case,
        TestCardFixture::ResourcefulSuccess
            | TestCardFixture::ResourcefulFailure
            | TestCardFixture::ResourcefulNoMatch
    ) {
        configure_resourceful(game, case);
        return;
    }
    if matches!(
        case,
        TestCardFixture::PromiseCurse
            | TestCardFixture::PromiseAtLimit
            | TestCardFixture::PromiseSealedLimit
            | TestCardFixture::PromiseCancelled
    ) {
        configure_promise(game, case);
        return;
    }
    let owner = &mut game.players[Investigator::Jacqueline.index()];
    owner.deck.append(&mut owner.hand);
    if case == TestCardFixture::CrystalPrediction {
        let at = owner
            .deck
            .iter()
            .position(|card| card.code == "60411")
            .expect("Selected Crystal Pendulum");
        let card = owner.deck.remove(at);
        owner.assets.push(AssetView {
            card,
            ready: true,
            uses: 0,
            damage: 0,
            horror: 0,
        });
        owner.resources -= 2;
        let at = owner
            .deck
            .iter()
            .position(|card| card.code == "02039")
            .expect("Selected Chronophobia");
        let weakness = owner.deck.remove(at);
        owner.deck.push(weakness);
    } else {
        let at = owner
            .deck
            .iter()
            .position(|card| card.code == "60419")
            .expect("Selected Prescient");
        owner.hand.push(owner.deck.remove(at));
        for code in ["60408", "60416", "60402", "04199"] {
            let at = owner
                .deck
                .iter()
                .position(|card| card.code == code)
                .expect("Selected recovery candidate");
            owner.discard.push(owner.deck.remove(at));
        }
        if case == TestCardFixture::PrescientIgnoredOnly {
            let at = owner
                .deck
                .iter()
                .position(|card| card.code == "04197")
                .expect("Selected Olive");
            let card = owner.deck.remove(at);
            owner.assets.push(AssetView {
                card,
                ready: true,
                uses: 0,
                damage: 0,
                horror: 0,
            });
            owner.resources -= 2;
        }
    }
    let at = game
        .encounter_deck
        .iter()
        .position(|card| card.code == "01163")
        .expect("Actual Rotting Remains");
    let card = game.encounter_deck.remove(at);
    let encounter = card.id;
    game.frames.push(Frame::Revelation(PendingRevelation {
        card,
        drawn_by: Investigator::Jacqueline,
        started: true,
        cancelled: false,
    }));
    game.begin_test(
        TestOperation::RottingRemains { encounter },
        Investigator::Jacqueline,
        Skill::Willpower,
        3,
    )
    .expect("One test");
    game.set_fixture_tokens(match case {
        TestCardFixture::CrystalPrediction => vec![TokenFace::Zero],
        TestCardFixture::PrescientResolved => vec![TokenFace::Skull],
        TestCardFixture::PrescientIgnoredOnly => {
            vec![TokenFace::Skull, TokenFace::Zero, TokenFace::MinusOne]
        }
        _ => unreachable!("Promise fixtures return above"),
    });
    game.history.push(format!("Privileged J7 {case:?} witness: actual selected card custody and a real Rotting Remains operation. Card inventory is preserved; this is not ordinary play."));
}

fn configure_promise(game: &mut Game, case: TestCardFixture) {
    if case == TestCardFixture::PromiseCancelled {
        game.players[Investigator::Jacqueline.index()].location =
            game.players[Investigator::Patrice.index()].location.clone();
    }
    let owner = &mut game.players[Investigator::Patrice.index()];
    owner.deck.append(&mut owner.hand);
    let at = owner
        .deck
        .iter()
        .position(|card| card.code == "07032")
        .expect("Selected Promise");
    owner.hand.push(owner.deck.remove(at));
    let limit = matches!(
        case,
        TestCardFixture::PromiseAtLimit | TestCardFixture::PromiseSealedLimit
    );
    if limit {
        game.bag.extend(std::iter::repeat_n(
            TokenFace::Curse,
            if case == TestCardFixture::PromiseSealedLimit {
                9
            } else {
                10
            },
        ));
    }
    if case == TestCardFixture::PromiseSealedLimit {
        let owner = &mut game.players[Investigator::Jacqueline.index()];
        let at = owner
            .deck
            .iter()
            .position(|card| card.code == "04199")
            .expect("Selected Premonition");
        game.premonition = Some(super::tokens::SealedPremonition {
            controller: Investigator::Jacqueline,
            card: owner.deck.remove(at),
            faces: vec![TokenFace::Curse],
        });
    }
    let at = game
        .encounter_deck
        .iter()
        .position(|card| card.code == "01163")
        .expect("Rotting Remains");
    let card = game.encounter_deck.remove(at);
    let encounter = card.id;
    game.frames.push(Frame::Revelation(PendingRevelation {
        card,
        drawn_by: Investigator::Patrice,
        started: true,
        cancelled: false,
    }));
    game.begin_test(
        TestOperation::RottingRemains { encounter },
        Investigator::Patrice,
        Skill::Willpower,
        3,
    )
    .expect("One test");
    game.set_fixture_tokens(match case {
        TestCardFixture::PromiseCurse => vec![TokenFace::Curse, TokenFace::Zero],
        TestCardFixture::PromiseCancelled => {
            vec![TokenFace::Curse, TokenFace::Zero, TokenFace::MinusOne]
        }
        _ => vec![TokenFace::Zero],
    });
    game.history.push(format!("Privileged {case:?} witness: actual Promise and Rotting Remains, with the stated finite curse supply. This is not ordinary play. [07032; Innsmouth p1]"));
}

fn configure_resourceful(game: &mut Game, case: TestCardFixture) {
    let owner = &mut game.players[Investigator::Patrice.index()];
    owner.deck.append(&mut owner.hand);
    let at = owner
        .deck
        .iter()
        .position(|card| card.code == "03039")
        .expect("Selected Resourceful");
    owner.hand.push(owner.deck.remove(at));
    for code in if case == TestCardFixture::ResourcefulNoMatch {
        vec!["03039", "01093"]
    } else {
        vec!["03039", "01093", "04200"]
    } {
        let at = owner
            .deck
            .iter()
            .position(|card| card.code == code)
            .expect("Selected discard candidate");
        owner.discard.push(owner.deck.remove(at));
    }
    owner.location = "01113".into();
    game.begin_test(
        TestOperation::Investigate {
            location: "01113".into(),
            clues: 1,
        },
        Investigator::Patrice,
        Skill::Intellect,
        1,
    )
    .expect("Actual Attic shroud");
    game.set_fixture_tokens(vec![if case == TestCardFixture::ResourcefulFailure {
        TokenFace::AutoFail
    } else {
        TokenFace::Zero
    }]);
    game.history.push(format!("Privileged {case:?} witness: Patrice investigates the revealed Attic using actual Resourceful and discard candidates. This is not ordinary play. [03039; 01113]"));
}
