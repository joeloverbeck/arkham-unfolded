//! Decision-only frame-stage recorder for #157. It is not meant for `main`.
//!
//! With `ARKHAM_FRAME_STAGES` set and not empty, startup creates one new file in the
//! directory it names, or fails, and every frame's stage times, the table's state and
//! one GPU reading per second go to that file. Without it the agent seat runs
//! unrecorded and the windowed table refuses to start.
//! `arkham-tabletop --frame-stage-report FILE` summarises it. These are waits inside
//! the application; none is a display timestamp.

mod report;

pub use report::run as report;

use crate::{
    tabletop_feedback::{CardFeedback, FeedbackSettings},
    tabletop_pacing::PresentationTime,
};
use bevy::{
    app::{MainScheduleOrder, SubApp},
    ecs::schedule::{ScheduleLabel, SingleThreadedExecutor},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        pipelined_rendering::RenderExtractApp,
        renderer::{RenderGraph, RenderGraphSystems, render_system},
        view::window::{create_surfaces, prepare_windows},
    },
    winit::{UpdateMode, WinitSettings},
};
use std::{
    fmt,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    process::{Child, Command, Stdio},
    str::FromStr,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Sender},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DIRECTORY: &str = "ARKHAM_FRAME_STAGES";

/// The table had settled when this update ended.
pub(crate) const IDLE: u8 = 1;
pub(crate) const FOCUSED: u8 = 1 << 1;
/// At least one component showed the gameplay availability cue.
pub(crate) const CUE: u8 = 1 << 2;
pub(crate) const REDUCED_MOTION: u8 = 1 << 3;
pub(crate) const KEY_DOWN: u8 = 1 << 4;
pub(crate) const BUTTON_DOWN: u8 = 1 << 5;
/// GPU readings were being taken.
pub(crate) const SAMPLING: u8 = 1 << 7;

/// One main-app update. Times are nanoseconds; instants count from the recorder's start.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct MainFrame {
    pub frame: u64,
    pub start: u64,
    /// The time step this update fed to `PresentationTime`.
    pub step: u64,
    pub first: u64,
    /// `PreUpdate` through the fixed main loop.
    pub pre: u64,
    pub update: u64,
    /// Scene spawning and `PostUpdate`.
    pub post: u64,
    pub last: u64,
    /// From the end of the update until the render thread handed its world back.
    pub wait: u64,
    pub extract: u64,
    pub end: u64,
    pub flags: u8,
    /// How far the 3D camera moved since the previous update, in world units.
    pub camera_step: f32,
}

impl MainFrame {
    pub(crate) fn main_update(&self) -> u64 {
        self.first + self.pre + self.update + self.post + self.last
    }

    pub(crate) fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }
}

/// The render thread's work on one frame, in nanoseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RenderFrame {
    pub frame: u64,
    pub start: u64,
    /// `prepare_windows`, which obtains the next surface image.
    pub acquire: u64,
    /// The render graph: encoding and submitting commands.
    pub graph: u64,
    /// From the end of the render graph to the end of `render_system`, which presents.
    pub present: u64,
    pub total: u64,
}

/// `nvidia-smi`'s utilisation (%), power draw (W), temperature (°C) and fan speed (%).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct GpuReading(pub [Option<f32>; 4]);

impl GpuReading {
    /// Four numbers, each of which may instead be empty or bracketed, as in `[N/A]`.
    /// A header line or values with units are not a reading.
    pub(crate) fn parse(line: &str) -> Option<Self> {
        let mut values = [None; 4];
        let mut fields = line.split(',');
        for value in &mut values {
            let field = fields.next()?.trim();
            *value = match field.parse() {
                Ok(number) => Some(number),
                Err(_) if field.is_empty() || field.starts_with('[') => None,
                Err(_) => return None,
            };
        }
        fields.next().is_none().then_some(Self(values))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Row {
    Start {
        unix_ms: u64,
    },
    Main(MainFrame),
    Render(RenderFrame),
    Gpu {
        at: u64,
        reading: GpuReading,
    },
    Sampling {
        at: u64,
        on: bool,
    },
    Window {
        at: u64,
        width: u32,
        height: u32,
        scale: f32,
    },
}

impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start { unix_ms } => write!(f, "H,{unix_ms}"),
            Self::Main(m) => write!(
                f,
                "M,{},{},{},{},{},{},{},{},{},{},{},{},{}",
                m.frame,
                m.start,
                m.step,
                m.first,
                m.pre,
                m.update,
                m.post,
                m.last,
                m.wait,
                m.extract,
                m.end,
                m.flags,
                m.camera_step
            ),
            Self::Render(r) => write!(
                f,
                "R,{},{},{},{},{},{}",
                r.frame, r.start, r.acquire, r.graph, r.present, r.total
            ),
            Self::Gpu { at, reading } => {
                write!(f, "G,{at}")?;
                for value in reading.0 {
                    match value {
                        Some(value) => write!(f, ",{value}")?,
                        None => f.write_str(",")?,
                    }
                }
                Ok(())
            }
            Self::Sampling { at, on } => write!(f, "S,{at},{}", u8::from(*on)),
            Self::Window {
                at,
                width,
                height,
                scale,
            } => write!(f, "W,{at},{width},{height},{scale}"),
        }
    }
}

