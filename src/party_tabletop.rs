//! Native presentation of the prepared Study escape attempt. Rules stay at the player seam.
use crate::{
    asset_config::AssetConfig,
    assets,
    party::{Investigator, Observation, Session, continuity},
    tabletop::asset_settings::AssetSettings,
    tabletop_camera::{
        CameraMode, Capture as CameraCapture, FrameInput, Orbit, parse_initial_view,
    },
    tabletop_feedback::{self as feedback, CardFeedback, FeedbackSettings},
    tabletop_gathering_map as location_map,
    tabletop_health::{HealthAssets, RenderHealth, TabletopHealthPlugin, exit_result},
    tabletop_layout::{self as layout, Area},
    tabletop_pacing::{PresentationTime, Redraw, RequestRedraw, TabletopPacingPlugin},
    tabletop_reveal as reveal,
};
use bevy::{
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    input_focus::InputFocus,
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot},
    ui_widgets::{Activate, Button, RequestClose},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
mod cards;
mod consequences;
mod controls;
mod scene;
mod ui;
use cards::CARDS;
use controls::{Context, Source};
const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.65, 0.74, 0.73);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);
const CARD_WIDTH: f32 = 1.35;
const CARD_LENGTH: f32 = 1.89;

struct Options {
    save: PathBuf,
    load: Option<PathBuf>,
    art_root: Option<PathBuf>,
    config: AssetConfig,
    screenshot: Option<PathBuf>,
    quit: bool,
    initial_view: Option<Area>,
}
impl Options {
    fn parse() -> Result<Option<Self>, String> {
        Self::parse_from(std::env::args().skip(1), AssetConfig::load_party()?)
    }
    fn parse_from(
        args: impl IntoIterator<Item = String>,
        config: AssetConfig,
    ) -> Result<Option<Self>, String> {
        let mut result = Self {
            save: "local/experiment/party.session.json".into(),
            load: None,
            art_root: Some(config.root().into()),
            config,
            screenshot: None,
            quit: false,
            initial_view: None,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--experiment" => {
                    if value()? != "party" {
                        return Err("Use --experiment party.".into());
                    }
                }
                "--save" => result.save = value()?.into(),
                "--load" => result.load = Some(value()?.into()),
                "--art-root" => result.art_root = Some(value()?.into()),
                "--no-art" => result.art_root = None,
                "--screenshot" => result.screenshot = Some(value()?.into()),
                "--quit-after-screenshot" => result.quit = true,
                "--initial-view" => result.initial_view = Some(parse_initial_view(&value()?)?),
                "--help" | "-h" => {
                    println!(
                        "Arkham Unfolded — Prepared Study escape attempt\n--experiment party\n--save PATH\n--load PATH\n--art-root PATH\n--no-art\n--screenshot PATH\n--quit-after-screenshot\n--initial-view overview|scenario|test|tony|patrice|jacqueline"
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown party option: {arg}")),
            }
        }
        if result.quit && result.screenshot.is_none() {
            return Err("--quit-after-screenshot requires --screenshot PATH".into());
        }
        Ok(Some(result))
    }
}
#[derive(Resource)]
struct Table {
    session: Session,
    save_path: PathBuf,
    notice: String,
    inspect: Option<usize>,
    history: bool,
    dirty: bool,
    reduced_motion: bool,
    context: Context,
}
impl Table {
    fn new(session: Session, save_path: PathBuf) -> Self {
        Self {
            session,
            save_path,
            notice: "Every committed decision is saved. Close and reopen at any choice.".into(),
            inspect: None,
            history: false,
            dirty: true,
            reduced_motion: false,
            context: Context::default(),
        }
    }
    fn view(&self) -> Observation {
        self.session.observe()
    }
    fn show_test_summary(&self, view: &Observation) -> bool {
        if test_window(view) {
            return true;
        }
        self.session.result_just_settled()
            && view.test.as_ref().is_some_and(|test| test.result.is_some())
    }
    fn modal(&self) -> bool {
        self.act_reveal() || self.inspect.is_some() || self.history
    }
    fn act_reveal(&self) -> bool {
        self.view().stage == crate::party::Stage::ActReveal
    }
    fn invalidate(&mut self) {
        self.context.sync(self.view().revision);
        self.dirty = true;
    }
}
#[derive(Resource, Default)]
struct Art {
    handles: BTreeMap<String, Handle<Image>>,
    symbol_font: Option<Handle<Font>>,
    status: String,
}
impl Art {
    fn image(&self, role: &str) -> Option<Handle<Image>> {
        self.handles.get(role).cloned()
    }
    fn text(&self, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
        crate::tabletop_harm_text::ui(value, size, color, &self.symbol_font)
    }
}
#[derive(Clone)]
enum Action {
    Choose(u64, String),
    Focus(Area),
    Select(u64, Source, Vec3),
    ContextChoice(u64, u64, String),
    Toggle(u64, u64, u16, bool),
    Confirm(u64, bool),
    Reveal(reveal::Request),
    Inspect(usize),
    Close,
    History,
    Dismiss,
    Utilities,
    Motion,
    Save,
    Reopen,
    New,
    AssetFolder,
}
#[derive(Resource, Default)]
struct Actions(Vec<Action>);
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Scene;
#[derive(Component)]
struct MainCamera;
use crate::tabletop_pieces::WorldLabel;
#[derive(Component)]
struct CardSource(Source);
#[derive(Component)]
struct Appearing;

