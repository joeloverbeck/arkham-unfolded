//! Native consumer of the Gathering decision seam. Rules remain in gathering::Session.

use crate::{
    asset_config::AssetConfig,
    gathering::{Investigator, Observation, PresentationMoment, Session, continuity},
    tabletop_camera::{Capture, FrameInput, Orbit, parse_initial_view},
    tabletop_feedback::{self as feedback, CardFeedback, FeedbackSettings},
    tabletop_health::{RenderHealth, TabletopHealthPlugin},
    tabletop_layout::{self as layout, Area},
    tabletop_pacing::{PresentationTime, Redraw, RequestRedraw, TabletopPacingPlugin},
    tabletop_pieces::WorldLabel,
    tabletop_support::asset_settings::AssetSettings,
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
mod doom;
mod draws;
mod encounter;
mod enemy_board;
mod map_motion;
mod markers;
mod motion;
mod pendulum;
mod phase_banner;
mod play;
mod presentation;
mod printed;
mod reading;
mod results;
mod reveal;
mod scene;
mod sealed;
mod seat;
mod shuffle;
mod slots;
mod story;
mod trays;
mod ui;
mod violin;

const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.65, 0.74, 0.73);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);

#[derive(Clone)]
enum Action {
    ContinueStory,
    ContinueResult(u64),
    Encounter(u64, encounter::Input),
    EncounterChoice(u64, u64, String),
    ResultChoice(u64, u64, String),
    Reveal(crate::tabletop_reveal::Request),
    Choose(u64, u64, String),
    Prediction(u64, u64, u16, pendulum::Input),
    Violin(u64, u64, violin::Input),
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
    CreditLink(&'static str),
    Save,
    ReturnToSelection,
    Reopen,
    Restart(u64),
    ConfirmRestart(u64),
    History,
    Close,
    Focus(Area),
    Resolve,
    Follow,
}

impl Action {
    /// The choice this control offers, when it offers one.
    fn choice(&self) -> Option<&str> {
        match self {
            Self::Choose(_, _, id)
            | Self::ResultChoice(_, _, id)
            | Self::EncounterChoice(_, _, id)
            | Self::ContextChoice(_, _, _, id)
            | Self::Prediction(_, _, _, pendulum::Input::Open(id)) => Some(id),
            _ => None,
        }
    }
}

/// Names a choice-bearing control by its choice identity, set where the control is
/// built. Activation still runs through the control's own handler; the agent seat
/// (#153) reads this to find the control a player would press.
#[derive(Component)]
struct ChoiceControl(String);

#[derive(Resource, Default)]
struct Actions(Vec<Action>);

#[derive(Resource)]
struct Table {
    sealed: sealed::State,
    session: Session,
    save_path: PathBuf,
    initial_save_pending: bool,
    leaving: bool,
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
    prediction: Option<pendulum::Picker>,
    violin: Option<violin::Picker>,
}

impl Table {
    fn open(save_path: PathBuf) -> Result<Self, String> {
        if save_path.exists() {
            return Self::open_existing(save_path);
        }
        Self::fresh(save_path)
    }

    fn fresh(save_path: PathBuf) -> Result<Self, String> {
        let session = Session::new().map_err(|error| error.to_string())?;
        let mut table = Self::new(session, save_path);
        table.initial_save_pending = true;
        Ok(table)
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
            initial_save_pending: false,
            leaving: false,
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
            prediction: None,
            violin: None,
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
            || self.presentation.encounter().current().is_some()
            || self.inspection.is_some()
            || self.history_open
            || self.restart_confirmation
            || self.prediction.is_some()
            || self.violin.is_some()
    }

    fn result_opportunities(&self) -> bool {
        !self.is_presenting() && controls::result_opportunities(&self.view()).is_some()
    }

    fn presentation_blocked(&self) -> bool {
        self.leaving || self.modal() || self.menu_open || self.context.is_open()
    }

