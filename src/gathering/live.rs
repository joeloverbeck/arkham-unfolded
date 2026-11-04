//! Facts needed by the current live presentation, independent of past views.
use super::{Observation, PlayObservation, PresentationMoment, Resolution};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LiveState {
    pub known_story: BTreeSet<String>,
    pub new_story: BTreeSet<String>,
    pub completed_before: usize,
}
impl LiveState {
    pub(super) fn capture(
        &mut self,
        before: &Observation,
        steps: &mut [PresentationMoment],
        after: &Observation,
    ) {
        self.completed_before = before
            .play
            .as_ref()
            .map_or(0, |play| play.completed_tests.len());
        let known = self.known_story.clone();
        let mut previous = None;
        for step in steps {
            self.observe_play(&step.play);
            if let Some((agenda, doom)) = previous
                && agenda == step.play.agenda
                && doom > 0
                && step.play.agenda_doom == 0
            {
                self.known_story
                    .insert(format!("{:05}", 1104 + u16::from(agenda)));
            }
            previous = Some((step.play.agenda, step.play.agenda_doom));
            step.revealed_story = self.known_story.iter().cloned().collect();
        }
        if let Some(play) = &after.play {
            self.observe_play(play);
            for choice in &after.choices {
                let code = match (choice.id.as_str(), play.agenda, play.act) {
                    ("agenda:lead-horror" | "agenda:random-discard", 1, _) => Some("01105"),
                    ("agenda:search", 2, _) => Some("01106"),
                    ("resolution:burn" | "resolution:refuse", _, 3) => Some("01110"),
                    _ => None,
                };
                if let Some(code) = code {
                    self.known_story.insert(code.into());
                }
            }
        }
        self.new_story = self.known_story.difference(&known).cloned().collect();
    }
    fn observe_play(&mut self, play: &PlayObservation) {
        if play.agenda >= 2 {
            self.known_story.insert("01105".into());
        }
        if play.agenda >= 3 {
            self.known_story.insert("01106".into());
        }
        if play.act >= 2 {
            self.known_story.insert("01108".into());
        }
        if play.act >= 3 {
            self.known_story.insert("01109".into());
        }
        if play.act == 3
            && (play
                .scenario
                .victory
                .iter()
                .any(|card| card.code == "01116")
                || play.scenario.outcome.as_ref().is_some_and(|outcome| {
                    matches!(
                        outcome.resolution,
                        Resolution::BurnHouse | Resolution::RefuseToBurn
                    )
                }))
        {
            self.known_story.insert("01110".into());
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Transition {
    pub before: Option<Observation>,
    pub steps: Vec<PresentationMoment>,
}
// Transient animation input is deliberately outside persisted rules identity.
impl PartialEq for Transition {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for Transition {}
