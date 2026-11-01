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
    time: Res<PresentationTime>,
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
        HarmRecipient::Tony => scene::investigator_at(),
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
    let entity = crate::tabletop_harm_tokens::spawn(
        commands,
        meshes,
        materials,
        art.image(role),
        at,
        horror,
    );
    let mut entity = commands.entity(entity);
    entity.insert(Scene);
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
    for (index, investigator) in view.investigators.iter().enumerate() {
        for horror in [false, true] {
            let at = layout::investigator_harm(index, horror);
            let role = if horror {
                "token/horror"
            } else {
                "token/damage"
            };
            let kind = if horror { "Horror" } else { "Damage" };
            let total = if horror {
                Some((investigator.horror, investigator.sanity))
            } else if index == 0 {
                Some((view.damage, view.health))
            } else {
                // Earlier companion observations never represented damage or health.
                None
            };
            let text = match total {
                Some((amount, capacity)) if art.image(role).is_some() => {
                    format!("{amount}/{capacity}")
                }
                Some((amount, capacity)) => format!("{kind} {amount}/{capacity}"),
                None => "Damage n/a".to_owned(),
            };
            let arriving = index == 0
                && hit.is_some_and(|hit| {
                    hit.recipient == HarmRecipient::Tony
                        && if horror {
                            hit.horror > 0
                        } else {
                            hit.damage > 0
                        }
                });
            counter(commands, meshes, materials, art, at, horror, arriving);
            counter_total(
                commands,
                text,
                layout::investigator_harm_caption(
                    index,
                    horror,
                    total.is_none() || art.image(role).is_none(),
                ),
            );
        }
    }
    if view.ghoul.alive && view.ghoul.damage > 0 {
        let at = recipient_position(HarmRecipient::Ghoul, view)
            + Vec3::new(0.0, 0.09, -CARD_LENGTH / 2.0);
        counter(
            commands,
            meshes,
            materials,
            art,
            at,
            false,
            hit.is_some_and(|hit| hit.recipient == HarmRecipient::Ghoul && hit.damage > 0),
        );
        let text = if art.image("token/damage").is_some() {
            format!("{}/2", view.ghoul.damage)
        } else {
            format!("Damage\n{}/2", view.ghoul.damage)
        };
        counter_total(commands, text, at + Vec3::Z * 0.5);
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
            scene::injury_at(),
            Vec2::new(CARD_WIDTH + 0.1, CARD_LENGTH + 0.1),
        ),
        HarmSource::Tablet | HarmSource::Cultist => (scene::token_at(), Vec2::splat(1.4)),
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
        if hit.recipient == HarmRecipient::Tony {
            scene::investigator_at() + Vec3::new(0.0, 0.45, 0.0)
        } else {
            target + Vec3::new(0.0, 0.45, -0.9)
        },
        GOLD,
    );
}

fn counter_total(commands: &mut Commands, text: String, at: Vec3) {
    commands.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(PAPER),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            width: px(80.0),
            ..default()
        },
        WorldLabel(at, 80.0),
        Scene,
        Pickable::IGNORE,
    ));
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
        view.investigators[0].horror = horror;
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
    fn permanent_harm_tokens_include_zero_totals_and_clear_each_investigator_card() {
        use bevy::camera::primitives::MeshAabb;
        for (damage, horror) in [(0, 0), (1, 1), (2, 0), (0, 1)] {
            let mut app = counters(damage, horror, true);
            let world = app.world_mut();
            let mut query = world.query_filtered::<
                (&Mesh3d, &Transform),
                With<crate::tabletop_harm_tokens::HarmToken>,
            >();
            let meshes = world.resource::<Assets<Mesh>>();
            assert_eq!(query.iter(world).count(), 6);
            for (mesh, transform) in query.iter(world) {
                let aabb = meshes.get(mesh).unwrap().get_aabb().unwrap();
                let index = (0..3)
                    .find(|&index| {
                        (transform.translation.x - layout::investigator_card(index).x).abs() < 1.0
                    })
                    .unwrap();
                let card = layout::investigator_card(index);
                let center = layout::investigator_center(index, 3);
                assert!(
                    (transform.translation.x - center.x).abs() + aabb.half_extents.x < 9.0,
                    "counter must stay on its investigator surface"
                );
                assert!(aabb.half_extents.y > 0.05, "counter has visible thickness");
                assert!(
                    transform.translation.z + aabb.half_extents.z < card.z - CARD_WIDTH / 2.0,
                    "the entire counter must clear the upper edge of the landscape card"
                );
            }
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
            .filter_map(|handle| {
                let material = materials.get(handle).unwrap();
                let image = material.base_color_texture.clone()?;
                assert_eq!(material.base_color, Color::WHITE);
                Some(image)
            })
            .collect();
        assert_eq!(shown.len(), 6);
        shown.sort_by_key(|image| image.id());
        shown.dedup();
        let mut expected: Vec<_> = world.resource::<Art>().handles.values().cloned().collect();
        expected.sort_by_key(|image| image.id());
        assert_eq!(shown, expected);
    }

    #[test]
    fn absent_harm_artwork_has_named_substitutes_instead_of_unlabelled_rectangles() {
        let mut app = counters(1, 1, false);
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<crate::tabletop_harm_tokens::HarmToken>>()
                .iter(world)
                .count(),
            6,
            "missing artwork must retain the actual physical counters"
        );
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
        assert!(labels.contains(&"Horror 2/9"));
        assert!(labels.contains(&"Damage n/a"));
    }
}
