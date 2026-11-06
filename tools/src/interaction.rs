use crate::native::capture_path;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy)]
pub struct Pointer {
    pub root: [f64; 2],
    pub client: [f64; 2],
    pub mask: u16,
}

/// The X11 boundary, independent of the application's renderer.
pub trait Desktop {
    fn focus(&mut self) -> Result<()>;
    fn verify(&mut self) -> Result<()>;
    fn geometry(&mut self) -> Result<Geometry>;
    fn pointer(&mut self) -> Result<Pointer>;
    fn capture(&mut self, path: &Path) -> Result<Value>;
}

/// The desktop portal boundary; the caller always closes even a partial start.
pub trait Portal {
    fn start(&mut self, emit: &mut dyn FnMut(Value) -> Result<()>) -> Result<()>;
    fn size(&self) -> [f64; 2];
    fn move_to(&mut self, point: [f64; 2]) -> Result<()>;
    fn button(&mut self, pressed: bool, button: i32) -> Result<()>;
    fn wheel(&mut self, steps: i32) -> Result<()>;
    fn close(&mut self) -> Result<()>;
}

pub const CLEANUP_RESERVE: Duration = Duration::from_secs(15);

/// Cancellation is observed while consent is pending as well as during input.
#[derive(Default)]
pub(crate) struct CommandInput {
    ended: AtomicBool,
    stopped: AtomicBool,
}

impl CommandInput {
    pub fn observe(&self, line: &str) {
        if matches!(
            serde_json::from_str::<Request>(line),
            Ok(Request {
                action: Action::Stop,
                ..
            })
        ) {
            self.stopped.store(true, Ordering::SeqCst);
        }
    }

    pub fn end(&self) {
        self.ended.store(true, Ordering::SeqCst);
    }

    pub fn check(&self) -> Result<()> {
        ensure!(!self.stopped.load(Ordering::SeqCst), StopRequested);
        ensure!(
            !self.ended.load(Ordering::SeqCst),
            "Command input ended or its reader stopped"
        );
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct StopRequested;
impl std::fmt::Display for StopRequested {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Session stopped by request")
    }
}
impl std::error::Error for StopRequested {}

/// One agreed end time; the injected monotonic clock also drives fault tests.
pub struct SessionTiming<'a> {
    pub deadline: Instant,
    pub command_timeout: Duration,
    pub now: &'a dyn Fn() -> Instant,
}

impl SessionTiming<'_> {
    pub fn input_deadline(&self) -> Instant {
        self.deadline
            .checked_sub(CLEANUP_RESERVE)
            .unwrap_or_else(|| (self.now)())
    }

    pub fn admit(&self) -> Result<Instant> {
        let now = (self.now)();
        ensure!(
            self.input_deadline().saturating_duration_since(now) >= self.command_timeout,
            SessionTimeExhausted
        );
        Ok(now + self.command_timeout)
    }
}

