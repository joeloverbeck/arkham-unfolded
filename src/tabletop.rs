//! The replaceable tabletop for the single, explicitly scripted rules experiment.
//! Every displayed decision and replay moment comes from the shared rules interface.

use crate::{
    Fixture, Observation, Session, Stage, TokenFace, TokenStatus,
    asset_config::AssetConfig,
    assets::{self, CollectionReport},
    continuity::{self, Replay},
    explanation::explain_result,
};
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::hover::Hovered,
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot, save_to_disk},
    ui::Pressed,
    ui_widgets::{Activate, Button, ModalDialog, ModalDialogBarrier, RequestClose},
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(crate) mod asset_settings;
mod inspection;
mod scene;
mod standees;
mod symbols;
use asset_settings::AssetSettings;
use symbols::Symbol;

const PANEL: f32 = 390.0;
const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.62, 0.71, 0.70);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);
const TEAL: Color = Color::srgb(0.13, 0.37, 0.36);
const BUTTON: Color = Color::srgb(0.12, 0.19, 0.21);

struct Card {
    code: &'static str,
    title: &'static str,
    kind: &'static str,
    text: &'static str,
    color: Color,
    landscape: bool,
}

const CARDS: [Card; 3] = [
    Card {
        code: "01163",
        title: "Rotting Remains",
        kind: "Treachery · The Gathering",
        text: "Revelation — Test willpower (3). For each point you fail by, take 1 horror.",
        color: Color::srgb(0.40, 0.22, 0.18),
        landscape: false,
    },
    Card {
        code: "07032",
        title: "Promise of Power",
        kind: "Skill · Four wild icons · Level 0",
        text: "After you commit Promise of Power to a skill test, add 1 curse token to the chaos bag. If you cannot, take 2 horror instead.",
        color: Color::srgb(0.29, 0.20, 0.42),
        landscape: false,
    },
    Card {
        code: "60401",
        title: "Jacqueline Fine",
        kind: "Investigator · Willpower 5",
        text: "Reaction — When an investigator at your location would reveal any number of chaos tokens: Reveal 2 additional tokens. Of the revealed tokens, choose and cancel 2 non-auto-fail tokens, or 1 auto-fail token. Limit once per round.\n\nElder sign: +1. If this effect is canceled or ignored, draw 1 card.\n\nThis experiment has no elder sign reveal or other replacement assets.",
        color: Color::srgb(0.30, 0.24, 0.48),
        landscape: true,
    },
];

#[derive(Resource)]
struct Tabletop {
    session: Session,
    fixture: Fixture,
    replay: Option<Replay>,
    save_path: PathBuf,
    notice: String,
    inspect: Option<usize>,
    history: bool,
    result_open: bool,
    dirty: bool,
}

impl Tabletop {
    fn view(&self) -> Observation {
        self.replay
            .as_ref()
            .map_or_else(|| self.session.observe(), |r| r.current().clone())
    }
}

fn has_resolved_curse(view: &Observation) -> bool {
    view.tokens
        .iter()
        .any(|token| token.face == TokenFace::Curse && token.status == TokenStatus::Resolved)
}

fn result_opens_automatically(view: &Observation) -> bool {
    // Leave the mandatory extra reveal visible until the player requests the result.
    view.stage == Stage::Finished && !has_resolved_curse(view)
}

#[derive(Clone)]
enum Action {
    Choose(u64, String),
    Save,
    Reopen,
    Replay,
    Back,
    Next,
    Live,
    New,
    Inspect(usize),
    CloseInspect,
    History,
    Follow,
    ShowResult,
    DismissResult,
    AssetFolder,
}

#[derive(Resource, Default)]
struct Actions(Vec<Action>);

#[derive(Resource)]
struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
    desired_distance: f32,
    target: Vec3,
    desired_target: Vec3,
    follow: bool,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.88,
            distance: 13.8,
            desired_distance: 13.8,
            target: Vec3::new(1.9, 0.0, 0.0),
            desired_target: Vec3::new(1.9, 0.0, 0.0),
            follow: false,
        }
    }
}

#[derive(Resource)]
struct Art {
    handles: [Option<Handle<Image>>; 3],
    available: usize,
    token_handles: Vec<(TokenFace, Handle<Image>)>,
    reminder: Option<Handle<Image>>,
    investigators: Vec<(String, Handle<Image>)>,
    symbol_font: Option<Handle<Font>>,
    status: String,
}

#[derive(Resource)]
struct Capture {
    path: Option<PathBuf>,
    quit: bool,
    frame: u32,
    requested: bool,
}

#[derive(Component)]
struct MainCamera;
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct ButtonTone(Color);

