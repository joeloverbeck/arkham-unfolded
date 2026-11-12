//! Redraw cadence for the two native tables. Rules never use this clock.

use bevy::{
    app::Main,
    ecs::schedule::ScheduleLabel,
    prelude::*,
    render::pipelined_rendering::RenderExtractApp,
    window::WindowEvent,
    winit::{UpdateMode, WinitSettings},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Caps settled dispatch, and bounds the step of any update that follows a settled one.
/// Moving frames follow FIFO presentation: the owner saw sleep caps judder at 144 Hz (#151).
const SETTLED_FRAME: Duration = Duration::from_nanos(16_666_667);
/// How often a table that only shows slow looping decorations is drawn: 60/s. A wait
/// that input interrupts paces it, not a sleep after dispatch. In #188's launch the
/// owner found the cue's breathing less smooth at 30/s and noticed no issue at 60/s,
/// which drew about 120 W less than the display rate there.
const DECORATIVE_FRAME: Duration = Duration::from_nanos(16_666_667);
const INPUT_SETTLE: Duration = Duration::from_millis(150);
const STARTUP_SETTLE: Duration = Duration::from_secs(2);

/// The render-extract sub-app cannot read main-world resources, so both hold this handle.
#[derive(Resource, Clone, Default)]
pub(crate) struct Settled(Arc<AtomicBool>);

impl Settled {
    fn set(&self, settled: bool) {
        self.0.store(settled, Ordering::Relaxed);
    }

    /// Whether this update chose to wait for input: no motion, work or decoration needs frames.
    pub(crate) fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Presentation uses main-update time, not a delayed timestamp from the render thread.
#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct Presentation;
pub(crate) type PresentationTime = Time<Presentation>;

/// What this update's requests ask of the next frames, weakest first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum Demand {
    #[default]
    None,
    /// Only slow looping decorations.
    Decorative,
    /// Unfinished work or input.
    Display,
}

#[derive(Resource, Default)]
pub(crate) struct Redraw {
    demand: Demand,
}

impl Redraw {
    /// Keep ticking while a visible transition or capture has unfinished work.
    pub(crate) fn request(&mut self) {
        self.demand = Demand::Display;
    }

    /// Keep drawing a slow looping decoration, at the decorative rate unless
    /// something else holds the display rate. Its phase must come from `PresentationTime`.
    pub(crate) fn decorate(&mut self) {
        self.demand = self.demand.max(Demand::Decorative);
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RequestRedraw;

/// The final activity decision for this frame, before render extraction can block.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SelectCadence;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Pace {
    /// Display rate: motion, presentations, checks, captures or recent input.
    #[default]
    Active,
    /// Only slow looping decorations ask for frames.
    Decorative,
    /// Nothing asks for frames; the table waits for input.
    Settled,
}

impl Pace {
    fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Decorative => "decorative",
            Self::Settled => "settled",
        }
    }
}

#[derive(Resource, Default)]
struct Cadence {
    last_update: Option<Instant>,
    active_until: Option<Instant>,
    pace: Pace,
}

impl Cadence {
    /// `input` says that a window event other than raw mouse motion woke this update.
    fn begin(&mut self, now: Instant, input: bool) -> Duration {
        let delta = self.last_update.map_or(Duration::ZERO, |last| {
            now.saturating_duration_since(last).min(
                if self.pace == Pace::Settled || (self.pace == Pace::Decorative && input) {
                    // A newly pressed key or queued action must not inherit thinking time.
                    SETTLED_FRAME
                } else {
                    // Retain Bevy's ordinary slow-frame bound while motion is underway.
                    // Decorative frames take their whole interval, so their phase keeps time.
                    Duration::from_millis(250)
                },
            )
        });
        self.last_update = Some(now);
        self.active_until.get_or_insert(now + STARTUP_SETTLE);
        delta
    }

    fn finish(&mut self, demand: Demand) -> WinitSettings {
        let now = self
            .last_update
            .expect("clock runs before cadence selection");
        if demand == Demand::Display {
            self.active_until = Some(self.active_until.unwrap().max(now + INPUT_SETTLE));
        }
        self.pace = if now < self.active_until.unwrap() {
            Pace::Active
        } else if demand == Demand::Decorative {
            Pace::Decorative
        } else {
            Pace::Settled
        };
        match self.pace {
            Pace::Active => WinitSettings::continuous(),
            // Decorations stop without focus, so the unfocused mode only bridges one update.
            Pace::Decorative => WinitSettings {
                focused_mode: UpdateMode::reactive_low_power(DECORATIVE_FRAME),
                unfocused_mode: UpdateMode::reactive_low_power(DECORATIVE_FRAME),
            },
            Pace::Settled => WinitSettings {
                focused_mode: UpdateMode::reactive_low_power(Duration::from_millis(250)),
                unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(1)),
            },
        }
    }
}

