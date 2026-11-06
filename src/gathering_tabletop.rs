//! Native consumer of the Gathering decision seam. Rules remain in gathering::Session.

use crate::{
    asset_config::AssetConfig,
    gathering::{Investigator, Observation, PresentationMoment, Session, continuity},
    tabletop::asset_settings::AssetSettings,
    tabletop_camera::{CameraMode, Capture, FrameInput, Orbit, parse_initial_view},
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
mod capture;
mod card_counters;
mod consequences;
mod controls;
mod draws;
mod enemy_board;
mod motion;
mod play;
mod presentation;
mod reading;
mod results;
mod reveal;
mod scene;
mod sealed;
mod shuffle;
mod slots;
mod story;
mod trays;
mod ui;

const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.65, 0.74, 0.73);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);

#[derive(Clone)]
enum Action {
    ContinueStory,
    ContinueResult(u64),
    ResultChoice(u64, u64, String),
    Reveal(crate::tabletop_reveal::Request),
    Choose(u64, u64, String),
    Select(u64, u64, controls::Selection),
    ContextChoice(u64, u64, u64, String),
    ContextInspect(u64, u64, u64, String, String),
    DismissContext,
    PendingChoices,
    TokenOptions(String),
    Utilities,
    Dismiss,
    Motion,
    AssetFolder,
    Inspect(String, String),
    Save,
    Reopen,
    Restart(u64),
    ConfirmRestart(u64),
    History,
    Close,
    Focus(Area),
    Resolve,
    Follow,
}

#[derive(Resource, Default)]
struct Actions(Vec<Action>);

#[derive(Resource)]
struct Table {
    sealed: sealed::State,
    session: Session,
    save_path: PathBuf,
    notice: String,
    inspection: Option<(String, String)>,
    history_open: bool,
    menu_open: bool,
    restart_confirmation: bool,
    reduced_motion: bool,
    presentation: presentation::Presentation,
    epoch: u64,
    dirty: bool,
    last_rendered: Option<Observation>,
    last_rendered_health: Option<(bool, String)>,
    motion_origins: motion::Origins,
    trays: trays::State,
    context: controls::Context,
    token_options: Option<String>,
}

impl Table {
    fn open(save_path: PathBuf) -> Result<Self, String> {
        if save_path.exists() {
            return Self::open_existing(save_path);
        }
        let session = Session::new().map_err(|error| error.to_string())?;
        continuity::save_session(&save_path, &session).map_err(|error| error.to_string())?;
        Ok(Self::new(session, save_path))
    }

    fn open_existing(save_path: PathBuf) -> Result<Self, String> {
        let session = continuity::load_session(&save_path).map_err(|error| error.to_string())?;
        if session.is_verification_fixture() {
            return Err(
                "This is a privileged verification fixture, not an ordinary Gathering setup."
                    .into(),
            );
        }
        Ok(Self::new(session, save_path))
    }

    fn new(session: Session, save_path: PathBuf) -> Self {
        let presentation = presentation::Presentation::new(&session);
        let mut trays = trays::State::default();
        trays.prepare(&session.observe(), true);
        Self {
            sealed: sealed::State::default(),
            session,
            save_path,
            notice: String::new(),
            inspection: None,
            history_open: false,
            menu_open: false,
            restart_confirmation: false,
            reduced_motion: false,
            presentation,
            epoch: 0,
            dirty: true,
            last_rendered: None,
            last_rendered_health: None,
            motion_origins: motion::Origins::default(),
            trays,
            context: controls::Context::default(),
            token_options: None,
        }
    }

    fn view(&self) -> Observation {
        self.presentation.view(&self.session)
    }

    fn is_presenting(&self) -> bool {
        self.presentation.active() || self.sealed.active()
    }

    fn modal(&self) -> bool {
        self.presentation.result().is_some()
            || self.result_opportunities()
            || self.presentation.story().is_some()
            || self.inspection.is_some()
            || self.history_open
            || self.restart_confirmation
    }

    fn result_opportunities(&self) -> bool {
        !self.is_presenting() && controls::result_opportunities(&self.view()).is_some()
    }

    fn presentation_blocked(&self) -> bool {
        self.modal() || self.menu_open || self.context.is_open()
    }

