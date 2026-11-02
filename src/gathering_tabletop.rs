//! Native consumer of the Gathering decision seam. Rules remain in gathering::Session.

use crate::{
    asset_config::AssetConfig,
    gathering::{
        Investigator, Observation, PresentationMoment, Session,
        continuity::{self, Replay},
    },
    tabletop::asset_settings::AssetSettings,
    tabletop_camera::{CameraMode, Capture, FrameInput, Orbit},
    tabletop_feedback::{self as feedback, CardFeedback, FeedbackSettings},
    tabletop_health::{RenderHealth, TabletopHealthPlugin, exit_result},
    tabletop_layout::{self as layout, Area},
    tabletop_pacing::{PresentationTime, Redraw, RequestRedraw, TabletopPacingPlugin},
};
use bevy::{
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    input_focus::{
        AutoFocus, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
};
use std::path::PathBuf;

mod art;
mod controls;
mod motion;
mod play;
mod story;

const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.65, 0.74, 0.73);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);

#[derive(Clone)]
enum Action {
    Choose(u64, u64, String),
    Select(u64, u64, controls::Selection),
    ContextChoice(u64, u64, u64, String),
    ContextInspect(u64, u64, u64, String, String),
    DismissContext,
    PendingChoices,
    AssetFolder,
    Inspect(String, String),
    Save,
    Reopen,
    Replay,
    Next,
    Back,
    Live,
    History,
    Close,
    Focus(Area),
    Follow,
    PresentationNext,
    PresentationPause,
    PresentationFinish,
}

#[derive(Resource, Default)]
struct Actions(Vec<Action>);

#[derive(Resource)]
struct Table {
    session: Session,
    replay: Option<Replay>,
    save_path: PathBuf,
    notice: String,
    inspection: Option<(String, String)>,
    history_open: bool,
    epoch: u64,
    dirty: bool,
    live_follow: bool,
    presentation: Vec<PresentationMoment>,
    presentation_index: usize,
    presentation_clock: f32,
    presentation_paused: bool,
    last_rendered: Option<Observation>,
    last_rendered_health: Option<(bool, String)>,
    motion_origins: motion::Origins,
    context: controls::Context,
}

impl Table {
    fn new(session: Session, save_path: PathBuf) -> Self {
        Self {
            session,
            replay: None,
            save_path,
            notice: "Each decision saves before it takes effect.".into(),
            inspection: None,
            history_open: false,
            epoch: 0,
            dirty: true,
            live_follow: false,
            presentation: vec![],
            presentation_index: 0,
            presentation_clock: 0.,
            presentation_paused: false,
            last_rendered: None,
            last_rendered_health: None,
            motion_origins: motion::Origins::default(),
            context: controls::Context::default(),
        }
    }

    fn view(&self) -> Observation {
        if let Some(replay) = &self.replay {
            return replay.current().clone();
        }
        self.presentation.get(self.presentation_index).map_or_else(
            || self.session.observe_for_play(),
            |moment| self.session.observe_presentation(moment),
        )
    }

    fn is_presenting(&self) -> bool {
        self.replay.is_none() && self.presentation_index < self.presentation.len()
    }

    fn next_presentation(&mut self) {
        if self.is_presenting() {
            self.presentation_index += 1;
            self.presentation_clock = 0.;
            self.epoch += 1;
            self.dirty = true;
        }
    }

    fn modal(&self) -> bool {
        self.inspection.is_some() || self.history_open
    }

    fn story_faces(&self, view: &Observation) -> story::Faces {
        if let Some(replay) = &self.replay {
            story::Faces::for_view(
                replay.recording(),
                view,
                replay.current_presentation_sequence(),
            )
        } else {
            let sequence = self
                .presentation
                .get(self.presentation_index)
                .filter(|_| self.is_presenting())
                .map(|moment| moment.sequence);
            story::Faces::for_view(&self.session.recording(), view, sequence)
        }
    }
}

#[derive(Component)]
struct MainCamera;
#[derive(Component)]
struct ScenePiece;
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct WorldLabel(Vec3, bool);
#[derive(Component)]
struct HoverLabel;
#[derive(Component)]
struct ScrollablePanel {
    inspection: bool,
}

#[derive(Component)]
struct ContextTarget {
    epoch: u64,
    revision: u64,
    source: controls::Source,
}

struct CardRender<'a> {
    art: &'a art::Art,
    view: &'a Observation,
    epoch: u64,
    story: &'a story::Faces,
}