struct Options {
    fixture: Fixture,
    save: PathBuf,
    load: Option<PathBuf>,
    art_root: Option<PathBuf>,
    config: AssetConfig,
    screenshot: Option<PathBuf>,
    quit_after_screenshot: bool,
}

impl Options {
    fn parse() -> Result<Option<Self>, String> {
        let config = AssetConfig::load()?;
        let mut options = Self {
            fixture: Fixture::curse(),
            save: "local/experiment/curse.session.json".into(),
            load: None,
            art_root: Some(config.root().to_path_buf()),
            config,
            screenshot: None,
            quit_after_screenshot: false,
        };
        let mut explicit_save = false;
        let mut selected_case = "curse";
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--case" => {
                    options.fixture = match value()?.as_str() {
                        "curse" => {
                            selected_case = "curse";
                            Fixture::curse()
                        }
                        "autofail" => {
                            selected_case = "autofail";
                            Fixture::auto_fail()
                        }
                        _ => return Err("--case must be curse or autofail".into()),
                    }
                }
                "--save" => {
                    options.save = value()?.into();
                    explicit_save = true;
                }
                "--load" => options.load = Some(value()?.into()),
                "--art-root" => options.art_root = Some(value()?.into()),
                "--no-art" => options.art_root = None,
                "--screenshot" => options.screenshot = Some(value()?.into()),
                "--quit-after-screenshot" => options.quit_after_screenshot = true,
                "--help" | "-h" => {
                    println!(
                        "Arkham Unfolded — scripted rules experiment\n\n--case curse|autofail  Choose the privileged starting fixture\n--save PATH            Session save and autosave path\n--load PATH            Reopen a saved pending decision\n--art-root PATH        Use a prepared asset collection for this launch\n--no-art               Use readable substitutes\n--screenshot PATH      Capture the actual window once ready\n--quit-after-screenshot\n\nCamera: right drag or arrow keys; wheel or +/- to zoom; Home to reset. F12 saves a screenshot."
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown option: {arg}")),
            }
        }
        if !explicit_save {
            options.save = format!("local/experiment/{selected_case}.session.json").into();
        }
        if options.quit_after_screenshot && options.screenshot.is_none() {
            return Err("--quit-after-screenshot requires --screenshot PATH".into());
        }
        Ok(Some(options))
    }
}

/// Launch the actual native consumer. Errors preserve existing saves.
pub fn run() -> Result<(), String> {
    let Some(options) = Options::parse()? else {
        return Ok(());
    };
    let load_path = options
        .load
        .or_else(|| options.save.is_file().then(|| options.save.clone()));
    let resumed = load_path.is_some();
    let session = if let Some(path) = &load_path {
        continuity::load_session(path).map_err(|e| format!("Could not reopen the session: {e}"))?
    } else {
        Session::new(options.fixture.clone())
    };
    let save_path = load_path.unwrap_or(options.save);
    let art_root = options.art_root.map(|path| {
        if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    });
    let collection = art_root.as_deref().map(assets::inspect);
    let status = collection.as_ref().map_or_else(
        || "Artwork disabled for this launch; readable substitutes are active.".into(),
        CollectionReport::summary,
    );
    eprintln!("{status}");
    if let Some(report) = &collection {
        for diagnostic in &report.diagnostics {
            eprintln!("{diagnostic}");
        }
    }
    let notice = if let Some(warning) = options.config.warning() {
        format!("Asset settings: {warning}")
    } else if resumed {
        "Saved experiment reopened at its exact pending decision.".into()
    } else {
        "Choose freely. The rules handle the consequences.".into()
    };
    let asset_root = art_root.as_ref().map_or_else(
        || "local/experiment/absent-art".to_string(),
        |p| p.to_string_lossy().into_owned(),
    );
    let mut app = App::new();
    app.insert_resource(ClearColor(INK))
        .insert_resource(Tabletop {
            fixture: session.fixture.clone(),
            result_open: result_opens_automatically(&session.observe()),
            session,
            replay: None,
            save_path,
            notice,
            inspect: None,
            history: false,
            dirty: true,
        })
        .insert_resource(Orbit::default())
        .insert_resource(Actions::default())
        .insert_resource(AssetSettings::new(options.config))
        .insert_resource(Capture {
            path: options.screenshot,
            quit: options.quit_after_screenshot,
            frame: 0,
            requested: false,
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Arkham Unfolded — Scripted Rules Experiment".into(),
                        resolution: (1600, 1000).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root,
                    ..default()
                }),
        )
        .add_plugins(MeshPickingPlugin)
        .add_systems(Startup, setup_table)
        .add_systems(
            Update,
            (
                mark_window_resized,
                poll_asset_settings,
                handle_actions,
                camera_controls,
                rebuild_panel,
                button_colors,
                capture_window,
            )
                .chain(),
        );
    let server = app.world().resource::<AssetServer>();
    let handles = std::array::from_fn(|i| {
        prepared_image(
            server,
            collection.as_ref(),
            &format!("card/{}", CARDS[i].code),
        )
    });
    let available = handles.iter().filter(|h| h.is_some()).count();
    let token_handles = [
        (TokenFace::Curse, "curse"),
        (TokenFace::MinusOne, "minus_one"),
        (TokenFace::Zero, "zero"),
        (TokenFace::MinusTwo, "minus_two"),
        (TokenFace::AutoFail, "auto_fail"),
    ]
    .into_iter()
    .filter_map(|(face, name)| {
        prepared_image(server, collection.as_ref(), &format!("token/{name}"))
            .map(|handle| (face, handle))
    })
    .collect();
    let reminder = prepared_image(server, collection.as_ref(), "marker/reaction");
    let investigators = [
        ("Tony Morgan", "06003"),
        ("Patrice Hathaway", "06005"),
        ("Jacqueline Fine", "60401"),
    ]
    .into_iter()
    .filter_map(|(name, code)| {
        prepared_image(server, collection.as_ref(), &format!("standee/{code}"))
            .map(|handle| (name.to_owned(), handle))
    })
    .collect();
    let symbol_font = symbols::load(
        art_root.as_deref(),
        collection.as_ref(),
        &mut app.world_mut().resource_mut::<Assets<Font>>(),
    );
    app.insert_resource(Art {
        handles,
        available,
        token_handles,
        reminder,
        investigators,
        symbol_font,
        status,
    });
    scene::install(&mut app);
    app.run();
    Ok(())
}

