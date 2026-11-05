//! Transient public steps within one accepted decision. They never execute rules.
use super::{CardView, Investigator, Observation, PlayObservation, Stage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationMoment {
    pub revision: u64,
    pub sequence: u32,
    pub caption: String,
    pub play: PlayObservation,
    pub revealed_story: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StepSnapshot {
    pub caption: String,
    pub play: PlayObservation,
    pub draw: Option<DrawBatch>,
}

/// Transient rendering information from an actual draw operation. This never
/// enters saved rules state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawBatch {
    pub source: DrawSource,
    pub cards: Vec<CardView>,
    pub returned: Vec<CardView>,
    pub steps: Vec<DeckStep>,
}

/// Actual deck-operation order. Counts are public; indices address only the
/// faces already disclosed in this batch. Hidden draws have no card identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeckStep {
    Draw { card: usize, remaining: usize },
    RevealHeld { card: usize },
    TakeHidden { remaining: usize, held: usize },
    Return,
    Recycle { count: usize },
    Shuffle { count: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawSource {
    Investigator(Investigator),
    Encounter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawCue {
    pub revision: u64,
    /// None denotes the committed setup observation; Some denotes an existing
    /// intermediate play snapshot. The recorded snapshot itself is unchanged.
    pub sequence: Option<u32>,
    pub batch: DrawBatch,
}

pub(super) fn project(
    before: &Observation,
    current: &Observation,
    steps: &[PresentationMoment],
    moment: &PresentationMoment,
) -> Observation {
    let mut view = before.clone();
    view.revision = moment.revision;
    view.stage = Stage::Playing;
    view.prompt = moment.caption.clone();
    view.choices.clear();
    view.decision_owner = None;
    view.lead = Some(moment.play.lead);
    view.history.extend(
        steps
            .iter()
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
        view.scenario = current.scenario.clone();
    }
    view.play = Some(moment.play.clone());
    view
}
