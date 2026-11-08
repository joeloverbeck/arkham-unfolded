//! Gathering's transient live presentation. Only committed sessions enter here;
//! clocks and reading acknowledgments never execute or save rules decisions.
use super::*;
use crate::gathering::{DrawCue, DrawSource, TestResult};

pub(super) struct Presentation {
    moments: Vec<PresentationMoment>,
    index: usize,
    clock: f32,
    draw_cues: Vec<DrawCue>,
    draws: draws::Playback,
    map: map_motion::Playback,
    story: story::Reading,
    results: results::Reading,
    encounter: encounter::Reading,
    announcement: Option<phase_banner::Announcement>,
}

/// One coherent scene input, including the transfers acknowledged by this render.
pub(super) struct Frame {
    pub view: Observation,
    pub story: story::Faces,
    pub transfers: Vec<draws::Transfer>,
    pub map_arrivals: Vec<(u16, Vec3)>,
}

#[derive(PartialEq, Eq)]
pub(super) enum Progress {
    Waiting,
    Draw,
    Map,
    Moment,
}

impl Presentation {
    pub(super) fn new(session: &Session) -> Self {
        Self {
            moments: vec![],
            index: 0,
            clock: 0.,
            draw_cues: vec![],
            draws: draws::Playback::default(),
            map: map_motion::Playback::default(),
            story: story::Reading::for_session(session),
            results: results::Reading::for_session(session),
            encounter: encounter::Reading::for_session(session),
            announcement: None,
        }
    }

    pub(super) fn view(&self, session: &Session) -> Observation {
        self.draws
            .view()
            .or_else(|| self.map.view())
            .cloned()
            .unwrap_or_else(|| self.base_view(session))
    }

    fn base_view(&self, session: &Session) -> Observation {
        self.moments.get(self.index).map_or_else(
            || session.observe(),
            |moment| session.observe_presentation(moment),
        )
    }

    pub(super) fn active(&self) -> bool {
        self.announcement.is_some()
            || self.drawing()
            || self.map.active()
            || self.index < self.moments.len()
    }

    pub(super) fn announcement(&self) -> Option<&phase_banner::Announcement> {
        self.announcement
            .as_ref()
            .filter(|announcement| !announcement.after_setup_draws || !self.drawing())
    }

    pub(super) fn drawing(&self) -> bool {
        self.draws.active()
    }

    pub(super) fn result(&self) -> Option<&TestResult> {
        self.results.current()
    }

    pub(super) fn story(&self) -> Option<story::StoryCard> {
        self.story.current()
    }

    pub(super) fn encounter(&self) -> &encounter::Reading {
        &self.encounter
    }

    pub(super) fn encounter_mut(&mut self) -> &mut encounter::Reading {
        &mut self.encounter
    }

    pub(super) fn encounter_input(&mut self, input: encounter::Input) {
        self.encounter.input(input);
    }

    pub(super) fn acknowledge_result(&mut self) {
        self.results.advance();
    }

    pub(super) fn acknowledge_story(&mut self) {
        self.story.advance();
    }

    pub(super) fn reduce_motion(&mut self) {
        self.draws.clear();
        self.map.clear();
    }

    pub(super) fn map(&self) -> &map_motion::Playback {
        &self.map
    }

    /// The caller has already saved and installed the successful choice.
    pub(super) fn committed(
        &mut self,
        session: &Session,
        before: &Observation,
        choice: &str,
        draws: Vec<DrawCue>,
        reduced_motion: bool,
    ) {
        if choice == "undo" {
            self.results = results::Reading::at(Some(&session.observe()));
        }
        self.moments = session.presentation_after(before.revision).to_vec();
        self.index = 0;
        self.clock = 0.;
        self.encounter.committed(session, &draws);
        self.draw_cues = draws;
        self.start_transitions(session, before, reduced_motion);
        if choice == "undo" {
            self.announcement = None;
        }
    }

    /// In-place reopen differs from fresh open: retain the exact ongoing reading
    /// and draw state only when the whole loaded Session is unchanged.
    pub(super) fn reopen(&mut self, previous: &Session, loaded: &Session) {
        if loaded != previous {
            *self = Self::new(loaded);
        }
    }