fn prepared_image(
    server: &AssetServer,
    collection: Option<&CollectionReport>,
    role: &str,
) -> Option<Handle<Image>> {
    Some(server.load(collection?.verified_path(role)?.to_owned()))
}

fn poll_asset_settings(mut settings: ResMut<AssetSettings>, mut table: ResMut<Tabletop>) {
    if let Some(notice) = settings.poll() {
        eprintln!("{notice}");
        eprintln!(
            "Configured asset folder: {}",
            settings.configured_root().display()
        );
        table.notice = notice;
        table.dirty = true;
    }
}

fn setup_table(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    art: Res<Art>,
) {
    let velvet = materials.add(StandardMaterial {
        base_color: Color::srgb(0.035, 0.16, 0.15),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.4, 0.3, 8.2))),
        MeshMaterial3d(velvet),
        Transform::from_xyz(0.0, -0.2, 0.0),
        Pickable::IGNORE,
    ));
    let brass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.31, 0.13),
        metallic: 0.7,
        perceptual_roughness: 0.5,
        ..default()
    });
    for (size, at) in [
        (Vec3::new(12.6, 0.15, 0.12), Vec3::new(0.0, -0.01, -4.1)),
        (Vec3::new(12.6, 0.15, 0.12), Vec3::new(0.0, -0.01, 4.1)),
        (Vec3::new(0.12, 0.15, 8.2), Vec3::new(-6.2, -0.01, 0.0)),
        (Vec3::new(0.12, 0.15, 8.2), Vec3::new(6.2, -0.01, 0.0)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(brass.clone()),
            Transform::from_translation(at),
            Pickable::IGNORE,
        ));
    }
    let zone = materials.add(Color::srgb(0.065, 0.22, 0.21));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.6, 0.035, 2.45))),
        MeshMaterial3d(zone),
        Transform::from_xyz(0.0, -0.02, -2.4),
        Pickable::IGNORE,
    ));
    let bag = materials.add(Color::srgb(0.18, 0.10, 0.16));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.65, 0.9))),
        MeshMaterial3d(bag),
        Transform::from_xyz(-4.5, 0.48, 1.75),
        Pickable::IGNORE,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.46, 0.025))),
        MeshMaterial3d(materials.add(Color::BLACK)),
        Transform::from_xyz(-4.5, 0.95, 1.75),
        Pickable::IGNORE,
    ));
    for (i, card) in CARDS.iter().enumerate() {
        let x = -2.0 + i as f32 * 2.4;
        // The source stores Jacqueline sideways in a portrait-shaped image.
        // Rotating the original face preserves both its artwork and proportions.
        let rotation = Quat::from_rotation_y(if card.landscape {
            std::f32::consts::FRAC_PI_2
        } else {
            0.0
        });
        let material = materials.add(StandardMaterial {
            base_color: if art.handles[i].is_some() {
                Color::WHITE
            } else {
                card.color
            },
            base_color_texture: art.handles[i].clone(),
            perceptual_roughness: 0.9,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.85, 0.07, 2.60))),
            MeshMaterial3d(materials.add(Color::srgb(0.14, 0.12, 0.10))),
            Transform::from_xyz(x, 0.08, 1.9).with_rotation(rotation),
            Pickable::IGNORE,
        ));
        commands
            .spawn((
                Mesh3d(meshes.add(Plane3d::default().mesh().size(1.80, 2.52))),
                MeshMaterial3d(material),
                Transform::from_xyz(x, 0.125, 1.9).with_rotation(rotation),
            ))
            .observe(move |_: On<PointerClick>, mut queue: ResMut<Actions>| {
                queue.0.push(Action::Inspect(i))
            });
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 7_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.0)),
    ));
    commands.spawn((
        PointLight {
            intensity: 250_000.0,
            color: Color::srgb(1.0, 0.76, 0.48),
            ..default()
        },
        Transform::from_xyz(-5.0, 6.0, 4.0),
    ));
    commands.spawn((Camera3d::default(), Transform::default(), MainCamera));
}