#[derive(Resource)]
struct Capture {
    path: Option<PathBuf>,
    quit: bool,
    frame: u32,
    requested: bool,
}

pub fn run() -> Result<(), String> {
    let Some(options) = Options::parse()? else {
        return Ok(());
    };
    let load = options
        .load
        .or_else(|| options.save.exists().then(|| options.save.clone()));
    let session = match &load {
        Some(path) => continuity::load_session(path)
            .map_err(|e| format!("Could not reopen party attempt: {e}"))?,
        None => Session::new().map_err(|e| e.to_string())?,
    };
    let root = options.art_root.map(|p| {
        if p.is_absolute() {
            p
        } else {
            std::env::current_dir().unwrap_or_default().join(p)
        }
    });
    let collection = root.as_deref().map(assets::inspect_party);
    let status = collection.as_ref().map_or_else(
        || "Readable substitutes active; artwork disabled.".into(),
        |r| r.summary(),
    );
    eprintln!(
        "Experiment: Prepared Study escape attempt (--experiment party)\nArtwork root: {}\n{status}",
        root.as_ref()
            .map_or_else(|| "disabled".into(), |p| p.display().to_string())
    );
    let mut table = Table::new(session, load.unwrap_or(options.save));
    if let Some(warning) = options.config.warning() {
        table.notice = format!("Asset settings: {warning}");
    }
    let mut app = App::new();
    app.insert_resource(ClearColor(INK))
        .insert_resource(table)
        .insert_resource(Orbit::new(CameraMode::Jacqueline).with_initial_view(options.initial_view))
        .insert_resource(AssetSettings::new_party(options.config))
        .insert_resource(Capture {
            path: options.screenshot,
            quit: options.quit,
            frame: 0,
            requested: false,
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Arkham Unfolded — Prepared Study Escape".into(),
                        resolution: (1600, 1000).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: root.map_or_else(
                        || "local/experiment/absent-art".into(),
                        |p| p.to_string_lossy().into_owned(),
                    ),
                    ..default()
                }),
        )
        .add_plugins(MeshPickingPlugin)
        .add_plugins(TabletopPacingPlugin)
        .add_plugins(TabletopHealthPlugin);
    install(&mut app);
    let server = app.world().resource::<AssetServer>();
    let mut art = Art {
        status,
        ..default()
    };
    for role in image_asset_roles() {
        if let Some(path) = collection.as_ref().and_then(|r| r.verified_path(&role)) {
            art.handles.insert(role, server.load(path.to_owned()));
        }
    }
    art.symbol_font = collection
        .as_ref()
        .and_then(|r| r.verified_path("font/symbols"))
        .map(|p| server.load(p.to_owned()));
    let health_assets = HealthAssets::new(
        art.handles
            .values()
            .map(|handle| handle.id().untyped())
            .chain(art.symbol_font.iter().map(|handle| handle.id().untyped())),
    );
    app.insert_resource(health_assets)
        .insert_resource(reveal::Artwork(art.image(reveal::ART_ROLE)))
        .insert_resource(art);
    exit_result(app.run())
}

