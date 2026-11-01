//! The Gathering's visible connections, relative placement, and physical links.
//! These are presentation projections of the inspected card faces, not movement rules.

use super::{Scene, layout};
use crate::party::LocationView;
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::PrimitiveTopology};

#[cfg(test)]
mod tests;

const ROOM_GAP: f32 = 1.6;
const PORT_GAP: f32 = 0.32;
const LINK_HALF_WIDTH: f32 = 0.085;
const HEAD_WIDTH_RATIO: f32 = 2.8;
const LINK_HEIGHT: f32 = -0.025;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Room {
    Study,
    Hallway,
    Attic,
    Cellar,
    Parlor,
}

impl Room {
    fn from_code(code: &str) -> Option<Self> {
        match code {
            "01111" => Some(Self::Study),
            "01112" => Some(Self::Hallway),
            "01113" => Some(Self::Attic),
            "01114" => Some(Self::Cellar),
            "01115" => Some(Self::Parlor),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Study => "Study",
            Self::Hallway => "Hallway",
            Self::Attic => "Attic",
            Self::Cellar => "Cellar",
            Self::Parlor => "Parlor",
        }
    }

    /// Only the shown face contributes symbols. The inspected original faces of
    /// these four rooms happen to keep their connections when revealed.
    fn connections(self, _revealed: bool) -> &'static [Self] {
        match self {
            Self::Study => &[],
            Self::Hallway => &[Self::Attic, Self::Cellar, Self::Parlor],
            Self::Attic | Self::Cellar | Self::Parlor => &[Self::Hallway],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Connection {
    from: Room,
    to: Room,
    forward: bool,
    backward: bool,
}

#[derive(Component)]
pub(super) struct ConnectionVisual;

#[derive(Clone, Copy, Debug)]
struct Footprint {
    center: Vec2,
    bounds: Rect,
}

impl Footprint {
    fn location(transform: Transform, size: Vec2) -> Self {
        let radius = crate::tabletop_standees::radius_for(size);
        // Reserve every standee slot, even when empty. The caption allowance
        // covers two short 13px lines at the normal Scenario focus. Native
        // framing remains the check for their projected pixel extents.
        let half_width = (size.x / 2.0 + radius * 2.0 + 0.1).max(2.3);
        let local = Rect::from_corners(
            Vec2::new(-half_width, -size.y / 2.0 - 0.2),
            Vec2::new(half_width, size.y / 2.0 + 2.0),
        );
        let mut bounds = Rect {
            min: Vec2::splat(f32::INFINITY),
            max: Vec2::splat(f32::NEG_INFINITY),
        };
        for point in [
            local.min,
            Vec2::new(local.min.x, local.max.y),
            local.max,
            Vec2::new(local.max.x, local.min.y),
        ] {
            let world = transform.transform_point(Vec3::new(point.x, 0.0, point.y));
            let point = world.xz();
            bounds.min = bounds.min.min(point);
            bounds.max = bounds.max.max(point);
        }
        Self {
            center: transform.translation.xz(),
            bounds,
        }
    }

    fn boundary(self, direction: Vec2) -> Vec2 {
        let mut distance = f32::INFINITY;
        for axis in 0..2 {
            if direction[axis] > 0.0 {
                distance =
                    distance.min((self.bounds.max[axis] - self.center[axis]) / direction[axis]);
            } else if direction[axis] < 0.0 {
                distance =
                    distance.min((self.bounds.min[axis] - self.center[axis]) / direction[axis]);
            }
        }
        self.center + direction * (distance + PORT_GAP)
    }

    fn ports(self) -> [Vec2; 4] {
        [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y].map(|direction| self.boundary(direction))
    }
}

#[derive(Clone, Copy)]
struct Placement {
    room: Room,
    at: Vec3,
    footprint: Footprint,
}

pub(super) struct GatheringMap {
    placements: Vec<Placement>,
    links: Vec<(Connection, Vec<Vec2>)>,
}

impl GatheringMap {
    pub fn new(locations: &[LocationView], size: Vec2) -> Result<Self, &'static str> {
        let mut shown = locations
            .iter()
            .map(|location| {
                Room::from_code(&location.code)
                    .map(|room| (room, location.revealed))
                    .ok_or("location is not part of the prepared Gathering map")
            })
            .collect::<Result<Vec<_>, _>>()?;
        shown.sort_by_key(|(room, _)| *room);
        if shown.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err("duplicate location in the prepared map");
        }
        let origin = layout::SCENARIO_MAP_CENTER;
        let footprint = Footprint::location(Transform::IDENTITY, size);
        let spacing = footprint.bounds.size() + Vec2::splat(ROOM_GAP);
        // The source supplies a star, not coordinates. These are relative
        // presentation hints: Hallway is the hub, Attic above, Cellar below.
        let placements: Vec<_> = shown
            .iter()
            .map(|(room, _)| {
                let offset = match room {
                    Room::Attic => Vec3::NEG_Z * spacing.y,
                    Room::Cellar => Vec3::Z * spacing.y,
                    Room::Parlor => Vec3::X * spacing.x,
                    Room::Study | Room::Hallway => Vec3::ZERO,
                };
                let at = origin + offset;
                Placement {
                    room: *room,
                    at,
                    footprint: Footprint::location(Transform::from_translation(at), size),
                }
            })
            .collect();
        // Leave the far strip for the book and decks, and a margin at the mat edge.
        let half_width = layout::SCENARIO_SIZE.x / 2.0 - 0.2;
        let usable = Rect::from_corners(
            Vec2::new(
                layout::SCENARIO_CENTER.x - half_width,
                layout::SCENARIO_FAR_EDGE + 4.5,
            ),
            Vec2::new(
                layout::SCENARIO_CENTER.x + half_width,
                layout::SCENARIO_CENTER.z + layout::SCENARIO_SIZE.y / 2.0 - 0.2,
            ),
        );
        if placements.iter().any(|placement| {
            !usable.contains(placement.footprint.bounds.min)
                || !usable.contains(placement.footprint.bounds.max)
        }) {
            return Err("location footprints do not fit the scenario surface");
        }
        let obstacles: Vec<_> = placements.iter().map(|p| p.footprint.bounds).collect();
        let mut links = Vec::new();
        for (index, &(from, from_revealed)) in shown.iter().enumerate() {
            for (other, &(to, to_revealed)) in shown.iter().enumerate().skip(index + 1) {
                let forward = from.connections(from_revealed).contains(&to);
                let backward = to.connections(to_revealed).contains(&from);
                if forward || backward {
                    let points = route(
                        placements[index].footprint,
                        placements[other].footprint,
                        &obstacles,
                    )
                    .ok_or("no clear connection route between location footprints")?;
                    links.push((
                        Connection {
                            from,
                            to,
                            forward,
                            backward,
                        },
                        points,
                    ));
                }
            }
        }
        Ok(Self { placements, links })
    }