fn handle_actions(
    mut queue: ResMut<Actions>,
    mut table: ResMut<Tabletop>,
    mut orbit: ResMut<Orbit>,
    mut settings: Option<ResMut<AssetSettings>>,
) {
    for action in queue.0.drain(..) {
        if settings.as_ref().is_some_and(|settings| settings.is_busy()) {
            continue;
        }
        // A modal consumes interaction with the underlying table, including queued clicks.
        if (table.result_open && !matches!(action, Action::DismissResult))
            || (table.inspect.is_some() && !matches!(action, Action::CloseInspect))
        {
            continue;
        }
        match action {
            Action::Choose(revision, id) if table.replay.is_none() => {
                let table = &mut *table;
                match continuity::commit_choice(&table.save_path, &mut table.session, revision, &id)
                {
                    Ok(()) => {
                        table.result_open = result_opens_automatically(&table.session.observe());
                        table.notice = "Decision saved. You can close and reopen safely.".into();
                    }
                    Err(error) => table.notice = format!("Decision was not committed: {error}"),
                }
            }
            Action::Save => {
                table.notice = match continuity::save_session(&table.save_path, &table.session) {
                    Ok(()) => "Live session saved at its current decision.".into(),
                    Err(e) => format!("Save failed; the current session is unchanged: {e}"),
                }
            }
            Action::Reopen => match continuity::load_session(&table.save_path) {
                Ok(session) => {
                    table.fixture = session.fixture.clone();
                    table.result_open = result_opens_automatically(&session.observe());
                    table.session = session;
                    table.replay = None;
                    orbit.follow = false;
                    table.notice = "Saved session reopened. The pending choice and revealed tokens are preserved.".into();
                }
                Err(e) => {
                    table.notice =
                        format!("Could not reopen; the current session is unchanged: {e}")
                }
            },
            Action::Replay => {
                let recording = table.session.recording();
                let digest = format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(&recording).expect("recordings serialize"))
                );
                let stem = table
                    .save_path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("experiment");
                let path = table
                    .save_path
                    .with_file_name(format!("{stem}.replay-{}.json", &digest[..16]));
                match continuity::save_recording(&path, &recording)
                    .and_then(|()| continuity::load_recording(&path))
                    .and_then(Replay::new)
                {
                    Ok(replay) => {
                        table.replay = Some(replay);
                        table.result_open = false;
                        orbit.follow = true;
                        table.notice = "Replay reopened from its saved record. Next advances one moment and pauses.".into();
                    }
                    Err(e) => table.notice = format!("Could not reopen the replay: {e}"),
                }
            }
            Action::Back => {
                if let Some(replay) = &mut table.replay
                    && replay.back()
                {
                    table.result_open = result_opens_automatically(replay.current());
                }
            }
            Action::Next => {
                if let Some(replay) = &mut table.replay
                    && replay.next()
                {
                    table.result_open = result_opens_automatically(replay.current());
                }
            }
            Action::Live => {
                table.replay = None;
                table.result_open = result_opens_automatically(&table.session.observe());
                orbit.follow = false;
                table.notice = "Returned to the live session; replay made no changes to it.".into();
            }
            Action::New => {
                let session = Session::new(table.fixture.clone());
                match continuity::restart_session(&table.save_path, &session) {
                    Ok(()) => {
                        table.session = session;
                        table.replay = None;
                        table.result_open = false;
                        orbit.follow = false;
                        table.notice =
                            "New example ready. The previous session has been retained.".into();
                    }
                    Err(e) => {
                        table.notice = format!(
                            "Could not start another example; the current one is unchanged: {e}"
                        )
                    }
                }
            }
            Action::Inspect(i) => {
                table.inspect = Some(i);
                table.history = false;
            }
            Action::CloseInspect => table.inspect = None,
            Action::History => {
                table.history = !table.history;
                table.inspect = None;
            }
            Action::Follow => {
                if table.replay.is_some() {
                    orbit.follow = !orbit.follow;
                }
            }
            Action::ShowResult => table.result_open = table.view().stage == Stage::Finished,
            Action::DismissResult => table.result_open = false,
            Action::AssetFolder => {
                if let Some(settings) = &mut settings {
                    settings.choose();
                    table.notice = "Choose a prepared asset folder in the file dialog. The current game stays unchanged.".into();
                }
            }
            Action::Choose(_, _) => {}
        }
        if !orbit.follow {
            orbit.desired_target = orbit.target;
            orbit.desired_distance = orbit.distance;
        }
        if orbit.follow && table.replay.is_some() {
            let (target, distance) = match table.view().stage {
                Stage::Commit => (Vec3::new(1.5, 0.2, -1.5), 13.8),
                Stage::BeforeReveal => (Vec3::new(1.6, 0.0, 0.5), 13.0),
                Stage::Cancellation => (Vec3::new(1.6, 0.0, 0.2), 11.7),
                Stage::Finished | Stage::Ended => (Vec3::new(1.7, 0.2, -1.2), 13.2),
            };
            orbit.desired_target = target;
            orbit.desired_distance = distance;
        }
        table.dirty = true;
    }
}