pub fn run_session(
    desktop: &mut dyn Desktop,
    portal: &mut dyn Portal,
    directory: &Path,
    commands: &mut dyn Iterator<Item = Result<String>>,
    emit: &mut dyn FnMut(Value) -> Result<()>,
    pause: &mut dyn FnMut(Duration) -> Result<()>,
    timing: SessionTiming<'_>,
) -> Result<()> {
    let result = (|| {
        let initial_deadline = timing.admit()?;
        desktop.focus()?;
        ensure!((timing.now)() < initial_deadline, CommandTimedOut);
        emit(capture(desktop, directory, "initial.png")?)?;
        ensure!((timing.now)() < initial_deadline, CommandTimedOut);
        timing.admit()?;
        portal.start(emit)?;
        let calibration_deadline = timing.admit()?;
        let mut controller = Controller {
            desktop,
            portal,
            pause,
            mapping: None,
            deadline: calibration_deadline,
            now: timing.now,
        };
        let ready = controller
            .desktop
            .focus()
            .and_then(|()| controller.calibrate());
        match ready {
            Ok(event) => emit(event)?,
            Err(error) => {
                emit(json!({"status":"refused", "reason":format!("{error:#}")}))?;
                if error.is::<CommandTimedOut>() || error.is::<StopRequested>() {
                    return Err(error);
                }
            }
        }
        for (index, line) in commands.enumerate() {
            let line = line?;
            let started = (timing.now)();
            controller.deadline = started + timing.command_timeout;
            emit(json!({"status":"command-started", "request":index + 1,
                "timeout_seconds":timing.command_timeout.as_secs_f64(),
                "session_remaining_ms":timing.deadline.saturating_duration_since(started).as_millis()}))?;
            let mut delivered_click = None;
            let event = (|| -> Result<Option<Value>> {
                let request: Request = serde_json::from_str(&line)?;
                if matches!(request.action, Action::Stop) {
                    return Ok(None);
                }
                timing.admit()?;
                controller.check_deadline()?;
                if let Action::Click {
                    capture_after: Some(after),
                    ..
                } = &request.action
                {
                    after.validate(directory)?;
                }
                controller.check_deadline()?;
                if request.focus && !matches!(request.action, Action::Focus) {
                    controller.desktop.focus()?;
                }
                controller.check_deadline()?;
                let event = match request.action {
                    Action::Stop => unreachable!(),
                    Action::Capture { name } => capture(controller.desktop, directory, &name)?,
                    Action::Focus => {
                        controller.desktop.focus()?;
                        json!({"status":"focused"})
                    }
                    Action::Calibrate => controller.calibrate()?,
                    Action::Click {
                        x,
                        y,
                        capture_after,
                    } => {
                        controller.click([x, y])?;
                        let mut event = json!({"status":"clicked", "client":[x, y]});
                        if let Some(after) = capture_after {
                            // Keep input success even when waiting or capture fails. Never retry it.
                            delivered_click = Some(event.clone());
                            controller.pause(Duration::from_millis(after.delay_ms))?;
                            event["capture"] = capture(controller.desktop, directory, &after.name)?;
                        }
                        event
                    }
                    Action::Wheel { x, y, steps } => {
                        controller.wheel([x, y], steps)?;
                        json!({"status":"wheeled", "client":[x, y], "steps":steps})
                    }
                    Action::RightDrag {
                        x,
                        y,
                        dx,
                        dy,
                        duration_seconds,
                    } => {
                        controller.right_drag([x, y], [dx, dy], duration_seconds)?;
                        json!({"status":"right-dragged", "client":[x, y], "delta":[dx, dy], "duration_seconds":duration_seconds})
                    }
                };
                controller.check_deadline()?;
                Ok(Some(event))
            })();
            let mut respond = |mut event: Value| {
                event["request"] = json!(index + 1);
                event["elapsed_ms"] = json!(
                    (timing.now)()
                        .saturating_duration_since(started)
                        .as_millis()
                );
                emit(event)
            };
            match event {
                Ok(None) => {
                    respond(json!({"status":"stopped"}))?;
                    break;
                }
                Ok(Some(event)) => respond(event)?,
                Err(error) => {
                    let mut failure = json!({"status":"refused", "reason":format!("{error:#}")});
                    if let Some(input) = delivered_click {
                        failure["input"] = input;
                        failure["capture"] =
                            json!({"status":"failed", "reason":format!("{error:#}")});
                    }
                    respond(failure)?;
                    if error.is::<ReleaseFailed>()
                        || error.is::<CommandTimedOut>()
                        || error.is::<SessionTimeExhausted>()
                        || error.is::<StopRequested>()
                    {
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    })();
    let result = match result {
        Err(error) if error.is::<StopRequested>() => emit(json!({"status":"stopped"})),
        other => other,
    };
    let closed = portal.close();
    match &closed {
        Ok(()) => emit(json!({"status":"session-closed"}))?,
        Err(error) => {
            emit(json!({"status":"session-close-failed", "reason":format!("{error:#}")}))?
        }
    }
    closed?;
    result
}

#[derive(Debug)]
struct ReleaseFailed;
impl std::fmt::Display for ReleaseFailed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Button release failed; ending the session for cleanup")
    }
}

#[derive(Debug)]
struct CommandTimedOut;
impl std::fmt::Display for CommandTimedOut {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Native command timed out; ending the session for cleanup")
    }
}
impl std::error::Error for CommandTimedOut {}

#[derive(Debug)]
struct SessionTimeExhausted;
impl std::fmt::Display for SessionTimeExhausted {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Insufficient desktop-control time for another operation and cleanup")
    }
}
impl std::error::Error for SessionTimeExhausted {}

