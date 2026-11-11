//! Gathering's location groups. Card identity and rules stay in the observation.
use super::markers::{self, MarkerKind};
use super::*;
use crate::gathering::PlayObservation;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;

const COLUMN_PITCH: f32 = 2.6;
const ROW_PITCH: f32 = 3.;
const GROUP_GAP: f32 = 0.65;
/// The first group's top sits this far inside the board's far edge.
const BOARD_INSET: f32 = 0.4;
/// The heading token's center below its group's top.
const HEADING_DROP: f32 = 0.32;
/// A ready card's damage token reaches 1.47 above the card center. This keeps
/// that token clear below the heading's token and its single 13px line at the
/// normal Scenario focus (about 0.45 units below center at 800 px tall), and
/// puts the card directly beneath.
const HEADING_TO_ROW: f32 = 2.15;
const LOCATIONS: [&str; 5] = ["01111", "01112", "01113", "01114", "01115"];

/// One exposed sideboard of location groups beside Map. Each board differs by
/// side, palette, marker kind and content; the group layout is shared.
pub(super) struct Sideboard {
    pub name: &'static str,
    pub center: Vec3,
    pub size: Vec2,
    pub surface: Color,
    pub rim: Color,
    pub marker: MarkerKind,
}

pub(super) const ENEMIES: Sideboard = Sideboard {
    name: "Enemies at locations",
    center: Vec3::new(-13.8, 0., layout::SCENARIO_CENTER.z),
    size: Vec2::new(8., layout::SCENARIO_SIZE.y),
    surface: Color::srgb(0.10, 0.17, 0.31),
    rim: Color::srgb(0.39, 0.53, 0.71),
    marker: MarkerKind::EnemyPresence,
};

/// A group's heading row and the positions of its cards, in group order.
pub(super) struct GroupLayout {
    pub heading: Vec3,
    pub cards: Vec<Vec3>,
}

impl Sideboard {
    /// Lay out nonempty groups top to bottom in location order.
    pub(super) fn layout(&self, groups: &[(&str, usize)]) -> Vec<GroupLayout> {
        let mut top = self.center.z - self.size.y / 2. + BOARD_INSET;
        groups
            .iter()
            .map(|&(_, count)| {
                let heading = Vec3::new(
                    self.center.x - self.size.x / 2. + 0.5,
                    0.12,
                    top + HEADING_DROP,
                );
                let first = heading.z + HEADING_TO_ROW;
                let cards = (0..count)
                    .map(|slot| {
                        Vec3::new(
                            self.center.x + (slot % 3) as f32 * COLUMN_PITCH - COLUMN_PITCH,
                            0.08,
                            first + (slot / 3) as f32 * ROW_PITCH,
                        )
                    })
                    .collect();
                top += GROUP_GAP + count.div_ceil(3) as f32 * ROW_PITCH;
                GroupLayout { heading, cards }
            })
            .collect()
    }

    /// The thick table body, inset top and rim shared by every sideboard.
    fn render_surface(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) {
        // Both pieces belong to this observation's scene and leave on rebuild.
        for entity in layout::surface(
            commands,
            meshes,
            materials,
            self.center,
            self.size,
            self.surface,
        ) {
            commands
                .entity(entity)
                .insert((Name::new(self.name), ScenePiece));
        }
        let rim = materials.add(StandardMaterial {
            base_color: self.rim,
            perceptual_roughness: 0.65,
            ..default()
        });
        let size = self.size;
        for (offset, extent) in [
            (
                Vec3::new(0., 0.01, -size.y / 2. + 0.04),
                Vec2::new(size.x, 0.08),
            ),
            (
                Vec3::new(0., 0.01, size.y / 2. - 0.04),
                Vec2::new(size.x, 0.08),
            ),
            (
                Vec3::new(-size.x / 2. + 0.04, 0.01, 0.),
                Vec2::new(0.08, size.y),
            ),
            (
                Vec3::new(size.x / 2. - 0.04, 0.01, 0.),
                Vec2::new(0.08, size.y),
            ),
        ] {
            commands.spawn((
                Name::new(format!("{} rim", self.name)),
                ScenePiece,
                Mesh3d(meshes.add(Cuboid::new(extent.x, 0.02, extent.y))),
                MeshMaterial3d(rim.clone()),
                Transform::from_translation(self.center + offset),
                Pickable::IGNORE,
            ));
        }
    }
}

