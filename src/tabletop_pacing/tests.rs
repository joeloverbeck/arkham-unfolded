use super::*;
use bevy::window::{CursorMoved, WindowFocused};

fn app() -> App {
    let mut app = App::new();
    app.add_message::<WindowEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_plugins(TabletopPacingPlugin);
    app.world_mut().spawn(Window::default());
    app
}

fn settled(app: &mut App) {
    app.update();
    app.world_mut().resource_mut::<Cadence>().active_until = Some(Instant::now());
    app.update();
    assert!(app.world().resource::<Cadence>().idle);
}

#[test]
fn settled_table_sleeps_but_input_wakes_then_returns_to_idle() {
    let mut app = app();
    settled(&mut app);
    let settings = app.world().resource::<WinitSettings>();
    assert_eq!(
        settings.focused_mode,
        UpdateMode::reactive_low_power(Duration::from_millis(250))
    );
    assert_eq!(
        settings.unfocused_mode,
        UpdateMode::reactive_low_power(Duration::from_secs(1))
    );

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .write_message(WindowEvent::CursorMoved(CursorMoved {
            window,
            position: Vec2::new(100.0, 200.0),
            delta: Some(Vec2::X),
        }));
    app.update();
    assert!(!app.world().resource::<Cadence>().idle);
    assert_eq!(
        app.world().resource::<WinitSettings>().focused_mode,
        UpdateMode::Continuous
    );
    app.update();
    settled(&mut app);
}

#[test]
fn held_navigation_keeps_ticking_until_release_or_focus_loss() {
    let mut app = app();
    settled(&mut app);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    app.update();
    assert!(!app.world().resource::<Cadence>().idle);
    app.world_mut().resource_mut::<Cadence>().active_until = Some(Instant::now());
    app.update();
    assert!(!app.world().resource::<Cadence>().idle);

    let (window, mut state) = app
        .world_mut()
        .query::<(Entity, &mut Window)>()
        .single_mut(app.world_mut())
        .unwrap();
    state.focused = false;
    app.world_mut()
        .write_message(WindowEvent::WindowFocused(WindowFocused {
            window,
            focused: false,
        }));
    app.update();
    settled(&mut app);
}

#[test]
fn unfinished_presentation_extends_cadence_then_settles() {
    let mut app = app();
    settled(&mut app);
    app.world_mut().resource_mut::<Redraw>().request();
    app.update();
    assert!(!app.world().resource::<Cadence>().idle);
    app.update();
    settled(&mut app);
}

#[test]
fn idle_wait_is_not_charged_to_new_motion_or_delayed_into_the_next_frame() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    assert_eq!(cadence.begin(start), Duration::ZERO);
    cadence.begin(start + Duration::from_secs(3));
    cadence.finish(false);
    assert!(cadence.idle);
    let wake = start + Duration::from_secs(60);
    assert!(cadence.begin(wake) <= Duration::from_millis(17));
    cadence.finish(true);
    assert_eq!(
        cadence.begin(wake + Duration::from_millis(20)),
        Duration::from_millis(20)
    );
    assert_eq!(
        cadence.begin(wake + Duration::from_millis(120)),
        Duration::from_millis(100)
    );
}

#[test]
fn raw_mouse_motion_elsewhere_does_not_keep_a_settled_table_awake() {
    let mut app = app();
    settled(&mut app);
    app.world_mut()
        .write_message(WindowEvent::MouseMotion(bevy::input::mouse::MouseMotion {
            delta: Vec2::X,
        }));
    app.world_mut()
        .write_message(WindowEvent::RequestRedraw(bevy::window::RequestRedraw));
    app.update();
    assert!(app.world().resource::<Cadence>().idle);
}

#[test]
fn render_dispatch_is_paced_even_when_events_request_repeated_updates() {
    let mut app = App::new();
    app.insert_sub_app(RenderExtractApp, bevy::app::SubApp::new());
    app.add_message::<WindowEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_plugins(TabletopPacingPlugin);
    app.update();
    let start = Instant::now();
    for _ in 0..4 {
        app.world_mut().resource_mut::<Redraw>().request();
        app.update();
    }
    assert!(start.elapsed() >= Duration::from_millis(60));
}
