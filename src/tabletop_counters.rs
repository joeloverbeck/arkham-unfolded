//! Physical totals and action allowances. Callers supply state and own lifetimes.
use crate::tabletop_layout::{self as layout, InvestigatorClass};
use bevy::{asset::RenderAssetUsages, prelude::*, render::render_resource::PrimitiveTopology};

pub(crate) struct CounterPlugin;

impl Plugin for CounterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, position_labels);
    }
}

/// The finite artwork vocabulary consumed by this shared presentation.
pub(crate) fn asset_roles() -> impl Iterator<Item = String> {
    ["token/clue", "token/resource"]
        .into_iter()
        .map(str::to_owned)
        .chain(
            [
                InvestigatorClass::Guardian,
                InvestigatorClass::Seeker,
                InvestigatorClass::Rogue,
                InvestigatorClass::Mystic,
                InvestigatorClass::Survivor,
                InvestigatorClass::Neutral,
            ]
            .into_iter()
            .flat_map(|class| {
                ["icon", "background"]
                    .into_iter()
                    .map(move |layer| format!("class/{}/{layer}", class.asset_name()))
            }),
        )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CounterKind {
    Clue,
    Resource,
}

impl CounterKind {
    fn size(self) -> f32 {
        match self {
            Self::Clue => 0.8,
            Self::Resource => 0.9,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Clue => "Clues",
            Self::Resource => "Resources",
        }
    }
}

#[derive(Component, Debug)]
pub(crate) struct Counter {
    pub kind: CounterKind,
    pub amount: u8,
}

impl Counter {
    fn caption(&self, artwork: bool) -> String {
        if artwork {
            self.amount.to_string()
        } else {
            format!("{} {}", self.kind.name(), self.amount)
        }
    }
}

pub(crate) struct CounterVisual {
    pub kind: CounterKind,
    pub amount: u8,
    pub image: Option<Handle<Image>>,
    pub at: Vec3,
    pub scale: f32,
}

#[derive(Component)]
pub(crate) struct CounterLabel {
    token: Entity,
    offset: Vec3,
}

/// Zero totals have neither a token nor a caption. A label follows its physical root.
pub(crate) fn spawn_count(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    visual: CounterVisual,
) -> Option<(Entity, Entity)> {
    if visual.amount == 0 {
        return None;
    }
    let counter = Counter {
        kind: visual.kind,
        amount: visual.amount,
    };
    let caption = counter.caption(visual.image.is_some());
    let token = spawn_body(
        commands,
        meshes,
        materials,
        visual.kind,
        visual.image,
        visual.at,
        visual.scale,
    );
    commands.entity(token).insert(counter);
    let label = spawn_label(
        commands,
        token,
        caption,
        Vec3::new(
            0.0,
            0.04,
            match visual.kind {
                CounterKind::Clue => 0.336,
                CounterKind::Resource => 0.403,
            },
        ),
        None,
    );
    commands.entity(label).insert((
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(3.0), px(1.0)),
            border_radius: BorderRadius::all(px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.015, 0.02, 0.02)),
    ));
    Some((token, label))
}

#[allow(clippy::too_many_arguments)]
fn spawn_body(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    kind: CounterKind,
    image: Option<Handle<Image>>,
    at: Vec3,
    scale: f32,
) -> Entity {
    let (mesh, color) = match kind {
        CounterKind::Clue => (
            Mesh::from(Cylinder::new(0.31, 0.12)),
            Color::srgb(0.13, 0.30, 0.22),
        ),
        CounterKind::Resource => (resource_body(), Color::srgb(0.33, 0.20, 0.10)),
    };
    // Resize the mesh rather than the root: traveling counters animate their
    // root scale independently and must retain the same physical token size.
    let size = kind.size();
    let mut root = commands.spawn((
        Name::new(kind.name()),
        Mesh3d(meshes.add(mesh.scaled_by(Vec3::splat(size)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.7,
            ..default()
        })),
        Transform::from_translation(at).with_scale(Vec3::splat(scale)),
        Pickable::IGNORE,
    ));
    if let Some(image) = image {
        let (face, rotation) = match kind {
            CounterKind::Clue => (
                Mesh::from(Circle::new(0.31)),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
            CounterKind::Resource => (
                Plane3d::default().mesh().size(0.58, 0.654).build(),
                Quat::IDENTITY,
            ),
        };
        root.with_child((
            Mesh3d(meshes.add(face.scaled_by(Vec3::splat(size)))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(image),
                unlit: true,
                // The selected clue atlas stores zero alpha through its
                // emblem, not a usable transparency mask. Preserve that RGB;
                // the circular mesh clips the square image's white corners.
                alpha_mode: match kind {
                    CounterKind::Clue => AlphaMode::Opaque,
                    CounterKind::Resource => AlphaMode::Blend,
                },
                ..default()
            })),
            Transform::from_xyz(0.0, 0.062 * size, 0.0).with_rotation(rotation),
            Pickable::IGNORE,
        ));
    }
    root.id()
}

