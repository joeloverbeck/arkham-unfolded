//! Captured public states between accepted decisions. They never execute rules.
use super::{Observation, PlayObservation, Stage};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationMoment {
    pub revision: u64,
    pub sequence: u32,
    pub caption: String,
    pub play: PlayObservation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StepSnapshot {
    pub caption: String,
    pub play: PlayObservation,
}

pub(super) fn project(
    decisions: &[Observation],
    tape: &[PresentationMoment],
    moment: &PresentationMoment,
) -> Observation {
    let mut view = decisions
        .iter()
        .rev()
        .find(|view| view.revision < moment.revision)
        .unwrap_or_else(|| decisions.first().expect("A recording has its origin"))
        .clone();
    view.revision = moment.revision;
    view.stage = Stage::Playing;
    view.prompt = moment.caption.clone();
    view.choices.clear();
    view.decision_owner = None;
    view.lead = Some(moment.play.lead);
    view.history.extend(
        tape.iter()
            .filter(|step| step.revision == moment.revision && step.sequence <= moment.sequence)
            .map(|step| step.caption.clone()),
    );
    for player in &moment.play.players {
        let investigator = &mut view.investigators[player.investigator.index()];
        investigator.resources = player.resources;
        investigator.damage = player.damage;
        investigator.horror = player.horror;
        investigator.physical_trauma = player.physical_trauma;
        investigator.mental_trauma = player.mental_trauma;
        investigator.hand = player.hand.clone();
        investigator.deck_count = player.deck_count;
        investigator.location = Some(player.location.clone());
        investigator.selected_mulligan.clear();
        investigator.mulligan_complete = true;
    }
    // Preparation facts are public and fixed once the runtime exists. The
    // previous decision can be the last mulligan, before this summary existed.
    if view.scenario.is_none() {
        view.scenario = decisions
            .iter()
            .find_map(|decision| decision.scenario.clone());
    }
    view.play = Some(moment.play.clone());
    view
}
