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
    tabletop_pieces::WorldLabel,
};
use bevy::{
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    input_focus::{AutoFocus, InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui_widgets::{Activate, Button},
};
use std::path::PathBuf;

mod art;
mod consequences;
mod controls;
mod motion;
mod play;
mod reveal;
mod scene;
mod story;
mod ui;

const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.65, 0.74, 0.73);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);

#[derive(Clone)]
enum Action {
    ContinueStory,
    Reveal(crate::tabletop_reveal::Request),
    Choose(u64, u64, String),
    Select(u64, u64, controls::Selection),
    ContextChoice(u64, u64, u64, String),
    ContextInspect(u64, u64, u64, String, String),
    DismissContext,
    PendingChoices,
    Utilities,
    Dismiss,
    Motion,
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
    menu_open: bool,
    reduced_motion: bool,
    reading: story::Reading,
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
        let reading = story::Reading::for_record(&session.recording());
        Self {
            session,
            replay: None,
            save_path,
            notice: "Each decision saves before it takes effect.".into(),
            inspection: None,
            history_open: false,
            menu_open: false,
            reduced_motion: false,
            reading,
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
        self.reading.current().is_some() || self.inspection.is_some() || self.history_open
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
    consequences: &'a consequences::State,
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
        if let Some((code, _)) = play::enemy_inspection(self.view, code) {
            return self.art.card(&code, false);
        }
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
            if let Some(who) = code.strip_prefix("removed:") {
                return play
                    .players
                    .iter()
                    .find(|player| player.investigator.id() == who)
                    .and_then(|player| player.removed.last())
                    .and_then(|card| self.art.card(&card.code, false));
            }
            let public_top = match code {
                "victory" => play.scenario.victory.last(),
                "scenario-removed" => play.scenario.removed.last(),
                "searched-player" => play.searched_player.last(),
                "searched-encounter" => play.searched_encounter.last(),
                "resolving-cards" => play.pending.resolving_cards.last(),
                "pending-discard" => play.pending.discard.as_ref().and_then(|discard| {
                    discard.ordered.last().or_else(|| discard.remaining.last())
                }),
                _ => None,
            };
            if let Some(card) = public_top {
                return self.art.card(&card.code, false);
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
        if let Some((code, details)) = play::enemy_inspection(self.view, code) {
            return Some(format!(
                "{details}\n\n{}",
                inspection_text(self.view, &code).unwrap_or_default()
            ));
        }
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
        .init_resource::<consequences::State>()
        .init_resource::<art::Art>()
        .add_plugins(feedback::FeedbackPlugin)
        .add_plugins(crate::tabletop_counters::CounterPlugin)
        .add_plugins(crate::tabletop_reveal::RevealPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                poll_settings,
                handle_actions,
                consequences::advance,
                reveal::sync,
                crate::tabletop_reveal::render,
                advance_presentation,
                keyboard_panel_scroll,
                camera_controls,
                render,
                controls::render,
                contextual_feedback,
                motion::animate,
                world_labels,
                crate::tabletop_tokens::position_labels,
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

#[allow(clippy::too_many_arguments)]
fn handle_actions(
    health: Res<RenderHealth>,
    mut actions: ResMut<Actions>,
    mut table: ResMut<Table>,
    mut orbit: ResMut<Orbit>,
    keys: Res<ButtonInput<KeyCode>>,
    mut feedback: ResMut<FeedbackSettings>,
    mut settings: Option<ResMut<AssetSettings>>,
    mut shortcut: ResMut<crate::tabletop_reveal::Shortcut>,
    mut consequences: ResMut<consequences::State>,
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
    actions
        .0
        .extend(shortcut.drain().into_iter().map(Action::Reveal));
    for action in actions.0.drain(..) {
        let mut reveal_focus = false;
        let action = match action {
            Action::Reveal(request) => {
                reveal::refresh(&table, settings.as_deref(), &health, &mut shortcut);
                match shortcut.admit(request) {
                    Some(crate::tabletop_reveal::Accepted::Choice(revision, id)) => {
                        reveal_focus = true;
                        Action::Choose(table.epoch, revision, id.into())
                    }
                    Some(_) => {
                        table.context.dismiss();
                        table.dirty = true;
                        continue;
                    }
                    None => continue,
                }
            }
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
        // The same dispatch boundary as Party: a dialog captures actions that
        // were already queued before its barrier appeared on screen.
        let reading = table.reading.current().is_some();
        let permitted_in_modal = if reading {
            matches!(action, Action::ContinueStory)
                || (table.replay.is_some()
                    && matches!(action, Action::Next | Action::Back | Action::Live))
        } else {
            matches!(action, Action::Close)
        };
        if table.modal() && !permitted_in_modal {
            continue;
        }
        if consequences.active()
            && matches!(
                action,
                Action::Choose(..) | Action::PresentationNext | Action::PresentationFinish
            )
        {
            continue;
        }
        if !reveal_focus {
            shortcut.dismiss();
        }
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
            Action::ContinueStory => {
                table.reading.advance();
                table.dirty = true;
            }
            Action::Utilities => {
                table.menu_open = !table.menu_open;
                table.context.dismiss();
                table.dirty = true;
            }
            Action::Dismiss => {
                table.menu_open = false;
                table.context.dismiss();
                table.dirty = true;
            }
            Action::Motion => {
                table.reduced_motion = !table.reduced_motion;
                table.dirty = true;
            }
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
                            if reveal_focus {
                                orbit.focus(layout::focus(Area::Test));
                            }
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
            Action::Select(..)
            | Action::ContextChoice(..)
            | Action::ContextInspect(..)
            | Action::Reveal(..) => {}
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
                table.menu_open = false;
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
                        consequences.clear();
                        table.context.reset();
                        table.epoch += 1;
                        if session != table.session {
                            table.reading = story::Reading::for_record(&session.recording());
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
                consequences.clear();
                match Replay::new(table.session.recording()) {
                    Ok(replay) => {
                        table.context.reset();
                        table.live_follow = orbit.is_following();
                        table.epoch += 1;
                        table.replay = Some(replay);
                        table.reading = story::Reading::at(table.story_faces(&table.view()));
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
                consequences.clear();
                table.reading.dismiss_for_navigation();
                if let Some(replay) = &mut table.replay {
                    replay.next();
                    table.dirty = true;
                    orbit.set_following(true);
                    orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
                }
            }
            Action::Back => {
                consequences.clear();
                table.reading.dismiss_for_navigation();
                if let Some(replay) = &mut table.replay {
                    replay.back();
                    table.dirty = true;
                    orbit.set_following(true);
                    orbit.update_follow_target(Some(layout::focus(play::focus(&table.view()))));
                }
            }
            Action::Live => {
                consequences.clear();
                table.context.reset();
                table.epoch += 1;
                table.replay = None;
                let faces = table.story_faces(&table.view());
                table.reading = story::Reading::at(faces);
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
    feedback.reduced_motion = table.reduced_motion;
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
    _color: Color,
) -> Entity {
    let landscape = matches!(code, "06003" | "06005" | "60401");
    let transform = Transform::from_translation(at).with_rotation(if landscape {
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
    } else {
        Quat::IDENTITY
    });
    let entity = crate::tabletop_pieces::card(
        commands,
        meshes,
        materials,
        context.image(code),
        transform,
        title,
    );
    let title = title.to_owned();
    let code = code.to_owned();
    commands
        .entity(entity)
        .insert((
            ScenePiece,
            CardFeedback {
                pending_choice: selected,
                symbol_font: context.art.symbol_font().clone(),
                ..default()
            },
        ))
        .observe({
            let title = title.clone();
            let code = code.clone();
            move |event: On<PointerClick>,
                  targets: Query<&ContextTarget>,
                  mut queue: ResMut<Actions>| {
                if event.button == bevy::picking::pointer::PointerButton::Primary {
                    queue.0.push(if let Ok(target) = targets.get(entity) {
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
        });
    // Party names investigators in the area heading; ordinary card captions sit
    // below the face and move with it without exposing physical copy IDs.
    if !landscape {
        let offset = story::StoryCard::from_code(&code).map_or(Vec3::Z * 1.1, |card| {
            Vec3::new(
                match card.side() {
                    layout::ScenarioBookSide::Agenda => -2.25,
                    layout::ScenarioBookSide::Act => 2.25,
                },
                0.25,
                0.,
            )
        });
        let label = scene::label(commands, context, title, at + offset, false);
        commands
            .entity(label)
            .insert(motion::CardLabel(entity, offset));
    }
    entity
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
    images: Res<Assets<Image>>,
    mut previous_ui: Local<(Vec2, [bool; 3])>,
    mut consequences: ResMut<consequences::State>,
    old: Query<PriorRenderedCard<'_>, RenderedContent>,
    shortcut: Res<crate::tabletop_reveal::Shortcut>,
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
    let size = Vec2::new(window.width(), window.height());
    let ui_state = (
        size,
        ui::portraits(&art, &images).map(|portrait| portrait.is_some()),
    );
    let ui_changed = *previous_ui != ui_state;
    *previous_ui = ui_state;
    if !table.dirty && !health_changed && !ui_changed {
        return;
    }
    table.last_rendered_health = Some((health.allows_play(), health.status().to_owned()));
    table.dirty = false;
    if !table.context.is_open()
        && !shortcut.is_open()
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
    consequences.prepare(
        table.last_rendered.as_ref(),
        &view,
        table.replay.is_none(),
        table.reduced_motion,
    );
    table.last_rendered = Some(view.clone());
    let story = table.story_faces(&view);
    if view.revision == 0 {
        table.reading = story::Reading::at(story.clone());
    } else {
        table.reading.synchronize(&story);
    }
    let context = CardRender {
        art: &art,
        view: &view,
        epoch: table.epoch,
        story: &story,
        consequences: &consequences,
    };
    scene::render(&mut commands, &mut meshes, &mut materials, &context);
    ui::render(
        &mut commands,
        &table,
        &view,
        &art,
        &images,
        &orbit,
        &health,
        size,
        &context,
        shortcut.is_open(),
    );
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
    consequences: Res<consequences::State>,
) {
    if !health.allows_play()
        || consequences.active()
        || table.modal()
        || table.context.is_open()
        || table.presentation_paused
        || !table.is_presenting()
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
    shortcut: Res<crate::tabletop_reveal::Shortcut>,
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
                || shortcut.is_open()
                || table.context.captures_keyboard()
                || focus
                    .as_ref()
                    .and_then(|f| f.get())
                    .is_some_and(|entity| focusable.contains(entity)),
            wheel: blocked || table.context.is_open() || shortcut.is_open() || over_ui,
        },
        reduced_motion: table.reduced_motion,
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
        &ComputedNode,
        Has<layout::TestAreaHeading>,
        &mut Node,
        &mut Visibility,
    )>,
    cards: Query<&Transform>,
) {
    for (label, card, computed, test_heading, mut node, mut visibility) in &mut labels {
        if orbit.pose().distance > layout::DETAIL_LABEL_DISTANCE && !label.1 {
            *visibility = Visibility::Hidden;
            continue;
        }
        let anchor = card
            .and_then(|card| {
                cards
                    .get(card.0)
                    .ok()
                    .map(|transform| transform.translation + card.1)
            })
            .unwrap_or(label.0);
        let point = if test_heading {
            layout::test_heading_screen_position(camera.0, camera.1, computed)
        } else {
            camera.0.world_to_viewport(camera.1, anchor).ok()
        };
        if let Some(point) = point {
            let width = match node.width {
                Val::Px(width) => width,
                _ => 190.,
            };
            node.left = px(point.x - width / 2.);
            node.top = px(point.y);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn contextual_feedback(table: Res<Table>, mut cards: Query<(&ContextTarget, &mut CardFeedback)>) {
    let view = table.view();
    let test = view.play.as_ref().and_then(|play| play.test.as_ref());
    for (target, mut feedback) in &mut cards {
        feedback.menu_source = table.context.is_menu_source(&target.source);
        feedback.commitment = None;
        if table.replay.is_none()
            && !table.modal()
            && !table.is_presenting()
            && let controls::Source::Card(id) = target.source
            && let Some(test) = test
            && let Some(contribution) = test
                .commitment
                .as_ref()
                .and_then(|cards| cards.iter().find(|card| card.card_id == id))
            && let Ok(bonus) = u8::try_from(contribution.contribution)
        {
            feedback.pending_choice = contribution.selected;
            feedback.commitment = Some(feedback::Contribution {
                bonus,
                skill: match test.skill {
                    crate::gathering::Skill::Willpower => {
                        crate::tabletop::symbols::Skill::Willpower
                    }
                    crate::gathering::Skill::Intellect => {
                        crate::tabletop::symbols::Skill::Intellect
                    }
                    crate::gathering::Skill::Combat => crate::tabletop::symbols::Skill::Combat,
                    crate::gathering::Skill::Agility => crate::tabletop::symbols::Skill::Agility,
                },
            });
        }
    }
}

fn request_redraw(
    orbit: Res<Orbit>,
    health: Res<RenderHealth>,
    table: Res<Table>,
    moving: Query<(), With<motion::CardMotion>>,
    consequences: Res<consequences::State>,
    mut redraw: ResMut<Redraw>,
) {
    if (!moving.is_empty()
        && !table.modal()
        && !(table.is_presenting() && table.presentation_paused))
        || orbit.is_moving()
        || health.is_checking()
        || (consequences.active()
            && !table.modal()
            && !(table.is_presenting() && table.presentation_paused))
        || (table.is_presenting() && !table.presentation_paused && !table.modal())
    {
        redraw.request();
    }
}

#[cfg(test)]
mod tests;