pub(crate) struct TabletopPacingPlugin;

impl Plugin for TabletopPacingPlugin {
    fn build(&self, app: &mut App) {
        let settled = Settled::default();
        app.init_resource::<PresentationTime>()
            .insert_resource(settled.clone())
            .init_resource::<Redraw>()
            .init_resource::<Cadence>()
            .init_resource::<WinitSettings>()
            .add_systems(First, update_clock)
            .configure_sets(Last, SelectCadence.after(RequestRedraw))
            .add_systems(Last, input_activity.in_set(RequestRedraw))
            .add_systems(Last, select_cadence.in_set(SelectCadence));

        if let Some(extract) = app.get_sub_app_mut(RenderExtractApp) {
            // This sub-app runs after render dispatch, before the next input poll.
            // Its schedule is unset by default; adding a system alone would not run it.
            extract.update_schedule = Some(Main.intern());
            extract
                .insert_resource(settled)
                .add_systems(Main, limit_frame_rate);
        }
    }
}

fn limit_frame_rate(settled: Res<Settled>, mut previous: Local<Option<Instant>>) {
    if settled.get()
        && let Some(previous) = *previous
        && let Some(remaining) = SETTLED_FRAME.checked_sub(previous.elapsed())
    {
        std::thread::sleep(remaining);
    }
    *previous = Some(Instant::now());
}

#[derive(Default)]
struct UpdateStats {
    since: Option<Instant>,
    count: u32,
    longest_gap: Duration,
}

fn update_clock(
    mut cadence: ResMut<Cadence>,
    mut time: ResMut<PresentationTime>,
    mut events: MessageReader<WindowEvent>,
    mut stats: Local<UpdateStats>,
) {
    let now = Instant::now();
    if let Some(previous) = cadence.last_update {
        stats.longest_gap = stats
            .longest_gap
            .max(now.saturating_duration_since(previous));
    }
    let input = read_input(&mut events);
    time.advance_by(cadence.begin(now, input));
    let since = *stats.since.get_or_insert(now);
    stats.count += 1;
    let elapsed = now.saturating_duration_since(since);
    if elapsed >= Duration::from_secs(1) {
        // Opt-in via RUST_LOG; these are app updates, not GPU presentation timestamps.
        debug!(
            "Tabletop updates: {} in {:.3}s; longest gap {:.1}ms; pace={}",
            stats.count,
            elapsed.as_secs_f64(),
            stats.longest_gap.as_secs_f64() * 1000.0,
            cadence.pace.label()
        );
        stats.since = Some(now);
        stats.count = 0;
        stats.longest_gap = Duration::ZERO;
    }
}

fn input_activity(
    mut events: MessageReader<WindowEvent>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut redraw: ResMut<Redraw>,
) {
    if read_input(&mut events)
        || (windows.iter().any(|window| window.focused)
            && (keys.get_pressed().next().is_some() || buttons.get_pressed().next().is_some()))
    {
        redraw.request();
    }
}

/// Reads every pending window event (`any` would stop early and leave the rest for
/// the next update). Raw mouse motion also arrives while another application has
/// focus. Logical cursor events cover our camera/hover input without waking for it.
fn read_input(events: &mut MessageReader<WindowEvent>) -> bool {
    events
        .read()
        .filter(|event| {
            !matches!(
                event,
                WindowEvent::MouseMotion(_) | WindowEvent::RequestRedraw(_)
            )
        })
        .count()
        > 0
}

fn select_cadence(
    mut cadence: ResMut<Cadence>,
    mut redraw: ResMut<Redraw>,
    mut settings: ResMut<WinitSettings>,
    settled: Res<Settled>,
) {
    let previous = cadence.pace;
    let next = cadence.finish(std::mem::take(&mut redraw.demand));
    settled.set(cadence.pace == Pace::Settled);
    if cadence.pace != previous {
        debug!("Tabletop cadence: {}", cadence.pace.label());
    }
    if settings.focused_mode != next.focused_mode || settings.unfocused_mode != next.unfocused_mode
    {
        *settings = next;
    }
}

#[cfg(test)]
mod tests;
