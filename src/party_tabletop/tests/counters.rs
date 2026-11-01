use super::*;
use crate::tabletop_counters::{ActionMarker, Counter, CounterKind};
use bevy::camera::primitives::MeshAabb;
use bevy::text::TextLayoutInfo;

#[test]
fn installed_party_scene_uses_totals_and_preserves_unused_restricted_allowance() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, _) = selection_lifecycle::production_app(
        directory.path().join("counters.json"),
        RenderHealth::ready_for_test(),
    );
    let world = app.world_mut();
    let counters: Vec<_> = world.query::<&Counter>().iter(world).collect();
    assert_eq!(
        counters.len(),
        1,
        "zero personal totals have no physical counters"
    );
    assert_eq!(counters[0].kind, CounterKind::Clue);
    assert_eq!(counters[0].amount, 6);
    let markers: Vec<_> = world.query::<&ActionMarker>().iter(world).collect();
    assert_eq!(markers.len(), 10);
    let bonus = markers.iter().find(|marker| marker.additional).unwrap();
    assert!(!bonus.spent);
    assert!(
        bonus.subdued,
        "Party has no enemy qualifying for Tony's unused allowance"
    );
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0 == "Clues 6")
    );

    // Supply just resource artwork. A card/class or other token image is not a
    // prerequisite for displaying this component, nor proof that clue art exists.
    app.world_mut()
        .resource_mut::<Art>()
        .handles
        .insert("token/resource".into(), Handle::default());
    choose(&mut app, "select:tony");
    choose(&mut app, "resource");
    choose(&mut app, "end-turn");
    let view = app.world().resource::<Table>().view();
    assert_eq!(view.investigators[0].ordinary_actions, 2);
    assert_eq!(view.investigators[0].resources, 1);
    assert!(view.investigators[0].turn_finished);
    let world = app.world_mut();
    assert!(world.query::<&Text>().iter(world).any(|text| text.0 == "1"));
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0 == "Clues 6")
    );
    assert!(
        !world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0.contains("resources ·") && text.0.contains("\n"))
    );
    let tony_x = layout::investigator_card(0).x;
    let markers: Vec<_> = world
        .query::<(&ActionMarker, &Transform)>()
        .iter(world)
        .filter(|(_, at)| (at.translation.x - tony_x).abs() < 2.0)
        .map(|(marker, _)| *marker)
        .collect();
    assert_eq!(markers.len(), 4);
    assert!(markers.iter().all(|marker| marker.subdued));
    assert_eq!(markers.iter().filter(|marker| marker.spent).count(), 1);
}

fn glyph_bounds(
    computed: &ComputedNode,
    layout: &TextLayoutInfo,
    transform: &UiGlobalTransform,
) -> Rect {
    assert!(
        !layout.glyphs.is_empty(),
        "measure actual laid-out font glyphs"
    );
    let origin = computed.content_box().min;
    let mut rectangle = Rect {
        min: Vec2::splat(f32::INFINITY),
        max: Vec2::splat(f32::NEG_INFINITY),
    };
    for glyph in &layout.glyphs {
        let half = glyph.atlas_info.rect.size() / 2.0;
        rectangle.min = rectangle
            .min
            .min(transform.transform_point2(origin + glyph.position - half));
        rectangle.max = rectangle
            .max
            .max(transform.transform_point2(origin + glyph.position + half));
    }
    rectangle
}

fn mesh_bounds(
    world: &World,
    camera: &Camera,
    camera_at: &GlobalTransform,
    entity: Entity,
) -> Rect {
    let mesh = world.get::<Mesh3d>(entity).unwrap();
    let bounds = world
        .resource::<Assets<Mesh>>()
        .get(mesh)
        .unwrap()
        .get_aabb()
        .unwrap();
    let transform = world.get::<GlobalTransform>(entity).unwrap();
    let mut projected = Rect {
        min: Vec2::splat(f32::INFINITY),
        max: Vec2::splat(f32::NEG_INFINITY),
    };
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                let point = Vec3::from(bounds.center)
                    + Vec3::from(bounds.half_extents) * Vec3::new(x, y, z);
                let screen = camera
                    .world_to_viewport(camera_at, transform.transform_point(point))
                    .unwrap();
                projected.min = projected.min.min(screen);
                projected.max = projected.max.max(screen);
            }
        }
    }
    projected
}

