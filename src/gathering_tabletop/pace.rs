//! Readings that continue on their own (#210). An agent partner's own test result,
//! and an encounter card its investigator draws, are shown as usual, then continue
//! after a reading delay proportional to their text, with visible progress. The
//! pointer over the reading holds it; a click on it holds it until Continue; Enter
//! activates its focused Continue at once. Reduced motion removes the progress
//! bar's motion, not the delay. Every other reading waits for the human's Continue,
//! as without partners: story readings, anything at the scenario's end, and a
//! result or encounter reading that names one of the human's investigators or
//! offers the human a response.
//!
//! Continuing a reading is the human's own acknowledgment, queued as an action; it
//! never decides anything for the rules session.

use super::*;
use crate::partners::Controller;
use bevy::picking::{events::PointerClick, hover::Hovered};

/// Reading time: a glance, then a share per word, about 240 words a minute.
const GLANCE: f32 = 1.5;
const PER_WORD: f32 = 0.25;
/// The shortest and longest delays, whatever the text.
const SHORTEST: f32 = 3.;
const LONGEST: f32 = 15.;

/// The reading delay for `text`, in seconds.
pub(super) fn delay(text: &str) -> f32 {
    (GLANCE + text.split_whitespace().count() as f32 * PER_WORD).clamp(SHORTEST, LONGEST)
}

/// A part of a reading (its heading, body or footer) where the pointer holds it and
/// a click holds it until Continue. Every reading has them; only a paced reading
/// counts down. The dialog's own frame keeps one entity across rebuilds and carries
/// only its look, so its parts carry this instead.
#[derive(Component)]
pub(super) struct Frame;

/// The countdown line of a paced reading, updated in place.
#[derive(Component)]
pub(super) struct Countdown;

/// The filled part of a paced reading's progress bar, updated in place.
#[derive(Component)]
pub(super) struct Fill;

/// Which reading a pace belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    /// A result, by how many results the table had acknowledged before it.
    Result(usize),
    /// An encounter reading, by its physical card.
    Encounter(u16),
}

/// The current paced reading's clock, kept across the table's rebuilds.
#[derive(Default)]
pub(super) struct Pace {
    key: Option<Key>,
    elapsed: f32,
    held: bool,
    hovered: bool,
    continued: bool,
}

impl Pace {
    /// Whether a paced reading is counting down now.
    pub(super) fn counting(&self) -> bool {
        self.key.is_some() && !self.held && !self.hovered && !self.continued
    }

    /// Hold the reading until the human continues it.
    pub(super) fn hold(&mut self) {
        if self.key.is_some() {
            self.held = true;
        }
    }

    /// What the reading's countdown line says.
    pub(super) fn line(&self, delay: f32) -> String {
        if self.held {
            "Held. Continue when you are ready.".into()
        } else if self.hovered {
            "Held while the pointer is over it. Enter continues now.".into()
        } else {
            let seconds = (delay - self.elapsed).max(0.).ceil();
            format!(
                "Continues on its own in {seconds} s. Hover or click to hold; Enter continues now."
            )
        }
    }

    /// The share of the delay already passed.
    pub(super) fn progress(&self, delay: f32) -> f32 {
        (self.elapsed / delay).clamp(0., 1.)
    }
}

/// A reading that continues on its own.
pub(super) struct Paced {
    key: Key,
    delay: f32,
    /// The human's Continue for this reading.
    continuation: Action,
}

