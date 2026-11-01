//! Admission to native play depends on observed rendering progress, not GPU-memory guesses.

use crate::tabletop_pacing::{Redraw, RequestRedraw, SelectCadence};
use bevy::{
    app::AppExit,
    asset::{LoadState, RecursiveDependencyLoadState, UntypedAssetId},
    picking::Pickable,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_resource::{CachedPipelineState, PipelineCache},
        renderer::{RenderQueue, render_system},
        view::window::ExtractedWindow,
    },
    window::WindowOccluded,
    winit::{UpdateMode, WinitSettings},
};
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const WARMUP: Duration = Duration::from_secs(2);
const SAMPLE: Duration = Duration::from_secs(1);
const STARTUP_LIMIT: Duration = Duration::from_secs(30);
const MIN_RATE: f64 = 30.0;
const REQUIRED_WINDOWS: u32 = 3;
const WATCHING: u8 = 0;
const CANCELLED: u8 = 1;
const EXPIRED: u8 = 2;

#[derive(Resource, Default)]
pub(crate) struct HealthAssets(Vec<UntypedAssetId>);

impl HealthAssets {
    pub(crate) fn new(ids: impl IntoIterator<Item = UntypedAssetId>) -> Self {
        Self(ids.into_iter().collect())
    }
}

#[derive(Default)]
enum Phase {
    #[default]
    Checking,
    Ready,
    Failed(String),
}

#[derive(Resource, Default)]
pub(crate) struct RenderHealth {
    phase: Phase,
    just_ready: bool,
    previous: Option<Observation>,
    foreground_elapsed: Duration,
    warmup_elapsed: Duration,
    window: Option<ProgressWindow>,
    healthy_windows: u32,
    poor_elapsed: Duration,
    last_rates: Option<(f64, f64)>,
}

impl RenderHealth {
    pub(crate) fn allows_play(&self) -> bool {
        matches!(self.phase, Phase::Ready)
    }

    pub(crate) fn is_checking(&self) -> bool {
        matches!(self.phase, Phase::Checking)
    }

    pub(crate) fn just_became_ready(&self) -> bool {
        self.just_ready
    }

    pub(crate) fn status(&self) -> &str {
        match &self.phase {
            Phase::Checking => "Checking graphics...",
            Phase::Ready => "Rendering is ready.",
            Phase::Failed(reason) => reason,
        }
    }

    pub(crate) fn diagnostic_elapsed(&self) -> Duration {
        self.previous.map_or(Duration::ZERO, |sample| sample.now)
    }

    /// Returns whether queued GPU callbacks from the preceding interval must expire.
    fn observe(&mut self, sample: Observation) -> bool {
        self.just_ready = false;
        if matches!(self.phase, Phase::Failed(_)) {
            return false;
        }
        let Some(previous) = self.previous.replace(sample) else {
            return true;
        };
        let elapsed = sample.now.saturating_sub(previous.now);
        if self.is_checking() && sample.foreground && previous.foreground {
            self.foreground_elapsed += elapsed;
            if self.foreground_elapsed >= STARTUP_LIMIT {
                self.phase = Phase::Failed("Graphics could not become ready within 30 seconds of visible foreground operation. No game decision was made; your save is unchanged. Restart the app to retry.".into());
                return true;
            }
        }
        if !(previous.eligible() && sample.eligible()) {
            self.warmup_elapsed = Duration::ZERO;
            self.window = None;
            self.healthy_windows = 0;
            self.poor_elapsed = Duration::ZERO;
            return true;
        }
        if self.is_checking() && self.warmup_elapsed < WARMUP {
            self.warmup_elapsed += elapsed;
            return true;
        }
        let Some(window) = &mut self.window else {
            self.window = Some(ProgressWindow::new(sample));
            return true;
        };
        window.updates += 1;
        let duration = sample.now.saturating_sub(window.started);
        if duration < SAMPLE {
            return false;
        }
        let update_rate = f64::from(window.updates) / duration.as_secs_f64();
        let completion_rate =
            sample.completed.saturating_sub(window.completed) as f64 / duration.as_secs_f64();
        self.last_rates = Some((update_rate, completion_rate));
        if update_rate >= MIN_RATE && completion_rate >= MIN_RATE {
            self.healthy_windows += 1;
            self.poor_elapsed = Duration::ZERO;
        } else {
            self.healthy_windows = 0;
            self.poor_elapsed += duration;
        }
        if self.is_checking() && self.healthy_windows >= REQUIRED_WINDOWS {
            self.phase = Phase::Ready;
            self.just_ready = true;
        } else if self.allows_play() && self.poor_elapsed >= SAMPLE * REQUIRED_WINDOWS {
            self.phase = Phase::Failed("Rendering is not responding fast enough. Gameplay has stopped; the last saved decision is unchanged. Restart the app to retry.".into());
        }
        self.window = Some(ProgressWindow::new(sample));
        true
    }

