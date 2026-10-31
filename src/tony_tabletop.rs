//! Native presentation of the bounded Tony turn. Rules and replay come from the player seam.

use crate::tabletop_camera::{
    CameraMode, Capture as CameraCapture, FrameInput, Orbit, parse_initial_view,
};
use crate::tabletop_feedback::{self as feedback, CardFeedback, ContextButton, FeedbackSettings};
use crate::tabletop_health::{HealthAssets, RenderHealth, TabletopHealthPlugin, exit_result};
use crate::tabletop_pacing::{PresentationTime, Redraw, RequestRedraw, TabletopPacingPlugin};
use crate::{
    TokenFace,
    asset_config::AssetConfig,
    assets,
    tabletop::asset_settings::AssetSettings,
    tabletop_layout::{self as layout, Area},
    tony::{
        Harm, Observation, Session, Stage,
        continuity::{self, Replay},
    },
};
use bevy::{
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    input_focus::{
        InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    picking::hover::Hovered,
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot, save_to_disk},
    ui::Pressed,
    ui_widgets::{Activate, Button, ModalDialog, ModalDialogBarrier, RequestClose},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf};

mod controls;
mod harm;
mod harm_lifetime;
mod scene;
use crate::tabletop_reveal as reveal;
use controls::{Context, Source};
use harm::{advance_presentation, animate_harm, harm_amounts, harm_scene};
use harm_lifetime::{HarmPresentation, Playback};
use scene::synchronize_scene;

const CARD_WIDTH: f32 = 1.35;
const CARD_LENGTH: f32 = 1.89;
// Card-stock thickness at this scene's scale; 32 cards occupy 0.208 units.
const CARD_THICKNESS: f32 = 0.0065;
const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.65, 0.74, 0.73);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);
const BUTTON: Color = Color::srgb(0.12, 0.19, 0.21);
const PRIMARY: Color = Color::srgb(0.13, 0.37, 0.36);

struct Card {
    code: &'static str,
    title: &'static str,
    kind: &'static str,
    text: &'static str,
    landscape: bool,
}

// Text is the effective original English face, followed by the present turn's context.
const CARDS: &[Card] = &[
    Card {
        code: "06003",
        title: "Tony Morgan",
        kind: "Investigator · Criminal. Hunter.",
        landscape: true,
        text: "Willpower 2 · Intellect 3 · Combat 5 · Agility 2\nHealth 9 · Sanity 5\n\nYou may take an additional action during your turn, which can only be used to engage or fight an enemy with 1 or more bounties on it.\n\nElder sign: +2. Place 1 bounty on Bounty Contracts.\n\nThe first qualifying action uses this restricted action automatically (FAQ 2.5 §1.10).",
    },
    Card {
        code: "01111",
        title: "Study",
        kind: "Revealed location · Shroud 2",
        landscape: false,
        text: "Two clues per investigator: six in this three-investigator position.\n\nThe door to your study has vanished.\n\nNo connecting location is in play. A successful basic investigation discovers one clue.",
    },
    Card {
        code: "01160",
        title: "Ghoul Minion",
        kind: "Enemy · Humanoid. Monster. Ghoul.",
        landscape: false,
        text: "Fight 2 · Health 2 · Evade 2\nAttack: 1 damage and 1 horror.\n\nBasic Fight deals one damage on success. A successful Evade exhausts and disengages this enemy. It can still be fought or engaged while exhausted.\n\nA ready enemy engaged with Tony attacks after costs are paid for Draw, Resource, Investigate, or removing Internal Injury. The two-action Injury ability provokes one attack. Engaging this exhausted Ghoul causes no attack.",
    },
    Card {
        code: "06010",
        title: "Bounty Contracts",
        kind: "Permanent asset · Job.",
        landscape: false,
        text: "Permanent. Tony Morgan deck only. Uses (6 bounties).\n\nReaction — After an enemy enters play: Move 1–3 bounties from Bounty Contracts to that enemy, to a maximum of that enemy's health.\n\nForced — After you defeat an enemy with 1 or more bounties on it: Move its bounties to your resource pool, as resources.\n\nThis position begins with five bounties here and one on the Ghoul. No new enemy enters play during this turn.",
    },
    Card {
        code: "02038",
        title: "Internal Injury",
        kind: "Basic weakness · Treachery. Injury.",
        landscape: false,
        text: "Revelation — Place Internal Injury into play in your threat area.\n\nForced — At the end of your turn: Take 1 direct damage.\n\nTwo actions: Discard Internal Injury.\n\nAlready in Tony's threat area. His restricted action cannot pay this cost. An investigator at this location could use this ability on their own turn; the companions do not take turns here.",
    },
    Card {
        code: "03263",
        title: "On the Hunt",
        kind: "Event · Tactic. · Level 0 · Cost 1\nCombat icon · Intellect icon",
        landscape: false,
        text: "Fast. Play when you would draw an encounter card during the mythos phase.\n\nInstead, search the top 9 cards of the encounter deck for an enemy, spawn it engaged with you (instead of its normal spawn location), and shuffle the encounter deck. If you cannot, draw the top card of the encounter deck.\n\nDuring this turn, it can be committed to Fight or Investigate for +1. Its event timing does not occur.",
    },
    Card {
        code: "01089",
        title: "Guts",
        kind: "Skill · Innate. · Level 0\nTwo willpower icons",
        landscape: false,
        text: "Max 1 committed per skill test.\n\nIf this test is successful, draw 1 card.\n\nNo willpower test occurs during this turn, so Guts cannot be committed here.",
    },
    Card {
        code: "01104",
        title: "The Gathering",
        kind: "Original scenario · Easy / Standard",
        landscape: false,
        text: "Skull: −X. X is the number of Ghoul enemies at your location.\n\nCultist: −1. If you fail, take 1 horror.\n\nTablet: −2. If there is a Ghoul enemy at your location, take 1 damage.\n\nExhausted Ghouls still count. Tony's Elder Sign is +2 and adds one bounty to Contracts. Auto-fail fails the test. All physical tokens return at test end.",
    },
    Card {
        code: "01108",
        title: "Trapped",
        kind: "Act 1a · Six clues to advance with three investigators",
        landscape: true,
        text: "As you leap to investigate, the door to your study vanishes before your eyes, leaving behind only solid wall. You're trapped inside your study until you can find another way out.\n\nThe investigators hold no clues at the start. Tony can discover at most three during this turn, so this act cannot advance.",
    },
    Card {
        code: "01105",
        title: "What's Going On?!",
        kind: "Agenda 1a · Doom threshold 3",
        landscape: true,
        text: "No agenda threshold or scenario trigger is pending in this prepared position. Tony's turn stops before another investigator or phase begins.",
    },
];

