//! Agent seat (#153): the game-neutral half. An agent sits at the real table through one
//! JSON request per line on standard input and one JSON response per line on standard
//! output, with no window. This module owns the windowless host, fixed-step stepping,
//! pointer and key synthesis, capture and the protocol loop. It reaches the game only
//! through [`Game`]: produce an observation, resolve a name to a screen position (and a
//! hit back to a name), and report whether the table is settled.
//!
//! Input enters where Bevy's own window-event conversion writes it: `PointerInput` and
//! `KeyboardInput` messages plus the logical window's cursor position. Everything after
//! that is production code. The seat never triggers observers, queues table actions or
//! calls handlers.

use crate::{
    tabletop_health::{self, RenderHealth},
    tabletop_pacing::{PresentationTime, Redraw},
};
use bevy::{
    app::{AppExit, PluginGroupBuilder},
    asset::AssetPlugin,
    camera::RenderTarget,
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    picking::{
        hover::HoverMap,
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    render::{
        render_resource::TextureFormat,
        renderer::RenderAdapterInfo,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    window::{ExitCondition, PrimaryWindow},
    winit::WinitPlugin,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// One step of the presentation clock. The seat, not wall-clock time, advances it.
pub(crate) const STEP: Duration = Duration::from_nanos(16_666_667);
/// Thirty seconds of table time: the opening deal alone presents for about fifteen.
const DEFAULT_SETTLE_STEPS: u32 = 1_800;
const MAX_SETTLE_STEPS: u32 = 7_200;
const CAPTURE_LIMIT: Duration = Duration::from_secs(30);

/// What an agent names. Lookup is exact; a nearest match is never used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Name {
    /// A table component, by the presentation's source identity.
    Component(String),
    /// A choice-bearing control, by its choice identity.
    Choice(String),
}

impl Name {
    fn describe(&self) -> String {
        match self {
            Self::Component(name) => format!("component {name}"),
            Self::Choice(id) => format!("choice {id}"),
        }
    }
}

/// What the pointer is over, named when the game can name it.
pub(crate) struct Hit {
    pub name: Option<Name>,
    pub description: Value,
}

pub(crate) struct Observed {
    /// The player-visible observation, as the headless player driver prints it.
    pub observation: Value,
    /// What the table shows: components, controls, modal state and settling.
    pub table: Value,
}

/// The whole game-facing interface of the seat.
pub(crate) trait Game {
    fn observe(&self, world: &mut World) -> Observed;
    /// Logical screen positions on exactly one shown component or control, as seen
    /// at `revision`: its middle first, then other points where a pointer could rest.
    /// Absent, ambiguous, off-screen and outdated names are errors, never a near match.
    fn resolve(&self, world: &mut World, name: &Name, revision: u64) -> Result<Vec<Vec2>, String>;
    /// The inverse of `resolve` for one picked entity.
    fn identify(&self, world: &mut World, entity: Entity) -> Hit;
    /// The conditions still in progress; empty when the table will accept input.
    fn unsettled(&self, world: &mut World) -> Vec<String>;
}

/// The seat's image and the camera that renders it.
#[derive(Resource)]
pub(crate) struct Surface {
    image: Handle<Image>,
    camera: Option<Entity>,
}

/// Native plugins without a window event loop: no operating-system window is ever
/// created and the caller steps updates itself.
pub(crate) fn windowless(asset: AssetPlugin) -> PluginGroupBuilder {
    DefaultPlugins
        .set(asset)
        .set(WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            ..default()
        })
        .disable::<WinitPlugin>()
}

/// Give the table a logical primary window with nothing behind it and an image for
/// camera `C` to render. Bevy's picking ignores every pointer without a primary window,
/// and draws UI to the image only for the default UI camera.
pub(crate) fn install<C: Component>(app: &mut App, size: UVec2) {
    app.world_mut().spawn((
        Window {
            resolution: (size.x, size.y).into(),
            focused: true,
            ..default()
        },
        PrimaryWindow,
    ));
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            size.x,
            size.y,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    app.insert_resource(Surface {
        image,
        camera: None,
    })
    .init_resource::<PresentationTime>()
    .init_resource::<Redraw>()
    .add_systems(PostStartup, retarget::<C>);
}

fn retarget<C: Component>(
    mut commands: Commands,
    mut surface: ResMut<Surface>,
    camera: Single<Entity, With<C>>,
) {
    commands.entity(*camera).insert((
        RenderTarget::Image(surface.image.clone().into()),
        bevy::ui::IsDefaultUiCamera,
    ));
    surface.camera = Some(*camera);
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Observe {},
    Activate {
        revision: u64,
        #[serde(default)]
        component: Option<String>,
        #[serde(default)]
        choice: Option<String>,
    },
    Key {
        key: String,
    },
    Settle {
        #[serde(default)]
        steps: Option<u32>,
    },
    Capture {
        path: PathBuf,
    },
    Quit {},
}

/// Present while an agent sits at the table. The agent operates the table, never the
/// owner's desktop, so a game refuses what native play hands to the desktop, such as
/// an address for the desktop's opener.
#[derive(Resource)]
pub(crate) struct SeatedAgent;

pub(crate) struct Seat<G> {
    game: G,
    camera: Entity,
    image: Option<Handle<Image>>,
    exit: Option<AppExit>,
}

impl<G: Game> Seat<G> {
    /// Sit at a table whose camera already has a render target. Without an image
    /// (the no-GPU test host) every operation but `capture` is available.
    pub(crate) fn new(
        game: G,
        app: &mut App,
        camera: Entity,
        image: Option<Handle<Image>>,
    ) -> Self {
        app.insert_resource(SeatedAgent);
        Self {
            game,
            camera,
            image,
            exit: None,
        }
    }

    /// Finish building a table prepared by [`install`] and sit at it. No runner ever
    /// starts: from here the seat alone steps updates.
    pub(crate) fn open(game: G, app: &mut App) -> Result<Self, String> {
        app.finish();
        app.cleanup();
        app.update();
        let surface = app.world().resource::<Surface>();
        let camera = surface
            .camera
            .ok_or("The table spawned no camera for the seat's image.")?;
        let image = surface.image.clone();
        Ok(Self::new(game, app, camera, Some(image)))
    }

    /// Advance the presentation clock by `advance` and run one update. Returns
    /// false once the table has asked to exit.
    fn step(&mut self, app: &mut App, advance: Duration) -> bool {
        if self.exit.is_some() {
            return false;
        }
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(advance);
        app.update();
        self.exit = app.should_exit();
        self.exit.is_none()
    }

    /// Step in wall-clock time until seat admission, which waits on asynchronous
    /// loading, then until the admitted table settles.
    pub(crate) fn admit(&mut self, app: &mut App, limit: Duration) -> Result<(), String> {
        let deadline = Instant::now() + limit;
        loop {
            if !self.step(app, STEP) {
                return Err(self.exit_reason(app));
            }
            let health = app.world().resource::<RenderHealth>();
            if health.allows_play() {
                break;
            }
            if !health.is_checking() {
                return Err(health.status().to_owned());
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "The seat could not admit play within {} seconds. No game decision was made; saves are unchanged.\n{}",
                    limit.as_secs(),
                    tabletop_health::admission_blocker(app.world())
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        for _ in 0..DEFAULT_SETTLE_STEPS {
            if self.game.unsettled(app.world_mut()).is_empty() {
                break;
            }
            if !self.step(app, STEP) {
                return Err(self.exit_reason(app));
            }
        }
        Ok(())
    }

    fn exit_reason(&self, app: &App) -> String {
        let health = app.world().resource::<RenderHealth>();
        if health.allows_play() || health.is_checking() {
            format!("The table exited ({:?}).", self.exit)
        } else {
            health.status().to_owned()
        }
    }

    /// Serve requests until `quit`, end of input or the table's own exit.
    pub(crate) fn serve(
        &mut self,
        app: &mut App,
        input: impl BufRead,
        mut output: impl Write,
    ) -> Result<(), String> {
        let mut ready = self.response(app, "ready", None);
        ready["adapter"] = adapter(app.world());
        emit(&mut output, &ready)?;
        for line in input.lines() {
            let line = line.map_err(|error| error.to_string())?;
            if line.trim().is_empty() {
                continue;
            }
            let (response, proceed) = self.respond(app, &line);
            emit(&mut output, &response)?;
            if !proceed {
                break;
            }
        }
        match &self.exit {
            Some(exit) if exit.is_error() => Err(self.exit_reason(app)),
            _ => Ok(()),
        }
    }

    /// One request, one response. The flag is false when the session has ended.
    pub(crate) fn respond(&mut self, app: &mut App, line: &str) -> (Value, bool) {
        let request = match serde_json::from_str::<Request>(line) {
            Ok(request) => request,
            Err(error) => return (self.refused(app, error.to_string()), true),
        };
        let response = match request {
            Request::Observe {} => self.response(app, "observed", None),
            Request::Activate {
                revision,
                component,
                choice,
            } => match (component, choice) {
                (Some(name), None) => self.activate(app, Name::Component(name), revision),
                (None, Some(id)) => self.activate(app, Name::Choice(id), revision),
                _ => self.refused(
                    app,
                    "activate needs exactly one of component or choice".into(),
                ),
            },
            Request::Key { key } => self.key(app, &key),
            Request::Settle { steps } => self.settle(app, steps),
            Request::Capture { path } => self.capture(app, &path),
            Request::Quit {} => return (self.response(app, "bye", None), false),
        };
        if let Some(exit) = &self.exit {
            // The table ended the session itself, for example by returning to selection.
            let error = exit.is_error().then(|| self.exit_reason(app));
            return (self.response(app, "exited", error), false);
        }
        (response, true)
    }

    fn response(&self, app: &mut App, status: &str, error: Option<String>) -> Value {
        let Observed { observation, table } = self.game.observe(app.world_mut());
        let mut response = json!({
            "ok": error.is_none(), "mode": "seat", "status": status,
            "observation": observation, "table": table,
        });
        if let Some(error) = error {
            response["error"] = json!(error);
        }
        response
    }

    fn refused(&self, app: &mut App, error: String) -> Value {
        self.response(app, "refused", Some(error))
    }

    fn activate(&mut self, app: &mut App, name: Name, revision: u64) -> Value {
        let pending = self.game.unsettled(app.world_mut());
        if !pending.is_empty() {
            return self.busy(app, &pending);
        }
        let candidates = match self.game.resolve(app.world_mut(), &name, revision) {
            Ok(candidates) => candidates,
            Err(error) => return self.refused(app, error),
        };
        // Rest the pointer on the named target as a player would: on a part of it
        // that nothing covers. One update per point establishes real hover.
        let mut covered = None;
        let mut reached = None;
        for position in candidates {
            if let Err(error) =
                self.pointer(app, position, PointerAction::Move { delta: Vec2::ZERO })
            {
                return self.refused(app, error);
            }
            self.step(app, STEP);
            let hovered = self.under_pointer(app);
            if hovered.iter().any(|hit| hit.name.as_ref() == Some(&name)) {
                reached = Some(position);
                break;
            }
            covered.get_or_insert((position, hovered));
        }
        let Some(position) = reached else {
            // Pressing here would act on whatever covers the named target.
            let Some((position, hovered)) = covered else {
                return self.refused(app, format!("{} has no screen position.", name.describe()));
            };
            return self.missed(app, &name, position, hovered, false);
        };
        for action in [
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            let pressing = matches!(action, PointerAction::Press(_));
            if let Err(error) = self.pointer(app, position, action) {
                return self.refused(app, error);
            }
            self.step(app, STEP);
            if pressing {
                let pressed = self.under_pointer(app);
                if !pressed.iter().any(|hit| hit.name.as_ref() == Some(&name)) {
                    // Something moved under the pointer after hover. A release here
                    // would click it, so cancel the press before letting go.
                    for undo in [
                        PointerAction::Cancel,
                        PointerAction::Release(PointerButton::Primary),
                    ] {
                        let _ = self.pointer(app, position, undo);
                        self.step(app, STEP);
                    }
                    return self.missed(app, &name, position, pressed, true);
                }
            }
        }
        let mut response = self.response(app, "activated", None);
        response["position"] = json!([position.x, position.y]);
        if let Some((tried, hits)) = covered {
            // Pressed at a later point: say what the pointer found at the first one,
            // so a partly covered target is reported, not hidden by the success.
            response["covered"] = json!({
                "position": [tried.x, tried.y],
                "hit": hits.into_iter().map(|hit| hit.description).collect::<Vec<_>>(),
            });
        }
        response
    }

    fn busy(&self, app: &mut App, pending: &[String]) -> Value {
        // The table drops input while presenting; never let it vanish silently.
        self.refused(
            app,
            format!("The table is busy ({}); settle first.", pending.join(", ")),
        )
    }

    fn missed(
        &self,
        app: &mut App,
        name: &Name,
        position: Vec2,
        hits: Vec<Hit>,
        pressed: bool,
    ) -> Value {
        let hit: Vec<Value> = hits.into_iter().map(|hit| hit.description).collect();
        let over = if hit.is_empty() {
            "nothing".to_owned()
        } else {
            hit.iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        };
        let error = if pressed {
            format!(
                "The press at ({:.0}, {:.0}) landed on {over}, not {}.",
                position.x,
                position.y,
                name.describe()
            )
        } else {
            format!(
                "The pointer at ({:.0}, {:.0}) is over {over}, not {}. Nothing was pressed.",
                position.x,
                position.y,
                name.describe()
            )
        };
        let mut response = self.response(app, "missed", Some(error));
        response["hit"] = json!(hit);
        response["pressed"] = json!(pressed);
        response["position"] = json!([position.x, position.y]);
        response
    }

    /// Everything under the pointer after the last update, nearest first.
    fn under_pointer(&self, app: &mut App) -> Vec<Hit> {
        let mut hovered: Vec<(Entity, f32)> = app
            .world()
            .get_resource::<HoverMap>()
            .and_then(|hover| hover.get(&PointerId::Mouse))
            .map(|hits| {
                hits.iter()
                    .map(|(entity, hit)| (*entity, hit.depth))
                    .collect()
            })
            .unwrap_or_default();
        hovered.sort_by(|a, b| a.1.total_cmp(&b.1));
        hovered
            .into_iter()
            .map(|(entity, _)| self.game.identify(app.world_mut(), entity))
            .collect()
    }

    /// The same message Bevy's window-event conversion writes, aimed at the camera's
    /// own render target, plus the logical window's cursor position.
    fn pointer(&self, app: &mut App, position: Vec2, action: PointerAction) -> Result<(), String> {
        let world = app.world_mut();
        let (window, mut cursor) = world
            .query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>()
            .single_mut(world)
            .map_err(|_| "The seat has no logical primary window.")?;
        cursor.set_cursor_position(Some(position));
        let target = world
            .get::<RenderTarget>(self.camera)
            .and_then(|target| target.normalize(Some(window)))
            .ok_or("The seat's camera has no render target.")?;
        world.write_message(PointerInput::new(
            PointerId::Mouse,
            Location { target, position },
            action,
        ));
        Ok(())
    }

    fn key(&mut self, app: &mut App, name: &str) -> Value {
        let Some((code, logical)) = key_named(name) else {
            let known: Vec<&str> = KEYS.iter().map(|(known, ..)| *known).collect();
            return self.refused(
                app,
                format!("Unknown key {name:?}. Known keys: {}.", known.join(", ")),
            );
        };
        let pending = self.game.unsettled(app.world_mut());
        if !pending.is_empty() {
            return self.busy(app, &pending);
        }
        let world = app.world_mut();
        let Ok(window) = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(world)
        else {
            return self.refused(app, "The seat has no logical primary window.".into());
        };
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().write_message(KeyboardInput {
                key_code: code,
                logical_key: logical.clone(),
                state,
                text: None,
                repeat: false,
                window,
            });
            self.step(app, STEP);
        }
        self.response(app, "pressed", None)
    }

    fn settle(&mut self, app: &mut App, steps: Option<u32>) -> Value {
        let limit = steps.unwrap_or(DEFAULT_SETTLE_STEPS).min(MAX_SETTLE_STEPS);
        let mut used = 0;
        loop {
            let pending = self.game.unsettled(app.world_mut());
            let done = pending.is_empty();
            if done || used == limit || self.exit.is_some() {
                let mut response = if done {
                    self.response(app, "settled", None)
                } else {
                    self.response(
                        app,
                        "unsettled",
                        Some(format!(
                            "Still in progress after {used} steps: {}.",
                            pending.join(", ")
                        )),
                    )
                };
                response["steps"] = json!(used);
                return response;
            }
            self.step(app, STEP);
            used += 1;
        }
    }

    /// Write a picture of the seat's image. The clock is frozen while the capture is
    /// read back, and the response follows the completed file.
    fn capture(&mut self, app: &mut App, path: &Path) -> Value {
        let Some(image) = self.image.clone() else {
            return self.refused(
                app,
                "This host renders no image; capture needs the windowless seat.".into(),
            );
        };
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            return self.refused(app, format!("Cannot create {}: {error}", parent.display()));
        }
        let deadline = Instant::now() + CAPTURE_LIMIT;
        // Frozen updates draw anything the last request changed. The readiness flag
        // trails the renderer by a frame, so require it on two consecutive frames.
        let mut ready = 0;
        while ready < 2 {
            if Instant::now() >= deadline || !self.step(app, Duration::ZERO) {
                return self.refused(app, "Render pipelines did not become ready.".into());
            }
            if tabletop_health::pipelines_ready(app.world()) {
                ready += 1;
            } else {
                ready = 0;
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        let written: Arc<Mutex<Option<Result<(), String>>>> = Arc::default();
        let slot = written.clone();
        let target = path.to_owned();
        app.world_mut().spawn(Screenshot::image(image)).observe(
            move |captured: On<ScreenshotCaptured>| {
                *slot.lock().unwrap() = Some(crate::tabletop_capture::write_image(
                    captured.image.clone(),
                    &target,
                ));
            },
        );
        loop {
            let proceed = self.step(app, Duration::ZERO);
            if let Some(result) = written.lock().unwrap().take() {
                return match result {
                    Ok(()) => {
                        let mut response = self.response(app, "captured", None);
                        response["path"] = json!(path);
                        response
                    }
                    Err(error) => self.refused(app, format!("Cannot save the capture: {error}")),
                };
            }
            if !proceed || Instant::now() >= deadline {
                return self.refused(
                    app,
                    format!(
                        "The capture did not complete within {} seconds; no file was written.",
                        CAPTURE_LIMIT.as_secs()
                    ),
                );
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn emit(output: &mut impl Write, value: &Value) -> Result<(), String> {
    serde_json::to_writer(&mut *output, value).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())
}

/// The graphics adapter that rendered the seat's image, when a renderer exists.
fn adapter(world: &World) -> Value {
    world
        .get_resource::<RenderAdapterInfo>()
        .map_or(Value::Null, |info| {
            json!({
                "name": info.name,
                "backend": format!("{:?}", info.backend),
                "device_type": format!("{:?}", info.device_type),
                "driver": info.driver,
                "driver_info": info.driver_info,
            })
        })
}

enum Logical {
    Named(Key),
    Typed(&'static str),
}

/// Keys the table reads: the protocol name (its `KeyCode` name), the physical key and
/// the logical key a keyboard would send with it.
static KEYS: [(&str, KeyCode, Logical); 18] = [
    ("Escape", KeyCode::Escape, Logical::Named(Key::Escape)),
    ("Enter", KeyCode::Enter, Logical::Named(Key::Enter)),
    ("Space", KeyCode::Space, Logical::Named(Key::Space)),
    ("Tab", KeyCode::Tab, Logical::Named(Key::Tab)),
    ("Home", KeyCode::Home, Logical::Named(Key::Home)),
    ("End", KeyCode::End, Logical::Named(Key::End)),
    ("PageUp", KeyCode::PageUp, Logical::Named(Key::PageUp)),
    ("PageDown", KeyCode::PageDown, Logical::Named(Key::PageDown)),
    ("ArrowUp", KeyCode::ArrowUp, Logical::Named(Key::ArrowUp)),
    (
        "ArrowDown",
        KeyCode::ArrowDown,
        Logical::Named(Key::ArrowDown),
    ),
    (
        "ArrowLeft",
        KeyCode::ArrowLeft,
        Logical::Named(Key::ArrowLeft),
    ),
    (
        "ArrowRight",
        KeyCode::ArrowRight,
        Logical::Named(Key::ArrowRight),
    ),
    ("KeyW", KeyCode::KeyW, Logical::Typed("w")),
    ("KeyA", KeyCode::KeyA, Logical::Typed("a")),
    ("KeyS", KeyCode::KeyS, Logical::Typed("s")),
    ("KeyD", KeyCode::KeyD, Logical::Typed("d")),
    ("Minus", KeyCode::Minus, Logical::Typed("-")),
    ("Equal", KeyCode::Equal, Logical::Typed("=")),
];

fn key_named(name: &str) -> Option<(KeyCode, Key)> {
    KEYS.iter()
        .find(|(known, ..)| *known == name)
        .map(|(_, code, logical)| {
            (
                *code,
                match logical {
                    Logical::Named(key) => key.clone(),
                    Logical::Typed(text) => Key::Character((*text).into()),
                },
            )
        })
}
