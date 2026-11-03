//! Short visual moves between observed public card positions. Rules never use this clock.

use super::*;
use std::collections::BTreeMap;

#[derive(Component)]
pub(super) struct PhysicalCard(pub u16);

#[derive(Component, Clone, Copy)]
pub(super) struct CardMotion {
    start: Vec3,
    end: Vec3,
    elapsed: f32,
}

#[derive(Component)]
pub(super) struct CardLabel(pub Entity, pub Vec3);

#[derive(Default)]
pub(super) struct Origins {
    positions: BTreeMap<u16, Vec3>,
    continuing: BTreeMap<u16, CardMotion>,
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
                for (slot, committed) in test.committed.iter().enumerate() {
                    result.positions.insert(
                        committed.card.id,
                        layout::test_commitment(test.committed.len(), slot),
                    );
                }
            }
            for (slot, enemy) in play.scenario.enemies.iter().enumerate() {
                result
                    .positions
                    .insert(enemy.card.id, play::enemy_position(play, slot));
            }
            for card in &play.pending.resolving_cards {
                result.positions.insert(card.id, play::resolving_position());
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
}

pub(super) fn animate(
    mut commands: Commands,
    table: Res<Table>,
    time: Res<PresentationTime>,
    mut cards: Query<(
        Entity,
        Ref<PhysicalCard>,
        &mut Transform,
        Option<&mut CardMotion>,
    )>,
) {
    let paused = table.modal() || (table.is_presenting() && table.presentation_paused);
    for (entity, card, mut transform, motion) in &mut cards {
        if table.reduced_motion {
            if let Some(motion) = motion {
                transform.translation = motion.end;
                commands.entity(entity).remove::<CardMotion>();
            }
            continue;
        }
        if let Some(mut motion) = motion {
            if !paused {
                motion.elapsed = (motion.elapsed + time.delta_secs()).min(0.35);
            }
            let fraction = motion.elapsed / 0.35;
            // Smooth endpoints keep a stopped inspection stable.
            let blend = fraction * fraction * (3. - 2. * fraction);
            transform.translation = motion.start.lerp(motion.end, blend);
            if fraction >= 1. {
                commands.entity(entity).remove::<CardMotion>();
            }
        } else if card.is_added() {
            let end = transform.translation;
            let motion = table
                .motion_origins
                .continuing
                .get(&card.0)
                .filter(|motion| motion.end.abs_diff_eq(end, 0.001))
                .copied()
                .or_else(|| {
                    table
                        .motion_origins
                        .positions
                        .get(&card.0)
                        .filter(|start| !start.abs_diff_eq(end, 0.001))
                        .map(|start| CardMotion {
                            start: *start,
                            end,
                            elapsed: 0.,
                        })
                });
            if let Some(motion) = motion {
                let fraction = motion.elapsed / 0.35;
                transform.translation = motion
                    .start
                    .lerp(motion.end, fraction * fraction * (3. - 2. * fraction));
                commands.entity(entity).insert(motion);
            }
        }
    }
}