struct Options {
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
        Self::parse_from(std::env::args().skip(1), AssetConfig::load_tony()?)
    }

    fn parse_from(
        args: impl IntoIterator<Item = String>,
        config: AssetConfig,
    ) -> Result<Option<Self>, String> {
        let mut result = Self {
            save: "local/experiment/tony.session.json".into(),
            load: None,
            art_root: Some(config.root().to_path_buf()),
            config,
            screenshot: None,
            quit_after_screenshot: false,
            initial_view: None,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--experiment" => {
                    if value()? != "tony" {
                        return Err("This mode requires --experiment tony.".into());
                    }
                }
                "--save" => result.save = value()?.into(),
                "--load" => result.load = Some(value()?.into()),
                "--art-root" => result.art_root = Some(value()?.into()),
                "--no-art" => result.art_root = None,
                "--screenshot" => result.screenshot = Some(value()?.into()),
                "--quit-after-screenshot" => result.quit_after_screenshot = true,
                "--initial-view" => result.initial_view = Some(parse_initial_view(&value()?)?),
                "--help" | "-h" => {
                    println!(
                        "Arkham Unfolded — Tony turn\n\n--experiment tony\n--save PATH\n--load PATH\n--art-root PATH\n--no-art\n--screenshot PATH\n--quit-after-screenshot\n--initial-view overview|scenario|test|tony|patrice|jacqueline  Diagnostic startup framing; Home is unchanged\n\nRight drag / arrows: orbit. Wheel / +/-: zoom. Home: reset. F12: capture."
                    );
                    return Ok(None);
                }
                _ => return Err(format!("Unknown Tony-turn option: {arg}")),
            }
        }
        if result.quit_after_screenshot && result.screenshot.is_none() {
            return Err("--quit-after-screenshot requires --screenshot PATH".into());
        }
        Ok(Some(result))
    }
}

#[derive(Resource)]
struct Table {
    session: Session,
    replay: Option<Replay>,
    save_path: PathBuf,
    notice: String,
    inspect: Option<usize>,
    history: bool,
    dirty: bool,
    presentation: HarmPresentation,
    reduced_motion: bool,
    context: Context,
}
impl Table {
    fn view(&self) -> Observation {
        if let Some(replay) = &self.replay {
            replay.current().clone()
        } else if let Some(stage) = self.presentation.current_stage() {
            stage.view.clone()
        } else {
            self.session.observe()
        }
    }
    fn harm(&self) -> Option<Harm> {
        if let Some(replay) = &self.replay {
            replay.harm().copied()
        } else {
            self.presentation.current_stage().and_then(|s| s.harm)
        }
    }
    fn presenting(&self) -> bool {
        self.replay.is_none() && self.presentation.has_pending()
    }
    fn modal(&self) -> bool {
        self.inspect.is_some() || self.history
    }
}
#[derive(Resource)]
struct Art {
    handles: BTreeMap<String, Handle<Image>>,
    symbol_font: Option<Handle<Font>>,
    status: String,
}
impl Art {
    fn text(&self, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
        crate::tabletop_harm_text::ui(value, size, color, &self.symbol_font)
    }
    fn image(&self, role: &str) -> Option<Handle<Image>> {
        self.handles.get(role).cloned()
    }
}
#[derive(Clone)]
enum Action {
    Choose(u64, String),
    Reveal(reveal::Request),
    Save,
    Reopen,
    Replay,
    Back,
    Next,
    Live,
    New,
    Inspect(usize),
    Close,
    History,
    Follow,
    AssetFolder,
    FinishAnimation,
    Motion,
    Select(u64, Source, Vec3),
    ContextChoice(u64, u64, String),
    ToggleCommit(u64, u64, u8),
    ConfirmCommit(u64),
    DismissMenu,
    Focus(Area),
    Utilities,
}
#[derive(Resource, Default)]
struct Actions(Vec<Action>);
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct Scene;
#[derive(Component)]
struct PlayerDeck;
#[derive(Component)]
struct MainCamera;
#[derive(Component)]
struct WorldLabel(Vec3, f32);

