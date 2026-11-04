//! Previously revealed story faces, retained as explicit current knowledge.

use crate::gathering::Session;
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

#[derive(Clone, Default)]
pub(super) struct Faces {
    known: BTreeSet<StoryCard>,
}

/// Reading acknowledges a displayed public reverse; it never advances rules.
/// Reopening a record at the revealing decision deliberately offers that last
/// transition again, without reapplying any of its already-saved effects.
#[derive(Default)]
pub(super) struct Reading {
    known: Faces,
    pending: std::collections::VecDeque<StoryCard>,
}

impl Reading {
    pub(super) fn for_session(session: &Session) -> Self {
        let state = session.live_state();
        Self {
            known: Faces::from_codes(
                state
                    .known_story
                    .difference(&state.new_story)
                    .map(String::as_str),
            ),
            pending: Default::default(),
        }
    }

    pub(super) fn at(known: Faces) -> Self {
        Self {
            known,
            pending: Default::default(),
        }
    }

    pub(super) fn synchronize(&mut self, faces: &Faces) {
        self.pending.retain(|card| faces.known.contains(card));
        self.pending
            .extend(faces.known.difference(&self.known.known).copied());
        self.known = faces.clone();
    }

    pub(super) fn current(&self) -> Option<StoryCard> {
        self.pending.front().copied()
    }
    pub(super) fn advance(&mut self) {
        self.pending.pop_front();
    }
}

impl Faces {
    pub(super) fn from_codes<'a>(codes: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            known: codes.into_iter().filter_map(StoryCard::from_code).collect(),
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