impl FromStr for Row {
    type Err = String;

    fn from_str(line: &str) -> Result<Self, String> {
        let fields: Vec<&str> = line.split(',').collect();
        fn number<T: FromStr>(fields: &[&str], index: usize) -> Result<T, String> {
            fields
                .get(index)
                .and_then(|field| field.parse().ok())
                .ok_or_else(|| format!("field {index} of `{}`", fields.join(",")))
        }
        let expect = |count: usize| {
            (fields.len() == count)
                .then_some(())
                .ok_or_else(|| format!("expected {count} fields in `{line}`"))
        };
        match fields[0] {
            "H" => {
                expect(2)?;
                Ok(Self::Start {
                    unix_ms: number(&fields, 1)?,
                })
            }
            "M" => {
                expect(14)?;
                Ok(Self::Main(MainFrame {
                    frame: number(&fields, 1)?,
                    start: number(&fields, 2)?,
                    step: number(&fields, 3)?,
                    first: number(&fields, 4)?,
                    pre: number(&fields, 5)?,
                    update: number(&fields, 6)?,
                    post: number(&fields, 7)?,
                    last: number(&fields, 8)?,
                    wait: number(&fields, 9)?,
                    extract: number(&fields, 10)?,
                    end: number(&fields, 11)?,
                    flags: number(&fields, 12)?,
                    camera_step: number(&fields, 13)?,
                }))
            }
            "R" => {
                expect(7)?;
                Ok(Self::Render(RenderFrame {
                    frame: number(&fields, 1)?,
                    start: number(&fields, 2)?,
                    acquire: number(&fields, 3)?,
                    graph: number(&fields, 4)?,
                    present: number(&fields, 5)?,
                    total: number(&fields, 6)?,
                }))
            }
            "G" => {
                expect(6)?;
                Ok(Self::Gpu {
                    at: number(&fields, 1)?,
                    reading: GpuReading::parse(&fields[2..].join(","))
                        .ok_or_else(|| format!("GPU reading in `{line}`"))?,
                })
            }
            "S" => {
                expect(3)?;
                Ok(Self::Sampling {
                    at: number(&fields, 1)?,
                    on: number::<u8>(&fields, 2)? != 0,
                })
            }
            "W" => {
                expect(5)?;
                Ok(Self::Window {
                    at: number(&fields, 1)?,
                    width: number(&fields, 2)?,
                    height: number(&fields, 3)?,
                    scale: number(&fields, 4)?,
                })
            }
            kind => Err(format!("unknown row kind `{kind}`")),
        }
    }
}

struct Recorder {
    origin: Instant,
    rows: Sender<Row>,
    frame: AtomicU64,
    sampler: Mutex<Option<Child>>,
}

static RECORDER: OnceLock<Option<Recorder>> = OnceLock::new();

fn recorder() -> Option<&'static Recorder> {
    RECORDER
        .get_or_init(|| {
            let directory = std::env::var_os(DIRECTORY).filter(|name| !name.is_empty())?;
            Some(Recorder::start(Path::new(&directory)))
        })
        .as_ref()
}