// The resource is a shaped crate, not a circular coin. Coordinates follow the
// selected 172x194 source silhouette; the planar artwork covers the solid top.
fn resource_body() -> Mesh {
    let outline = [
        Vec2::new(-0.0232, -0.327),
        Vec2::new(0.029, -0.327),
        Vec2::new(0.29, -0.157),
        Vec2::new(0.29, 0.150),
        Vec2::new(0.0174, 0.327),
        Vec2::new(-0.029, 0.327),
        Vec2::new(-0.29, 0.157),
        Vec2::new(-0.29, -0.157),
    ];
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for index in 0..outline.len() {
        let a = outline[index];
        let b = outline[(index + 1) % outline.len()];
        let top_a = [a.x, 0.06, a.y];
        let top_b = [b.x, 0.06, b.y];
        let bottom_a = [a.x, -0.06, a.y];
        let bottom_b = [b.x, -0.06, b.y];
        positions.extend([[0.0, 0.06, 0.0], top_b, top_a]);
        normals.extend([[0.0, 1.0, 0.0]; 3]);
        positions.extend([[0.0, -0.06, 0.0], bottom_a, bottom_b]);
        normals.extend([[0.0, -1.0, 0.0]; 3]);
        positions.extend([top_a, top_b, bottom_b, top_a, bottom_b, bottom_a]);
        let edge = b - a;
        let normal = Vec3::new(edge.y, 0.0, -edge.x).normalize();
        normals.extend([normal.to_array(); 6]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ActionMarker {
    pub spent: bool,
    pub subdued: bool,
    pub additional: bool,
}

pub(crate) struct ActionRow {
    pub investigator: usize,
    pub remaining: u8,
    pub turn_ended: bool,
    pub additional: Option<bool>,
    pub additional_eligible: bool,
    pub class: InvestigatorClass,
    pub icon: Option<Handle<Image>>,
    pub background: Option<Handle<Image>>,
}

/// Returns every separately owned root/caption, for the caller's scene lifetime.
pub(crate) fn spawn_actions(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    row: ActionRow,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    let has_additional = row.additional.is_some();
    let count = if has_additional { 4 } else { 3 };
    for slot in 0..count {
        let additional = slot == 3;
        let spent = if additional {
            !row.additional.unwrap()
        } else {
            slot >= usize::from(row.remaining)
        };
        let subdued = spent || row.turn_ended || (additional && !row.additional_eligible);
        let marker = ActionMarker {
            spent,
            subdued,
            additional,
        };
        let at = layout::investigator_action(row.investigator, slot, has_additional);
        let mut root = commands.spawn((
            Name::new(if additional {
                "Restricted additional action"
            } else {
                "Ordinary action"
            }),
            marker,
            Mesh3d(meshes.add(Cylinder::new(0.21, 0.10))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: if marker.subdued {
                    row.class.color().mix(&Color::BLACK, 0.6)
                } else {
                    row.class.color()
                },
                perceptual_roughness: 0.65,
                ..default()
            })),
            Transform::from_translation(at),
            Pickable::IGNORE,
        ));
        for (layer, image) in [&row.background, &row.icon].into_iter().enumerate() {
            if let Some(image) = image {
                root.with_child((
                    Mesh3d(meshes.add(Circle::new(0.21))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: if marker.subdued {
                            Color::srgb(0.35, 0.35, 0.35)
                        } else {
                            Color::WHITE
                        },
                        base_color_texture: Some(image.clone()),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.052 + layer as f32 * 0.002, 0.0)
                        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                    Pickable::IGNORE,
                ));
            }
        }
        if marker.spent {
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.92, 0.55, 0.49),
                unlit: true,
                ..default()
            });
            for angle in [-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_4] {
                root.with_child((
                    Mesh3d(meshes.add(Cuboid::new(0.35, 0.018, 0.035))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.0, 0.063, 0.0)
                        .with_rotation(Quat::from_rotation_y(angle)),
                    Pickable::IGNORE,
                ));
            }
        }
        if marker.additional {
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.94, 0.87, 0.61),
                unlit: true,
                ..default()
            });
            for angle in [0.0, std::f32::consts::FRAC_PI_2] {
                root.with_child((
                    Mesh3d(meshes.add(Cuboid::new(0.14, 0.02, 0.035))),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(0.16, 0.075, -0.16)
                        .with_rotation(Quat::from_rotation_y(angle)),
                    Pickable::IGNORE,
                ));
            }
        }
        let token = root.id();
        entities.push(token);
        if additional {
            entities.push(spawn_label(
                commands,
                token,
                "Fight / Engage\nEnemy with bounty".into(),
                Vec3::new(1.65, 0.0, -0.05),
                Some(128.0),
            ));
        } else if slot == 1 && (row.icon.is_none() || row.background.is_none()) {
            entities.push(spawn_label(
                commands,
                token,
                "Actions".into(),
                Vec3::new(0.0, 0.0, 0.20),
                Some(72.0),
            ));
        }
    }
    entities
}