#[derive(Component)]
pub(super) struct Link {
    room: String,
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

fn resident(play: &PlayObservation, room: &str) -> impl Iterator<Item = u16> {
    play.scenario
        .enemies
        .iter()
        .filter(move |enemy| enemy.engaged.is_none() && enemy.location == room)
        .map(|enemy| enemy.card.id)
}

fn groups(play: &PlayObservation) -> Vec<(&'static str, usize)> {
    LOCATIONS
        .into_iter()
        .filter(|room| !play.scenario.location_in_victory_display(room))
        .map(|room| (room, resident(play, room).count()))
        .filter(|(_, count)| *count > 0)
        .collect()
}

pub(super) fn position(play: &PlayObservation, id: u16) -> Vec3 {
    let groups = groups(play);
    groups
        .iter()
        .zip(ENEMIES.layout(&groups))
        .find_map(|((room, _), group)| {
            let slot = resident(play, room).position(|enemy| enemy == id)?;
            Some(group.cards[slot])
        })
        .unwrap_or(ENEMIES.center)
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
    let board = &ENEMIES;
    board.render_surface(commands, meshes, materials);
    let map = crate::tabletop_gathering_map::GatheringMap::new(
        play.scenario
            .locations
            .iter()
            .filter(|room| !play.scenario.location_in_victory_display(&room.code))
            .map(|room| (room.code.as_str(), room.revealed)),
        Vec2::new(1.35, 1.89),
    )
    .expect("The selected Gathering map is finite");
    let groups = groups(play);
    // Bevy collects only focusables that are not themselves tab groups, so the
    // headings' group is a parent at the viewport origin, not the headings.
    let headings = commands
        .spawn((
            Name::new(format!("{} headings", board.name)),
            ScenePiece,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.),
                top: px(0.),
                ..default()
            },
            TabGroup::new(2),
            Pickable::IGNORE,
        ))
        .id();
    for (&(room, count), group) in groups.iter().zip(board.layout(&groups)) {
        let name = play::location_name(room);
        let marker_at = markers::band_positions(&map, room, &[board.marker])[0].1;
        let selection = controls::Selection {
            source: controls::Source::Location(room.into()),
            at: marker_at,
            title: name.into(),
            inspect_code: room.into(),
        };
        let marker = markers::spawn_token(
            commands,
            meshes,
            materials,
            board.marker,
            room,
            marker_at,
            markers::MAP_RADIUS,
        );
        commands.entity(marker).insert(Link { room: room.into() });
        scene::bind_source(commands, context, marker, selection.clone());
        let pill = markers::spawn_pill(
            commands,
            context,
            format!("{name} enemies: {count}"),
            count,
            markers::pill_anchor(marker_at),
        );
        commands.entity(pill).insert(PersistentLabel);
        let token = markers::spawn_token(
            commands,
            meshes,
            materials,
            board.marker,
            room,
            group.heading,
            markers::HEADING_RADIUS,
        );
        commands.entity(token).insert(Pickable::IGNORE);
        let header = scene::label(
            commands,
            context,
            format!("{} · {count}", name.to_uppercase()),
            group.heading + Vec3::X * (markers::HEADING_RADIUS + 0.18),
            false,
        );
        commands
            .entity(header)
            .remove::<(Pickable, ScenePiece)>()
            .insert((
                PersistentLabel,
                markers::HeadingLabel,
                Link { room: room.into() },
                markers::Tint {
                    room: room.into(),
                    color: markers::HEADING_TEXT,
                },
                TextColor(markers::HEADING_TEXT),
                bevy::text::LineHeight::Px(13.),
                TextLayout::justify(Justify::Left).with_no_wrap(),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Button,
                Hovered::default(),
                TabIndex(0),
                feedback::context_button(),
            ));
        commands.entity(headings).add_child(header);
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
        &markers::Tint,
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
    for (tint, material, text) in &mut visuals {
        let selected = active.as_deref() == Some(tint.room.as_str());
        let color = if selected { Color::WHITE } else { tint.color };
        if let Some(text) = text {
            text.into_inner().0 = color;
        }
        if let Some(mut material) = material.and_then(|handle| materials.get_mut(&handle.0)) {
            material.base_color = color;
        }
    }
}
