//! Public, ordered encounter-discard views for the two prepared experiments.
//!
//! These one-card collections are derived from recorded defeat/completion facts.
//! They are not additional persisted state or a general card-zone model.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct EncounterCard {
    /// The card instance in its prepared experiment, not a catalog identifier.
    pub id: &'static str,
    pub code: &'static str,
    pub title: &'static str,
}

impl crate::tony::Observation {
    /// Bottom-to-top order; the prepared encounter discard begins empty.
    pub fn encounter_discard(&self) -> &'static [EncounterCard] {
        if self.ghoul.alive {
            &[]
        } else {
            &[EncounterCard {
                id: "prepared-ghoul-minion",
                code: "01160",
                title: "Ghoul Minion",
            }]
        }
    }
}

impl crate::Observation {
    /// Bottom-to-top order; an abandoned revelation has not finished resolving.
    pub fn encounter_discard(&self) -> &'static [EncounterCard] {
        if self.stage == crate::Stage::Finished {
            &[EncounterCard {
                id: "prepared-rotting-remains",
                code: "01163",
                title: "Rotting Remains",
            }]
        } else {
            &[]
        }
    }
}
