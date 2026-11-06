//! Headless measurement of text spawned and positioned by the tabletops.

use super::{Area, TestAreaHeading, focus};
use crate::tabletop_camera::{Capture, FrameInput, Orbit};
use bevy::{
    camera::{
        CameraProjection, ComputedCameraValues, RenderTarget, RenderTargetInfo,
        primitives::MeshAabb,
    },
    prelude::*,
    text::TextLayoutInfo,
    window::{PrimaryWindow, WindowRef},
};

/// Supply the viewport metadata normally provided by the renderer, retaining
/// the camera and controls installed by the production application.
pub(crate) fn viewport<C: Component>(app: &mut App, size: Vec2, scale: f32) {
    let world = app.world_mut();
    let (window_entity, mut window) = world
        .query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>()
        .single_mut(world)
        .unwrap();
    let physical = (size * scale).as_uvec2();
    window.resolution.set_scale_factor_override(Some(scale));
    window
        .resolution
        .set_physical_resolution(physical.x, physical.y);
    let (mut camera, mut projection, mut target) = world
        .query_filtered::<(&mut Camera, &mut Projection, &mut RenderTarget), With<C>>()
        .single_mut(world)
        .unwrap();
    projection.update(size.x, size.y);
    camera.computed = ComputedCameraValues {
        clip_from_view: projection.get_clip_from_view(),
        target_info: Some(RenderTargetInfo {
            physical_size: physical,
            scale_factor: scale,
        }),
        ..default()
    };
    *target = RenderTarget::Window(WindowRef::Entity(window_entity));
}

pub(crate) fn rendered_text_bounds(world: &World, entity: Entity, scale: f32) -> Rect {
    let computed = world.get::<ComputedNode>(entity).unwrap();
    let layout = world.get::<TextLayoutInfo>(entity).unwrap();
    let transform = world.get::<UiGlobalTransform>(entity).unwrap();
    assert!(world.get::<InheritedVisibility>(entity).unwrap().get());
    assert!(
        !layout.glyphs.is_empty(),
        "caption must have real font glyphs"
    );
    let mut rect = Rect {
        min: Vec2::splat(f32::INFINITY),
        max: Vec2::splat(f32::NEG_INFINITY),
    };
    // Match the renderer: node width includes empty space, not just visible ink.
    for glyph in &layout.glyphs {
        let half = glyph.atlas_info.rect.size() / 2.;
        let at = computed.content_box().min + glyph.position;
        rect.min = rect.min.min(transform.transform_point2(at - half) / scale);
        rect.max = rect.max.max(transform.transform_point2(at + half) / scale);
    }
    rect
}