    #[cfg(test)]
    pub(crate) fn ready_for_test() -> Self {
        Self {
            phase: Phase::Ready,
            ..default()
        }
    }

    #[cfg(test)]
    pub(crate) fn failed_for_test() -> Self {
        Self {
            phase: Phase::Failed("Rendering stopped progressing.".into()),
            ..default()
        }
    }

    #[cfg(test)]
    pub(crate) fn just_ready_for_test() -> Self {
        Self {
            phase: Phase::Ready,
            just_ready: true,
            ..default()
        }
    }
}

#[derive(Clone, Copy)]
struct Observation {
    now: Duration,
    foreground: bool,
    active: bool,
    loaded: bool,
    completed: u64,
}

impl Observation {
    fn eligible(self) -> bool {
        self.foreground && self.active && self.loaded
    }
}

struct ProgressWindow {
    started: Duration,
    updates: u32,
    completed: u64,
}

impl ProgressWindow {
    fn new(sample: Observation) -> Self {
        Self {
            started: sample.now,
            updates: 0,
            completed: sample.completed,
        }
    }
}

pub(crate) struct TabletopHealthPlugin;

#[derive(Resource)]
struct HealthClock {
    origin: Instant,
    #[cfg(test)]
    injected: Option<Duration>,
}

impl Default for HealthClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
            #[cfg(test)]
            injected: None,
        }
    }
}

impl HealthClock {
    fn now(&self) -> Duration {
        #[cfg(test)]
        if let Some(now) = self.injected {
            return now;
        }
        self.origin.elapsed()
    }
}

impl Plugin for TabletopHealthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderHealth>()
            .init_resource::<HealthAssets>()
            .init_resource::<HealthClock>()
            .init_resource::<HealthEnvironment>()
            .init_resource::<GpuProgress>()
            .add_message::<WindowOccluded>()
            .add_message::<AppExit>()
            .add_systems(Startup, show_checking_overlay)
            .add_systems(PreUpdate, (observe_environment, monitor_rendering).chain())
            .add_systems(Update, update_checking_overlay)
            .add_systems(Last, keep_checking_active.in_set(RequestRedraw))
            .add_systems(Last, refresh_watchdog.after(SelectCadence));
        let progress = app.world().resource::<GpuProgress>().clone();
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(progress.clone())
                .init_resource::<SurfaceFrame>()
                .add_systems(
                    Render,
                    (
                        observe_surface
                            .before(render_system)
                            .in_set(RenderSystems::Render),
                        acknowledge_gpu_work
                            .after(render_system)
                            .in_set(RenderSystems::Render),
                    ),
                );
            app.insert_resource(HealthWatchdog::start(progress));
        }
    }
}

#[derive(Resource, Default)]
struct HealthEnvironment {
    foreground: bool,
    active: bool,
    loaded: bool,
}

