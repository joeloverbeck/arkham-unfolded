//! Location colors and the in-engine marker tokens that link a map location to
//! its sideboard group. Tokens are table geometry; their counts are counter-style
//! labels, so no artwork or prepared asset is involved.
use super::*;
use crate::tabletop_gathering_map::GatheringMap;
use bevy::{asset::RenderAssetUsages, render::render_resource::PrimitiveTopology};

/// Okabe–Ito colors in assignment order. They avoid the availability teal, the
/// gold selection, the menu-source blue, brass links, clue green and class colors.
const PALETTE: [Color; 4] = [
    Color::srgb(0xD5 as f32 / 255., 0x5E as f32 / 255., 0.),
    Color::srgb(0xF0 as f32 / 255., 0xE4 as f32 / 255., 0x42 as f32 / 255.),
    Color::srgb(0., 0x72 as f32 / 255., 0xB2 as f32 / 255.),
    Color::srgb(0xCC as f32 / 255., 0x79 as f32 / 255., 0xA7 as f32 / 255.),
];
const OUTLINE: Color = Color::srgb(0.05, 0.045, 0.04);
const IVORY: Color = Color::srgb(0.94, 0.91, 0.82);
const INK: Color = Color::srgb(0.07, 0.06, 0.06);
const PILL: Color = Color::srgba(0.015, 0.02, 0.02, 0.96);
/// Heading words stay ordinary readable text; the token carries the color.
pub(super) const HEADING_TEXT: Color = Color::srgb(0.82, 0.88, 0.95);

/// Map token radius; sideboard headings use a larger copy beside 13px text.
pub(super) const MAP_RADIUS: f32 = 0.22;
pub(super) const HEADING_RADIUS: f32 = 0.30;
/// Centers of a two-marker group without a link between them.
const GROUP_PITCH: f32 = 0.72;
/// A marker beside a top-entering link clears its arrowhead by this margin.
const LINK_CLEARANCE: f32 = 0.06;
const TOKEN_HEIGHT: f32 = 0.10;
/// Map tokens hang just inside the band's top, leaving the rest for the count.
const BAND_TOP_MARGIN: f32 = 0.03;
/// The count keeps the counter-label size used across the table at every zoom.
const COUNT_SIZE: f32 = 11.;

/// Location identity decides the color. Study and Hallway share one because
/// they are never in play together; every other Gathering location differs.
pub(super) fn location_color(code: &str) -> Color {
    PALETTE[match code {
        "01111" | "01112" => 0,
        "01113" => 1,
        "01114" => 2,
        "01115" => 3,
        _ => unreachable!("every displayed Gathering location has a location color"),
    }]
}

/// What a marker links to. Order is the left-to-right order in a location's band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MarkerKind {
    EnemyPresence,
    Attachment,
}

impl MarkerKind {
    fn name(self) -> &'static str {
        match self {
            Self::EnemyPresence => "Enemy presence",
            Self::Attachment => "Attachment",
        }
    }

    /// Body corners; a round body is a fine polygon. The hexagon's first
    /// corner points right, so its flat edges face the band's top and the card.
    fn sides(self) -> u32 {
        match self {
            Self::EnemyPresence => 48,
            Self::Attachment => 6,
        }
    }

    /// A lone marker beside a top-entering link keeps this side of it.
    fn side(self) -> f32 {
        match self {
            Self::EnemyPresence => -1.,
            Self::Attachment => 1.,
        }
    }

    /// How far the body reaches from its center toward the card: the radius,
    /// or the hexagon's flat edge.
    fn reach(self, radius: f32) -> f32 {
        match self {
            Self::EnemyPresence => radius,
            Self::Attachment => radius * std::f32::consts::FRAC_PI_6.cos(),
        }
    }
}

/// A location-colored part that turns white while its group is highlighted.
#[derive(Component)]
pub(super) struct Tint {
    pub room: String,
    pub kind: MarkerKind,
    pub color: Color,
}

/// A sideboard heading's text starts at its anchor beside the heading token
/// and is centered vertically on it.
#[derive(Component)]
pub(super) struct HeadingLabel;