/// Anchor a caption outward from a card or paired component.
#[derive(Component)]
struct OuterLabel {
    left: bool,
}
#[derive(Component)]
struct OverviewHeading;
#[derive(Component)]
struct Appearing;
#[derive(Component)]
struct ChaosToken;
#[derive(Component)]
struct ButtonColor(Color);
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
        .or_else(|| options.save.is_file().then(|| options.save.clone()));
    let session = if let Some(path) = &load {
        continuity::load_session(path).map_err(|e| format!("Could not reopen Tony turn: {e}"))?
    } else {
        Session::new().map_err(|e| e.to_string())?
    };
    let save_path = load.unwrap_or(options.save);
    let root = options.art_root.map(|p| {
        if p.is_absolute() {
            p
        } else {
            std::env::current_dir().unwrap_or_default().join(p)
        }
    });
    let collection = root.as_deref().map(assets::inspect_tony);
    let status = collection.as_ref().map_or_else(
        || "Artwork disabled; readable substitutes are active.".into(),
        |r| r.summary(),
    );
    eprintln!("Experiment: Tony turn (--experiment tony)");
    match &root {
        Some(root) => eprintln!("Artwork root: {}", root.display()),
        None => eprintln!("Artwork root: disabled for this launch (--no-art)"),
    }
    eprintln!("{status}");
    let notice = options.config.warning().map_or_else(
        || "Choose your actions. Every committed decision is saved.".into(),
        |w| format!("Asset settings: {w}"),
    );
    let mut app = App::new();
    app.insert_resource(ClearColor(INK))
        .insert_resource(Table {
            session,
            replay: None,
            save_path,
            notice,
            inspect: None,
            history: false,
            dirty: true,
            presentation: HarmPresentation::default(),
            reduced_motion: false,
            context: Context::default(),
        })
        .insert_resource(Actions::default())
        .insert_resource(Orbit::new(CameraMode::Tony).with_initial_view(options.initial_view))
        .insert_resource(AssetSettings::new_tony(options.config))
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
                    file_path: root.map_or_else(
                        || "local/experiment/absent-art".into(),
                        |p| p.to_string_lossy().into_owned(),
                    ),
                    ..default()
                }),
        )
        .add_plugins(MeshPickingPlugin)
        .add_plugins(TabletopPacingPlugin)
        .add_plugins(TabletopHealthPlugin)
        .add_plugins(feedback::FeedbackPlugin)
        .add_plugins(reveal::RevealPlugin)
        .add_systems(Last, request_redraw.in_set(RequestRedraw))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                poll_settings,
                handle_actions,
                advance_presentation,
                camera_controls,
                controls::invalidate,
                synchronize_scene,
                synchronize_feedback,
                rebuild_panel,
                controls::draw_menu,
                sync_reveal,
                reveal::render,
                world_labels,
                crate::tabletop_tokens::position_labels,
                hide_overview_token_labels,
                animate,
                animate_harm,
                button_colors,
                capture_window,
            )
                .chain(),
        );
    let server = app.world().resource::<AssetServer>();
    let mut handles = BTreeMap::new();
    let roles = CARDS
        .iter()
        .map(|c| format!("card/{}", c.code))
        .chain(["card-back/player".to_owned()])
        .chain(["token/damage".to_owned(), "token/horror".to_owned()])
        .chain(["06003", "06005", "60401"].map(|c| format!("standee/{c}")))
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
            .map(|t| format!("token/{t}")),
        );
    for role in roles {
        if let Some(path) = collection.as_ref().and_then(|r| r.verified_path(&role)) {
            handles.insert(role, server.load(path.to_owned()));
        }
    }
    let symbol_font = collection
        .as_ref()
        .and_then(|r| r.verified_path("font/symbols"))
        .map(|path| server.load(path.to_owned()));
    let reveal_image = collection
        .as_ref()
        .and_then(|r| r.verified_path(reveal::ART_ROLE))
        .map(|path| server.load(path.to_owned()));
    let art = Art {
        handles,
        symbol_font,
        status,
    };
    let health_assets = HealthAssets::new(
        art.handles
            .values()
            .map(|handle| handle.id().untyped())
            .chain(art.symbol_font.iter().map(|handle| handle.id().untyped()))
            .chain(reveal_image.iter().map(|handle| handle.id().untyped())),
    );
    app.insert_resource(health_assets)
        .insert_resource(reveal::Artwork(reveal_image))
        .insert_resource(art);
    exit_result(app.run())
}