#[derive(Deserialize)]
struct Request {
    /// Explicitly reacquire this client immediately before the guarded operation.
    #[serde(default)]
    focus: bool,
    #[serde(flatten)]
    action: Action,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
enum Action {
    Stop,
    Capture {
        name: String,
    },
    Focus,
    Calibrate,
    Click {
        x: f64,
        y: f64,
        #[serde(default)]
        capture_after: Option<CaptureAfterClick>,
    },
    Wheel {
        x: f64,
        y: f64,
        steps: i32,
    },
    RightDrag {
        x: f64,
        y: f64,
        dx: f64,
        dy: f64,
        #[serde(default = "drag_duration")]
        duration_seconds: f64,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureAfterClick {
    name: String,
    #[serde(default)]
    delay_ms: u64,
}

impl CaptureAfterClick {
    fn validate(&self, directory: &Path) -> Result<()> {
        ensure!(
            self.delay_ms <= 12_000,
            "Capture delay must be at most 12000 ms"
        );
        capture_path(directory, &self.name)?;
        Ok(())
    }
}

fn drag_duration() -> f64 {
    0.6
}

fn capture(desktop: &mut dyn Desktop, directory: &Path, name: &str) -> Result<Value> {
    let mut event = desktop.capture(&capture_path(directory, name)?)?;
    event["status"] = json!("capture");
    Ok(event)
}

struct Controller<'a> {
    desktop: &'a mut dyn Desktop,
    portal: &'a mut dyn Portal,
    pause: &'a mut dyn FnMut(Duration) -> Result<()>,
    mapping: Option<[f64; 4]>,
    deadline: Instant,
    now: &'a dyn Fn() -> Instant,
}

impl Controller<'_> {
    fn check_deadline(&self) -> Result<()> {
        ensure!((self.now)() < self.deadline, CommandTimedOut);
        Ok(())
    }

    fn pause(&mut self, duration: Duration) -> Result<()> {
        self.check_deadline()?;
        (self.pause)(duration)?;
        self.check_deadline()
    }

    fn sample(&mut self, point: [f64; 2]) -> Result<[f64; 2]> {
        self.check_deadline()?;
        self.desktop.verify()?;
        ensure!(
            self.desktop.pointer()?.mask == 0,
            "Release held buttons and modifiers before moving"
        );
        self.check_deadline()?;
        self.portal.move_to(point)?;
        self.pause(Duration::from_millis(80))?;
        let mut previous = self.desktop.pointer()?.root;
        for _ in 0..20 {
            self.pause(Duration::from_millis(20))?;
            let current = self.desktop.pointer()?.root;
            if current == previous {
                return Ok(current);
            }
            previous = current;
        }
        bail!("Pointer did not settle")
    }

    fn calibrate(&mut self) -> Result<Value> {
        self.mapping = None;
        let [width, height] = self.portal.size();
        ensure!(
            width.is_finite() && height.is_finite() && width >= 100.0 && height >= 100.0,
            "Selected monitor has no usable coordinate extent"
        );
        let geometry = self.desktop.geometry()?;
        let p0 = [width * 0.45, height * 0.45];
        let p1 = [width * 0.55, height * 0.45];
        let p2 = [width * 0.45, height * 0.55];
        let r0 = self.sample(p0)?;
        let r1 = self.sample(p1)?;
        let r2 = self.sample(p2)?;
        let sx = (r1[0] - r0[0]) / (p1[0] - p0[0]);
        let sy = (r2[1] - r0[1]) / (p2[1] - p0[1]);
        ensure!(
            sx > 0.0 && sy > 0.0 && (r1[1] - r0[1]).abs() <= 2.0 && (r2[0] - r0[0]).abs() <= 2.0,
            "Monitor mapping is not an aligned, positive scale"
        );
        let ox = r0[0] - sx * p0[0];
        let oy = r0[1] - sy * p0[1];
        ensure!(
            [sx, sy, ox, oy].iter().all(|v| v.is_finite()),
            "Invalid measured monitor mapping"
        );
        let check = [width * 0.5, height * 0.5];
        let actual = self.sample(check)?;
        ensure!(
            (actual[0] - (ox + sx * check[0])).abs() <= 2.0
                && (actual[1] - (oy + sy * check[1])).abs() <= 2.0,
            "Independent pointer calibration check failed"
        );
        self.desktop.verify()?;
        ensure!(
            self.desktop.geometry()? == geometry,
            "Window geometry changed during calibration"
        );
        let mapping = [sx, sy, ox, oy];
        self.mapping = Some(mapping);
        Ok(json!({"status":"ready", "mapping":mapping}))
    }

