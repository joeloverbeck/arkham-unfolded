//! A short, local presentation of already committed harm. No rules run here.

use super::*;
use crate::tabletop_harm_text::{DAMAGE_COLOR, HORROR_COLOR};
use crate::tony::{HarmRecipient, HarmSource};

#[derive(Component)]
pub(super) struct HarmVisual {
    rest: Vec3,
    falling: bool,
}

pub(super) fn advance_presentation(
    time: Res<Time>,
    mut table: ResMut<Table>,
    settings: Res<AssetSettings>,
) {
    let paused = table.modal() || settings.is_busy();
    let playback = table
        .replay
        .as_ref()
        .map_or(Playback::Live, |replay| Playback::Replay {
            harm_visible: replay.harm().is_some(),
        });
    if table.presentation.tick(time.delta_secs(), paused, playback) {
        table.dirty = true;
    }
}

pub(super) fn recipient_position(recipient: HarmRecipient, view: &Observation) -> Vec3 {
    match recipient {
        HarmRecipient::Tony => scene::INVESTIGATOR_AT,
        HarmRecipient::Ghoul => scene::ghoul_at(view),
    }
}

pub(super) fn harm_amounts(parent: &mut ChildSpawnerCommands, art: &Art, hit: Harm) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            ..default()
        })
        .with_children(|p| {
            for (amount, label, color) in [
                (
                    hit.damage,
                    if hit.is_direct() {
                        "Direct damage"
                    } else {
                        "Damage"
                    },
                    DAMAGE_COLOR,
                ),
                (hit.horror, "Horror", HORROR_COLOR),
            ] {
                if amount > 0 {
                    p.spawn(art.text(format!("+{amount} {label}"), 22.0, color));
                }
            }
        });
}

fn counter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    at: Vec3,
    horror: bool,
    arriving: bool,
) {
    let role = if horror {
        "token/horror"
    } else {
        "token/damage"
    };
    let Some(image) = art.image(role) else {
        // The projected label supplies the readable substitute; a plain colored
        // rectangle is not a recognizable heart or brain.
        return;
    };
    let size = if horror {
        Vec2::new(0.43, 0.35)
    } else {
        Vec2::new(0.35, 0.43)
    };
    let mut entity = commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size.x, size.y))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(image),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 2.0,
            ..default()
        })),
        Transform::from_translation(at),
        Scene,
        Pickable::IGNORE,
    ));
    if arriving {
        entity.insert(HarmVisual {
            rest: at,
            falling: true,
        });
    }
}

fn outline(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    size: Vec2,
    color: Color,
) {
    let material = materials.add(StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    });
    for (offset, dimensions) in [
        (
            Vec3::new(-size.x / 2.0, 0.02, 0.0),
            Vec3::new(0.035, 0.025, size.y),
        ),
        (
            Vec3::new(size.x / 2.0, 0.02, 0.0),
            Vec3::new(0.035, 0.025, size.y),
        ),
        (
            Vec3::new(0.0, 0.02, -size.y / 2.0),
            Vec3::new(size.x, 0.025, 0.035),
        ),
        (
            Vec3::new(0.0, 0.02, size.y / 2.0),
            Vec3::new(size.x, 0.025, 0.035),
        ),
    ] {
        let at = center + offset;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(dimensions))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(at),
            HarmVisual {
                rest: at,
                falling: false,
            },
            Scene,
            Pickable::IGNORE,
        ));
    }
}

