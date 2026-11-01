//! Headless text layout for the headings spawned and positioned by each tabletop.

use super::{Area, TestAreaHeading, focus};
use crate::tabletop_camera::{Capture, FrameInput, Orbit};
use bevy::{
    camera::{CameraProjection, ComputedCameraValues, RenderTarget, RenderTargetInfo},
    prelude::*,
    text::TextLayoutInfo,
    window::{PrimaryWindow, WindowRef},
};

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
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<Time<Real>>();
    app
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
    }
}
