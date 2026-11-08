//! Gathering's location groups. Card identity and rules stay in the observation.
use super::*;
use crate::gathering::PlayObservation;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;

const CENTER: Vec3 = Vec3::new(-13.8, 0., layout::SCENARIO_CENTER.z);
const SIZE: Vec2 = Vec2::new(8., layout::SCENARIO_SIZE.y);
const COLUMN_PITCH: f32 = 2.6;
const ROW_PITCH: f32 = 3.;
const GROUP_GAP: f32 = 0.65;
const LOCATIONS: [&str; 5] = ["01111", "01112", "01113", "01114", "01115"];

#[derive(Component)]
pub(super) struct Link {
    room: String,
    color: Color,
}

#[derive(Component)]
pub(super) struct PersistentLabel;

#[derive(Component)]
pub(super) struct Caption;

pub(super) fn caption_width(camera: &Camera, transform: &GlobalTransform, at: Vec3) -> Option<f32> {
    let left = camera
        .world_to_viewport(transform, at - Vec3::X * 1.2)
        .ok()?;
    let right = camera
        .world_to_viewport(transform, at + Vec3::X * 1.2)
        .ok()?;
    Some(left.distance(right).clamp(1., 190.))
}

fn identity(room: &str) -> (&'static str, Color) {
    match room {
        "01111" => ("S", Color::srgb(0.92, 0.72, 0.24)),
        "01112" => ("H", Color::srgb(0.28, 0.78, 0.69)),
        "01113" => ("A", Color::srgb(0.73, 0.52, 0.92)),
        "01114" => ("C", Color::srgb(0.40, 0.67, 0.96)),
        "01115" => ("P", Color::srgb(0.96, 0.48, 0.40)),
        _ => ("?", Color::WHITE),
    }
}

