//! Agent-seat admission (#153). The seat renders to an image with no window, so the
//! native gate, which samples a focused window's presented frames, can never admit it.
//! This gate changes nothing in the native one and makes no claim about display rate.
//! It admits play once required artwork is decoded, the symbol font is registered,
//! render pipelines are ready and GPU work for the seat's image has completed.

use super::*;
use bevy::{camera::NormalizedRenderTarget, render::camera::ExtractedCamera};

/// Completed image frames required after artwork and pipelines are ready. Frames
/// extracted before loading finished can still complete afterwards; one is not enough.
const REQUIRED_FRAMES: u64 = 3;

#[derive(Default)]
struct ImageFrames {
    generation: u64,
    completed: u64,
}

struct SharedImageProgress {
    frames: Mutex<ImageFrames>,
    pipelines_pending: AtomicBool,
}

#[derive(Resource, Clone)]
struct ImageProgress(Arc<SharedImageProgress>);

impl Default for ImageProgress {
    fn default() -> Self {
        Self(Arc::new(SharedImageProgress {
            frames: Mutex::default(),
            pipelines_pending: AtomicBool::new(true),
        }))
    }
}

impl ImageProgress {
    fn ticket(&self) -> u64 {
        self.0.frames.lock().unwrap().generation
    }

    fn completed(&self) -> u64 {
        self.0.frames.lock().unwrap().completed
    }

    /// Frames submitted before required artwork finished loading earn nothing.
    fn restart(&self) {
        let mut frames = self.0.frames.lock().unwrap();
        frames.generation += 1;
        frames.completed = 0;
    }

    fn complete(&self, ticket: u64) {
        let mut frames = self.0.frames.lock().unwrap();
        if ticket == frames.generation {
            frames.completed += 1;
        }
    }

    fn pipelines_pending(&self) -> bool {
        self.0.pipelines_pending.load(Ordering::Acquire)
    }
}

pub(crate) struct SeatAdmissionPlugin;

impl Plugin for SeatAdmissionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderHealth>()
            .init_resource::<HealthAssets>()
            .init_resource::<HealthEnvironment>()
            .init_resource::<ImageProgress>()
            .add_message::<WindowOccluded>()
            .add_message::<AppExit>()
            .add_systems(Startup, show_checking_overlay)
            .add_systems(PreUpdate, (observe_environment, admit_image).chain())
            .add_systems(Update, update_checking_overlay);
        let progress = app.world().resource::<ImageProgress>().clone();
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(progress)
                .init_resource::<ImageFrame>()
                .add_systems(
                    Render,
                    (
                        observe_image
                            .before(render_system)
                            .in_set(RenderSystems::Render),
                        acknowledge_image
                            .after(render_system)
                            .in_set(RenderSystems::Render),
                    ),
                );
        }
    }
}

fn admit_image(
    mut health: ResMut<RenderHealth>,
    environment: Res<HealthEnvironment>,
    progress: Res<ImageProgress>,
    mut exit: MessageWriter<AppExit>,
) {
    if health.just_ready {
        health.just_ready = false;
    }
    if matches!(health.phase, Phase::Failed(_)) {
        return;
    }
    let refusal = if let Some(reason) = &environment.asset_failure {
        Some(reason.clone())
    } else if !environment.loaded {
        if health.allows_play() {
            Some("Required artwork became unavailable. Gameplay has stopped; the last committed save is unchanged.".to_owned())
        } else {
            progress.restart();
            None
        }
    } else {
        if health.is_checking() && progress.completed() >= REQUIRED_FRAMES {
            health.phase = Phase::Ready;
            health.just_ready = true;
        }
        None
    };
    if let Some(reason) = refusal {
        // As in native play, a refusal ends the session. The seat reports it as text.
        eprintln!("{reason}");
        health.phase = Phase::Failed(reason);
        exit.write(AppExit::error());
    }
}

/// Why the seat has not admitted play yet, for a refusal when its startup limit expires.
pub(crate) fn admission_blocker(world: &World) -> String {
    let health = world.resource::<RenderHealth>();
    if let Phase::Failed(reason) = &health.phase {
        return reason.clone();
    }
    let environment = world.resource::<HealthEnvironment>();
    if !environment.loaded {
        return if environment.pending_assets.is_empty() {
            "Required artwork has not finished loading.".into()
        } else {
            format!(
                "Required artwork has not finished loading.\n{}",
                environment.pending_assets
            )
        };
    }
    let progress = world.resource::<ImageProgress>();
    if progress.pipelines_pending() {
        return "Render pipelines are not ready.".into();
    }
    format!(
        "GPU work for the seat's image has not completed ({} of {REQUIRED_FRAMES} frames).",
        progress.completed()
    )
}

/// A capture taken while a pipeline is still compiling can omit what it draws.
pub(crate) fn pipelines_ready(world: &World) -> bool {
    !world.resource::<ImageProgress>().pipelines_pending()
}

#[derive(Resource, Default)]
struct ImageFrame(Option<u64>);

fn observe_image(
    cameras: Query<&ExtractedCamera>,
    pipelines: Res<PipelineCache>,
    progress: Res<ImageProgress>,
    mut frame: ResMut<ImageFrame>,
) {
    let pending = pipelines.waiting_pipelines().next().is_some()
        || pipelines
            .pipelines()
            .any(|pipeline| !matches!(pipeline.state, CachedPipelineState::Ok(_)));
    progress
        .0
        .pipelines_pending
        .store(pending, Ordering::Release);
    let renders_image = cameras
        .iter()
        .any(|camera| matches!(camera.target, Some(NormalizedRenderTarget::Image(_))));
    frame.0 = (renders_image && !pending).then(|| progress.ticket());
}

fn acknowledge_image(
    queue: Res<RenderQueue>,
    progress: Res<ImageProgress>,
    mut frame: ResMut<ImageFrame>,
) {
    let Some(ticket) = frame.0.take() else {
        return;
    };
    let progress = progress.clone();
    // Completed GPU work for a frame that drew the image. It is not a display timestamp.
    queue.on_submitted_work_done(move || progress.complete(ticket));
}