fn button(
    art: &Art,
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: Action,
    primary: bool,
) -> Entity {
    let color = if primary { PRIMARY } else { BUTTON };
    parent
        .spawn((
            Button,
            Hovered::default(),
            TabIndex(0),
            Node {
                min_height: px(38),
                width: percent(100),
                padding: UiRect::axes(px(12), px(8)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(4)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(color),
            ButtonColor(color),
        ))
        .observe(move |_: On<Activate>, mut queue: ResMut<Actions>| queue.0.push(action.clone()))
        .with_child(art.text(label, 16.0, PAPER))
        .id()
}

fn poll_settings(mut settings: ResMut<AssetSettings>, mut table: ResMut<Table>) {
    if let Some(notice) = settings.poll() {
        table.notice = notice;
        table.dirty = true;
    }
}
#[allow(clippy::too_many_arguments)]
fn handle_actions(
    health: Res<RenderHealth>,
    mut queue: ResMut<Actions>,
    mut table: ResMut<Table>,
    mut orbit: ResMut<Orbit>,
    mut settings: ResMut<AssetSettings>,
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: Option<ResMut<InputFocus>>,
    mut shortcut: Option<ResMut<reveal::Shortcut>>,
) {
    let view = table.view();
    let replay = table.replay.as_ref().map(Replay::index);
    table.context.sync(view.revision, replay);
    if keys.just_pressed(KeyCode::Escape) && table.modal() {
        queue.0.push(Action::Close);
    } else if keys.just_pressed(KeyCode::Escape) {
        queue.0.push(Action::DismissMenu);
    }
    if let Some(shortcut) = shortcut.as_mut() {
        queue
            .0
            .extend(shortcut.drain().into_iter().map(Action::Reveal));
    }
    for action in queue.0.drain(..) {
        if (!health.allows_play() || health.just_became_ready())
            && !matches!(
                action,
                Action::Focus(_) | Action::Close | Action::DismissMenu
            )
        {
            continue;
        }
        if settings.is_busy() || (table.modal() && !matches!(action, Action::Close)) {
            continue;
        }
        let mut reveal_focus = false;
        let action = match action {
            Action::Reveal(request) => {
                let Some(shortcut) = shortcut.as_mut() else {
                    continue;
                };
                refresh_reveal(&table, &settings, shortcut);
                match shortcut.admit(request) {
                    Some(reveal::Accepted::Choice(revision, id)) => {
                        reveal_focus = true;
                        table.context.dismiss();
                        Action::Choose(revision, id.into())
                    }
                    Some(reveal::Accepted::Opened | reveal::Accepted::Dismissed) => {
                        table.context.dismiss();
                        if let Some(focus) = focus.as_mut() {
                            focus.clear();
                        }
                        continue;
                    }
                    None => continue,
                }
            }
            Action::ContextChoice(epoch, revision, id) => {
                let Some(choice) = table.context.contextual_choice(epoch, revision, id) else {
                    continue;
                };
                choice
            }
            Action::ConfirmCommit(revision) => {
                let Some(choice) = table.context.confirm(&table.view(), revision) else {
                    continue;
                };
                choice
            }
            action => action,
        };
        if !reveal_focus && let Some(shortcut) = shortcut.as_mut() {
            shortcut.dismiss();
        }
        match action {
            Action::Choose(revision, id) if table.replay.is_none() && !table.presenting() => {
                let table = &mut *table;
                let before = table.session.observe();
                table.notice = match continuity::commit_choice(
                    &table.save_path,
                    &mut table.session,
                    revision,
                    &id,
                ) {
                    Ok(()) => {
                        table
                            .presentation
                            .committed_choice(&before, &table.session.observe());
                        if reveal_focus {
                            orbit.focus(layout::focus(Area::Test));
                            if let Some(focus) = focus.as_mut() {
                                focus.clear();
                            }
                        }
                        "Decision saved. Close and reopen safely at any choice.".into()
                    }
                    Err(e) => format!("Decision not committed: {e}"),
                };
            }
            Action::Save if table.replay.is_none() => {
                table.notice = match continuity::save_session(&table.save_path, &table.session) {
                    Ok(()) => "Live turn saved.".into(),
                    Err(e) => format!("Save failed; the live turn is unchanged: {e}"),
                }
            }
            Action::Reopen if table.replay.is_none() => {
                match continuity::load_session(&table.save_path) {
                    Ok(session) => {
                        table.session = session;
                        table.presentation.discard_staging();
                        table.notice = "Reopened the exact saved choice and randomness.".into();
                    }
                    Err(e) => {
                        table.notice = format!("Reopen failed; the live turn is unchanged: {e}")
                    }
                }
            }
            Action::Replay => {
                let record = table.session.recording();
                let digest = format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(&record).expect("recordings serialize"))
                );
                let stem = table
                    .save_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("tony");
                let path = table
                    .save_path
                    .with_file_name(format!("{stem}.replay-{}.json", &digest[..16]));
                match continuity::save_recording(&path, &record)
                    .and_then(|()| continuity::load_recording(&path))
                    .and_then(Replay::new)
                {
                    Ok(replay) => {
                        table.replay = Some(replay);
                        table.presentation.discard_staging();
                        orbit.set_following(true);
                        table.notice = "Recorded replay. Each step pauses for narration.".into();
                    }
                    Err(e) => table.notice = format!("Could not open recorded replay: {e}"),
                }
            }
            Action::Back => {
                if let Some(r) = &mut table.replay {
                    r.back();
                    table.presentation.replay_navigated();
                }
            }
            Action::Next => {
                if let Some(r) = &mut table.replay {
                    r.next();
                    table.presentation.replay_navigated();
                }
            }
            Action::Live => {
                table.replay = None;
                table.presentation.discard_staging();
                orbit.set_following(false);
                table.notice = "Returned to the unchanged live turn.".into();
            }
            Action::New
                if table.replay.is_none()
                    && !table.presenting()
                    && table.session.observe().stage == Stage::Ended =>
            {
                match Session::new() {
                    Ok(session) => match continuity::restart_session(
                        &table.save_path,
                        &table.session,
                        &session,
                    ) {
                        Ok(()) => {
                            table.session = session;
                            table.presentation.discard_staging();
                            orbit.set_following(false);
                            table.notice =
                                "New turn ready. The previous turn has been retained.".into();
                        }
                        Err(e) => table.notice = format!("Could not start another turn: {e}"),
                    },
                    Err(e) => table.notice = format!("Could not start another turn: {e}"),
                }
            }
            Action::Select(revision, source, at) => {
                let view = table.view();
                let gameplay = table.replay.is_none() && !table.presenting();
                if let Some(effect) = table.context.select(&view, gameplay, revision, source, at) {
                    if let controls::SelectionEffect::Inspect(index) = effect {
                        table.inspect = Some(index);
                    }
                    if let Some(focus) = focus.as_mut() {
                        focus.clear();
                    }
                }
            }
            Action::ToggleCommit(epoch, revision, id) => {
                let view = table.view();
                let gameplay = table.replay.is_none() && !table.presenting();
                table
                    .context
                    .toggle_commitment(&view, gameplay, epoch, revision, id);
            }
            Action::DismissMenu => {
                table.context.close_menus();
                if let Some(focus) = focus.as_mut() {
                    focus.clear();
                }
            }
            Action::Focus(area) => {
                orbit.focus(layout::focus(area));
                table.context.dismiss();
                if let Some(focus) = focus.as_mut() {
                    focus.clear();
                }
            }
            Action::Utilities => {
                table.context.toggle_utilities();
            }
            Action::Inspect(index) => {
                table.inspect = Some(table.context.inspect(index));
            }
            Action::Close => {
                table.inspect = None;
                table.history = false;
            }
            Action::History => {
                table.context.dismiss();
                table.history = true;
            }
            Action::FinishAnimation if table.replay.is_none() => {
                table.presentation.finish_live_stage();
            }
            Action::Motion => table.reduced_motion = !table.reduced_motion,
            Action::Follow if table.replay.is_some() => {
                let following = !orbit.is_following();
                orbit.set_following(following);
            }
            Action::AssetFolder => {
                settings.choose();
                table.notice =
                    "Choose a prepared Tony asset folder. Current play is unchanged.".into();
            }
            _ => {}
        }
        if orbit.is_following() && table.replay.is_some() {
            let target = if let Some(hit) = table.harm() {
                (harm::recipient_position(hit.recipient, &table.view()), 16.0)
            } else if table.view().held_token.is_some() {
                layout::focus(Area::Test)
            } else {
                layout::focus(Area::Investigator(0))
            };
            orbit.update_follow_target(Some(target));
        }
        let view = table.view();
        let replay = table.replay.as_ref().map(Replay::index);
        table.context.sync(view.revision, replay);
        table.dirty = true;
    }
}

