//! Agent seat (#153): Arkham's side of the game-neutral half. An agent sits at the real
//! table through one JSON request per line and one JSON response per line, with no
//! window: on standard input and output in batch form, or held across shell commands on
//! a local socket ([`socket`], #175). The protocol loop both forms share, its guards,
//! fixed-step stepping, pointer and key synthesis, capture and the held session come
//! from `unfolded-tabletop`'s `seat`. It reaches the game only through [`Game`], which
//! `gathering_tabletop::seat` implements: produce an observation and its revision,
//! resolve a name to a screen position (and a hit back to a name), say what has
//! keyboard focus, report whether the table is settled, advance the presentation clock,
//! say whether the table is admitted and its render pipelines are ready, and name the
//! keys the seat may press.
//!
//! What stays here is Arkham's: the plugins of its windowless host, the two pacing
//! resources its table reads, the keys the table reads and the two of them that can
//! commit, what the seat says when admission runs out of time, the name that marks a
//! launched seat, and how long a held session idles unless told otherwise.

use crate::{
    tabletop_health,
    tabletop_pacing::{PresentationTime, Redraw},
};
use bevy::{
    app::PluginGroupBuilder, asset::AssetPlugin, input::keyboard::Key, prelude::*,
    winit::WinitPlugin,
};
use std::time::Duration;
use unfolded_tabletop::capture;
pub(crate) use unfolded_tabletop::seat::{
    Admission, Focus, Game, Hit, Logical, Name, Observed, Seat, SeatKey, SeatedAgent, Unadmitted,
    refuse,
};

/// Keys the table reads, which are the keys the seat may press: the protocol name (its
/// `KeyCode` name), the physical key, the logical key a keyboard would send with it,
/// and whether it can commit.
///
/// Enter and Space can commit a choice or dismiss a reading, so a request for one names
/// the control it expects to press. A focused Bevy button activates on Enter or Space,
/// and the table's own keyboard handling may confirm with Enter. Every other key moves
/// focus, the camera, a scroll position or a stepper, or closes without deciding.
pub(crate) static KEYS: [SeatKey; 18] = [
    key(
        "Escape",
        KeyCode::Escape,
        Logical::Named(Key::Escape),
        false,
    ),
    key("Enter", KeyCode::Enter, Logical::Named(Key::Enter), true),
    key("Space", KeyCode::Space, Logical::Named(Key::Space), true),
    key("Tab", KeyCode::Tab, Logical::Named(Key::Tab), false),
    key("Home", KeyCode::Home, Logical::Named(Key::Home), false),
    key("End", KeyCode::End, Logical::Named(Key::End), false),
    key(
        "PageUp",
        KeyCode::PageUp,
        Logical::Named(Key::PageUp),
        false,
    ),
    key(
        "PageDown",
        KeyCode::PageDown,
        Logical::Named(Key::PageDown),
        false,
    ),
    key(
        "ArrowUp",
        KeyCode::ArrowUp,
        Logical::Named(Key::ArrowUp),
        false,
    ),
    key(
        "ArrowDown",
        KeyCode::ArrowDown,
        Logical::Named(Key::ArrowDown),
        false,
    ),
    key(
        "ArrowLeft",
        KeyCode::ArrowLeft,
        Logical::Named(Key::ArrowLeft),
        false,
    ),
    key(
        "ArrowRight",
        KeyCode::ArrowRight,
        Logical::Named(Key::ArrowRight),
        false,
    ),
    key("KeyW", KeyCode::KeyW, Logical::Typed("w"), false),
    key("KeyA", KeyCode::KeyA, Logical::Typed("a"), false),
    key("KeyS", KeyCode::KeyS, Logical::Typed("s"), false),
    key("KeyD", KeyCode::KeyD, Logical::Typed("d"), false),
    key("Minus", KeyCode::Minus, Logical::Typed("-"), false),
    key("Equal", KeyCode::Equal, Logical::Typed("="), false),
];

const fn key(name: &'static str, code: KeyCode, logical: Logical, commits: bool) -> SeatKey {
    SeatKey {
        name,
        code,
        logical,
        commits,
    }
}