impl Recorder {
    /// A launch that cannot record must not look like one that did.
    fn start(directory: &Path) -> Self {
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64);
        let path = directory.join(format!("frames-{unix_ms}.csv"));
        let file = File::create_new(&path).unwrap_or_else(|error| {
            panic!("{DIRECTORY}: cannot create {}: {error}", path.display())
        });
        let (rows, received) = mpsc::channel::<Row>();
        std::thread::Builder::new()
            .name("frame-stage-writer".into())
            .spawn(move || {
                let mut file = BufWriter::new(file);
                let mut flushed = Instant::now();
                // The sender is never dropped, so this ends with the process: rows
                // from its last second can be lost.
                loop {
                    let mut written = match received.recv_timeout(Duration::from_millis(500)) {
                        Ok(row) => writeln!(file, "{row}"),
                        Err(_) => Ok(()),
                    };
                    if written.is_ok() && flushed.elapsed() >= Duration::from_secs(1) {
                        written = file.flush();
                        flushed = Instant::now();
                    }
                    if let Err(error) = written {
                        error!("Frame-stage recording stopped: {error}");
                        break;
                    }
                }
            })
            .expect("frame-stage writer thread");
        info!("Recording frame stages to {}", path.display());
        let recorder = Self {
            origin: Instant::now(),
            rows,
            frame: AtomicU64::new(0),
            sampler: Mutex::new(None),
        };
        recorder.send(Row::Start { unix_ms });
        recorder
    }

    fn since_start(&self, instant: Instant) -> u64 {
        instant.saturating_duration_since(self.origin).as_nanos() as u64
    }

    fn send(&self, row: Row) {
        // Fails only after the writer reported a failed write and stopped.
        let _ = self.rows.send(row);
    }

    fn sampling(&self) -> bool {
        self.sampler.lock().unwrap().is_some()
    }

    fn mark_sampling(&self, on: bool) {
        self.send(Row::Sampling {
            at: self.since_start(Instant::now()),
            on,
        });
        info!("GPU readings {}", if on { "on" } else { "off" });
    }

    /// Starts the one-second GPU readings, once. The launch needs them, so it does
    /// not start without them.
    fn start_sampling(&'static self) {
        let mut sampler = self.sampler.lock().unwrap();
        if sampler.is_some() {
            return;
        }
        let mut child = Command::new("nvidia-smi")
            .args([
                "--query-gpu=utilization.gpu,power.draw,temperature.gpu,fan.speed",
                "--format=csv,noheader,nounits",
                "-l",
                "1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|error| panic!("{DIRECTORY}: nvidia-smi did not start: {error}"));
        let output = child.stdout.take().expect("piped nvidia-smi output");
        *sampler = Some(child);
        self.mark_sampling(true);
        // Ends when the child does. If this process dies first, the child's next
        // write fails and ends it.
        std::thread::Builder::new()
            .name("frame-stage-gpu".into())
            .spawn(move || {
                for line in BufReader::new(output).lines().map_while(Result::ok) {
                    if let Some(reading) = GpuReading::parse(&line) {
                        self.send(Row::Gpu {
                            at: self.since_start(Instant::now()),
                            reading,
                        });
                    }
                }
                // Still held here means nobody stopped it: it ended by itself.
                if let Some(mut child) = self.sampler.lock().unwrap().take() {
                    let _ = child.wait();
                    self.mark_sampling(false);
                    warn!("nvidia-smi ended; no further GPU readings");
                }
            })
            .expect("frame-stage GPU thread");
    }

    /// Stops the readings for the rest of the launch. A reading can itself disturb
    /// frames, so the route ends with a stretch without them.
    fn stop_sampling(&self) {
        if let Some(mut child) = self.sampler.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
            self.mark_sampling(false);
        }
    }
}

/// Six boundaries between the main schedules, each stamped by a schedule of its own
/// so the stamp cannot be reordered among other systems.
#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Boundary(u8);

#[derive(Resource)]
struct MainMarks {
    frame: u64,
    /// The six boundaries, then the end of this module's own bookkeeping.
    stamps: [Instant; 7],
    step: Duration,
    flags: u8,
    camera: Option<Vec3>,
    camera_step: f32,
    received: Option<Instant>,
    window: Option<(u32, u32, f32)>,
}

#[derive(Resource)]
struct RenderMarks {
    frame: u64,
    start: Instant,
    acquire: [Instant; 2],
    graph: [Instant; 2],
    presented: Instant,
}

pub(crate) struct FrameStagesPlugin {
    /// The windowed table of this diagnostic build exists only to be recorded, so
    /// it refuses to start without the variable. The agent seat records only when asked.
    pub required: bool,
}

