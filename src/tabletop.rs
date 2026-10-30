//! The replaceable tabletop for the single, explicitly scripted rules experiment.
//! Every displayed decision and replay moment comes from the shared rules interface.

use crate::tabletop_camera::{
    CameraMode, Capture as CameraCapture, FrameInput, Orbit, parse_initial_view,
};
use crate::tabletop_pacing::{PresentationTime, Redraw, RequestRedraw, TabletopPacingPlugin};
use crate::{
    Fixture, Observation, Session, Stage, TokenFace, TokenStatus,
    asset_config::AssetConfig,
    assets::{self, CollectionReport},
    continuity::{self, Replay},
    explanation::explain_result,
};
use bevy::{
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
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
mod context;
mod inspection;
use crate::tabletop_layout::{self, Area};
use context::{Context, Source};
mod scene;
mod symbols;
use asset_settings::AssetSettings;
use symbols::Symbol;

const PANEL: f32 = 190.0;
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
    Select(Source, Vec2),
    ContextChoose(u64, u64, String),
    PrepareCancellation(u64, String),
    DismissContext,
    Focus(Area),
    Utilities,
    SourceLess(u64, u64, String),
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
struct Art {
    handles: [Option<Handle<Image>>; 3],
    available: usize,
    token_handles: Vec<(TokenFace, Handle<Image>)>,
    reminder: Option<Handle<Image>>,
    investigators: Vec<(String, Handle<Image>)>,
    symbol_font: Option<Handle<Font>>,
    status: String,
}

impl Art {
    fn text(&self, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
        crate::tabletop_harm_text::ui(value, size, color, &self.symbol_font)
    }
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
    initial_view: Option<Area>,
}

impl Options {
    fn parse() -> Result<Option<Self>, String> {
        Self::parse_from(std::env::args().skip(1), AssetConfig::load()?)
    }

    fn parse_from(
        args: impl IntoIterator<Item = String>,
        config: AssetConfig,
    ) -> Result<Option<Self>, String> {
        let mut options = Self {
            fixture: Fixture::curse(),
            save: "local/experiment/curse.session.json".into(),
            load: None,
            art_root: Some(config.root().to_path_buf()),
            config,
            screenshot: None,
            quit_after_screenshot: false,
            initial_view: None,
        };
        let mut explicit_save = false;
        let mut selected_case = "curse";
        let mut args = args.into_iter();
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
                "--initial-view" => options.initial_view = Some(parse_initial_view(&value()?)?),
                "--help" | "-h" => {
                    println!(
                        "Arkham Unfolded — scripted rules experiment\n\n--case curse|autofail  Choose the privileged starting fixture\n--save PATH            Session save and autosave path\n--load PATH            Reopen a saved pending decision\n--art-root PATH        Use a prepared asset collection for this launch\n--no-art               Use readable substitutes\n--screenshot PATH      Capture the actual window once ready\n--quit-after-screenshot\n--initial-view overview|scenario|test  Diagnostic startup framing; Home is unchanged\n\nCamera: WASD to pan; right drag or arrow keys to orbit; wheel or +/- to zoom; Home to reset. F12 saves a screenshot."
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
    eprintln!("Experiment: Jacqueline skill test");
    match &art_root {
        Some(root) => eprintln!("Artwork root: {}", root.display()),
        None => eprintln!("Artwork root: disabled for this launch (--no-art)"),
    }
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
        .insert_resource(Orbit::new(CameraMode::Jacqueline).with_initial_view(options.initial_view))
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
        .add_plugins(TabletopPacingPlugin)
        .add_systems(Last, request_redraw.in_set(RequestRedraw))
        .add_systems(Startup, setup_table)
        .add_systems(
            Update,
            (
                mark_window_resized,
                poll_asset_settings,
                context::keyboard,
                handle_actions,
                context::synchronize,
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
) {
    tabletop_layout::spawn_surfaces(&mut commands, &mut meshes, &mut materials);
    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, -0.055, 4.0),
        ))
        .observe(|event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == PointerButton::Primary {
                queue.0.push(Action::DismissContext);
            }
        });
    let bag = materials.add(Color::srgb(0.18, 0.10, 0.16));
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.65, 0.9))),
            MeshMaterial3d(bag),
            Transform::from_xyz(-6.4, 0.48, 0.5),
            context::Selectable(Source::Bag),
        ))
        .observe(|event: On<PointerClick>, mut queue: ResMut<Actions>| {
            context::select(event, Source::Bag, &mut queue)
        });
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.46, 0.025))),
            MeshMaterial3d(materials.add(Color::BLACK)),
            Transform::from_xyz(-6.4, 0.95, 0.5),
        ))
        .observe(|event: On<PointerClick>, mut queue: ResMut<Actions>| {
            context::select(event, Source::Bag, &mut queue)
        });
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
    mut context: Option<ResMut<Context>>,
    mut input_focus: Option<ResMut<bevy::input_focus::InputFocus>>,
) {
    for mut action in queue.0.drain(..) {
        if settings.as_ref().is_some_and(|settings| settings.is_busy()) {
            continue;
        }
        // A modal consumes interaction with the underlying table, including queued clicks.
        if (table.result_open && !matches!(action, Action::DismissResult))
            || (table.inspect.is_some() && !matches!(action, Action::CloseInspect))
        {
            continue;
        }
        if matches!(
            action,
            Action::Reopen
                | Action::Replay
                | Action::Back
                | Action::Next
                | Action::Live
                | Action::New
        ) && let Some(context) = context.as_mut()
        {
            context.invalidate();
        }
        if let Action::SourceLess(generation, revision, ref id) = action {
            let view = table.view();
            let Some(choice) = context.as_ref().and_then(|context| {
                context.offered_source_less(
                    &view,
                    table.replay.as_ref().map(Replay::index),
                    generation,
                    id,
                )
            }) else {
                continue;
            };
            action = Action::Choose(revision, choice.id.clone());
        }
        if let Action::ContextChoose(generation, revision, ref id) = action {
            let view = table.view();
            let Some(choice) = context.as_ref().and_then(|context| {
                context.offered_choice(
                    &view,
                    table.replay.as_ref().map(Replay::index),
                    generation,
                    id,
                )
            }) else {
                continue;
            };
            action = Action::Choose(revision, choice.id.clone());
        }
        match action {
            Action::Select(source, anchor) => {
                if let Some(context) = context.as_mut() {
                    context.select(
                        &table.view(),
                        table.replay.as_ref().map(Replay::index),
                        source,
                        anchor,
                    );
                }
                if let Some(focus) = input_focus.as_mut() {
                    focus.clear();
                }
            }
            Action::PrepareCancellation(generation, id) => {
                if let Some(context) = context.as_mut() {
                    context.prepare_cancellation(
                        &table.view(),
                        table.replay.as_ref().map(Replay::index),
                        generation,
                        &id,
                    );
                }
            }
            Action::Utilities => {
                if let Some(context) = context.as_mut() {
                    context.toggle_utilities();
                }
                if let Some(focus) = input_focus.as_mut() {
                    focus.clear();
                }
            }
            Action::DismissContext => {
                if let Some(context) = context.as_mut() {
                    context.dismiss();
                }
                if let Some(focus) = input_focus.as_mut() {
                    focus.clear();
                }
            }
            Action::Focus(area) => {
                orbit.focus(tabletop_layout::focus(area));
                if let Some(context) = context.as_mut() {
                    context.close_menu();
                }
                if let Some(focus) = input_focus.as_mut() {
                    focus.clear();
                }
            }
            Action::Choose(revision, id) if table.replay.is_none() => {
                if let Some(focus) = input_focus.as_mut() {
                    focus.clear();
                }
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
                    orbit.set_following(false);
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
                        orbit.set_following(true);
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
                orbit.set_following(false);
                table.notice = "Returned to the live session; replay made no changes to it.".into();
            }
            Action::New => {
                let session = Session::new(table.fixture.clone());
                match continuity::restart_session(&table.save_path, &session) {
                    Ok(()) => {
                        table.session = session;
                        table.replay = None;
                        table.result_open = false;
                        orbit.set_following(false);
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
                if let Some(context) = context.as_mut() {
                    context.close_menu();
                }
                table.inspect = Some(i);
                table.history = false;
            }
            Action::CloseInspect => {
                table.inspect = None;
                if let Some(focus) = input_focus.as_mut() {
                    focus.clear();
                }
            }
            Action::History => {
                table.history = !table.history;
                table.inspect = None;
            }
            Action::Follow => {
                if table.replay.is_some() {
                    let following = !orbit.is_following();
                    orbit.set_following(following);
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
            Action::Choose(_, _) | Action::ContextChoose(_, _, _) | Action::SourceLess(_, _, _) => {
            }
        }
        if let Some(context) = context.as_mut() {
            context.synchronize(&table.view(), table.replay.as_ref().map(Replay::index));
        }
        let framing = (orbit.is_following() && table.replay.is_some())
            .then(|| tabletop_layout::focus(Area::Test));
        orbit.update_follow_target(framing);
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
    scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window>,
    mut table: ResMut<Tabletop>,
    time: Res<PresentationTime>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    settings: Option<Res<AssetSettings>>,
    mut context: Option<ResMut<Context>>,
    input_focus: Option<Res<bevy::input_focus::InputFocus>>,
    focusable: Query<(), With<Button>>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    ui_nodes: Query<(), (With<Node>, Without<context::DismissalBackdrop>)>,
) {
    let modal_open = table.result_open
        || table.inspect.is_some()
        || settings.as_ref().is_some_and(|settings| settings.is_busy());
    let over_ui = hover
        .as_ref()
        .and_then(|hover| hover.get(&bevy::picking::pointer::PointerId::Mouse))
        .is_some_and(|hits| hits.keys().any(|entity| ui_nodes.contains(*entity)));
    let keyboard_captured = modal_open
        || table.history
        || context
            .as_ref()
            .is_some_and(|context| context.menu_open() || context.utilities_open())
        || input_focus
            .as_ref()
            .and_then(|focus| focus.get())
            .is_some_and(|entity| focusable.contains(entity))
        || keys.just_pressed(KeyCode::Escape);
    let step = orbit.step(FrameInput {
        keys: &keys,
        buttons: &buttons,
        window: &window,
        scroll_y: scroll.delta.y,
        dt: time.delta_secs(),
        capture: CameraCapture {
            pointer_blocked: modal_open || table.history,
            over_ui,
            keyboard: keyboard_captured,
            wheel: context
                .as_ref()
                .is_some_and(|context| context.menu_open() || context.utilities_open()),
        },
        reduced_motion: false,
    });
    if step.manual_navigation {
        if let Some(context) = context.as_mut()
            && context.camera_moved()
        {
            table.dirty = true;
        }
        if step.stopped_following {
            table.dirty = true;
        }
    }
    **camera = step.transform;
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
                match run {
                    Run::Icon(symbol) => {
                        parent.spawn((
                            // Keep an adjacent quantity with its symbol across font runs.
                            TextSpan::new(format!("\u{2060}{}\u{2060}", symbol.glyph())),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(size.max(18.0) * 1.35),
                                ..default()
                            },
                            TextColor(symbol.color().unwrap_or(color)),
                        ));
                    }
                    Run::Words(words) => {
                        for span in crate::tabletop_harm_text::spans(words, size, color, font) {
                            parent.spawn(span);
                        }
                    }
                }
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

fn button(
    parent: &mut ChildSpawnerCommands,
    art: &Art,
    label: &str,
    action: Action,
    primary: bool,
) -> Entity {
    let tone = if primary { TEAL } else { BUTTON };
    parent
        .spawn((
            Button,
            TabIndex(0),
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
        .with_child(art.text(label, 16.0, PAPER))
        .id()
}

#[allow(clippy::too_many_arguments)]
fn rebuild_panel(
    mut commands: Commands,
    mut table: ResMut<Tabletop>,
    orbit: Res<Orbit>,
    art: Res<Art>,
    panels: Query<Entity, With<Panel>>,
    windows: Query<&Window>,
    context: Option<Res<Context>>,
    images: Option<Res<Assets<Image>>>,
    mut last_portrait_readiness: Local<[bool; 3]>,
) {
    let readiness = ["Tony Morgan", "Patrice Hathaway", "Jacqueline Fine"].map(|name| {
        art.investigators.iter().any(|(found, handle)| {
            found == name
                && images
                    .as_ref()
                    .is_some_and(|images| images.get(handle).is_some())
        })
    });
    if *last_portrait_readiness != readiness {
        *last_portrait_readiness = readiness;
        table.dirty = true;
    }
    if !table.dirty {
        return;
    }
    table.dirty = false;
    for entity in &panels {
        commands.entity(entity).despawn();
    }
    let view = table.view();
    let viewport = windows
        .iter()
        .next()
        .map_or(Vec2::new(1600.0, 1000.0), |window| {
            Vec2::new(window.width(), window.height())
        });
    let generation = context.as_ref().map_or(0, |context| context.generation());
    commands.spawn((Node { position_type: PositionType::Absolute, left: px(16), top: px(12),
        max_width: px((viewport.x - PANEL - 48.0).max(180.0)), padding: UiRect::all(px(10)),
        flex_direction: FlexDirection::Column, row_gap: px(5), ..default()
    }, BackgroundColor(Color::srgba(0.035, 0.055, 0.065, 0.94)), Panel))
    .with_children(|parent| {
        let mode = table.replay.as_ref().map_or("LIVE DECISION".into(), |r| format!("REPLAY / MOMENT {} / {}", r.index() + 1, r.len()));
        parent.spawn(art.text(format!("{mode} / Decision: {} / Test: Jacqueline", context::actor(&view)), 15.0, GOLD));
        skill_summary(parent, &art, &view, 16.0, PAPER);
        parent.spawn(art.text(&view.prompt, 16.0, PAPER));
        let hint = match view.stage {
            Stage::Commit => "Focus Patrice, then select Promise of Power to commit it.",
            Stage::BeforeReveal => "Select Jacqueline's card for her reaction, or the chaos bag to reveal.",
            Stage::Cancellation => "Focus Test and select any revealed token to choose a complete cancellation set.",
            Stage::Finished => "Inspect the tokens and Show test result when ready.",
            Stage::Ended => "This bounded experiment has ended; its record is preserved.",
        };
        parent.spawn(art.text(hint, 12.0, MUTED));
        parent.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(5), ..default() })
            .with_children(|row| {
                let mut control = |label: &str, action: Action, primary| {
                    row.spawn(Node { width: px(210), ..default() }).with_children(|slot| { button(slot, &art, label, action, primary); });
                };
                if table.replay.is_none() {
                    for choice in view.choices.iter().filter(|choice| context::source_less(&choice.id)) {
                        control(&choice.label, Action::SourceLess(generation, view.revision, choice.id.clone()), choice.id != "end");
                    }
                } else if let Some(replay) = &table.replay {
                    if replay.index() + 1 < replay.len() { control("Next moment", Action::Next, true); }
                    if replay.index() > 0 { control("Previous moment", Action::Back, false); }
                    control("Return to live session", Action::Live, false);
                }
                if view.stage == Stage::Finished { control("Show test result", Action::ShowResult, true); }
                if view.stage == Stage::Ended { control("Start another example", Action::New, false); }
            });
        if view.stage == Stage::Finished && has_resolved_curse(&view) {
            parent.spawn(art.text("Curse: -2. Reveal another token.", 13.0, GOLD));
        }
        if !table.notice.is_empty() { parent.spawn(art.text(&table.notice, 12.0, MUTED)); }
    });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(10),
                top: px(12),
                width: px(PANEL),
                max_height: px((viewport.y - 55.0).max(180.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            Pickable::IGNORE,
            Panel,
        ))
        .observe(scroll_panel)
        .with_children(|parent| {
            button(parent, &art, "Menu", Action::Utilities, false);
            context::focus_strip(parent, &art, images.as_deref());
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                bottom: px(10),
                max_width: percent(85),
                ..default()
            },
            Pickable::IGNORE,
            Panel,
        ))
        .with_child(art.text(
            "WASD: pan / Right drag or arrows: orbit / Wheel or +/-: zoom / Home: overview",
            12.0,
            MUTED,
        ));
    if context
        .as_ref()
        .is_some_and(|context| context.utilities_open())
    {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(12),
                    top: px(62),
                    width: px(305.0_f32.min(viewport.x - 24.0)),
                    max_height: px((viewport.y - 160.0).max(150.0)),
                    padding: UiRect::all(px(12)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(7),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(INK),
                GlobalZIndex(45),
                ScrollPosition::default(),
                Panel,
            ))
            .observe(scroll_panel)
            .with_children(|parent| {
                button(
                    parent,
                    &art,
                    "Explain / action history",
                    Action::History,
                    false,
                );
                if table.replay.is_none() {
                    button(parent, &art, "Save live session", Action::Save, false);
                    button(parent, &art, "Reopen saved session", Action::Reopen, false);
                    button(parent, &art, "Open recorded replay", Action::Replay, false);
                } else {
                    button(
                        parent,
                        &art,
                        if orbit.is_following() {
                            "Camera following: on"
                        } else {
                            "Resume camera following"
                        },
                        Action::Follow,
                        false,
                    );
                }
                if matches!(view.stage, Stage::Finished | Stage::Ended) {
                    button(parent, &art, "Start another example", Action::New, false);
                }
                parent.spawn(art.text(
                    format!(
                        "Curse pool: {} / 10 / Jacqueline reaction: {}",
                        view.curses_in_pool,
                        if view.jacqueline_used {
                            "used"
                        } else {
                            "available"
                        }
                    ),
                    13.0,
                    MUTED,
                ));
                parent.spawn(art.text(&art.status, 12.0, MUTED));
                parent.spawn(art.text(
                    format!("{}/3 original card faces available", art.available),
                    12.0,
                    MUTED,
                ));
                button(
                    parent,
                    &art,
                    "Choose asset folder",
                    Action::AssetFolder,
                    false,
                );
                button(parent, &art, "Close menu", Action::DismissContext, false);
            });
    }
    if let Some(context) = context.as_ref() {
        context::show(&mut commands, context, &table, &art, viewport);
    }
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
                parent.spawn(art.text("What happened — and why", 22.0, GOLD));
                for item in view
                    .history
                    .iter()
                    .rev()
                    .take(7)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                {
                    parent.spawn(art.text(item, 15.0, PAPER));
                }
                if view.history.len() > 7 {
                    parent.spawn(art.text(
                        "Earlier entries are available by stepping back in the recorded replay.",
                        12.0,
                        MUTED,
                    ));
                }
                button(parent, &art, "Close history", Action::History, false);
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
                                art.text("ROTTING REMAINS / SKILL TEST COMPLETE", 14.0, GOLD),
                                TextLayout::justify(Justify::Center),
                            ));
                            header.spawn((
                                art.text(
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
                    panel.spawn(art.text(
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
                        panel.spawn(art.text(reason, 18.0, PAPER));
                    }
                    if let Some(reason) = explanation.curse_reason {
                        panel.spawn(art.text("Why another token?", 22.0, GOLD));
                        panel.spawn(art.text(reason, 18.0, PAPER));
                        for continuation in explanation.continuation {
                            panel.spawn(art.text(continuation, 16.0, MUTED));
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
                        .with_child(art.text("Return to table", 18.0, PAPER));
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

fn request_redraw(
    mut redraw: ResMut<Redraw>,
    orbit: Res<Orbit>,
    capture: Res<Capture>,
    screenshots: Query<(), With<Screenshot>>,
) {
    if orbit.is_moving()
        || (capture.path.is_some() && (!capture.requested || capture.quit))
        || !screenshots.is_empty()
    {
        redraw.request();
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

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn diagnostic_initial_view_parses_without_changing_default_framing_or_files() {
        let directory = tempfile::tempdir().unwrap();
        let parse = |args: &[&str]| {
            Options::parse_from(
                args.iter().map(|arg| (*arg).to_owned()),
                AssetConfig::load_at(
                    directory.path().join("settings.toml"),
                    directory.path().join("assets"),
                ),
            )
        };
        assert_eq!(parse(&[]).unwrap().unwrap().initial_view, None);
        for (name, area) in [
            ("overview", Area::Overview),
            ("scenario", Area::Scenario),
            ("test", Area::Test),
        ] {
            assert_eq!(
                parse(&["--initial-view", name])
                    .unwrap()
                    .unwrap()
                    .initial_view,
                Some(area)
            );
        }
        assert!(parse(&["--initial-view"]).is_err());
        assert!(parse(&["--initial-view", "missing"]).is_err());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[cfg(test)]
mod harm_text_tests {
    use super::*;
    use crate::tabletop_harm_text::HORROR_COLOR;

    fn art() -> Art {
        Art {
            handles: [None, None, None],
            available: 0,
            token_handles: Vec::new(),
            reminder: None,
            investigators: Vec::new(),
            symbol_font: Some(Handle::default()),
            status: "Readable card substitutes.".into(),
        }
    }

    fn assert_horror_spelling(world: &mut World, english: &str) {
        let entity = world
            .query::<(Entity, &AccessibleLabel)>()
            .iter(world)
            .find(|(_, label)| label.0 == english)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("missing accessible label: {english}"));
        let spans: Vec<_> = world
            .get::<Children>(entity)
            .unwrap()
            .iter()
            .filter_map(|child| {
                Some((
                    world.get::<TextSpan>(child)?,
                    world.get::<TextColor>(child)?,
                ))
            })
            .collect();
        assert!(
            spans
                .iter()
                .any(|(text, color)| { text.0 == "\u{2060}R\u{2060}" && color.0 == HORROR_COLOR })
        );
        assert!(spans.iter().all(|(text, _)| !text.0.contains("horror")));
    }

    #[test]
    fn jacqueline_history_and_card_rules_use_blue_horror_with_english_labels() {
        let session = Session::new(Fixture::curse());
        let history = session.observe().history[0].clone();
        let mut app = App::new();
        app.insert_resource(Tabletop {
            fixture: Fixture::curse(),
            session,
            replay: None,
            save_path: PathBuf::from("unused-harm-text-test.json"),
            notice: String::new(),
            inspect: None,
            history: true,
            result_open: false,
            dirty: true,
        })
        .insert_resource(Orbit::new(CameraMode::Jacqueline))
        .insert_resource(art())
        .add_systems(Update, rebuild_panel);
        app.update();
        assert_horror_spelling(app.world_mut(), &history);

        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, app.world());
        inspection::show(&mut commands, &art(), 0, Vec2::new(1200.0, 900.0));
        queue.apply(app.world_mut());
        assert_horror_spelling(app.world_mut(), CARDS[0].text);
        assert_eq!(
            app.world().resource::<Tabletop>().session.observe().history[0],
            history
        );
    }
}
