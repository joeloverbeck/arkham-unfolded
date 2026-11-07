//! Solid damage/horror counters shared by the existing tabletop consumers.
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct HarmToken;

// Match the 0.31 × 0.8 radius and 0.12 × 0.8 thickness of ammo/bounty counters.
pub(crate) const RADIUS: f32 = 0.248;
const HEIGHT: f32 = 0.096;
const ART_SCALE: f32 = RADIUS / 0.35;

/// The caller owns totals, accessible names, scene lifetime and any animation.
pub(crate) fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    image: Option<Handle<Image>>,
    at: Vec3,
    horror: bool,
) -> Entity {
    let mut token = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(RADIUS, HEIGHT))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: if horror {
                Color::srgb(0.13, 0.36, 0.68)
            } else {
                Color::srgb(0.66, 0.12, 0.10)
            },
            perceptual_roughness: 0.65,
            ..default()
        })),
        Transform::from_translation(at),
        HarmToken,
        Pickable::IGNORE,
    ));
    if let Some(image) = image {
        let size = if horror {
            Vec2::new(0.56, 0.46)
        } else {
            Vec2::new(0.46, 0.56)
        } * ART_SCALE;
        token.with_child((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(size.x, size.y))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(image),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, HEIGHT / 2.0 + 0.005, 0.0),
            Pickable::IGNORE,
        ));
    }
    token.id()
}
