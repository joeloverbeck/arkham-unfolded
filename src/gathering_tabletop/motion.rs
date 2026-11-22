//! Short visual moves between observed public card positions. Rules never use this clock.

use super::*;
use std::collections::BTreeMap;
use unfolded_tabletop::card_motion::{CardMove, Departure, Lift, Turn};

pub(super) const CARD_DURATION: f32 = 0.35;

#[derive(Component)]
pub(super) struct PhysicalCard(pub u16);

#[derive(Component, Clone, Copy)]
pub(super) struct CardMotion {
    path: CardMove,
    elapsed: f32,
    transfer: Option<draws::Transfer>,
}

#[derive(Component)]
pub(super) struct CardLabel(pub Entity, pub Vec3);

#[derive(Default)]
pub(super) struct Origins {
    positions: BTreeMap<u16, Vec3>,
    continuing: BTreeMap<u16, CardMotion>,
    transfers: BTreeMap<u16, draws::Transfer>,
}

impl Origins {
    pub(super) fn from_view(view: Option<&Observation>) -> Self {
        let mut result = Self::default();
        let Some(view) = view else { return result };
        for (index, player) in view.investigators.iter().enumerate() {
            for (slot, card) in player.hand.iter().enumerate() {
                result.positions.insert(
                    card.id,
                    layout::hand_position(index, player.hand.len(), slot),
                );
            }
            if view.play.is_none() {
                for (slot, card) in player.permanents.iter().enumerate() {
                    result
                        .positions
                        .insert(card.id, layout::investigator_asset(index, slot));
                }
            }
        }
        if let Some(play) = &view.play {
            if let Some(pending) = &play.pending_revelation {
                result.transfers.insert(
                    pending.card.id,
                    draws::Transfer {
                        id: pending.card.id,
                        start: None,
                        source: if pending.card.weakness {
                            crate::gathering::DrawSource::Investigator(pending.drawn_by)
                        } else {
                            crate::gathering::DrawSource::Encounter
                        },
                        turn: draws::Turn::FaceUp,
                    },
                );
            }
            for (index, player) in play.players.iter().enumerate() {
                for (slot, asset) in player.assets.iter().enumerate() {
                    result
                        .positions
                        .insert(asset.card.id, layout::investigator_asset(index, slot));
                }
                for (slot, card) in player.threats.iter().enumerate() {
                    result
                        .positions
                        .insert(card.id, layout::investigator_threat(index, slot));
                }
                for card in &player.discard {
                    result
                        .positions
                        .insert(card.id, layout::investigator_discard(index));
                }
                for card in &player.removed {
                    result
                        .positions
                        .insert(card.id, layout::investigator_discard(index) + Vec3::Z * 3.);
                }
            }
            if let Some(test) = &play.test {
                if let Some(source) = &test.source_card {
                    result
                        .positions
                        .entry(source.id)
                        .or_insert(layout::test_source());
                }
                let cards = play::test_resolving_cards(play);
                let count = test.committed.len() + cards.len();
                for (slot, committed) in test.committed.iter().enumerate() {
                    result
                        .positions
                        .insert(committed.card.id, layout::test_commitment(count, slot));
                }
                for (slot, card) in cards.iter().enumerate() {
                    result.positions.insert(
                        card.id,
                        layout::test_commitment(count, test.committed.len() + slot),
                    );
                }
            }
            for (slot, enemy) in play.scenario.enemies.iter().enumerate() {
                result
                    .positions
                    .insert(enemy.card.id, play::enemy_position(play, slot));
            }
            for card in play
                .scenario
                .locations
                .iter()
                .filter_map(|room| room.fog.as_ref())
            {
                result.positions.insert(
                    card.id,
                    sideboard::LOCATION_CARDS.position(play, &controls::Source::Card(card.id)),
                );
            }
            if let Some(lita) = &play.scenario.lita_at_parlor {
                result.positions.insert(
                    lita.id,
                    sideboard::LOCATION_CARDS.position(play, &controls::Source::Lita),
                );
            }
            if let Some(tray) = trays::Layout::for_play(play) {
                result.positions.extend(tray.positions());
            }
            if let Some(sealed) = &play.premonition {
                result.positions.insert(
                    sealed.card.id,
                    trays::sealed_position(play, sealed.controller),
                );
            }
            for card in &play.encounter_discard {
                result.positions.insert(card.id, layout::ENCOUNTER_DISCARD);
            }
            for card in &play.scenario.victory {
                result.positions.insert(card.id, layout::VICTORY_DISPLAY);
            }
        }
        result
    }

    pub(super) fn remember(&mut self, id: u16, at: Vec3, motion: Option<&CardMotion>) {
        self.positions.insert(id, at);
        if let Some(motion) = motion {
            self.continuing.insert(id, *motion);
        }
    }

    pub(super) fn contains(&self, id: u16) -> bool {
        self.positions.contains_key(&id)
    }

    pub(super) fn transfer(&mut self, transfer: draws::Transfer) {
        self.transfers.insert(transfer.id, transfer);
    }