    fn motion_blocked(&self) -> bool {
        self.presentation.announcement().is_some()
            || self.presentation.result().is_some()
            || self.presentation.story().is_some()
            || self.presentation.encounter().current().is_some()
            || self.inspection.is_some()
            || self.history_open
            || self.restart_confirmation
            || self.menu_open
            || self.context.is_open()
            || self.violin.is_some()
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
    art_root: PathBuf,
    config: AssetConfig,
    screenshot: Option<PathBuf>,
    quit_after_screenshot: bool,
    initial_view: Option<InitialView>,
    attempt: Option<String>,
    agent_seat: bool,
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
        let mut save = None;
        let mut root = None;
        let mut screenshot = None;
        let mut quit_after_screenshot = false;
        let mut initial_view = None;
        let mut attempt = None;
        let mut agent_seat = false;
        let mut direct = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--experiment" => {
                    if args.next().as_deref() != Some("gathering") {
                        return Err("Use --experiment gathering.".into());
                    }
                    direct = true;
                }
                "--save" => save = Some(PathBuf::from(args.next().ok_or("--save needs a path")?)),
                "--agent-seat" => agent_seat = true,
                "--attempt" => {
                    let intent = args.next().ok_or("--attempt needs new or open")?;
                    if !matches!(intent.as_str(), "new" | "open") {
                        return Err("--attempt needs new or open".into());
                    }
                    attempt = Some(intent);
                }
                "--art-root" => {
                    root = Some(PathBuf::from(args.next().ok_or("--art-root needs a path")?))
                }
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
                        "The Gathering\n--experiment gathering\n--save PATH\n--art-root PREPARED_FOLDER\n--screenshot PATH\n--quit-after-screenshot\n--initial-view overview|scenario|test|tony|patrice|jacqueline|resolve\n--agent-seat\nReopens an existing session; a new path starts setup and scenario play. Artwork uses the saved folder unless overridden. Initial view changes diagnostic startup framing; Home is unchanged. The resolve view requires an active resolution drawer. The agent seat plays the same table with no window, exchanging JSON lines on standard input and output; it requires --experiment gathering and --save PATH, and excludes --attempt and --screenshot."
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown Gathering option: {arg}")),
            }
        }
        if quit_after_screenshot && screenshot.is_none() {
            return Err("--quit-after-screenshot requires --screenshot PATH".into());
        }
        if agent_seat {
            if !direct {
                return Err(
                    "--agent-seat belongs to the direct launch: add --experiment gathering".into(),
                );
            }
            if save.is_none() {
                return Err("--agent-seat requires an explicit --save PATH".into());
            }
            if attempt.is_some() || screenshot.is_some() {
                return Err(
                    "--agent-seat cannot be combined with --attempt or --screenshot; use its capture request".into(),
                );
            }
        }
        Ok(Some(Self {
            save: save.unwrap_or_else(|| PathBuf::from("local/gathering/setup.session.json")),
            art_root: config.launch_root(root)?,
            config,
            screenshot,
            quit_after_screenshot,
            initial_view,
            attempt,
            agent_seat,
        }))
    }

    fn open_table(&self) -> Result<Table, String> {
        crate::attempts::verification_path(&self.save)?;
        if self.attempt.as_deref() == Some("open") {
            return Table::open_existing(self.save.clone());
        }
        if self.attempt.as_deref() == Some("new") {
            match std::fs::symlink_metadata(&self.save) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
                Ok(_) => return Err("The new attempt path is already occupied.".into()),
            }
            // Never reopen here: another process may occupy the path after the
            // preflight. Admission's exclusive create owns the final decision.
            return Table::fresh(self.save.clone());
        }
        if matches!(self.initial_view, Some(InitialView::Resolve)) {
            Table::open_existing(self.save.clone()).map_err(|error| {
                format!("Cannot open the saved resolution for --initial-view resolve: {error}")
            })
        } else {
            Table::open(self.save.clone())
        }
    }

    fn configure_diagnostics(&self, app: &mut App) -> Result<(), String> {
        let mut orbit = Orbit::new().with_home(enemy_board::focus(Area::Overview));
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
    if agent_seat_requested() {
        // Never the return to scenario selection below: it opens a window.
        return seat::run(parse_launch());
    }
    let selected = std::env::args().any(|arg| arg == "--attempt");
    let result = run_gathering();
    if selected && let Err(error) = &result {
        let args: Vec<_> = std::env::args().collect();
        let root = args
            .windows(2)
            .find(|p| p[0] == "--art-root")
            .map(|p| std::path::Path::new(&p[1]));
        return crate::scenario_selection::return_to_selection(root, Some(error));
    }
    result
}

#[derive(Resource)]
struct RememberAttempt {
    reference: PathBuf,
    pending: bool,
}

