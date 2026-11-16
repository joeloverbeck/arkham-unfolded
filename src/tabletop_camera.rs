//! Gathering framing policy over the shared table camera.

use bevy::prelude::*;

use crate::tabletop_layout::Area;
pub(crate) use unfolded_tabletop::camera::{Framing, Orbit};

#[cfg(test)]
use crate::tabletop_layout as layout;
#[cfg(test)]
pub(crate) use unfolded_tabletop::camera::{CameraStep, Capture, FrameInput, Pose};

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

/// Arkham's camera at its overview, which is also its home.
#[cfg(test)]
pub(crate) fn overview() -> Orbit {
    Orbit::new(layout::focus(Area::Overview))
}

#[cfg(test)]
mod tests;