fn observe_environment(
    windows: Query<(Entity, &Window)>,
    mut events: MessageReader<WindowOccluded>,
    settings: Option<Res<WinitSettings>>,
    assets: Res<HealthAssets>,
    server: Option<Res<AssetServer>>,
    mut occluded: Local<HashSet<Entity>>,
    mut environment: ResMut<HealthEnvironment>,
) {
    for event in events.read() {
        if event.occluded {
            occluded.insert(event.window);
        } else {
            occluded.remove(&event.window);
        }
    }
    environment.foreground = windows.iter().any(|(entity, window)| {
        window.focused
            && window.visible
            && window.physical_width() > 0
            && window.physical_height() > 0
            && !occluded.contains(&entity)
    });
    environment.active =
        settings.is_none_or(|settings| matches!(settings.focused_mode, UpdateMode::Continuous));
    environment.loaded = server.is_none_or(|server| {
        assets.0.iter().all(|id| {
            !matches!(
                server.get_load_state(*id),
                Some(LoadState::Loading | LoadState::NotLoaded)
            ) && !matches!(
                server.get_recursive_dependency_load_state(*id),
                Some(
                    RecursiveDependencyLoadState::Loading | RecursiveDependencyLoadState::NotLoaded
                )
            )
        })
    });
}

fn monitor_rendering(
    mut health: ResMut<RenderHealth>,
    clock: Res<HealthClock>,
    environment: Res<HealthEnvironment>,
    progress: Res<GpuProgress>,
    mut exit: MessageWriter<AppExit>,
    mut diagnostic: Local<Option<(HealthConditions, Duration)>>,
) {
    let was_checking = health.is_checking();
    let was_warming = health.warmup_elapsed < WARMUP;
    let was_failed = matches!(health.phase, Phase::Failed(_));
    let now = clock.now();
    let conditions = HealthConditions {
        foreground: environment.foreground,
        active: environment.active,
        assets_ready: environment.loaded,
        pipelines_ready: !progress.0.pipelines_pending.load(Ordering::Acquire),
    };
    if diagnostic.as_ref().is_none_or(|(previous, emitted)| {
        *previous != conditions && now.saturating_sub(*emitted) >= Duration::from_secs(1)
    }) {
        info!(
            elapsed_seconds = now.as_secs_f64(),
            foreground = conditions.foreground,
            active = conditions.active,
            assets_ready = conditions.assets_ready,
            pipelines_ready = conditions.pipelines_ready,
            "Graphics health conditions"
        );
        *diagnostic = Some((conditions, now));
    }
    let loaded = conditions.assets_ready && conditions.pipelines_ready;
    let reset = health.observe(Observation {
        now,
        foreground: environment.foreground,
        active: environment.active,
        loaded,
        completed: progress.completed(),
    });
    if reset {
        // Expire callbacks and take the next baseline together: a callback that races
        // the boundary must not be credited to both adjacent sampling intervals.
        let completed = progress.next_epoch();
        if let Some(window) = &mut health.window {
            window.completed = completed;
        }
    }
    if let Some((updates, completions)) = health.last_rates.take() {
        info!(
            "Graphics health: {updates:.1} main updates/s; {completions:.1} completed GPU submissions/s; checking={}",
            health.is_checking()
        );
    }
    if was_warming && health.warmup_elapsed >= WARMUP {
        info!(
            elapsed_seconds = now.as_secs_f64(),
            "Graphics health: scene warmup complete; sampling foreground rendering progress."
        );
    }
    if health.just_became_ready() {
        info!(
            elapsed_seconds = now.as_secs_f64(),
            "Graphics health admitted play after three healthy foreground samples."
        );
    }
    if !was_failed && matches!(health.phase, Phase::Failed(_)) {
        error!(
            elapsed_seconds = now.as_secs_f64(),
            "Graphics health refused play: {}; assets_and_pipelines_ready={loaded}; startup={was_checking}",
            health.status()
        );
        eprintln!("{}", health.status());
        exit.write(AppExit::error());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct HealthConditions {
    foreground: bool,
    active: bool,
    assets_ready: bool,
    pipelines_ready: bool,
}

fn refresh_watchdog(
    health: Res<RenderHealth>,
    environment: Res<HealthEnvironment>,
    settings: Option<Res<WinitSettings>>,
    progress: Res<GpuProgress>,
) {
    // Current input can change cadence after PreUpdate. Arm for that decision
    // before entering render extraction, which may wait for a blocked renderer.
    let active =
        settings.is_none_or(|settings| matches!(settings.focused_mode, UpdateMode::Continuous));
    progress.0.heartbeat_ms.store(
        progress.0.origin.elapsed().as_millis() as u64,
        Ordering::Release,
    );
    progress.0.armed.store(
        environment.foreground && active && !matches!(health.phase, Phase::Failed(_)),
        Ordering::Release,
    );
}

fn keep_checking_active(
    health: Res<RenderHealth>,
    environment: Res<HealthEnvironment>,
    redraw: Option<ResMut<Redraw>>,
) {
    if health.is_checking()
        && environment.foreground
        && let Some(mut redraw) = redraw
    {
        redraw.request();
    }
}

#[derive(Component)]
struct CheckingOverlay;
#[derive(Component)]
struct CheckingLabel;

fn show_checking_overlay(mut commands: Commands) {
    info!("Graphics health: checking real scene rendering before enabling gameplay.");
    commands
        .spawn((
            CheckingOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(10_000),
            BackgroundColor(Color::srgba(0.02, 0.03, 0.03, 0.65)),
            Pickable::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                CheckingLabel,
                Text::new("Checking graphics..."),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.89, 0.72)),
                Pickable::IGNORE,
            ));
        });
}