#[derive(Clone, Default, Resource)]
struct Navigation {
    returning: std::sync::Arc<std::sync::atomic::AtomicBool>,
    created: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Whether this launch asks for the agent seat (#153). Such a launch must never take a
/// path that opens a window, including when its options are refused.
pub fn agent_seat_requested() -> bool {
    std::env::args().any(|argument| argument == "--agent-seat")
}

fn parse_launch() -> Result<Option<Options>, String> {
    Options::parse(AssetConfig::load_gathering()?, std::env::args().skip(1))
}

fn run_gathering() -> Result<(), String> {
    let Some(options) = parse_launch()? else {
        return Ok(());
    };
    let (recovery, collection) = crate::tabletop_startup::Recovery::prepare(
        options.config.clone(),
        options.art_root.clone(),
        crate::tabletop_support::asset_settings::Collection::Gathering,
    )?;
    let artwork_use = crate::assets::package::acquire_play(&recovery.config(), recovery.root())?;
    recovery.run_window(artwork_use, |recovery| {
        run_window(&options, recovery, &collection)
    })
}

fn run_window(
    options: &Options,
    recovery: &crate::tabletop_startup::Recovery,
    collection: &crate::assets::CollectionReport,
) -> Result<crate::tabletop_startup::WindowExit, String> {
    let prepared = art::Prepared::new(recovery.root().to_owned())?;
    let mut app = App::new();
    let mut table = options.open_table()?;
    if let Some(warning) = options.config.warning() {
        table.notice = format!("Asset settings: {warning}");
    }
    app.insert_resource(table);
    if options.attempt.is_some() {
        app.insert_resource(RememberAttempt {
            reference: crate::attempts::reference_path()?,
            pending: true,
        });
    }
    options.configure_diagnostics(&mut app)?;
    app.insert_resource(ClearColor(INK))
        .insert_resource(AssetSettings::new_gathering(recovery.config()))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // tools/src/desktop.rs verifies this exact title before focus or capture.
                        title: "Arkham Unfolded — Gathering Setup".into(),
                        resolution: (1600, 1000).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(prepared.asset_plugin()),
        )
        .add_plugins(MeshPickingPlugin)
        .add_plugins(crate::tabletop_chaos_pouch::ChaosPouchPlugin)
        .add_plugins(TabletopPacingPlugin)
        .add_plugins(TabletopHealthPlugin);
    let art = prepared.load(app.world().resource::<AssetServer>());
    app.insert_resource(art);
    install(&mut app);
    recovery.install(&mut app, collection);
    let navigation = app.world().resource::<Navigation>().clone();
    let exit = app.run();
    if navigation
        .returning
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        let args: Vec<_> = std::env::args().collect();
        return Ok(crate::tabletop_startup::WindowExit::Navigate(
            return_selection_command(&args, recovery.root())?,
        ));
    }
    Ok(crate::tabletop_startup::WindowExit::Finished(
        exit,
        navigation
            .created
            .load(std::sync::atomic::Ordering::Relaxed),
    ))
}

/// Construct the actual outgoing process command before replacing the renderer.
/// Recovery changes an existing override; ordinary launches keep using preferences.
fn return_selection_command(
    args: &[String],
    recovered_root: &std::path::Path,
) -> Result<std::process::Command, String> {
    let root = args
        .windows(2)
        .find(|p| p[0] == "--art-root")
        .map(|_| recovered_root);
    crate::scenario_selection::return_command(root, None)
}

