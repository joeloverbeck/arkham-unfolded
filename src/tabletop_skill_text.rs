//! A shared presentation of already-observed skill-test arithmetic.
//!
//! Each consumer supplies its represented facts. This formatter does not resolve
//! tests, inspect future tokens, or change the English observations in records.

use crate::tabletop::symbols::Skill;

pub(crate) struct TestSummary {
    pub skill: Skill,
    pub base: i32,
    pub card_bonus: i32,
    pub chaos: Option<i32>,
    pub chaos_pending: bool,
    pub automatic_failure: bool,
    pub total: Option<i32>,
    pub difficulty: i32,
    pub result: Option<bool>,
}

impl TestSummary {
    pub(crate) fn label(self) -> String {
        self.label_with_pending("Pending")
    }

    pub(crate) fn label_with_pending(self, pending: &str) -> String {
        let chaos = if self.automatic_failure {
            "auto-fail".into()
        } else {
            self.chaos
                .map_or_else(|| "pending".into(), |value| format!("{value:+}"))
        };
        let more = if self.chaos_pending && self.chaos.is_some() {
            " (more pending)"
        } else {
            ""
        };
        let value_label = if self.result.is_some() {
            "final"
        } else {
            "current"
        };
        let total = self
            .total
            .map_or_else(|| "unavailable".into(), |value| value.to_string());
        let outcome = self.result.map_or(
            pending,
            |success| if success { "Success" } else { "Failure" },
        );
        format!(
            "base {} {} · cards {:+} · chaos {chaos}{more} · {value_label} value {total} · difficulty {} · {outcome}",
            self.skill.title(),
            self.base,
            self.card_bonus,
            self.difficulty,
        )
    }
}
