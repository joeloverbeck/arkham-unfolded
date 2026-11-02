//! Previously revealed story faces, derived only from the displayed public prefix.

use crate::gathering::{Observation, PlayObservation, Recording, Resolution};
use crate::tabletop_layout::ScenarioBookSide;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum StoryCard {
    AgendaOne,
    AgendaTwo,
    AgendaThree,
    ActOne,
    ActTwo,
    ActThree,
}

impl StoryCard {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::AgendaOne => "01105",
            Self::AgendaTwo => "01106",
            Self::AgendaThree => "01107",
            Self::ActOne => "01108",
            Self::ActTwo => "01109",
            Self::ActThree => "01110",
        }
    }

    pub(super) fn from_code(code: &str) -> Option<Self> {
        match code {
            "01105" => Some(Self::AgendaOne),
            "01106" => Some(Self::AgendaTwo),
            "01107" => Some(Self::AgendaThree),
            "01108" => Some(Self::ActOne),
            "01109" => Some(Self::ActTwo),
            "01110" => Some(Self::ActThree),
            _ => None,
        }
    }

    pub(super) fn side(self) -> ScenarioBookSide {
        match self {
            Self::AgendaOne | Self::AgendaTwo | Self::AgendaThree => ScenarioBookSide::Agenda,
            Self::ActOne | Self::ActTwo | Self::ActThree => ScenarioBookSide::Act,
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::AgendaOne => "Agenda 1 reverse",
            Self::AgendaTwo => "Agenda 2 reverse",
            Self::AgendaThree => "Agenda 3 reverse",
            Self::ActOne => "Act 1 reverse",
            Self::ActTwo => "Act 2 reverse",
            Self::ActThree => "Act 3 reverse",
        }
    }
}

#[derive(Default)]
pub(super) struct Faces {
    known: BTreeSet<StoryCard>,
}

impl Faces {
    pub(super) fn for_view(
        recording: &Recording,
        view: &Observation,
        presentation_sequence: Option<u32>,
    ) -> Self {
        let mut result = Self::default();
        for decision in recording.moments.iter().filter(|decision| {
            decision.revision < view.revision
                || (decision.revision == view.revision && presentation_sequence.is_none())
        }) {
            result.observe_decision(decision);
        }
        let mut previous: Option<(u64, &PlayObservation)> = None;
        for moment in recording.presentation_moments.iter().filter(|moment| {
            moment.revision < view.revision
                || (moment.revision == view.revision
                    && presentation_sequence.is_none_or(|sequence| moment.sequence <= sequence))
        }) {
            result.observe_play(&moment.play);
            // Only advancement clears agenda doom between consecutive public
            // snapshots inside one decision. A later revision may instead undo
            // a choice; it must never manufacture knowledge of a reverse.
            if let Some((revision, before)) = previous
                && revision == moment.revision
                && before.agenda == moment.play.agenda
                && before.agenda_doom > 0
                && moment.play.agenda_doom == 0
            {
                result.known.extend(match moment.play.agenda {
                    1 => Some(StoryCard::AgendaOne),
                    2 => Some(StoryCard::AgendaTwo),
                    3 => Some(StoryCard::AgendaThree),
                    _ => None,
                });
            }
            previous = Some((moment.revision, &moment.play));
        }
        if presentation_sequence.is_none() {
            result.observe_decision(view);
        } else if let Some(play) = &view.play {
            result.observe_play(play);
        }
        result
    }

    fn observe_decision(&mut self, view: &Observation) {
        if let Some(play) = &view.play {
            self.observe_play(play);
            for choice in &view.choices {
                self.known
                    .extend(match (choice.id.as_str(), play.agenda, play.act) {
                        ("agenda:lead-horror" | "agenda:random-discard", 1, _) => {
                            Some(StoryCard::AgendaOne)
                        }
                        ("agenda:search", 2, _) => Some(StoryCard::AgendaTwo),
                        ("resolution:burn" | "resolution:refuse", _, 3) => {
                            Some(StoryCard::ActThree)
                        }
                        _ => None,
                    });
            }
        }
    }

    fn observe_play(&mut self, play: &PlayObservation) {
        match play.agenda {
            2 => {
                self.known.insert(StoryCard::AgendaOne);
            }
            3 => {
                self.known
                    .extend([StoryCard::AgendaOne, StoryCard::AgendaTwo]);
            }
            _ => {}
        }
        match play.act {
            2 => {
                self.known.insert(StoryCard::ActOne);
            }
            3 => {
                self.known.extend([StoryCard::ActOne, StoryCard::ActTwo]);
            }
            _ => {}
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
            self.known.insert(StoryCard::ActThree);
        }
    }

    pub(super) fn known_reverses(
        &self,
        side: ScenarioBookSide,
    ) -> impl Iterator<Item = StoryCard> + '_ {
        self.known.iter().copied().filter(move |card| {
            matches!(
                (card.side(), side),
                (ScenarioBookSide::Agenda, ScenarioBookSide::Agenda)
                    | (ScenarioBookSide::Act, ScenarioBookSide::Act)
            )
        })
    }

    pub(super) fn is_reverse(&self, code: &str) -> bool {
        StoryCard::from_code(code).is_some_and(|card| self.known.contains(&card))
    }
}

#[cfg(test)]
mod tests;