fn groups(play: &PlayObservation) -> impl Iterator<Item = (&'static str, usize)> + '_ {
    LOCATIONS.into_iter().filter_map(|room| {
        let count = play
            .scenario
            .enemies
            .iter()
            .filter(|enemy| {
                enemy.engaged.is_none()
                    && enemy.location == room
                    && !play.scenario.location_in_victory_display(room)
            })
            .count();
        (count > 0).then_some((room, count))
    })
}

pub(super) fn position(play: &PlayObservation, id: u16) -> Vec3 {
    let mut top = CENTER.z - SIZE.y / 2. + 0.4;
    for (room, count) in groups(play) {
        if let Some(slot) = play
            .scenario
            .enemies
            .iter()
            .filter(|enemy| enemy.engaged.is_none() && enemy.location == room)
            .position(|enemy| enemy.card.id == id)
        {
            return Vec3::new(
                CENTER.x + (slot % 3) as f32 * COLUMN_PITCH - COLUMN_PITCH,
                0.08,
                top + 2.75 + (slot / 3) as f32 * ROW_PITCH,
            );
        }
        top += GROUP_GAP + count.div_ceil(3) as f32 * ROW_PITCH;
    }
    CENTER
}

pub(super) fn focus(area: Area) -> (Vec3, f32) {
    let (mut at, distance) = layout::focus(area);
    if matches!(area, Area::Scenario | Area::Overview) {
        at.x -= 4.4;
    }
    (at, distance)
}

pub(super) fn render(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
) {
    let Some(play) = &context.view.play else {
        return;
    };
    // The dynamic sideboard uses the ordinary table body and inset surface.
    // Both pieces belong to this observation's scene and leave on rebuild.
    for entity in layout::surface(
        commands,
        meshes,
        materials,
        CENTER,
        SIZE,
        Color::srgb(0.10, 0.17, 0.31),
    ) {
        commands
            .entity(entity)
            .insert((Name::new("Enemies at locations"), ScenePiece));
    }
    let rim = materials.add(StandardMaterial {
        base_color: Color::srgb(0.39, 0.53, 0.71),
        perceptual_roughness: 0.65,
        ..default()
    });
    for (offset, size) in [
        (
            Vec3::new(0., 0.01, -SIZE.y / 2. + 0.04),
            Vec2::new(SIZE.x, 0.08),
        ),
        (
            Vec3::new(0., 0.01, SIZE.y / 2. - 0.04),
            Vec2::new(SIZE.x, 0.08),
        ),
        (
            Vec3::new(-SIZE.x / 2. + 0.04, 0.01, 0.),
            Vec2::new(0.08, SIZE.y),
        ),
        (
            Vec3::new(SIZE.x / 2. - 0.04, 0.01, 0.),
            Vec2::new(0.08, SIZE.y),
        ),
    ] {
        commands.spawn((
            Name::new("Enemy board rim"),
            ScenePiece,
            Mesh3d(meshes.add(Cuboid::new(size.x, 0.02, size.y))),
            MeshMaterial3d(rim.clone()),
            Transform::from_translation(CENTER + offset),
            Pickable::IGNORE,
        ));
    }
    let map = crate::tabletop_gathering_map::GatheringMap::new(
        play.scenario
            .locations
            .iter()
            .map(|room| (room.code.as_str(), room.revealed)),
        Vec2::new(1.35, 1.89),
    )
    .expect("The selected Gathering map is finite");
    let mut top = CENTER.z - SIZE.y / 2. + 0.4;
    for (room, count) in groups(play) {
        let (id, color) = identity(room);
        let marker_at = map.position(room) + Vec3::new(-0.32, 0.15, -1.30);
        let header_at = Vec3::new(CENTER.x, 0.15, top + 0.05);
        let selection = controls::Selection {
            source: controls::Source::Location(room.into()),
            at: marker_at,
            title: play::location_name(room).into(),
            inspect_code: room.into(),
        };
        let marker = commands
            .spawn((
                Name::new(format!("Enemy presence {id}")),
                ScenePiece,
                Link {
                    room: room.into(),
                    color,
                },
                Mesh3d(meshes.add(Cylinder::new(0.23, 0.10))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color,
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(marker_at),
            ))
            .with_children(|root| {
                // Three pale claw strokes distinguish enemy presence from map connections.
                for x in [-0.075, 0., 0.075] {
                    root.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.025, 0.012, 0.24))),
                        MeshMaterial3d(materials.add(Color::srgb(0.95, 0.93, 0.85))),
                        Transform::from_xyz(x, 0.056, 0.)
                            .with_rotation(Quat::from_rotation_y(-0.25)),
                        Pickable::IGNORE,
                    ));
                }
            })
            .id();
        scene::bind_source(commands, context, marker, selection.clone());
        let marker_label = scene::label(
            commands,
            context,
            format!("{id} · {count}"),
            marker_at + Vec3::new(0.65, 0.08, -0.1),
            false,
        );
        commands.entity(marker_label).insert((
            PersistentLabel,
            Link {
                room: room.into(),
                color,
            },
            TextColor(color),
            Node {
                position_type: PositionType::Absolute,
                width: px(55),
                ..default()
            },
        ));
        let header = scene::label(
            commands,
            context,
            format!(
                "{id} · {} · {count}",
                play::location_name(room).to_uppercase()
            ),
            header_at,
            false,
        );
        commands.entity(header).remove::<Pickable>().insert((
            PersistentLabel,
            Link {
                room: room.into(),
                color,
            },
            TextColor(color),
            Button,
            Hovered::default(),
            TabIndex(0),
            TabGroup::new(2),
            feedback::context_button(),
        ));
        context.bind(commands, header, selection.source.clone());
        let epoch = context.epoch;
        let revision = context.view.revision;
        commands
            .entity(header)
            .observe(move |_: On<Activate>, mut actions: ResMut<Actions>| {
                actions
                    .0
                    .push(Action::Select(epoch, revision, selection.clone()));
            });
        top += GROUP_GAP + count.div_ceil(3) as f32 * ROW_PITCH;
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn highlight(
    table: Res<Table>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    focus: Option<Res<InputFocus>>,
    targets: Query<&ContextTarget>,
    links: Query<&Link>,
    mut visuals: Query<(
        &Link,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&mut TextColor>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let view = table.view();
    let hovered = hover
        .as_ref()
        .and_then(|hover| hover.get(&bevy::picking::pointer::PointerId::Mouse));
    let focused = focus.as_ref().and_then(|focus| focus.get());
    let active = (!table.modal() && !table.menu_open)
        .then(|| {
            hovered
                .into_iter()
                .flat_map(|hits| hits.keys().copied())
                .chain(focused)
                .find_map(|entity| {
                    if let Ok(link) = links.get(entity) {
                        return Some(link.room.clone());
                    }
                    match &targets.get(entity).ok()?.source {
                        controls::Source::Location(room) => Some(room.clone()),
                        controls::Source::Card(id) => view
                            .play
                            .as_ref()?
                            .scenario
                            .enemies
                            .iter()
                            .find(|enemy| enemy.card.id == *id && enemy.engaged.is_none())
                            .map(|enemy| enemy.location.clone()),
                        _ => None,
                    }
                })
        })
        .flatten();
    for (link, material, text) in &mut visuals {
        let selected = active.as_deref() == Some(link.room.as_str());
        let color = if selected { Color::WHITE } else { link.color };
        if let Some(text) = text {
            text.into_inner().0 = color;
        }
        if let Some(mut material) = material.and_then(|handle| materials.get_mut(&handle.0)) {
            material.base_color = color;
        }
    }
}