fn spawn_label(
    commands: &mut Commands,
    token: Entity,
    text: String,
    offset: Vec3,
    width: Option<f32>,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::srgb(0.94, 0.92, 0.84)),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                width: width.map_or(Val::Auto, px),
                ..default()
            },
            CounterLabel { token, offset },
            Pickable::IGNORE,
        ))
        .id()
}

fn position_labels(
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    transforms: Query<&Transform>,
    mut labels: Query<(
        Entity,
        &CounterLabel,
        &ComputedNode,
        &mut Node,
        &mut Visibility,
    )>,
) {
    let Ok((camera, camera_at)) = camera.single() else {
        return;
    };
    for (entity, label, computed, mut node, mut visibility) in &mut labels {
        let Ok(at) = transforms.get(label.token) else {
            commands.entity(entity).despawn();
            continue;
        };
        if let Ok(point) =
            camera.world_to_viewport(camera_at, at.translation + label.offset * at.scale)
        {
            let width = computed.size().x * computed.inverse_scale_factor;
            node.left = px(point.x - width / 2.0);
            node.top = px(point.y);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub(crate) fn clue_transfer_position(start: Vec3, destination: Vec3, fraction: f32) -> Vec3 {
    let fraction = fraction.clamp(0.0, 1.0);
    let eased = fraction * fraction * (3.0 - 2.0 * fraction);
    start.lerp(destination, eased) + Vec3::Y * (std::f32::consts::PI * fraction).sin() * 1.2
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        camera::primitives::MeshAabb, ecs::world::CommandQueue, mesh::VertexAttributeValues,
    };

    #[test]
    fn clue_face_preserves_rgb_with_zero_alpha_while_resources_keep_their_cutout() {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut images = Assets::<Image>::default();
        // The selected clue image's center has this RGB with alpha zero. Its
        // artwork must remain visible; the circle itself supplies the outline.
        let face = images.add(Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[254, 253, 197, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let roots = [CounterKind::Clue, CounterKind::Resource].map(|kind| {
            spawn_count(
                &mut Commands::new(&mut queue, &world),
                &mut meshes,
                &mut materials,
                CounterVisual {
                    kind,
                    amount: 1,
                    image: Some(face.clone()),
                    at: Vec3::ZERO,
                    scale: 1.0,
                },
            )
            .unwrap()
            .0
        });
        queue.apply(&mut world);
        for (root, expected_alpha) in roots.into_iter().zip([AlphaMode::Opaque, AlphaMode::Blend]) {
            let face_entity = world.get::<Children>(root).unwrap()[0];
            let material = materials
                .get(
                    &world
                        .get::<MeshMaterial3d<StandardMaterial>>(face_entity)
                        .unwrap()
                        .0,
                )
                .unwrap();
            assert_eq!(material.base_color_texture, Some(face.clone()));
            assert_eq!(material.alpha_mode, expected_alpha);
            let transform = world.get::<Transform>(face_entity).unwrap();
            let body = meshes
                .get(&world.get::<Mesh3d>(root).unwrap().0)
                .unwrap()
                .get_aabb()
                .unwrap();
            assert!(transform.translation.y > body.center.y + body.half_extents.y);
        }
    }

    #[test]
    fn totals_hide_zero_and_fall_back_independently_of_other_artwork() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut images = Assets::<Image>::default();
        let image = images.add(Image::default());
        let mut commands = Commands::new(&mut queue, &world);
        assert!(
            spawn_count(
                &mut commands,
                &mut meshes,
                &mut materials,
                CounterVisual {
                    kind: CounterKind::Clue,
                    amount: 0,
                    image: None,
                    at: Vec3::ZERO,
                    scale: 1.0
                }
            )
            .is_none()
        );
        let (resource, resource_label) = spawn_count(
            &mut commands,
            &mut meshes,
            &mut materials,
            CounterVisual {
                kind: CounterKind::Resource,
                amount: 5,
                image: Some(image),
                at: layout::investigator_resource(0),
                scale: 0.85,
            },
        )
        .unwrap();
        let (clue, clue_label) = spawn_count(
            &mut commands,
            &mut meshes,
            &mut materials,
            CounterVisual {
                kind: CounterKind::Clue,
                amount: 2,
                image: None,
                at: layout::investigator_clue(0),
                scale: 0.85,
            },
        )
        .unwrap();
        queue.apply(&mut world);
        assert_eq!(world.get::<Text>(resource_label).unwrap().0, "5");
        assert_eq!(world.get::<Text>(clue_label).unwrap().0, "Clues 2");
        assert_eq!(
            world.get::<Counter>(resource).unwrap().kind,
            CounterKind::Resource
        );
        assert_eq!(world.get::<Counter>(clue).unwrap().amount, 2);
        assert_eq!(world.query::<&Counter>().iter(&world).count(), 2);
        assert_eq!(world.get::<CounterLabel>(clue_label).unwrap().token, clue);
    }

    #[test]
    fn early_ending_and_ineligible_bonus_are_not_false_spends() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        spawn_actions(
            &mut Commands::new(&mut queue, &world),
            &mut meshes,
            &mut materials,
            ActionRow {
                investigator: 0,
                remaining: 2,
                turn_ended: true,
                additional: Some(true),
                additional_eligible: false,
                class: InvestigatorClass::Rogue,
                icon: None,
                background: None,
            },
        );
        queue.apply(&mut world);
        let markers: Vec<_> = world.query::<&ActionMarker>().iter(&world).collect();
        assert_eq!(markers.len(), 4);
        assert_eq!(markers.iter().filter(|marker| marker.spent).count(), 1);
        assert!(markers.iter().all(|marker| marker.subdued));
        assert!(
            !markers
                .iter()
                .find(|marker| marker.additional)
                .unwrap()
                .spent
        );
        assert_eq!(
            world
                .query::<&Text>()
                .iter(&world)
                .filter(|text| text.0 == "Actions")
                .count(),
            1
        );
    }

    #[test]
    fn resource_body_has_crate_outline_and_real_side_volume() {
        let mesh = resource_body();
        let VertexAttributeValues::Float32x3(points) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!("missing vertices");
        };
        assert!(points.iter().any(|point| point[1] == -0.06));
        assert!(points.iter().any(|point| point[1] == 0.06));
        // Crate shoulder corners exist, but no square backing fills the corners.
        assert!(
            points
                .iter()
                .any(|point| point[0] == 0.29 && point[2] < -0.15)
        );
        assert!(
            !points
                .iter()
                .any(|point| point[0].abs() > 0.28 && point[2].abs() > 0.32)
        );
    }

    #[test]
    fn moving_clue_lifts_between_exact_homes_and_clamps_boundaries() {
        let source = layout::location_clue(Vec3::new(0.0, 0.05, -8.5));
        let destination = layout::investigator_clue(1);
        assert_eq!(clue_transfer_position(source, destination, -0.1), source);
        assert!(clue_transfer_position(source, destination, 1.1).abs_diff_eq(destination, 0.00001));
        let middle = clue_transfer_position(source, destination, 0.5);
        assert!(middle.y > source.y.max(destination.y) + 1.0);
        assert!(middle.x >= source.x.min(destination.x) && middle.x <= source.x.max(destination.x));
    }
}
