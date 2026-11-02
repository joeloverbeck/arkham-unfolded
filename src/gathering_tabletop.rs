//! Native consumer of the Gathering decision seam. Rules remain in gathering::Session.

use crate::{
    gathering::{
        Investigator, Observation, Session,
        continuity::{self, Replay},
    },
    tabletop_camera::{CameraMode, Capture, FrameInput, Orbit},
    tabletop_feedback::{self as feedback, CardFeedback, FeedbackSettings},
    tabletop_health::{HealthAssets, RenderHealth, TabletopHealthPlugin, exit_result},
    tabletop_layout::{self as layout, Area},
    tabletop_pacing::{PresentationTime, Redraw, RequestRedraw, TabletopPacingPlugin},
};
use bevy::{
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    input_focus::{InputFocus, tab_navigation::TabIndex},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
};
use std::path::PathBuf;

const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const MUTED: Color = Color::srgb(0.65, 0.74, 0.73);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);

#[derive(Clone)]
enum Action {
    Choose(u64, u64, String),
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
}

impl Table {
    fn new(session: Session, save_path: PathBuf) -> Self {
        Self {
            session,
            replay: None,
            save_path,
            notice: "Readable card substitutes. Each decision saves before it takes effect.".into(),
            inspection: None,
            history_open: false,
            epoch: 0,
            dirty: true,
        }
    }

    fn view(&self) -> Observation {
        self.replay.as_ref().map_or_else(
            || self.session.observe_for_play(),
            |replay| replay.current().clone(),
        )
    }

    fn modal(&self) -> bool {
        self.inspection.is_some() || self.history_open
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

pub fn run() -> Result<(), String> {
    let mut save = PathBuf::from("local/gathering/setup.session.json");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--experiment" => {
                if args.next().as_deref() != Some("gathering") {
                    return Err("Use --experiment gathering.".into());
                }
            }
            "--save" => save = args.next().ok_or("--save needs a path")?.into(),
            "--no-art" => {}
            "--help" | "-h" => {
                println!(
                    "Gathering setup\n--experiment gathering\n--save PATH\n--no-art\nReopens an existing setup; a new path starts a separate setup. Scenario play is not yet available."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown Gathering option: {arg}")),
        }
    }
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
    let mut app = App::new();
    app.insert_resource(ClearColor(INK))
        .insert_resource(Table::new(session, save))
        .insert_resource(Orbit::new(CameraMode::Jacqueline))
        .insert_resource(HealthAssets::new([]))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Arkham Unfolded — Gathering Setup".into(),
                resolution: (1600, 1000).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .add_plugins(TabletopPacingPlugin)
        .add_plugins(TabletopHealthPlugin);
    install(&mut app);
    exit_result(app.run())
}

/// The same registration is used by the native app and headless input tests.
fn install(app: &mut App) {
    app.init_resource::<Actions>()
        .add_plugins(feedback::FeedbackPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (handle_actions, camera_controls, render, world_labels).chain(),
        )
        .add_systems(Last, request_redraw.in_set(RequestRedraw));
}

fn handle_actions(
    health: Res<RenderHealth>,
    mut actions: ResMut<Actions>,
    mut table: ResMut<Table>,
    mut orbit: ResMut<Orbit>,
    keys: Res<ButtonInput<KeyCode>>,
    mut feedback: ResMut<FeedbackSettings>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        actions.0.push(Action::Close);
    }
    for action in actions.0.drain(..) {
        if (!health.allows_play() || health.just_became_ready())
            && !matches!(action, Action::Focus(_) | Action::Close)
        {
            continue;
        }
        match action {
            Action::Choose(epoch, revision, id)
                if epoch == table.epoch && table.replay.is_none() && !table.modal() =>
            {
                let path = table.save_path.clone();
                table.notice =
                    match continuity::commit_choice(&path, &mut table.session, revision, &id) {
                        Ok(()) => "Decision saved.".into(),
                        Err(error) => format!("Decision not committed: {error}"),
                    };
                table.dirty = true;
            }
            Action::Choose(_, _, _) => {}
            Action::Focus(area) => orbit.focus(layout::focus(area)),
            Action::Close => {
                table.epoch += 1;
                table.inspection = None;
                table.history_open = false;
                table.dirty = true;
            }
            Action::Inspect(title, code) => {
                table.epoch += 1;
                table.inspection = Some((title, code));
                table.history_open = false;
                table.dirty = true;
            }
            Action::History => {
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
                        table.epoch += 1;
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
                        table.epoch += 1;
                        table.replay = Some(replay);
                        table.inspection = None;
                        table.history_open = false;
                    }
                    Err(error) => table.notice = format!("Replay unavailable: {error}"),
                }
                table.dirty = true;
            }
            Action::Next => {
                if let Some(replay) = &mut table.replay {
                    replay.next();
                    table.dirty = true;
                }
            }
            Action::Back => {
                if let Some(replay) = &mut table.replay {
                    replay.back();
                    table.dirty = true;
                }
            }
            Action::Live => {
                table.epoch += 1;
                table.replay = None;
                table.inspection = None;
                table.history_open = false;
                table.dirty = true;
            }
            Action::Reopen | Action::Save => {}
        }
    }
    feedback.blocked = table.modal();
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
    (
        Text::new(value),
        TextFont {
            font_size: bevy::text::FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn button(parent: &mut ChildSpawnerCommands, label: &str, action: Action) {
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
        });
}