fn image_asset_roles() -> impl Iterator<Item = String> {
    CARDS
        .iter()
        .map(|card| card.role.to_owned())
        .chain(
            [
                "card-back/player",
                "ui/reveal-chaos-token",
                "token/curse",
                "token/damage",
                "token/horror",
            ]
            .map(str::to_owned),
        )
        .chain(["06003", "06005", "60401"].map(|code| format!("standee/{code}")))
        .chain(crate::tabletop_counters::asset_roles())
        .chain(
            [
                "plus_one",
                "zero",
                "minus_one",
                "minus_two",
                "minus_three",
                "minus_four",
                "skull",
                "cultist",
                "tablet",
                "auto_fail",
                "elder_sign",
            ]
            .map(|face| format!("token/{face}")),
        )
}
/// Party's application loop, shared by native startup and its behavioral host.
/// Hosts supply input/UI, assets, window, health, time, capture and session state.
fn install(app: &mut App) {
    app.init_resource::<Actions>()
        .init_resource::<consequences::Lifetimes>()
        .add_plugins(feedback::FeedbackPlugin)
        .add_plugins(reveal::RevealPlugin)
        .add_plugins(crate::tabletop_counters::CounterPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                poll_settings,
                handle_actions,
                camera_controls,
                consequences::advance_clue,
                scene::synchronize,
                synchronize_feedback,
                ui::render,
                controls::render,
                sync_reveal,
                reveal::render,
                world_labels,
                crate::tabletop_tokens::position_labels,
                animate,
                consequences::advance_returning,
                capture_window,
            )
                .chain(),
        )
        .add_systems(Last, request_redraw.in_set(RequestRedraw));
}