fn mark_window_resized(
    windows: Query<&Window>,
    mut previous_size: Local<Option<Vec2>>,
    mut table: ResMut<Tabletop>,
) {
    if let Some(window) = windows.iter().next() {
        let size = Vec2::new(window.width(), window.height());
        if previous_size.as_ref() != Some(&size) {
            *previous_size = Some(size);
            table.dirty = true;
        }
    }
}

// Bevy injects each input, resource and query independently into this system.
#[allow(clippy::too_many_arguments)]
fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window>,
    mut table: ResMut<Tabletop>,
    time: Res<Time>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    settings: Option<Res<AssetSettings>>,
) {
    let modal_open = table.result_open
        || table.inspect.is_some()
        || settings.as_ref().is_some_and(|settings| settings.is_busy());
    let scene_pointer = !modal_open
        && !table.history
        && window
            .cursor_position()
            .is_some_and(|p| p.x < window.width() - PANEL);
    let was_following = orbit.follow;
    let mut changed = false;
    if buttons.pressed(MouseButton::Right) && scene_pointer && motion.delta != Vec2::ZERO {
        orbit.yaw -= motion.delta.x * 0.006;
        orbit.pitch += motion.delta.y * 0.004;
        changed = true;
    }
    if scene_pointer && scroll.delta.y != 0.0 {
        orbit.distance -= scroll.delta.y * 0.6;
        changed = true;
    }
    for (key, yaw, pitch, zoom) in [
        (KeyCode::ArrowLeft, 1.0, 0.0, 0.0),
        (KeyCode::ArrowRight, -1.0, 0.0, 0.0),
        (KeyCode::ArrowUp, 0.0, 0.7, 0.0),
        (KeyCode::ArrowDown, 0.0, -0.7, 0.0),
        (KeyCode::Equal, 0.0, 0.0, -5.0),
        (KeyCode::Minus, 0.0, 0.0, 5.0),
    ] {
        if !modal_open && keys.pressed(key) {
            orbit.yaw += yaw * time.delta_secs();
            orbit.pitch += pitch * time.delta_secs();
            orbit.distance += zoom * time.delta_secs();
            changed = true;
        }
    }
    if !modal_open && keys.just_pressed(KeyCode::Home) {
        *orbit = Orbit::default();
        changed = true;
    }
    if changed {
        if was_following {
            table.dirty = true;
        }
        orbit.follow = false;
        orbit.desired_target = orbit.target;
        orbit.desired_distance = orbit.distance;
    }
    orbit.pitch = orbit.pitch.clamp(0.35, 1.35);
    orbit.distance = orbit.distance.clamp(8.0, 22.0);
    let blend = (time.delta_secs() * 4.0).min(1.0);
    orbit.distance += (orbit.desired_distance - orbit.distance) * blend;
    let desired = orbit.desired_target;
    orbit.target = orbit
        .target
        .lerp(desired, (time.delta_secs() * 4.0).min(1.0));
    let direction = Vec3::new(
        orbit.yaw.sin() * orbit.pitch.cos(),
        orbit.pitch.sin(),
        orbit.yaw.cos() * orbit.pitch.cos(),
    );
    **camera = Transform::from_translation(orbit.target + direction * orbit.distance)
        .looking_at(orbit.target, Vec3::Y);
}