fn update_checking_overlay(
    health: Res<RenderHealth>,
    mut overlays: Query<&mut Node, With<CheckingOverlay>>,
    mut labels: Query<&mut Text, With<CheckingLabel>>,
) {
    for mut node in &mut overlays {
        node.display = if health.allows_play() {
            Display::None
        } else {
            Display::Flex
        };
    }
    for mut label in &mut labels {
        label.0 = health.status().to_owned();
    }
}

#[derive(Default)]
struct CompletionEpoch {
    generation: u64,
    total: u64,
}

struct SharedProgress {
    origin: Instant,
    completions: Mutex<CompletionEpoch>,
    pipelines_pending: AtomicBool,
    heartbeat_ms: AtomicU64,
    armed: AtomicBool,
    watchdog_state: AtomicU8,
}

#[derive(Resource, Clone)]
struct GpuProgress(Arc<SharedProgress>);

impl Default for GpuProgress {
    fn default() -> Self {
        Self(Arc::new(SharedProgress {
            origin: Instant::now(),
            completions: Mutex::default(),
            pipelines_pending: AtomicBool::new(true),
            heartbeat_ms: AtomicU64::new(0),
            armed: AtomicBool::new(false),
            watchdog_state: AtomicU8::new(WATCHING),
        }))
    }
}

impl GpuProgress {
    fn ticket(&self) -> u64 {
        self.0.completions.lock().unwrap().generation
    }
    fn completed(&self) -> u64 {
        self.0.completions.lock().unwrap().total
    }
    fn next_epoch(&self) -> u64 {
        let mut completions = self.0.completions.lock().unwrap();
        completions.generation += 1;
        completions.total
    }
    fn complete(&self, ticket: u64) {
        let mut completions = self.0.completions.lock().unwrap();
        if ticket == completions.generation {
            completions.total += 1;
        }
    }

