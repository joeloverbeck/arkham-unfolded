//! Agenda-only altar and physical changes at each doom bearer's own location.
use super::*;
use crate::{gathering::PlayObservation, tabletop_counters as counters};
use bevy::{asset::RenderAssetUsages, render::render_resource::PrimitiveTopology};

pub(super) const ALTAR: Vec3 = Vec3::new(0., 0.3, -18.4);
const DURATION: f32 = 0.8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    Agenda,
    Enemy(u16),
}

struct Change {
    owner: Owner,
    old: u8,
    new: u8,
    at: Vec3,
    elapsed: f32,
}

#[derive(Default)]
pub(super) struct State {
    changes: Vec<Change>,
}

impl State {
    pub(super) fn active(&self) -> bool {
        !self.changes.is_empty()
    }
    pub(super) fn clear(&mut self) {
        self.changes.clear();
    }

    pub(super) fn prepare(&mut self, before: &PlayObservation, after: &PlayObservation) {
        // Replaced observations invalidate motion toward another destination.
        if self.changes.iter().any(|change| {
            let current = match change.owner {
                Owner::Agenda => after.agenda_doom,
                Owner::Enemy(id) => after
                    .scenario
                    .enemies
                    .iter()
                    .find(|enemy| enemy.card.id == id)
                    .map_or(0, |enemy| enemy.doom),
            };
            current != change.new
        }) {
            self.clear();
        }
        if self.active() {
            return;
        }
        self.change(Owner::Agenda, before.agenda_doom, after.agenda_doom, ALTAR);
        for (slot, enemy) in before.scenario.enemies.iter().enumerate() {
            let next = after
                .scenario
                .enemies
                .iter()
                .find(|next| next.card.id == enemy.card.id);
            self.change(
                Owner::Enemy(enemy.card.id),
                enemy.doom,
                next.map_or(0, |next| next.doom),
                enemy_at(before, slot),
            );
        }
        for (slot, enemy) in after.scenario.enemies.iter().enumerate() {
            if !before
                .scenario
                .enemies
                .iter()
                .any(|prior| prior.card.id == enemy.card.id)
            {
                self.change(
                    Owner::Enemy(enemy.card.id),
                    0,
                    enemy.doom,
                    enemy_at(after, slot),
                );
            }
        }
    }

    fn change(&mut self, owner: Owner, old: u8, new: u8, at: Vec3) {
        if old != new {
            self.changes.push(Change {
                owner,
                old,
                new,
                at,
                elapsed: 0.,
            });
        }
    }

    fn shown(&self, owner: Owner, current: u8) -> u8 {
        self.changes
            .iter()
            .find(|change| change.owner == owner)
            .map_or(current, |change| change.old.min(change.new))
    }

    pub(super) fn agenda(&self, current: u8) -> u8 {
        self.shown(Owner::Agenda, current)
    }
    pub(super) fn enemy(&self, id: u16, current: u8) -> u8 {
        self.shown(Owner::Enemy(id), current)
    }

    fn total(&self, play: &PlayObservation) -> u16 {
        let current = i32::from(total(play));
        let pending: i32 = self
            .changes
            .iter()
            .map(|change| i32::from(change.old) - i32::from(change.new))
            .sum();
        u16::try_from(current + pending).expect("Displayed doom includes pending physical changes")
    }
}

fn enemy_at(play: &PlayObservation, slot: usize) -> Vec3 {
    let edge = if play.scenario.enemies[slot].exhausted {
        0.94
    } else {
        0.67
    };
    play::enemy_position(play, slot) + Vec3::new(edge, 0.15, 0.)
}

fn total(play: &PlayObservation) -> u16 {
    u16::from(play.agenda_doom)
        + play
            .scenario
            .enemies
            .iter()
            .map(|enemy| u16::from(enemy.doom))
            .sum::<u16>()
}

fn threshold(play: &PlayObservation) -> u16 {
    match play.agenda {
        1 => 3,
        2 => 7,
        3 => 10,
        _ => unreachable!("Original Gathering agenda"),
    }
}

pub(super) fn inspection(play: &PlayObservation) -> String {
    let mut lines = vec![
        format!("Total doom {}/{}", total(play), threshold(play)),
        format!("Agenda {}: {} doom", play.agenda, play.agenda_doom),
    ];
    lines.extend(
        play.scenario
            .enemies
            .iter()
            .filter(|enemy| enemy.doom > 0)
            .map(|enemy| {
                format!(
                    "{} in {}: {} doom",
                    enemy.card.title,
                    play::location_text(&enemy.location),
                    enemy.doom
                )
            }),
    );
    lines.push("The altar holds only agenda doom. Doom on enemies remains on those enemies. Check all doom during Mythos; other placement advances the agenda only when the effect explicitly says so.".into());
    lines.join("\n")
}