/// The reading the table shows now, if it continues on its own.
pub(super) fn paced(table: &Table) -> Option<Paced> {
    let control = table.presentation.control();
    if control.solo() {
        return None;
    }
    let partner = |who: Investigator| control.controller(who) != Controller::Human;
    let ended = || {
        table
            .view()
            .play
            .is_some_and(|play| play.scenario.outcome.is_some())
    };
    if let Some(report) = table.presentation.result() {
        if ended() {
            return None;
        }
        let detail = &report.breakdown;
        let own = std::iter::once(report.tester)
            .chain(detail.cards.iter().map(|row| row.owner))
            .chain(detail.bonuses.iter().map(|row| row.owner))
            .all(partner);
        return own.then(|| Paced {
            key: Key::Result(table.presentation.results_acknowledged()),
            delay: delay(&results::text(report)),
            continuation: Action::ContinueResult(table.epoch),
        });
    }
    if table.presentation.story().is_some() {
        return None;
    }
    let reading = table.presentation.encounter();
    let reveal = reading.current()?;
    if ended() {
        return None;
    }
    let responses = reading
        .decision(&table.session)
        .is_some_and(|_| !encounter::Reading::responses(table).is_empty());
    (partner(reveal.drawn_by) && !responses).then(|| Paced {
        key: Key::Encounter(reveal.card.id),
        delay: delay(&encounter::text(&reveal.card)),
        continuation: Action::Encounter(table.epoch, encounter::Input::Continue),
    })
}

/// The progress shown in a paced reading's footer: the countdown line, and a bar
/// unless motion is reduced.
pub(super) fn progress(footer: &mut ChildSpawnerCommands, art: &art::Art, table: &Table) {
    let Some(paced) = paced(table) else {
        return;
    };
    let pace = &table.pace;
    let fresh = Pace::default();
    let pace = if pace.key == Some(paced.key) {
        pace
    } else {
        &fresh
    };
    footer.spawn((
        art.text(pace.line(paced.delay), 15., MUTED),
        TextLayout::justify(Justify::Center),
        Countdown,
    ));
    if !table.reduced_motion {
        footer
            .spawn((
                Node {
                    height: px(4),
                    width: percent(100),
                    flex_shrink: 0.,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.65, 0.74, 0.73, 0.25)),
                Pickable::IGNORE,
            ))
            .with_children(|bar| {
                bar.spawn((
                    Node {
                        width: percent(100. * pace.progress(paced.delay)),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(MUTED),
                    Fill,
                    Pickable::IGNORE,
                ));
            });
    }
}

/// Make a part of a reading hold it under the pointer, and until Continue once
/// clicked.
pub(super) fn frame(part: &mut EntityCommands) {
    part.insert((Frame, Hovered::default())).observe(
        |_: On<PointerClick>, mut table: ResMut<Table>| {
            table.pace.hold();
        },
    );
}

/// Count the paced reading down, show its progress in place, and continue it once
/// its delay has passed. A rebuild of the table's panels keeps its clock.
#[allow(clippy::too_many_arguments)]
pub(super) fn advance(
    time: Res<PresentationTime>,
    health: Res<RenderHealth>,
    mut table: ResMut<Table>,
    frames: Query<&Hovered, With<Frame>>,
    mut lines: Query<&mut Text, With<Countdown>>,
    mut fills: Query<&mut Node, With<Fill>>,
    mut actions: ResMut<Actions>,
    mut redraw: ResMut<Redraw>,
) {
    let Some(paced) = paced(&table) else {
        if table.pace.key.is_some() {
            table.pace = Pace::default();
        }
        return;
    };
    let pace = &mut table.pace;
    if pace.key != Some(paced.key) {
        *pace = Pace {
            key: Some(paced.key),
            ..default()
        };
    }
    pace.hovered = frames.iter().any(|hovered| hovered.0);
    if pace.counting() && health.allows_play() {
        pace.elapsed += time.delta_secs();
        if pace.elapsed >= paced.delay {
            pace.continued = true;
            actions.0.push(paced.continuation);
            redraw.request();
        } else {
            // Whole frames keep the clock and the bar moving at their real pace.
            redraw.decorate();
        }
    }
    let line = pace.line(paced.delay);
    for mut text in &mut lines {
        if text.0 != line {
            text.0.clone_from(&line);
        }
    }
    let width = percent(100. * pace.progress(paced.delay));
    for mut fill in &mut fills {
        if fill.width != width {
            fill.width = width;
        }
    }
}