fn poll_settings(mut settings: ResMut<AssetSettings>, mut table: ResMut<Table>) {
    if let Some(message) = settings.poll() {
        table.notice = message;
        table.dirty = true;
    }
}
#[allow(clippy::too_many_arguments)]
fn handle_actions(
    health: Res<RenderHealth>,
    mut actions: ResMut<Actions>,
    mut table: ResMut<Table>,
    mut orbit: ResMut<Orbit>,
    mut settings: ResMut<AssetSettings>,
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: Option<ResMut<InputFocus>>,
    mut shortcut: ResMut<reveal::Shortcut>,
    mut consequences: consequences::Presentation,
) {
    let revision = table.view().revision;

    table.context.sync(revision);
    if keys.just_pressed(KeyCode::Escape) {
        actions.0.push(if table.modal() {
            Action::Close
        } else {
            Action::Dismiss
        });
    }
    actions
        .0
        .extend(shortcut.drain().into_iter().map(Action::Reveal));
    for action in actions.0.drain(..) {
        if consequences.blocks(&action) {
            continue;
        }
        // Discard input queued before admission, including a click from the last
        // checking frame. Camera navigation and dismissal never change the record.
        if (!health.allows_play() || health.just_became_ready())
            && !matches!(action, Action::Focus(_) | Action::Close | Action::Dismiss)
        {
            continue;
        }
        let permitted_in_modal = if table.act_reveal() {
            match &action {
                Action::Choose(_, id) => id == "continue-act",
                Action::Save | Action::Reopen => true,
                _ => false,
            }
        } else {
            matches!(action, Action::Close)
                || (table.history && matches!(action, Action::Inspect(_)))
        };
        if settings.is_busy() || (table.modal() && !permitted_in_modal) {
            continue;
        }
        let mut reveal_focus = false;
        let action = match action {
            Action::ContextChoice(epoch, revision, id) => {
                let Some(choice) = table.context.contextual_choice(epoch, revision, id) else {
                    continue;
                };
                choice
            }
            Action::Confirm(revision, cancel) => {
                let Some(choice) = table.context.confirm(&table.view(), revision, cancel) else {
                    continue;
                };
                choice
            }
            Action::Reveal(request) => {
                refresh_reveal(&table, &settings, &mut shortcut);
                match shortcut.admit(request) {
                    Some(reveal::Accepted::Choice(revision, id)) => {
                        reveal_focus = true;
                        Action::Choose(revision, id.into())
                    }
                    Some(_) => {
                        table.context.dismiss();
                        table.dirty = true;
                        continue;
                    }
                    None => continue,
                }
            }
            action => action,
        };
        if !reveal_focus {
            shortcut.dismiss();
        }
        match action {
            Action::Choose(revision, id) => {
                let table = &mut *table;
                let before = table.session.observe();
                table.notice = match continuity::commit_choice(
                    &table.save_path,
                    &mut table.session,
                    revision,
                    &id,
                ) {
                    Ok(()) => {
                        consequences.committed(
                            &before,
                            &table.session.observe(),
                            &id,
                            table.reduced_motion,
                        );
                        if reveal_focus {
                            orbit.focus(layout::focus(Area::Test));
                        }
                        "Decision saved. Close and reopen at any choice.".into()
                    }
                    Err(e) => format!("Decision not committed: {e}"),
                };
            }
            Action::Focus(area) => {
                orbit.focus(layout::focus(area));
                table.context.dismiss();
            }
            Action::Select(revision, source, at) => {
                let view = table.view();
                if let Some(controls::SelectionEffect::Inspect(index)) =
                    table.context.select(&view, revision, source, at)
                {
                    table.inspect = Some(index);
                }
            }
            Action::Toggle(epoch, revision, id, cancel) => {
                let view = table.view();
                table.context.toggle(&view, epoch, revision, id, cancel);
            }
            Action::Inspect(index) => {
                let view = table.view();
                if let Some(index) = table.context.inspect(&view, index) {
                    table.inspect = Some(index);
                    table.history = false;
                }
            }
            Action::History => {
                table.history = true;
                table.context.dismiss();
            }
            Action::Close => {
                table.inspect = None;
                table.history = false;
            }
            Action::Dismiss => {
                table.context.close_menus();
            }
            Action::Utilities => {
                table.context.toggle_utilities();
            }
            Action::Motion => table.reduced_motion = !table.reduced_motion,
            Action::Save => {
                table.notice = match continuity::save_session(&table.save_path, &table.session) {
                    Ok(()) => "Live attempt saved.".into(),
                    Err(e) => format!("Save failed; live play is unchanged: {e}"),
                }
            }
            Action::Reopen => match continuity::load_session(&table.save_path) {
                Ok(session) => {
                    table.session = session;
                    consequences.replaced_view();
                    table.context.reset();
                    table.notice = "Reopened the exact saved decision and randomness.".into();
                }
                Err(e) => table.notice = format!("Reopen failed; live play is unchanged: {e}"),
            },
            Action::New if table.view().endpoint.is_some() => match Session::new() {
                Ok(session) => {
                    match continuity::restart_session(&table.save_path, &table.session, &session) {
                        Ok(()) => {
                            table.session = session;
                            consequences.replaced_view();
                            table.context.reset();
                            orbit.set_following(false);
                            table.notice = "New attempt; the previous save is archived.".into();
                        }
                        Err(e) => table.notice = format!("Could not start another attempt: {e}"),
                    }
                }
                Err(e) => table.notice = e.to_string(),
            },
            Action::AssetFolder => {
                settings.choose();
                table.notice =
                    "Choose a prepared party artwork folder. Restart to apply it.".into();
            }
            _ => {}
        }
        if let Some(focus) = focus.as_mut() {
            focus.clear();
        }

        table.invalidate();
    }
}

