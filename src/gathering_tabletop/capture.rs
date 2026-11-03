//! Diagnostic capture of the ordinary Gathering window.

use crate::{
    tabletop_capture,
    tabletop_health::RenderHealth,
    tabletop_pacing::{Redraw, RequestRedraw},
};
use bevy::{prelude::*, render::view::screenshot::Screenshot};
use std::path::PathBuf;

#[derive(Default, Resource)]
pub(super) struct State {
    path: Option<PathBuf>,
    quit: bool,
    ready_frames: u32,
    requested: bool,
}

impl State {
    pub(super) fn new(path: Option<PathBuf>, quit: bool) -> Self {
        Self {
            path,
            quit,
            ..default()
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<AppExit>()
        .add_systems(Update, capture_window.after(super::render))
        .add_systems(Last, request_redraw.in_set(RequestRedraw));
}

fn capture_window(mut commands: Commands, health: Res<RenderHealth>, mut capture: ResMut<State>) {
    if capture.path.is_none()
        || capture.requested
        || !health.allows_play()
        || health.just_became_ready()
    {
        return;
    }
    capture.ready_frames += 1;
    if capture.ready_frames < 90 {
        return;
    }
    let path = capture.path.as_ref().unwrap();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut screenshot = commands.spawn(Screenshot::primary_window());
    if capture.quit {
        screenshot.observe(tabletop_capture::save_to_disk_and_exit(
            path.clone(),
            health.diagnostic_elapsed(),
        ));
    } else {
        screenshot.observe(tabletop_capture::save_to_disk(
            path.clone(),
            health.diagnostic_elapsed(),
        ));
    }
    capture.requested = true;
}

fn request_redraw(
    capture: Res<State>,
    screenshots: Query<(), With<Screenshot>>,
    mut redraw: ResMut<Redraw>,
) {
    if (capture.path.is_some() && !capture.requested) || !screenshots.is_empty() {
        redraw.request();
    }
}