impl CardRender<'_> {
    fn bind(&self, commands: &mut Commands, entity: Entity, source: controls::Source) {
        commands.entity(entity).insert(ContextTarget {
            epoch: self.epoch,
            revision: self.view.revision,
            source,
        });
    }

    fn image(&self, code: &str) -> Option<Handle<Image>> {
        if self.story.is_reverse(code) {
            return self.art.card(code, true);
        }
        if let Some(play) = &self.view.play {
            if let Some(room) = play
                .scenario
                .locations
                .iter()
                .find(|room| room.code == code)
            {
                return self.art.card(code, !room.revealed);
            }
            if let Some(id) = code
                .strip_prefix("backpack:")
                .and_then(|id| id.parse::<u16>().ok())
            {
                return play
                    .players
                    .iter()
                    .flat_map(|player| &player.assets)
                    .find(|asset| asset.card.id == id)
                    .and_then(|asset| self.art.card(&asset.card.code, false));
            }
            if let Some(who) = code.strip_prefix("discard:") {
                return play
                    .players
                    .iter()
                    .find(|player| player.investigator.id() == who)
                    .and_then(|player| player.discard.last())
                    .and_then(|card| self.art.card(&card.code, false));
            }
            if code == "encounter-discard" {
                return play
                    .encounter_discard
                    .last()
                    .and_then(|card| self.art.card(&card.code, false));
            }
        }
        match code {
            "hidden-player-deck" => self.art.image("card-back/player"),
            "encounter-deck" => self.art.image("card-back/encounter"),
            "chaos-bag" => self.art.image("ui/reveal-chaos-token"),
            _ => self.art.card(code, false),
        }
    }

    fn rules(&self, code: &str) -> Option<String> {
        if self.story.is_reverse(code) {
            crate::gathering::inspection::scenario_card_reverse(code)
        } else {
            inspection_text(self.view, code)
        }
    }
}

struct Options {
    save: PathBuf,
    art_root: Option<PathBuf>,
    config: AssetConfig,
}

impl Options {
    fn parse(
        config: AssetConfig,
        mut args: impl Iterator<Item = String>,
    ) -> Result<Option<Self>, String> {
        let mut save = PathBuf::from("local/gathering/setup.session.json");
        let mut root = None;
        let mut no_art = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--experiment" => {
                    if args.next().as_deref() != Some("gathering") {
                        return Err("Use --experiment gathering.".into());
                    }
                }
                "--save" => save = args.next().ok_or("--save needs a path")?.into(),
                "--art-root" => {
                    root = Some(PathBuf::from(args.next().ok_or("--art-root needs a path")?))
                }
                "--no-art" => no_art = true,
                "--help" | "-h" => {
                    println!(
                        "The Gathering\n--experiment gathering\n--save PATH\n--art-root PREPARED_FOLDER\n--no-art\nReopens an existing session; a new path starts setup and scenario play. Artwork uses the saved folder unless overridden."
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown Gathering option: {arg}")),
            }
        }
        Ok(Some(Self {
            save,
            art_root: (!no_art).then(|| root.unwrap_or_else(|| config.root().to_path_buf())),
            config,
        }))
    }
}

pub fn run() -> Result<(), String> {
    let Some(options) = Options::parse(AssetConfig::load_gathering()?, std::env::args().skip(1))?
    else {
        return Ok(());
    };
    let save = options.save;
    let session = if save.exists() {
        continuity::load_session(&save).map_err(|error| error.to_string())?
    } else {
        let session = Session::new().map_err(|error| error.to_string())?;
        continuity::save_session(&save, &session).map_err(|error| error.to_string())?;
        session
    };
    if session.is_verification_fixture() {
        return Err(
            "This is a privileged verification fixture, not an ordinary Gathering setup.".into(),
        );
    }
    let prepared = art::Prepared::new(options.art_root)?;
    eprintln!(
        "Experiment: The Gathering (--experiment gathering)\nArtwork root: {}\n{}",
        prepared
            .root()
            .map_or_else(|| "disabled".into(), |path| path.display().to_string()),
        prepared.status()
    );
    let mut table = Table::new(session, save);
    if let Some(warning) = options.config.warning() {
        table.notice = format!("Asset settings: {warning}");
    }
    let mut app = App::new();
    app.insert_resource(ClearColor(INK))
        .insert_resource(table)
        .insert_resource(Orbit::new(CameraMode::Jacqueline))
        .insert_resource(AssetSettings::new_gathering(options.config))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Arkham Unfolded — Gathering Setup".into(),
                        resolution: (1600, 1000).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(prepared.asset_plugin()),
        )
        .add_plugins(MeshPickingPlugin)
        .add_plugins(TabletopPacingPlugin)
        .add_plugins(TabletopHealthPlugin);
    let art = prepared.load(app.world().resource::<AssetServer>());
    app.insert_resource(art.health_assets())
        .insert_resource(art);
    install(&mut app);
    exit_result(app.run())
}

/// The same registration is used by the native app and headless input tests.
fn install(app: &mut App) {
    controls::install(app);
    app.init_resource::<Actions>()
        .init_resource::<art::Art>()
        .add_plugins(feedback::FeedbackPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                poll_settings,
                handle_actions,
                advance_presentation,
                keyboard_panel_scroll,
                camera_controls,
                render,
                controls::render,
                contextual_feedback,
                motion::animate,
                world_labels,
            )
                .chain(),
        )
        .add_systems(Last, request_redraw.in_set(RequestRedraw));
}

fn poll_settings(mut settings: Option<ResMut<AssetSettings>>, mut table: ResMut<Table>) {
    if let Some(message) = settings.as_mut().and_then(|settings| settings.poll()) {
        table.notice = message;
        table.dirty = true;
    }
}