    /// A map beat already carried this card to its committed destination.
    pub(super) fn arrived(&mut self, id: u16, at: Vec3) {
        self.positions.insert(id, at);
        self.continuing.remove(&id);
        self.transfers.remove(&id);
    }
}

impl CardMotion {
    /// A plain move slides flat to `end`. A draw or return first slides clear
    /// of its pile, rises to a height that grows with the distance and sways by
    /// a fixed amount, and turns face up or face down on the way, unless its
    /// transfer keeps the card face up.
    fn new(start: Vec3, end: Transform, transfer: Option<draws::Transfer>) -> Self {
        let mut path = CardMove::new(
            start,
            end,
            if transfer.is_some() {
                draws::DURATION
            } else {
                CARD_DURATION
            },
        );
        if let Some(transfer) = transfer {
            path = path
                .lifted(Lift {
                    height: (0.45 + start.distance(end.translation) * 0.035).min(1.2),
                    yaw: 0.10,
                    roll: 0.07,
                })
                .departing(Departure {
                    offset: Vec3::X * 0.22,
                    share: 0.12,
                });
            // Cards lie in XZ. Turn around the long axis so both faces keep their top edge.
            let turn = |from, to| Turn {
                axis: Vec3::Z,
                from,
                to,
                begins: 0.2,
                lasts: 0.58,
            };
            match transfer.turn {
                draws::Turn::Up => path = path.turned(turn(-std::f32::consts::PI, 0.)),
                draws::Turn::Down => path = path.turned(turn(0., std::f32::consts::PI)),
                draws::Turn::FaceUp => {}
            }
        }
        Self {
            path,
            elapsed: 0.,
            transfer,
        }
    }

    fn duration(&self) -> f32 {
        self.path.duration
    }

    fn transform(&self) -> Transform {
        self.path.pose(self.elapsed)
    }
}

fn attach_back(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &art::Art,
    parent: Entity,
    source: crate::gathering::DrawSource,
) {
    let image = art.image(match source {
        crate::gathering::DrawSource::Investigator(_) => "card-back/player",
        crate::gathering::DrawSource::Encounter => "card-back/encounter",
    });
    attach_reverse(
        commands,
        meshes,
        materials,
        parent,
        image,
        "Drawn card back",
    );
}

pub(super) fn attach_reverse(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
    image: Option<Handle<Image>>,
    name: &'static str,
) {
    use crate::tabletop_pieces::{CARD_LENGTH, CARD_WIDTH, card_shape};
    commands.spawn((
        Name::new(name),
        Mesh3d(meshes.add(card_shape::face(CARD_WIDTH, CARD_LENGTH))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: image,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0., -0.007, 0.)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::PI)),
        Pickable::IGNORE,
        ChildOf(parent),
    ));
    commands.spawn((
        Name::new("Drawn card edge"),
        Mesh3d(meshes.add(card_shape::body(CARD_WIDTH, 0.005, CARD_LENGTH))),
        MeshMaterial3d(materials.add(PAPER)),
        Transform::from_xyz(0., -0.0035, 0.),
        Pickable::IGNORE,
        ChildOf(parent),
    ));
}

pub(super) fn animate(
    mut commands: Commands,
    table: Res<Table>,
    time: Res<PresentationTime>,
    art: Res<art::Art>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cards: Query<(
        Entity,
        Ref<PhysicalCard>,
        &mut Transform,
        Option<&mut CardMotion>,
    )>,
) {
    let paused = table.motion_blocked();
    for (entity, card, mut transform, motion) in &mut cards {
        if table.reduced_motion {
            if let Some(motion) = motion {
                transform.translation = motion.path.to.translation;
                transform.rotation = motion.path.to.rotation;
                commands.entity(entity).remove::<CardMotion>();
            }
            continue;
        }
        if let Some(mut motion) = motion {
            if !paused && !table.sealed.holds(card.0) {
                motion.elapsed = (motion.elapsed + time.delta_secs()).min(motion.duration());
            }
            *transform = motion.transform();
            if motion.elapsed >= motion.duration() {
                commands.entity(entity).remove::<CardMotion>();
            }
        } else if card.is_added() {
            let rest = *transform;
            let end = rest.translation;
            let motion = table
                .motion_origins
                .continuing
                .get(&card.0)
                .filter(|motion| motion.path.to.translation.abs_diff_eq(end, 0.001))
                .copied()
                .or_else(|| {
                    let transfer = table.motion_origins.transfers.get(&card.0).copied();
                    let start = transfer
                        .and_then(|transfer| transfer.start)
                        .or_else(|| table.motion_origins.positions.get(&card.0).copied());
                    start
                        .filter(|start| !start.abs_diff_eq(end, 0.001))
                        .map(|start| CardMotion::new(start, rest, transfer))
                });
            if let Some(motion) = motion {
                *transform = motion.transform();
                if let Some(transfer) = motion.transfer {
                    attach_back(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &art,
                        entity,
                        transfer.source,
                    );
                }
                commands.entity(entity).insert(motion);
            } else if let Some(source) = table.presentation.returning(card.0) {
                transform.rotation = Quat::from_rotation_z(std::f32::consts::PI) * rest.rotation;
                attach_back(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &art,
                    entity,
                    source,
                );
            }
        }
    }
}
