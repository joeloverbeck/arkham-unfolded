//! A local experiment for GitHub #2, not a campaign engine.

pub mod asset_config;
pub mod assets;
pub mod continuity;
mod explanation;
#[cfg(feature = "tabletop")]
mod tabletop_camera;
#[cfg(feature = "tabletop")]
mod tabletop_harm_text;
#[cfg(feature = "tabletop")]
mod tabletop_layout;
#[cfg(feature = "tabletop")]
mod tabletop_pacing;
#[cfg(all(test, feature = "tabletop"))]
mod tabletop_standee_checks;
#[cfg(feature = "tabletop")]
mod tabletop_standees;
#[cfg(feature = "tabletop")]
mod tabletop_tokens;
pub mod tony;
#[cfg(feature = "tabletop")]
pub mod tony_tabletop;

#[cfg(feature = "tabletop")]
pub mod tabletop;

use serde::{Deserialize, Serialize};
use std::fmt;

pub const SOURCE_REVISION: &str = "ef905f5:chapter-one-jacqueline-curse-v1";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenFace {
    PlusOne,
    Zero,
    MinusOne,
    MinusTwo,
    MinusThree,
    MinusFour,
    Skull,
    Cultist,
    Tablet,
    AutoFail,
    ElderSign,
    Curse,
}

