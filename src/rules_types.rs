//! Shared choice and chaos-token values used by The Gathering.
use serde::{Deserialize, Serialize};
use std::fmt;
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
    pub(crate) fn modifier(self) -> Option<i32> {
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
pub enum TokenStatus {
    Pending,
    Resolved,
    Canceled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    /// Only the already revealed faces this choice would cancel.
    pub cancel_faces: Vec<TokenFace>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleError(pub String);
impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for RuleError {}