impl Plugin for FrameStagesPlugin {
    fn build(&self, app: &mut App) {
        let Some(recorder) = recorder() else {
            assert!(
                !self.required,
                "this diagnostic build records frame stages: set {DIRECTORY} to a directory"
            );
            return;
        };
        let now = Instant::now();
        app.insert_resource(MainMarks {
            frame: 0,
            stamps: [now; 7],
            step: Duration::ZERO,
            flags: 0,
            camera: None,
            camera_step: 0.0,
            received: None,
            window: None,
        })
        .add_systems(Boundary(0), begin_frame)
        .add_systems(Boundary(1), (stamp::<1>, read_step).chain())
        .add_systems(Boundary(2), stamp::<2>)
        .add_systems(Boundary(3), stamp::<3>)
        .add_systems(Boundary(4), stamp::<4>)
        .add_systems(
            Boundary(5),
            (stamp::<5>, stop_gpu_readings, observe, stamp::<6>).chain(),
        );
        for boundary in 0..6 {
            // The default executor would hand each stamp to the task pool.
            app.edit_schedule(Boundary(boundary), |schedule| {
                schedule.set_executor(SingleThreadedExecutor::new());
            });
        }
        let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
        order.insert_before(First, Boundary(0));
        order.insert_after(First, Boundary(1));
        order.insert_before(Update, Boundary(2));
        order.insert_after(Update, Boundary(3));
        order.insert_before(Last, Boundary(4));
        order.insert_after(Last, Boundary(5));

        // A record without the render thread would look complete and not be.
        let render = app
            .get_sub_app_mut(RenderApp)
            .expect("frame stages are recorded with the render app");
        render
            .insert_resource(RenderMarks {
                frame: 0,
                start: now,
                acquire: [now; 2],
                graph: [now; 2],
                presented: now,
            })
            .add_systems(
                Render,
                (
                    render_start.before(RenderSystems::ExtractCommands),
                    acquire_start
                        .in_set(RenderSystems::PrepareViews)
                        .after(create_surfaces)
                        .before(prepare_windows),
                    acquire_end
                        .in_set(RenderSystems::PrepareViews)
                        .after(prepare_windows),
                    presented.in_set(RenderSystems::Render).after(render_system),
                    render_end.after(RenderSystems::PostCleanup),
                ),
            )
            .add_systems(
                RenderGraph,
                (
                    graph_start.before(RenderGraphSystems::Begin),
                    graph_end.after(RenderGraphSystems::Finish),
                ),
            );
        // The render world arrives here once the render thread has finished with it.
        wrap_extract(render, receive_render_world, |_, _| {});
        // Pipelined rendering waits for the render thread, then extracts, inside
        // this sub-app's extraction.
        let extract = app
            .get_sub_app_mut(RenderExtractApp)
            .expect("frame stages are recorded with pipelined rendering");
        wrap_extract(extract, |_, _| {}, |main, _| finish_frame(main));
        // The agent seat's tests open the table more than once in one process; a later
        // table keeps the readings the first one started.
        recorder.start_sampling();
    }
}

/// Runs `before` and `after` around the sub-app's own extraction of the main world.
fn wrap_extract(
    app: &mut SubApp,
    before: fn(&mut World, &mut World),
    after: fn(&mut World, &mut World),
) {
    let mut extract = app.take_extract();
    app.set_extract(move |main, sub| {
        before(main, sub);
        if let Some(extract) = extract.as_mut() {
            extract(main, sub);
        }
        after(main, sub);
    });
}

fn receive_render_world(main: &mut World, render: &mut World) {
    if let Some(mut marks) = main.get_resource_mut::<MainMarks>() {
        marks.received = Some(Instant::now());
        let frame = marks.frame;
        render.resource_mut::<RenderMarks>().frame = frame;
    }
}

fn finish_frame(main: &mut World) {
    let (Some(recorder), Some(marks)) = (recorder(), main.get_resource::<MainMarks>()) else {
        return;
    };
    let end = Instant::now();
    let received = marks.received.unwrap_or(end);
    let span = |from: usize, to: usize| {
        marks.stamps[to]
            .saturating_duration_since(marks.stamps[from])
            .as_nanos() as u64
    };
    recorder.send(Row::Main(MainFrame {
        frame: marks.frame,
        start: recorder.since_start(marks.stamps[0]),
        step: marks.step.as_nanos() as u64,
        first: span(0, 1),
        pre: span(1, 2),
        update: span(2, 3),
        post: span(3, 4),
        last: span(4, 5),
        wait: received
            .saturating_duration_since(marks.stamps[6])
            .as_nanos() as u64,
        extract: end.saturating_duration_since(received).as_nanos() as u64,
        end: recorder.since_start(end),
        flags: marks.flags,
        camera_step: marks.camera_step,
    }));
}