fn handle_actions(
    health: Res<RenderHealth>,
    mut actions: ResMut<Actions>,
    mut table: ResMut<Table>,
    mut orbit: ResMut<Orbit>,
    keys: Res<ButtonInput<KeyCode>>,
    mut feedback: ResMut<FeedbackSettings>,
    mut settings: Option<ResMut<AssetSettings>>,
) {
    let revision = table
        .last_rendered
        .as_ref()
        .map_or_else(|| table.view().revision, |view| view.revision);
    let epoch = table.epoch;
    let replay_index = table.replay.as_ref().map(Replay::index);
    table.context.sync(epoch, revision, replay_index);
    if !health.allows_play()
        || health.just_became_ready()
        || settings.as_ref().is_some_and(|settings| settings.is_busy())
    {
        table.context.reset();
    }
    if keys.just_pressed(KeyCode::Escape) {
        actions.0.push(Action::Close);
    }
    for action in actions.0.drain(..) {
        let action = match action {
            Action::ContextChoice(generation, epoch, revision, id)
                if table.context.accepts(generation) =>
            {
                Action::Choose(epoch, revision, id)
            }
            Action::ContextChoice(..) => continue,
            Action::ContextInspect(generation, epoch, revision, title, code)
                if table.context.accepts(generation)
                    && epoch == table.epoch
                    && revision == table.view().revision =>
            {
                Action::Inspect(title, code)
            }
            Action::ContextInspect(..) => continue,
            action => action,
        };
        if (!health.allows_play()
            || health.just_became_ready()
            || settings.as_ref().is_some_and(|settings| settings.is_busy()))
            && !matches!(
                action,
                Action::Focus(_) | Action::Close | Action::AssetFolder
            )
        {
            continue;
        }
        match action {
            Action::Choose(epoch, revision, id)
                if epoch == table.epoch
                    && table.replay.is_none()
                    && !table.modal()
                    && !table.is_presenting() =>
            {
                let path = table.save_path.clone();
                table.notice =
                    match continuity::commit_choice(&path, &mut table.session, revision, &id) {
                        Ok(()) => {
                            table.context.reset();
                            table.presentation =
                                table.session.presentation_after(revision).to_vec();
                            table.presentation_index = 0;
                            table.presentation_clock = 0.;
                            table.presentation_paused = false;
                            table.epoch += 1;
                            orbit.update_follow_target(Some(layout::focus(play::focus(
                                &table.view(),
                            ))));
                            "Decision saved.".into()
                        }
                        Err(error) => format!("Decision not committed: {error}"),
                    };
                table.dirty = true;
            }
            Action::Choose(_, _, _) => {}
            Action::Select(epoch, revision, selection)
                if epoch == table.epoch && revision == table.view().revision && !table.modal() =>
            {
                let view = table.view();
                let live = table.replay.is_none() && !table.is_presenting();
                if let Some(effect) = table.context.select(&view, selection, live) {
                    if let controls::SelectionEffect::Inspect(title, code) = effect {
                        table.inspection = Some((title, code));
                        table.epoch += 1;
                    }
                    table.dirty = true;
                }
            }
            Action::Select(..) | Action::ContextChoice(..) | Action::ContextInspect(..) => {}
            Action::DismissContext => table.context.dismiss(),
            Action::PendingChoices
                if table.replay.is_none() && !table.modal() && !table.is_presenting() =>
            {
                table.context.open_pending();
            }
            Action::PendingChoices => {}
            Action::AssetFolder => {
                if let Some(settings) = settings.as_mut() {
                    table.context.reset();
                    settings.choose();
                    table.notice =
                        "Choose a prepared Gathering artwork folder. Restart to apply it.".into();
                    table.dirty = true;
                }
            }
            Action::Focus(area) => {
                table.context.dismiss();
                orbit.focus(layout::focus(area));
                if table.is_presenting() {
                    table.presentation_paused = true;
                }
                table.dirty = true;
            }
            Action::Follow => {
                table.context.dismiss();
                let following = !orbit.is_following();
                orbit.set_following(following);
                orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
                table.dirty = true;
            }
            Action::PresentationNext if !table.modal() => {
                table.next_presentation();
                orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
            }
            Action::PresentationPause if table.is_presenting() => {
                table.presentation_paused = !table.presentation_paused;
                table.dirty = true;
            }
            Action::PresentationFinish if !table.modal() && table.is_presenting() => {
                table.presentation_index = table.presentation.len();
                table.presentation_clock = 0.;
                table.epoch += 1;
                table.dirty = true;
                orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
            }
            Action::PresentationNext | Action::PresentationPause | Action::PresentationFinish => {}
            Action::Close => {
                table.context.reset();
                table.epoch += 1;
                table.inspection = None;
                table.history_open = false;
                table.dirty = true;
            }
            Action::Inspect(title, code) => {
                table.context.reset();
                table.epoch += 1;
                table.inspection = Some((title, code));
                table.history_open = false;
                table.dirty = true;
            }
            Action::History => {
                table.context.reset();
                table.epoch += 1;
                table.history_open = true;
                table.inspection = None;
                table.dirty = true;
            }
            Action::Save if table.replay.is_none() => {
                table.notice = match continuity::save_session(&table.save_path, &table.session) {
                    Ok(()) => "Saved the live decision.".into(),
                    Err(error) => format!("Save failed: {error}"),
                };
                table.dirty = true;
            }
            Action::Reopen if table.replay.is_none() => {
                match continuity::load_session(&table.save_path) {
                    Ok(session) if !session.is_verification_fixture() => {
                        table.context.reset();
                        table.epoch += 1;
                        if session != table.session {
                            table.presentation.clear();
                            table.presentation_index = 0;
                            table.presentation_clock = 0.;
                        }
                        table.session = session;
                        table.inspection = None;
                        table.history_open = false;
                        table.notice = "Reopened the saved decision.".into();
                    }
                    Ok(_) => {
                        table.notice =
                            "Reopen refused: this record is a privileged verification fixture."
                                .into()
                    }
                    Err(error) => table.notice = format!("Reopen failed: {error}"),
                }
                table.dirty = true;
            }
            Action::Replay => {
                match Replay::new(table.session.recording()) {
                    Ok(replay) => {
                        table.context.reset();
                        table.live_follow = orbit.is_following();
                        table.epoch += 1;
                        table.replay = Some(replay);
                        table.inspection = None;
                        table.history_open = false;
                        orbit.set_following(true);
                        orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
                    }
                    Err(error) => table.notice = format!("Replay unavailable: {error}"),
                }
                table.dirty = true;
            }
            Action::Next => {
                if let Some(replay) = &mut table.replay {
                    replay.next();
                    table.dirty = true;
                    orbit.set_following(true);
                    orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
                }
            }
            Action::Back => {
                if let Some(replay) = &mut table.replay {
                    replay.back();
                    table.dirty = true;
                    orbit.set_following(true);
                    orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
                }
            }
            Action::Live => {
                table.context.reset();
                table.epoch += 1;
                table.replay = None;
                table.inspection = None;
                table.history_open = false;
                table.dirty = true;
                orbit.set_following(table.live_follow);
                orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
            }
            Action::Reopen | Action::Save => {}
        }
    }
    feedback.blocked =
        table.modal() || settings.as_ref().is_some_and(|settings| settings.is_busy());
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    layout::spawn_surfaces(&mut commands, &mut meshes, &mut materials);
    commands.spawn((
        DirectionalLight {
            illuminance: 7000.,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.)),
    ));
    commands.spawn((Camera3d::default(), Transform::default(), MainCamera));
}

