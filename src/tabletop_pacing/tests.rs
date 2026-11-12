use super::*;
use bevy::window::{CursorMoved, WindowFocused};

pub(super) fn app() -> App {
    let mut app = App::new();
    app.add_message::<WindowEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_plugins(TabletopPacingPlugin);
    app.world_mut().spawn(Window::default());
    app
}

pub(super) fn pace(app: &App) -> Pace {
    app.world().resource::<Cadence>().pace
}

fn settled(app: &mut App) {
    app.update();
    app.world_mut().resource_mut::<Cadence>().active_until = Some(Instant::now());
    app.update();
    assert_eq!(pace(app), Pace::Settled);
}

/// One update in which only a slow looping decoration asks for a frame.
pub(super) fn decorate(app: &mut App) {
    app.world_mut().resource_mut::<Redraw>().decorate();
    app.update();
}

pub(super) fn end_active_interval(app: &mut App) {
    app.world_mut().resource_mut::<Cadence>().active_until = Some(Instant::now());
}

fn move_cursor(app: &mut App) {
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
}

#[test]
fn decorations_alone_draw_at_the_decorative_rate_then_settle_when_they_stop() {
    let mut app = app();
    // Startup keeps the display rate whatever asks for frames.
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Active);
    settled(&mut app);

    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Decorative);
    assert!(!app.world().resource::<Settled>().get());
    let mode = app.world().resource::<WinitSettings>().focused_mode;
    assert_eq!(mode, UpdateMode::reactive_low_power(DECORATIVE_FRAME));
    // Window input ends the wait between decorative frames.
    assert!(matches!(
        mode,
        UpdateMode::Reactive {
            react_to_window_events: true,
            ..
        }
    ));
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Decorative);

    // No choice offered, Reduced motion or focus loss: nothing decorates any more.
    app.update();
    assert_eq!(pace(&app), Pace::Settled);
    assert!(app.world().resource::<Settled>().get());
    assert_eq!(
        app.world().resource::<WinitSettings>().focused_mode,
        UpdateMode::reactive_low_power(Duration::from_millis(250))
    );
}

#[test]
fn input_or_other_work_returns_a_decorative_table_to_display_rate() {
    let mut app = app();
    settled(&mut app);
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Decorative);

    move_cursor(&mut app);
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Active);
    assert_eq!(
        app.world().resource::<WinitSettings>().focused_mode,
        UpdateMode::Continuous
    );
    // The settling interval still holds the display rate.
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Active);
    end_active_interval(&mut app);
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Decorative);

    app.world_mut().resource_mut::<Redraw>().request();
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Active);
    end_active_interval(&mut app);
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Decorative);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    end_active_interval(&mut app);
    decorate(&mut app);
    assert_eq!(pace(&app), Pace::Active);
}

#[test]
fn decorative_frames_keep_their_phase_but_input_does_not_inherit_the_wait() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    cadence.begin(start, false);
    let frame = start + Duration::from_secs(3);
    cadence.begin(frame, false);
    cadence.finish(Demand::Decorative);
    assert_eq!(cadence.pace, Pace::Decorative);
    // A timed frame advances presentation by its whole interval, so the breath keeps pace.
    let frame = frame + DECORATIVE_FRAME;
    assert_eq!(cadence.begin(frame, false), DECORATIVE_FRAME);
    cadence.finish(Demand::Decorative);
    // Input that ends the wait moves at most one 60/s step, as on a settled table.
    assert_eq!(
        cadence.begin(frame + Duration::from_millis(30), true),
        SETTLED_FRAME
    );
}

