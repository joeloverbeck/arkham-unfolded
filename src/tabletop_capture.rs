//! Timed diagnostics and actual write outcomes for Bevy screenshots.

use bevy::{prelude::*, render::view::screenshot::ScreenshotCaptured};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) fn save_to_disk(path: PathBuf, elapsed: Duration) -> impl FnMut(On<ScreenshotCaptured>) {
    let mut write = writer(path, elapsed);
    move |captured| {
        let _ = write(&captured.image);
    }
}

pub(crate) fn save_to_disk_and_exit(
    path: PathBuf,
    elapsed: Duration,
) -> impl FnMut(On<ScreenshotCaptured>, Res<crate::tabletop_health::RenderHealth>, MessageWriter<AppExit>)
{
    let mut write = writer(path, elapsed);
    move |captured, health, mut exit| {
        exit.write(if write(&captured.image).is_ok() && health.allows_play() {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}

fn writer(path: PathBuf, elapsed: Duration) -> impl FnMut(&Image) -> Result<(), String> {
    let requested = Instant::now();
    info!(elapsed_seconds = elapsed.as_secs_f64(), path = %path.display(), "Screenshot requested");
    move |image| {
        let span = info_span!("Screenshot write", write_started_seconds = (elapsed + requested.elapsed()).as_secs_f64(), path = %path.display());
        let _entered = span.enter();
        let result = write_image(image.clone(), &path);
        match &result {
            Ok(()) => info!("Screenshot saved to {}", path.display()),
            Err(error) => error!("Cannot save screenshot: {error}"),
        }
        info!(
            elapsed_seconds = (elapsed + requested.elapsed()).as_secs_f64(),
            "Screenshot writer returned"
        );
        result
    }
}

fn write_image(image: Image, path: &std::path::Path) -> Result<(), String> {
    let image = image
        .try_into_dynamic()
        .map_err(|error| error.to_string())?;
    // Screenshots from the native window use an 8-bit surface. Preserve Bevy's
    // RGB output (alpha can carry HDR brightness), including linear-to-sRGB
    // conversion if a floating-point render target supplies the capture.
    let rgb = if matches!(
        image,
        image::DynamicImage::ImageRgb32F(_) | image::DynamicImage::ImageRgba32F(_)
    ) {
        let linear = image.into_rgb32f();
        image::RgbImage::from_fn(linear.width(), linear.height(), |x, y| {
            let [r, g, b] = linear.get_pixel(x, y).0;
            image::Rgb(Srgba::from(LinearRgba::rgb(r, g, b)).to_u8_array_no_alpha())
        })
    } else {
        image.to_rgb8()
    };
    rgb.save(path).map_err(|error| error.to_string())
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
    fn capture_exit_uses_the_actual_write_result_and_current_graphics_health() {
        let directory = tempfile::tempdir().unwrap();
        let mut world = World::new();
        world.init_resource::<Messages<AppExit>>();
        for (name, health, succeeds) in [
            (
                "saved.png",
                crate::tabletop_health::RenderHealth::ready_for_test(),
                true,
            ),
            (
                "stale.unsupported",
                crate::tabletop_health::RenderHealth::ready_for_test(),
                false,
            ),
            (
                "unhealthy.png",
                crate::tabletop_health::RenderHealth::failed_for_test(),
                false,
            ),
        ] {
            let path = directory.path().join(name);
            std::fs::write(&path, "older file").unwrap();
            world.insert_resource(health);
            world.resource_mut::<Messages<AppExit>>().clear();
            let entity = world
                .spawn_empty()
                .observe(save_to_disk_and_exit(path.clone(), Duration::ZERO))
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
            let exits: Vec<_> = world
                .resource::<Messages<AppExit>>()
                .iter_current_update_messages()
                .collect();
            assert_eq!(exits.len(), 1);
            assert_eq!(exits[0].is_success(), succeeds, "{name}");
            if name == "stale.unsupported" {
                assert_eq!(std::fs::read_to_string(path).unwrap(), "older file");
            }
        }
    }

    #[test]
    fn screenshot_writer_reports_timed_request_and_actual_write_outcome() {
        const CHILD: &str = "ARKHAM_CAPTURE_DIAGNOSTICS_TEST_CHILD";
        const VERIFIED: &str = "capture diagnostic assertions completed";
        if std::env::var_os(CHILD).is_none() {
            // Other capture tests register the same tracing callsites on their
            // own threads. Isolate the diagnostic subscriber and make it
            // process-global, as it is in native startup.
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "tabletop_capture::tests::screenshot_writer_reports_timed_request_and_actual_write_outcome",
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
        bevy::log::tracing::subscriber::set_global_default(subscriber).unwrap();
        {
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
        }
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
        println!("{VERIFIED}");
    }
}
