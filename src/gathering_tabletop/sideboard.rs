//! Gathering's exposed sideboards beside Map. Each groups its cards by location
//! beneath a heading and links the group to its location with a marker in the
//! location's band. Card identity and rules stay in the observation; the boards
//! differ only by side, palette, marker kind and content.
use super::markers::{self, MarkerKind};
use super::*;
use crate::gathering::PlayObservation;
use crate::tabletop_gathering_map::GatheringMap;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;

/// Columns spread as far as the board allows a 2.4-wide card cell, so the
/// captions "Exhausted" of neighbors stay apart at 1600x1000 Scenario.
const COLUMN_PITCH: f32 = 2.75;
const ROW_PITCH: f32 = 3.;
const GROUP_GAP: f32 = 0.65;
/// Below a group's last row, room for its captions: three 13px lines (a
/// two-line name and "Exhausted") at the normal Scenario focus at 1280x800,
/// so the next group's heading and damage counts start below them.
const CAPTION_ROOM: f32 = 3.2;
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

/// One exposed sideboard of location groups beside Map.
pub(super) struct Sideboard {
    pub name: &'static str,
    pub center: Vec3,
    pub size: Vec2,
    pub surface: Color,
    pub rim: Color,
    pub marker: MarkerKind,
    /// A location's group, in stable display order.
    content: fn(&PlayObservation, &str) -> Vec<u16>,
    /// What a group count counts, in accessible names and seat pieces.
    pub members: &'static str,
    /// The seat's piece name for this board's headings.
    pub heading_piece: &'static str,
    /// Accessible context before a heading, which its token alone conveys.
    heading_context: Option<&'static str>,
}

/// Location-resident enemies, left of Map.
pub(super) const ENEMIES: Sideboard = Sideboard {
    name: "Enemies at locations",
    center: Vec3::new(-13.8, 0., layout::SCENARIO_CENTER.z),
    size: Vec2::new(8., layout::SCENARIO_SIZE.y),
    surface: Color::srgb(0.10, 0.17, 0.31),
    rim: Color::srgb(0.39, 0.53, 0.71),
    marker: MarkerKind::EnemyPresence,
    content: resident,
    members: "enemies",
    heading_piece: "enemy-sideboard",
    heading_context: None,
};

/// Cards attached to locations, mirroring the enemy sideboard right of Map.
pub(super) const ATTACHMENTS: Sideboard = Sideboard {
    name: "Attachments at locations",
    center: Vec3::new(-ENEMIES.center.x, 0., ENEMIES.center.z),
    size: ENEMIES.size,
    surface: Color::srgb(0x2B as f32 / 255., 0x33 as f32 / 255., 0x18 as f32 / 255.),
    rim: Color::srgb(0x8E as f32 / 255., 0x9A as f32 / 255., 0x4A as f32 / 255.),
    marker: MarkerKind::Attachment,
    content: attached,
    members: "attachments",
    heading_piece: "attachment-sideboard",
    heading_context: Some("Attached to"),
};

/// Both boards, in heading Tab order.
pub(super) const BOARDS: [&Sideboard; 2] = [&ENEMIES, &ATTACHMENTS];

fn resident(play: &PlayObservation, room: &str) -> Vec<u16> {
    play.scenario
        .enemies
        .iter()
        .filter(|enemy| enemy.engaged.is_none() && enemy.location == room)
        .map(|enemy| enemy.card.id)
        .collect()
}

fn attached(play: &PlayObservation, room: &str) -> Vec<u16> {
    play.scenario
        .locations
        .iter()
        .filter(|location| location.code == room)
        .filter_map(|location| location.fog.as_ref())
        .map(|card| card.id)
        .collect()
}

/// A group's heading row and the positions of its cards, in group order.
pub(super) struct GroupLayout {
    pub heading: Vec3,
    pub cards: Vec<Vec3>,
}

impl Sideboard {
    /// Nonempty groups in location order. A location in the Victory display
    /// has left the map, so nothing links to it.
    fn groups(&self, play: &PlayObservation) -> Vec<(&'static str, Vec<u16>)> {
        LOCATIONS
            .into_iter()
            .filter(|room| !play.scenario.location_in_victory_display(room))
            .map(|room| (room, (self.content)(play, room)))
            .filter(|(_, cards)| !cards.is_empty())
            .collect()
    }

