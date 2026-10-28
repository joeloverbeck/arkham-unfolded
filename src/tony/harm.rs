//! Applied harm in the bounded Tony turn, independent of animation timing.

use super::{GhoulView, Observation, Stage, TestKind};
use crate::TokenFace;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarmSource {
    OpportunityAttack,
    Tablet,
    Cultist,
    BasicFight,
    InternalInjury,
}

impl HarmSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpportunityAttack => "Ghoul Minion — attack of opportunity",
            Self::Tablet => "Tablet token",
            Self::Cultist => "Cultist token",
            Self::BasicFight => "Tony's Fight",
            Self::InternalInjury => "Internal Injury",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarmRecipient {
    Tony,
    Ghoul,
}

impl HarmRecipient {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tony => "Tony Morgan",
            Self::Ghoul => "Ghoul Minion",
        }
    }
}

/// One source application. An attack's damage and horror remain together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Harm {
    pub source: HarmSource,
    pub recipient: HarmRecipient,
    pub damage: u8,
    pub horror: u8,
}

impl Harm {
    pub fn is_direct(self) -> bool {
        self.source == HarmSource::InternalInjury
    }
}

impl Observation {
    /// Predicts only the attack caused by a currently offered action.
    /// It does not evaluate hidden draws or change any state.
    pub fn opportunity_attack(&self, choice: &str) -> Option<Harm> {
        if self.stage != Stage::Action || !self.choices.iter().any(|c| c.id == choice) {
            return None;
        }
        opportunity_attack(&self.ghoul, choice)
    }
}

pub(super) fn opportunity_attack(ghoul: &GhoulView, choice: &str) -> Option<Harm> {
    (matches!(choice, "investigate" | "injury" | "resource" | "draw")
        && ghoul.alive
        && ghoul.ready
        && ghoul.engaged)
        .then_some(Harm {
            source: HarmSource::OpportunityAttack,
            recipient: HarmRecipient::Tony,
            damage: 1,
            horror: 1,
        })
}

/// Facts are optional only for the legacy prefix. Once captured, their known
/// source amounts, causal order and counter changes must remain consistent.
pub(super) fn valid_recording(moments: &[Observation]) -> bool {
    let mut captured = false;
    for (index, current) in moments.iter().enumerate() {
        let Some(events) = &current.harm else {
            if captured {
                return false;
            }
            continue;
        };
        captured = true;
        let Some(previous) = index.checked_sub(1).map(|i| &moments[i]) else {
            if !events.is_empty() {
                return false;
            }
            continue;
        };
        if !valid_events(previous, current, events) {
            return false;
        }
    }
    true
}

fn valid_events(before: &Observation, after: &Observation, events: &[Harm]) -> bool {
    let mut totals = [before.damage, before.horror, before.ghoul.damage].map(u16::from);
    let end = [after.damage, after.horror, after.ghoul.damage].map(u16::from);
    // Undo restores a prior state without representing healing or another hit.
    if events.is_empty() && end.iter().zip(totals).all(|(end, start)| *end <= start) {
        return true;
    }
    let mut last_order = None;
    for event in events {
        let (recipient, damage, horror, order, applicable) = match event.source {
            HarmSource::OpportunityAttack => (
                HarmRecipient::Tony,
                1,
                1,
                0,
                before.stage == Stage::Action
                    && before.ghoul.alive
                    && before.ghoul.ready
                    && before.ghoul.engaged,
            ),
            HarmSource::Tablet => (
                HarmRecipient::Tony,
                1,
                0,
                0,
                before.stage == Stage::Resolve
                    && before.held_token == Some(TokenFace::Tablet)
                    && before.ghoul.alive,
            ),
            HarmSource::Cultist => (
                HarmRecipient::Tony,
                0,
                1,
                1,
                before.stage == Stage::Resolve
                    && before.held_token == Some(TokenFace::Cultist)
                    && after
                        .test
                        .as_ref()
                        .is_some_and(|test| test.success == Some(false)),
            ),
            HarmSource::BasicFight => (
                HarmRecipient::Ghoul,
                1,
                0,
                1,
                before.stage == Stage::Resolve
                    && before
                        .test
                        .as_ref()
                        .is_some_and(|test| test.kind == TestKind::Fight)
                    && after
                        .test
                        .as_ref()
                        .is_some_and(|test| test.success == Some(true)),
            ),
            HarmSource::InternalInjury => (
                HarmRecipient::Tony,
                1,
                0,
                2,
                before.injury
                    && after.injury
                    && before.stage != Stage::Ended
                    && after.stage == Stage::Ended,
            ),
        };
        if !applicable
            || (event.recipient, event.damage, event.horror) != (recipient, damage, horror)
            || last_order.is_some_and(|last| order <= last)
        {
            return false;
        }
        last_order = Some(order);
        // The bounded source cases above allow at most three one-point events.
        match event.recipient {
            HarmRecipient::Tony => {
                totals[0] += u16::from(event.damage);
                totals[1] += u16::from(event.horror);
            }
            HarmRecipient::Ghoul => totals[2] += u16::from(event.damage),
        }
    }
    totals == end
}
