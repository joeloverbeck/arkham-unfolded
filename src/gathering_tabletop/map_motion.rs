//! Finite visual beats between committed Gathering observations. This clock never
//! moves a rules piece, reveals a rules face, places a clue, or writes a save.
use super::*;
use crate::tabletop_gathering_map::GatheringMap;
use std::{collections::VecDeque, f32::consts::PI};

const SIZE: Vec2 = Vec2::new(1.35, 1.89);
const TRAVEL: f32 = 0.85;
const FLIP: f32 = 0.55;
const CLUES: f32 = 0.4;

#[derive(Component, Clone, PartialEq, Eq)]
pub(super) enum Surface {
    Investigator(Investigator),
    Location(String),
    Clue(String),
    Enemy(u16),
    Glint(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Travel(usize),
    Flip,
    Clues,
    Settle,
}

struct Beat {
    stage: Stage,
    view: Observation,
    cards: Vec<CardTravel>,
}

struct CardTravel {
    id: u16,
    start: Vec3,
    end: Vec3,
}

struct Travel {
    piece: Surface,
    group: usize,
    from: String,
    path: Vec<Vec3>,
    title: String,
}

struct Reveal {
    code: String,
    at: Vec3,
    clues: u8,
}

#[derive(Default)]
pub(super) struct Playback {
    beats: VecDeque<Beat>,
    travels: Vec<Travel>,
    reveals: Vec<Reveal>,
    elapsed: f32,
    rendered: bool,
    arrivals: Vec<(u16, Vec3)>,
}

fn map(view: &crate::gathering::PlayObservation) -> GatheringMap {
    GatheringMap::new(
        view.scenario
            .locations
            .iter()
            .filter(|room| !view.scenario.location_in_victory_display(&room.code))
            .map(|room| (room.code.as_str(), room.revealed)),
        SIZE,
    )
    .expect("The admitted Gathering locations fit their shared surface")
}

fn slot(map: &GatheringMap, room: &str, who: Investigator) -> Vec3 {
    layout::location_standee_position(map.position(room), SIZE, who.index())
        .expect("Each of the three investigators has a stable standee slot")
}

impl Playback {
    pub(super) fn new(before: &Observation, target: &Observation) -> Self {
        let (Some(prior), Some(after)) = (&before.play, &target.play) else {
            return Self::default();
        };
        let map = map(after);
        let mut result = Self::default();
        for player in &after.players {
            let old = &prior.players[player.investigator.index()];
            if old.location == player.location || player.defeated || player.resigned {
                continue;
            }
            // Act 1 placement has no connecting arrow and keeps the same map center.
            let Some(route) = map.connection_route(&old.location, &player.location) else {
                continue;
            };
            let start = slot(&map, &old.location, player.investigator);
            let end = slot(&map, &player.location, player.investigator);
            result.travels.push(Travel {
                piece: Surface::Investigator(player.investigator),
                group: result.travels.len(),
                from: old.location.clone(),
                path: rounded_path(start, &route, end),
                title: player.investigator.name().into(),
            });
        }
        for enemy in &after.scenario.enemies {
            let Some(old) = prior
                .scenario
                .enemies
                .iter()
                .find(|old| old.card.id == enemy.card.id)
            else {
                continue;
            };
            if old.location == enemy.location {
                continue;
            }
            let Some(route) = map.connection_route(&old.location, &enemy.location) else {
                continue;
            };
            let marker = |room: &str| map.position(room) + Vec3::new(0., 0.08, -1.35);
            // Engaged enemies accompany their investigator; independent enemy
            // moves get separate beats so simultaneous agenda movement is legible.
            let group = old
                .engaged
                .and_then(|who| {
                    result
                        .travels
                        .iter()
                        .find(|travel| {
                            travel.piece == Surface::Investigator(who)
                                && after.players[who.index()].location == enemy.location
                        })
                        .map(|travel| travel.group)
                })
                .unwrap_or(result.travels.len());
            result.travels.push(Travel {
                piece: Surface::Enemy(enemy.card.id),
                group,
                from: old.location.clone(),
                path: rounded_path(marker(&old.location), &route, marker(&enemy.location)),
                title: enemy.card.title.clone(),
            });
        }
        for room in &after.scenario.locations {
            if room.revealed
                && !after.scenario.location_in_victory_display(&room.code)
                && prior
                    .scenario
                    .locations
                    .iter()
                    .any(|old| old.code == room.code && !old.revealed)
            {
                result.reveals.push(Reveal {
                    code: room.code.clone(),
                    at: map.position(&room.code),
                    clues: room.clues,
                });
            }
        }
        if result.travels.is_empty() && result.reveals.is_empty() {
            return result;
        }
        let mut arrived = target.clone();
        arrived.choices.clear();
        arrived.decision_owner = None;
        // Hold new clues until the entire physical face has landed.
        for room in &mut arrived.play.as_mut().unwrap().scenario.locations {
            if result.reveals.iter().any(|reveal| reveal.code == room.code) {
                room.clues = 0;
            }
        }
        if !result.travels.is_empty() {
            let mut travelling = arrived.clone();
            let play = travelling.play.as_mut().unwrap();
            for travel in &result.travels {
                match travel.piece {
                    Surface::Investigator(who) => {
                        play.players[who.index()].location = travel.from.clone()
                    }
                    Surface::Enemy(id) => {
                        let enemy = play
                            .scenario
                            .enemies
                            .iter_mut()
                            .find(|enemy| enemy.card.id == id)
                            .unwrap();
                        enemy.location = travel.from.clone();
                        enemy.engaged = prior
                            .scenario
                            .enemies
                            .iter()
                            .find(|old| old.card.id == id)
                            .unwrap()
                            .engaged;
                    }
                    _ => unreachable!("Only investigators and enemies travel"),
                }
            }
            // A resident enemy can engage the arriving investigator without
            // moving rooms. Keep that engagement at the arrival boundary too.
            for enemy in &mut play.scenario.enemies {
                if let Some(old) = prior
                    .scenario
                    .enemies
                    .iter()
                    .find(|old| old.card.id == enemy.card.id)
                {
                    enemy.engaged = old.engaged;
                }
            }
            for room in &mut play.scenario.locations {
                if result.reveals.iter().any(|reveal| reveal.code == room.code) {
                    room.revealed = false;
                }
            }
            let mut groups: Vec<_> = result.travels.iter().map(|travel| travel.group).collect();
            groups.sort_unstable();
            groups.dedup();
            for group in groups {
                let departing = travelling.clone();
                let play = travelling.play.as_mut().unwrap();
                for travel in result.travels.iter().filter(|travel| travel.group == group) {
                    match travel.piece {
                        Surface::Investigator(who) => {
                            play.players[who.index()].location =
                                after.players[who.index()].location.clone()
                        }
                        Surface::Enemy(id) => {
                            let target = after
                                .scenario
                                .enemies
                                .iter()
                                .find(|enemy| enemy.card.id == id)
                                .unwrap();
                            let enemy = play
                                .scenario
                                .enemies
                                .iter_mut()
                                .find(|enemy| enemy.card.id == id)
                                .unwrap();
                            enemy.location = target.location.clone();
                            enemy.engaged = target.engaged;
                        }
                        _ => unreachable!("Only investigators and enemies travel"),
                    }
                }
                for enemy in &mut play.scenario.enemies {
                    let target = after
                        .scenario
                        .enemies
                        .iter()
                        .find(|target| target.card.id == enemy.card.id)
                        .unwrap();
                    let old = prior
                        .scenario
                        .enemies
                        .iter()
                        .find(|old| old.card.id == enemy.card.id);
                    if target
                        .engaged
                        .into_iter()
                        .chain(old.and_then(|enemy| enemy.engaged))
                        .any(|who| {
                            result.travels.iter().any(|travel| {
                                travel.group == group && travel.piece == Surface::Investigator(who)
                            })
                        })
                    {
                        enemy.engaged = target.engaged;
                    }
                }
                let prior_play = departing.play.as_ref().unwrap();
                let cards = play
                    .scenario
                    .enemies
                    .iter()
                    .enumerate()
                    .filter_map(|(index, enemy)| {
                        let old_index = prior_play
                            .scenario
                            .enemies
                            .iter()
                            .position(|old| old.card.id == enemy.card.id)?;
                        let start = play::enemy_position(prior_play, old_index);
                        let end = play::enemy_position(play, index);
                        (!start.abs_diff_eq(end, 0.001)).then_some(CardTravel {
                            id: enemy.card.id,
                            start,
                            end,
                        })
                    })
                    .collect();
                result.beats.push_back(Beat {
                    stage: Stage::Travel(group),
                    view: departing,
                    cards,
                });
            }
        }
        if !result.reveals.is_empty() {
            result.beats.push_back(Beat {
                stage: Stage::Flip,
                view: arrived,
                cards: vec![],
            });
        }
        let mut settled = target.clone();
        settled.choices.clear();
        settled.decision_owner = None;
        if result.reveals.iter().any(|reveal| reveal.clues > 0) {
            result.beats.push_back(Beat {
                stage: Stage::Clues,
                view: settled,
                cards: vec![],
            });
        } else if result.reveals.is_empty() {
            // Let the corresponding enemy card arrive at its board/threat slot.
            result.beats.push_back(Beat {
                stage: Stage::Settle,
                view: settled,
                cards: vec![],
            });
        }
        result
    }