fn text(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    crate::tabletop_harm_text::ui(value, size, color, &None)
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, action: Action) -> Entity {
    parent
        .spawn((
            Button,
            Hovered::default(),
            TabIndex(0),
            AccessibleLabel::new(label),
            feedback::context_button(),
            Node {
                min_height: px(32),
                padding: UiRect::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.19, 0.21)),
        ))
        .observe(move |_: On<Activate>, mut queue: ResMut<Actions>| queue.0.push(action.clone()))
        .with_children(|p| {
            p.spawn(text(label, 15., PAPER));
        })
        .id()
}

#[allow(clippy::too_many_arguments)]
fn card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    at: Vec3,
    title: &str,
    code: &str,
    selected: bool,
    color: Color,
) -> Entity {
    let title = title.to_owned();
    let code = code.to_owned();
    let image = context.image(&code);
    let has_art = image.is_some();
    let landscape = matches!(code.as_str(), "06003" | "06005" | "60401");
    let (width, length) = if landscape { (2.6, 1.85) } else { (1.35, 1.89) };
    let entity = commands
        .spawn((
            ScenePiece,
            Mesh3d(meshes.add(Cuboid::new(width, 0.08, length))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(at),
        ))
        .observe({
            let title = title.clone();
            let code = code.clone();
            move |event: On<PointerClick>,
                  targets: Query<&ContextTarget>,
                  mut queue: ResMut<Actions>| {
                if event.button == bevy::picking::pointer::PointerButton::Primary {
                    queue.0.push(if let Ok(target) = targets.get(event.entity) {
                        Action::Select(
                            target.epoch,
                            target.revision,
                            controls::Selection {
                                source: target.source.clone(),
                                at,
                                title: title.clone(),
                                inspect_code: code.clone(),
                            },
                        )
                    } else {
                        Action::Inspect(title.clone(), code.clone())
                    });
                }
            }
        })
        .id();
    if let Some(image) = image {
        let (face_width, face_length, rotation) = if landscape {
            (
                length,
                width,
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            )
        } else {
            (width, length, Quat::IDENTITY)
        };
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Plane3d::default().mesh().size(face_width, face_length))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color_texture: Some(image),
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0., 0.045, 0.).with_rotation(rotation),
                Pickable::IGNORE,
            ));
        });
    }
    feedback::attach_card_feedback(commands, meshes, materials, entity, width, length, 0.04);
    commands.entity(entity).insert(CardFeedback {
        pending_choice: selected,
        symbol_font: context.art.symbol_font().clone(),
        ..default()
    });
    let label = commands
        .spawn((
            ScenePiece,
            WorldLabel(at + Vec3::Y * 0.1, false),
            motion::CardLabel(entity),
            context.art.text(title, 13., INK),
            Node {
                position_type: PositionType::Absolute,
                width: px(130),
                padding: UiRect::all(px(3)),
                ..default()
            },
            BackgroundColor(PAPER),
            Pickable::IGNORE,
        ))
        .id();
    if has_art {
        commands.entity(label).insert(HoverLabel);
    }
    entity
}

