use crate::native::capture_path;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

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
    fn close(&mut self) -> Result<()>;
}

pub fn run_session(
    desktop: &mut dyn Desktop,
    portal: &mut dyn Portal,
    directory: &Path,
    commands: &mut dyn Iterator<Item = Result<String>>,
    emit: &mut dyn FnMut(Value) -> Result<()>,
    pause: &mut dyn FnMut(Duration) -> Result<()>,
) -> Result<()> {
    let result = (|| {
        desktop.focus()?;
        emit(capture(desktop, directory, "initial.png")?)?;
        portal.start(emit)?;
        let mut controller = Controller {
            desktop,
            portal,
            pause,
            mapping: None,
        };
        let ready = controller
            .desktop
            .focus()
            .and_then(|()| controller.calibrate());
        emit(ready.unwrap_or_else(refused))?;
        for line in commands {
            let line = line?;
            let event = (|| -> Result<Option<Value>> {
                Ok(Some(match serde_json::from_str::<Request>(&line)? {
                    Request::Stop => return Ok(None),
                    Request::Capture { name } => capture(controller.desktop, directory, &name)?,
                    Request::Focus => {
                        controller.desktop.focus()?;
                        json!({"status":"focused"})
                    }
                    Request::Calibrate => controller.calibrate()?,
                    Request::Click { x, y } => {
                        controller.click([x, y])?;
                        json!({"status":"clicked", "client":[x, y]})
                    }
                    Request::RightDrag {
                        x,
                        y,
                        dx,
                        dy,
                        duration_seconds,
                    } => {
                        controller.right_drag([x, y], [dx, dy], duration_seconds)?;
                        json!({"status":"right-dragged", "client":[x, y], "delta":[dx, dy], "duration_seconds":duration_seconds})
                    }
                }))
            })();
            match event {
                Ok(None) => break,
                Ok(Some(event)) => emit(event)?,
                Err(error) if error.is::<ReleaseFailed>() => return Err(error),
                Err(error) => emit(refused(error))?,
            }
        }
        Ok(())
    })();
    let closed = portal.close();
    if closed.is_ok() {
        emit(json!({"status":"session-closed"}))?;
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

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
enum Request {
    Stop,
    Capture {
        name: String,
    },
    Focus,
    Calibrate,
    Click {
        x: f64,
        y: f64,
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

fn drag_duration() -> f64 {
    0.6
}

fn refused(error: anyhow::Error) -> Value {
    json!({"status":"refused", "reason": format!("{error:#}")})
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
}

impl Controller<'_> {
    fn sample(&mut self, point: [f64; 2]) -> Result<[f64; 2]> {
        self.desktop.verify()?;
        ensure!(
            self.desktop.pointer()?.mask == 0,
            "Release held buttons and modifiers before moving"
        );
        self.portal.move_to(point)?;
        (self.pause)(Duration::from_millis(80))?;
        let mut previous = self.desktop.pointer()?.root;
        for _ in 0..20 {
            (self.pause)(Duration::from_millis(20))?;
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
        Ok(())
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
            .and_then(|()| (self.pause)(Duration::from_millis(100)));
        let release = self.portal.button(false, 272);
        release.context(ReleaseFailed)?;
        result
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
            (self.pause)(Duration::from_millis(100))?;
            for step in 1..=20 {
                self.verify_pointer(geometry, points[step - 1], 1 << 10)?;
                self.portal.move_to(mapped[step])?;
                (self.pause)(Duration::from_secs_f64(seconds / 20.0))?;
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
