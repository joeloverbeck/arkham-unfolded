//! Card-owned counts stay upright and travel with their physical card.
use super::*;
use crate::{
    gathering::UseKind,
    tabletop_counters::{self as counters, CounterKind, CounterVisual},
};

#[derive(Component)]
pub(super) struct Attached(pub Entity, pub Vec3);

pub(super) fn use_counter(kind: Option<UseKind>) -> Option<CounterKind> {
    match kind? {
        UseKind::Ammo => Some(CounterKind::Ammo),
        UseKind::Charge => Some(CounterKind::Charge),
        UseKind::Bounty => Some(CounterKind::Bounty),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn count(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    card: Entity,
    at: Vec3,
    offset: Vec3,
    kind: CounterKind,
    amount: u8,
) {
    if let Some((token, label)) = counters::spawn_count(
        commands,
        meshes,
        materials,
        CounterVisual {
            kind,
            amount,
            image: context.art.loaded_image(kind.image_role()),
            at: at + offset,
            scale: 1.,
        },
    ) {
        commands
            .entity(token)
            .insert((ScenePiece, Attached(card, offset)));
        commands.entity(label).insert(ScenePiece);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn damage(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    card: Entity,
    at: Vec3,
    amount: u8,
    capacity: Option<u8>,
    exhausted: bool,
) {
    let image = context.art.loaded_image("token/damage");
    let value = format!(
        "{amount}/{}",
        capacity.map_or_else(|| "?".into(), |max| max.to_string())
    );
    let caption = if image.is_some() {
        value
    } else {
        format!("Damage {value}")
    };
    let offset = Vec3::new(0., 0.16, if exhausted { -1.02 } else { -1.30 });
    let token =
        crate::tabletop_harm_tokens::spawn(commands, meshes, materials, image, at + offset, false);
    commands
        .entity(token)
        .insert((ScenePiece, Attached(card, offset)));
    let label = scene::label(commands, context, caption, at + offset, false);
    commands.entity(label).insert((
        motion::CardLabel(token, Vec3::new(0., 0.10, 0.24)),
        enemy_board::PersistentLabel,
    ));
}

pub(super) fn follow(
    cards: Query<&Transform, Without<Attached>>,
    mut tokens: Query<(&Attached, &mut Transform)>,
) {
    for (attached, mut at) in &mut tokens {
        if let Ok(card) = cards.get(attached.0) {
            at.translation = card.translation + attached.1;
        }
    }
}