pub(super) fn harm_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    view: &Observation,
    hit: Option<Harm>,
) {
    for (recipient, damage, horror, visible) in [
        (HarmRecipient::Tony, view.damage, view.horror, true),
        (HarmRecipient::Ghoul, view.ghoul.damage, 0, view.ghoul.alive),
    ] {
        if !visible {
            continue;
        }
        let at = recipient_position(recipient, view);
        // Center the *visible bounds* as one row. Damage is narrower than
        // horror, so symmetric center offsets would still leave the pair askew.
        let counters: Vec<_> = [(damage, false, 0.35), (horror, true, 0.43)]
            .into_iter()
            .filter(|(amount, _, _)| *amount > 0)
            .collect();
        let gap = 0.18;
        let width = counters.iter().map(|(_, _, width)| width).sum::<f32>()
            + gap * counters.len().saturating_sub(1) as f32;
        let mut left = -width / 2.0;
        let top_edge = if recipient == HarmRecipient::Tony {
            -CARD_WIDTH / 2.0
        } else {
            -CARD_LENGTH / 2.0
        };
        // Leave room for the whole token and its screen-sized total between
        // Tony's counter row and the landscape card, not just the token center.
        let row_z = top_edge
            - if recipient == HarmRecipient::Tony {
                0.9
            } else {
                0.0
            };
        let needs_substitute = (damage > 0 && art.image("token/damage").is_none())
            || (horror > 0 && art.image("token/horror").is_none());
        for (amount, is_horror, width) in counters {
            let position = at + Vec3::new(left + width / 2.0, 0.06, row_z);
            left += width + gap;
            let arriving = hit.is_some_and(|h| {
                h.recipient == recipient
                    && if is_horror {
                        h.horror > 0
                    } else {
                        h.damage > 0
                    }
            });
            counter(
                commands, meshes, materials, art, position, is_horror, arriving,
            );
            if needs_substitute {
                continue;
            }
            commands.spawn((
                art.text(amount.to_string(), 16.0, PAPER),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(30),
                    ..default()
                },
                BackgroundColor(INK),
                WorldLabel(position + Vec3::new(0.0, 0.08, 0.23), 30.0),
                Scene,
                Pickable::IGNORE,
            ));
        }
        if needs_substitute {
            let mut amounts = Vec::new();
            if damage > 0 {
                amounts.push(format!("{damage} Damage"));
            }
            if horror > 0 {
                amounts.push(format!("{horror} Horror"));
            }
            let color = match (damage > 0, horror > 0) {
                (true, false) => DAMAGE_COLOR,
                (false, true) => HORROR_COLOR,
                _ => PAPER,
            };
            // One centered label remains readable even when the camera makes
            // adjacent world positions too close for separate English names.
            commands.spawn((
                art.text(amounts.join(" · "), 13.0, color),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(160),
                    ..default()
                },
                BackgroundColor(INK),
                WorldLabel(at + Vec3::new(0.0, 0.14, row_z + 0.23), 160.0),
                Scene,
                Pickable::IGNORE,
            ));
        }
    }
    let Some(hit) = hit else {
        return;
    };
    let target = recipient_position(hit.recipient, view);
    let target_size = if hit.recipient == HarmRecipient::Tony {
        Vec2::new(CARD_LENGTH + 0.1, CARD_WIDTH + 0.1)
    } else {
        Vec2::new(CARD_WIDTH + 0.1, CARD_LENGTH + 0.1)
    };
    outline(commands, meshes, materials, target, target_size, PAPER);
    let (source, size) = match hit.source {
        HarmSource::OpportunityAttack => (
            recipient_position(HarmRecipient::Ghoul, view),
            Vec2::new(CARD_WIDTH + 0.1, CARD_LENGTH + 0.1),
        ),
        HarmSource::BasicFight => (
            recipient_position(HarmRecipient::Tony, view),
            Vec2::new(CARD_LENGTH + 0.1, CARD_WIDTH + 0.1),
        ),
        HarmSource::InternalInjury => (
            scene::INJURY_AT,
            Vec2::new(CARD_WIDTH + 0.1, CARD_LENGTH + 0.1),
        ),
        HarmSource::Tablet | HarmSource::Cultist => (scene::TOKEN_AT, Vec2::splat(1.4)),
    };
    outline(commands, meshes, materials, source, size, GOLD);
    let mut amounts = Vec::new();
    if hit.damage > 0 {
        amounts.push(format!(
            "+{} {}",
            hit.damage,
            if hit.is_direct() {
                "Direct damage"
            } else {
                "Damage"
            }
        ));
    }
    if hit.horror > 0 {
        amounts.push(format!("+{} Horror", hit.horror));
    }
    world_label(
        commands,
        art,
        amounts.join("\n"),
        target
            + Vec3::new(
                0.0,
                0.45,
                if hit.recipient == HarmRecipient::Tony {
                    -2.4
                } else {
                    -0.9
                },
            ),
        GOLD,
    );
}