#[allow(clippy::too_many_arguments)]
fn card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
    title: &str,
    code: &str,
    selected: bool,
    color: Color,
) -> Entity {
    let action = Action::Inspect(title.into(), code.into());
    let (width, length) = if matches!(code, "06003" | "06005" | "60401") {
        (2.6, 1.85)
    } else {
        (1.35, 1.89)
    };
    let entity = commands
        .spawn((
            ScenePiece,
            Mesh3d(meshes.add(Cuboid::new(width, 0.08, length))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(at),
        ))
        .observe(move |event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(action.clone());
            }
        })
        .id();
    feedback::attach_card_feedback(commands, meshes, materials, entity, width, length, 0.04);
    commands.entity(entity).insert(CardFeedback {
        pending_choice: selected,
        ..default()
    });
    commands.spawn((
        ScenePiece,
        WorldLabel(at + Vec3::Y * 0.1, false),
        text(title, 13., INK),
        Node {
            position_type: PositionType::Absolute,
            width: px(130),
            padding: UiRect::all(px(3)),
            ..default()
        },
        BackgroundColor(PAPER),
        Pickable::IGNORE,
    ));
    entity
}

fn inspection_text(view: &Observation, code: &str) -> Option<String> {
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

fn render(
    mut commands: Commands,
    mut table: ResMut<Table>,
    health: Res<RenderHealth>,
    old: Query<Entity, RenderedContent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !table.dirty && !health.is_changed() {
        return;
    }
    table.dirty = false;
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let view = table.view();
    for (index, investigator) in view.investigators.iter().enumerate() {
        for (slot, hand) in investigator.hand.iter().enumerate() {
            card(
                &mut commands,
                &mut meshes,
                &mut materials,
                layout::hand_position(index, investigator.hand.len(), slot),
                &hand.title,
                &hand.code,
                investigator.selected_mulligan.contains(&hand.id),
                PAPER,
            );
        }
        for (slot, permanent) in investigator.permanents.iter().enumerate() {
            card(
                &mut commands,
                &mut meshes,
                &mut materials,
                layout::investigator_asset(index, slot),
                &permanent.title,
                &permanent.code,
                false,
                PAPER,
            );
        }
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
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
        commands.spawn((
            ScenePiece,
            WorldLabel(layout::investigator_heading(index), true),
            text(
                format!(
                    "{}\nResources {} · Deck {} · Damage {} · Horror {}",
                    investigator.investigator.name(),
                    investigator.resources,
                    investigator.deck_count,
                    investigator.damage,
                    investigator.horror
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
    if let Some(scenario) = &view.scenario {
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
                    pose.translation,
                    label,
                    code,
                    false,
                    PAPER,
                );
                commands.entity(entity).insert(pose);
            }
        }
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            layout::SCENARIO_REFERENCE,
            "The Gathering · Standard",
            &scenario.reference,
            false,
            PAPER,
        );
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            layout::SCENARIO_MAP_CENTER,
            &format!("Study · {} clues", scenario.study_clues),
            "01111",
            false,
            PAPER,
        );
        card(
            &mut commands,
            &mut meshes,
            &mut materials,
            layout::ENCOUNTER_DECK,
            &format!("Encounter deck · {} cards", scenario.encounter_deck_count),
            "encounter-deck",
            false,
            MUTED,
        );
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
        ))
        .with_children(|p| {
            p.spawn(text("The Gathering · Standard · Setup", 22., GOLD));
            for (label, area) in [
                ("Overview", Area::Overview),
                ("Scenario", Area::Scenario),
                ("Tony", Area::Investigator(0)),
                ("Patrice", Area::Investigator(1)),
                ("Jacqueline", Area::Investigator(2)),
            ] {
                button(p, label, Action::Focus(area));
            }
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
        ))
        .observe(scroll_panel)
        .with_children(|p| {
            if !health.allows_play() {
                p.spawn(text(health.status(), 20., GOLD));
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
                    button(row, "Return to live setup", Action::Live);
                } else if !table.modal() {
                    for choice in &view.choices {
                        button(
                            row,
                            &choice.label,
                            Action::Choose(table.epoch, view.revision, choice.id.clone()),
                        );
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
                    left: percent(22),
                    right: percent(22),
                    top: percent(14),
                    bottom: percent(38),
                    padding: UiRect::all(px(20)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(INK),
            ))
            .observe(scroll_panel)
            .with_children(|p| {
                if let Some((title, code)) = &table.inspection {
                    p.spawn(text(title, 24., GOLD));
                    p.spawn(text(format!("Card {code}"), 14., MUTED));
                    if let Some(rules) = inspection_text(&view, code) {
                        p.spawn(text(rules, 17., PAPER));
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

#[allow(clippy::too_many_arguments)]
fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window>,
    table: Res<Table>,
    time: Res<PresentationTime>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
    focus: Option<Res<InputFocus>>,
    focusable: Query<(), With<Button>>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    ui_nodes: Query<(), With<Node>>,
) {
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
            pointer_blocked: table.modal(),
            over_ui,
            keyboard: table.modal()
                || focus
                    .as_ref()
                    .and_then(|f| f.get())
                    .is_some_and(|entity| focusable.contains(entity)),
            wheel: table.modal() || over_ui,
        },
        reduced_motion: false,
    });
    **camera = step.transform;
}

fn world_labels(
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    orbit: Res<Orbit>,
    mut labels: Query<(&WorldLabel, &mut Node, &mut Visibility)>,
) {
    for (label, mut node, mut visibility) in &mut labels {
        if orbit.pose().distance > layout::DETAIL_LABEL_DISTANCE && !label.1 {
            *visibility = Visibility::Hidden;
        } else if let Ok(point) = camera.0.world_to_viewport(camera.1, label.0) {
            let width = if let Val::Px(width) = node.width {
                width
            } else {
                130.
            };
            node.left = px(point.x - width / 2.);
            node.top = px(point.y);
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn request_redraw(orbit: Res<Orbit>, health: Res<RenderHealth>, mut redraw: ResMut<Redraw>) {
    if orbit.is_moving() || health.is_checking() {
        redraw.request();
    }
}

#[cfg(test)]
mod tests;