#[test]
fn breathing_availability_is_decorative_while_a_hover_fade_keeps_display_rate() {
    use crate::tabletop_feedback::{
        CardFeedback, FeedbackPlugin, FeedbackSettings, attach_card_feedback,
    };
    use bevy::picking::{backend::HitData, hover::HoverMap, pointer::PointerId};
    let mut app = app();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<HoverMap>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .add_plugins(bevy::transform::TransformPlugin)
        .add_plugins(FeedbackPlugin);
    {
        let mut window = app
            .world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .unwrap();
        window.focused = true;
        window.set_cursor_position(Some(Vec2::new(30.0, 30.0)));
    }
    let card = app.world_mut().spawn(Transform::default()).id();
    app.world_mut()
        .resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
                let mut commands = world.commands();
                attach_card_feedback(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    card,
                    2.0,
                    2.8,
                    0.0,
                );
            });
        });
    app.world_mut().flush();
    app.world_mut()
        .get_mut::<CardFeedback>(card)
        .unwrap()
        .opportunity = true;
    app.update();
    end_active_interval(&mut app);
    app.update();
    assert_eq!(pace(&app), Pace::Decorative);

    app.world_mut()
        .resource_mut::<HoverMap>()
        .entry(PointerId::Mouse)
        .or_default()
        .insert(card, HitData::new(Entity::PLACEHOLDER, 1.0, None, None));
    end_active_interval(&mut app);
    app.update();
    assert_eq!(pace(&app), Pace::Active);

    // Reduced motion completes the hover and steadies the cue: nothing asks for frames.
    app.world_mut()
        .resource_mut::<FeedbackSettings>()
        .reduced_motion = true;
    end_active_interval(&mut app);
    app.update();
    assert_eq!(pace(&app), Pace::Settled);
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

    move_cursor(&mut app);
    app.update();
    assert_eq!(pace(&app), Pace::Active);
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
    assert_eq!(pace(&app), Pace::Active);
    app.world_mut().resource_mut::<Cadence>().active_until = Some(Instant::now());
    app.update();
    assert_eq!(pace(&app), Pace::Active);

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
    assert_eq!(pace(&app), Pace::Active);
    app.update();
    settled(&mut app);
}

#[test]
fn reveal_control_attention_moves_only_the_icon_and_sleeps_when_paused() {
    use crate::tabletop_reveal::{self as reveal, Artwork, Shortcut};
    let mut app = app();
    app.add_plugins(reveal::RevealPlugin).add_systems(
        Update,
        (
            |mut time: ResMut<PresentationTime>| time.advance_by(Duration::from_millis(400)),
            reveal::render,
        )
            .chain(),
    );
    app.world_mut().resource_mut::<Artwork>().0 = Some(Handle::default());
    let choices = [crate::Choice {
        id: "test:reveal".into(),
        label: "Reveal".into(),
        cancel_faces: vec![],
    }];
    {
        let mut shortcut = app.world_mut().resource_mut::<Shortcut>();
        shortcut.attention_cue(true);
        shortcut.sync_offered(1, 1, &choices, true, true);
    }
    app.update();
    app.world_mut()
        .resource_mut::<Shortcut>()
        .sync_offered(1, 1, &choices, true, false);
    for _ in 0..3 {
        app.update();
    }
    let (icon, initial) = app
        .world_mut()
        .query_filtered::<(Entity, &UiTransform), With<ImageNode>>()
        .single(app.world())
        .map(|(entity, transform)| (entity, *transform))
        .unwrap();
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<reveal::Root>>()
        .single(app.world())
        .unwrap();
    let button = app
        .world_mut()
        .query_filtered::<Entity, With<reveal::RevealButton>>()
        .single(app.world())
        .unwrap();
    let hitbox = *app.world().get::<UiTransform>(button).unwrap();
    // End the active interval: only the production attention cue keeps it drawing.
    app.world_mut().resource_mut::<Cadence>().active_until = Some(Instant::now());
    app.update();
    assert_ne!(
        app.world().get::<UiTransform>(icon).unwrap().translation,
        initial.translation
    );
    assert_eq!(
        app.world().get::<UiTransform>(root).unwrap().scale,
        Vec2::ONE
    );
    assert_eq!(*app.world().get::<UiTransform>(button).unwrap(), hitbox);
    assert_eq!(pace(&app), Pace::Decorative);

    app.world_mut()
        .query::<&mut Window>()
        .single_mut(app.world_mut())
        .unwrap()
        .focused = false;
    settled(&mut app);
    let paused = *app.world().get::<UiTransform>(icon).unwrap();
    app.update();
    assert_eq!(*app.world().get::<UiTransform>(icon).unwrap(), paused);
    app.world_mut()
        .query::<&mut Window>()
        .single_mut(app.world_mut())
        .unwrap()
        .focused = true;
    app.world_mut()
        .resource_mut::<Shortcut>()
        .sync_offered(1, 1, &choices, true, true);
    settled(&mut app);
    let alternatives = [
        choices[0].clone(),
        crate::Choice {
            id: "tokens:jacqueline".into(),
            label: "Use Jacqueline".into(),
            cancel_faces: vec![],
        },
    ];
    app.world_mut()
        .resource_mut::<Shortcut>()
        .sync_offered(1, 1, &alternatives, true, false);
    app.update();
    let button = app
        .world_mut()
        .query_filtered::<Entity, With<reveal::RevealButton>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: button });
    let request = app
        .world_mut()
        .resource_mut::<Shortcut>()
        .drain()
        .pop()
        .unwrap();
    assert!(matches!(
        app.world_mut().resource_mut::<Shortcut>().admit(request),
        Some(reveal::Accepted::Opened)
    ));
    settled(&mut app);
    app.world_mut().resource_mut::<Shortcut>().dismiss();
    app.world_mut().resource_mut::<Artwork>().0 = None;
    settled(&mut app);
    assert!(
        app.world_mut()
            .query::<&ImageNode>()
            .iter(app.world())
            .next()
            .is_none()
    );
    app.world_mut()
        .resource_mut::<Shortcut>()
        .sync_offered(1, 1, &choices, false, true);
    settled(&mut app);
    assert!(
        app.world_mut()
            .query_filtered::<Entity, With<reveal::Root>>()
            .iter(app.world())
            .next()
            .is_none()
    );
}

