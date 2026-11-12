//! Decision-only MSAA comparison for #189, installed only while frame stages are
//! recorded. It is not meant for `main`.
//!
//! F5, F6 and F7 set the table camera's MSAA to 4×, 2× and off. A key pressed again
//! keeps its setting. Only the table camera changes: the cameras that draw text into
//! images keep Bevy's default. `ARKHAM_MSAA_START` (`4`, `2` or `off`) chooses the
//! setting at startup so the agent seat, which has no keyboard, can render each one on
//! a real adapter; unset, the table starts at Bevy's default of 4×.
//!
//! A setting the adapter does not allow is refused: a key press leaves the setting
//! unchanged and the title says so, and a start setting stops the launch. Every
//! choice, allowed or refused, is a row in the record, and the window title shows the
//! setting and whether GPU readings are being taken.

use super::{Row, recorder};
use crate::gathering_tabletop::MainCamera;
use bevy::{
    prelude::*,
    render::{
        render_resource::{TextureFormat, TextureFormatFeatureFlags},
        renderer::{RenderAdapter, RenderDevice},
        settings::WgpuFeatures,
        view::Msaa,
    },
};
use std::time::Instant;

const START: &str = "ARKHAM_MSAA_START";
/// Separates the window's own title from the comparison's suffix.
const TITLE_SEPARATOR: &str = " · #189: ";

/// One key per setting, so a repeated press cannot undo a choice.
pub(crate) const KEYS: [(KeyCode, Msaa); 3] = [
    (KeyCode::F5, Msaa::Sample4),
    (KeyCode::F6, Msaa::Sample2),
    (KeyCode::F7, Msaa::Off),
];

/// Every format the table's multisampled targets can use: the main pass's colour with
/// sRGB compositing, a window's colour after Bevy turns BGRA into RGBA, an image's
/// colour as created, and depth.
const FORMATS: [TextureFormat; 4] = [
    TextureFormat::Rgba8Unorm,
    TextureFormat::Rgba8UnormSrgb,
    TextureFormat::Bgra8UnormSrgb,
    TextureFormat::Depth32Float,
];

#[derive(Resource, Debug, PartialEq)]
pub(crate) struct ComparedMsaa {
    pub chosen: Msaa,
    /// The counts the adapter allows, as in [`Row::MsaaSupport`]. `None` until the
    /// renderer exists.
    pub counts: Option<u32>,
    /// The last refused choice, shown until a choice is allowed.
    pub refused: Option<Msaa>,
}

pub(crate) struct MsaaComparisonPlugin;

impl Plugin for MsaaComparisonPlugin {
    fn build(&self, app: &mut App) {
        let chosen = start_setting(std::env::var(START).ok().as_deref())
            .unwrap_or_else(|error| panic!("{START}: {error}"));
        app.insert_resource(ComparedMsaa {
            chosen,
            counts: None,
            refused: None,
        })
        .add_systems(Update, (check_support, choose, apply, label).chain());
    }
}

/// Sends a row stamped with the time since the recorder started, when recording.
fn mark(row: impl FnOnce(u64) -> Row) {
    if let Some(recorder) = recorder() {
        recorder.send(row(recorder.since_start(Instant::now())));
    }
}

pub(crate) fn start_setting(value: Option<&str>) -> Result<Msaa, String> {
    match value.map(str::trim) {
        None | Some("" | "4") => Ok(Msaa::Sample4),
        Some("2") => Ok(Msaa::Sample2),
        Some("off" | "1") => Ok(Msaa::Off),
        Some(other) => Err(format!("`{other}` is not 4, 2 or off")),
    }
}

pub(crate) fn name(msaa: Msaa) -> &'static str {
    match msaa {
        Msaa::Off => "off",
        Msaa::Sample2 => "2×",
        Msaa::Sample4 => "4×",
        Msaa::Sample8 => "8×",
    }
}

/// The sample counts every format in [`FORMATS`] allows. Without adapter-specific
/// format features a device accepts only WebGPU's guaranteed counts, 1 and 4.
pub(crate) fn support_counts(
    adapter_specific: bool,
    flags: impl Fn(TextureFormat) -> TextureFormatFeatureFlags,
) -> u32 {
    [2, 4, 8]
        .into_iter()
        .filter(|count| *count == 4 || adapter_specific)
        .filter(|count| {
            FORMATS
                .iter()
                .all(|format| flags(*format).sample_count_supported(*count))
        })
        .fold(0, |counts, count| counts | count)
}