fn refresh_reveal(table: &Table, settings: &AssetSettings, shortcut: &mut reveal::Shortcut) {
    let view = table.view();
    shortcut.sync(
        view.revision,
        &view.choices,
        table.replay.is_none() && !table.presenting() && !table.modal() && !settings.is_busy(),
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
    // A background hit target dismisses controls on empty felt and outside the mats.
    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
            MeshMaterial3d(materials.add(INK)),
            Transform::from_xyz(0.0, -0.5, 0.0),
        ))
        .observe(|event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::DismissMenu);
            }
        });
    commands.spawn((
        DirectionalLight {
            illuminance: 7_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.0)),
    ));
    commands.spawn((Camera3d::default(), Transform::default(), MainCamera));
}
fn world_label(
    commands: &mut Commands,
    art: &Art,
    value: String,
    point: Vec3,
    color: Color,
) -> Entity {
    world_label_sized(commands, art, value, point, color, 160.0, 13.0)
}

fn world_label_sized(
    commands: &mut Commands,
    art: &Art,
    value: String,
    point: Vec3,
    color: Color,
    width: f32,
    font_size: f32,
) -> Entity {
    commands
        .spawn((
            art.text(value, font_size, color),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                width: px(width),
                padding: UiRect::axes(px(5), px(3)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.055, 0.92)),
            Pickable::IGNORE,
            WorldLabel(point, width),
            Scene,
        ))
        .id()
}
fn area_heading(commands: &mut Commands, art: &Art, value: String, point: Vec3) {
    let entity = world_label(commands, art, value, point, GOLD);
    commands.entity(entity).insert(OverviewHeading);
}

#[derive(Component)]
struct CardSource(Source);

