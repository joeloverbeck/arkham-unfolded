//! Standard physical investigator standees for the tabletop.
//! Both prepared scenes use this same rounded base, holder and two-sided portrait.

use crate::tabletop_layout::InvestigatorClass;
use bevy::prelude::*;
use std::f32::consts::PI;

pub(crate) fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    image: Option<&Handle<Image>>,
    position: Vec3,
    class: InvestigatorClass,
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
            Transform::from_translation(position),
            Visibility::default(),
            Pickable::IGNORE,
        ))
        .with_children(|standee| {
            standee.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.52, 0.08))),
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
            let Some(image) = image else {
                // The caller retains the investigator's name and state label.
                standee.spawn((
                    Name::new("Investigator substitute; portrait unavailable"),
                    Mesh3d(meshes.add(Capsule3d::new(0.25, 0.72))),
                    MeshMaterial3d(base),
                    Transform::from_xyz(0.0, 0.75, 0.0),
                    Pickable::IGNORE,
                ));
                return;
            };

            const WIDTH: f32 = 1.0;
            const HEIGHT: f32 = WIDTH * 744.0 / 484.0;
            const BOTTOM: f32 = 0.20;
            let center = BOTTOM + HEIGHT / 2.0;
            let edge = materials.add(Color::srgb(0.11, 0.12, 0.12));
            standee.spawn((
                Mesh3d(meshes.add(Cuboid::new(WIDTH + 0.025, HEIGHT + 0.025, 0.03))),
                MeshMaterial3d(edge),
                Transform::from_xyz(0.0, center, 0.0),
                Pickable::IGNORE,
            ));
            standee.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.82, 0.12, 0.17))),
                MeshMaterial3d(base),
                Transform::from_xyz(0.0, 0.16, 0.0),
                Pickable::IGNORE,
            ));

            let face = meshes.add(Rectangle::new(WIDTH, HEIGHT));
            let portrait = materials.add(StandardMaterial {
                base_color_texture: Some(image.clone()),
                unlit: true,
                ..default()
            });
            // Separate outward-facing planes retain each face's normal UV order.
            // A single double-sided plane would mirror the portrait from behind.
            // The archive's grayscale back is deliberately not a rear texture:
            // camera direction must not suggest a change of investigator state.
            for (depth, yaw) in [(0.016, 0.0), (-0.016, PI)] {
                standee.spawn((
                    Mesh3d(face.clone()),
                    MeshMaterial3d(portrait.clone()),
                    Transform::from_xyz(0.0, center, depth)
                        .with_rotation(Quat::from_rotation_y(yaw)),
                    Pickable::IGNORE,
                ));
            }
        })
        .id()
}