/// One sample is always allowed. Before the renderer exists only Bevy's default is.
pub(crate) fn allowed(counts: Option<u32>, msaa: Msaa) -> bool {
    msaa == Msaa::Off || counts.unwrap_or(4) & msaa.samples() != 0
}

/// Once the renderer exists, records what the adapter allows, then the start setting,
/// and refuses a start setting the adapter does not allow.
fn check_support(
    mut compared: ResMut<ComparedMsaa>,
    adapter: Option<Res<RenderAdapter>>,
    device: Option<Res<RenderDevice>>,
) {
    let (None, Some(adapter), Some(device)) = (compared.counts, adapter, device) else {
        return;
    };
    let counts = support_counts(
        device
            .features()
            .contains(WgpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES),
        |format| adapter.get_texture_format_features(format).flags,
    );
    compared.counts = Some(counts);
    mark(|at| Row::MsaaSupport { at, counts });
    let allowed_names: Vec<&str> = [Msaa::Off, Msaa::Sample2, Msaa::Sample4, Msaa::Sample8]
        .into_iter()
        .filter(|msaa| allowed(Some(counts), *msaa))
        .map(name)
        .collect();
    info!(
        "MSAA comparison (#189): the adapter allows {}",
        allowed_names.join(", ")
    );
    let chosen = compared.chosen;
    assert!(
        allowed(Some(counts), chosen),
        "{START}: the adapter does not allow MSAA {}",
        name(chosen)
    );
    mark(|at| Row::Msaa {
        at,
        samples: chosen.samples(),
        allowed: true,
    });
    info!("MSAA comparison (#189): starting at {}", name(chosen));
}

fn choose(keys: Option<Res<ButtonInput<KeyCode>>>, mut compared: ResMut<ComparedMsaa>) {
    let Some(keys) = keys else {
        return;
    };
    for (key, msaa) in KEYS {
        if !keys.just_pressed(key) {
            continue;
        }
        let allowed = allowed(compared.counts, msaa);
        if allowed {
            compared.chosen = msaa;
            compared.refused = None;
            info!("MSAA comparison (#189): {}", name(msaa));
        } else {
            compared.refused = Some(msaa);
            warn!(
                "MSAA comparison (#189): the adapter does not allow {}; staying at {}",
                name(msaa),
                name(compared.chosen)
            );
        }
        mark(|at| Row::Msaa {
            at,
            samples: msaa.samples(),
            allowed,
        });
    }
}

/// Every frame, so a table camera spawned later gets the chosen setting too. Bevy
/// requires `Msaa` on every camera.
fn apply(compared: Res<ComparedMsaa>, mut cameras: Query<&mut Msaa, With<MainCamera>>) {
    for mut msaa in &mut cameras {
        msaa.set_if_neq(compared.chosen);
    }
}

/// The window title's suffix: the setting, a refused choice, whether GPU readings are
/// taken, and the keys, each marked when the adapter does not allow its setting.
pub(crate) fn title(compared: &ComparedMsaa, sampling: bool) -> String {
    let mut title = format!("{TITLE_SEPARATOR}MSAA {}", name(compared.chosen));
    if let Some(refused) = compared.refused {
        title.push_str(&format!(" ({} unavailable)", name(refused)));
    }
    title.push_str(if sampling {
        " · GPU readings on · "
    } else {
        " · GPU readings off · "
    });
    let keys: Vec<String> = KEYS
        .iter()
        .map(|(key, msaa)| {
            let key = format!("{key:?}");
            match compared.counts {
                Some(counts) if !allowed(Some(counts), *msaa) => {
                    format!("{key} {} unavailable", name(*msaa))
                }
                _ => format!("{key} {}", name(*msaa)),
            }
        })
        .collect();
    title.push_str(&keys.join(", "));
    if sampling {
        title.push_str(", F9 stops readings");
    }
    title
}

/// Rewrites the title only when its suffix changes.
fn label(compared: Res<ComparedMsaa>, mut shown: Local<String>, mut windows: Query<&mut Window>) {
    let suffix = title(
        &compared,
        recorder().is_some_and(|recorder| recorder.sampling()),
    );
    if *shown == suffix {
        return;
    }
    for mut window in &mut windows {
        let base = window
            .title
            .split(TITLE_SEPARATOR)
            .next()
            .unwrap_or_default();
        window.title = format!("{base}{suffix}");
    }
    *shown = suffix;
}

#[cfg(test)]
mod tests;
