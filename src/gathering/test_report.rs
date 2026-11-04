//! Arithmetic captured at its rules boundary for inspection and recorded play.
use super::{
    Investigator,
    runtime::{Game, Skill, SkillTest, TestOperation},
};
use crate::TokenFace;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    pub label: String,
    pub owner: Investigator,
    pub amount: i32,
}

impl Contribution {
    pub(super) fn new(label: impl Into<String>, owner: Investigator, amount: i32) -> Self {
        Self {
            label: label.into(),
            owner,
            amount,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenContribution {
    pub face: TokenFace,
    pub amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestBreakdown {
    pub skill: Skill,
    pub base: i32,
    pub cards: Vec<Contribution>,
    pub bonuses: Vec<Contribution>,
    pub tokens: Vec<TokenContribution>,
    pub token_total: i32,
    pub ignored: Vec<TokenFace>,
    pub canceled: Vec<TokenFace>,
    pub outcome_reason: Option<String>,
}

pub(super) fn static_bonuses(game: &Game, test: &SkillTest) -> Vec<Contribution> {
    let player = &game.players[test.tester.index()];
    let mut bonuses = Vec::new();
    for asset in &player.assets {
        if matches!(
            (asset.card.code.as_str(), test.skill),
            ("60411", Skill::Willpower) | ("02033", Skill::Agility)
        ) {
            bonuses.push(Contribution::new(&asset.card.title, test.tester, 1));
        }
    }
    if test.skill == Skill::Combat
        && let Some(owner) = super::scenario::lita_controller(game)
        && game.players[owner.index()].location == player.location
    {
        bonuses.push(Contribution::new("Lita Chantler", owner, 1));
    }
    if let Some(card) = &test.source_card
        && matches!(card.code.as_str(), "06117" | "06201" | "06164")
    {
        bonuses.push(Contribution::new(
            format!("{}: add base willpower", card.title),
            test.tester,
            super::test_cards::base_skill(game, test.tester, Skill::Willpower),
        ));
        for asset in &player.assets {
            if asset.card.code == "60411" {
                bonuses.push(Contribution::new(
                    format!("{}: added willpower", asset.card.title),
                    test.tester,
                    1,
                ));
            }
        }
    }
    bonuses
}

impl SkillTest {
    pub(super) fn record_bonus(
        &mut self,
        label: impl Into<String>,
        owner: Investigator,
        amount: i32,
    ) {
        if self.capture_breakdown {
            self.bonus_sources
                .push(Contribution::new(label, owner, amount));
        }
    }
}

pub(super) fn breakdown(game: &Game, test: &SkillTest) -> TestBreakdown {
    if let Some(report) = &test.final_breakdown {
        return report.as_ref().clone();
    }
    let mut bonuses = static_bonuses(game, test);
    bonuses.extend(test.bonus_sources.clone());
    // Historical pending tests can have an aggregate without source detail.
    // Preserve the known amount without inventing the missing provenance.
    let unrecorded = test.bonus - test.bonus_sources.iter().map(|row| row.amount).sum::<i32>();
    if unrecorded != 0 {
        bonuses.push(Contribution::new(
            "Earlier bonus (source not recorded)",
            test.tester,
            unrecorded,
        ));
    }
    if let TestOperation::Fight {
        skill_bonus,
        weapon,
        ..
    } = &test.operation
        && *skill_bonus != 0
    {
        let label = test
            .source_card
            .as_ref()
            .map(|card| card.title.clone())
            .or_else(|| {
                game.players[test.tester.index()]
                    .assets
                    .iter()
                    .find(|asset| Some(asset.card.id) == *weapon)
                    .map(|asset| asset.card.title.clone())
            })
            .unwrap_or_else(|| "Attack ability".into());
        bonuses.push(Contribution::new(label, test.tester, *skill_bonus));
    }
    let player = &game.players[test.tester.index()];
    let outcome_reason = if test.revealed.contains(&TokenFace::AutoFail) {
        Some("Automatic failure: the final skill value is 0.".into())
    } else if game.players.iter().any(|owner| {
        owner.location == player.location && owner.threats.iter().any(|card| card.code == "05041")
    }) {
        Some("The 13th Vision: matching the difficulty fails; exceed it to succeed.".into())
    } else {
        None
    };
    TestBreakdown {
        skill: test.skill,
        base: super::test_cards::base_skill(game, test.tester, test.skill),
        cards: test
            .committed
            .iter()
            .map(|entry| {
                Contribution::new(
                    &entry.card.title,
                    entry.owner,
                    game.commit_icons(test, &entry.card),
                )
            })
            .collect(),
        bonuses,
        tokens: test.token_contributions.clone(),
        token_total: test.modifier,
        ignored: test.ignored_tokens.clone(),
        canceled: test.cancelled_tokens.clone(),
        outcome_reason,
    }
}

fn match_test(expected: &mut SkillTest, recorded: &SkillTest) {
    if !recorded.capture_breakdown {
        expected.capture_breakdown = false;
        expected.bonus_sources.clear();
        expected.token_contributions.clear();
        expected.final_breakdown = None;
    }
}

fn match_frames(expected: &mut [super::runtime::Frame], recorded: &[super::runtime::Frame]) {
    use super::runtime::Frame;
    for (expected, recorded) in expected.iter_mut().zip(recorded) {
        match (expected, recorded) {
            (Frame::TestCleanup(expected), Frame::TestCleanup(recorded)) => {
                match_test(expected, recorded)
            }
            (Frame::AfterTest(expected), Frame::AfterTest(recorded)) => {
                match_test(&mut expected.test, &recorded.test)
            }
            _ => {}
        }
    }
}

/// Additive explanation fields are absent in historical records. Only their
/// presence is normalized; recorded values, rules state and arithmetic still
/// have to match deterministic reconstruction.
pub(super) fn match_runtime(expected: &mut Game, recorded: &Game) {
    if let (Some(expected), Some(recorded)) = (&mut expected.test, &recorded.test) {
        match_test(expected, recorded);
    }
    match_frames(&mut expected.frames, &recorded.frames);
    for (expected, recorded) in expected
        .completed_tests
        .iter_mut()
        .zip(&recorded.completed_tests)
    {
        if recorded.breakdown.is_none() {
            expected.breakdown = None;
        }
    }
    if let (Some(expected), Some(recorded)) = (&mut expected.stopped, &recorded.stopped) {
        if let (Some(expected), Some(recorded)) = (&mut expected.test, &recorded.test) {
            match_test(expected, recorded);
        }
        match_frames(&mut expected.frames, &recorded.frames);
    }
}