fn test_window(view: &Observation) -> bool {
    use crate::party::Stage;
    matches!(
        view.stage,
        Stage::Commit
            | Stage::BeforeReveal
            | Stage::Cancellation
            | Stage::Resolve
            | Stage::Lucky
            | Stage::Result
            | Stage::AfterTest
    )
}

fn refresh_reveal(table: &Table, settings: &AssetSettings, shortcut: &mut reveal::Shortcut) {
    let view = table.view();
    shortcut.sync(
        view.revision,
        &view.choices,
        !table.modal() && !settings.is_busy(),
        table.reduced_motion,
    );
}
fn sync_reveal(
    table: Res<Table>,
    settings: Res<AssetSettings>,
    mut shortcut: ResMut<reveal::Shortcut>,
) {
    refresh_reveal(&table, &settings, &mut shortcut);
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    layout::spawn_surfaces(&mut commands, &mut meshes, &mut materials);
    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(200., 200.))),
            MeshMaterial3d(materials.add(INK)),
            Transform::from_xyz(0., -0.5, 0.),
        ))
        .observe(|event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Dismiss);
            }
        });
    commands.spawn((
        DirectionalLight {
            illuminance: 7000.,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.)),
    ));
    commands.spawn((Camera3d::default(), Transform::default(), MainCamera));
}
fn synchronize_feedback(
    table: Res<Table>,
    settings: Res<AssetSettings>,
    art: Option<Res<Art>>,
    mut feedback: ResMut<FeedbackSettings>,
    mut cards: Query<(&CardSource, &mut CardFeedback)>,
) {
    feedback.blocked = table.modal() || settings.is_busy();
    feedback.reduced_motion = table.reduced_motion;
    let view = table.view();
    for (source, mut mark) in &mut cards {
        mark.menu_source = table.context.is_menu_source(source.0);
        mark.pending_choice = table.context.is_pending(source.0);
        mark.commitment = table.context.contribution(&view, source.0);
        mark.symbol_font = art.as_ref().and_then(|art| art.symbol_font.clone());
    }
}
#[allow(clippy::too_many_arguments)]
fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window>,
    mut table: ResMut<Table>,
    time: Res<PresentationTime>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    settings: Res<AssetSettings>,
    focus: Option<Res<InputFocus>>,
    focusable: Query<(), With<Button>>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    ui_nodes: Query<(), With<Node>>,
    mut shortcut: ResMut<reveal::Shortcut>,
) {
    let modal = table.modal() || settings.is_busy();
    let over_ui = hover
        .as_ref()
        .and_then(|h| h.get(&bevy::picking::pointer::PointerId::Mouse))
        .is_some_and(|hits| hits.keys().any(|e| ui_nodes.contains(*e)));
    let keyboard = modal
        || table.context.captures_keyboard()
        || shortcut.is_open()
        || focus
            .as_ref()
            .and_then(|f| f.get())
            .is_some_and(|e| focusable.contains(e));
    let step = orbit.step(FrameInput {
        keys: &keys,
        buttons: &buttons,
        window: &window,
        scroll_y: crate::tabletop_camera::wheel_steps(&scroll),
        dt: time.delta_secs(),
        capture: CameraCapture {
            pointer_blocked: modal,
            over_ui,
            keyboard,
            pan: None,
            wheel: table.context.is_open() || shortcut.is_open(),
        },
        reduced_motion: table.reduced_motion,
    });
    if step.manual_navigation {
        table.context.dismiss();
        shortcut.dismiss();
        table.dirty = true;
    }
    **camera = step.transform;
}
#[allow(clippy::type_complexity)]
fn world_labels(
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    orbit: Res<Orbit>,
    mut labels: Query<(
        &WorldLabel,
        &ComputedNode,
        Has<layout::TestAreaHeading>,
        Has<layout::ScenarioAreaHeading>,
        Has<crate::tabletop_pieces::CardCaption>,
        &mut Node,
        &mut Visibility,
    )>,
) {
    for (label, computed, test_heading, scenario_heading, card_caption, mut node, mut visibility) in
        &mut labels
    {
        if orbit.pose().distance > layout::DETAIL_LABEL_DISTANCE && !label.1 {
            *visibility = Visibility::Hidden;
            continue;
        }
        let point = if test_heading {
            layout::test_heading_screen_position(camera.0, camera.1, computed)
        } else if scenario_heading {
            layout::scenario_heading_screen_position(camera.0, camera.1, computed)
        } else {
            camera.0.world_to_viewport(camera.1, label.0).ok()
        };
        if let Some(point) = point {
            if card_caption
                && let Some(width) =
                    crate::tabletop_pieces::CardCaption::width(camera.0, camera.1, label.0)
            {
                node.width = px(width);
            }
            let width = match node.width {
                Val::Px(width) => width,
                _ => 190.0,
            };
            node.left = px(point.x - width / 2.0);
            node.top = px(point.y);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}
fn animate(
    time: Res<PresentationTime>,
    table: Res<Table>,
    mut transforms: Query<&mut Transform, With<Appearing>>,
) {
    for mut transform in &mut transforms {
        transform.scale = if table.reduced_motion {
            Vec3::ONE
        } else {
            let scale = transform
                .scale
                .lerp(Vec3::ONE, (time.delta_secs() * 8.).min(1.));
            if scale.abs_diff_eq(Vec3::ONE, 0.001) {
                Vec3::ONE
            } else {
                scale
            }
        };
    }
}
fn request_redraw(
    mut redraw: ResMut<Redraw>,
    orbit: Res<Orbit>,
    appearing: Query<&Transform, With<Appearing>>,
    consequences: consequences::Presentation,
    capture: Res<Capture>,
    screenshots: Query<(), With<Screenshot>>,
) {
    if orbit.is_moving()
        || appearing.iter().any(|t| t.scale != Vec3::ONE)
        || consequences.is_active()
        || (capture.path.is_some() && (!capture.requested || capture.quit))
        || !screenshots.is_empty()
    {
        redraw.request();
    }
}
fn scroll_panel(event: On<PointerScroll>, mut panels: Query<(&mut ScrollPosition, &ComputedNode)>) {
    if let Ok((mut position, node)) = panels.get_mut(event.entity) {
        let delta = event.y
            * if event.unit == MouseScrollUnit::Line {
                32.
            } else {
                1.
            };
        position.y =
            (position.y - delta).clamp(0., (node.content_size().y - node.size().y).max(0.));
    }
}
fn capture_window(
    health: Res<RenderHealth>,
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut capture: ResMut<Capture>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !health.allows_play() || health.just_became_ready() {
        return;
    }
    capture.frame += 1;
    if keys.just_pressed(KeyCode::F12) {
        let path = "local/experiment/party-tabletop.png";
        let _ = std::fs::create_dir_all("local/experiment");
        commands.spawn(Screenshot::primary_window()).observe(
            crate::tabletop_capture::save_to_disk(path.into(), health.diagnostic_elapsed()),
        );
    }
    if !capture.requested
        && capture.frame >= 90
        && let Some(path) = &capture.path
    {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        commands.spawn(Screenshot::primary_window()).observe(
            crate::tabletop_capture::save_to_disk(path.clone(), health.diagnostic_elapsed()),
        );
        capture.requested = true;
    }
    if capture.quit
        && capture.requested
        && capture.frame > 95
        && capturing.is_empty()
        && capture.path.as_ref().is_some_and(|p| p.is_file())
    {
        exit.write(AppExit::Success);
    }
}
#[cfg(test)]
mod tests;
