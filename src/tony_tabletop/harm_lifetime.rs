//! Lifetime of already committed live harm and the animation shared with replay.
//! Replay retains its own sequence and navigation; this owner never advances it.

use crate::tony::{
    Observation,
    presentation::{PresentationStage, stages},
};
use std::collections::VecDeque;

const STAGE_SECONDS: f32 = 1.2;
const MOTION_SECONDS: f32 = 0.5;

pub(super) enum Playback {
    Live,
    Replay { harm_visible: bool },
}

#[derive(Default)]
pub(super) struct HarmPresentation {
    live: VecDeque<PresentationStage>,
    elapsed: f32,
}

impl HarmPresentation {
    /// Call only after saving and installing the committed choice.
    pub fn committed_choice(&mut self, before: &Observation, after: &Observation) {
        self.live = stages(before, after)
            .into_iter()
            .filter(|stage| stage.harm.is_some())
            .collect();
        self.elapsed = 0.0;
    }

    pub fn current_stage(&self) -> Option<&PresentationStage> {
        self.live.front()
    }

    pub fn has_pending(&self) -> bool {
        !self.live.is_empty()
    }

    pub fn animation_progress(&self) -> f32 {
        (self.elapsed / MOTION_SECONDS).clamp(0.0, 1.0)
    }

    /// Successful reopen, replay opening, return to live, or new turn.
    pub fn discard_staging(&mut self) {
        self.live.clear();
        self.elapsed = 0.0;
    }

    /// Every replay navigation attempt restarts motion, including at endpoints.
    pub fn replay_navigated(&mut self) {
        self.elapsed = 0.0;
    }

    /// Manual live completion and timed completion share the same reset.
    pub fn finish_live_stage(&mut self) {
        self.live.pop_front();
        self.elapsed = 0.0;
    }

    /// Returns whether the displayed live stage changed and needs a redraw.
    /// Each call completes at most one stage and discards excess elapsed time.
    pub fn tick(&mut self, seconds: f32, paused: bool, playback: Playback) -> bool {
        let visible = match playback {
            Playback::Live => self.has_pending(),
            Playback::Replay { harm_visible } => harm_visible,
        };
        if paused || !visible {
            return false;
        }
        self.elapsed += seconds;
        if matches!(playback, Playback::Live) && self.elapsed >= STAGE_SECONDS {
            self.finish_live_stage();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TokenFace, tony::Session};

    fn investigation() -> (Observation, Observation) {
        let mut session = Session::with_draws(vec![TokenFace::Zero]);
        let before = session.observe();
        session.choose(before.revision, "investigate").unwrap();
        (before, session.observe())
    }

    #[test]
    fn committed_choices_replace_staging_and_filter_out_settled_observations() {
        let (before, after) = investigation();
        let mut presentation = HarmPresentation::default();
        presentation.committed_choice(&before, &after);
        let stage = presentation.current_stage().unwrap();
        assert_eq!((stage.view.damage, stage.view.horror), (1, 1));
        assert!(stage.view.choices.is_empty());
        assert!(presentation.has_pending());
        assert!(!presentation.tick(0.25, false, Playback::Live));
        assert_eq!(presentation.animation_progress(), 0.5);

        presentation.committed_choice(&before, &after);
        assert_eq!(presentation.animation_progress(), 0.0);
        presentation.finish_live_stage();
        assert!(!presentation.has_pending());
        assert!(presentation.current_stage().is_none());

        presentation.committed_choice(&before, &after);
        presentation.tick(0.25, false, Playback::Live);
        presentation.committed_choice(&before, &before);
        assert!(!presentation.has_pending());
        assert_eq!(presentation.animation_progress(), 0.0);
    }

    #[test]
    fn paused_and_idle_ticks_do_not_spend_the_stage_duration() {
        let (before, after) = investigation();
        let mut presentation = HarmPresentation::default();
        assert!(!presentation.tick(5.0, false, Playback::Live));
        assert_eq!(presentation.animation_progress(), 0.0);
        presentation.committed_choice(&before, &after);
        assert!(!presentation.tick(0.25, false, Playback::Live));
        assert!(!presentation.tick(5.0, true, Playback::Live));
        assert_eq!(presentation.animation_progress(), 0.5);
        assert!(!presentation.tick(0.25, false, Playback::Live));
        assert_eq!(presentation.animation_progress(), 1.0);
        assert!(presentation.has_pending());
        assert!(presentation.tick(0.71, false, Playback::Live));
        assert!(!presentation.has_pending());
        assert_eq!(presentation.animation_progress(), 0.0);
    }

    #[test]
    fn replay_motion_settles_and_restarts_without_consuming_live_stages() {
        let (before, after) = investigation();
        let mut presentation = HarmPresentation::default();
        presentation.committed_choice(&before, &after);
        assert!(!presentation.tick(0.25, false, Playback::Replay { harm_visible: true }));
        assert_eq!(presentation.animation_progress(), 0.5);
        assert!(!presentation.tick(
            5.0,
            false,
            Playback::Replay {
                harm_visible: false
            }
        ));
        assert_eq!(presentation.animation_progress(), 0.5);
        assert!(!presentation.tick(5.0, false, Playback::Replay { harm_visible: true }));
        assert_eq!(presentation.animation_progress(), 1.0);
        assert!(presentation.has_pending());
        presentation.replay_navigated();
        assert_eq!(presentation.animation_progress(), 0.0);
        assert!(presentation.has_pending());
        presentation.discard_staging();
        assert!(!presentation.has_pending());
        assert_eq!(presentation.animation_progress(), 0.0);
    }
}