    pub(super) fn view(&self) -> Option<&Observation> {
        self.beats.front().map(|beat| &beat.view)
    }
    pub(super) fn active(&self) -> bool {
        !self.beats.is_empty()
    }
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn rendered(&mut self) {
        self.rendered = true;
    }
    pub(super) fn take_arrivals(&mut self) -> Vec<(u16, Vec3)> {
        std::mem::take(&mut self.arrivals)
    }
    fn stage(&self) -> Option<Stage> {
        self.beats.front().map(|beat| beat.stage)
    }

    fn travelling(&self) -> impl Iterator<Item = &Travel> {
        self.travels
            .iter()
            .filter(|travel| self.stage() == Some(Stage::Travel(travel.group)))
    }

    pub(super) fn advance(&mut self, delta: f32, moving: bool) -> bool {
        if !self.rendered {
            return false;
        }
        let duration = match self.stage() {
            Some(Stage::Travel(_)) => TRAVEL,
            Some(Stage::Flip) => FLIP,
            Some(Stage::Clues) => CLUES,
            Some(Stage::Settle) => motion::CARD_DURATION,
            None => return false,
        };
        self.elapsed = (self.elapsed + delta).min(duration);
        if self.elapsed < duration || (self.stage() == Some(Stage::Settle) && moving) {
            return false;
        }
        if let Some(beat) = self.beats.pop_front() {
            self.arrivals
                .extend(beat.cards.into_iter().map(|card| (card.id, card.end)));
        }
        self.elapsed = 0.;
        self.rendered = false;
        true
    }