    pub fn position(&self, code: &str) -> Vec3 {
        let room = Room::from_code(code).expect("map construction checked location identities");
        self.placements
            .iter()
            .find(|p| p.room == room)
            .expect("shown location has a placement")
            .at
    }

    pub fn entry_blocked(location: &LocationView) -> bool {
        location.code == "01115" && !location.revealed
    }

    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) {
        if self.links.is_empty() {
            return;
        }
        let rim = materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.13, 0.08),
            unlit: true,
            ..default()
        });
        let brass = materials.add(StandardMaterial {
            base_color: Color::srgb(0.70, 0.54, 0.27),
            unlit: true,
            ..default()
        });
        for (connection, points) in &self.links {
            let arrow = match (connection.forward, connection.backward) {
                (true, true) => "↔",
                (true, false) => "→",
                _ => "←",
            };
            commands
                .spawn((
                    Name::new(format!(
                        "Location connection {} {arrow} {}",
                        connection.from.name(),
                        connection.to.name()
                    )),
                    ConnectionVisual,
                    Transform::IDENTITY,
                    Visibility::default(),
                    Pickable::IGNORE,
                    Scene,
                ))
                .with_children(|parent| {
                    for (half_width, height, material) in [
                        (LINK_HALF_WIDTH, LINK_HEIGHT, rim.clone()),
                        (0.052, LINK_HEIGHT + 0.004, brass.clone()),
                    ] {
                        parent.spawn((
                            Mesh3d(meshes.add(link_mesh(points, *connection, half_width, height))),
                            MeshMaterial3d(material),
                            Transform::IDENTITY,
                            Pickable::IGNORE,
                        ));
                    }
                });
        }
    }
}

fn intersects_segment(bounds: Rect, a: Vec2, b: Vec2) -> bool {
    let delta = b - a;
    let mut lower: f32 = 0.0;
    let mut upper: f32 = 1.0;
    for axis in 0..2 {
        if delta[axis].abs() < 0.00001 {
            if a[axis] < bounds.min[axis] || a[axis] > bounds.max[axis] {
                return false;
            }
        } else {
            let start = (bounds.min[axis] - a[axis]) / delta[axis];
            let end = (bounds.max[axis] - a[axis]) / delta[axis];
            lower = lower.max(start.min(end));
            upper = upper.min(start.max(end));
            if lower > upper {
                return false;
            }
        }
    }
    true
}

