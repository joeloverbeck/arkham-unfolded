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
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const ACTIVE_FRAME: Duration = Duration::from_nanos(16_666_667);
const INPUT_SETTLE: Duration = Duration::from_millis(150);
const STARTUP_SETTLE: Duration = Duration::from_secs(2);
const COMPARISON_TITLE: &str = " · F9 cadence: ";

/// Decision-only comparison for #151, removed by its adopt or reject outcome.
/// F9 cycles the active-motion frame limit so one launch can judge camera motion under each.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ComparedCadence {
    #[default]
    Cap60,
    Cap72,
    DisplayRate,
}

impl ComparedCadence {
    fn next(self) -> Self {
        match self {
            Self::Cap60 => Self::Cap72,
            Self::Cap72 => Self::DisplayRate,
            Self::DisplayRate => Self::Cap60,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cap60 => "60/s baseline",
            Self::Cap72 => "72/s",
            Self::DisplayRate => "display rate",
        }
    }

    fn active_limit(self) -> Duration {
        match self {
            Self::Cap60 => ACTIVE_FRAME,
            // About two refreshes of the owner's 143.99 Hz display; sleep is not phase-locked.
            Self::Cap72 => Duration::from_nanos(13_888_889),
            // FIFO presentation alone paces active dispatch.
            Self::DisplayRate => Duration::ZERO,
        }
    }
}

/// The render-extract sub-app cannot read main-world resources, so both hold this handle.
#[derive(Resource, Clone)]
struct FrameLimit(Arc<AtomicU64>);

impl Default for FrameLimit {
    fn default() -> Self {
        let limit = Self(Arc::default());
        limit.set(ACTIVE_FRAME);
        limit
    }
}

impl FrameLimit {
    fn set(&self, limit: Duration) {
        let nanos = u64::try_from(limit.as_nanos()).unwrap_or(u64::MAX);
        self.0.store(nanos, Ordering::Relaxed);
    }

    fn get(&self) -> Duration {
        Duration::from_nanos(self.0.load(Ordering::Relaxed))
    }
}

/// Presentation uses main-update time, not a delayed timestamp from the render thread.
#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct Presentation;
pub(crate) type PresentationTime = Time<Presentation>;

#[derive(Resource, Default)]
pub(crate) struct Redraw {
    requested: bool,
}