/// Project the actual installed mesh, including its world rotation and scale.
pub(crate) fn mesh_bounds(
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
    for x in [-1., 1.] {
        for y in [-1., 1.] {
            for z in [-1., 1.] {
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

/// The empty mat band between the asset and hand regions, independent of any
/// text surface placed inside it. Coordinates are world x/z.
pub(crate) fn asset_hand_gap(index: usize) -> Rect {
    let center = super::investigator_center(index, 3);
    let half_width = super::ASSET_REGION_SIZE.x.min(super::HAND_REGION_SIZE.x) / 2.;
    Rect::from_corners(
        Vec2::new(
            center.x - half_width,
            center.z + super::ASSET_REGION_Z + super::ASSET_REGION_SIZE.y / 2.,
        ),
        Vec2::new(
            center.x + half_width,
            center.z + super::HAND_REGION_Z - super::HAND_REGION_SIZE.y / 2.,
        ),
    )
}

pub(crate) fn assert_harm_captions<C: Component>(
    app: &mut App,
    captions: [Entity; 2],
    card: Entity,
    size: Vec2,
    scale: f32,
) {
    let world = app.world_mut();
    let (camera, camera_transform) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<C>>()
        .single(world)
        .unwrap();
    let mesh = world.get::<Mesh3d>(card).unwrap();
    let bounds = world
        .resource::<Assets<Mesh>>()
        .get(mesh)
        .unwrap()
        .get_aabb()
        .unwrap();
    let card_transform = world.get::<GlobalTransform>(card).unwrap();
    let mut card_top = f32::INFINITY;
    for x in [-1.0, 1.0] {
        for z in [-1.0, 1.0] {
            let local =
                Vec3::from(bounds.center) + Vec3::from(bounds.half_extents) * Vec3::new(x, 1.0, z);
            card_top = card_top.min(
                camera
                    .world_to_viewport(camera_transform, card_transform.transform_point(local))
                    .unwrap()
                    .y,
            );
        }
    }
    let rectangles = captions.map(|entity| {
        let text = world.get::<Text>(entity).unwrap();
        let computed = world.get::<ComputedNode>(entity).unwrap();
        let layout = world.get::<TextLayoutInfo>(entity).unwrap();
        let transform = world.get::<UiGlobalTransform>(entity).unwrap();
        let visible = world.get::<InheritedVisibility>(entity).unwrap();
        let context = format!("{} at {size:?}, {scale}x", text.0);
        assert!(visible.get(), "{context}: caption must be visible");
        assert!(
            !layout.glyphs.is_empty(),
            "{context}: real font glyphs must be laid out"
        );
        assert!(
            layout.glyphs.iter().all(|glyph| glyph.line_index == 0),
            "{context}: caption must fit on one line"
        );
        let content_origin = computed.content_box().min;
        let mut rendered = Rect {
            min: Vec2::splat(f32::INFINITY),
            max: Vec2::splat(f32::NEG_INFINITY),
        };
        // Match the renderer's glyph rectangles inside the computed content box.
        // The node width includes empty space; it is not the visible text width.
        for glyph in &layout.glyphs {
            let half = glyph.atlas_info.rect.size() / 2.0;
            let min = transform.transform_point2(content_origin + glyph.position - half) / scale;
            let max = transform.transform_point2(content_origin + glyph.position + half) / scale;
            rendered.min = rendered.min.min(min);
            rendered.max = rendered.max.max(max);
        }
        let bottom = transform.transform_point2(computed.size() / 2.0).y / scale;
        let gap = card_top - rendered.max.y;
        assert!(
            (0.5..=computed.size().y / scale).contains(&gap),
            "{context}: rendered glyphs must leave a positive card gap no larger than one caption line, got {gap}px (line-box gap {}px)", card_top - bottom,
        );
        assert!(
            rendered.min.cmpge(Vec2::ZERO).all() && rendered.max.cmple(size).all(),
            "{context}: complete caption must fit in the viewport: {rendered:?}"
        );
        rendered
    });
    assert!(
        rectangles[0].max.x + 0.5 <= rectangles[1].min.x,
        "harm captions must not overlap at {size:?}, {scale}x: {rectangles:?}"
    );
}

pub(crate) fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::text::TextPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::visibility::VisibilityPlugin,
        bevy::input::InputPlugin,
        bevy::input_focus::InputFocusPlugin,
        bevy::input_focus::InputDispatchPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ))
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<Time<Real>>();
    app
}

/// Measure actual text glyphs after the production scene and camera systems ran.
pub(crate) fn assert_hand_captions(
    app: &mut App,
    captions: &[Entity],
    decision_panel: Entity,
    size: Vec2,
    scale: f32,
) {
    assert!(
        captions.len() >= 4,
        "exercise neighboring captions in a full hand"
    );
    let world = app.world();
    let panel_node = world.get::<ComputedNode>(decision_panel).unwrap();
    let panel_transform = world.get::<UiGlobalTransform>(decision_panel).unwrap();
    let panel_top = panel_transform.transform_point2(-panel_node.size() / 2.).y / scale;
    let mut rectangles = captions
        .iter()
        .map(|&entity| {
            let text = world.get::<Text>(entity).unwrap();
            let rect = rendered_text_bounds(world, entity, scale);
            assert!(
                rect.min.cmpge(Vec2::ZERO).all() && rect.max.cmple(size).all(),
                "{} at {size:?}, {scale}x: complete caption must fit: {rect:?}", text.0
            );
            assert!(rect.max.y + 0.5 <= panel_top,
                "{} at {size:?}, {scale}x: caption bottom {} must clear decision panel top {panel_top}", text.0, rect.max.y);
            (text.0.clone(), rect)
        })
        .collect::<Vec<_>>();
    rectangles.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x));
    for pair in rectangles.windows(2) {
        assert!(
            pair[0].1.max.x + 0.5 <= pair[1].1.min.x,
            "hand captions must not overlap at {size:?}, {scale}x: {pair:?}"
        );
    }
}

