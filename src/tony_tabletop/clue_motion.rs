//! A visual transfer for Tony's committed Study discovery, never a game-state change.

use super::*;
use crate::tabletop_counters::{self as counters, CounterKind, CounterVisual};

#[derive(Component)]
pub(super) struct TravelingClue;

#[derive(Component)]
pub(super) struct TravelingClueLabel;

#[allow(clippy::too_many_arguments)]
pub(super) fn synchronize(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<Art>,
    tokens: Query<Entity, With<TravelingClue>>,
    labels: Query<Entity, With<TravelingClueLabel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rendered: Local<Option<(u64, u8)>>,
) {
    let transfer = table
        .replay
        .is_none()
        .then(|| table.presentation.clue())
        .flatten();
    let key = transfer.map(|clue| (table.view().revision, clue.amount));
    if *rendered == key {
        return;
    }
    for entity in tokens.iter().chain(labels.iter()) {
        commands.entity(entity).despawn();
    }
    *rendered = key;
    if let Some(clue) = transfer {
        let (token, label) = counters::spawn_count(
            &mut commands,
            &mut meshes,
            &mut materials,
            CounterVisual {
                kind: CounterKind::Clue,
                amount: clue.amount,
                image: art.handles.get("token/clue").cloned(),
                at: scene::study_clue_at(),
                scale: 1.0,
            },
        )
        .expect("a discovery transfers at least one clue");
        // The scene rebuild owns stationary counters. These transient roots
        // survive a camera/menu rebuild and use the staging clock's progress.
        commands.entity(token).insert(TravelingClue);
        commands.entity(label).insert(TravelingClueLabel);
    }
}

pub(super) fn animate(table: Res<Table>, mut tokens: Query<&mut Transform, With<TravelingClue>>) {
    let progress = table.presentation.clue_progress();
    for mut transform in &mut tokens {
        transform.translation = counters::clue_transfer_position(
            scene::study_clue_at(),
            layout::investigator_clue(0),
            progress,
        );
        transform.scale = Vec3::splat(1.0 - 0.15 * progress);
    }
}

#[cfg(test)]
pub(super) fn traveling_count(world: &mut World) -> usize {
    world
        .query_filtered::<(), With<TravelingClue>>()
        .iter(world)
        .count()
}