    pub(super) fn render(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        context: &CardRender<'_>,
    ) {
        if matches!(self.stage(), Some(Stage::Travel(_))) {
            for travel in self.travelling() {
                let Surface::Enemy(_) = travel.piece else {
                    continue;
                };
                let marker = commands
                    .spawn((
                        ScenePiece,
                        Name::new(format!("Moving {}", travel.title)),
                        travel.piece.clone(),
                        Mesh3d(meshes.add(Cylinder::new(0.28, 0.12))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.32, 0.45, 0.64),
                            metallic: 0.45,
                            ..default()
                        })),
                        Transform::from_translation(travel.path[0]),
                        Pickable::IGNORE,
                    ))
                    .id();
                let label = scene::label(commands, context, &travel.title, travel.path[0], false);
                commands.entity(label).insert((
                    motion::CardLabel(marker, Vec3::new(0., 0.2, 0.55)),
                    enemy_board::PersistentLabel,
                ));
            }
        }
        if self.stage() == Some(Stage::Clues) {
            for reveal in self.reveals.iter().filter(|reveal| reveal.clues > 0) {
                commands.spawn((
                    ScenePiece,
                    Name::new("New clues glint"),
                    Surface::Glint(reveal.code.clone()),
                    Mesh3d(meshes.add(Torus::new(0.38, 0.43))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(0.75, 0.9, 0.45, 0.),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    })),
                    Transform::from_translation(layout::location_clue(reveal.at)),
                    Pickable::IGNORE,
                ));
            }
        }
    }
}