/// A closed, flat-shaded clipped rectangle, low enough to clear both book faces.
fn slab(width: f32, depth: f32, height: f32) -> Mesh {
    let (x, z, c) = (width / 2., depth / 2., 0.18);
    let ring = [
        (-x + c, -z),
        (x - c, -z),
        (x, -z + c),
        (x, z - c),
        (x - c, z),
        (-x + c, z),
        (-x, z - c),
        (-x, -z + c),
    ];
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut triangle = |a: Vec3, b: Vec3, c: Vec3| {
        let normal = (b - a).cross(c - a).normalize().to_array();
        positions.extend([a.to_array(), b.to_array(), c.to_array()]);
        normals.extend([normal; 3]);
    };
    for index in 0..ring.len() {
        let (ax, az) = ring[index];
        let (bx, bz) = ring[(index + 1) % ring.len()];
        let (a, b) = (Vec3::new(ax, 0., az), Vec3::new(bx, 0., bz));
        let up = Vec3::Y * height;
        triangle(up, b + up, a + up);
        triangle(Vec3::ZERO, a, b);
        triangle(a, a + up, b + up);
        triangle(a, b + up, b);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

pub(super) fn render(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender,
) {
    let Some(play) = &context.view.play else {
        return;
    };
    let state = &context.consequences.doom;
    for (name, width, depth, height, y, color) in [
        (
            "Agenda doom altar",
            2.,
            1.2,
            0.10,
            0.07,
            Color::srgb(0.07, 0.09, 0.11),
        ),
        ("Altar brass edge", 1.86, 1.06, 0.035, 0.17, GOLD),
        (
            "Altar slate top",
            1.78,
            0.98,
            0.055,
            0.205,
            Color::srgb(0.045, 0.065, 0.08),
        ),
    ] {
        commands.spawn((
            ScenePiece,
            Name::new(name),
            Mesh3d(meshes.add(slab(width, depth, height))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.35,
                perceptual_roughness: 0.6,
                ..default()
            })),
            Transform::from_xyz(0., y, ALTAR.z),
            Pickable::IGNORE,
        ));
    }
    if let Some((token, label)) = counters::spawn_count(
        commands,
        meshes,
        materials,
        counters::CounterVisual {
            kind: counters::CounterKind::Doom,
            amount: state.agenda(play.agenda_doom),
            image: context.art.image("token/doom"),
            at: ALTAR,
            scale: 1.,
        },
    ) {
        commands.entity(token).insert(ScenePiece);
        commands.entity(label).insert(ScenePiece);
    }
}

/// Mat lettering follows the same physical-arrival counts as the solid tokens.
pub(super) fn readouts(context: &CardRender) -> Option<[String; 2]> {
    let play = context.view.play.as_ref()?;
    let state = &context.consequences.doom;
    Some([
        format!("Total doom: {}/{}", state.total(play), threshold(play)),
        format!("Agenda doom: {}", state.agenda(play.agenda_doom)),
    ])
}

#[derive(Component)]
pub(super) struct Moving {
    owner: Owner,
    label: Entity,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn advance(
    mut commands: Commands,
    mut state: ResMut<consequences::State>,
    mut table: ResMut<Table>,
    health: Res<RenderHealth>,
    time: Res<PresentationTime>,
    art: Res<art::Art>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut moving: Query<(Entity, &Moving, &mut Transform)>,
) {
    let paused = !health.allows_play() || table.motion_blocked();
    if table.reduced_motion {
        state.doom.clear();
    }
    if !paused {
        for change in &mut state.doom.changes {
            change.elapsed += time.delta_secs();
        }
    }
    let before = state.doom.changes.len();
    state
        .doom
        .changes
        .retain(|change| change.elapsed < DURATION);
    if before != state.doom.changes.len() {
        table.dirty = true;
    }
    for (entity, token, _) in &moving {
        if !state
            .doom
            .changes
            .iter()
            .any(|change| change.owner == token.owner)
        {
            commands.entity(entity).despawn();
            commands.entity(token.label).try_despawn();
        }
    }
    for change in &state.doom.changes {
        let fraction = (change.elapsed / DURATION).clamp(0., 1.);
        let adding = change.new > change.old;
        let at = change.at
            + Vec3::Y
                * if adding {
                    1.5 * (1. - fraction).powi(2)
                } else {
                    -0.2 * fraction
                };
        let scale = if adding {
            1.
        } else {
            (1. - fraction).max(0.001)
        };
        if let Some((_, _, mut transform)) = moving
            .iter_mut()
            .find(|(_, token, _)| token.owner == change.owner)
        {
            transform.translation = at;
            transform.scale = Vec3::splat(scale);
        } else {
            let (token, label) = counters::spawn_count(
                &mut commands,
                &mut meshes,
                &mut materials,
                counters::CounterVisual {
                    kind: counters::CounterKind::Doom,
                    amount: change.old.abs_diff(change.new),
                    image: art.image("token/doom"),
                    at,
                    scale,
                },
            )
            .expect("Doom changes have a nonzero amount");
            commands.entity(token).insert(Moving {
                owner: change.owner,
                label,
            });
        }
    }
}