fn text(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    // The embedded font is intentionally sufficient without the external archive.
    // Spell punctuation with supported glyphs so token modifiers never become boxes.
    (
        Text::new(readable_text(value.into())),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

pub(crate) fn readable_text(value: String) -> String {
    value
        .replace(['−', '–'], "-")
        .replace('§', "section ")
        .replace('·', " / ")
        .replace('—', " - ")
        .replace('→', ">")
        .replace('←', "<")
}

enum Run<'a> {
    Words(&'a str),
    Icon(Symbol),
}

// These few runs are explicitly authored for this case, not parsed from rules text.
// Plain-English labels remain available even when the displayed text uses glyphs.
fn symbol_text(
    parent: &mut ChildSpawnerCommands,
    font: &Option<Handle<Font>>,
    runs: &[Run<'_>],
    label: &str,
    size: f32,
    color: Color,
) {
    let Some(font) = font else {
        parent.spawn((
            text(label, size, color),
            AccessibleLabel::new(label),
            Pickable::IGNORE,
        ));
        return;
    };
    parent
        .spawn((
            text("", size, color),
            AccessibleLabel::new(label),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            for run in runs {
                let (value, text_font) = match run {
                    Run::Icon(symbol) => (
                        // Keep an adjacent quantity with its symbol across font runs.
                        // NBSP can still break on overflow in the pinned renderer.
                        format!("\u{2060}{}\u{2060}", symbol.glyph()),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(size.max(18.0) * 1.35),
                            ..default()
                        },
                    ),
                    Run::Words(words) => (
                        readable_text((*words).to_owned()),
                        TextFont {
                            font_size: FontSize::Px(size),
                            ..default()
                        },
                    ),
                };
                parent.spawn((TextSpan::new(value), text_font, TextColor(color)));
            }
        });
}

fn skill_summary(
    parent: &mut ChildSpawnerCommands,
    art: &Art,
    view: &Observation,
    size: f32,
    color: Color,
) {
    symbol_text(
        parent,
        &art.symbol_font,
        &[
            Run::Icon(Symbol::Willpower),
            Run::Words(&format!(" {} / TEST ", view.skill_value)),
            Run::Icon(Symbol::Willpower),
            Run::Words(&format!(" ({})", view.difficulty)),
        ],
        &format!(
            "Willpower {} / Test willpower ({})",
            view.skill_value, view.difficulty
        ),
        size,
        color,
    );
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, action: Action, primary: bool) {
    let tone = if primary { TEAL } else { BUTTON };
    parent
        .spawn((
            Button,
            Hovered::default(),
            Node {
                width: percent(100),
                min_height: px(42),
                padding: UiRect::axes(px(12), px(10)),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(tone),
            ButtonTone(tone),
        ))
        .observe(move |_: On<Activate>, mut queue: ResMut<Actions>| queue.0.push(action.clone()))
        .with_child(text(label, 16.0, PAPER));
}

fn rebuild_panel(
    mut commands: Commands,
    mut table: ResMut<Tabletop>,
    orbit: Res<Orbit>,
    art: Res<Art>,
    panels: Query<Entity, With<Panel>>,
    windows: Query<&Window>,
) {
    if !table.dirty {
        return;
    }
    table.dirty = false;
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    let view = table.view();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(25),
                top: px(20),
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                ..default()
            },
            Pickable::IGNORE,
            Panel,
        ))
        .with_children(|parent| {
            parent.spawn(text("ARKHAM  /  UNFOLDED", 28.0, GOLD));
            parent.spawn(text(
                "Scripted rules experiment · The Gathering",
                16.0,
                PAPER,
            ));
            parent.spawn(text(
                "Prepared token order · one skill-test rules case",
                13.0,
                MUTED,
            ));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(25),
                bottom: px(16),
                ..default()
            },
            Pickable::IGNORE,
            Panel,
        ))
        .with_child(text(
            "Right drag / arrows: orbit    Wheel / + −: zoom    Home: reset    F12: capture",
            13.0,
            MUTED,
        ));
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: px(PANEL),
                padding: UiRect::all(px(24)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.055, 0.085, 0.10)),
            ScrollPosition::default(),
            Panel,
        ))
        .observe(scroll_panel)
        .with_children(|parent| {
            let mode = table.replay.as_ref().map_or("LIVE DECISION".into(), |r| {
                format!("REPLAY  ·  MOMENT {} / {}", r.index() + 1, r.len())
            });
            parent.spawn(text(mode, 13.0, GOLD));
            parent.spawn(text("Rotting Remains", 27.0, PAPER));
            skill_summary(parent, &art, &view, 17.0, GOLD);
            if view.stage == Stage::Finished && has_resolved_curse(&view) {
                parent.spawn(text("Curse: −2. Reveal another token.", 19.0, GOLD));
                parent.spawn(text(
                    "Inspect the additional token, then choose Show test result when ready.",
                    16.0,
                    PAPER,
                ));
            } else {
                parent.spawn(text(&view.prompt, 19.0, PAPER));
            }
            if table.replay.is_none() {
                for choice in &view.choices {
                    button(
                        parent,
                        &choice.label,
                        Action::Choose(view.revision, choice.id.clone()),
                        choice.id != "end",
                    );
                }
            } else if let Some(replay) = &table.replay {
                if replay.index() + 1 < replay.len() {
                    button(parent, "Next moment  →", Action::Next, true);
                }
                if replay.index() > 0 {
                    button(parent, "←  Previous moment", Action::Back, false);
                }
                button(parent, "Return to live session", Action::Live, false);
            }
            if matches!(view.stage, Stage::Finished) {
                parent.spawn(text(
                    format!("{} horror from this test", view.horror_dealt),
                    20.0,
                    if view.horror_dealt > 0 {
                        Color::srgb(0.95, 0.59, 0.49)
                    } else {
                        Color::srgb(0.50, 0.80, 0.67)
                    },
                ));
                button(parent, "Show test result", Action::ShowResult, true);
            }
            parent.spawn(text(
                format!(
                    "Curse pool: {} / 10\nJacqueline's reaction: {}",
                    view.curses_in_pool,
                    if view.jacqueline_used {
                        "used this round"
                    } else {
                        "available"
                    }
                ),
                14.0,
                MUTED,
            ));
            parent.spawn(text(&table.notice, 14.0, PAPER));
            parent.spawn((Node {
                flex_grow: 1.0,
                ..default()
            },));
            button(parent, "Save live session", Action::Save, false);
            button(parent, "Reopen saved session", Action::Reopen, false);
            if table.replay.is_none() {
                button(parent, "Open recorded replay", Action::Replay, false);
            }
            if matches!(view.stage, Stage::Finished | Stage::Ended) {
                button(parent, "Start another example", Action::New, false);
            }
            button(
                parent,
                if table.history {
                    "Hide action history"
                } else {
                    "Explain / action history"
                },
                Action::History,
                false,
            );
            if table.replay.is_some() {
                button(
                    parent,
                    if orbit.follow {
                        "Camera: following · switch to manual"
                    } else {
                        "Camera: manual · follow replay"
                    },
                    Action::Follow,
                    false,
                );
            }
            button(parent, "Choose asset folder", Action::AssetFolder, false);
            parent.spawn(text(&art.status, 12.0, MUTED));
            parent.spawn(text(
                if art.available == 3 {
                    "Original card faces available. Click any card to inspect."
                } else {
                    "Readable card substitutes are active. Click any card to inspect."
                },
                12.0,
                MUTED,
            ));
        });
    if let Some(index) = table.inspect {
        let viewport = windows
            .iter()
            .next()
            .map_or(Vec2::new(1600.0, 1000.0), |window| {
                Vec2::new(window.width(), window.height())
            });
        inspection::show(&mut commands, &art, index, viewport);
    }
    if table.history {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(24),
                    bottom: px(55),
                    width: px(700),
                    max_height: px(420),
                    padding: UiRect::all(px(20)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.055, 0.065, 0.97)),
                ScrollPosition::default(),
                Panel,
            ))
            .observe(scroll_panel)
            .with_children(|parent| {
                parent.spawn(text("What happened — and why", 22.0, GOLD));
                for item in view
                    .history
                    .iter()
                    .rev()
                    .take(7)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                {
                    parent.spawn(text(item, 15.0, PAPER));
                }
                if view.history.len() > 7 {
                    parent.spawn(text(
                        "Earlier entries are available by stepping back in the recorded replay.",
                        12.0,
                        MUTED,
                    ));
                }
                button(parent, "Close history", Action::History, false);
            });
    }
    if table.result_open && view.stage == Stage::Finished {
        show_result(&mut commands, &view, &art);
    }
}