    fn start_transitions(&mut self, session: &Session, before: &Observation, reduced_motion: bool) {
        self.draws.clear();
        self.map.clear();
        let target = self.base_view(session);
        self.announcement = phase_banner::Announcement::between(before, &target);
        if reduced_motion {
            return;
        }
        self.map = map_motion::Playback::new(before, &target);
        let sequence = self.moments.get(self.index).map(|moment| moment.sequence);
        let batches = self
            .draw_cues
            .iter()
            .filter(|cue| cue.revision == target.revision && cue.sequence == sequence)
            .map(|cue| cue.batch.clone())
            .collect::<Vec<_>>();
        self.draws = draws::Playback::new(before, &target, &batches);
    }

    /// Called only when the adapter is rebuilding the actual scene. Its view is
    /// also retained by the adapter as evidence for subsequent advancement.
    pub(super) fn prepare(&mut self, session: &Session, doom_moving: bool) -> Frame {
        let transfers = self.draws.take_transfers();
        let view = self.view(session);
        if !self.drawing() {
            self.map.rendered();
        }
        if self.announcement().is_none() && !doom_moving {
            if !self.drawing() && !self.map.active() {
                self.encounter.arrive(
                    view.revision,
                    self.moments.get(self.index).map(|moment| moment.sequence),
                );
            }
            self.results.synchronize(&view);
        }
        let story = if let Some(moment) = self.moments.get(self.index) {
            story::Faces::from_codes(moment.revealed_story.iter().map(String::as_str))
        } else {
            story::Faces::from_codes(session.live_state().known_story.iter().map(String::as_str))
        };
        if self.announcement().is_none() && !doom_moving {
            if view.revision == 0 {
                self.story = story::Reading::at(story.clone());
            } else {
                self.story.synchronize(&story);
            }
        }
        Frame {
            view,
            story,
            transfers,
            map_arrivals: self.map.take_arrivals(),
        }
    }

    /// The adapter checks its distinct dialog, health and consequence blockers.
    /// Even a long unblocked tick cannot skip a beat that has not reached the
    /// scene or whose physical cards are still moving.
    pub(super) fn advance(
        &mut self,
        session: &Session,
        delta: f32,
        last_rendered: Option<&Observation>,
        moving: bool,
        reduced_motion: bool,
    ) -> Progress {
        if !self.active() {
            return Progress::Waiting;
        }
        let view = self.view(session);
        if last_rendered != Some(&view) {
            return Progress::Waiting;
        }
        if self.announcement().is_some() {
            if self
                .announcement
                .as_mut()
                .expect("Visible announcement")
                .advance(delta)
            {
                self.announcement = None;
                return Progress::Moment;
            }
            return Progress::Waiting;
        }
        if self.drawing() {
            if self.draws.advance(delta, moving) {
                self.clock = 0.;
                return Progress::Draw;
            }
            return Progress::Waiting;
        }
        if self.map.active() {
            return if self.map.advance(delta, moving) {
                Progress::Map
            } else {
                Progress::Waiting
            };
        }
        self.clock += delta;
        let play = view
            .play
            .as_ref()
            .expect("A presentation moment has play state");
        let duration = if play
            .test
            .as_ref()
            .is_some_and(|test| !test.revealed.is_empty())
            || play.pending.harm.is_some()
        {
            1.0
        } else {
            motion::CARD_DURATION
        };
        if self.clock < duration || moving {
            return Progress::Waiting;
        }
        self.index += 1;
        self.clock = 0.;
        self.start_transitions(session, &view, reduced_motion);
        Progress::Moment
    }

    pub(super) fn returning(&self, id: u16) -> Option<DrawSource> {
        self.draws.returning(id)
    }

    pub(super) fn deck_motion(&self) -> Option<(shuffle::Operation, f32)> {
        self.draws.deck_motion()
    }

    pub(super) fn render_draws(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        context: &CardRender<'_>,
    ) {
        self.draws.render(commands, meshes, materials, context);
    }

    /// Synthetic populated-layout input, never used by lifecycle tests.
    #[cfg(test)]
    pub(super) fn at_moment(session: &Session, moment: PresentationMoment) -> Self {
        Self {
            moments: vec![moment],
            ..Self::new(session)
        }
    }
}
