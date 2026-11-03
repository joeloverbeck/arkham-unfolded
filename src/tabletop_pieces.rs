//! The physical presentation established by the Party tabletop. Rules adapters
//! supply public facts and attach their own selection and scene-lifetime data.

use crate::{
    tabletop_counters as counters, tabletop_feedback as feedback, tabletop_layout as layout,
};
use bevy::prelude::*;

pub(crate) const CARD_WIDTH: f32 = 1.35;
pub(crate) const CARD_LENGTH: f32 = 1.89;
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);

#[derive(Component)]
pub(crate) struct WorldLabel(pub Vec3, pub bool);

/// Card titles wrap inside their projected lane instead of overlapping neighbors.
#[derive(Component)]
pub(crate) struct CardCaption;

impl CardCaption {
    pub(crate) fn width(camera: &Camera, transform: &GlobalTransform, at: Vec3) -> Option<f32> {
        // The closest card centers (the hand) are 1.93 world units apart.
        // Leave a gap on both sides, while retaining Party's 190px maximum.
        let left = camera
            .world_to_viewport(transform, at - Vec3::X * 0.875)
            .ok()?;
        let right = camera
            .world_to_viewport(transform, at + Vec3::X * 0.875)
            .ok()?;
        // At a sideways orbit the lane runs vertically on screen; its length
        // still provides readable wrapping space for upright UI text.
        Some(left.distance(right).clamp(1., 190.))
    }
}

pub(crate) fn label(
    commands: &mut Commands,
    value: impl Into<String>,
    at: Vec3,
    heading: bool,
    symbol_font: &Option<Handle<Font>>,
) -> Entity {
    commands
        .spawn((
            crate::tabletop_harm_text::ui(
                value,
                if heading { 18. } else { 13. },
                PAPER,
                symbol_font,
            ),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                width: px(190.),
                ..default()
            },
            Pickable::IGNORE,
            WorldLabel(at, heading),
        ))
        .id()
}

pub(crate) fn card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    image: Option<Handle<Image>>,
    transform: Transform,
    title: impl Into<String>,
) -> Entity {
    let entity = commands
        .spawn((
            Name::new(title.into()),
            Mesh3d(meshes.add(Plane3d::default().mesh().size(CARD_WIDTH, CARD_LENGTH))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if image.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(0.34, 0.31, 0.23)
                },
                base_color_texture: image,
                unlit: true,
                ..default()
            })),
            transform,
        ))
        .id();
    feedback::attach_card_feedback(
        commands,
        meshes,
        materials,
        entity,
        CARD_WIDTH,
        CARD_LENGTH,
        0.,
    );
    entity
}

pub(crate) struct Deck {
    pub body: Entity,
    pub top: Entity,
}

pub(crate) struct ChaosArea {
    pub bag: Entity,
    pub entities: Vec<Entity>,
}

pub(crate) fn chaos_area(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    bag_count: usize,
    curse_pool: Option<u8>,
    symbol_font: &Option<Handle<Font>>,
) -> ChaosArea {
    let bag_at = layout::test_bag();
    let bag = commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.75, 0.55))),
            MeshMaterial3d(materials.add(Color::srgb(0.22, 0.14, 0.12))),
            Transform::from_translation(bag_at),
        ))
        .id();
    let bag_label = label(
        commands,
        format!("Chaos bag\n{bag_count} tokens"),
        bag_at + Vec3::Z * 0.9,
        false,
        symbol_font,
    );
    let pool_at = bag_at - Vec3::X * 2.;
    let pool = commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.6, 0.12))),
            MeshMaterial3d(materials.add(Color::srgb(0.25, 0.16, 0.32))),
            Transform::from_translation(pool_at),
            Pickable::IGNORE,
        ))
        .id();
    let pool_label = label(
        commands,
        curse_pool.map_or_else(
            || "Curse pool\nCount not recorded".into(),
            |count| format!("Curse pool\n{count} tokens"),
        ),
        pool_at + Vec3::Z * 0.9,
        false,
        symbol_font,
    );
    ChaosArea {
        bag,
        entities: vec![bag, bag_label, pool, pool_label],
    }
}

pub(crate) fn deck(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    image: Option<Handle<Image>>,
    count: usize,
    at: Vec3,
) -> Deck {
    let height = count.max(1) as f32 * 0.0065;
    let body = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(CARD_WIDTH, height, CARD_LENGTH))),
            MeshMaterial3d(materials.add(PAPER)),
            Transform::from_translation(at + Vec3::Y * height / 2.),
            Pickable::IGNORE,
        ))
        .id();
    let top = commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(CARD_WIDTH, CARD_LENGTH))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if image.is_some() {
                    Color::WHITE
                } else {
                    Color::srgb(0.12, 0.19, 0.30)
                },
                base_color_texture: image,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(at + Vec3::Y * (height + 0.002)),
        ))
        .id();
    Deck { body, top }
}

