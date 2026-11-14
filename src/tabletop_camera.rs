//! Gathering framing policy over the shared table camera.

use bevy::prelude::*;

use crate::tabletop_layout::{self as layout, Area};
pub(crate) use unfolded_tabletop::camera::{Capture, FrameInput, wheel_steps};

#[cfg(test)]
use bevy::input::mouse::AccumulatedMouseScroll;
#[cfg(test)]
pub(crate) use unfolded_tabletop::camera::{CameraStep, Pose};

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

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct Orbit(unfolded_tabletop::camera::Orbit);

impl Orbit {
    pub(crate) fn new() -> Self {
        Self(unfolded_tabletop::camera::Orbit::new(layout::focus(
            Area::Overview,
        )))
    }

    pub(crate) fn with_home(self, framing: (Vec3, f32)) -> Self {
        Self(self.0.with_home(framing))
    }
}

#[cfg(test)]
mod tests;