fn synchronize_feedback(
    table: Res<Table>,
    settings: Res<AssetSettings>,
    mut feedback_settings: ResMut<FeedbackSettings>,
    mut cards: Query<(&CardSource, &mut CardFeedback)>,
) {
    feedback_settings.blocked = table.modal() || settings.is_busy();
    feedback_settings.reduced_motion = table.reduced_motion;
    for (source, mut marks) in &mut cards {
        marks.menu_source = table.context.is_menu_source(source.0);
        marks.pending_choice = table.context.is_pending(source.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn card_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    index: usize,
    at: Vec3,
    exhausted: bool,
    selection: (u64, Source),
) {
    let card = &CARDS[index];
    let image = art.image(&format!("card/{}", card.code));
    let material = materials.add(StandardMaterial {
        base_color: if image.is_some() {
            Color::WHITE
        } else {
            Color::srgb(0.34, 0.31, 0.23)
        },
        base_color_texture: image,
        unlit: true,
        ..default()
    });
    let transform = scene::card_transform(index, at, exhausted);
    let anchor = transform.translation;
    let entity = commands
        .spawn((
            Name::new(format!("Card {}", card.code)),
            Mesh3d(meshes.add(Plane3d::default().mesh().size(CARD_WIDTH, CARD_LENGTH))),
            MeshMaterial3d(material),
            transform,
            Scene,
            CardSource(selection.1),
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue
                    .0
                    .push(Action::Select(selection.0, selection.1, anchor));
            }
        })
        .id();
    feedback::attach_card_feedback(
        commands,
        meshes,
        materials,
        entity,
        CARD_WIDTH,
        CARD_LENGTH,
        0.0,
    );
}
fn deck_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    count: usize,
    revision: u64,
) {
    if count == 0 {
        return;
    }
    // Sit on the tabletop. Shared geometry/materials keep the individual edges cheap.
    let base = scene::deck_at();
    let card = meshes.add(Cuboid::new(CARD_WIDTH, CARD_THICKNESS, CARD_LENGTH));
    let edges = [Color::srgb(0.76, 0.73, 0.65), Color::srgb(0.57, 0.55, 0.49)].map(|color| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 1.0,
            ..default()
        })
    });
    for layer in 0..count {
        commands.spawn((
            Mesh3d(card.clone()),
            MeshMaterial3d(edges[layer % edges.len()].clone()),
            Transform::from_translation(base + Vec3::Y * ((layer as f32 + 0.5) * CARD_THICKNESS)),
            Scene,
            PlayerDeck,
            Pickable::IGNORE,
        ));
    }
    let image = art.image("card-back/player");
    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(CARD_WIDTH, CARD_LENGTH))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if image.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(0.12, 0.19, 0.30)
                },
                base_color_texture: image,
                unlit: true,
                depth_bias: 1.0,
                ..default()
            })),
            Transform::from_translation(base + Vec3::Y * (count as f32 * CARD_THICKNESS)),
            Scene,
            PlayerDeck,
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::Select(revision, Source::Deck, base));
            }
        });
}
fn token_role(face: TokenFace) -> &'static str {
    match face {
        TokenFace::PlusOne => "plus_one",
        TokenFace::Zero => "zero",
        TokenFace::MinusOne => "minus_one",
        TokenFace::MinusTwo => "minus_two",
        TokenFace::MinusThree => "minus_three",
        TokenFace::MinusFour => "minus_four",
        TokenFace::Skull => "skull",
        TokenFace::Cultist => "cultist",
        TokenFace::Tablet => "tablet",
        TokenFace::AutoFail => "auto_fail",
        TokenFace::ElderSign => "elder_sign",
        TokenFace::Curse => "curse",
    }
}
#[allow(clippy::too_many_arguments)]
fn rebuild_panel(
    mut commands: Commands,
    mut table: ResMut<Table>,
    art: Res<Art>,
    orbit: Res<Orbit>,
    panels: Query<Entity, With<Panel>>,
    window: Single<&Window>,
    mut last_size: Local<Vec2>,
    images: Option<Res<Assets<Image>>>,
    mut last_portrait_readiness: Local<[bool; 3]>,
) {
    let portraits = ["06003", "06005", "60401"].map(|code| {
        art.image(&format!("standee/{code}")).and_then(|handle| {
            images
                .as_ref()
                .and_then(|images| images.get(&handle))
                .map(|image| (handle, image.size_f32()))
        })
    });
    let readiness = portraits.each_ref().map(Option::is_some);
    if *last_portrait_readiness != readiness {
        *last_portrait_readiness = readiness;
        table.dirty = true;
    }
    let size = Vec2::new(window.width(), window.height());
    if *last_size != size {
        *last_size = size;
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
    commands.spawn((Node { position_type: PositionType::Absolute, left: px(16), top: px(10), right: px(125),
        flex_direction: FlexDirection::Column, row_gap: px(3), ..default() }, Pickable::IGNORE, Panel))
        .with_children(|p| {
            p.spawn(art.text("Tony turn · The Gathering · Standard · Prepared position", 19.0, GOLD));
            p.spawn(art.text(format!("Resources {} · Clues {} · Damage {}/{} · Horror {}/{}   |   {} ordinary actions · Bonus Fight/Engage: {}",
                view.resources, view.clues, view.damage, view.health, view.horror, view.sanity, view.ordinary_actions,
                if view.bonus_action { "available" } else { "spent" }), 14.0, PAPER));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                top: px(12),
                width: px(108),
                ..default()
            },
            Panel,
        ))
        .with_children(|p| {
            button(&art, p, "Menu", Action::Utilities, false);
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(10),
                top: px(76),
                bottom: px(180),
                width: px(124),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Pickable::IGNORE,
            Panel,
        ))
        .observe(scroll_panel)
        .with_children(|p| {
            for (index, name) in ["Tony", "Patrice", "Jacqueline"].into_iter().enumerate() {
                let color = layout::investigator_color(index);
                let portrait = portraits[index]
                    .as_ref()
                    .map(|(image, size)| (image, size * (108.0 / size.x).min(82.0 / size.y)));
                p.spawn((
                    Button,
                    Hovered::default(),
                    TabIndex(0),
                    AccessibleLabel::new(format!("Focus {name}")),
                    Node {
                        width: portrait.map_or(percent(100), |(_, size)| px(size.x + 10.0)),
                        min_height: px(portrait.map_or(94.0, |(_, size)| size.y + 10.0)),
                        padding: UiRect::all(px(5)),
                        align_self: AlignSelf::Center,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(color),
                    ButtonColor(color),
                ))
                .observe(move |_: On<Activate>, mut queue: ResMut<Actions>| {
                    queue.0.push(Action::Focus(Area::Investigator(index)))
                })
                .with_children(|p| {
                    if let Some((image, size)) = portrait {
                        p.spawn((
                            ImageNode::new(image.clone()),
                            Node {
                                width: px(size.x),
                                height: px(size.y),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    } else {
                        p.spawn(art.text(name, 14.0, PAPER));
                    }
                });
            }
            for (label, area) in [
                ("Scenario", Area::Scenario),
                ("Test", Area::Test),
                ("Overview", Area::Overview),
            ] {
                button(&art, p, label, Action::Focus(area), false);
            }
            p.spawn(art.text(
                "WASD: pan\nRight drag: orbit\nWheel: zoom\nHome: Tony",
                11.0,
                MUTED,
            ));
        });
    commands.spawn((Node { position_type: PositionType::Absolute, left: px(150), right: px(12), bottom: px(10),
        max_height: px((size.y * 0.29).min(250.0)), padding: UiRect::all(px(12)), flex_direction: FlexDirection::Column,
        row_gap: px(5), overflow: Overflow::scroll_y(), ..default() }, BackgroundColor(INK), Panel))
        .observe(scroll_panel).with_children(|p| {
            p.spawn(art.text(table.replay.as_ref().map_or_else(|| "TONY · LIVE DECISION".into(),
                |r| format!("REPLAY · MOMENT {} / {}", r.index() + 1, r.len())), 12.0, GOLD));
            p.spawn(art.text(&view.prompt, 18.0, PAPER));
            if let Some(test) = &view.test {
                p.spawn(art.text(format!("{}: skill {} vs {} · {}{}", test.kind.label(), test.skill_value, test.difficulty,
                    test.success.map_or("Pending", |ok| if ok { "Success" } else { "Failure" }),
                    test.modifier.map_or(String::new(), |m| format!(" · modifier {m:+}"))), 15.0, GOLD));
            }
            if let Some(hit) = table.harm() {
                p.spawn(art.text(hit.source.label(), 19.0, GOLD));
                harm_amounts(p, &art, hit);
            }
            p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(5), ..default() })
                .with_children(|row| {
                    let mut control = |label: &str, action: Action, primary| {
                        row.spawn(Node { width: px(230), ..default() }).with_children(|p| { button(&art, p, label, action, primary); });
                    };
                    if let Some(replay) = &table.replay {
                        if replay.index() > 0 { control("← Previous moment", Action::Back, false); }
                        if replay.index() + 1 < replay.len() { control("Next moment →", Action::Next, true); }
                        control("Return to live turn", Action::Live, false);
                    } else if table.presenting() { control("Finish animation", Action::FinishAnimation, false); }
                    else {
                        for choice in &view.choices {
                            if matches!(choice.id.as_str(), "end_turn" | "undo" | "commit:none") {
                                control(&choice.label, Action::Choose(view.revision, choice.id.clone()), choice.id == "commit:none");
                            }
                        }
                        if let Some((label, action)) = table.context.confirmation_control(&view) {
                            control(&label, action, true);
                        }
                        if view.stage == Stage::Ended { control("Start another Tony turn", Action::New, true); }
                    }
                });
            if let Some(summary) = table.context.pending_summary() {
                p.spawn(art.text(summary, 13.0, GOLD));
            }
            if table.replay.is_none() && !table.presenting() {
                let hint = match view.stage {
                    Stage::Action => "Select a component: Ghoul for Fight/Evade, Study for Investigate, Tony for Resource, deck for Draw.",
                    Stage::Commit => "Select cards in Tony's hand, then Commit selected; or commit no cards.",
                    Stage::Reveal => "Use Reveal chaos token at the lower left, or select the chaos bag.",
                    Stage::Resolve => "Focus Test, then select the revealed token to resolve.",
                    Stage::Ended => "Only Tony's prepared turn has ended. Companions do not take turns here.",
                };
                p.spawn(art.text(hint, 12.0, MUTED));
            }
            if !table.notice.is_empty() { p.spawn(art.text(&table.notice, 12.0, MUTED)); }
        });
    if table.context.utilities_open() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(12),
                    top: px(62),
                    width: px(305),
                    max_height: px((size.y - 220.0).max(150.0)),
                    padding: UiRect::all(px(12)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(7),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(INK),
                GlobalZIndex(45),
                Panel,
            ))
            .observe(scroll_panel)
            .with_children(|p| {
                button(
                    &art,
                    p,
                    "Action history and rules reasons",
                    Action::History,
                    false,
                );
                button(
                    &art,
                    p,
                    if table.reduced_motion {
                        "Motion: reduced"
                    } else {
                        "Motion: animated"
                    },
                    Action::Motion,
                    false,
                );
                if table.replay.is_none() {
                    button(&art, p, "Save live turn", Action::Save, false);
                    button(&art, p, "Reopen saved turn", Action::Reopen, false);
                    button(&art, p, "Open recorded replay", Action::Replay, false);
                } else {
                    button(
                        &art,
                        p,
                        if orbit.is_following() {
                            "Camera following: on"
                        } else {
                            "Resume camera following"
                        },
                        Action::Follow,
                        false,
                    );
                }
                p.spawn(art.text(&art.status, 12.0, MUTED));
                button(&art, p, "Choose asset folder", Action::AssetFolder, false);
                button(&art, p, "Close menu", Action::DismissMenu, false);
            });
    }
    if table.modal() {
        inspection(&mut commands, &table, &view, &art, size);
    }
}

