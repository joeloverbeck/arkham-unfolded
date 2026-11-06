//! Physical custody changes for Premonition's already-known chaos tokens.
use super::*;
use crate::{TokenFace, gathering::PlayObservation};

const DURATION: f32 = 0.6;

#[derive(Component)]
pub(super) struct Piece {
    pub face: TokenFace,
    pub destination: Vec3,
}

struct Travel {
    face: TokenFace,
    start: Vec3,
    end: Vec3,
}

#[derive(Default)]
pub(super) struct State {
    travels: Vec<Travel>,
    elapsed: f32,
    fresh: bool,
    departing_card: Option<u16>,
}

impl State {
    pub(super) fn active(&self) -> bool {
        !self.travels.is_empty() && self.elapsed < DURATION
    }

    pub(super) fn holds(&self, card: u16) -> bool {
        self.active() && self.departing_card == Some(card)
    }

    pub(super) fn prepare(
        &mut self,
        before: Option<&Observation>,
        after: &Observation,
        reduced: bool,
    ) {
        if reduced || before.is_none() {
            *self = Self::default();
            return;
        }
        if before == Some(after) {
            return;
        }
        *self = Self::default();
        let Some((before, after)) = before
            .and_then(|before| before.play.as_ref())
            .zip(after.play.as_ref())
        else {
            return;
        };
        if let Some(sealed) = &before.premonition
            && after
                .premonition
                .as_ref()
                .is_none_or(|next| next.card.id != sealed.card.id)
            && let Some(tokens) = &after.tokens
            && tokens.revealed.starts_with(&sealed.faces)
        {
            for (slot, face) in sealed.faces.iter().enumerate() {
                self.travels.push(Travel {
                    face: *face,
                    start: trays::sealed_token(before, sealed.controller, slot, sealed.faces.len()),
                    end: token_position(after, slot),
                });
            }
            self.departing_card = Some(sealed.card.id);
        } else if let Some(sealed) = &after.premonition
            && before
                .premonition
                .as_ref()
                .is_none_or(|prior| prior.card.id != sealed.card.id)
            && let Some(tokens) = &before.tokens
            && tokens.sealing_card() == Some(sealed.card.id)
        {
            let mut available: Vec<_> = tokens
                .revealed
                .iter()
                .enumerate()
                .filter(|(slot, _)| {
                    !tokens.ignored.contains(slot) && !tokens.cancelled.contains(slot)
                })
                .collect();
            for (slot, face) in sealed.faces.iter().enumerate() {
                if let Some(index) = available.iter().position(|(_, prior)| *prior == face) {
                    let (prior, _) = available.remove(index);
                    self.travels.push(Travel {
                        face: *face,
                        start: token_position(before, prior),
                        end: trays::sealed_token(
                            after,
                            sealed.controller,
                            slot,
                            sealed.faces.len(),
                        ),
                    });
                }
            }
        }
        self.fresh = self.active();
    }
}

fn token_position(play: &PlayObservation, slot: usize) -> Vec3 {
    if play
        .tokens
        .as_ref()
        .is_some_and(|tokens| tokens.is_skill_test())
    {
        layout::test_token(false, slot)
    } else {
        trays::Layout::for_play(play)
            .expect("Non-test token resolution has a tray")
            .token(slot)
    }
}

pub(super) fn animate(
    mut table: ResMut<Table>,
    time: Res<PresentationTime>,
    mut tokens: Query<(&Piece, &mut Transform)>,
) {
    let paused = table.motion_blocked();
    let reduced = table.reduced_motion;
    let state = &mut table.sealed;
    let was_active = state.active();
    if reduced {
        state.elapsed = DURATION;
    } else if state.fresh {
        state.fresh = false;
    } else if !paused {
        state.elapsed += time.delta_secs();
    }
    let fraction = (state.elapsed / DURATION).clamp(0., 1.);
    let smooth = fraction * fraction * (3. - 2. * fraction);
    for (piece, mut at) in &mut tokens {
        if let Some(travel) = state.travels.iter().find(|travel| {
            travel.face == piece.face && travel.end.abs_diff_eq(piece.destination, 0.001)
        }) {
            at.translation = travel.start.lerp(travel.end, smooth)
                + Vec3::Y * (std::f32::consts::PI * fraction).sin() * 0.5;
        }
    }
    if was_active && !state.active() {
        table.dirty = true;
    }
}