/// World centers for the given markers of one location, inside its reserved band
/// above the card. The group is centered unless a displayed link enters through
/// the card's top edge; then each marker takes its own side of that link.
pub(super) fn band_positions(
    map: &GatheringMap,
    room: &str,
    kinds: &[MarkerKind],
) -> Vec<(MarkerKind, Vec3)> {
    let band = map.marker_band(room);
    let center = map.position(room);
    let z = band.min.y + BAND_TOP_MARGIN + MAP_RADIUS;
    let mut kinds = kinds.to_vec();
    kinds.sort();
    let count = kinds.len() as f32;
    kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| {
            let x = match map.top_entry(room) {
                Some(link) => {
                    link + kind.side()
                        * (crate::tabletop_gathering_map::LINK_HEAD_HALF_WIDTH
                            + LINK_CLEARANCE
                            + MAP_RADIUS)
                }
                None => center.x + (index as f32 - (count - 1.) / 2.) * GROUP_PITCH,
            };
            (kind, Vec3::new(x, 0.12, z))
        })
        .collect()
}

/// The count pill hangs from a map token's lower edge.
fn pill_anchor(kind: MarkerKind, token: Vec3) -> Vec3 {
    token + Vec3::new(0., 0.02, kind.reach(MAP_RADIUS))
}

/// A token with a dark outline, a location-color ring, an ivory face and the
/// kind's ink emblem. The root carries the outline and stays pickable; the
/// layers above it are decoration.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_token(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: MarkerKind,
    room: &str,
    at: Vec3,
    radius: f32,
) -> Entity {
    let color = location_color(room);
    let flat = |color: Color, materials: &mut Assets<StandardMaterial>| {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        })
    };
    let body = |radius: f32, height: f32, meshes: &mut Assets<Mesh>| {
        meshes.add(
            Cylinder::new(radius, height)
                .mesh()
                .resolution(kind.sides())
                .build(),
        )
    };
    let outline = body(radius, TOKEN_HEIGHT, meshes);
    let ring = body(radius * 0.88, TOKEN_HEIGHT + 0.01, meshes);
    let face = body(radius * 0.70, TOKEN_HEIGHT + 0.02, meshes);
    let mut layers = vec![
        (
            "Location ring",
            ring,
            flat(color, materials),
            Vec3::ZERO,
            true,
        ),
        (
            "Ivory face",
            face,
            flat(IVORY, materials),
            Vec3::ZERO,
            false,
        ),
    ];
    let top = TOKEN_HEIGHT / 2. + 0.012;
    match kind {
        MarkerKind::EnemyPresence => {
            let width = radius * 0.56;
            layers.extend([
                (
                    "Slit-eye emblem",
                    meshes.add(convex(&lens(width, radius * 0.30))),
                    flat(INK, materials),
                    Vec3::Y * top,
                    false,
                ),
                (
                    "Iris",
                    meshes.add(convex(&ellipse(radius * 0.25, radius * 0.25))),
                    flat(color, materials),
                    Vec3::Y * (top + 0.002),
                    false,
                ),
                (
                    "Pupil slit",
                    meshes.add(convex(&ellipse(radius * 0.06, radius * 0.22))),
                    flat(INK, materials),
                    Vec3::Y * (top + 0.004),
                    false,
                ),
            ]);
        }
        MarkerKind::Attachment => {
            // Two links along the rising diagonal, the upper one over the lower.
            let axis = Vec2::new(1., -1.).normalize();
            for (index, center) in [-1., 1.].into_iter().enumerate() {
                let center = axis * center * radius * 0.20;
                let lift = top + index as f32 * 0.004;
                layers.extend([
                    (
                        "Chain link",
                        meshes.add(convex(&stadium(center, axis, radius * 0.16, radius * 0.19))),
                        flat(INK, materials),
                        Vec3::Y * lift,
                        false,
                    ),
                    (
                        "Link opening",
                        meshes.add(convex(&stadium(center, axis, radius * 0.16, radius * 0.09))),
                        flat(IVORY, materials),
                        Vec3::Y * (lift + 0.002),
                        false,
                    ),
                ]);
            }
        }
    }
    let name = format!("{} {}", kind.name(), play::location_name(room));
    commands
        .spawn((
            Name::new(name),
            ScenePiece,
            Mesh3d(outline),
            MeshMaterial3d(flat(OUTLINE, materials)),
            Transform::from_translation(at),
        ))
        .with_children(|root| {
            for (name, mesh, material, offset, tinted) in layers {
                let mut layer = root.spawn((
                    Name::new(name),
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(offset),
                    Pickable::IGNORE,
                ));
                if tinted {
                    layer.insert(Tint {
                        room: room.into(),
                        kind,
                        color,
                    });
                }
            }
        })
        .id()
}