impl TokenFace {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlusOne => "+1",
            Self::Zero => "0",
            Self::MinusOne => "−1",
            Self::MinusTwo => "−2",
            Self::MinusThree => "−3",
            Self::MinusFour => "−4",
            Self::Skull => "Skull",
            Self::Cultist => "Cultist",
            Self::Tablet => "Tablet",
            Self::AutoFail => "Auto-fail",
            Self::ElderSign => "Elder sign",
            Self::Curse => "Curse",
        }
    }
    fn modifier(self) -> Option<i32> {
        match self {
            Self::PlusOne => Some(1),
            Self::Zero => Some(0),
            Self::MinusOne => Some(-1),
            Self::MinusTwo | Self::Curse => Some(-2),
            Self::MinusThree => Some(-3),
            Self::MinusFour => Some(-4),
            Self::AutoFail => Some(0),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Commit,
    BeforeReveal,
    Cancellation,
    Finished,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenStatus {
    Pending,
    Resolved,
    Canceled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleToken {
    pub id: u8,
    pub face: TokenFace,
    pub status: TokenStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    /// Only the already revealed faces this choice would cancel.
    pub cancel_faces: Vec<TokenFace>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigatorView {
    pub name: String,
    pub location: String,
    pub horror: u8,
    pub sanity: u8,
}

/// The complete player interface. It deliberately omits the scripted draw order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub revision: u64,
    pub stage: Stage,
    pub prompt: String,
    pub choices: Vec<Choice>,
    pub investigators: Vec<InvestigatorView>,
    pub tokens: Vec<VisibleToken>,
    pub skill_value: i32,
    pub difficulty: i32,
    pub horror_dealt: u8,
    pub bag_count: usize,
    pub curses_in_bag: usize,
    pub curses_in_pool: usize,
    pub promise_committed: bool,
    pub jacqueline_used: bool,
    pub history: Vec<String>,
}

/// Privileged setup for this experiment. Never accepted as a player decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixture {
    order: Vec<TokenFace>,
    patrice_here: bool,
    jacqueline_used: bool,
}

impl Fixture {
    pub fn curse() -> Self {
        Self {
            order: vec![
                TokenFace::Curse,
                TokenFace::MinusOne,
                TokenFace::Zero,
                TokenFace::MinusTwo,
            ],
            patrice_here: true,
            jacqueline_used: false,
        }
    }
    pub fn auto_fail() -> Self {
        Self {
            order: vec![
                TokenFace::AutoFail,
                TokenFace::Curse,
                TokenFace::Zero,
                TokenFace::MinusTwo,
            ],
            ..Self::curse()
        }
    }
    pub fn with_patrice_elsewhere(mut self) -> Self {
        self.patrice_here = false;
        self
    }
    pub fn with_jacqueline_used(mut self) -> Self {
        self.jacqueline_used = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub source_revision: String,
    pub app_version: String,
    pub moments: Vec<Observation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    revision: u64,
    stage: Stage,
    fixture: Fixture,
    bag: Vec<TokenFace>,
    tokens: Vec<VisibleToken>,
    group: Vec<u8>,
    order_cursor: usize,
    promise_committed: bool,
    jacqueline_used: bool,
    modifier: i32,
    auto_failed: bool,
    extra_draws: usize,
    horror_dealt: u8,
    curses_in_pool: usize,
    history: Vec<String>,
    moments: Vec<Observation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleError(pub String);
impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for RuleError {}

impl Session {
    pub fn new(fixture: Fixture) -> Self {
        use TokenFace::*;
        let mut session = Self {
            revision: 0, stage: Stage::Commit, jacqueline_used: fixture.jacqueline_used,
            fixture, bag: vec![PlusOne, Zero, Zero, MinusOne, MinusOne, MinusOne,
                MinusTwo, MinusTwo, MinusThree, MinusFour, Skull, Skull, Cultist,
                Tablet, AutoFail, ElderSign],
            tokens: vec![], group: vec![], order_cursor: 0, promise_committed: false,
            modifier: 0, auto_failed: false, extra_draws: 0, horror_dealt: 0,
            curses_in_pool: 10,
            history: vec!["Scripted experiment: Jacqueline faces Rotting Remains (willpower 3). Two Arcane Research copies account for her 2 starting horror. Only Patrice has a committable card: Promise of Power.".into()],
            moments: vec![],
        };
        session.moments.push(session.observe());
        session
    }

    pub fn observe(&self) -> Observation {
        let prompt = match self.stage {
            Stage::Commit => "Will Patrice commit Promise of Power?",
            Stage::BeforeReveal if self.jacqueline_used => {
                "Reveal the next token. Jacqueline has used her ability this round."
            }
            Stage::BeforeReveal => "Before revealing: will Jacqueline use her ability?",
            Stage::Cancellation => "Choose which revealed tokens to cancel.",
            Stage::Finished if self.horror_dealt > 0 => {
                "The test failed. Inspect the consequence or replay the record."
            }
            Stage::Finished => "The test succeeded. Inspect the consequence or replay the record.",
            Stage::Ended => "You ended the experiment. Its record is preserved.",
        };
        Observation {
            revision: self.revision,
            stage: self.stage,
            prompt: prompt.into(),
            choices: self.choices(),
            investigators: vec![
                InvestigatorView {
                    name: "Tony Morgan".into(),
                    location: "Study".into(),
                    horror: 0,
                    sanity: 5,
                },
                InvestigatorView {
                    name: "Patrice Hathaway".into(),
                    location: if self.fixture.patrice_here {
                        "Study"
                    } else {
                        "Hallway"
                    }
                    .into(),
                    horror: 0,
                    sanity: 7,
                },
                InvestigatorView {
                    name: "Jacqueline Fine".into(),
                    location: "Study".into(),
                    horror: 2 + self.horror_dealt,
                    sanity: 9,
                },
            ],
            tokens: self.tokens.clone(),
            skill_value: self.skill_value(),
            difficulty: 3,
            horror_dealt: self.horror_dealt,
            bag_count: self.bag.len(),
            curses_in_bag: self.bag.iter().filter(|&&f| f == TokenFace::Curse).count(),
            curses_in_pool: self.curses_in_pool,
            promise_committed: self.promise_committed,
            jacqueline_used: self.jacqueline_used,
            history: self.history.clone(),
        }
    }

    pub fn recording(&self) -> Recording {
        Recording {
            source_revision: SOURCE_REVISION.into(),
            app_version: APP_VERSION.into(),
            moments: self.moments.clone(),
        }
    }

    /// Refused or stale decisions leave the session unchanged, including its record.
    pub fn choose(&mut self, revision: u64, id: &str) -> Result<(), RuleError> {
        if revision != self.revision {
            return Err(RuleError(
                "This choice is stale. Observe the current decision.".into(),
            ));
        }
        let choice = self
            .choices()
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| RuleError("That choice is not legal at this decision.".into()))?;
        let mut candidate = self.clone();
        candidate.apply(&choice)?;
        candidate.revision += 1;
        candidate.moments.push(candidate.observe());
        *self = candidate;
        Ok(())
    }

    fn choices(&self) -> Vec<Choice> {
        let simple = |id: &str, label: &str| Choice {
            id: id.into(),
            label: label.into(),
            cancel_faces: vec![],
        };
        let mut choices = match self.stage {
            Stage::Commit => {
                let mut choices = vec![];
                if self.fixture.patrice_here {
                    choices.push(simple(
                        "commit-promise",
                        "Commit Promise of Power (+4; add a curse)",
                    ));
                }
                choices.push(simple("decline-promise", "Do not commit (willpower 5)"));
                choices
            }
            Stage::BeforeReveal => {
                let mut choices = vec![];
                if !self.jacqueline_used {
                    choices.push(simple(
                        "use-jacqueline",
                        "Use Jacqueline: reveal 3, then choose cancellations",
                    ));
                }
                choices.push(simple(
                    "reveal",
                    if self.jacqueline_used {
                        "Reveal next token"
                    } else {
                        "Decline for this reveal: draw 1 token"
                    },
                ));
                choices
            }
            Stage::Cancellation => self.cancellation_choices(),
            Stage::Finished | Stage::Ended => vec![],
        };
        if !matches!(self.stage, Stage::Finished | Stage::Ended) {
            choices.push(simple("end", "End this experiment"));
        }
        choices
    }

    fn cancellation_choices(&self) -> Vec<Choice> {
        let mut sets = vec![];
        if let Some(&auto) = self
            .group
            .iter()
            .find(|&&id| self.tokens[id as usize].face == TokenFace::AutoFail)
        {
            sets.push(vec![auto]);
            sets.push(
                self.group
                    .iter()
                    .copied()
                    .filter(|&id| id != auto)
                    .collect(),
            );
        } else {
            for a in 0..self.group.len() {
                for b in a + 1..self.group.len() {
                    sets.push(vec![self.group[a], self.group[b]]);
                }
            }
        }
        sets.into_iter()
            .map(|ids| {
                let faces: Vec<_> = ids
                    .iter()
                    .map(|&id| self.tokens[id as usize].face)
                    .collect();
                Choice {
                    id: format!(
                        "cancel-{}",
                        ids.iter().map(u8::to_string).collect::<Vec<_>>().join("-")
                    ),
                    label: format!(
                        "Cancel {}",
                        faces
                            .iter()
                            .map(|f| f.label())
                            .collect::<Vec<_>>()
                            .join(" and ")
                    ),
                    cancel_faces: faces,
                }
            })
            .collect()
    }

    fn apply(&mut self, choice: &Choice) -> Result<(), RuleError> {
        self.history.push(choice.label.clone());
        match choice.id.as_str() {
            "end" => {
                self.stage = Stage::Ended;
            }
            "commit-promise" => {
                self.promise_committed = true;
                self.bag.push(TokenFace::Curse);
                self.curses_in_pool -= 1;
                self.history.push("Promise of Power: four wild icons bring skill to 9. Add one curse before revealing. [07032; RR ST.2]".into());
                self.stage = Stage::BeforeReveal;
            }
            "decline-promise" => {
                self.stage = Stage::BeforeReveal;
            }
            "use-jacqueline" => {
                self.jacqueline_used = true;
                self.draw(3)?;
                self.stage = Stage::Cancellation;
            }
            "reveal" => {
                self.draw(1)?;
                self.resolve_group()?;
            }
            _ if self.stage == Stage::Cancellation => {
                let ids = choice.id.strip_prefix("cancel-").unwrap();
                for id in ids.split('-') {
                    let id: usize = id
                        .parse()
                        .map_err(|_| RuleError("Invalid cancellation.".into()))?;
                    self.tokens[id].status = TokenStatus::Canceled;
                }
                self.resolve_group()?;
            }
            _ => return Err(RuleError("Unsupported choice.".into())),
        }
        Ok(())
    }

    fn draw(&mut self, count: usize) -> Result<(), RuleError> {
        self.group.clear();
        for _ in 0..count {
            // A fixture priority order, not a claim of random sampling. If Promise
            // was declined, its nonexistent curse is skipped; other tokens retain
            // the fixed order. Every actual reveal removes a real bag token.
            let face = loop {
                let face = *self.fixture.order.get(self.order_cursor).ok_or_else(|| {
                    RuleError("This scripted fixture has no supported continuation.".into())
                })?;
                self.order_cursor += 1;
                if let Some(index) = self.bag.iter().position(|&f| f == face) {
                    self.bag.remove(index);
                    break face;
                }
            };
            let id = self.tokens.len() as u8;
            self.group.push(id);
            self.tokens.push(VisibleToken {
                id,
                face,
                status: TokenStatus::Pending,
            });
        }
        self.history.push(explanation::revealed_history(
            self.group.iter().map(|&id| self.tokens[id as usize].face),
        ));
        Ok(())
    }

    fn resolve_group(&mut self) -> Result<(), RuleError> {
        for &id in &self.group {
            let token = &mut self.tokens[id as usize];
            if token.status == TokenStatus::Canceled {
                continue;
            }
            token.status = TokenStatus::Resolved;
            self.modifier += token.face.modifier().ok_or_else(|| {
                RuleError("That symbol's effect is outside this experiment.".into())
            })?;
            if token.face == TokenFace::Curse {
                self.extra_draws += 1;
            }
            if token.face == TokenFace::AutoFail {
                self.auto_failed = true;
            }
        }
        if self.extra_draws > 0 {
            self.extra_draws -= 1;
            self.stage = Stage::BeforeReveal;
            self.history
                .push(explanation::resolved_curse_history().into());
            // No optional ability remains in this fixture once Jacqueline has
            // spent her use. Administer the mandatory continuation ourselves.
            if self.jacqueline_used {
                self.draw(1)?;
                self.resolve_group()?;
            }
        } else {
            self.horror_dealt = (3 - self.skill_value()).max(0) as u8;
            self.stage = Stage::Finished;
            for token in &self.tokens {
                if token.face == TokenFace::Curse && token.status == TokenStatus::Resolved {
                    self.curses_in_pool += 1;
                } else {
                    self.bag.push(token.face);
                }
            }
            self.history.push(format!("Skill {} versus 3: {} horror dealt. Resolved curses go to the pool; other revealed tokens return to the bag. [01163; RR ST.5–8; FAQ 2.5 §2.9]", self.skill_value(), self.horror_dealt));
        }
        Ok(())
    }

    fn skill_value(&self) -> i32 {
        if self.auto_failed {
            0
        } else {
            ((if self.promise_committed { 9 } else { 5 }) + self.modifier).max(0)
        }
    }
}
