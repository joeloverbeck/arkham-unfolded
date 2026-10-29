//! Shared assertions over the entities emitted by each production scene.

use crate::tabletop_layout::InvestigatorClass;
use bevy::{camera::primitives::MeshAabb, mesh::VertexAttributeValues, prelude::*};

pub(super) fn assert_standees(
    world: &mut World,
    expected: &[(InvestigatorClass, Option<Handle<Image>>)],
) {
    let roots: Vec<_> = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .filter_map(|(entity, name)| (name.as_str() == "Investigator standee").then_some(entity))
        .collect();
    let meshes = world.resource::<Assets<Mesh>>();
    let materials = world.resource::<Assets<StandardMaterial>>();
    for (class, portrait) in expected {
        let parts = roots
            .iter()
            .filter_map(|root| world.get::<Children>(*root))
            .map(|children| {
                children
                    .iter()
                    .filter_map(|child| {
                        Some((
                            meshes.get(&world.get::<Mesh3d>(child)?.0)?,
                            materials
                                .get(&world.get::<MeshMaterial3d<StandardMaterial>>(child)?.0)?,
                            world.get::<Transform>(child)?,
                            world.get::<Name>(child),
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .find(|parts| match portrait {
                Some(image) => parts
                    .iter()
                    .any(|(_, material, _, _)| material.base_color_texture.as_ref() == Some(image)),
                None => parts.iter().any(|(_, material, _, name)| {
                    material.base_color == class.color()
                        && name.is_some_and(|name| name.as_str().contains("portrait unavailable"))
                }),
            })
            .unwrap_or_else(|| {
                panic!("{class:?} must have its own rendered standee or substitute")
            });

        assert!(
            parts.iter().any(|(mesh, material, transform, _)| {
                let Some(bounds) = mesh.get_aabb() else {
                    return false;
                };
                let size = bounds.half_extents * 2.0;
                if material.base_color != class.color()
                    || transform.translation.y > 0.15
                    || size.x < 0.5
                    || size.z < 0.5
                    || !(0.015..=0.2).contains(&size.y)
                {
                    return false;
                }
                let Some(VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    return false;
                };
                // The horizontal outline is rounded, not a zero-depth portrait
                // or a rectangular slab that merely has the same bounding box.
                positions.iter().all(|p| {
                    (p[0] / bounds.half_extents.x).powi(2) + (p[2] / bounds.half_extents.z).powi(2)
                        <= 1.05
                })
            }),
            "{class:?} needs a thick rounded base in its class color"
        );

        let Some(image) = portrait else {
            assert!(
                parts.iter().any(|(mesh, material, _, name)| {
                    let bounds = mesh.get_aabb().unwrap();
                    material.base_color == class.color()
                        && name.is_some_and(|name| name.as_str().contains("portrait unavailable"))
                        && bounds.half_extents.y > bounds.half_extents.x
                        && bounds.half_extents.z > 0.0
                }),
                "missing art must retain a dimensional investigator substitute"
            );
            continue;
        };

        assert!(
            parts.iter().any(|(mesh, material, transform, _)| {
                let Some(bounds) = mesh.get_aabb() else {
                    return false;
                };
                let size = bounds.half_extents * 2.0;
                if material.base_color != class.color()
                    || !(0.1..0.4).contains(&transform.translation.y)
                    || size.x < 0.4
                    || !(0.04..0.3).contains(&size.y)
                    || size.z < 0.05
                    || size.x < size.z * 2.0
                {
                    return false;
                }
                let Some(VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    return false;
                };
                positions.iter().all(|p| {
                    (Vec3::from_array(*p).abs() - Vec3::from(bounds.half_extents))
                        .abs()
                        .max_element()
                        < 0.0001
                })
            }),
            "{class:?} needs a solid rectangular holder above its base"
        );
        for direction in [-1.0, 1.0] {
            assert!(
                parts.iter().any(|(mesh, material, transform, _)| {
                    let Some(bounds) = mesh.get_aabb() else {
                        return false;
                    };
                    let normal = transform.rotation * Vec3::Z;
                    material.base_color_texture.as_ref() == Some(image)
                        && bounds.half_extents.x > 0.0
                        && bounds.half_extents.y > bounds.half_extents.x
                        && bounds.half_extents.z.abs() < 0.0001
                        && normal.z * direction > 0.99
                        && transform.translation.z * direction > 0.0
                }),
                "{class:?} needs an outward portrait face in direction {direction}"
            );
        }
    }
}