pub(crate) fn assert_chaos_captions(
    app: &mut App,
    decision_panel: Entity,
    size: Vec2,
    scale: f32,
    area: Area,
) {
    let world = app.world_mut();
    let captions = [
        "Curse pool\n10 tokens",
        "Chaos bag\n16 tokens",
        "Committed cards",
    ]
    .map(|expected| {
        world
            .query_filtered::<(Entity, &Text), With<crate::tabletop_pieces::WorldLabel>>()
            .iter(world)
            .find_map(|(entity, text)| (text.0 == expected).then_some(entity))
            .unwrap_or_else(|| panic!("the production scene must retain {expected:?}"))
    });
    let panel_node = world.get::<ComputedNode>(decision_panel).unwrap();
    let panel_transform = world.get::<UiGlobalTransform>(decision_panel).unwrap();
    let panel_top = panel_transform.transform_point2(-panel_node.size() / 2.).y / scale;
    let rectangles = captions.map(|entity| {
        let rect = rendered_text_bounds(world, entity, scale);
        if area == Area::Test {
            assert!(rect.max.y + 0.5 <= panel_top, "chaos caption {rect:?} must clear panel top {panel_top} at {area:?}, {size:?}, {scale}x");
            assert!(rect.min.cmpge(Vec2::ZERO).all() && rect.max.cmple(size).all(),
                "Test focus must show the complete chaos caption: {rect:?} at {size:?}, {scale}x");
        }
        rect
    });
    assert!(
        (rectangles[0].min.y - rectangles[1].min.y).abs() < 0.5
            && (rectangles[0].max.y - rectangles[1].max.y).abs() < 0.5,
        "Curse pool and Chaos bag captions must share a row: {rectangles:?}"
    );
    for (index, left) in rectangles.iter().enumerate() {
        for right in &rectangles[index + 1..] {
            assert!(
                left.max.x + 0.5 <= right.min.x
                    || right.max.x + 0.5 <= left.min.x
                    || left.max.y + 0.5 <= right.min.y
                    || right.max.y + 0.5 <= left.min.y,
                "Curse pool, Chaos bag and Committed cards captions must not overlap at {area:?}, {size:?}, {scale}x: {left:?}, {right:?}"
            );
        }
    }
}

pub(crate) fn assert_sideways_caption_wrapping(app: &mut App, captions: &[Entity]) {
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::ArrowLeft);
    app.world_mut().resource_mut::<Orbit>().step(FrameInput {
        keys: &keys,
        buttons: &ButtonInput::default(),
        window: &Window {
            focused: true,
            ..default()
        },
        scroll_y: 0.,
        dt: std::f32::consts::FRAC_PI_2,
        capture: Capture::default(),
        reduced_motion: true,
    });
    for pitch in [1., 0.45] {
        if pitch < 1. {
            let mut keys = ButtonInput::default();
            keys.press(KeyCode::ArrowDown);
            app.world_mut().resource_mut::<Orbit>().step(FrameInput {
                keys: &keys,
                buttons: &ButtonInput::default(),
                window: &Window {
                    focused: true,
                    ..default()
                },
                scroll_y: 0.,
                dt: (1. - pitch) / 0.7,
                capture: Capture::default(),
                reduced_motion: true,
            });
        }
        for _ in 0..5 {
            app.update();
        }
        for &entity in captions {
            let text = app.world().get::<Text>(entity).unwrap();
            let layout = app.world().get::<TextLayoutInfo>(entity).unwrap();
            assert!(!layout.glyphs.is_empty());
            let lines = layout
                .glyphs
                .iter()
                .map(|glyph| glyph.line_index)
                .max()
                .unwrap()
                + 1;
            assert!(
                lines <= 4,
                "sideways camera at pitch {pitch} must not collapse {} into a letter-by-letter column, got {lines} lines",
                text.0
            );
        }
    }
}