pub(super) fn animate_harm(table: Res<Table>, mut visuals: Query<(&HarmVisual, &mut Transform)>) {
    let progress = table.presentation.animation_progress();
    for (visual, mut transform) in &mut visuals {
        transform.translation = visual.rest;
        transform.scale = Vec3::ONE;
        if table.reduced_motion {
            continue;
        }
        if visual.falling {
            transform.translation.y += 0.4 * (1.0 - progress).powi(2);
        } else {
            let pulse = (progress * std::f32::consts::PI).sin() * 0.06;
            transform.scale = Vec3::splat(1.0 + pulse);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn counters(damage: u8, horror: u8, artwork: bool) -> App {
        let mut app = App::new();
        let mut images = Assets::<Image>::default();
        let damage_image = images.add(Image::default());
        let horror_image = images.add(Image::default());
        app.insert_resource(Art {
            handles: if artwork {
                [
                    ("token/damage".into(), damage_image),
                    ("token/horror".into(), horror_image),
                ]
                .into()
            } else {
                BTreeMap::new()
            },
            symbol_font: None,
            status: String::new(),
        })
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>();
        let mut view = Session::new().unwrap().observe();
        view.damage = damage;
        view.horror = horror;
        app.world_mut()
            .run_system_once(
                move |mut commands: Commands,
                      mut meshes: ResMut<Assets<Mesh>>,
                      mut materials: ResMut<Assets<StandardMaterial>>,
                      art: Res<Art>| {
                    harm_scene(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &art,
                        &view,
                        None,
                    );
                },
            )
            .unwrap();
        app
    }

    #[test]
    fn harm_counters_center_above_the_landscape_investigator_without_covering_it() {
        use bevy::camera::primitives::MeshAabb;
        for (damage, horror) in [(1, 1), (2, 0), (0, 1)] {
            let mut app = counters(damage, horror, true);
            let world = app.world_mut();
            let mut query = world.query::<(&Mesh3d, &Transform)>();
            let meshes = world.resource::<Assets<Mesh>>();
            let mut left = f32::INFINITY;
            let mut right = f32::NEG_INFINITY;
            let center = scene::INVESTIGATOR_AT;
            for (mesh, transform) in query.iter(world) {
                let aabb = meshes.get(mesh).unwrap().get_aabb().unwrap();
                left = left.min(transform.translation.x - aabb.half_extents.x);
                right = right.max(transform.translation.x + aabb.half_extents.x);
                assert!(
                    transform.translation.z + aabb.half_extents.z < center.z - CARD_WIDTH / 2.0,
                    "the entire counter must clear the top long edge of the landscape card"
                );
            }
            assert!(
                ((left + right) / 2.0 - center.x).abs() < 0.001,
                "the full visible group must be centered, including different token widths"
            );
        }
    }

    #[test]
    fn available_harm_artwork_reaches_the_drawn_counter_materials() {
        let mut app = counters(1, 1, true);
        let world = app.world_mut();
        let mut query = world.query::<&MeshMaterial3d<StandardMaterial>>();
        let materials = world.resource::<Assets<StandardMaterial>>();
        let mut shown: Vec<_> = query
            .iter(world)
            .map(|handle| {
                let material = materials.get(handle).unwrap();
                assert_eq!(material.base_color, Color::WHITE);
                material
                    .base_color_texture
                    .clone()
                    .expect("available artwork must be textured")
            })
            .collect();
        shown.sort_by_key(|image| image.id());
        let mut expected: Vec<_> = world.resource::<Art>().handles.values().cloned().collect();
        expected.sort_by_key(|image| image.id());
        assert_eq!(shown, expected);
    }

    #[test]
    fn absent_harm_artwork_has_named_substitutes_instead_of_unlabelled_rectangles() {
        let mut app = counters(1, 1, false);
        let world = app.world_mut();
        let labels: Vec<_> = world
            .query::<&Text>()
            .iter(world)
            .map(|text| text.0.as_str())
            .collect();
        assert!(
            labels.iter().any(|label| label.contains("Damage")),
            "damage needs a readable substitute"
        );
        assert!(
            labels.iter().any(|label| label.contains("Horror")),
            "horror needs a readable substitute"
        );
    }
}
