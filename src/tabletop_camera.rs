//! Camera lifecycle shared by the two tabletop adapters. No rules or choices live here.

use bevy::prelude::*;

use crate::tabletop_layout::{self as layout, Area};

pub(crate) fn parse_initial_view(value: &str) -> Result<Area, String> {
    match value {
        "overview" => Ok(Area::Overview),
        "scenario" => Ok(Area::Scenario),
        "test" => Ok(Area::Test),
        "tony" => Ok(Area::Investigator(0)),
        "patrice" => Ok(Area::Investigator(1)),
        "jacqueline" => Ok(Area::Investigator(2)),
        _ => Err(
            "--initial-view must be overview, scenario, test, tony, patrice or jacqueline".into(),
        ),
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CameraMode {
    Jacqueline,
    Tony,
}

impl CameraMode {
    fn home(self) -> (Vec3, f32) {
        layout::focus(match self {
            Self::Jacqueline => Area::Overview,
            Self::Tony => Area::Investigator(0),
        })
    }
}

/// A read-only snapshot; the camera alone owns the mutable orbit state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pose {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
}

impl Pose {
    fn at((target, distance): (Vec3, f32)) -> Self {
        Self {
            yaw: 0.0,
            pitch: 1.0,
            distance,
            target,
        }
    }

    fn transform(self) -> Transform {
        let direction = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        Transform::from_translation(self.target + direction * self.distance)
            .looking_at(self.target, Vec3::Y)
    }
}

/// Facts supplied by the adapter's existing UI/picking systems.
#[derive(Clone, Copy, Default)]
pub(crate) struct Capture {
    pub pointer_blocked: bool,
    pub over_ui: bool,
    pub keyboard: bool,
    pub wheel: bool,
}

pub(crate) struct FrameInput<'a> {
    pub keys: &'a ButtonInput<KeyCode>,
    pub buttons: &'a ButtonInput<MouseButton>,
    pub window: &'a Window,
    pub scroll_y: f32,
    pub dt: f32,
    pub capture: Capture,
    pub reduced_motion: bool,
}

pub(crate) struct CameraStep {
    pub transform: Transform,
    pub manual_navigation: bool,
    pub stopped_following: bool,
}

#[derive(Resource)]
pub(crate) struct Orbit {
    mode: CameraMode,
    pose: Pose,
    desired_target: Vec3,
    desired_distance: f32,
    following: bool,
    drag: Option<Vec2>,
}

impl Orbit {
    pub(crate) fn new(mode: CameraMode) -> Self {
        let pose = Pose::at(mode.home());
        Self {
            mode,
            pose,
            desired_target: pose.target,
            desired_distance: pose.distance,
            following: false,
            drag: None,
        }
    }

    /// A diagnostic launch changes the starting pose, never the mode's Home pose.
    pub(crate) fn with_initial_view(mut self, view: Option<Area>) -> Self {
        if let Some(view) = view {
            self.focus(layout::focus(view));
        }
        self
    }

    pub(crate) fn pose(&self) -> Pose {
        self.pose
    }

    pub(crate) fn is_following(&self) -> bool {
        self.following
    }

    pub(crate) fn is_moving(&self) -> bool {
        !self.pose.target.abs_diff_eq(self.desired_target, 0.001)
            || (matches!(self.mode, CameraMode::Jacqueline)
                && (self.pose.distance - self.desired_distance).abs() >= 0.001)
    }

    pub(crate) fn set_following(&mut self, following: bool) {
        self.following = following;
    }

    pub(crate) fn focus(&mut self, framing: (Vec3, f32)) {
        self.pose = Pose::at(framing);
        self.freeze_framing();
        self.following = false;
    }

    /// Called at the existing action boundary, never derived afresh each frame.
    /// Jacqueline freezes framing on actions while following is off; Tony may
    /// finish converging on the last requested target after following stops.
    pub(crate) fn update_follow_target(&mut self, framing: Option<(Vec3, f32)>) {
        if !self.following {
            if matches!(self.mode, CameraMode::Jacqueline) {
                self.freeze_framing();
            }
        } else if let Some((target, distance)) = framing {
            self.desired_target = target;
            self.desired_distance = distance;
            if matches!(self.mode, CameraMode::Tony) {
                self.pose.distance = distance;
            }
        }
    }

    fn freeze_framing(&mut self) {
        self.desired_target = self.pose.target;
        self.desired_distance = self.pose.distance;
    }