fn show_result(commands: &mut Commands, view: &Observation, art: &Art) {
    let Some(explanation) = explain_result(view) else {
        return;
    };
    let failed = explanation.failed();
    let accent = if failed {
        Color::srgb(0.95, 0.59, 0.49)
    } else {
        Color::srgb(0.50, 0.80, 0.67)
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.025, 0.03, 0.78)),
            GlobalZIndex(100),
            ModalDialogBarrier,
            Panel,
        ))
        .observe(|_: On<RequestClose>, mut queue: ResMut<Actions>| {
            queue.0.push(Action::DismissResult)
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(640),
                        max_width: percent(90),
                        max_height: percent(88),
                        padding: UiRect::all(px(30)),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(10)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(16),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.055, 0.085, 0.10)),
                    BorderColor::all(accent),
                    ScrollPosition::default(),
                    ModalDialog,
                    TabGroup::modal(),
                ))
                .observe(scroll_panel)
                .with_children(|panel| {
                    panel
                        .spawn(Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: px(12),
                            ..default()
                        })
                        .with_children(|header| {
                            header.spawn((
                                text("ROTTING REMAINS / SKILL TEST COMPLETE", 14.0, GOLD),
                                TextLayout::justify(Justify::Center),
                            ));
                            header.spawn((
                                text(
                                    if failed {
                                        "TEST FAILED"
                                    } else {
                                        "TEST SUCCEEDED"
                                    },
                                    36.0,
                                    accent,
                                ),
                                TextLayout::justify(Justify::Center),
                            ));
                            skill_summary(header, art, view, 24.0, PAPER);
                            let amount = if failed {
                                format!("+{} ", explanation.additional_horror)
                            } else {
                                "No additional ".into()
                            };
                            symbol_text(
                                header,
                                &art.symbol_font,
                                &[Run::Words(&amount), Run::Icon(Symbol::Horror)],
                                &format!("{} additional horror", explanation.additional_horror),
                                28.0,
                                accent,
                            );
                            if let Some(horror) = &explanation.jacqueline_horror {
                                let total = if failed {
                                    format!(
                                        " {} -> {} / {}",
                                        horror.previous, horror.total, horror.sanity
                                    )
                                } else {
                                    format!(" {} / {} (unchanged)", horror.total, horror.sanity)
                                };
                                let label = if failed {
                                    format!(
                                        "Jacqueline's horror: {} -> {} of {}.",
                                        horror.previous, horror.total, horror.sanity
                                    )
                                } else {
                                    format!(
                                        "Jacqueline's horror: {} of {} (unchanged).",
                                        horror.total, horror.sanity
                                    )
                                };
                                symbol_text(
                                    header,
                                    &art.symbol_font,
                                    &[
                                        Run::Words("Jacqueline: "),
                                        Run::Icon(Symbol::Horror),
                                        Run::Words(&total),
                                    ],
                                    &label,
                                    18.0,
                                    PAPER,
                                );
                            }
                        });
                    panel.spawn(text(
                        format!(
                            "Resolved: {}",
                            explanation
                                .resolved
                                .iter()
                                .map(|face| face.label())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        17.0,
                        MUTED,
                    ));
                    if let Some(reason) = explanation.auto_fail_reason {
                        panel.spawn(text(reason, 18.0, PAPER));
                    }
                    if let Some(reason) = explanation.curse_reason {
                        panel.spawn(text("Why another token?", 22.0, GOLD));
                        panel.spawn(text(reason, 18.0, PAPER));
                        for continuation in explanation.continuation {
                            panel.spawn(text(continuation, 16.0, MUTED));
                        }
                    }
                    panel
                        .spawn((
                            Button,
                            Hovered::default(),
                            TabIndex(0),
                            Node {
                                width: percent(100),
                                min_height: px(48),
                                flex_shrink: 0.0,
                                padding: UiRect::all(px(12)),
                                align_items: AlignItems::Center,
                                border_radius: BorderRadius::all(px(5)),
                                ..default()
                            },
                            BackgroundColor(TEAL),
                            ButtonTone(TEAL),
                        ))
                        .observe(|_: On<Activate>, mut queue: ResMut<Actions>| {
                            queue.0.push(Action::DismissResult)
                        })
                        .with_child(text("Return to table", 18.0, PAPER));
                });
        });
}

