//! The physical chaos-token disk and caption shared by the tabletops.
//!
//! Callers own placement, reveal motion and lifetime. The face is parented to
//! the disk so it follows the caller's transform throughout an animation.

use crate::{TokenFace, TokenStatus, tabletop::readable_text};
use bevy::prelude::*;

pub(crate) struct TokenVisual {
    pub face: TokenFace,
    pub status: TokenStatus,
    pub ordinal: Option<usize>,
    pub image: Option<Handle<Image>>,
    pub transform: Transform,
    pub label_offset: Vec3,
}

#[derive(Component)]
pub(crate) struct TokenLabel(Vec3);

/// Returns the disk and its separately projected label. Both belong to the
/// caller's scene lifetime; the face and cancellation marks belong to the disk.
pub(crate) fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    visual: TokenVisual,
) -> (Entity, Entity) {
    let canceled = visual.status == TokenStatus::Canceled;
    let color = match visual.status {
        TokenStatus::Pending => Color::srgb(0.86, 0.69, 0.37),
        TokenStatus::Resolved => Color::srgb(0.28, 0.62, 0.51),
        TokenStatus::Canceled => Color::srgb(0.37, 0.25, 0.28),
    };
    let mut disk = commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.46, 0.12))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.32,
            perceptual_roughness: 0.5,
            ..default()
        })),
        visual.transform,
        Pickable::IGNORE,
    ));
    if let Some(image) = visual.image {
        // The plane's UV top points toward -Z, upright from the default camera.
        disk.with_child((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(0.88, 0.88))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if canceled {
                    Color::srgb(0.35, 0.35, 0.35)
                } else {
                    Color::WHITE
                },
                base_color_texture: Some(image),
                alpha_mode: AlphaMode::Mask(0.5),
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.065, 0.0),
            Pickable::IGNORE,
        ));
    }
    if canceled {
        let crossed = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.55, 0.48),
            unlit: true,
            ..default()
        });
        for angle in [-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4] {
            disk.with_child((
                Mesh3d(meshes.add(Cuboid::new(0.75, 0.02, 0.065))),
                MeshMaterial3d(crossed.clone()),
                Transform::from_xyz(0.0, 0.085, 0.0).with_rotation(Quat::from_rotation_y(angle)),
                Pickable::IGNORE,
            ));
        }
    }
    let disk = disk.id();
    let status = match visual.status {
        TokenStatus::Pending => "Revealed",
        TokenStatus::Resolved => "Resolved",
        TokenStatus::Canceled => "Canceled",
    };
    let ordinal = visual.ordinal.map_or(String::new(), |n| format!("#{n}  "));
    let label = commands
        .spawn((
            Text::new(readable_text(format!(
                "{ordinal}{}\n{status}",
                visual.face.label()
            ))),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextLayout::justify(Justify::Center),
            TextColor(if canceled {
                Color::srgb(0.62, 0.71, 0.70)
            } else {
                Color::srgb(0.91, 0.89, 0.81)
            }),
            Node {
                position_type: PositionType::Absolute,
                width: px(84),
                padding: UiRect::axes(px(6), px(4)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.055, 0.90)),
            Pickable::IGNORE,
            TokenLabel(visual.transform.translation + visual.label_offset),
        ))
        .id();
    (disk, label)
}

pub(crate) fn position_labels(
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut labels: Query<(&TokenLabel, &mut Node, &mut Visibility)>,
) {
    for (anchor, mut node, mut visibility) in &mut labels {
        if let Ok(point) = camera.0.world_to_viewport(camera.1, anchor.0) {
            node.left = px(point.x - 42.0);
            node.top = px(point.y - 16.0);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{ecs::world::CommandQueue, mesh::VertexAttributeValues};

    #[test]
    fn artwork_sits_above_a_solid_disk_and_follows_its_reveal_transform() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut images = Assets::<Image>::default();
        let image = images.add(Image::default());
        let transform = Transform::from_xyz(2.0, 0.2, 3.0).with_scale(Vec3::splat(0.5));
        let (disk, label) = spawn(
            &mut Commands::new(&mut queue, &world),
            &mut meshes,
            &mut materials,
            TokenVisual {
                face: TokenFace::MinusTwo,
                status: TokenStatus::Pending,
                ordinal: None,
                image: Some(image.clone()),
                transform,
                label_offset: Vec3::new(0.0, -0.05, -1.0),
            },
        );
        queue.apply(&mut world);
        assert_eq!(*world.get::<Transform>(disk).unwrap(), transform);
        let mesh = meshes.get(&world.get::<Mesh3d>(disk).unwrap().0).unwrap();
        let VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!("disk has no vertex positions");
        };
        let min_y = positions.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let max_y = positions
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((max_y - min_y - 0.12).abs() < 0.0001);
        let rim = materials
            .get(
                &world
                    .get::<MeshMaterial3d<StandardMaterial>>(disk)
                    .unwrap()
                    .0,
            )
            .unwrap();
        assert!(!rim.unlit);
        assert!(rim.base_color_texture.is_none());
        let children = world.get::<Children>(disk).unwrap();
        assert_eq!(children.len(), 1);
        let face = children[0];
        assert!(world.get::<Transform>(face).unwrap().translation.y > max_y);
        let face_material = materials
            .get(
                &world
                    .get::<MeshMaterial3d<StandardMaterial>>(face)
                    .unwrap()
                    .0,
            )
            .unwrap();
        assert_eq!(face_material.base_color_texture, Some(image));
        assert_eq!(face_material.base_color, Color::WHITE);
        assert!(face_material.unlit);
        assert_eq!(face_material.alpha_mode, AlphaMode::Mask(0.5));
        assert_eq!(world.get::<Text>(label).unwrap().0, "-2\nRevealed");
        assert!(world.get::<BackgroundColor>(label).is_some());
        assert_eq!(
            world.get::<TokenLabel>(label).unwrap().0,
            Vec3::new(2.0, 0.15, 2.0)
        );
    }
}
