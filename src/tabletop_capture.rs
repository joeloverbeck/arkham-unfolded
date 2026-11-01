//! Timed diagnostics around Bevy's screenshot writer.

use bevy::{prelude::*, render::view::screenshot::ScreenshotCaptured};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) fn save_to_disk(path: PathBuf, elapsed: Duration) -> impl FnMut(On<ScreenshotCaptured>) {
    let requested = Instant::now();
    info!(elapsed_seconds = elapsed.as_secs_f64(), path = %path.display(), "Screenshot requested");
    let mut save = bevy::render::view::screenshot::save_to_disk(path.clone());
    move |captured| {
        // Bevy reports success only after the write returns, and retains its
        // encoding and IO errors. Contextualize those events, not file existence.
        let span = info_span!("Screenshot write", write_started_seconds = (elapsed + requested.elapsed()).as_secs_f64(), path = %path.display());
        let _entered = span.enter();
        save(captured);
        info!(
            elapsed_seconds = (elapsed + requested.elapsed()).as_secs_f64(),
            "Screenshot writer returned"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        asset::RenderAssetUsages,
        log::tracing_subscriber,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    use std::sync::Arc;

    #[test]
    fn screenshot_writer_reports_timed_request_and_actual_write_outcome() {
        let directory = tempfile::tempdir().unwrap();
        let log_path = directory.path().join("capture.log");
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_target(false)
            .with_writer(Arc::new(std::fs::File::create(&log_path).unwrap()))
            .finish();
        let saved = directory.path().join("saved.png");
        let refused = directory.path().join("absent-parent/refused.png");
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            let mut world = World::new();
            for path in [&saved, &refused] {
                let entity = world
                    .spawn_empty()
                    .observe(save_to_disk(path.clone(), Duration::from_secs(5)))
                    .id();
                world.trigger(ScreenshotCaptured {
                    entity,
                    image: Image::new_fill(
                        Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        &[255, 0, 0, 255],
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::MAIN_WORLD,
                    ),
                });
            }
        });
        assert!(
            std::fs::read(&saved)
                .unwrap()
                .starts_with(b"\x89PNG\r\n\x1a\n")
        );
        assert!(!refused.exists());
        let contents = std::fs::read_to_string(log_path).unwrap();
        let requests: Vec<_> = contents
            .lines()
            .filter(|line| line.contains("Screenshot requested"))
            .collect();
        assert_eq!(requests.len(), 2, "{contents}");
        assert!(
            requests
                .iter()
                .all(|line| line.contains("elapsed_seconds=5.0"))
        );
        let successes: Vec<_> = contents
            .lines()
            .filter(|line| line.contains("Screenshot saved to"))
            .collect();
        assert_eq!(successes.len(), 1, "{contents}");
        let failure = contents
            .lines()
            .find(|line| line.contains("Cannot save screenshot"))
            .expect("the writer must report its failed write");
        let returns: Vec<_> = contents
            .lines()
            .filter(|line| line.contains("Screenshot writer returned"))
            .collect();
        assert_eq!(returns.len(), 2, "{contents}");
        for (line, path) in returns.iter().zip([&saved, &refused]) {
            assert!(line.contains("elapsed_seconds="), "{line}");
            assert!(line.contains(path.to_str().unwrap()), "{line}");
        }
        for (line, path) in [(successes[0], &saved), (failure, &refused)] {
            assert!(line.contains("Screenshot write"), "{line}");
            assert!(line.contains("write_started_seconds="), "{line}");
            assert!(line.contains(path.to_str().unwrap()), "{line}");
        }
        for (outcome, returned) in [successes[0], failure].into_iter().zip(returns) {
            assert!(contents.find(outcome).unwrap() < contents.find(returned).unwrap());
            let seconds = |field| {
                returned
                    .split_once(field)
                    .unwrap()
                    .1
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .parse::<f64>()
                    .unwrap()
            };
            assert!(seconds("elapsed_seconds=") >= seconds("write_started_seconds="));
        }
    }
}