/// Admit the table within `limit`, then settle it. An admission still checking at the
/// limit is refused with what blocks it.
pub(crate) fn admit<G: Game>(
    seat: &mut Seat<G>,
    app: &mut App,
    limit: Duration,
) -> Result<(), String> {
    seat.admit(app, limit)
        .map_err(|unadmitted| match unadmitted {
            Unadmitted::Refused(reason) => reason,
            Unadmitted::Unfinished => format!(
                "The seat could not admit play within {} seconds. No game decision was made; saves are unchanged.\n{}",
                limit.as_secs(),
                tabletop_health::admission_blocker(app.world())
            ),
        })
}

/// The held session (#175), under Arkham's own names: the shared listener, launcher
/// and announcement, with the variable that marks a launched Arkham seat and Arkham's
/// idle period.
pub(crate) mod socket {
    use std::{path::Path, time::Duration};
    use unfolded_tabletop::seat::socket as shared;
    pub(crate) use unfolded_tabletop::seat::socket::{Listener, announce};

    /// Set on the process the launcher starts, which serves instead of launching.
    const DETACHED: &str = "ARKHAM_AGENT_SEAT_DETACHED";
    /// A held session without any request for this long ends itself.
    pub(crate) const IDLE: Duration = Duration::from_secs(30 * 60);

    /// Whether this process is the seat a launcher started.
    pub(crate) fn detached() -> bool {
        shared::detached(DETACHED)
    }

    /// Start the seat held on `socket` and wait for its admission.
    pub(crate) fn launch(socket: &Path) -> Result<(), String> {
        shared::launch(socket, DETACHED)
    }

    /// Become the seat a launcher started, and take the listener it handed over.
    pub(crate) fn detach(socket: &Path) -> Result<Listener, String> {
        shared::detach(socket, DETACHED)
    }
}

/// Native plugins without a window event loop: no operating-system window is ever
/// created and the caller steps updates itself. The window settings are the shared
/// ones; Arkham enabled Bevy's event loop, so Arkham leaves it out.
pub(crate) fn windowless(asset: AssetPlugin) -> PluginGroupBuilder {
    DefaultPlugins
        .set(asset)
        .set(capture::no_window())
        .disable::<WinitPlugin>()
}

/// Give the table the shared windowless surface: a logical primary window with
/// nothing behind it and an image for camera `C` to render. The seat steps the
/// presentation clock itself, so native pacing is not installed with it.
pub(crate) fn install<C: Component>(app: &mut App, size: UVec2) {
    capture::install::<C>(app, size);
    app.init_resource::<PresentationTime>()
        .init_resource::<Redraw>();
}

/// The windowless host without a graphics adapter. The protocol is tested through the
/// table that sits at it, in `gathering_tabletop::seat`.
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        camera::RenderTarget, render::render_resource::TextureFormat, window::PrimaryWindow,
    };

    #[derive(Component)]
    struct Seen;

    #[test]
    fn the_windowless_host_has_no_window_event_loop() {
        assert!(!windowless(AssetPlugin::default()).enabled::<WinitPlugin>());
    }

    #[test]
    fn the_seats_image_is_rendered_by_the_named_camera_behind_a_logical_window() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        install::<Seen>(&mut app, UVec2::new(640, 360));
        let other = app.world_mut().spawn(Camera3d::default()).id();
        let seen = app.world_mut().spawn((Camera3d::default(), Seen)).id();
        app.update();

        let Some(RenderTarget::Image(target)) = app.world().get::<RenderTarget>(seen) else {
            panic!("the named camera does not render an image");
        };
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&target.handle)
            .unwrap();
        assert_eq!(image.size(), UVec2::new(640, 360));
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        // UI is drawn to the image only for the default UI camera.
        assert!(
            app.world()
                .get::<bevy::ui::IsDefaultUiCamera>(seen)
                .is_some()
        );
        assert!(!matches!(
            app.world().get::<RenderTarget>(other),
            Some(RenderTarget::Image(_))
        ));

        // Picking ignores every pointer without a primary window.
        let mut windows = app
            .world_mut()
            .query_filtered::<&Window, With<PrimaryWindow>>();
        let window = windows.single(app.world()).unwrap();
        assert_eq!((window.width(), window.height()), (640.0, 360.0));
        assert!(window.focused);

        // The seat steps the presentation clock itself; native pacing is not installed.
        assert!(app.world().contains_resource::<PresentationTime>());
        assert!(app.world().contains_resource::<Redraw>());
    }
}