#[test]
fn idle_wait_is_not_charged_to_new_motion_or_delayed_into_the_next_frame() {
    let start = Instant::now();
    let mut cadence = Cadence::default();
    assert_eq!(cadence.begin(start, false), Duration::ZERO);
    cadence.begin(start + Duration::from_secs(3), false);
    cadence.finish(Demand::None);
    assert_eq!(cadence.pace, Pace::Settled);
    let wake = start + Duration::from_secs(60);
    assert!(cadence.begin(wake, true) <= Duration::from_millis(17));
    cadence.finish(Demand::Display);
    assert_eq!(
        cadence.begin(wake + Duration::from_millis(20), false),
        Duration::from_millis(20)
    );
    assert_eq!(
        cadence.begin(wake + Duration::from_millis(120), false),
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
    assert_eq!(pace(&app), Pace::Settled);
}

fn dispatching_app() -> App {
    let mut app = App::new();
    app.insert_sub_app(RenderExtractApp, bevy::app::SubApp::new());
    app.add_message::<WindowEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_plugins(TabletopPacingPlugin);
    app
}

fn dispatch(app: &mut App, updates: u32, request_redraw: bool) -> Duration {
    let start = Instant::now();
    for _ in 0..updates {
        if request_redraw {
            app.world_mut().resource_mut::<Redraw>().request();
        }
        app.update();
    }
    start.elapsed()
}

#[test]
fn moving_table_dispatch_is_left_to_presentation_pacing() {
    let mut app = dispatching_app();
    app.update();
    // A 60/s cap needs at least 133 ms for eight dispatches and a 72/s cap 111 ms.
    assert!(dispatch(&mut app, 8, true) < Duration::from_millis(100));
}

#[test]
fn settled_dispatch_keeps_its_cap_when_wakeups_repeat() {
    let mut app = dispatching_app();
    settled(&mut app);
    assert!(dispatch(&mut app, 4, false) >= Duration::from_millis(60));
}

#[test]
fn decorative_dispatch_leaves_no_sleep_for_input_to_wait_behind() {
    let mut app = dispatching_app();
    settled(&mut app);
    let start = Instant::now();
    for _ in 0..8 {
        decorate(&mut app);
    }
    // Even a 60/s sleep cap would need more than 100 ms for eight dispatches.
    assert!(start.elapsed() < Duration::from_millis(100));
    assert_eq!(pace(&app), Pace::Decorative);
}
