//! Presentation of an actual successful Study investigation. This adapter owns
//! the finite source identity; counter differences alone never establish a move.
use super::*;
use crate::tabletop_counters::{self as counters, CounterKind, CounterVisual};

const DURATION: f32 = 0.8;

#[derive(Debug, Clone)]
pub(super) struct Transfer {
    pub investigator: Investigator,
    pub previous_count: u8,
    amount: u8,
    start: Vec3,
    destination: Vec3,
    elapsed: f32,
}

impl Transfer {
    pub fn from_committed_discovery(before: &Observation, after: &Observation) -> Option<Self> {
        let pending = before.test.as_ref()?;
        let completed = after.test.as_ref()?;
        // Every represented Party test is an investigation of the revealed Study.
        // Require that actual transition and its exact source/recipient, rather
        // than interpreting history strings or pairing arbitrary counter deltas.
        if pending.result.is_some()
            || completed.result != Some(true)
            || pending.investigator != completed.investigator
            || after.revision <= before.revision
        {
            return None;
        }
        let investigator = completed.investigator;
        let previous = before.investigator(investigator);
        let current = after.investigator(investigator);
        let source = before
            .locations
            .iter()
            .find(|location| location.code == "01111")?;
        let destination_source = after
            .locations
            .iter()
            .find(|location| location.code == "01111")?;
        let amount = current.clues.checked_sub(previous.clues)?;
        if amount == 0
            || previous.location != "Study"
            || current.location != "Study"
            || !source.revealed
            || !destination_source.revealed
            || source.clues.checked_sub(destination_source.clues) != Some(amount)
        {
            return None;
        }
        let map =
            location_map::GatheringMap::new(&after.locations, Vec2::new(CARD_WIDTH, CARD_LENGTH))
                .ok()?;
        Some(Self {
            investigator,
            previous_count: previous.clues,
            amount,
            start: layout::location_clue(map.position("01111")),
            destination: layout::investigator_clue(investigator.index()),
            elapsed: 0.,
        })
    }
}

#[derive(Component)]
pub(super) struct MovingClue {
    label: Entity,
}

pub(super) fn animate(
    mut commands: Commands,
    mut table: ResMut<Table>,
    time: Res<PresentationTime>,
    art: Res<Art>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut moving: Query<(Entity, &MovingClue, &mut Transform)>,
) {
    let reduced = table.reduced_motion;
    let Some(transfer) = table.clue_transfer.as_mut() else {
        for (entity, clue, _) in &moving {
            commands.entity(entity).despawn();
            commands.entity(clue.label).try_despawn();
        }
        return;
    };
    transfer.elapsed += time.delta_secs();
    let fraction = if reduced {
        1.
    } else {
        (transfer.elapsed / DURATION).min(1.)
    };
    if fraction >= 1. {
        for (entity, clue, _) in &moving {
            commands.entity(entity).despawn();
            commands.entity(clue.label).try_despawn();
        }
        table.clue_transfer = None;
        table.dirty = true;
        return;
    }
    let at = counters::clue_transfer_position(transfer.start, transfer.destination, fraction);
    let scale = 1. - 0.15 * fraction;
    if moving.is_empty() {
        let (entity, label) = counters::spawn_count(
            &mut commands,
            &mut meshes,
            &mut materials,
            CounterVisual {
                kind: CounterKind::Clue,
                amount: transfer.amount,
                image: art.image("token/clue"),
                at,
                scale,
            },
        )
        .expect("a discovery moves at least one clue");
        commands.entity(entity).insert(MovingClue { label });
    } else {
        for (_, _, mut transform) in &mut moving {
            transform.translation = at;
            transform.scale = Vec3::splat(scale);
        }
    }
}
