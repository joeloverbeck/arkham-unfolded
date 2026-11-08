//! Card-owned counts stay upright and travel with their physical card.
use super::*;
use crate::{
    gathering::UseKind,
    tabletop_counters::{self as counters, CounterKind, CounterVisual},
};

#[derive(Component)]
pub(super) struct Attached(pub Entity, pub Vec3);

pub(super) fn harm_offset(exhausted: bool) -> Vec3 {
    let length = if exhausted {
        crate::tabletop_pieces::CARD_WIDTH
    } else {
        crate::tabletop_pieces::CARD_LENGTH
    };
    // The smaller disk clears the card edge; the caption keeps its prior gap.
    Vec3::new(0., 0.16, -length / 2. - 0.275)
}

pub(super) fn use_counter(kind: Option<UseKind>) -> Option<CounterKind> {
    match kind? {
        UseKind::Ammo => Some(CounterKind::Ammo),
        UseKind::Charge => Some(CounterKind::Charge),
        UseKind::Bounty => Some(CounterKind::Bounty),
        UseKind::Supply => Some(CounterKind::Supply),
        UseKind::Key => Some(CounterKind::Key),
        UseKind::Resource => Some(CounterKind::Resource),
        UseKind::Offering => Some(CounterKind::Offering),
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
    if let Some((token, label)) = counters::spawn_asset_count(
        commands,
        meshes,
        materials,
        CounterVisual {
            kind,
            amount,
            image: context.art.image(kind.image_role()),
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
    harm(
        commands,
        meshes,
        materials,
        context,
        card,
        at,
        amount,
        capacity,
        false,
        harm_offset(exhausted),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn harm(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    context: &CardRender<'_>,
    card: Entity,
    at: Vec3,
    amount: u8,
    capacity: Option<u8>,
    horror: bool,
    offset: Vec3,
) {
    let image = context.art.image(if horror {
        "token/horror"
    } else {
        "token/damage"
    });
    let value = format!(
        "{amount}/{}",
        capacity.map_or_else(|| "?".into(), |max| max.to_string())
    );
    let accessible = format!("{} {value}", if horror { "Horror" } else { "Damage" });
    let token =
        crate::tabletop_harm_tokens::spawn(commands, meshes, materials, image, at + offset, horror);
    commands
        .entity(token)
        .insert((ScenePiece, Attached(card, offset)));
    let label = scene::label(commands, context, value, at + offset, false);
    commands.entity(label).insert((
        AccessibleLabel::new(accessible),
        motion::CardLabel(token, Vec3::new(0., 0.10, 0.16)),
        enemy_board::PersistentLabel,
        TextLayout::justify(Justify::Center).with_no_wrap(),
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(3.), px(1.)),
            border_radius: BorderRadius::all(px(2.)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.02, 0.02, 0.96)),
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