fn smooth(value: f32) -> f32 {
    let t = value.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

// Round the visible arrow's corners very slightly, keeping its actual route and
// endpoints. This avoids a rigid right-angle turn while the piece is airborne.
fn rounded_path(start: Vec3, route: &[Vec3], end: Vec3) -> Vec<Vec3> {
    let points: Vec<_> = std::iter::once(start)
        .chain(route.iter().copied())
        .chain([end])
        .collect();
    let mut rounded = vec![start];
    for triple in points.windows(3) {
        let [a, b, c] = [triple[0], triple[1], triple[2]];
        let distance = 0.18_f32.min(a.distance(b) * 0.25).min(b.distance(c) * 0.25);
        let entry = b + (a - b).normalize_or_zero() * distance;
        let exit = b + (c - b).normalize_or_zero() * distance;
        rounded.push(entry);
        for step in 1..=4 {
            let t = step as f32 / 4.;
            rounded.push(entry.lerp(b, t).lerp(b.lerp(exit, t), t));
        }
    }
    rounded.push(end);
    rounded
}

fn travel_pose(path: &[Vec3], elapsed: f32) -> (Vec3, Quat) {
    let t = (elapsed / TRAVEL).clamp(0., 1.);
    let carry = smooth((t - 0.18) / 0.64);
    let total: f32 = path.windows(2).map(|p| p[0].distance(p[1])).sum();
    let mut remaining = carry * total;
    let mut at = *path.last().unwrap();
    for pair in path.windows(2) {
        let length = pair[0].distance(pair[1]);
        if length > 0. && remaining <= length {
            at = pair[0].lerp(pair[1], remaining / length);
            break;
        }
        remaining -= length;
    }
    let lift = smooth(t / 0.18) * (1. - smooth((t - 0.82) / 0.18));
    (
        at + Vec3::Y * 0.72 * lift,
        Quat::from_rotation_z(-0.07 * (PI * carry).sin() * lift)
            * Quat::from_rotation_y(0.06 * (PI * carry).sin()),
    )
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn animate(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<art::Art>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pieces: Query<(
        Entity,
        Ref<Surface>,
        &mut Transform,
        Option<&MeshMaterial3d<StandardMaterial>>,
    )>,
    mut cards: Query<(Entity, &motion::PhysicalCard, &mut Transform), Without<Surface>>,
) {
    let playback = table.presentation.map();
    if table.presentation.drawing() {
        return;
    }
    if let Some(beat) = playback.beats.front() {
        for (entity, card, mut pose) in &mut cards {
            if let Some(travel) = beat.cards.iter().find(|travel| travel.id == card.0) {
                let t = smooth(
                    (playback.elapsed - (TRAVEL - motion::CARD_DURATION)) / motion::CARD_DURATION,
                );
                pose.translation = travel.start.lerp(travel.end, t);
                commands.entity(entity).remove::<motion::CardMotion>();
            }
        }
    }
    for (entity, surface, mut transform, material) in &mut pieces {
        if let Some(travel) = playback
            .travelling()
            .find(|travel| travel.piece == *surface)
        {
            let (at, rotation) = travel_pose(&travel.path, playback.elapsed);
            transform.translation = at;
            transform.rotation = rotation;
        }
        match &*surface {
            Surface::Location(code) if playback.stage() == Some(Stage::Flip) => {
                let Some(reveal) = playback.reveals.iter().find(|reveal| reveal.code == *code)
                else {
                    continue;
                };
                if surface.is_added() {
                    motion::attach_reverse(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        entity,
                        art.card(code, true),
                        "Unrevealed location face",
                    );
                }
                let t = smooth(playback.elapsed / FLIP);
                transform.translation = reveal.at + Vec3::Y * (0.86 * (PI * t).sin());
                transform.rotation = Quat::from_rotation_z(-PI * (1. - t));
            }
            Surface::Clue(code) | Surface::Glint(code)
                if playback.stage() == Some(Stage::Clues) =>
            {
                let Some(reveal) = playback.reveals.iter().find(|reveal| reveal.code == *code)
                else {
                    continue;
                };
                let t = (playback.elapsed / CLUES).clamp(0., 1.);
                let settle = smooth(t / 0.65);
                let pulse = (PI * smooth((t - 0.35) / 0.65)).sin();
                if matches!(&*surface, Surface::Clue(_)) {
                    transform.translation =
                        layout::location_clue(reveal.at) + Vec3::Y * (0.5 * (1. - settle));
                    transform.scale = Vec3::splat(0.7 + 0.3 * settle + 0.06 * pulse);
                } else {
                    transform.scale = Vec3::splat(1. + 0.45 * t);
                    if let Some(mut material) =
                        material.and_then(|handle| materials.get_mut(&handle.0))
                    {
                        material.base_color = Color::srgba(0.75, 0.9, 0.45, 0.6 * pulse);
                    }
                }
            }
            _ => {}
        }
    }
}