    fn callback_for(&self, ticket: u64) -> impl FnOnce() + Send + 'static {
        let progress = self.clone();
        move || progress.complete(ticket)
    }

    #[cfg(test)]
    fn callback(&self) -> impl FnOnce() + Send + 'static {
        self.callback_for(self.ticket())
    }

    fn unresponsive(&self, now_ms: u64) -> bool {
        self.0.watchdog_state.load(Ordering::Acquire) == WATCHING
            && self.0.armed.load(Ordering::Acquire)
            && now_ms.saturating_sub(self.0.heartbeat_ms.load(Ordering::Acquire)) >= 15_000
    }

    fn claim_timeout(&self, now_ms: u64) -> bool {
        self.unresponsive(now_ms)
            && self
                .0
                .watchdog_state
                .compare_exchange(WATCHING, EXPIRED, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }

    fn cancel_watchdog(&self) -> bool {
        self.0
            .watchdog_state
            .compare_exchange(WATCHING, CANCELLED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

#[derive(Resource, Default)]
struct SurfaceFrame(Option<u64>);

fn observe_surface(
    windows: Query<&ExtractedWindow>,
    pipelines: Res<PipelineCache>,
    progress: Res<GpuProgress>,
    mut frame: ResMut<SurfaceFrame>,
) {
    frame.0 = windows
        .iter()
        .any(|window| window.swap_chain_texture.is_some())
        .then(|| progress.ticket());
    progress.0.pipelines_pending.store(
        pipelines.waiting_pipelines().next().is_some()
            || pipelines
                .pipelines()
                .any(|pipeline| !matches!(pipeline.state, CachedPipelineState::Ok(_))),
        Ordering::Release,
    );
}

fn acknowledge_gpu_work(
    queue: Res<RenderQueue>,
    progress: Res<GpuProgress>,
    mut frame: ResMut<SurfaceFrame>,
) {
    let Some(ticket) = frame.0.take() else {
        return;
    };
    // This acknowledges completed GPU work for an acquired surface. It is not a display timestamp.
    queue.on_submitted_work_done(progress.callback_for(ticket));
}

#[derive(Resource)]
struct HealthWatchdog {
    progress: GpuProgress,
    thread: Option<JoinHandle<()>>,
}

impl HealthWatchdog {
    fn start(progress: GpuProgress) -> Self {
        let watch = progress.clone();
        let thread = std::thread::Builder::new().name("tabletop-render-watchdog".into()).spawn(move || {
            while watch.0.watchdog_state.load(Ordering::Acquire) == WATCHING {
                std::thread::sleep(Duration::from_millis(100));
                let now = watch.0.origin.elapsed().as_millis() as u64;
                if watch.claim_timeout(now) {
                    // A blocked renderer cannot show a new UI. Last-known visibility is all that
                    // remains observable here; changes during the stall cannot be reconstructed.
                    eprintln!("Rendering stopped progressing for 15 seconds during active foreground operation. Exiting; the last saved decision is unchanged. Restart the app to retry.");
                    std::process::exit(2);
                }
            }
        }).expect("could not start the rendering watchdog");
        Self {
            progress,
            thread: Some(thread),
        }
    }
}

impl Drop for HealthWatchdog {
    fn drop(&mut self) {
        // Cancellation and expiry have one terminal decision. A cancellation that
        // wins this claim cannot later become an exit from a previously read timeout.
        self.progress.cancel_watchdog();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) fn exit_result(exit: AppExit) -> Result<(), String> {
    if exit.is_success() {
        Ok(())
    } else {
        Err(format!(
            "The tabletop exited unsuccessfully ({exit:?}); see the diagnostics above."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_health_diagnostics_explain_waiting_without_repeating_stable_frames() {
        use bevy::log::tracing_subscriber;

        const CHILD: &str = "ARKHAM_HEALTH_DIAGNOSTICS_TEST_CHILD";
        const VERIFIED: &str = "health diagnostic assertions completed";
        if std::env::var_os(CHILD).is_none() {
            // A process-global subscriber observes the production scheduler's
            // worker threads without sharing tracing state with other tests.
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "tabletop_health::tests::registered_health_diagnostics_explain_waiting_without_repeating_stable_frames",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains(VERIFIED),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let log = tempfile::NamedTempFile::new().unwrap();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_target(false)
            .with_writer(Arc::new(log.reopen().unwrap()))
            .finish();
        bevy::log::tracing::subscriber::set_global_default(subscriber).unwrap();
        {
            let (mut app, window) = app_with_window();
            update_at(&mut app, Duration::ZERO, false);
            for frame in 1..60 {
                update_at(&mut app, sample(frame).now, false);
            }
            app.world()
                .resource::<GpuProgress>()
                .0
                .pipelines_pending
                .store(false, Ordering::Release);
            update_at(&mut app, Duration::from_secs(1), false);
            app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
            update_at(&mut app, Duration::from_millis(1_100), false);
            // A change inside the rate limit is retained until the next report.
            update_at(&mut app, Duration::from_secs(2), false);
            app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
            update_at(&mut app, Duration::from_secs(3), false);
            app.world_mut().resource_mut::<WinitSettings>().focused_mode =
                UpdateMode::reactive_low_power(Duration::from_secs(1));
            update_at(&mut app, Duration::from_secs(4), false);
            update_at(&mut app, Duration::from_secs(5), false);
            update_at(&mut app, Duration::from_secs(6), false);
            assert!(app.world().resource::<RenderHealth>().is_checking());
        }
        let contents = std::fs::read_to_string(log.path()).unwrap();
        let changes: Vec<_> = contents
            .lines()
            .filter(|line| line.contains("Graphics health conditions"))
            .collect();
        assert_eq!(changes.len(), 5, "{contents}");
        for (line, expected) in changes.iter().zip([
            "elapsed_seconds=0.0 foreground=true active=true assets_ready=true pipelines_ready=false",
            "elapsed_seconds=1.0 foreground=true active=true assets_ready=true pipelines_ready=true",
            "elapsed_seconds=2.0 foreground=false active=true assets_ready=true pipelines_ready=true",
            "elapsed_seconds=3.0 foreground=true active=true assets_ready=true pipelines_ready=true",
            "elapsed_seconds=4.0 foreground=true active=false assets_ready=true pipelines_ready=true",
        ]) {
            assert!(line.contains(expected), "{line}");
        }
        println!("{VERIFIED}");
    }

    fn sample(frame: u64) -> Observation {
        Observation {
            now: Duration::from_secs_f64(frame as f64 / 60.0),
            foreground: true,
            active: true,
            loaded: true,
            completed: frame,
        }
    }

    fn app_with_window() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TabletopHealthPlugin));
        app.insert_resource(WinitSettings::continuous());
        let window = app.world_mut().spawn(Window::default()).id();
        (app, window)
    }

    fn update_at(app: &mut App, now: Duration, gpu_completed: bool) {
        app.world_mut().resource_mut::<HealthClock>().injected = Some(now);
        if gpu_completed {
            app.world().resource::<GpuProgress>().callback()();
        }
        app.update();
    }

    #[test]
    fn real_progress_is_required_before_play_is_admitted() {
        let mut health = RenderHealth::default();
        assert!(!health.allows_play());
        for frame in 0..=360 {
            health.observe(sample(frame));
        }
        assert!(health.allows_play());
    }

    #[test]
    fn fast_main_updates_without_gpu_completion_never_admit_play() {
        let mut health = RenderHealth::default();
        for frame in 0..=1800 {
            health.observe(Observation {
                completed: 0,
                ..sample(frame)
            });
        }
        assert!(!health.allows_play());
        assert!(!health.is_checking());
        assert!(health.status().contains("30 seconds"));
    }

    #[test]
    fn startup_can_recover_from_slow_samples_before_its_deadline() {
        let mut health = RenderHealth::default();
        for frame in 0..=600 {
            health.observe(Observation {
                completed: frame / 6,
                ..sample(frame)
            });
        }
        assert!(health.is_checking());
        for frame in 601..=900 {
            health.observe(Observation {
                completed: 100 + frame - 600,
                ..sample(frame)
            });
        }
        assert!(health.allows_play());
    }

    #[test]
    fn old_completion_bursts_do_not_replace_three_healthy_intervals() {
        let mut health = RenderHealth::default();
        for frame in 0..=600 {
            let completed = if frame < 180 { 0 } else { 600 };
            health.observe(Observation {
                completed,
                ..sample(frame)
            });
        }
        assert!(health.is_checking());
        assert!(!health.allows_play());
    }

    #[test]
    fn active_slow_rendering_after_admission_stops_play() {
        let mut health = RenderHealth::default();
        for frame in 0..=360 {
            health.observe(sample(frame));
        }
        assert!(health.allows_play());
        for frame in 361..=660 {
            health.observe(Observation {
                completed: 360,
                ..sample(frame)
            });
        }
        assert!(!health.allows_play());
        assert!(health.status().contains("Gameplay has stopped"));
    }

    #[test]
    fn a_long_active_gap_is_not_treated_as_a_single_short_bad_frame() {
        let mut health = RenderHealth::default();
        for frame in 0..=360 {
            health.observe(sample(frame));
        }
        health.observe(Observation {
            completed: 360,
            ..sample(660)
        });
        assert!(!health.allows_play());
    }

    #[test]
    fn loading_focus_and_idle_intervals_do_not_count_as_slow_rendering() {
        for excluded in [
            Observation {
                foreground: false,
                ..sample(361)
            },
            Observation {
                active: false,
                ..sample(361)
            },
            Observation {
                loaded: false,
                ..sample(361)
            },
        ] {
            let mut health = RenderHealth::default();
            for frame in 0..=360 {
                health.observe(sample(frame));
            }
            health.observe(excluded);
            // Both edges of the interval matter: resuming does not inherit 60 seconds idle.
            health.observe(Observation {
                now: Duration::from_secs(66),
                completed: 361,
                ..sample(362)
            });
            assert!(health.allows_play());
        }
    }

    #[test]
    fn foreground_loading_has_a_finite_startup_budget() {
        let mut health = RenderHealth::default();
        for frame in 0..=1800 {
            health.observe(Observation {
                loaded: false,
                ..sample(frame)
            });
        }
        assert!(!health.is_checking());
        assert!(!health.allows_play());
    }

    #[test]
    fn background_time_does_not_spend_the_startup_budget() {
        let mut health = RenderHealth::default();
        health.observe(Observation {
            foreground: false,
            ..sample(0)
        });
        health.observe(Observation {
            foreground: false,
            ..sample(36000)
        });
        for frame in 36001..=36361 {
            health.observe(sample(frame));
        }
        assert!(health.allows_play());
    }

    #[test]
    fn gpu_callbacks_expire_without_resetting_the_cumulative_count() {
        let progress = GpuProgress::default();
        progress.callback()();
        let delayed = progress.callback();
        assert_eq!(progress.next_epoch(), 1);
        delayed();
        assert_eq!(progress.completed(), 1);
        progress.callback()();
        assert_eq!(progress.completed(), 2);
        let prior_frame = progress.ticket();
        assert_eq!(progress.next_epoch(), 2);
        // Even if registration itself was delayed, the surface's original epoch wins.
        progress.callback_for(prior_frame)();
        assert_eq!(progress.completed(), 2);
    }

    #[test]
    fn registered_plugin_admits_actual_callback_progress_and_emits_one_transition_pulse() {
        let (mut app, _) = app_with_window();
        app.world()
            .resource::<GpuProgress>()
            .0
            .pipelines_pending
            .store(false, Ordering::Release);
        let mut admitted = false;
        for frame in 0..=360 {
            update_at(&mut app, sample(frame).now, true);
            if app.world().resource::<RenderHealth>().allows_play() {
                assert!(app.world().resource::<RenderHealth>().just_became_ready());
                update_at(&mut app, sample(frame + 1).now, true);
                assert!(!app.world().resource::<RenderHealth>().just_became_ready());
                admitted = true;
                break;
            }
        }
        assert!(admitted);
        let world = app.world_mut();
        assert!(
            world
                .query_filtered::<&Node, With<CheckingOverlay>>()
                .iter(world)
                .all(|node| node.display == Display::None)
        );
    }

    #[test]
    fn registered_plugin_pauses_for_occlusion_and_resumes_without_inheriting_the_gap() {
        let (mut app, window) = app_with_window();
        app.world()
            .resource::<GpuProgress>()
            .0
            .pipelines_pending
            .store(false, Ordering::Release);
        update_at(&mut app, Duration::ZERO, true);
        app.world_mut().write_message(WindowOccluded {
            window,
            occluded: true,
        });
        update_at(&mut app, Duration::from_secs(1), true);
        update_at(&mut app, Duration::from_secs(90), true);
        assert!(app.world().resource::<RenderHealth>().is_checking());
        assert!(
            !app.world()
                .resource::<GpuProgress>()
                .0
                .armed
                .load(Ordering::Acquire)
        );
        app.world_mut().write_message(WindowOccluded {
            window,
            occluded: false,
        });
        for frame in 0..=360 {
            update_at(&mut app, Duration::from_secs(90) + sample(frame).now, true);
        }
        assert!(app.world().resource::<RenderHealth>().allows_play());
    }

    #[test]
    fn watchdog_only_expires_active_work_and_is_cancelled_when_its_owner_drops() {
        let progress = GpuProgress::default();
        assert!(!progress.unresponsive(100_000));
        progress.0.armed.store(true, Ordering::Release);
        progress.0.heartbeat_ms.store(1_000, Ordering::Release);
        assert!(!progress.unresponsive(15_999));
        assert!(progress.unresponsive(16_000));
        let watchdog = HealthWatchdog::start(progress.clone());
        drop(watchdog);
        assert!(!progress.unresponsive(100_000));
    }

    #[test]
    fn watchdog_cancellation_and_expiry_have_only_one_winner() {
        let cancelled = GpuProgress::default();
        cancelled.0.armed.store(true, Ordering::Release);
        assert!(cancelled.unresponsive(15_000));
        assert!(cancelled.cancel_watchdog());
        assert!(!cancelled.claim_timeout(15_000));

        let expired = GpuProgress::default();
        expired.0.armed.store(true, Ordering::Release);
        assert!(expired.claim_timeout(15_000));
        assert!(!expired.cancel_watchdog());
        assert!(!expired.claim_timeout(15_000));
    }

    #[test]
    fn registered_pacing_arms_and_disarms_the_watchdog_in_the_same_update() {
        use crate::tabletop_pacing::TabletopPacingPlugin;
        use bevy::window::WindowEvent;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<WindowEvent>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_plugins((TabletopPacingPlugin, TabletopHealthPlugin))
            .insert_resource(RenderHealth::ready_for_test());
        app.world_mut().spawn(Window::default());
        app.update();
        let progress = app.world().resource::<GpuProgress>().clone();
        assert!(progress.0.armed.load(Ordering::Acquire));

        // Use production timer expiry, so neither the pacing decision nor its
        // registration/order is replaced by a test-only scheduling substitute.
        std::thread::sleep(Duration::from_millis(2_100));
        app.update();
        assert!(!matches!(
            app.world().resource::<WinitSettings>().focused_mode,
            UpdateMode::Continuous
        ));
        assert!(!progress.0.armed.load(Ordering::Acquire));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        assert!(matches!(
            app.world().resource::<WinitSettings>().focused_mode,
            UpdateMode::Continuous
        ));
        assert!(progress.0.armed.load(Ordering::Acquire));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyW);
        std::thread::sleep(Duration::from_millis(200));
        app.update();
        assert!(!matches!(
            app.world().resource::<WinitSettings>().focused_mode,
            UpdateMode::Continuous
        ));
        assert!(!progress.0.armed.load(Ordering::Acquire));
    }

    #[test]
    fn unsuccessful_app_exit_reaches_the_command_result() {
        assert!(exit_result(AppExit::Success).is_ok());
        assert!(exit_result(AppExit::error()).is_err());
    }

    #[test]
    fn registered_plugin_refuses_a_foreground_window_without_completed_render_work() {
        let (mut app, _) = app_with_window();
        for tenth in 0..=310 {
            update_at(&mut app, Duration::from_millis(tenth * 100), false);
            if !app.world().resource::<RenderHealth>().is_checking() {
                break;
            }
        }
        assert!(!app.world().resource::<RenderHealth>().is_checking());
        assert!(!app.world().resource::<RenderHealth>().allows_play());
        assert!(!app.world().resource::<Messages<AppExit>>().is_empty());
    }
}