fn inspection(
    commands: &mut Commands,
    table: &Table,
    view: &Observation,
    art: &Art,
    viewport: Vec2,
) {
    let width = (viewport.x * 0.94).min(1200.0);
    let height = viewport.y * 0.90;
    let index = table.inspect;
    let card = index.map(|i| &CARDS[i]);
    let title = card.map_or("Action history", |c| c.title);
    commands.spawn((Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100), align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() }, BackgroundColor(Color::srgba(0.015, 0.025, 0.03, 0.88)), GlobalZIndex(90), ModalDialogBarrier, Panel))
        .observe(|_: On<RequestClose>, mut queue: ResMut<Actions>| queue.0.push(Action::Close))
        .with_children(|overlay| {
            overlay.spawn((Node { width: px(width), max_height: px(height), padding: UiRect::all(px(22)), flex_direction: FlexDirection::Column, row_gap: px(16), ..default() }, BackgroundColor(INK), ModalDialog, TabGroup::modal()))
                .with_children(|p| {
                    p.spawn(art.text(title, 27.0, GOLD));
                    button(art, p, "Close · Esc", Action::Close, false);
                    p.spawn((Node { max_height: px((height - 150.0).max(100.0)), overflow: Overflow::scroll_y(), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(16), ..default() }, ScrollPosition::default()))
                        .observe(scroll_panel)
                        .with_children(|body| {
                            if let Some(card) = card {
                                if let Some(image) = art.image(&format!("card/{}", card.code)) {
                                    let w = (width - 55.0).min(if card.landscape { 760.0 } else { 430.0 });
                                    let h = w * if card.landscape { 5.0 / 7.0 } else { 7.0 / 5.0 };
                                    body.spawn(Node { width: px(w), height: px(h), flex_shrink: 0.0, ..default() }).with_children(|frame| {
                                        frame.spawn((ImageNode::new(image), Node { position_type: PositionType::Absolute, left: percent(50), top: percent(50), width: px(if card.landscape { h } else { w }), height: px(if card.landscape { w } else { h }), ..default() }, UiTransform { translation: Val2::percent(-50, -50), rotation: if card.landscape { Rot2::degrees(-90.0) } else { Rot2::IDENTITY }, ..default() }, Pickable::IGNORE));
                                    });
                                }
                                body.spawn((art.text(card.kind, 18.0, GOLD), Node { width: percent(100), ..default() }));
                                body.spawn((art.text(card.text, 20.0, PAPER), Node { width: percent(100), ..default() }));
                            } else {
                                for line in &view.history { body.spawn((art.text(line, 18.0, PAPER), Node { width: percent(100), ..default() })); }
                                body.spawn((art.text("Rules: original Rules Reference (actions pp3–5, Fight/Evade/Investigate pp11–13, skill tests pp26–27), legacy FAQ 2.5 §1.10, original Gathering Standard reference and the displayed card faces.", 14.0, MUTED), Node { width: percent(100), ..default() }));
                            }
                        });
                });
        });
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
    mut shortcut: Option<ResMut<reveal::Shortcut>>,
) {
    let modal = table.modal() || settings.is_busy();
    let captures_keys = modal
        || table.context.captures_keyboard()
        || shortcut.as_ref().is_some_and(|s| s.is_open())
        || focus
            .as_ref()
            .and_then(|focus| focus.get())
            .is_some_and(|entity| focusable.contains(entity));
    let over_ui = hover
        .as_ref()
        .and_then(|hover| hover.get(&bevy::picking::pointer::PointerId::Mouse))
        .is_some_and(|hits| hits.keys().any(|entity| ui_nodes.contains(*entity)));
    let step = orbit.step(FrameInput {
        keys: &keys,
        buttons: &buttons,
        window: &window,
        scroll_y: scroll.delta.y,
        dt: time.delta_secs(),
        capture: CameraCapture {
            pointer_blocked: modal,
            over_ui,
            keyboard: captures_keys,
            wheel: table.context.is_open() || shortcut.as_ref().is_some_and(|s| s.is_open()),
        },
        reduced_motion: table.reduced_motion,
    });
    if step.manual_navigation {
        table.context.dismiss();
        if let Some(shortcut) = shortcut.as_mut() {
            shortcut.dismiss();
        }
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
        Has<OverviewHeading>,
        Option<&OuterLabel>,
        &mut Node,
        &mut Visibility,
    )>,
) {
    let (camera, transform) = *camera;
    for (label, heading, standee, mut node, mut visibility) in &mut labels {
        if orbit.pose().distance > 30.0 && !heading {
            *visibility = Visibility::Hidden;
            continue;
        }
        if let Ok(point) = camera.world_to_viewport(transform, label.0) {
            let offset =
                standee.map_or(label.1 / 2.0, |side| if side.left { label.1 } else { 0.0 });
            node.left = px(point.x - offset);
            node.top = px(point.y);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}
fn hide_overview_token_labels(
    orbit: Res<Orbit>,
    mut labels: Query<&mut Visibility, With<crate::tabletop_tokens::TokenLabel>>,
) {
    if orbit.pose().distance > 30.0 {
        for mut visibility in &mut labels {
            *visibility = Visibility::Hidden;
        }
    }
}
fn animate(
    time: Res<PresentationTime>,
    table: Res<Table>,
    mut transforms: Query<&mut Transform, With<Appearing>>,
) {
    for mut t in &mut transforms {
        t.scale = if table.reduced_motion {
            Vec3::ONE
        } else {
            let scale = t.scale.lerp(Vec3::ONE, (time.delta_secs() * 8.0).min(1.0));
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
    table: Res<Table>,
    settings: Res<AssetSettings>,
    appearing: Query<&Transform, With<Appearing>>,
    capture: Res<Capture>,
    screenshots: Query<(), With<Screenshot>>,
) {
    let harm_running = !table.modal()
        && !settings.is_busy()
        && if table.replay.is_some() {
            table.harm().is_some() && table.presentation.animation_progress() < 1.0
        } else {
            table.presentation.has_pending()
        };
    if orbit.is_moving()
        || harm_running
        || appearing.iter().any(|t| t.scale != Vec3::ONE)
        || (capture.path.is_some() && (!capture.requested || capture.quit))
        || !screenshots.is_empty()
    {
        redraw.request();
    }
}
#[allow(clippy::type_complexity)]
fn button_colors(
    mut buttons: Query<
        (
            Entity,
            &Hovered,
            Has<Pressed>,
            &ButtonColor,
            &mut BackgroundColor,
        ),
        (With<Button>, Without<ContextButton>),
    >,
    focus: Res<InputFocus>,
) {
    for (entity, hover, pressed, tone, mut background) in &mut buttons {
        background.0 = if pressed {
            GOLD
        } else if hover.0 || focus.get() == Some(entity) {
            Color::srgb(0.24, 0.40, 0.39)
        } else {
            tone.0
        };
    }
}
fn scroll_panel(event: On<PointerScroll>, mut panels: Query<(&mut ScrollPosition, &ComputedNode)>) {
    if let Ok((mut position, node)) = panels.get_mut(event.entity) {
        let delta = event.y
            * if event.unit == MouseScrollUnit::Line {
                32.0
            } else {
                1.0
            };
        position.y =
            (position.y - delta).clamp(0.0, (node.content_size().y - node.size().y).max(0.0));
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
        let path = "local/experiment/tony-tabletop.png";
        let _ = std::fs::create_dir_all("local/experiment");
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
        && capture.path.as_ref().is_some_and(|p| p.is_file())
    {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests;