    fn motion_blocked(&self) -> bool {
        self.presentation.result().is_some()
            || self.presentation.story().is_some()
            || self.inspection.is_some()
            || self.history_open
            || self.restart_confirmation
            || self.menu_open
            || self.context.is_open()
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
            if code == "victory" {
                return play
                    .scenario
                    .victory_display()
                    .last()
                    .and_then(|card| self.art.card(card.code, false));
            }
            let public_top = match code {
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
    screenshot: Option<PathBuf>,
    quit_after_screenshot: bool,
    initial_view: Option<InitialView>,
}

#[derive(Clone, Copy)]
enum InitialView {
    Area(Area),
    Resolve,
}

impl Options {
    fn parse(
        config: AssetConfig,
        mut args: impl Iterator<Item = String>,
    ) -> Result<Option<Self>, String> {
        let mut save = PathBuf::from("local/gathering/setup.session.json");
        let mut root = None;
        let mut no_art = false;
        let mut screenshot = None;
        let mut quit_after_screenshot = false;
        let mut initial_view = None;
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
                "--screenshot" => {
                    screenshot = Some(args.next().ok_or("--screenshot needs a path")?.into())
                }
                "--quit-after-screenshot" => quit_after_screenshot = true,
                "--initial-view" => {
                    let value = args.next().ok_or("--initial-view needs a view")?;
                    initial_view = Some(if value == "resolve" {
                        InitialView::Resolve
                    } else {
                        InitialView::Area(parse_initial_view(&value).map_err(|error| {
                            format!("{error}, or resolve (active resolution drawer)")
                        })?)
                    });
                }
                "--help" | "-h" => {
                    println!(
                        "The Gathering\n--experiment gathering\n--save PATH\n--art-root PREPARED_FOLDER\n--no-art\n--screenshot PATH\n--quit-after-screenshot\n--initial-view overview|scenario|test|tony|patrice|jacqueline|resolve\nReopens an existing session; a new path starts setup and scenario play. Artwork uses the saved folder unless overridden. Initial view changes diagnostic startup framing; Home is unchanged. The resolve view requires an active resolution drawer."
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown Gathering option: {arg}")),
            }
        }
        if quit_after_screenshot && screenshot.is_none() {
            return Err("--quit-after-screenshot requires --screenshot PATH".into());
        }
        Ok(Some(Self {
            save,
            art_root: (!no_art).then(|| config.launch_root(root)).transpose()?,
            config,
            screenshot,
            quit_after_screenshot,
            initial_view,
        }))
    }

    fn open_table(&self) -> Result<Table, String> {
        if matches!(self.initial_view, Some(InitialView::Resolve)) {
            Table::open_existing(self.save.clone()).map_err(|error| {
                format!("Cannot open the saved resolution for --initial-view resolve: {error}")
            })
        } else {
            Table::open(self.save.clone())
        }
    }

    fn configure_diagnostics(&self, app: &mut App) -> Result<(), String> {
        let mut orbit =
            Orbit::new(CameraMode::Jacqueline).with_home(enemy_board::focus(Area::Overview));
        match self.initial_view {
            Some(InitialView::Area(area)) => orbit.focus(enemy_board::focus(area)),
            Some(InitialView::Resolve) => {
                let focus = app
                    .world()
                    .get_resource::<Table>()
                    .map(Table::view)
                    .and_then(|view| {
                        view.play
                            .as_ref()
                            .and_then(trays::Layout::for_play)
                            .map(|tray| tray.focus())
                    })
                    .ok_or("No active resolution drawer for --initial-view resolve.")?;
                orbit.focus(focus);
            }
            None => {}
        }
        app.insert_resource(orbit)
            .insert_resource(capture::State::new(
                self.screenshot.clone(),
                self.quit_after_screenshot,
            ));
        Ok(())
    }
}

pub fn run() -> Result<(), String> {
    let Some(options) = Options::parse(AssetConfig::load_gathering()?, std::env::args().skip(1))?
    else {
        return Ok(());
    };
    let mut app = App::new();
    let mut table = options.open_table()?;
    let prepared = art::Prepared::new(options.art_root.clone())?;
    eprintln!(
        "Experiment: The Gathering (--experiment gathering)\nArtwork root: {}\n{}",
        prepared
            .root()
            .map_or_else(|| "disabled".into(), |path| path.display().to_string()),
        prepared.status()
    );
    if let Some(warning) = options.config.warning() {
        table.notice = format!("Asset settings: {warning}");
    }
    app.insert_resource(table);
    options.configure_diagnostics(&mut app)?;
    app.insert_resource(ClearColor(INK))
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
    capture::install(app);
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
                advance_framework,
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
                enemy_board::highlight,
                motion::animate,
                sealed::animate,
                shuffle::animate,
                card_counters::follow,
                trays::animate,
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

    table.context.sync(epoch, revision);
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
        let reading = table.presentation.story().is_some();
        let result_choice = matches!(action, Action::ResultChoice(..))
            && table.presentation.result().is_none()
            && table.result_opportunities();
        let permitted_in_modal = if table.presentation.result().is_some() {
            matches!(action, Action::ContinueResult(_))
        } else if reading {
            matches!(action, Action::ContinueStory)
        } else if table.result_opportunities() {
            result_choice
        } else if table.restart_confirmation {
            matches!(action, Action::Close | Action::ConfirmRestart(_))
        } else {
            matches!(action, Action::Close)
        };
        if table.modal() && !permitted_in_modal {
            continue;
        }
        if consequences.active() && matches!(action, Action::Choose(..) | Action::ResultChoice(..))
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
            Action::ContinueResult(epoch) if epoch == table.epoch => {
                table.presentation.acknowledge_result();
                table.epoch += 1;
                table.dirty = true;
            }
            Action::ContinueResult(_) => {}
            Action::ContinueStory => {
                table.presentation.acknowledge_story();
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
                if table.reduced_motion {
                    table.presentation.reduce_motion();
                }
                table.dirty = true;
            }
            Action::Choose(epoch, revision, id) | Action::ResultChoice(epoch, revision, id)
                if epoch == table.epoch
                    && (!table.modal() || result_choice)
                    && !table.is_presenting() =>
            {
                table.token_options = None;
                let path = table.save_path.clone();
                let before = table.view();
                table.notice = match continuity::commit_choice_with_draws(
                    &path,
                    &mut table.session,
                    revision,
                    &id,
                ) {
                    Ok(draws) => {
                        let table = &mut *table;
                        let shuffled = table.reduced_motion
                            && draws.iter().any(|cue| {
                                cue.batch.steps.iter().any(|step| {
                                    matches!(step, crate::gathering::DeckStep::Shuffle { .. })
                                })
                            });
                        table.presentation.committed(
                            &table.session,
                            &before,
                            &id,
                            draws,
                            table.reduced_motion,
                        );
                        table.context.reset();
                        table.epoch += 1;
                        orbit.update_follow_target(Some(play::framing(&table.view())));
                        if reveal_focus {
                            orbit.focus(play::framing(&table.view()));
                        }
                        if shuffled {
                            "Decks shuffled. Reduced motion is on.".into()
                        } else {
                            String::new()
                        }
                    }
                    Err(error) => format!("Decision not committed: {error}"),
                };
                table.dirty = true;
            }
            Action::Choose(_, _, _) | Action::ResultChoice(_, _, _) => {}
            Action::Select(epoch, revision, selection)
                if epoch == table.epoch && revision == table.view().revision && !table.modal() =>
            {
                let view = table.view();
                let live = !table.is_presenting();
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
            Action::PendingChoices if !table.modal() && !table.is_presenting() => {
                table.context.open_pending();
            }
            Action::PendingChoices => {}
            Action::TokenOptions(source) => {
                table.token_options =
                    (table.token_options.as_ref() != Some(&source)).then_some(source);
                table.dirty = true;
            }
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
                orbit.focus(enemy_board::focus(area));
                table.dirty = true;
            }
            Action::Resolve => {
                let view = table.view();
                if let Some(tray) = view.play.as_ref().and_then(trays::Layout::for_play) {
                    table.context.dismiss();
                    orbit.focus(tray.focus());
                    table.dirty = true;
                }
            }
            Action::Follow => {
                table.context.dismiss();
                let following = !orbit.is_following();
                orbit.set_following(following);
                orbit.update_follow_target(Some(play::framing(&table.view())));
                table.dirty = true;
            }
            Action::Close => {
                table.context.reset();
                table.epoch += 1;
                table.inspection = None;
                table.history_open = false;
                table.menu_open = false;
                table.restart_confirmation = false;
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
            Action::Save => {
                table.notice = match continuity::save_session(&table.save_path, &table.session) {
                    Ok(()) => "Saved the live decision.".into(),
                    Err(error) => format!("Save failed: {error}"),
                };
                table.dirty = true;
            }
            Action::Reopen => {
                match continuity::load_session(&table.save_path) {
                    Ok(session) if !session.is_verification_fixture() => {
                        consequences.clear();
                        table.context.reset();
                        table.epoch += 1;
                        let table = &mut *table;
                        table.presentation.reopen(&table.session, &session);
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
            Action::Restart(epoch) if epoch == table.epoch && table.menu_open => {
                table.context.reset();
                table.epoch += 1;
                table.menu_open = false;
                table.restart_confirmation = true;
                table.notice.clear();
                table.dirty = true;
            }
            Action::ConfirmRestart(epoch) if epoch == table.epoch && table.restart_confirmation => {
                let result =
                    Session::new()
                        .map_err(|error| error.to_string())
                        .and_then(|session| {
                            continuity::restart_session(&table.save_path, &table.session, &session)
                                .map(|archive| (session, archive))
                                .map_err(|error| error.to_string())
                        });
                match result {
                    Ok((session, archive)) => {
                        let mut next = Table::new(session, table.save_path.clone());
                        next.epoch = table.epoch.wrapping_add(1);
                        next.reduced_motion = table.reduced_motion;
                        next.notice = format!(
                            "Started a new Gathering attempt. Previous attempt archived at {}",
                            archive.display()
                        );
                        eprintln!("{}", next.notice);
                        *table = next;
                        consequences.clear();
                        orbit.set_following(false);
                        orbit.focus(enemy_board::focus(Area::Overview));
                    }
                    Err(error) => {
                        table.notice = format!("Restart failed: {error}");
                        table.dirty = true;
                    }
                }
            }
            Action::Restart(_) | Action::ConfirmRestart(_) => {}
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

/// Existing caption roles keep their distinct literal and rich-text treatment.
enum Caption<'a> {
    Text(&'a str),
    AssetStatus { title: &'a str, status: &'a str },
}

/// The requested Gathering representation. Motion, contextual selection and
/// inspection identities are independent: a preview can select a physical card
/// without being that card, and a mulligan card can move without offering actions.
struct CardSpec<'a> {
    title: &'a str,
    inspect_code: &'a str,
    pose: Transform,
    caption: Option<Caption<'a>>,
    source: Option<controls::Source>,
    physical_id: Option<u16>,
    selected: bool,
}

impl<'a> CardSpec<'a> {
    fn new(at: Vec3, title: &'a str, inspect_code: &'a str) -> Self {
        let landscape = matches!(inspect_code, "06003" | "06005" | "60401");
        Self {
            title,
            inspect_code,
            pose: Transform::from_translation(at).with_rotation(if landscape {
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
            } else {
                Quat::IDENTITY
            }),
            // Investigators are named by their area heading.
            caption: (!landscape).then_some(Caption::Text(title)),
            source: None,
            physical_id: None,
            selected: false,
        }
    }
}

fn card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    spec: CardSpec<'_>,
) -> Entity {
    let at = spec.pose.translation;
    let entity = crate::tabletop_pieces::card(
        commands,
        meshes,
        materials,
        context.image(spec.inspect_code),
        spec.pose,
        spec.title,
    );
    let title = spec.title.to_owned();
    let code = spec.inspect_code.to_owned();
    commands
        .entity(entity)
        .insert((
            ScenePiece,
            CardFeedback {
                pending_choice: spec.selected,
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
    if let Some(source) = spec.source {
        if controls::window_opportunities(context.view)
            .iter()
            .any(|(available, _)| available == &source)
        {
            let label = scene::label(
                commands,
                context,
                "Available now",
                at + Vec3::new(0., 0.15, -1.45),
                false,
            );
            commands
                .entity(label)
                .insert(motion::CardLabel(entity, Vec3::new(0., 0.15, -1.45)));
        }
        context.bind(commands, entity, source);
    }
    if let Some(id) = spec.physical_id {
        commands.entity(entity).insert(motion::PhysicalCard(id));
    }
    // Party names investigators in the area heading; ordinary card captions sit
    // below the face and move with it without exposing physical copy IDs.
    if let Some(caption) = spec.caption {
        let book_side = story::StoryCard::from_code(&code).map(|card| card.side());
        let offset = book_side.map_or(Vec3::Z * 1.1, |side| {
            let edge = crate::tabletop_pieces::CARD_LENGTH / 2. + 0.4;
            Vec3::new(
                match side {
                    layout::ScenarioBookSide::Agenda => -edge,
                    layout::ScenarioBookSide::Act => edge,
                },
                0.25,
                0.,
            )
        });
        let label_text = match caption {
            Caption::Text(text) => text,
            Caption::AssetStatus { title, .. } => title,
        };
        let label = scene::label(commands, context, label_text, at + offset, false);
        // Keep literal status separate from rich-text card names.
        match caption {
            Caption::Text(_) => {}
            Caption::AssetStatus { status, .. } => {
                let status = format!("\n{status}");
                commands.queue(move |world: &mut World| {
                    world.get_mut::<Text>(label).unwrap().0.push_str(&status);
                });
            }
        }
        if let Some(side) = book_side {
            commands.entity(label).insert((
                side,
                TextLayout::justify(match side {
                    layout::ScenarioBookSide::Agenda => Justify::Right,
                    layout::ScenarioBookSide::Act => Justify::Left,
                }),
            ));
        } else {
            commands
                .entity(label)
                .insert(crate::tabletop_pieces::CardCaption);
        }
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
    let scenario = view.scenario_for_display();
    if code == "chaos-bag" {
        return Some(format!(
            "Standard chaos bag · {} tokens\nCurse pool · 10 tokens\n\n{}",
            view.standard_bag.len(),
            crate::gathering::inspection::scenario_card("01104", false)?
        ));
    }
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
    mut art: ResMut<art::Art>,
    mut images: ResMut<Assets<Image>>,
    mut slot_surfaces: Local<slots::Surfaces>,
    ui_scale: Res<UiScale>,
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
    let ui_changed = *previous_ui != ui_state || art.refresh_loaded(&images);
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
    let presentation::Frame {
        view,
        story,
        transfers,
    } = {
        let table = &mut *table;
        table.presentation.prepare(&table.session)
    };
    for transfer in transfers {
        table.motion_origins.transfer(transfer);
    }
    consequences.prepare(
        table.last_rendered.as_ref(),
        &view,
        true,
        table.reduced_motion,
    );
    let previous = table.last_rendered.clone();
    let reduced = table.reduced_motion;
    table.sealed.prepare(previous.as_ref(), &view, reduced);
    table.last_rendered = Some(view.clone());
    let reduced_motion = table.reduced_motion;
    table.trays.prepare(&view, reduced_motion);
    let context = CardRender {
        art: &art,
        view: &view,
        epoch: table.epoch,
        story: &story,
        consequences: &consequences,
    };
    scene::render(&mut commands, &mut meshes, &mut materials, &context);
    slot_surfaces.render(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &context,
        ui_scale.0,
    );
    table
        .trays
        .render(&mut commands, &mut meshes, &mut materials, &context);
    table
        .presentation
        .render_draws(&mut commands, &mut meshes, &mut materials, &context);
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
    moving: Query<(), With<motion::CardMotion>>,
) {
    if !health.allows_play()
        || consequences.active()
        || table.sealed.active()
        || table.presentation_blocked()
    {
        return;
    }
    let table = &mut *table;
    let progress = table.presentation.advance(
        &table.session,
        time.delta_secs(),
        table.last_rendered.as_ref(),
        !moving.is_empty(),
        table.reduced_motion,
    );
    if progress != presentation::Progress::Waiting {
        table.epoch += 1;
        table.dirty = true;
    }
    if progress == presentation::Progress::Moment {
        orbit.update_follow_target(Some(play::framing(&table.view())));
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
    mut shortcut: ResMut<crate::tabletop_reveal::Shortcut>,
) {
    let blocked = table.modal()
        || table.menu_open
        || settings.as_ref().is_some_and(|settings| settings.is_busy());
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
            pan: Some(blocked),
            wheel: blocked || table.context.is_open() || shortcut.is_open() || over_ui,
        },
        reduced_motion: table.reduced_motion,
    });
    **camera = step.transform;
    if step.manual_navigation && table.context.is_open() {
        table.context.dismiss();
    }
    if step.manual_navigation && shortcut.is_open() {
        shortcut.dismiss();
    }
    if step.stopped_following {
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
        Has<layout::ScenarioAreaHeading>,
        Has<crate::tabletop_pieces::CardCaption>,
        Has<enemy_board::Caption>,
        Has<enemy_board::PersistentLabel>,
        Option<&layout::ScenarioBookSide>,
        &mut Node,
        &mut Visibility,
    )>,
    cards: Query<&Transform>,
) {
    for (
        label,
        card,
        computed,
        test_heading,
        scenario_heading,
        card_caption,
        enemy_caption,
        persistent,
        book_side,
        mut node,
        mut visibility,
    ) in &mut labels
    {
        if orbit.pose().distance > layout::DETAIL_LABEL_DISTANCE && !label.1 && !persistent {
            *visibility = Visibility::Hidden;
            continue;
        }
        if card.is_some_and(|label| {
            cards
                .get(label.0)
                .is_ok_and(|transform| (transform.rotation * Vec3::Y).y < 0.)
        }) {
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
        } else if scenario_heading {
            layout::scenario_heading_screen_position(camera.0, camera.1, computed)
        } else {
            camera.0.world_to_viewport(camera.1, anchor).ok()
        };
        if let Some(point) = point {
            if enemy_caption {
                if let Some(width) = enemy_board::caption_width(camera.0, camera.1, anchor) {
                    node.width = px(width);
                }
            } else if card_caption
                && let Some(width) =
                    crate::tabletop_pieces::CardCaption::width(camera.0, camera.1, anchor)
            {
                node.width = px(width);
            }
            let width = match node.width {
                Val::Px(width) => width,
                _ => 190.,
            };
            node.left = px(match book_side {
                Some(layout::ScenarioBookSide::Agenda) => point.x - width,
                Some(layout::ScenarioBookSide::Act) => point.x,
                None => point.x - width / 2.,
            });
            node.top = px(point.y);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn contextual_feedback(
    table: Res<Table>,
    mut cards: Query<(&ContextTarget, &mut CardFeedback)>,
    buttons: Query<(Entity, &bevy::picking::hover::Hovered, &AccessibleLabel), With<Button>>,
    focus: Option<Res<InputFocus>>,
) {
    let view = table.view();
    let opportunities = controls::window_opportunities(&view);
    let test = view.play.as_ref().and_then(|play| play.test.as_ref());
    let preview: Vec<_> = view
        .choices
        .iter()
        .filter(|choice| {
            buttons.iter().any(|(entity, hovered, label)| {
                (hovered.0
                    || focus
                        .as_ref()
                        .is_some_and(|focus| focus.get() == Some(entity)))
                    && (label.0 == choice.label || label.0 == feedback::menu::label(&choice.label))
            })
        })
        .flat_map(|choice| controls::replacement_members(&choice.id))
        .collect();
    for (target, mut feedback) in &mut cards {
        feedback.menu_source = table.context.is_menu_source(&target.source)
            || (!table.modal()
                && !table.is_presenting()
                && matches!(target.source, controls::Source::Card(id) if preview.contains(&id)));
        feedback.opportunity = !table.modal()
            && !table.is_presenting()
            && opportunities
                .iter()
                .any(|(source, _)| source == &target.source);
        feedback.commitment = None;
        if !table.modal()
            && !table.is_presenting()
            && let controls::Source::Card(id) = target.source
            && let Some(test) = test
            && let Some(contribution) = test.commitment.iter().find(|card| card.card_id == id)
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

fn advance_framework(
    table: Res<Table>,
    health: Res<RenderHealth>,
    settings: Option<Res<AssetSettings>>,
    consequences: Res<consequences::State>,
    mut actions: ResMut<Actions>,
    mut redraw: ResMut<Redraw>,
) {
    if table.last_rendered.is_none()
        || !health.allows_play()
        || health.just_became_ready()
        || table.presentation_blocked()
        || table.is_presenting()
        || table.sealed.active()
        || consequences.active()
        || !actions.0.is_empty()
        || table.notice.starts_with("Decision not committed:")
        || settings.as_ref().is_some_and(|settings| settings.is_busy())
    {
        return;
    }
    let view = table.view();
    if let Some(id) = controls::automatic_choice(&view) {
        actions
            .0
            .push(Action::Choose(table.epoch, view.revision, id.into()));
        redraw.request();
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
    if (!moving.is_empty() && !table.motion_blocked())
        || (table.trays.active() && !table.motion_blocked())
        || (table.sealed.active() && !table.motion_blocked())
        || orbit.is_moving()
        || health.is_checking()
        || (consequences.active() && !table.motion_blocked())
        || (table.is_presenting() && !table.presentation_blocked())
    {
        redraw.request();
    }
}

#[cfg(test)]
mod tests;
