//! Anonymous, face-down deck choreography driven by the presentation clock.
use super::*;
use crate::gathering::DrawSource;

pub(super) const DURATION: f32 = 1.3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Operation {
    Shuffle {
        source: DrawSource,
        count: usize,
    },
    Recycle {
        source: DrawSource,
        count: usize,
        base: usize,
    },
    Take {
        source: DrawSource,
        remaining: usize,
        held: usize,
    },
}

impl Operation {
    pub(super) fn source(self) -> DrawSource {
        match self {
            Self::Shuffle { source, .. }
            | Self::Recycle { source, .. }
            | Self::Take { source, .. } => source,
        }
    }
    pub(super) fn duration(self) -> f32 {
        match self {
            Self::Shuffle { count: 0..=1, .. } => 0.3,
            Self::Shuffle { .. } => DURATION,
            _ => draws::DURATION,
        }
    }
}

#[derive(Component)]
pub(super) struct DeckSurface(pub DrawSource);

#[derive(Component)]
pub(super) struct Piece {
    operation: Operation,
    index: usize,
    groups: usize,
    start: usize,
    count: usize,
    top: bool,
}

fn smooth(value: f32) -> f32 {
    let t = value.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

impl Piece {
    fn pose(&self, elapsed: f32) -> Transform {
        let height = self.count as f32 * 0.0065;
        let surface = if self.top {
            height + 0.002
        } else {
            height / 2.
        };
        let deck = draws::deck_position(self.operation.source());
        let (at, yaw) = match self.operation {
            Operation::Shuffle { count: 0..=1, .. } => (
                deck + Vec3::Y
                    * (0.06 * (std::f32::consts::PI * (elapsed / 0.3).clamp(0., 1.)).sin()),
                0.,
            ),
            Operation::Shuffle { .. } => {
                let split = smooth((elapsed - 0.16) / 0.24);
                let interleave =
                    smooth((elapsed - 0.40 - self.index as f32 / self.groups as f32 * 0.18) / 0.30);
                let square = smooth((elapsed - 0.88) / 0.28);
                let settle = smooth((elapsed - 1.16) / 0.14);
                let side = if self.index.is_multiple_of(2) {
                    -1.
                } else {
                    1.
                };
                let spread = split * (1. - square);
                let height = self.start as f32 * 0.0065;
                let packet_height = height * (1. - 0.5 * split * (1. - interleave));
                (
                    deck + Vec3::new(
                        side * 0.70 * spread * (1. - 0.75 * interleave),
                        packet_height + 0.20 * smooth(elapsed / 0.16) * (1. - settle),
                        side * 0.14 * spread,
                    ),
                    side * 0.10 * spread * (1. - interleave),
                )
            }
            Operation::Recycle { source, base, .. } => {
                let start = match source {
                    DrawSource::Investigator(who) => layout::investigator_discard(who.index()),
                    DrawSource::Encounter => layout::ENCOUNTER_DISCARD,
                };
                let t = smooth(elapsed / draws::DURATION);
                (
                    start.lerp(deck + Vec3::Y * (base as f32 * 0.0065), t)
                        + Vec3::Y * (0.55 * (std::f32::consts::PI * t).sin()),
                    0.,
                )
            }
            Operation::Take {
                remaining, held, ..
            } => {
                let t = smooth(elapsed / draws::DURATION);
                let start = deck + Vec3::Y * ((remaining + 1) as f32 * 0.0065);
                let end = deck + Vec3::new(0., held.saturating_sub(1) as f32 * 0.0065, 2.4);
                (
                    start.lerp(end, t) + Vec3::Y * (0.65 * (std::f32::consts::PI * t).sin()),
                    0.,
                )
            }
        };
        Transform::from_translation(at + Vec3::Y * surface)
            .with_rotation(Quat::from_rotation_y(yaw))
    }
}

pub(super) fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    operation: Operation,
    elapsed: f32,
) {
    let count = match operation {
        Operation::Shuffle { count, .. } | Operation::Recycle { count, .. } => count,
        Operation::Take { .. } => 1,
    };
    if count == 0 {
        return;
    }
    let groups = if matches!(operation, Operation::Shuffle { .. }) {
        count.min(12)
    } else {
        1
    };
    let image = context.art.image(match operation.source() {
        DrawSource::Encounter => "card-back/encounter",
        _ => "card-back/player",
    });
    for index in 0..groups {
        let start = count * index / groups;
        let size = count * (index + 1) / groups - start;
        let packet = crate::tabletop_pieces::deck(
            commands,
            meshes,
            materials,
            image.clone(),
            size,
            Vec3::ZERO,
        );
        for (entity, top) in [(packet.body, false), (packet.top, true)] {
            let piece = Piece {
                operation,
                index,
                groups,
                start,
                count: size,
                top,
            };
            let pose = piece.pose(elapsed);
            commands.entity(entity).insert((
                Name::new("Face-down deck packet"),
                ScenePiece,
                Pickable::IGNORE,
                piece,
                pose,
            ));
        }
    }
}

pub(super) fn held(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
) {
    let Some(play) = &context.view.play else {
        return;
    };
    for held in &play.pending.held_draws {
        let at = draws::deck_position(DrawSource::Investigator(held.investigator)) + Vec3::Z * 2.4;
        let packet = crate::tabletop_pieces::deck(
            commands,
            meshes,
            materials,
            context.art.image("card-back/player"),
            held.count,
            at,
        );
        for entity in [packet.body, packet.top] {
            commands.entity(entity).insert((
                Name::new("Held face-down draw"),
                ScenePiece,
                Pickable::IGNORE,
            ));
        }
        let label = scene::label(
            commands,
            context,
            format!("Draw in progress · {} face down", held.count),
            at + Vec3::Z * 1.05,
            false,
        );
        commands.entity(label).insert(ScenePiece);
    }
}

pub(super) fn animate(
    table: Res<Table>,
    mut packets: Query<(&Piece, &mut Transform)>,
    mut decks: Query<(&DeckSurface, &mut Visibility)>,
) {
    let current = table.presentation.deck_motion();
    for (piece, mut transform) in &mut packets {
        if let Some((operation, elapsed)) = current
            && operation == piece.operation
        {
            *transform = piece.pose(elapsed);
        }
    }
    for (deck, mut visible) in &mut decks {
        *visible = if current.is_some_and(|(operation, _)| matches!(operation, Operation::Shuffle { source, .. } if source == deck.0)) { Visibility::Hidden } else { Visibility::Inherited };
    }
}