impl Redraw {
    /// Keep ticking while a visible transition or capture has unfinished work.
    pub(crate) fn request(&mut self) {
        self.requested = true;
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RequestRedraw;

/// The final activity decision for this frame, before render extraction can block.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SelectCadence;

#[derive(Resource, Default)]
struct Cadence {
    last_update: Option<Instant>,
    active_until: Option<Instant>,
    idle: bool,
}

impl Cadence {
    fn begin(&mut self, now: Instant) -> Duration {
        let delta = self.last_update.map_or(Duration::ZERO, |last| {
            now.saturating_duration_since(last).min(if self.idle {
                // A newly pressed key or queued action must not inherit thinking time.
                ACTIVE_FRAME
            } else {
                // Retain Bevy's ordinary slow-frame bound while motion is underway.
                Duration::from_millis(250)
            })
        });
        self.last_update = Some(now);
        self.active_until.get_or_insert(now + STARTUP_SETTLE);
        delta
    }

    fn finish(&mut self, requested: bool) -> WinitSettings {
        let now = self
            .last_update
            .expect("clock runs before cadence selection");
        if requested {
            self.active_until = Some(self.active_until.unwrap().max(now + INPUT_SETTLE));
        }
        self.idle = now >= self.active_until.unwrap();
        if self.idle {
            WinitSettings {
                focused_mode: UpdateMode::reactive_low_power(Duration::from_millis(250)),
                unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(1)),
            }
        } else {
            WinitSettings::continuous()
        }
    }
}

pub(crate) struct TabletopPacingPlugin;

impl Plugin for TabletopPacingPlugin {
    fn build(&self, app: &mut App) {
        let limit = FrameLimit::default();
        app.init_resource::<PresentationTime>()
            .insert_resource(limit.clone())
            .init_resource::<ComparedCadence>()
            .init_resource::<Redraw>()
            .init_resource::<Cadence>()
            .init_resource::<WinitSettings>()
            .add_systems(First, update_clock)
            .configure_sets(Last, SelectCadence.after(RequestRedraw))
            .add_systems(Last, input_activity.in_set(RequestRedraw))
            .add_systems(Last, select_cadence.in_set(SelectCadence))
            .add_systems(Last, compare_cadence.before(SelectCadence));

        if let Some(extract) = app.get_sub_app_mut(RenderExtractApp) {
            // This sub-app runs after render dispatch, before the next input poll.
            // Its schedule is unset by default; adding a system alone would not run it.
            extract.update_schedule = Some(Main.intern());
            extract
                .insert_resource(limit)
                .add_systems(Main, limit_frame_rate);
        }
    }
}

fn limit_frame_rate(limit: Res<FrameLimit>, mut previous: Local<Option<Instant>>) {
    if let Some(previous) = *previous
        && let Some(remaining) = limit.get().checked_sub(previous.elapsed())
    {
        std::thread::sleep(remaining);
    }
    *previous = Some(Instant::now());
}

fn compare_cadence(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<ComparedCadence>,
    mut labelled: Local<bool>,
    mut windows: Query<&mut Window>,
) {
    if keys.just_pressed(KeyCode::F9) {
        *selected = selected.next();
    } else if *labelled {
        return;
    }
    *labelled = true;
    info!("Cadence comparison (#151): {}", selected.label());
    // The owner judges this run; the agent desktop helper refuses the suffixed title.
    for mut window in &mut windows {
        let base = window
            .title
            .split(COMPARISON_TITLE)
            .next()
            .unwrap_or_default()
            .to_owned();
        window.title = format!("{base}{COMPARISON_TITLE}{}", selected.label());
    }
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
    mut stats: Local<UpdateStats>,
) {
    let now = Instant::now();
    if let Some(previous) = cadence.last_update {
        stats.longest_gap = stats
            .longest_gap
            .max(now.saturating_duration_since(previous));
    }
    time.advance_by(cadence.begin(now));
    let since = *stats.since.get_or_insert(now);
    stats.count += 1;
    let elapsed = now.saturating_duration_since(since);
    if elapsed >= Duration::from_secs(1) {
        // Opt-in via RUST_LOG; these are app updates, not GPU presentation timestamps.
        debug!(
            "Tabletop updates: {} in {:.3}s; longest gap {:.1}ms; idle={}",
            stats.count,
            elapsed.as_secs_f64(),
            stats.longest_gap.as_secs_f64() * 1000.0,
            cadence.idle
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
    // Raw mouse motion also arrives while another application has focus. Logical
    // cursor events cover our camera/hover input without waking for that traffic.
    for event in events.read() {
        if !matches!(
            event,
            WindowEvent::MouseMotion(_) | WindowEvent::RequestRedraw(_)
        ) {
            redraw.request();
        }
    }
    if windows.iter().any(|window| window.focused)
        && (keys.get_pressed().next().is_some() || buttons.get_pressed().next().is_some())
    {
        redraw.request();
    }
}

fn select_cadence(
    mut cadence: ResMut<Cadence>,
    mut redraw: ResMut<Redraw>,
    mut settings: ResMut<WinitSettings>,
    compared: Res<ComparedCadence>,
    limit: Res<FrameLimit>,
) {
    let next = cadence.finish(std::mem::take(&mut redraw.requested));
    // Settled dispatch keeps the baseline cap in every compared mode.
    limit.set(if cadence.idle {
        ACTIVE_FRAME
    } else {
        compared.active_limit()
    });
    if settings.focused_mode != next.focused_mode || settings.unfocused_mode != next.unfocused_mode
    {
        debug!(
            "Tabletop cadence: {}",
            if cadence.idle { "idle" } else { "active" }
        );
        *settings = next;
    }
}

#[cfg(test)]
mod tests;
