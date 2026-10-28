//! Native presentation of the bounded Tony turn. Rules and replay come from the player seam.

use crate::{
    TokenFace,
    asset_config::AssetConfig,
    assets,
    tabletop::asset_settings::AssetSettings,
    tony::{
        Harm, Observation, Session, Stage,
        continuity::{self, Replay},
        presentation::{self, PresentationStage},
    },
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
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
};

mod harm;
use harm::{advance_presentation, animate_harm, harm_amounts, harm_scene};

const PANEL_WIDTH: f32 = 430.0;
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
                "--help" | "-h" => {
                    println!(
                        "Arkham Unfolded — Tony turn\n\n--experiment tony\n--save PATH\n--load PATH\n--art-root PATH\n--no-art\n--screenshot PATH\n--quit-after-screenshot\n\nRight drag / arrows: orbit. Wheel / +/-: zoom. Home: reset. F12: capture."
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
    presentation: VecDeque<PresentationStage>,
    stage_elapsed: f32,
    reduced_motion: bool,
}
impl Table {
    fn view(&self) -> Observation {
        if let Some(replay) = &self.replay {
            replay.current().clone()
        } else if let Some(stage) = self.presentation.front() {
            stage.view.clone()
        } else {
            self.session.observe()
        }
    }
    fn harm(&self) -> Option<Harm> {
        if let Some(replay) = &self.replay {
            replay.harm().copied()
        } else {
            self.presentation.front().and_then(|s| s.harm)
        }
    }
    fn presenting(&self) -> bool {
        self.replay.is_none() && !self.presentation.is_empty()
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
#[derive(Component)]
struct Appearing;
#[derive(Component)]
struct ChaosToken;
#[derive(Component)]
struct ButtonColor(Color);
#[derive(Resource)]
struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
    desired_target: Vec3,
    follow: bool,
}
impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 1.0,
            distance: 16.0,
            target: Vec3::new(2.2, 0.0, 0.0),
            desired_target: Vec3::new(2.2, 0.0, 0.0),
            follow: false,
        }
    }
}
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
            presentation: VecDeque::new(),
            stage_elapsed: 0.0,
            reduced_motion: false,
        })
        .insert_resource(Actions::default())
        .insert_resource(Orbit::default())
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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                poll_settings,
                handle_actions,
                advance_presentation,
                camera_controls,
                synchronize_scene,
                rebuild_panel,
                world_labels,
                crate::tabletop_tokens::position_labels,
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
    app.insert_resource(Art {
        handles,
        symbol_font,
        status,
    });
    app.run();
    Ok(())
}

fn button(
    art: &Art,
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: Action,
    primary: bool,
) {
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
        .with_child(art.text(label, 16.0, PAPER));
}

