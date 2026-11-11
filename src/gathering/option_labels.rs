//! Option label composition. Each option states its effective cost once, as
//! currently modified, followed by its effect; the effect never restates the
//! cost. Cost wording comes from the effective cost after modifiers such as
//! Frozen in Fear, never from a phrase written into a card's effect text.

use super::scenario::{FOG_SHROUD, ScenarioState};

/// A special action that pays an option's base action instead of an ordinary
/// action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Allowance {
    /// Tony Morgan's additional action, restricted to bounty engages and fights.
    TonyBounty,
    /// An immediate additional action, such as Quick Thinking's.
    Granted,
}

impl Allowance {
    fn phrase(self) -> &'static str {
        match self {
            Self::TonyBounty => "Tony's additional bounty action",
            Self::Granted => "the granted action",
        }
    }
}

/// What choosing an option spends, after every modifier currently in force.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EffectiveCost {
    allowance: Option<Allowance>,
    actions: u8,
}

impl EffectiveCost {
    /// Ordinary actions only. Zero is a free option and states no cost.
    pub(super) fn actions(actions: u8) -> Self {
        Self {
            allowance: None,
            actions,
        }
    }

    /// `total` actions, the first paid by `allowance` and the rest ordinary.
    pub(super) fn paid_by(allowance: Allowance, total: u8) -> Self {
        Self {
            allowance: Some(allowance),
            actions: total.saturating_sub(1),
        }
    }

    fn phrase(self) -> Option<String> {
        match (self.allowance, self.actions) {
            (None, 0) => None,
            (None, actions) => Some(format!("Spend {}", action_count(actions))),
            (Some(allowance), 0) => Some(format!("Spend {}", allowance.phrase())),
            (Some(allowance), actions) => Some(format!(
                "Spend {} and {}",
                allowance.phrase(),
                action_count(actions)
            )),
        }
    }
}

/// "Spend two actions: Discard The 13th Vision". A free option is its effect.
pub(super) fn label(cost: EffectiveCost, effect: &str) -> String {
    match cost.phrase() {
        Some(cost) => format!("{cost}: {effect}"),
        None => effect.to_owned(),
    }
}

fn action_count(actions: u8) -> String {
    match actions {
        1 => "an action".into(),
        2 => "two actions".into(),
        3 => "three actions".into(),
        actions => format!("{actions} actions"),
    }
}

/// What an investigation of `location` faces, for an option's details: its
/// shroud and any attachment changing it, and, where there is no clue to
/// discover, what a success still does. Investigating there stays legal and
/// can still trigger abilities (FAQ v2.5 p.15). `success_also` names what the
/// option's own source does on a success, such as Old Keyring using a key. A
/// location with clues and no attachment needs no details.
pub(super) fn investigation_details(
    scenario: &ScenarioState,
    location: &str,
    success_also: &[&str],
) -> Vec<String> {
    let (Some(state), Some(shroud)) = (scenario.location(location), scenario.shroud(location))
    else {
        return vec![];
    };
    if state.clues > 0 && state.fog.is_none() {
        return vec![];
    }
    let mut details = vec![match &state.fog {
        Some(fog) => format!("shroud {shroud} including {}'s +{FOG_SHROUD}", fog.title),
        None => format!("shroud {shroud}"),
    }];
    if state.clues == 0 {
        details.push("no clues to discover".into());
        let mut success: Vec<String> = success_also.iter().map(|&also| also.into()).collect();
        if let Some(fog) = &state.fog {
            success.push(format!("discards {}", fog.title));
        }
        if !success.is_empty() {
            details.push(format!("a success still {}", success.join(" and ")));
        }
    }
    details
}

/// " (shroud 1 · no clues to discover)", or nothing without details.
pub(super) fn parenthesized(details: &[String]) -> String {
    if details.is_empty() {
        String::new()
    } else {
        format!(" ({})", details.join(" · "))
    }
}

/// A count that agrees in number with its noun: "1 resource", "2 resources".
pub(super) fn counted(count: impl Into<usize>, singular: &str, plural: &str) -> String {
    let count = count.into();
    format!("{count} {}", if count == 1 { singular } else { plural })
}