pub(crate) fn assert_heading<C: Component>(app: &mut App, camera_marker: C, scale: f32) {
    let world = app.world_mut();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .iter(world)
        .next()
        .unwrap_or_else(|| world.spawn(Window::default()).id());
    world.entity_mut(window).insert(PrimaryWindow);
    let physical_size = UVec2::new((1600.0 * scale) as u32, (1000.0 * scale) as u32);
    {
        let mut window = world.get_mut::<Window>(window).unwrap();
        window.resolution.set_scale_factor_override(Some(scale));
        window
            .resolution
            .set_physical_resolution(physical_size.x, physical_size.y);
    }
    let mut projection = PerspectiveProjection::default();
    projection.update(1600.0, 1000.0);
    let camera = world
        .spawn((
            camera_marker,
            Camera {
                computed: ComputedCameraValues {
                    clip_from_view: projection.get_clip_from_view(),
                    target_info: Some(RenderTargetInfo {
                        physical_size,
                        scale_factor: scale,
                    }),
                    ..default()
                },
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window)),
            Transform::default(),
        ))
        .id();
    app.finish();
    app.cleanup();

    for area in [Area::Overview, Area::Scenario, Area::Test] {
        let window_state = app.world().get::<Window>(window).unwrap().clone();
        let mut orbit = app.world_mut().resource_mut::<Orbit>();
        orbit.focus(focus(area));
        let transform = orbit
            .step(FrameInput {
                keys: &ButtonInput::default(),
                buttons: &ButtonInput::default(),
                window: &window_state,
                scroll_y: 0.0,
                dt: 0.0,
                capture: Capture::default(),
                reduced_motion: true,
            })
            .transform;
        app.world_mut()
            .entity_mut(camera)
            .insert((transform, GlobalTransform::from(transform)));
        // Text layout runs after projection. Allow the actual font/layout,
        // label placement and propagated UI rectangle to settle in that order.
        for _ in 0..5 {
            app.update();
        }
        let world = app.world_mut();
        let (text, computed, text_layout, ui_transform, visible) = world
            .query_filtered::<(
                &Text,
                &ComputedNode,
                &TextLayoutInfo,
                &UiGlobalTransform,
                &InheritedVisibility,
            ), With<TestAreaHeading>>()
            .single(world)
            .expect("the production scene has exactly one Test area heading");
        assert!(
            !text_layout.glyphs.is_empty(),
            "{area:?} at {scale}x: {} must be measured from real font glyphs",
            text.0,
        );
        assert!(
            visible.get(),
            "{area:?} at {scale}x: {} must remain visible after text layout (size {:?})",
            text.0,
            computed.size(),
        );
        let half_size = computed.size() / 2.0;
        let rectangle = Rect::from_corners(
            ui_transform.transform_point2(-half_size) / scale,
            ui_transform.transform_point2(half_size) / scale,
        );
        let camera = world.get::<Camera>(camera).unwrap();
        // Accepted table edges, independently of the heading placement helper.
        let scenario_edge = camera
            .world_to_viewport(&GlobalTransform::from(transform), Vec3::new(0.0, 0.0, 1.0))
            .unwrap();
        let test_edge = camera
            .world_to_viewport(&GlobalTransform::from(transform), Vec3::new(0.0, 0.0, 3.0))
            .unwrap();
        assert!(
            rectangle.min.y >= scenario_edge.y.min(test_edge.y) + 0.5
                && rectangle.max.y <= scenario_edge.y.max(test_edge.y) - 0.5,
            "{area:?} at {scale}x: full heading rectangle {rectangle:?} must clear the projected edges {scenario_edge:?} and {test_edge:?}",
        );
        assert!(
            rectangle.min.x >= 0.0 && rectangle.max.x <= 1600.0,
            "{area:?} at {scale}x: the complete heading stays inside the viewport",
        );
        let test_gap = test_edge.y - rectangle.max.y;
        let scenario_edge = camera
            .world_to_viewport(
                &GlobalTransform::from(transform),
                Vec3::new(0.0, 0.0, -21.0),
            )
            .unwrap();
        let (computed, transform, visible) = world
            .query_filtered::<(&ComputedNode, &UiGlobalTransform, &InheritedVisibility),
                With<super::ScenarioAreaHeading>>().single(world)
            .expect("the production scene marks its Scenario heading");
        assert!(
            visible.get(),
            "Scenario heading must remain visible at {area:?}"
        );
        let bottom = transform.transform_point2(computed.size() / 2.0).y / scale;
        assert!(
            ((scenario_edge.y - bottom) - test_gap).abs() <= 1.0,
            "{area:?} at {scale}x: Scenario and Test heading gaps must match: {} vs {test_gap}",
            scenario_edge.y - bottom
        );
    }
}