fn action_button(
    art: &Art,
    parent: &mut ChildSpawnerCommands,
    view: &Observation,
    choice: &crate::Choice,
) {
    let action = Action::Choose(view.revision, choice.id.clone());
    if let Some(attack) = view.opportunity_attack(&choice.id) {
        let color = Color::srgb(0.24, 0.17, 0.07);
        parent
            .spawn((
                Button,
                Hovered::default(),
                TabIndex(0),
                Node {
                    width: percent(100),
                    padding: UiRect::all(px(10)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    border: UiRect::left(px(4)),
                    flex_shrink: 0.0,
                    ..default()
                },
                BorderColor::all(GOLD),
                BackgroundColor(color),
                ButtonColor(color),
            ))
            .observe(move |_: On<Activate>, mut queue: ResMut<Actions>| {
                queue.0.push(action.clone())
            })
            .with_children(|p| {
                p.spawn(art.text(format!("! {}", choice.label), 16.0, PAPER));
                p.spawn(art.text(
                    format!(
                        "Attack of opportunity: Ghoul Minion\n{} damage + {} horror",
                        attack.damage, attack.horror
                    ),
                    14.0,
                    GOLD,
                ));
            });
    } else {
        button(
            art,
            parent,
            &choice.label,
            action,
            choice.id != "undo" && choice.id != "end_turn",
        );
    }
}
fn poll_settings(mut settings: ResMut<AssetSettings>, mut table: ResMut<Table>) {
    if let Some(notice) = settings.poll() {
        table.notice = notice;
        table.dirty = true;
    }
}
fn handle_actions(
    mut queue: ResMut<Actions>,
    mut table: ResMut<Table>,
    mut orbit: ResMut<Orbit>,
    mut settings: ResMut<AssetSettings>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Escape) && table.modal() {
        queue.0.push(Action::Close);
    }
    for action in queue.0.drain(..) {
        if settings.is_busy() || (table.modal() && !matches!(action, Action::Close)) {
            continue;
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
                        table.presentation =
                            presentation::stages(&before, &table.session.observe())
                                .into_iter()
                                .filter(|s| s.harm.is_some())
                                .collect();
                        table.stage_elapsed = 0.0;
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
                        table.presentation.clear();
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
                        table.presentation.clear();
                        table.stage_elapsed = 0.0;
                        orbit.follow = true;
                        table.notice = "Recorded replay. Each step pauses for narration.".into();
                    }
                    Err(e) => table.notice = format!("Could not open recorded replay: {e}"),
                }
            }
            Action::Back => {
                if let Some(r) = &mut table.replay {
                    r.back();
                    table.stage_elapsed = 0.0;
                }
            }
            Action::Next => {
                if let Some(r) = &mut table.replay {
                    r.next();
                    table.stage_elapsed = 0.0;
                }
            }
            Action::Live => {
                table.replay = None;
                table.presentation.clear();
                orbit.follow = false;
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
                            table.presentation.clear();
                            orbit.follow = false;
                            table.notice =
                                "New turn ready. The previous turn has been retained.".into();
                        }
                        Err(e) => table.notice = format!("Could not start another turn: {e}"),
                    },
                    Err(e) => table.notice = format!("Could not start another turn: {e}"),
                }
            }
            Action::Inspect(index) => table.inspect = Some(index),
            Action::Close => {
                table.inspect = None;
                table.history = false;
            }
            Action::History => table.history = true,
            Action::FinishAnimation if table.replay.is_none() => {
                table.presentation.pop_front();
                table.stage_elapsed = 0.0;
            }
            Action::Motion => table.reduced_motion = !table.reduced_motion,
            Action::Follow if table.replay.is_some() => orbit.follow = !orbit.follow,
            Action::AssetFolder => {
                settings.choose();
                table.notice =
                    "Choose a prepared Tony asset folder. Current play is unchanged.".into();
            }
            _ => {}
        }
        if orbit.follow && table.replay.is_some() {
            orbit.desired_target = if let Some(hit) = table.harm() {
                harm::recipient_position(hit.recipient)
            } else if table.view().held_token.is_some() {
                Vec3::new(1.0, 0.0, 0.2)
            } else {
                Vec3::new(2.2, 0.0, -0.7)
            };
        }
        table.dirty = true;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(14.0, 0.3, 10.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.035, 0.16, 0.15))),
        Transform::from_xyz(0.0, -0.2, 0.0),
        Pickable::IGNORE,
    ));
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
fn world_label(commands: &mut Commands, art: &Art, value: String, point: Vec3, color: Color) {
    commands.spawn((
        art.text(value, 13.0, color),
        Node {
            position_type: PositionType::Absolute,
            width: px(160),
            padding: UiRect::axes(px(5), px(3)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.045, 0.055, 0.92)),
        Pickable::IGNORE,
        WorldLabel(point, 160.0),
        Scene,
    ));
}
fn card_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    index: usize,
    at: Vec3,
    exhausted: bool,
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
    let rotation = if card.landscape || exhausted {
        std::f32::consts::FRAC_PI_2
    } else {
        0.0
    };
    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(CARD_WIDTH, CARD_LENGTH))),
            MeshMaterial3d(material),
            Transform::from_translation(at).with_rotation(Quat::from_rotation_y(rotation)),
            Scene,
        ))
        .observe(move |_: On<PointerClick>, mut queue: ResMut<Actions>| {
            queue.0.push(Action::Inspect(index))
        });
}
fn deck_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    count: usize,
) {
    if count == 0 {
        return;
    }
    // Sit on the tabletop. Shared geometry/materials keep the individual edges cheap.
    let base = Vec3::new(4.8, -0.05, 2.5);
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
    commands.spawn((
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
        Pickable::IGNORE,
    ));
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
fn synchronize_scene(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<Art>,
    entities: Query<Entity, With<Scene>>,
    token: Query<&Transform, With<ChaosToken>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rendered: Local<Option<Observation>>,
) {
    let view = table.view();
    if rendered.as_ref() == Some(&view) {
        return;
    }
    let token_scale = if table.reduced_motion {
        Vec3::ONE
    } else if rendered
        .as_ref()
        .is_some_and(|last| last.held_token == view.held_token)
    {
        token.single().map_or(Vec3::ONE, |t| t.scale)
    } else {
        Vec3::splat(0.1)
    };
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    for (index, at, caption, exhausted) in [
        (
            0,
            Vec3::new(-4.7, 0.05, 1.2),
            format!("Tony\nDamage {}/9\nHorror {}/5", view.damage, view.horror),
            false,
        ),
        (
            1,
            Vec3::new(-4.7, 0.05, -1.9),
            format!("Study\n{} clues · Shroud 2", view.study_clues),
            false,
        ),
        (
            2,
            Vec3::new(-2.1, 0.05, -1.9),
            if view.ghoul.alive {
                format!(
                    "Ghoul Minion\nDamage {}/2\n{} / {}\nBounty {}",
                    view.ghoul.damage,
                    if view.ghoul.ready {
                        "Ready"
                    } else {
                        "Exhausted"
                    },
                    if view.ghoul.engaged {
                        "Engaged"
                    } else {
                        "Unengaged"
                    },
                    view.ghoul.bounties
                )
            } else {
                "Ghoul defeated".into()
            },
            !view.ghoul.ready,
        ),
        (
            3,
            Vec3::new(0.5, 0.05, -1.9),
            format!("Bounty Contracts\n{} bounties", view.contracts_bounties),
            false,
        ),
        (
            4,
            Vec3::new(3.0, 0.05, -1.9),
            if view.injury {
                "Internal Injury\nIn Tony's threat area".into()
            } else {
                "Internal Injury discarded".into()
            },
            false,
        ),
    ] {
        if (index != 2 || view.ghoul.alive) && (index != 4 || view.injury) {
            card_mesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                index,
                at,
                exhausted,
            );
        }
        world_label(
            &mut commands,
            &art,
            caption,
            at + Vec3::new(0.0, 0.1, 1.2),
            PAPER,
        );
    }
    for (i, investigator) in view.investigators.iter().enumerate() {
        let code = ["06003", "06005", "60401"][i];
        let x = -3.7 + i as f32 * 2.8;
        let image = art.image(&format!("standee/{code}"));
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(0.85, 1.2))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if image.is_some() { Color::WHITE } else { GOLD },
                base_color_texture: image,
                alpha_mode: AlphaMode::Mask(0.5),
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            Transform::from_xyz(x, 0.65, -4.15)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            Scene,
            Pickable::IGNORE,
        ));
        world_label(
            &mut commands,
            &art,
            format!("{}\nStudy", investigator.name),
            Vec3::new(x, 1.3, -4.1),
            MUTED,
        );
    }
    for (i, card) in view.hand.iter().enumerate() {
        if let Some(index) = CARDS.iter().position(|c| c.code == card.code) {
            let at = Vec3::new(-2.5 + i as f32 * 1.7, 0.05, 2.5);
            card_mesh(
                &mut commands,
                &mut meshes,
                &mut materials,
                &art,
                index,
                at,
                false,
            );
            world_label(
                &mut commands,
                &art,
                card.title.clone(),
                at + Vec3::new(0.0, 0.1, 1.15),
                PAPER,
            );
        }
    }
    deck_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        &art,
        view.deck_count,
    );
    world_label(
        &mut commands,
        &art,
        format!("Tony's deck\n{} cards", view.deck_count),
        Vec3::new(4.8, 0.2, 3.6),
        MUTED,
    );
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.45, 0.7))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.10, 0.16))),
        Transform::from_xyz(4.8, 0.35, 0.0),
        Scene,
        Pickable::IGNORE,
    ));
    world_label(
        &mut commands,
        &art,
        format!("Chaos bag\n{} tokens", view.bag_count),
        Vec3::new(4.8, 0.9, 0.8),
        GOLD,
    );
    if let Some(face) = view.held_token {
        let status = if view
            .test
            .as_ref()
            .is_some_and(|test| test.success.is_some())
        {
            crate::TokenStatus::Resolved
        } else {
            crate::TokenStatus::Pending
        };
        let (disk, label) = crate::tabletop_tokens::spawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            crate::tabletop_tokens::TokenVisual {
                face,
                status,
                ordinal: None,
                image: art.image(&format!("token/{}", token_role(face))),
                transform: Transform::from_xyz(0.5, 0.2, 0.7).with_scale(token_scale),
                label_offset: Vec3::new(0.0, 0.0, 1.05),
            },
        );
        commands.entity(disk).insert((Appearing, ChaosToken, Scene));
        commands.entity(label).insert(Scene);
    }
    harm_scene(
        &mut commands,
        &mut meshes,
        &mut materials,
        &art,
        &view,
        table.harm(),
    );
    *rendered = Some(view);
}