/// The same registration is used by the native app and headless input tests.
fn install(app: &mut App) {
    capture::install(app);
    controls::install(app);
    app.init_resource::<Actions>()
        .init_resource::<Navigation>()
        .init_resource::<consequences::State>()
        .init_resource::<art::Art>()
        .add_plugins(feedback::FeedbackPlugin)
        .add_plugins(crate::tabletop_counters::CounterPlugin)
        .add_plugins(crate::tabletop_reveal::RevealPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (
                    poll_settings,
                    complete_artwork_import,
                    save_initial_decision,
                    remember_attempt,
                )
                    .chain(),
                advance_framework,
                (pendulum::keyboard, handle_actions).chain(),
                (consequences::advance, doom::advance).chain(),
                reveal::sync,
                crate::tabletop_reveal::render,
                advance_presentation,
                keyboard_panel_scroll,
                camera_controls,
                (render, phase_banner::render, encounter::animate).chain(),
                (controls::render, violin::render).chain(),
                contextual_feedback,
                enemy_board::highlight,
                (motion::animate, map_motion::animate).chain(),
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

fn complete_artwork_import(
    health: Res<RenderHealth>,
    recovery: Option<Res<crate::tabletop_startup::Recovery>>,
    mut table: ResMut<Table>,
) {
    if health.just_became_ready()
        && let Some(recovery) = recovery
        && let Err(error) = recovery.complete_import()
    {
        eprintln!("{error}");
        table.notice = error;
        table.dirty = true;
    }
}

fn save_initial_decision(
    mut table: ResMut<Table>,
    mut health: ResMut<RenderHealth>,
    mut exit: MessageWriter<AppExit>,
    navigation: Res<Navigation>,
) {
    if !table.initial_save_pending || !health.allows_play() {
        return;
    }
    match crate::saved_state::create(&table.save_path, &table.session) {
        Ok(()) => {
            table.initial_save_pending = false;
            navigation
                .created
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Err(error) => {
            let reason = format!("Could not save the initial Gathering decision: {error}");
            eprintln!("{reason}");
            health.stop(reason);
            exit.write(AppExit::error());
        }
    }
}

fn remember_attempt(
    mut selected: Option<ResMut<RememberAttempt>>,
    mut table: ResMut<Table>,
    health: Res<RenderHealth>,
) {
    let Some(selected) = selected.as_mut() else {
        return;
    };
    if !selected.pending || table.initial_save_pending || !health.allows_play() {
        return;
    }
    selected.pending = false;
    if let Err(error) = crate::attempts::remember(&selected.reference, &table.save_path) {
        table.notice = format!("Attempt opened, but Continue could not be updated: {error}");
        table.dirty = true;
    }
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
    mut exit: MessageWriter<AppExit>,
    navigation: Res<Navigation>,
    seated: Option<Res<crate::agent_seat::SeatedAgent>>,
) {
    if table.leaving {
        actions.0.clear();
        return;
    }
    let revision = table
        .last_rendered
        .as_ref()
        .map_or_else(|| table.view().revision, |view| view.revision);
    let epoch = table.epoch;

    table.context.sync(epoch, revision);
    violin::sync(&mut table);
    if !health.allows_play()
        || health.just_became_ready()
        || settings.as_ref().is_some_and(|settings| settings.is_busy())
    {
        table.context.reset();
        if table.violin.take().is_some() {
            table.epoch += 1;
            table.dirty = true;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        actions.0.push(Action::Close);
    }
    actions
        .0
        .extend(shortcut.drain().into_iter().map(Action::Reveal));
    for action in actions.0.drain(..) {
        if table.presentation.announcement().is_some() {
            continue;
        }
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
        let encounter_choice = if let Action::EncounterChoice(epoch, revision, id) = &action {
            *epoch == table.epoch
                && table
                    .presentation
                    .encounter()
                    .accepts(&table.session, *revision, id)
        } else {
            false
        };
        if matches!(action, Action::EncounterChoice(..)) && !encounter_choice {
            continue;
        }
        let permitted_in_modal = if table.presentation.result().is_some() {
            matches!(action, Action::ContinueResult(_))
        } else if reading {
            matches!(action, Action::ContinueStory)
        } else if table.presentation.encounter().current().is_some() {
            matches!(action, Action::Encounter(..)) || encounter_choice
        } else if table.result_opportunities() {
            result_choice
        } else if table.violin.is_some() && table.inspection.is_none() {
            matches!(action, Action::Violin(..) | Action::Close)
        } else if table.prediction.is_some() {
            matches!(action, Action::Prediction(..) | Action::Close)
        } else if table.restart_confirmation {
            matches!(action, Action::Close | Action::ConfirmRestart(_))
        } else {
            matches!(action, Action::Close)
                || matches!(action, Action::CreditLink(_))
                    && table
                        .inspection
                        .as_ref()
                        .is_some_and(|(_, code)| code == "credits")
        };
        if table.modal() && !permitted_in_modal {
            continue;
        }
        if consequences.active()
            && matches!(
                action,
                Action::Choose(..) | Action::ResultChoice(..) | Action::EncounterChoice(..)
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
        let Some(action) = pendulum::dispatch(&mut table, action) else {
            continue;
        };
        let violin_confirmation = matches!(&action, Action::Violin(_, _, violin::Input::Confirm));
        let Some(action) = violin::dispatch(&mut table, action) else {
            continue;
        };
        match action {
            Action::Encounter(epoch, input)
                if epoch == table.epoch && table.presentation.encounter().current().is_some() =>
            {
                table.presentation.encounter_input(input);
                table.epoch += 1;
                table.dirty = true;
            }
            Action::Encounter(..) => {}
            Action::Prediction(..) => unreachable!("Local picker dispatch consumes its controls"),
            Action::Violin(..) => unreachable!("Local picker dispatch consumes its controls"),
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
            Action::Choose(epoch, revision, id)
            | Action::ResultChoice(epoch, revision, id)
            | Action::EncounterChoice(epoch, revision, id)
                if epoch == table.epoch
                    && (!table.modal()
                        || result_choice
                        || encounter_choice
                        || violin_confirmation)
                    && (!table.is_presenting() || encounter_choice) =>
            {
                let id = if id == "test-card:pass"
                    && controls::automatic_choice(&table.session.observe()) == Some(id.as_str())
                {
                    "test-card:continue".into()
                } else {
                    id
                };
                if let Some(asset) = id
                    .strip_prefix("violin:open:")
                    .and_then(|id| id.parse().ok())
                {
                    violin::open(&mut table, epoch, revision, asset);
                    continue;
                }
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
                        table.violin = None;
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
            Action::Choose(..) | Action::ResultChoice(..) | Action::EncounterChoice(..) => {}
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
                if table.inspection.is_none() {
                    table.violin = None;
                }
                table.prediction = None;
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
            Action::CreditLink(url) if seated.is_some() => {
                // An agent at the seat never reaches the owner's desktop.
                table.notice = format!("The agent seat does not open links. Address: {url}");
                table.dirty = true;
            }
            Action::CreditLink(url) => {
                // Only the static credit links constructed by the Credits view
                // reach this action. Reap the opener without blocking rendering.
                std::thread::spawn(move || {
                    if let Err(error) = std::process::Command::new("xdg-open").arg(url).status() {
                        eprintln!("Could not open credit link {url}: {error}");
                    }
                });
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
            Action::ReturnToSelection => {
                match continuity::save_session(&table.save_path, &table.session) {
                    Ok(()) => {
                        table.leaving = true;
                        navigation
                            .returning
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        table.context.reset();
                        shortcut.dismiss();
                        exit.write(AppExit::Success);
                        break;
                    }
                    Err(error) => {
                        table.notice = format!("Could not return to selection: {error}");
                        table.dirty = true;
                    }
                }
            }
            Action::Reopen => {
                match continuity::load_session(&table.save_path) {
                    Ok(session) if !session.is_verification_fixture() => {
                        consequences.clear();
                        table.context.reset();
                        table.epoch += 1;
                        let table = &mut *table;
                        table.presentation.reopen(&table.session, &session);
                        table.prediction = None;
                        table.violin = None;
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
    feedback.blocked = table.modal()
        || table.presentation.announcement().is_some()
        || settings.as_ref().is_some_and(|settings| settings.is_busy());
    feedback.reduced_motion = table.reduced_motion;
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    layout::spawn_gathering_surfaces(&mut commands, &mut meshes, &mut materials);
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
    art: Res<art::Art>,
    mut images: ResMut<Assets<Image>>,
    mut surfaces: Local<(slots::Surfaces, printed::Surfaces)>,
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
    let ui_changed = *previous_ui != ui_state;
    *previous_ui = ui_state;
    if !table.dirty && !health_changed && !ui_changed {
        return;
    }
    table.last_rendered_health = Some((health.allows_play(), health.status().to_owned()));
    table.dirty = false;
    if !table.context.is_open()
        && table.violin.is_none()
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
    consequences.prepare(
        table.last_rendered.as_ref(),
        &table.view(),
        true,
        table.reduced_motion,
    );
    let presentation::Frame {
        view,
        story,
        transfers,
        map_arrivals,
    } = {
        let table = &mut *table;
        table
            .presentation
            .prepare(&table.session, consequences.doom.active())
    };
    for transfer in transfers {
        table.motion_origins.transfer(transfer);
    }
    for (id, at) in map_arrivals {
        table.motion_origins.arrived(id, at);
    }
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
    surfaces.0.render(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &context,
        ui_scale.0,
    );
    surfaces.1.render(
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
    if !table.presentation.drawing() {
        table
            .presentation
            .map()
            .render(&mut commands, &mut meshes, &mut materials, &context);
    }
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
        || (consequences.active() && table.presentation.announcement().is_none())
        || (table.sealed.active() && table.presentation.announcement().is_none())
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
        || table.presentation.announcement().is_some()
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
        Has<crate::tabletop_pieces::CardCaption>,
        Has<enemy_board::Caption>,
        Has<enemy_board::PersistentLabel>,
        Option<&layout::ScenarioBookSide>,
        Has<markers::HeadingLabel>,
        &mut Node,
        &mut Visibility,
    )>,
    cards: Query<&Transform>,
) {
    for (
        label,
        card,
        computed,
        card_caption,
        enemy_caption,
        persistent,
        book_side,
        heading,
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
        let point = camera.0.world_to_viewport(camera.1, anchor).ok();
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
                _ => computed.size().x * computed.inverse_scale_factor,
            };
            node.left = px(match book_side {
                Some(layout::ScenarioBookSide::Agenda) => point.x - width,
                Some(layout::ScenarioBookSide::Act) => point.x,
                None if heading => point.x,
                None => point.x - width / 2.,
            });
            node.top = px(if heading {
                point.y - computed.size().y * computed.inverse_scale_factor / 2.
            } else {
                point.y
            });
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn contextual_feedback(
    table: Res<Table>,
    health: Res<RenderHealth>,
    settings: Option<Res<AssetSettings>>,
    consequences: Res<consequences::State>,
    mut cards: Query<(&ContextTarget, &mut CardFeedback)>,
    buttons: Query<(Entity, &bevy::picking::hover::Hovered, &AccessibleLabel), With<Button>>,
    focus: Option<Res<InputFocus>>,
) {
    let view = table.view();
    let choices_enabled = !table.leaving
        && !table.modal()
        && !table.menu_open
        && !table.is_presenting()
        && !consequences.active()
        && health.allows_play()
        && !health.just_became_ready()
        && !settings.as_ref().is_some_and(|settings| settings.is_busy());
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
        feedback.opportunity = choices_enabled
            && target.epoch == table.epoch
            && target.revision == view.revision
            && !controls::source_choices(&view, &target.source).is_empty();
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
                        crate::tabletop_support::symbols::Skill::Willpower
                    }
                    crate::gathering::Skill::Intellect => {
                        crate::tabletop_support::symbols::Skill::Intellect
                    }
                    crate::gathering::Skill::Combat => {
                        crate::tabletop_support::symbols::Skill::Combat
                    }
                    crate::gathering::Skill::Agility => {
                        crate::tabletop_support::symbols::Skill::Agility
                    }
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

/// What must finish before the table takes a decision; empty when it is settled.
/// `Redraw` cannot answer this: the availability cue requests redraws for as long as
/// a choice is offered. A dialog or menu waiting for the player is settled, even when
/// it holds a presentation or motion in place behind it.
fn unsettled(world: &mut World) -> Vec<&'static str> {
    let moving = world
        .query_filtered::<(), With<motion::CardMotion>>()
        .iter(world)
        .next()
        .is_some();
    let table = world.resource::<Table>();
    let health = world.resource::<RenderHealth>();
    let consequences = world.resource::<consequences::State>();
    let held = table.motion_blocked();
    let mut pending = Vec::new();
    if !health.allows_play() || health.just_became_ready() {
        pending.push("graphics admission");
    }
    if table.initial_save_pending {
        pending.push("initial save");
    }
    if table.leaving {
        pending.push("leaving the table");
    }
    if table.dirty || table.last_rendered.is_none() {
        pending.push("scene rebuild");
    }
    if !world.resource::<Actions>().0.is_empty() {
        pending.push("queued input");
    }
    if table.presentation.announcement().is_some() {
        // Every action is dropped while a phase announcement shows, even behind a menu.
        pending.push("announcement");
    }
    if table.presentation.active() && !table.presentation_blocked() {
        pending.push("presentation");
    }
    if consequences.active() && !held {
        pending.push("consequences");
    }
    if table.trays.active() && !held {
        pending.push("trays");
    }
    if table.sealed.active() && !held {
        pending.push("sealed tokens");
    }
    if moving && !held {
        pending.push("card motion");
    }
    if world.resource::<Orbit>().is_moving() {
        pending.push("camera motion");
    }
    if !table.presentation_blocked()
        && !table.notice.starts_with("Decision not committed:")
        && controls::automatic_choice(&table.view()).is_some()
    {
        pending.push("automatic choice");
    }
    pending
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