fn button_colors(
    mut buttons: Query<(&Hovered, Has<Pressed>, &ButtonTone, &mut BackgroundColor), With<Button>>,
) {
    for (hovered, pressed, tone, mut color) in &mut buttons {
        if pressed {
            color.0 = Color::srgb(0.25, 0.48, 0.43);
        } else if hovered.get() {
            color.0 = Color::srgb(0.20, 0.33, 0.34);
        } else {
            color.0 = tone.0;
        }
    }
}

fn scroll_panel(event: On<PointerScroll>, mut panels: Query<(&mut ScrollPosition, &ComputedNode)>) {
    if let Ok((mut position, node)) = panels.get_mut(event.entity) {
        let delta = match event.unit {
            MouseScrollUnit::Line => event.y * 28.0,
            MouseScrollUnit::Pixel => event.y,
        };
        let range = (node.content_size.y - node.size.y).max(0.0) * node.inverse_scale_factor;
        position.y = (position.y - delta).clamp(0.0, range);
    }
}

fn capture_window(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut capture: ResMut<Capture>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
) {
    capture.frame += 1;
    if keys.just_pressed(KeyCode::F12) {
        let path = "local/experiment/tabletop.png";
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if !capture.requested
        && capture.frame >= 90
        && let Some(path) = &capture.path
    {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        capture.requested = true;
    }
    if capture.quit
        && capture.requested
        && capture.frame > 95
        && capturing.is_empty()
        && capture.path.as_ref().is_some_and(|path| path.is_file())
    {
        exit.write(AppExit::Success);
    }
}