fn begin_frame(mut marks: ResMut<MainMarks>) {
    if let Some(recorder) = recorder() {
        marks.frame = recorder.frame.fetch_add(1, Ordering::Relaxed) + 1;
    }
    marks.received = None;
    marks.stamps[0] = Instant::now();
}

fn stamp<const BOUNDARY: usize>(mut marks: ResMut<MainMarks>) {
    marks.stamps[BOUNDARY] = Instant::now();
}

fn read_step(mut marks: ResMut<MainMarks>, time: Option<Res<PresentationTime>>) {
    marks.step = time.map_or(Duration::ZERO, |time| time.delta());
}

/// F9 stops the GPU readings; pressing it again does nothing.
fn stop_gpu_readings(keys: Option<Res<ButtonInput<KeyCode>>>) {
    if let Some(recorder) = recorder()
        && keys.is_some_and(|keys| keys.just_pressed(KeyCode::F9))
    {
        recorder.stop_sampling();
    }
}

#[allow(clippy::too_many_arguments)]
fn observe(
    mut marks: ResMut<MainMarks>,
    settings: Option<Res<WinitSettings>>,
    feedback: Option<Res<FeedbackSettings>>,
    windows: Query<&Window>,
    cards: Query<&CardFeedback>,
    cameras: Query<&Transform, With<Camera3d>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
) {
    let Some(recorder) = recorder() else {
        return;
    };
    let mut flags = 0;
    let mut set = |flag: u8, on: bool| flags |= if on { flag } else { 0 };
    set(
        IDLE,
        settings.is_some_and(|settings| settings.focused_mode != UpdateMode::Continuous),
    );
    set(FOCUSED, windows.iter().any(|window| window.focused));
    set(CUE, cards.iter().any(|card| card.opportunity));
    set(
        REDUCED_MOTION,
        feedback.is_some_and(|feedback| feedback.reduced_motion),
    );
    set(
        KEY_DOWN,
        keys.is_some_and(|keys| keys.get_pressed().next().is_some()),
    );
    set(
        BUTTON_DOWN,
        buttons.is_some_and(|buttons| buttons.get_pressed().next().is_some()),
    );
    set(SAMPLING, recorder.sampling());
    let camera = cameras.iter().next().map(|camera| camera.translation);
    marks.flags = flags;
    marks.camera_step = match (marks.camera, camera) {
        (Some(before), Some(now)) => before.distance(now),
        _ => 0.0,
    };
    marks.camera = camera;
    let window = windows.iter().next().map(|window| {
        (
            window.physical_width(),
            window.physical_height(),
            window.scale_factor(),
        )
    });
    if window != marks.window {
        marks.window = window;
        if let Some((width, height, scale)) = window {
            recorder.send(Row::Window {
                at: recorder.since_start(Instant::now()),
                width,
                height,
                scale,
            });
        }
    }
}

fn render_start(mut marks: ResMut<RenderMarks>) {
    marks.start = Instant::now();
}

fn acquire_start(mut marks: ResMut<RenderMarks>) {
    marks.acquire[0] = Instant::now();
}

fn acquire_end(mut marks: ResMut<RenderMarks>) {
    marks.acquire[1] = Instant::now();
}

fn graph_start(mut marks: ResMut<RenderMarks>) {
    marks.graph[0] = Instant::now();
}

fn graph_end(mut marks: ResMut<RenderMarks>) {
    marks.graph[1] = Instant::now();
}

fn presented(mut marks: ResMut<RenderMarks>) {
    marks.presented = Instant::now();
}

fn render_end(marks: Res<RenderMarks>) {
    let Some(recorder) = recorder() else {
        return;
    };
    let span = |from: Instant, to: Instant| to.saturating_duration_since(from).as_nanos() as u64;
    recorder.send(Row::Render(RenderFrame {
        frame: marks.frame,
        start: recorder.since_start(marks.start),
        acquire: span(marks.acquire[0], marks.acquire[1]),
        graph: span(marks.graph[0], marks.graph[1]),
        present: span(marks.graph[1], marks.presented),
        total: span(marks.start, Instant::now()),
    }));
}