fn clear(points: &[Vec2], obstacles: &[Rect]) -> bool {
    points.windows(2).all(|pair| {
        obstacles.iter().all(|bounds| {
            !intersects_segment(
                bounds.inflate(LINK_HALF_WIDTH * HEAD_WIDTH_RATIO + 0.02),
                pair[0],
                pair[1],
            )
        })
    })
}

fn rounded(points: &[Vec2]) -> Vec<Vec2> {
    let mut result = vec![points[0]];
    for triple in points.windows(3) {
        let incoming = triple[1] - triple[0];
        let outgoing = triple[2] - triple[1];
        let radius = (incoming.length().min(outgoing.length()) * 0.25).min(0.12);
        let start = triple[1] - incoming.normalize_or_zero() * radius;
        let end = triple[1] + outgoing.normalize_or_zero() * radius;
        result.push(start);
        for step in 1..=4 {
            let t = step as f32 / 4.0;
            result.push(start.lerp(triple[1], t).lerp(triple[1].lerp(end, t), t));
        }
    }
    result.push(*points.last().unwrap());
    result.dedup_by(|a, b| a.distance_squared(*b) < 0.000001);
    result
}

fn route(from: Footprint, to: Footprint, obstacles: &[Rect]) -> Option<Vec<Vec2>> {
    let direction = (to.center - from.center).try_normalize()?;
    let direct = vec![from.boundary(direction), to.boundary(-direction)];
    if (direct[1] - direct[0]).dot(direction) >= 0.65 && clear(&direct, obstacles) {
        return Some(direct);
    }
    let mut best: Option<(f32, Vec<Vec2>)> = None;
    for start in from.ports() {
        for end in to.ports() {
            let mut candidates = vec![
                vec![start, Vec2::new(start.x, end.y), end],
                vec![start, Vec2::new(end.x, start.y), end],
            ];
            for obstacle in obstacles {
                for z in [obstacle.min.y - PORT_GAP, obstacle.max.y + PORT_GAP] {
                    candidates.push(vec![start, Vec2::new(start.x, z), Vec2::new(end.x, z), end]);
                }
                for x in [obstacle.min.x - PORT_GAP, obstacle.max.x + PORT_GAP] {
                    candidates.push(vec![start, Vec2::new(x, start.y), Vec2::new(x, end.y), end]);
                }
            }
            for mut candidate in candidates {
                candidate.dedup_by(|a, b| a.distance_squared(*b) < 0.000001);
                if candidate.len() < 2 {
                    continue;
                }
                let candidate = rounded(&candidate);
                let length = candidate
                    .windows(2)
                    .map(|p| p[0].distance(p[1]))
                    .sum::<f32>();
                if length >= 0.65
                    && clear(&candidate, obstacles)
                    && best.as_ref().is_none_or(|(old, _)| length < *old)
                {
                    best = Some((length, candidate));
                }
            }
        }
    }
    best.map(|(_, points)| points)
}

fn link_mesh(points: &[Vec2], connection: Connection, half_width: f32, height: f32) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut triangle = |a: Vec2, mut b: Vec2, mut c: Vec2| {
        if (b - a).perp_dot(c - a) > 0.0 {
            std::mem::swap(&mut b, &mut c);
        }
        positions.extend([a, b, c].map(|point| [point.x, height, point.y]));
    };
    let length = points.windows(2).map(|p| p[0].distance(p[1])).sum::<f32>();
    let tip = (length * 0.20).min(0.25);
    let mut shaft = points.to_vec();
    for (arrow, tip_index, neighbor) in [
        (connection.backward, 0, 1),
        (connection.forward, points.len() - 1, points.len() - 2),
    ] {
        if arrow {
            let segment = points[tip_index] - points[neighbor];
            let direction = segment.normalize();
            let base = points[tip_index] - direction * tip.min(segment.length() * 0.8);
            let side = direction.perp() * (half_width * HEAD_WIDTH_RATIO);
            triangle(points[tip_index], base + side, base - side);
            shaft[tip_index] = base;
        }
    }
    for pair in shaft.windows(2) {
        let side = (pair[1] - pair[0]).normalize_or_zero().perp() * half_width;
        triangle(pair[0] + side, pair[1] + side, pair[1] - side);
        triangle(pair[0] + side, pair[1] - side, pair[0] - side);
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}