    /// Lay out nonempty groups of the given sizes top to bottom.
    pub(super) fn layout(&self, counts: &[usize]) -> Vec<GroupLayout> {
        let mut top = self.center.z - self.size.y / 2. + BOARD_INSET;
        counts
            .iter()
            .map(|&count| {
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
                top += GROUP_GAP + count.div_ceil(3) as f32 * ROW_PITCH + CAPTION_ROOM;
                GroupLayout { heading, cards }
            })
            .collect()
    }

    /// Where a card of this board lies; the board's center when it has none.
    pub(super) fn position(&self, play: &PlayObservation, id: u16) -> Vec3 {
        let groups = self.groups(play);
        let counts: Vec<_> = groups.iter().map(|(_, cards)| cards.len()).collect();
        groups
            .iter()
            .zip(self.layout(&counts))
            .find_map(|((_, cards), group)| {
                let slot = cards.iter().position(|card| *card == id)?;
                Some(group.cards[slot])
            })
            .unwrap_or(self.center)
    }

    /// The location whose group on this board shows the card.
    fn room_of(&self, play: &PlayObservation, id: u16) -> Option<&'static str> {
        self.groups(play)
            .into_iter()
            .find_map(|(room, cards)| cards.contains(&id).then_some(room))
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

    /// Each group's map marker with its count, and its heading: the same token
    /// followed by "LOCATION NAME · count". Markers and headings open the
    /// location's group menu; each card keeps its own.
    #[allow(clippy::too_many_arguments)]
    fn render_groups(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        context: &CardRender<'_>,
        play: &PlayObservation,
        map: &GatheringMap,
        headings: Entity,
    ) {
        let groups = self.groups(play);
        let counts: Vec<_> = groups.iter().map(|(_, cards)| cards.len()).collect();
        for ((room, cards), group) in groups.iter().zip(self.layout(&counts)) {
            let count = cards.len();
            let name = play::location_name(room);
            let marker_at = marker_at(play, map, room, self.marker);
            let selection = controls::Selection {
                source: controls::Source::Location((*room).into()),
                at: marker_at,
                title: name.into(),
                inspect_code: (*room).into(),
            };
            let link = Link {
                room: (*room).into(),
                kind: self.marker,
            };
            let marker = markers::spawn_token(
                commands,
                meshes,
                materials,
                self.marker,
                room,
                marker_at,
                markers::MAP_RADIUS,
            );
            commands.entity(marker).insert(link.clone());
            scene::bind_source(commands, context, marker, selection.clone());
            let pill = markers::spawn_pill(
                commands,
                context,
                format!("{name} {}: {count}", self.members),
                count,
                self.marker,
                marker_at,
            );
            commands.entity(pill).insert(PersistentLabel);
            let token = markers::spawn_token(
                commands,
                meshes,
                materials,
                self.marker,
                room,
                group.heading,
                markers::HEADING_RADIUS,
            );
            commands.entity(token).insert(Pickable::IGNORE);
            let text = format!("{} · {count}", name.to_uppercase());
            let accessible = match self.heading_context {
                Some(context) => format!("{context} {text}"),
                None => text.clone(),
            };
            let header = scene::label(
                commands,
                context,
                text,
                group.heading + Vec3::X * (markers::HEADING_RADIUS + 0.18),
                false,
            );
            commands
                .entity(header)
                .remove::<(Pickable, ScenePiece)>()
                .insert((
                    PersistentLabel,
                    markers::HeadingLabel,
                    link,
                    markers::Tint {
                        room: (*room).into(),
                        kind: self.marker,
                        color: markers::HEADING_TEXT,
                    },
                    TextColor(markers::HEADING_TEXT),
                    AccessibleLabel::new(accessible),
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
            commands.entity(header).observe(
                move |_: On<Activate>, mut actions: ResMut<Actions>| {
                    actions
                        .0
                        .push(Action::Select(epoch, revision, selection.clone()));
                },
            );
        }
    }
}

/// Every marker kind with a nonempty group at this location, in band order.
fn kinds_at(play: &PlayObservation, room: &str) -> Vec<MarkerKind> {
    BOARDS
        .into_iter()
        .filter(|board| {
            !play.scenario.location_in_victory_display(room)
                && !(board.content)(play, room).is_empty()
        })
        .map(|board| board.marker)
        .collect()
}

/// Where a location's marker of this kind stands, beside whichever other
/// markers the location shows in the same observation.
pub(super) fn marker_at(
    play: &PlayObservation,
    map: &GatheringMap,
    room: &str,
    kind: MarkerKind,
) -> Vec3 {
    let mut kinds = kinds_at(play, room);
    if !kinds.contains(&kind) {
        kinds.push(kind);
    }
    markers::band_positions(map, room, &kinds)
        .into_iter()
        .find_map(|(shown, at)| (shown == kind).then_some(at))
        .expect("the requested marker is placed")
}

/// Links a marker or heading to its location group on one board.
#[derive(Component, Clone)]
pub(super) struct Link {
    pub room: String,
    pub kind: MarkerKind,
}

impl Link {
    pub(super) fn board(&self) -> &'static Sideboard {
        BOARDS
            .into_iter()
            .find(|board| board.marker == self.kind)
            .expect("every marker kind has its board")
    }
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

/// A sideboard card's labels stay shown in Overview and carry the card's full
/// accessible title; on a board, its caption takes the card's projected width.
pub(super) fn label_card(commands: &mut Commands, card: Entity, title: String, on_board: bool) {
    commands.queue(move |world: &mut World| {
        let labels: Vec<_> = world
            .query::<(Entity, &motion::CardLabel)>()
            .iter(world)
            .filter(|(_, label)| label.0 == card)
            .map(|(entity, _)| entity)
            .collect();
        for label in labels {
            world
                .entity_mut(label)
                .insert((PersistentLabel, AccessibleLabel::new(title.clone())));
            if on_board {
                world.entity_mut(label).insert(Caption);
            }
        }
    });
}

/// Scenario and Overview frame Map between both sideboards, at the distance
/// each view already uses.
pub(super) fn focus(area: Area) -> (Vec3, f32) {
    let (mut at, distance) = layout::focus(area);
    if matches!(area, Area::Scenario | Area::Overview) {
        at.x = (ENEMIES.center.x + ATTACHMENTS.center.x) / 2.;
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
    let map = GatheringMap::new(
        play.scenario
            .locations
            .iter()
            .filter(|room| !play.scenario.location_in_victory_display(&room.code))
            .map(|room| (room.code.as_str(), room.revealed)),
        Vec2::new(1.35, 1.89),
    )
    .expect("The selected Gathering map is finite");
    // Bevy collects only focusables that are not themselves tab groups, so the
    // headings' group is a parent at the viewport origin, not the headings.
    let headings = commands
        .spawn((
            Name::new("Sideboard headings"),
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
    for board in BOARDS {
        board.render_surface(commands, meshes, materials);
        board.render_groups(commands, meshes, materials, context, play, &map, headings);
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
    // A marker, heading or card lights its own group; the location lights each of its groups.
    let active = (!table.modal() && !table.menu_open)
        .then(|| {
            hovered
                .into_iter()
                .flat_map(|hits| hits.keys().copied())
                .chain(focused)
                .find_map(|entity| {
                    if let Ok(link) = links.get(entity) {
                        return Some((link.room.clone(), Some(link.kind)));
                    }
                    match &targets.get(entity).ok()?.source {
                        controls::Source::Location(room) => Some((room.clone(), None)),
                        controls::Source::Card(id) => {
                            let play = view.play.as_ref()?;
                            BOARDS.into_iter().find_map(|board| {
                                board
                                    .room_of(play, *id)
                                    .map(|room| (room.to_owned(), Some(board.marker)))
                            })
                        }
                        _ => None,
                    }
                })
        })
        .flatten();
    for (tint, material, text) in &mut visuals {
        let selected = active.as_ref().is_some_and(|(room, kind)| {
            *room == tint.room && kind.is_none_or(|kind| kind == tint.kind)
        });
        let color = if selected { Color::WHITE } else { tint.color };
        if let Some(text) = text {
            text.into_inner().0 = color;
        }
        if let Some(mut material) = material.and_then(|handle| materials.get_mut(&handle.0)) {
            material.base_color = color;
        }
    }
}