#[test]
fn installed_personal_fallback_captions_have_separate_rendered_text_bounds() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, _) = selection_lifecycle::production_app(
        directory.path().join("caption-layout.json"),
        RenderHealth::ready_for_test(),
    );
    app.world_mut().resource_mut::<Table>().reduced_motion = true;
    for choice in [
        "select:tony",
        "resource",
        "investigate",
        "commit:none",
        "reveal",
        "resolve:0",
    ] {
        choose(&mut app, choice);
    }
    app.world_mut()
        .resource_mut::<Orbit>()
        .focus(layout::focus(Area::Investigator(0)));
    for _ in 0..5 {
        app.update();
    }
    for size in [Vec2::new(1280.0, 720.0), Vec2::new(1600.0, 1000.0)] {
        for scale in [1.0, 2.0] {
            layout::heading_checks::viewport::<MainCamera>(&mut app, size, scale);
            for _ in 0..5 {
                app.update();
            }
            let world = app.world_mut();
            for (horror, title) in [(false, "Damage 0/9"), (true, "Horror 0/5")] {
                let caption = world
                    .query::<(&Text, &ComputedNode, &TextLayoutInfo, &UiGlobalTransform)>()
                    .iter(world)
                    .find(|(text, _, _, _)| text.0 == title)
                    .map(|(_, node, text, transform)| glyph_bounds(node, text, transform))
                    .unwrap();
                let token = world
                    .query_filtered::<(Entity, &Transform), With<crate::tabletop_harm_tokens::HarmToken>>()
                    .iter(world)
                    .find(|(_, at)| at.translation == layout::investigator_harm(0, horror))
                    .unwrap().0;
                let (camera, camera_at) = world
                    .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
                    .single(world)
                    .unwrap();
                let bounds = mesh_bounds(world, camera, camera_at, token);
                let gap = caption.min.y / scale - bounds.max.y;
                assert!(
                    gap >= 0.0,
                    "{title} at {size:?}, {scale}x: moving the harm caption closer must not overlap its token, got {gap}px"
                );
            }
        }
    }
    layout::heading_checks::viewport::<MainCamera>(&mut app, Vec2::new(1600.0, 1000.0), 1.0);
    for _ in 0..5 {
        app.update();
    }
    let world = app.world_mut();
    let mut rectangles = Vec::new();
    for title in [
        "Resources 1",
        "Clues 1",
        "Actions",
        "Fight / Engage\nEnemy with bounty",
        "Damage 0/9",
        "Horror 0/5",
    ] {
        let rectangle = world
            .query::<(
                &Text,
                &ComputedNode,
                &TextLayoutInfo,
                &UiGlobalTransform,
                Option<&BackgroundColor>,
            )>()
            .iter(world)
            .filter(|(text, _, _, _, _)| text.0 == title)
            .map(|(_, node, layout, transform, background)| {
                if background.is_some_and(|color| color.0.alpha() > 0.0) {
                    Rect {
                        min: transform.transform_point2(-node.size() / 2.0),
                        max: transform.transform_point2(node.size() / 2.0),
                    }
                } else {
                    glyph_bounds(node, layout, transform)
                }
            })
            .min_by(|a, b| a.min.x.total_cmp(&b.min.x))
            .unwrap_or_else(|| panic!("missing caption {title}"));
        rectangles.push((title, rectangle));
    }
    for (index, (left_title, left)) in rectangles.iter().enumerate() {
        for (right_title, right) in &rectangles[index + 1..] {
            assert!(
                left.max.x < right.min.x
                    || right.max.x < left.min.x
                    || left.max.y < right.min.y
                    || right.max.y < left.min.y,
                "{left_title} {left:?} overlaps {right_title} {right:?}"
            );
        }
    }
    // Counts must clear the investigator card, counters, action disks and their
    // raised crosses, using meshes actually installed in the scene.
    let mut roots: Vec<_> = world
        .query_filtered::<Entity, With<ActionMarker>>()
        .iter(world)
        .collect();
    roots.extend(
        world
            .query::<(Entity, &Name)>()
            .iter(world)
            .filter(|(_, name)| ["Tony Morgan", "Bounty Contracts"].contains(&name.as_str()))
            .map(|(entity, _)| entity),
    );
    roots.extend(world.query_filtered::<Entity, With<Counter>>().iter(world));
    let (camera, camera_at) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
        .single(world)
        .unwrap();
    for root in roots {
        let mut components = vec![root];
        if let Some(children) = world.get::<Children>(root) {
            components.extend(children.iter());
        }
        for entity in components {
            let Some(_) = world.get::<Mesh3d>(entity) else {
                continue;
            };
            let projected = mesh_bounds(world, camera, camera_at, entity);
            let captions = if world
                .get::<Name>(root)
                .is_some_and(|name| ["Tony Morgan", "Bounty Contracts"].contains(&name.as_str()))
            {
                &rectangles[..]
            } else {
                &rectangles[..2]
            };
            for (title, caption) in captions {
                // Keep a measurable half-pixel gap, matching the harm-caption
                // check; preference for wider spacing remains native review.
                assert!(
                    caption.max.x + 0.5 < projected.min.x
                        || projected.max.x + 0.5 < caption.min.x
                        || caption.max.y + 0.5 < projected.min.y
                        || projected.max.y + 0.5 < caption.min.y,
                    "{title} {caption:?} touches component {entity:?} geometry {projected:?}"
                );
            }
            if entity == root
                && world
                    .get::<Name>(root)
                    .is_some_and(|name| name.as_str() == "Tony Morgan")
            {
                for (title, caption) in &rectangles[4..] {
                    let gap = projected.min.y - caption.max.y;
                    assert!(
                        gap > 1.0,
                        "{title}: harm caption must retain a visible gap above the investigator card, got {gap}px"
                    );
                }
            }
        }
    }
    assert!(
        !world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0.contains("Bonus action: no eligible enemy"))
    );
}