pub(crate) struct ActionState {
    pub remaining: u8,
    pub allowance: u8,
    pub spent: u8,
    pub turn_ended: bool,
    pub additional: Option<bool>,
    pub additional_eligible: bool,
}

pub(crate) struct InvestigatorCounters {
    pub index: usize,
    pub resources: u8,
    pub clues: u8,
    pub damage: u8,
    pub health: u8,
    pub horror: u8,
    pub sanity: u8,
    pub actions: Option<ActionState>,
}

pub(crate) struct InvestigatorPieces {
    pub entities: Vec<Entity>,
    pub counters: Vec<Entity>,
}

pub(crate) fn investigator_counters(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    panel: InvestigatorCounters,
    image: impl Fn(&str) -> Option<Handle<Image>>,
) -> InvestigatorPieces {
    let mut entities = Vec::new();
    let mut selectable = Vec::new();
    for (kind, amount, role, at) in [
        (
            counters::CounterKind::Resource,
            panel.resources,
            "token/resource",
            layout::investigator_resource(panel.index),
        ),
        (
            counters::CounterKind::Clue,
            panel.clues,
            "token/clue",
            layout::investigator_clue(panel.index),
        ),
    ] {
        if let Some((token, caption)) = counters::spawn_count(
            commands,
            meshes,
            materials,
            counters::CounterVisual {
                kind,
                amount,
                image: image(role),
                at,
                scale: 0.85,
            },
        ) {
            entities.extend([token, caption]);
            selectable.push(token);
        }
    }
    if let Some(actions) = panel.actions {
        let class = layout::investigator_class(panel.index);
        let name = class.asset_name();
        entities.extend(counters::spawn_actions_with_allowance(
            commands,
            meshes,
            materials,
            counters::ActionRow {
                investigator: panel.index,
                remaining: actions.remaining,
                turn_ended: actions.turn_ended,
                additional: actions.additional,
                additional_eligible: actions.additional_eligible,
                class,
                icon: image(&format!("class/{name}/icon")),
                background: image(&format!("class/{name}/background")),
            },
            actions.allowance,
            actions.spent,
        ));
    }
    for (amount, capacity, horror, role, title) in [
        (panel.damage, panel.health, false, "token/damage", "Damage"),
        (panel.horror, panel.sanity, true, "token/horror", "Horror"),
    ] {
        let at = layout::investigator_harm(panel.index, horror);
        let art = image(role);
        let caption = layout::investigator_harm_caption(panel.index, horror, art.is_none());
        let value = if art.is_some() {
            format!("{amount}/{capacity}")
        } else {
            format!("{title} {amount}/{capacity}")
        };
        entities.push(crate::tabletop_harm_tokens::spawn(
            commands, meshes, materials, art, at, horror,
        ));
        entities.push(
            commands
                .spawn((
                    Text::new(value),
                    TextFont {
                        font_size: FontSize::Px(11.),
                        ..default()
                    },
                    TextColor(PAPER),
                    TextLayout::justify(Justify::Center),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(80.),
                        ..default()
                    },
                    WorldLabel(caption, false),
                    Pickable::IGNORE,
                ))
                .id(),
        );
    }
    InvestigatorPieces {
        entities,
        counters: selectable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{camera::primitives::MeshAabb, ecs::world::CommandQueue};

    #[test]
    fn party_decks_keep_card_sized_tops_and_count_driven_thickness() {
        let mut world = World::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        for count in [0, 1, 34, 45] {
            let mut queue = CommandQueue::default();
            let pieces = deck(
                &mut Commands::new(&mut queue, &world),
                &mut meshes,
                &mut materials,
                None,
                count,
                Vec3::ZERO,
            );
            queue.apply(&mut world);
            let body = meshes
                .get(&world.get::<Mesh3d>(pieces.body).unwrap().0)
                .unwrap()
                .get_aabb()
                .unwrap();
            let height = count.max(1) as f32 * 0.0065;
            assert!((body.half_extents.y * 2. - height).abs() < 0.0001);
            assert!((body.half_extents.x * 2. - 1.35).abs() < 0.0001);
            assert!((body.half_extents.z * 2. - 1.89).abs() < 0.0001);
            assert_eq!(
                world.get::<Transform>(pieces.top).unwrap().translation.y,
                height + 0.002
            );
            assert!(world.get::<Pickable>(pieces.top).is_none());
        }
    }
}
