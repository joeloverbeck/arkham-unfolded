//! Shared face-up encounter-pile rendering; adapters retain their own input sources.

use crate::encounter_discard::EncounterCard;
use bevy::prelude::*;

pub(crate) const ENCOUNTER_DISCARD_AT: Vec3 = crate::tabletop_layout::ENCOUNTER_DISCARD;

pub(crate) fn caption(cards: &[EncounterCard]) -> String {
    match cards {
        [] => "Encounter discard\nEmpty · 0 cards".into(),
        [card] => format!("Encounter discard · 1 card\n{}", card.title),
        _ => format!("Encounter discard\n{} cards", cards.len()),
    }
}

pub(crate) struct Appearance {
    pub size: Vec2,
    pub thickness: f32,
    pub image: Option<Handle<Image>>,
    pub fallback: Color,
}

pub(crate) fn spawn_card(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    card: &EncounterCard,
    index: usize,
    appearance: Appearance,
) -> Entity {
    let face = materials.add(StandardMaterial {
        base_color: if appearance.image.is_some() {
            Color::WHITE
        } else {
            appearance.fallback
        },
        base_color_texture: appearance.image,
        unlit: true,
        ..default()
    });
    let entity = commands
        .spawn((
            Name::new(format!("Encounter discard {}", card.id)),
            Mesh3d(
                meshes.add(
                    Plane3d::default()
                        .mesh()
                        .size(appearance.size.x, appearance.size.y),
                ),
            ),
            MeshMaterial3d(face),
            Transform::from_translation(
                ENCOUNTER_DISCARD_AT + Vec3::Y * index as f32 * appearance.thickness,
            ),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(
                    appearance.size.x,
                    appearance.thickness,
                    appearance.size.y,
                ))),
                MeshMaterial3d(materials.add(Color::srgb(0.57, 0.55, 0.49))),
                Transform::from_xyz(0.0, -appearance.thickness * 0.5, 0.0),
                Pickable::IGNORE,
            ));
        })
        .id();
    crate::tabletop_feedback::attach_card_feedback(
        commands,
        meshes,
        materials,
        entity,
        appearance.size.x,
        appearance.size.y,
        0.0,
    );
    entity
}