    pub(crate) fn step(&mut self, input: FrameInput<'_>) -> CameraStep {
        let pointer_enabled = input.window.focused && !input.capture.pointer_blocked;
        let cursor = input.window.cursor_position();
        let scene_pointer = pointer_enabled && cursor.is_some() && !input.capture.over_ui;
        // Logical window positions are the only motion source: raw device motion
        // must never be added to this displacement, including on high-DPI displays.
        let motion = match cursor {
            Some(position) if pointer_enabled && input.buttons.pressed(MouseButton::Right) => {
                if input.buttons.just_pressed(MouseButton::Right) {
                    self.drag = scene_pointer.then_some(position);
                    Vec2::ZERO
                } else if let Some(previous) = self.drag.as_mut() {
                    let delta = position - *previous;
                    *previous = position;
                    delta
                } else {
                    Vec2::ZERO
                }
            }
            _ => {
                self.drag = None;
                Vec2::ZERO
            }
        };
        let was_following = self.following;
        let mut changed = motion != Vec2::ZERO;
        if changed {
            self.pose.yaw -= motion.x * 0.006;
            self.pose.pitch += motion.y * 0.004;
        }
        if scene_pointer && !input.capture.wheel && input.scroll_y != 0.0 {
            self.pose.distance -= input.scroll_y * 0.6;
            changed = true;
        }
        if input.window.focused && !input.capture.keyboard {
            for (key, yaw, pitch, zoom) in [
                (KeyCode::ArrowLeft, 1.0, 0.0, 0.0),
                (KeyCode::ArrowRight, -1.0, 0.0, 0.0),
                (KeyCode::ArrowUp, 0.0, 0.7, 0.0),
                (KeyCode::ArrowDown, 0.0, -0.7, 0.0),
                (KeyCode::Equal, 0.0, 0.0, -5.0),
                (KeyCode::Minus, 0.0, 0.0, 5.0),
            ] {
                if input.keys.pressed(key) {
                    self.pose.yaw += yaw * input.dt;
                    self.pose.pitch += pitch * input.dt;
                    self.pose.distance += zoom * input.dt;
                    changed = true;
                }
            }
            if input.keys.just_pressed(KeyCode::Home) {
                // Home resets framing, not a held scene drag's capture.
                self.focus(self.mode.home());
                changed = true;
            }
            let delta = pan_delta(input.keys, self.pose.yaw, input.dt, self.pose.distance);
            if delta != Vec3::ZERO {
                self.pose.target += delta;
                changed = true;
            }
        }
        if changed {
            self.following = false;
            self.freeze_framing();
        }
        self.pose.pitch = self.pose.pitch.clamp(0.35, 1.35);
        self.pose.distance = self.pose.distance.clamp(6.0, 90.0);
        let blend = (input.dt * 4.0).min(1.0);
        // Retain Jacqueline's existing clamp-then-interpolate ordering.
        if matches!(self.mode, CameraMode::Jacqueline) {
            self.pose.distance += (self.desired_distance - self.pose.distance) * blend;
        }
        self.pose.target = if matches!(self.mode, CameraMode::Tony) && input.reduced_motion {
            self.desired_target
        } else {
            self.pose.target.lerp(self.desired_target, blend)
        };
        if self.pose.target.abs_diff_eq(self.desired_target, 0.001) {
            self.pose.target = self.desired_target;
        }
        if matches!(self.mode, CameraMode::Jacqueline)
            && (self.pose.distance - self.desired_distance).abs() < 0.001
        {
            self.pose.distance = self.desired_distance;
        }
        CameraStep {
            transform: self.pose.transform(),
            manual_navigation: changed,
            stopped_following: changed && was_following,
        }
    }
}

/// Unmodified WASD pans in the camera's horizontal plane at a zoom-scaled rate.
fn pan_delta(keys: &ButtonInput<KeyCode>, yaw: f32, dt: f32, distance: f32) -> Vec3 {
    if keys.any_pressed([
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]) {
        return Vec3::ZERO;
    }
    let horizontal =
        f32::from(keys.pressed(KeyCode::KeyD)) - f32::from(keys.pressed(KeyCode::KeyA));
    let forward = f32::from(keys.pressed(KeyCode::KeyW)) - f32::from(keys.pressed(KeyCode::KeyS));
    let direction = Vec2::new(horizontal, forward).normalize_or_zero();
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    let ahead = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    (right * direction.x + ahead * direction.y) * dt * distance * 0.55
}

#[cfg(test)]
mod tests;