fn inspection_image(
    parent: &mut ChildSpawnerCommands,
    image: Handle<Image>,
    code: &str,
    available_width: f32,
) {
    let landscape = matches!(
        code,
        "06003" | "06005" | "60401" | "01105" | "01106" | "01107" | "01108" | "01109" | "01110"
    );
    let width = available_width
        .max(160.)
        .min(if landscape { 760. } else { 430. });
    let height = width * if landscape { 5. / 7. } else { 7. / 5. };
    parent
        .spawn(Node {
            width: px(width),
            height: px(height),
            flex_shrink: 0.,
            align_self: AlignSelf::Center,
            ..default()
        })
        .with_children(|frame| {
            frame.spawn((
                ImageNode::new(image),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: percent(50),
                    width: px(if landscape { height } else { width }),
                    height: px(if landscape { width } else { height }),
                    ..default()
                },
                UiTransform {
                    translation: Val2::percent(-50, -50),
                    rotation: if landscape {
                        Rot2::degrees(-90.)
                    } else {
                        Rot2::IDENTITY
                    },
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

fn inspection_text(view: &Observation, code: &str) -> Option<String> {
    if view.play.is_some() {
        return play::inspect(view, code);
    }
    if let Some(text) = crate::gathering::inspection::card(code) {
        return Some(text);
    }
    let scenario = view.scenario.as_ref()?;
    if code == "encounter-deck" {
        let cards = scenario
            .encounter_inventory
            .iter()
            .map(|card| format!("{} × {} · {}", card.quantity, card.title, card.code))
            .collect::<Vec<_>>()
            .join("\n");
        return Some(format!(
            "Initial encounter deck: 26 cards\n{cards}\n\nThe other 14 gathered cards are the scenario reference, three agendas, three acts, Study, and six set-aside cards. This public setup inventory does not reveal shuffled order."
        ));
    }
    let visible = code == "01111"
        || code == scenario.reference
        || scenario.agenda.first().is_some_and(|front| front == code)
        || scenario.act.first().is_some_and(|front| front == code);
    visible
        .then(|| crate::gathering::inspection::scenario_card(code, code == "01111"))
        .flatten()
}

type RenderedContent = Or<(With<ScenePiece>, With<Panel>)>;
type PriorRenderedCard<'a> = (
    Entity,
    Option<&'a motion::PhysicalCard>,
    Option<&'a Transform>,
    Option<&'a motion::CardMotion>,
);

#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    mut table: ResMut<Table>,
    health: Res<RenderHealth>,
    art: Res<art::Art>,
    old: Query<PriorRenderedCard<'_>, RenderedContent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    orbit: Res<Orbit>,
    mut focus: Option<ResMut<InputFocus>>,
    window: Single<&Window>,
) {
    // Health sampling changes every frame. Rebuilding for those observations
    // destroys the controls between pointer hover, press, and release.
    let health_changed = table
        .last_rendered_health
        .as_ref()
        .is_none_or(|(allowed, status)| {
            *allowed != health.allows_play() || status != health.status()
        });
    if !table.dirty && !health_changed {
        return;
    }
    table.last_rendered_health = Some((health.allows_play(), health.status().to_owned()));
    table.dirty = false;
    if !table.context.is_open()
        && let Some(focus) = focus.as_mut()
    {
        focus.clear();
    }
    let mut origins = motion::Origins::from_view(table.last_rendered.as_ref());
    for (entity, card, transform, motion) in &old {
        if let (Some(card), Some(transform)) = (card, transform) {
            origins.remember(card.0, transform.translation, motion);
        }
        commands.entity(entity).despawn();
    }
    table.motion_origins = origins;
    let view = table.view();
    table.last_rendered = Some(view.clone());
    let story = table.story_faces(&view);
    let context = CardRender {
        art: &art,
        view: &view,
        epoch: table.epoch,
        story: &story,
    };
    for (index, investigator) in view.investigators.iter().enumerate() {
        for (slot, hand) in investigator.hand.iter().enumerate() {
            let entity = card(
                &mut commands,
                &mut meshes,
                &mut materials,
                &context,
                layout::hand_position(index, investigator.hand.len(), slot),
                &format!("{} · copy {}", hand.title, hand.id),
                &hand.code,
                investigator.selected_mulligan.contains(&hand.id)
                    || view
                        .play
                        .as_ref()
                        .and_then(|play| play.test.as_ref())
                        .is_some_and(|test| test.selected.contains(&hand.id)),
                PAPER,
            );
            commands
                .entity(entity)
                .insert(motion::PhysicalCard(hand.id));
            context.bind(&mut commands, entity, controls::Source::Card(hand.id));
        }
        for (slot, permanent) in investigator
            .permanents
            .iter()
            .enumerate()
            .filter(|_| view.play.is_none())
        {
            let entity = card(
                &mut commands,
                &mut meshes,
                &mut materials,
                &context,
                layout::investigator_asset(index, slot),
                &permanent.title,
                &permanent.code,
                false,
                PAPER,
            );
            commands
                .entity(entity)
                .insert(motion::PhysicalCard(permanent.id));
            context.bind(&mut commands, entity, controls::Source::Card(permanent.id));
        }
        let entity = card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &context,
            layout::investigator_card(index),
            investigator.investigator.name(),
            match investigator.investigator {
                Investigator::Tony => "06003",
                Investigator::Patrice => "06005",
                Investigator::Jacqueline => "60401",
            },
            false,
            layout::investigator_color(index),
        );
        context.bind(
            &mut commands,
            entity,
            controls::Source::Investigator(investigator.investigator),
        );
        commands.spawn((
            ScenePiece,
            WorldLabel(layout::investigator_heading(index), true),
            art.text(
                view.play.as_ref().map_or_else(
                    || {
                        format!(
                            "{}\nResources {} · Deck {} · Damage {} · Horror {}",
                            investigator.investigator.name(),
                            investigator.resources,
                            investigator.deck_count,
                            investigator.damage,
                            investigator.horror
                        )
                    },
                    |play| play::player_status(play, index),
                ),
                15.,
                PAPER,
            ),
            Node {
                position_type: PositionType::Absolute,
                width: px(260),
                ..default()
            },
            Pickable::IGNORE,
        ));
    }
    if view.play.is_some() {
        play::render(&mut commands, &mut meshes, &mut materials, &context);
    } else if let Some(scenario) = &view.scenario {
        for (side, codes, label) in [
            (
                layout::ScenarioBookSide::Agenda,
                &scenario.agenda,
                "Agenda 1",
            ),
            (layout::ScenarioBookSide::Act, &scenario.act, "Act 1"),
        ] {
            if let Some(code) = codes.first() {
                let pose = layout::scenario_book_card_transform(side, Vec2::new(1.35, 1.89));
                let entity = card(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &context,
                    pose.translation,
                    label,
                    code,
                    false,
                    PAPER,
                );
                commands.entity(entity).insert(pose);
                context.bind(
                    &mut commands,
                    entity,
                    match side {
                        layout::ScenarioBookSide::Agenda => controls::Source::Agenda,
                        layout::ScenarioBookSide::Act => controls::Source::Act,
                    },
                );
            }
        }
        let reference = card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &context,
            layout::SCENARIO_REFERENCE,
            "The Gathering · Standard",
            &scenario.reference,
            false,
            PAPER,
        );
        context.bind(
            &mut commands,
            reference,
            controls::Source::ScenarioReference,
        );
        let study = card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &context,
            layout::SCENARIO_MAP_CENTER,
            &format!("Study · {} clues", scenario.study_clues),
            "01111",
            false,
            PAPER,
        );
        context.bind(
            &mut commands,
            study,
            controls::Source::Location("01111".into()),
        );
        let encounter = card(
            &mut commands,
            &mut meshes,
            &mut materials,
            &context,
            layout::ENCOUNTER_DECK,
            &format!("Encounter deck · {} cards", scenario.encounter_deck_count),
            "encounter-deck",
            false,
            MUTED,
        );
        context.bind(&mut commands, encounter, controls::Source::EncounterDeck);
    }
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                top: px(10),
                left: px(12),
                right: px(12),
                padding: UiRect::all(px(10)),
                flex_direction: FlexDirection::Row,
                column_gap: px(12),
                ..default()
            },
            BackgroundColor(INK),
            TabGroup::new(0),
        ))
        .with_children(|p| {
            p.spawn(text(
                if view.play.is_some() {
                    "The Gathering · Standard"
                } else {
                    "The Gathering · Standard · Setup"
                },
                22.,
                GOLD,
            ));
            for (label, area) in [
                ("Overview", Area::Overview),
                ("Scenario", Area::Scenario),
                ("Test", Area::Test),
                ("Tony", Area::Investigator(0)),
                ("Patrice", Area::Investigator(1)),
                ("Jacqueline", Area::Investigator(2)),
            ] {
                let entity = button(p, label, Action::Focus(area));
                if area == Area::Overview
                    && !table.modal()
                    && !table.context.is_open()
                    && controls::global_choices(&view).is_empty()
                {
                    p.commands().entity(entity).insert(AutoFocus);
                }
            }
            button(
                p,
                if orbit.is_following() {
                    "Automatic camera: on"
                } else {
                    "Automatic camera: off"
                },
                Action::Follow,
            );
        });
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                right: px(12),
                bottom: px(12),
                max_height: percent(34),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(INK),
            TabGroup::new(1),
        ))
        .insert((
            ScrollPosition::default(),
            ScrollablePanel { inspection: false },
        ))
        .observe(scroll_panel)
        .with_children(|p| {
            p.spawn(text(art.status(), 12., MUTED));
            if !health.allows_play() {
                p.spawn(text(health.status(), 20., GOLD));
                button(p, "Choose asset folder", Action::AssetFolder);
                return;
            }
            if let Some(replay) = &table.replay {
                p.spawn(text(
                    format!(
                        "Recorded replay · {} / {}",
                        replay.index() + 1,
                        replay.len()
                    ),
                    18.,
                    GOLD,
                ));
            }
            p.spawn(text(&view.prompt, 18., PAPER));
            if table.replay.is_none() && !table.is_presenting() && !view.choices.is_empty() {
                p.spawn(text(
                    "Click a card or table component for its available actions.",
                    13.,
                    MUTED,
                ));
            }
            if table.is_presenting() {
                p.spawn(text(
                    format!(
                        "Event {} of {} · The decision is saved",
                        table.presentation_index + 1,
                        table.presentation.len()
                    ),
                    14.,
                    MUTED,
                ));
            }
            if let Some(status) = play::status(&view) {
                p.spawn(text(status, 16., GOLD));
            }
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(8),
                row_gap: px(5),
                ..default()
            })
            .with_children(|row| {
                if let Some(replay) = &table.replay {
                    if replay.index() > 0 {
                        button(row, "Previous moment", Action::Back);
                    }
                    if replay.index() + 1 < replay.len() {
                        button(row, "Next moment", Action::Next);
                    }
                    button(row, "Return to live table", Action::Live);
                } else if table.is_presenting() && !table.modal() {
                    button(row, "Next event", Action::PresentationNext);
                    button(
                        row,
                        if table.presentation_paused {
                            "Resume presentation"
                        } else {
                            "Pause presentation"
                        },
                        Action::PresentationPause,
                    );
                    button(row, "Show pending choice", Action::PresentationFinish);
                } else if !table.modal() {
                    for (index, choice) in controls::global_choices(&view).into_iter().enumerate() {
                        let entity = button(
                            row,
                            &choice.label,
                            Action::Choose(table.epoch, view.revision, choice.id.clone()),
                        );
                        if index == 0 {
                            row.commands().entity(entity).insert(AutoFocus);
                        }
                    }
                    if !controls::pending_choices(&view).is_empty() {
                        button(row, "Pending decision", Action::PendingChoices);
                    }
                }
            });
            p.spawn(text(&table.notice, 12., MUTED));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                ..default()
            })
            .with_children(|row| {
                button(row, "History", Action::History);
                button(row, "Choose asset folder", Action::AssetFolder);
                if table.replay.is_none() {
                    button(row, "Save", Action::Save);
                    button(row, "Reopen", Action::Reopen);
                    button(row, "Replay", Action::Replay);
                }
            });
        });
    if table.modal() {
        commands
            .spawn((
                Panel,
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(12),
                    right: percent(12),
                    top: percent(10),
                    bottom: percent(18),
                    padding: UiRect::all(px(20)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(INK),
                ScrollPosition::default(),
                ScrollablePanel { inspection: true },
                TabGroup::modal(),
            ))
            .observe(scroll_panel)
            .with_children(|p| {
                let close = button(p, "Close", Action::Close);
                p.commands().entity(close).insert(AutoFocus);
                if let Some((title, code)) = &table.inspection {
                    p.spawn(text(title, 24., GOLD));
                    if let Some(image) = context.image(code) {
                        inspection_image(p, image, code, window.width() * 0.76 - 40.);
                    }
                    if code.len() == 5 && code.bytes().all(|byte| byte.is_ascii_digit()) {
                        p.spawn(text(format!("Card {code}"), 14., MUTED));
                    }
                    if let Some(rules) = context.rules(code) {
                        p.spawn(art.text(rules, 17., PAPER));
                    }
                    if let Some(current) = story::StoryCard::from_code(code) {
                        for previous in story
                            .known_reverses(current.side())
                            .filter(|card| *card != current)
                        {
                            p.spawn(text(
                                format!("Previously revealed · {}", previous.title()),
                                22.,
                                GOLD,
                            ));
                            if let Some(image) = art.card(previous.code(), true) {
                                inspection_image(
                                    p,
                                    image,
                                    previous.code(),
                                    window.width() * 0.76 - 40.,
                                );
                            }
                            if let Some(rules) =
                                crate::gathering::inspection::scenario_card_reverse(previous.code())
                            {
                                p.spawn(art.text(rules, 17., PAPER));
                            }
                        }
                    }
                } else {
                    p.spawn(text("Action history", 22., GOLD));
                    for line in &view.history {
                        p.spawn(text(line, 15., PAPER));
                    }
                }
                button(p, "Close", Action::Close);
            });
    }
}