fn rebuild_panel(
    mut commands: Commands,
    mut table: ResMut<Table>,
    art: Res<Art>,
    orbit: Res<Orbit>,
    panels: Query<Entity, With<Panel>>,
    window: Single<&Window>,
    mut last_size: Local<Vec2>,
) {
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
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(20),
                top: px(18),
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                ..default()
            },
            Pickable::IGNORE,
            Panel,
        ))
        .with_children(|p| {
            p.spawn(art.text("ARKHAM / UNFOLDED", 25.0, GOLD));
            p.spawn(art.text("Tony turn · The Gathering · Standard", 18.0, PAPER));
            p.spawn(art.text("Prepared position · Ends after Tony's turn", 14.0, MUTED));
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(20),
                bottom: px(12),
                ..default()
            },
            Pickable::IGNORE,
            Panel,
        ))
        .with_child(art.text(
            "Right drag / arrows: orbit   Wheel / +/-: zoom   Home: reset   F12: capture",
            12.0,
            MUTED,
        ));
    commands.spawn((Node { position_type: PositionType::Absolute, right: px(0), top: px(0), bottom: px(0), width: px(PANEL_WIDTH.min(size.x * 0.48)), padding: UiRect::all(px(18)), flex_direction: FlexDirection::Column, row_gap: px(9), overflow: Overflow::scroll_y(), ..default() }, BackgroundColor(Color::srgb(0.055, 0.085, 0.10)), ScrollPosition::default(), Panel))
        .observe(scroll_panel)
        .with_children(|p| {
            p.spawn(art.text(table.replay.as_ref().map_or_else(|| "LIVE TURN".into(), |r| format!("REPLAY · MOMENT {} / {}", r.index() + 1, r.len())), 13.0, GOLD));
            p.spawn(art.text(format!("{} ordinary actions\nBonus Fight/Engage: {}", view.ordinary_actions, if view.bonus_action { "available" } else { "spent" }), 20.0, PAPER));
            p.spawn(art.text(format!("Resources {} · Clues {}\nDamage {}/{} · Horror {}/{}\nContracts {} bounties · Injury {}", view.resources, view.clues, view.damage, view.health, view.horror, view.sanity, view.contracts_bounties, if view.injury { "present" } else { "removed" }), 15.0, MUTED));
            p.spawn(art.text(&view.prompt, 19.0, PAPER));
            if let Some(test) = &view.test {
                let outcome = test.success.map_or("Pending", |ok| if ok { "Success" } else { "Failure" });
                p.spawn(art.text(format!("{}: skill {} vs {} · {}{}", test.kind.label(), test.skill_value, test.difficulty, outcome, test.modifier.map_or(String::new(), |m| format!(" · modifier {m:+}"))), 16.0, GOLD));
            }
            if let Some(face) = view.held_token { p.spawn(art.text(format!("Revealed {}", face.label()), 21.0, GOLD)); }
            if let Some(hit) = table.harm() {
                p.spawn(art.text(hit.source.label(), 19.0, GOLD));
                p.spawn(art.text(format!("{} receives", hit.recipient.label()), 16.0, PAPER));
                harm_amounts(p, &art, hit);
            }
            if let Some(replay) = &table.replay {
                if replay.index() + 1 < replay.len() { button(&art, p, "Next moment →", Action::Next, true); }
                if replay.index() > 0 { button(&art, p, "← Previous moment", Action::Back, false); }
                button(&art, p, "Return to live turn", Action::Live, false);
                button(&art, p, if orbit.follow { "Camera following: on" } else { "Resume camera following" }, Action::Follow, false);
            } else if table.presenting() {
                button(&art, p, "Finish animation", Action::FinishAnimation, false);
            } else {
                for choice in &view.choices { action_button(&art, p, &view, choice); }
            }
            p.spawn(art.text(&table.notice, 13.0, MUTED));
            button(&art, p, if table.reduced_motion { "Motion: reduced" } else { "Motion: animated" }, Action::Motion, false);
            p.spawn(art.text("HAND", 13.0, GOLD));
            if view.hand.is_empty() { p.spawn(art.text("Empty", 14.0, MUTED)); }
            for card in &view.hand {
                if let Some(index) = CARDS.iter().position(|c| c.code == card.code) { button(&art, p, &format!("Inspect {}", card.title), Action::Inspect(index), false); }
            }
            if !view.committed.is_empty() { p.spawn(art.text(format!("Committed: {}", view.committed.iter().map(|c| c.title.as_str()).collect::<Vec<_>>().join(", ")), 15.0, GOLD)); }
            button(&art, p, "Action history and rules reasons", Action::History, false);
            if table.replay.is_none() {
                button(&art, p, "Save live turn", Action::Save, false);
                button(&art, p, "Reopen saved turn", Action::Reopen, false);
                button(&art, p, "Open recorded replay", Action::Replay, false);
                if view.stage == Stage::Ended && !table.presenting() { button(&art, p, "Start another Tony turn", Action::New, true); }
            }
            p.spawn(art.text("SCENARIO AND CARDS", 13.0, GOLD));
            for (index, card) in CARDS.iter().enumerate().filter(|(i, _)| *i < 5 || *i >= 7) { button(&art, p, card.title, Action::Inspect(index), false); }
            p.spawn(art.text("Only Tony acts. The companions remain at the Study. No next investigator or enemy phase runs. Movement has no destination; the six-clue act threshold cannot be reached this turn.", 14.0, MUTED));
            p.spawn(art.text(&art.status, 13.0, MUTED));
            button(&art, p, "Choose asset folder", Action::AssetFolder, false);
        });
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
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window>,
    mut table: ResMut<Table>,
    time: Res<Time>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    settings: Res<AssetSettings>,
) {
    let modal = table.modal() || settings.is_busy();
    let scene_pointer = !modal
        && window
            .cursor_position()
            .is_some_and(|p| p.x < window.width() - PANEL_WIDTH.min(window.width() * 0.48));
    let mut changed = false;
    if scene_pointer && buttons.pressed(MouseButton::Right) && motion.delta != Vec2::ZERO {
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
        if !modal && keys.pressed(key) {
            orbit.yaw += yaw * time.delta_secs();
            orbit.pitch += pitch * time.delta_secs();
            orbit.distance += zoom * time.delta_secs();
            changed = true;
        }
    }
    if !modal && keys.just_pressed(KeyCode::Home) {
        *orbit = Orbit::default();
        changed = true;
    }
    if changed {
        orbit.follow = false;
        orbit.desired_target = orbit.target;
        table.dirty = true;
    }
    orbit.pitch = orbit.pitch.clamp(0.35, 1.35);
    orbit.distance = orbit.distance.clamp(8.0, 24.0);
    let target = orbit.desired_target;
    orbit.target = if table.reduced_motion {
        target
    } else {
        orbit
            .target
            .lerp(target, (time.delta_secs() * 4.0).min(1.0))
    };
    let direction = Vec3::new(
        orbit.yaw.sin() * orbit.pitch.cos(),
        orbit.pitch.sin(),
        orbit.yaw.cos() * orbit.pitch.cos(),
    );
    **camera = Transform::from_translation(orbit.target + direction * orbit.distance)
        .looking_at(orbit.target, Vec3::Y);
}
fn world_labels(
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut labels: Query<(&WorldLabel, &mut Node, &mut Visibility)>,
) {
    let (camera, transform) = *camera;
    for (label, mut node, mut visibility) in &mut labels {
        if let Ok(point) = camera.world_to_viewport(transform, label.0) {
            node.left = px(point.x - label.1 / 2.0);
            node.top = px(point.y);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}
fn animate(
    time: Res<Time>,
    table: Res<Table>,
    mut transforms: Query<&mut Transform, With<Appearing>>,
) {
    for mut t in &mut transforms {
        t.scale = if table.reduced_motion {
            Vec3::ONE
        } else {
            t.scale.lerp(Vec3::ONE, (time.delta_secs() * 8.0).min(1.0))
        };
    }
}
fn button_colors(
    mut buttons: Query<(&Hovered, Has<Pressed>, &ButtonColor, &mut BackgroundColor), With<Button>>,
) {
    for (hover, pressed, tone, mut background) in &mut buttons {
        background.0 = if pressed {
            GOLD
        } else if hover.0 {
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
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut capture: ResMut<Capture>,
    capturing: Query<(), With<Capturing>>,
    mut exit: MessageWriter<AppExit>,
) {
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