    fn mapped(&self, geometry: Geometry, point: [f64; 2]) -> Result<[f64; 2]> {
        let [x, y] = point;
        ensure!(
            x.is_finite()
                && y.is_finite()
                && x >= 0.0
                && y >= 0.0
                && x < geometry.width
                && y < geometry.height,
            "Target lies outside the app client"
        );
        let [sx, sy, ox, oy] = self
            .mapping
            .ok_or_else(|| anyhow::anyhow!("Calibrate before sending input"))?;
        let mapped = [(geometry.left + x - ox) / sx, (geometry.top + y - oy) / sy];
        let [width, height] = self.portal.size();
        ensure!(
            mapped[0] >= 0.0 && mapped[0] < width && mapped[1] >= 0.0 && mapped[1] < height,
            "Target lies outside the selected monitor mapping"
        );
        Ok(mapped)
    }

    fn verify_pointer(&mut self, geometry: Geometry, expected: [f64; 2], mask: u16) -> Result<()> {
        self.check_deadline()?;
        self.desktop.verify()?;
        let pointer = self.desktop.pointer()?;
        ensure!(
            self.desktop.geometry()? == geometry
                && pointer.mask == mask
                && pointer
                    .client
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| (a - b).abs() <= 2.0),
            "Window, pointer, or button state changed before or during input"
        );
        self.check_deadline()
    }

    fn click(&mut self, point: [f64; 2]) -> Result<()> {
        self.desktop.verify()?;
        let geometry = self.desktop.geometry()?;
        let mapped = self.mapped(geometry, point)?;
        self.sample(mapped)?;
        self.verify_pointer(geometry, point, 0)?;
        let result = self
            .portal
            .button(true, 272)
            .and_then(|()| self.pause(Duration::from_millis(100)));
        let release = self.portal.button(false, 272);
        release.context(ReleaseFailed)?;
        result
    }

    fn wheel(&mut self, point: [f64; 2], steps: i32) -> Result<()> {
        ensure!(
            steps != 0 && (-10..=10).contains(&steps),
            "Wheel input must be 1–10 signed vertical steps"
        );
        self.desktop.verify()?;
        let geometry = self.desktop.geometry()?;
        let mapped = self.mapped(geometry, point)?;
        self.sample(mapped)?;
        self.verify_pointer(geometry, point, 0)?;
        self.portal.wheel(steps)
    }

    fn right_drag(&mut self, start: [f64; 2], delta: [f64; 2], seconds: f64) -> Result<()> {
        ensure!(
            start
                .into_iter()
                .chain(delta)
                .chain([seconds])
                .all(f64::is_finite),
            "Drag coordinates and duration must be finite numbers"
        );
        let travel = delta[0].abs() + delta[1].abs();
        ensure!(
            (0.2..=2.0).contains(&seconds) && travel > 0.0 && travel <= 600.0,
            "Drag pacing must be 0.2–2 seconds with nonzero travel of at most 600 client pixels"
        );
        self.desktop.verify()?;
        let geometry = self.desktop.geometry()?;
        let points: Vec<_> = (0..=20)
            .map(|step| {
                [
                    start[0] + delta[0] * step as f64 / 20.0,
                    start[1] + delta[1] * step as f64 / 20.0,
                ]
            })
            .collect();
        // Validate the complete path before moving or pressing.
        let mapped: Vec<_> = points
            .iter()
            .map(|point| self.mapped(geometry, *point))
            .collect::<Result<_>>()?;
        self.sample(mapped[0])?;
        self.verify_pointer(geometry, points[0], 0)?;
        let result = (|| {
            self.portal.button(true, 273)?;
            self.pause(Duration::from_millis(100))?;
            for step in 1..=20 {
                self.verify_pointer(geometry, points[step - 1], 1 << 10)?;
                self.portal.move_to(mapped[step])?;
                self.pause(Duration::from_secs_f64(seconds / 20.0))?;
                self.verify_pointer(geometry, points[step], 1 << 10)?;
            }
            Ok(())
        })();
        let release = self.portal.button(false, 273);
        release.context(ReleaseFailed)?;
        result
    }
}

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