fn scroll_panel(event: On<PointerScroll>, mut nodes: Query<(&mut ScrollPosition, &ComputedNode)>) {
    if let Ok((mut scroll, node)) = nodes.get_mut(event.entity) {
        let delta = event.y
            * if event.unit == MouseScrollUnit::Line {
                32.
            } else {
                1.
            };
        scroll.y = (scroll.y - delta).clamp(0., (node.content_size().y - node.size().y).max(0.));
    }
}

fn keyboard_panel_scroll(
    keys: Res<ButtonInput<KeyCode>>,
    table: Res<Table>,
    mut panels: Query<(&ScrollablePanel, &mut ScrollPosition, &ComputedNode)>,
) {
    if table.context.captures_keyboard() {
        return;
    }
    for (panel, mut scroll, node) in &mut panels {
        if panel.inspection != table.modal() {
            continue;
        }
        scroll_with_keys(&keys, &mut scroll, node);
    }
}

fn scroll_with_keys(keys: &ButtonInput<KeyCode>, scroll: &mut ScrollPosition, node: &ComputedNode) {
    let maximum = (node.content_size().y - node.size().y).max(0.);
    if keys.just_pressed(KeyCode::PageDown) {
        scroll.y = (scroll.y + node.size().y * 0.8).min(maximum);
    }
    if keys.just_pressed(KeyCode::PageUp) {
        scroll.y = (scroll.y - node.size().y * 0.8).max(0.);
    }
    if keys.just_pressed(KeyCode::End) {
        scroll.y = maximum;
    }
    if keys.just_pressed(KeyCode::Home) {
        scroll.y = 0.;
    }
}