/// The group count in a small dark pill beneath a map token, using the
/// counter-label renderer at the counter-label size in every view. Where the
/// band's projected room beneath the token is shorter than the pill, the pill
/// overlaps the card's top margin rather than shrinking.
pub(super) fn spawn_pill(
    commands: &mut Commands,
    context: &CardRender<'_>,
    accessible: String,
    count: usize,
    kind: MarkerKind,
    token: Vec3,
) -> Entity {
    let at = pill_anchor(kind, token);
    let label = scene::label(commands, context, count.to_string(), at, false);
    commands.entity(label).insert((
        AccessibleLabel::new(accessible),
        TextFont {
            font_size: bevy::text::FontSize::Px(COUNT_SIZE),
            ..default()
        },
        // A compact line keeps the pill as short as its digits.
        bevy::text::LineHeight::Px(COUNT_SIZE),
        TextLayout::justify(Justify::Center).with_no_wrap(),
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(5.), px(0.)),
            border_radius: BorderRadius::all(px(7.)),
            ..default()
        },
        BackgroundColor(PILL),
    ));
    label
}

/// Two circular arcs meeting at sharp corners on the horizontal axis.
fn lens(half_width: f32, half_height: f32) -> Vec<Vec2> {
    let radius = (half_width * half_width + half_height * half_height) / (2. * half_height);
    let reach = (half_width / radius).asin();
    let mut points = Vec::new();
    for (center, sign) in [(radius - half_height, -1.), (half_height - radius, 1.)] {
        for step in 0..12 {
            let angle = -reach + 2. * reach * step as f32 / 12.;
            let x = radius * angle.sin() * -sign;
            points.push(Vec2::new(x, center + sign * radius * angle.cos()));
        }
    }
    points
}

/// A link's outline: two half circles joined along `axis` around `center`.
fn stadium(center: Vec2, axis: Vec2, half_length: f32, radius: f32) -> Vec<Vec2> {
    let across = axis.perp();
    let mut points = Vec::new();
    for end in [1., -1.] {
        for step in 0..=12 {
            let angle = std::f32::consts::PI * step as f32 / 12.;
            points.push(
                center
                    + axis * end * (half_length + radius * angle.sin())
                    + across * end * radius * angle.cos(),
            );
        }
    }
    points
}

fn ellipse(half_width: f32, half_height: f32) -> Vec<Vec2> {
    (0..32)
        .map(|step| {
            let angle = std::f32::consts::TAU * step as f32 / 32.;
            Vec2::new(half_width * angle.cos(), half_height * angle.sin())
        })
        .collect()
}

/// A flat, upward-facing fan over a convex outline in table (x, z) coordinates.
fn convex(outline: &[Vec2]) -> Mesh {
    let center = outline.iter().copied().sum::<Vec2>() / outline.len() as f32;
    let mut positions = Vec::with_capacity(outline.len() * 3);
    for (index, &a) in outline.iter().enumerate() {
        let mut b = a;
        let mut c = outline[(index + 1) % outline.len()];
        if (b - center).perp_dot(c - center) > 0. {
            std::mem::swap(&mut b, &mut c);
        }
        positions.extend([center, b, c].map(|point| [point.x, 0., point.y]));
    }
    let normals = vec![[0., 1., 0.]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}
