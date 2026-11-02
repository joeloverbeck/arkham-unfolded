use super::*;
use bevy::{
    input::{mouse::MouseWheel, touch::TouchPhase},
    picking::pointer::{PointerAction, PointerId, PointerInput, PointerLocation},
    ui_widgets::Button,
};

fn rect(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).unwrap();
    let transform = app.world().get::<UiGlobalTransform>(entity).unwrap();
    Rect {
        min: transform.transform_point2(-node.size() * 0.5),
        max: transform.transform_point2(node.size() * 0.5),
    }
}

fn input(app: &mut App, pointer: Entity, action: PointerAction) {
    let location = app
        .world()
        .get::<PointerLocation>(pointer)
        .unwrap()
        .location
        .clone()
        .unwrap();
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, location, action));
    app.update();
}

fn scroll(app: &mut App, window: Entity, pointer: Entity, unit: MouseScrollUnit, y: f32) {
    // Native input supplies both the camera's wheel message and UI pointer input.
    app.world_mut().write_message(MouseWheel {
        unit,
        x: 0.0,
        y,
        window,
        phase: TouchPhase::Moved,
    });
    input(
        app,
        pointer,
        PointerAction::Scroll {
            unit,
            x: 0.0,
            y,
            phase: TouchPhase::Moved,
        },
    );
    app.update();
}

#[test]
fn contextual_overflow_scroll_reaches_last_row_once_without_moving_camera_at_display_scale() {
    #[derive(Resource, Default)]
    struct Activations(usize);

    let directory = tempfile::tempdir().unwrap();
    for scale in [1.0, 2.0] {
        let (mut app, window, pointer) = contextual_input_app(
            Session::new(Fixture::curse()),
            directory.path().join(format!("overflow-{scale}.json")),
        );
        // Leave more than one line of overflow, so the line/pixel assertions
        // exercise scrolling rather than the lower bound's clamp.
        let size = Vec2::new(420., 200.);
        {
            let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
            window.resolution.set_scale_factor_override(Some(scale));
            window.resolution.set(size.x, size.y);
        }
        let world = app.world_mut();
        let mut camera = world
            .query_filtered::<&mut Camera, With<MainCamera>>()
            .single_mut(world)
            .unwrap();
        camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: (size * scale).as_uvec2(),
            scale_factor: scale,
        });
        act(
            &mut app,
            Action::Select(Source::Card(1), size - Vec2::splat(20.)),
        );
        app.update();
        let before = app.world().resource::<Tabletop>().view();
        let pose = app.world().resource::<Orbit>().pose();
        let world = app.world_mut();
        let menu = world
            .query::<(Entity, &GlobalZIndex)>()
            .iter(world)
            .find_map(|(entity, z)| (z.0 == 66).then_some(entity))
            .unwrap();
        let inspect = world
            .get::<Children>(menu)
            .unwrap()
            .iter()
            .rfind(|entity| world.get::<Button>(*entity).is_some())
            .unwrap();
        let text = world.get::<Children>(inspect).unwrap()[0];
        assert_eq!(world.get::<Text>(text).unwrap().0, "Inspect");
        let panel = rect(&app, menu);
        assert!(rect(&app, inspect).max.y > panel.max.y);
        contextual_cursor(
            &mut app,
            window,
            pointer,
            (panel.min + Vec2::splat(16.)) / scale,
        );
        app.update();
        scroll(&mut app, window, pointer, MouseScrollUnit::Line, -1.);
        assert_eq!(app.world().get::<ScrollPosition>(menu).unwrap().y, 28.);
        scroll(&mut app, window, pointer, MouseScrollUnit::Pixel, -5.);
        assert_eq!(app.world().get::<ScrollPosition>(menu).unwrap().y, 33.);
        scroll(&mut app, window, pointer, MouseScrollUnit::Line, -100.);
        let last = rect(&app, inspect);
        assert!(last.min.y >= panel.min.y && last.max.y <= panel.max.y);
        assert_eq!(app.world().resource::<Orbit>().pose(), pose);
        assert_eq!(app.world().resource::<Tabletop>().view(), before);

        app.init_resource::<Activations>();
        app.world_mut()
            .entity_mut(inspect)
            .observe(|_: On<Activate>, mut activations: ResMut<Activations>| activations.0 += 1);
        contextual_cursor(&mut app, window, pointer, last.center() / scale);
        app.update();
        input(
            &mut app,
            pointer,
            PointerAction::Press(PointerButton::Primary),
        );
        input(
            &mut app,
            pointer,
            PointerAction::Release(PointerButton::Primary),
        );
        app.update();
        assert_eq!(app.world().resource::<Activations>().0, 1);
        assert_eq!(app.world().resource::<Tabletop>().inspect, Some(1));
        assert_eq!(app.world().resource::<Tabletop>().view(), before);
        assert_eq!(app.world().resource::<Orbit>().pose(), pose);
    }
}