fn advance_presentation(
    time: Res<PresentationTime>,
    health: Res<RenderHealth>,
    mut table: ResMut<Table>,
    mut orbit: ResMut<Orbit>,
) {
    if !health.allows_play() || table.modal() || table.presentation_paused || !table.is_presenting()
    {
        return;
    }
    table.presentation_clock += time.delta_secs();
    let duration = table
        .presentation
        .get(table.presentation_index)
        .map_or(2., |moment| {
            (1. + moment.caption.chars().count() as f32 / 40.).clamp(2., 6.)
        });
    if table.presentation_clock >= duration {
        table.next_presentation();
        orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
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
    focus: Option<Res<InputFocus>>,
    focusable: Query<(), With<Button>>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    ui_nodes: Query<(), With<Node>>,
    settings: Option<Res<AssetSettings>>,
) {
    let blocked = table.modal() || settings.as_ref().is_some_and(|settings| settings.is_busy());
    let over_ui = hover
        .as_ref()
        .and_then(|hover| hover.get(&bevy::picking::pointer::PointerId::Mouse))
        .is_some_and(|hits| hits.keys().any(|entity| ui_nodes.contains(*entity)));
    let step = orbit.step(FrameInput {
        keys: &keys,
        buttons: &buttons,
        window: &window,
        scroll_y: crate::tabletop_camera::wheel_steps(&scroll),
        dt: time.delta_secs(),
        capture: Capture {
            pointer_blocked: blocked,
            over_ui,
            keyboard: blocked
                || table.context.captures_keyboard()
                || focus
                    .as_ref()
                    .and_then(|f| f.get())
                    .is_some_and(|entity| focusable.contains(entity)),
            wheel: blocked || table.context.is_open() || over_ui,
        },
        reduced_motion: false,
    });
    **camera = step.transform;
    if step.manual_navigation && table.context.is_open() {
        table.context.dismiss();
    }
    if step.stopped_following {
        table.dirty = true;
    }
    if step.manual_navigation && table.is_presenting() {
        table.presentation_paused = true;
        table.dirty = true;
    }
}

#[allow(clippy::type_complexity)]
fn world_labels(
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    orbit: Res<Orbit>,
    mut labels: Query<(
        &WorldLabel,
        Option<&motion::CardLabel>,
        Option<&HoverLabel>,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
    )>,
    cards: Query<&Transform>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
) {
    let hovered = hover
        .as_ref()
        .and_then(|map| map.get(&bevy::picking::pointer::PointerId::Mouse));
    for (label, card, hover_only, computed, mut node, mut visibility) in &mut labels {
        let anchor = card
            .and_then(|card| cards.get(card.0).ok())
            .map_or(label.0, |transform| transform.translation + Vec3::Y * 0.1);
        let hidden = hover_only.is_some()
            && !card.is_some_and(|card| hovered.is_some_and(|hits| hits.contains_key(&card.0)));
        if hidden || (orbit.pose().distance > layout::DETAIL_LABEL_DISTANCE && !label.1) {
            *visibility = Visibility::Hidden;
        } else if let Ok(point) = camera.0.world_to_viewport(camera.1, anchor) {
            let width = if let Val::Px(width) = node.width {
                width
            } else {
                130.
            };
            let viewport = camera
                .0
                .logical_viewport_size()
                .unwrap_or(Vec2::new(1280., 720.));
            node.left = px((point.x - width / 2.).clamp(8., (viewport.x - width - 8.).max(8.)));
            node.top = px(point.y.clamp(
                72.,
                (viewport.y - computed.size().y * computed.inverse_scale_factor - 100.).max(72.),
            ));
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn contextual_feedback(table: Res<Table>, mut cards: Query<(&ContextTarget, &mut CardFeedback)>) {
    for (target, mut feedback) in &mut cards {
        feedback.menu_source = table.context.is_menu_source(&target.source);
    }
}

fn request_redraw(
    orbit: Res<Orbit>,
    health: Res<RenderHealth>,
    table: Res<Table>,
    moving: Query<(), With<motion::CardMotion>>,
    mut redraw: ResMut<Redraw>,
) {
    if (!moving.is_empty()
        && !table.modal()
        && !(table.is_presenting() && table.presentation_paused))
        || orbit.is_moving()
        || health.is_checking()
        || (table.is_presenting() && !table.presentation_paused && !table.modal())
    {
        redraw.request();
    }
}

#[cfg(test)]
mod tests;
