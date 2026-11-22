//! Standard physical investigator standees for the tabletop.
//! Both prepared scenes use this same rounded base, holder and two-sided portrait.

use crate::tabletop_layout::InvestigatorClass;
use bevy::prelude::*;
use unfolded_tabletop::slab::{self, Slab};

const BASE_RADIUS_UNSCALED: f32 = 0.52;

pub(crate) fn radius_for(card_size: Vec2) -> f32 {
    card_size.y * 0.21
}

pub(crate) fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    image: Option<&Handle<Image>>,
    position: Vec3,
    class: InvestigatorClass,
    card_size: Vec2,
) -> Entity {
    let color = class.color();
    let base = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.75,
        ..default()
    });
    let trim = materials.add(StandardMaterial {
        base_color: color.mix(&Color::WHITE, 0.25),
        metallic: 0.4,
        perceptual_roughness: 0.45,
        ..default()
    });
    commands
        .spawn((
            Name::new("Investigator standee"),
            Transform::from_translation(position)
                .with_scale(Vec3::splat(radius_for(card_size) / BASE_RADIUS_UNSCALED)),
            Visibility::default(),
            Pickable::IGNORE,
        ))
        .with_children(|standee| {
            standee.spawn((
                Mesh3d(meshes.add(Cylinder::new(BASE_RADIUS_UNSCALED, 0.08))),
                MeshMaterial3d(trim),
                Transform::from_xyz(0.0, 0.04, 0.0),
                Pickable::IGNORE,
            ));
            standee.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.49, 0.045))),
                MeshMaterial3d(base.clone()),
                Transform::from_xyz(0.0, 0.09, 0.0),
                Pickable::IGNORE,
            ));
            const WIDTH: f32 = 1.0;
            const HEIGHT: f32 = WIDTH * 744.0 / 484.0;
            const BOTTOM: f32 = 0.20;
            let center = BOTTOM + HEIGHT / 2.0;
            let edge = materials.add(Color::srgb(0.11, 0.12, 0.12));
            standee.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.82, 0.12, 0.17))),
                MeshMaterial3d(base),
                Transform::from_xyz(0.0, 0.16, 0.0),
                Pickable::IGNORE,
            ));

            let face = meshes.add(Rectangle::new(WIDTH, HEIGHT));
            let portrait = materials.add(StandardMaterial {
                base_color_texture: image.cloned(),
                unlit: true,
                ..default()
            });
            // Use the same portrait on both sides so that turning the camera
            // cannot suggest a change of investigator state.
            slab::spawn(
                standee,
                Slab {
                    body: meshes.add(Cuboid::new(WIDTH + 0.025, HEIGHT + 0.025, 0.03)),
                    face,
                    body_material: edge,
                    face_material: portrait,
                    center: Vec3::new(0.0, center, 0.0),
                    face_offset: 0.016,
                    pickable: Pickable::IGNORE,
                },
            );
        })
        .id()
}
