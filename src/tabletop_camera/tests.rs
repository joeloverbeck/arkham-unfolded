use super::*;

struct Inputs {
    keys: ButtonInput<KeyCode>,
    buttons: ButtonInput<MouseButton>,
    window: Window,
    capture: Capture,
    scroll_y: f32,
    dt: f32,
    reduced_motion: bool,
}

impl Default for Inputs {
    fn default() -> Self {
        let mut window = Window {
            focused: true,
            ..default()
        };
        window.set_cursor_position(Some(Vec2::new(800.0, 400.0)));
        Self {
            keys: default(),
            buttons: default(),
            window,
            capture: default(),
            scroll_y: 0.0,
            dt: 0.1,
            reduced_motion: false,
        }
    }
}

impl Inputs {
    fn step(&mut self, orbit: &mut Orbit) -> CameraStep {
        let step = orbit.step(FrameInput {
            keys: &self.keys,
            buttons: &self.buttons,
            window: &self.window,
            scroll_y: self.scroll_y,
            dt: self.dt,
            capture: self.capture,
            reduced_motion: self.reduced_motion,
        });
        self.buttons.clear();
        self.keys.clear();
        self.scroll_y = 0.0;
        step
    }

    fn move_cursor(&mut self, delta: Vec2) {
        let position = self.window.cursor_position().unwrap() + delta;
        self.window.set_cursor_position(Some(position));
    }
}

#[test]
fn scene_drag_uses_logical_cursor_displacement_and_crosses_ui_until_release() {
    let mut orbit = Orbit::new(CameraMode::Jacqueline);
    let mut input = Inputs::default();
    input.buttons.press(MouseButton::Right);
    assert!(!input.step(&mut orbit).manual_navigation);
    assert_eq!(orbit.pose().yaw, 0.0, "the press only captures the cursor");
    input.capture.over_ui = true;
    input.move_cursor(Vec2::new(40.0, 10.0));
    assert!(input.step(&mut orbit).manual_navigation);
    assert!((orbit.pose().yaw + 0.24).abs() < 0.00001);
    assert!((orbit.pose().pitch - 1.04).abs() < 0.00001);
    let moved = orbit.pose();
    input.buttons.release(MouseButton::Right);
    input.move_cursor(Vec2::splat(40.0));
    assert!(!input.step(&mut orbit).manual_navigation);
    assert_eq!(orbit.pose(), moved);
}

#[test]
fn drag_begun_on_ui_requires_a_fresh_scene_press() {
    let mut orbit = Orbit::new(CameraMode::Tony);
    let mut input = Inputs::default();
    let initial = orbit.pose();
    input.capture.over_ui = true;
    input.buttons.press(MouseButton::Right);
    input.step(&mut orbit);
    input.capture.over_ui = false;
    input.move_cursor(Vec2::splat(40.0));
    input.step(&mut orbit);
    assert_eq!(orbit.pose(), initial);
    input.buttons.release(MouseButton::Right);
    input.step(&mut orbit);
    input.buttons.press(MouseButton::Right);
    input.step(&mut orbit);
    input.move_cursor(Vec2::splat(40.0));
    assert!(input.step(&mut orbit).manual_navigation);
}

#[test]
fn interruptions_cancel_capture_without_a_resume_jump() {
    for interruption in 0..3 {
        let mut orbit = Orbit::new(CameraMode::Tony);
        let mut input = Inputs::default();
        input.buttons.press(MouseButton::Right);
        input.step(&mut orbit);
        input.move_cursor(Vec2::splat(10.0));
        input.step(&mut orbit);
        let before = orbit.pose();
        match interruption {
            0 => input.window.focused = false,
            1 => input.capture.pointer_blocked = true,
            _ => input.window.set_cursor_position(None),
        }
        input.step(&mut orbit);
        input.window.focused = true;
        input.capture.pointer_blocked = false;
        input
            .window
            .set_cursor_position(Some(Vec2::new(1000.0, 500.0)));
        input.step(&mut orbit);
        assert_eq!(orbit.pose(), before, "interruption {interruption}");
        input.buttons.release(MouseButton::Right);
        input.step(&mut orbit);
        input.buttons.press(MouseButton::Right);
        input.step(&mut orbit);
        input.move_cursor(Vec2::splat(10.0));
        assert!(input.step(&mut orbit).manual_navigation);
    }
}

#[test]
fn pan_is_camera_relative_normalized_and_stops_on_release() {
    let mut orbit = Orbit::new(CameraMode::Tony);
    // Exercise navigation at a chosen zoom independently of the Home preset.
    orbit.focus((Vec3::ZERO, 16.0));
    let mut input = Inputs::default();
    let start = orbit.pose();
    input.keys.press(KeyCode::KeyW);
    input.step(&mut orbit);
    let panned = orbit.pose();
    assert!((panned.target.z - (start.target.z - 0.88)).abs() < 0.00001);
    assert_eq!(panned.pitch, start.pitch);
    assert_eq!(panned.distance, start.distance);
    input.keys.press(KeyCode::KeyD);
    input.step(&mut orbit);
    assert!((orbit.pose().target.distance(panned.target) - 0.88).abs() < 0.00001);
    input.keys.reset_all();
    let stopped = orbit.pose();
    input.step(&mut orbit);
    assert_eq!(orbit.pose(), stopped);
    input.keys.press(KeyCode::ArrowLeft);
    input.dt = std::f32::consts::FRAC_PI_2;
    input.step(&mut orbit);
    input.keys.reset_all();
    input.keys.press(KeyCode::KeyW);
    input.dt = 0.1;
    input.step(&mut orbit);
    let delta = orbit.pose().target - stopped.target;
    assert!((delta.x + 0.88).abs() < 0.00001 && delta.z.abs() < 0.00001);
}

#[test]
fn keyboard_capture_focus_loss_and_modified_wasd_preserve_the_pose() {
    let mut orbit = Orbit::new(CameraMode::Tony);
    let mut input = Inputs::default();
    orbit.focus(layout::focus(Area::Scenario));
    let before = orbit.pose();
    for key in [
        KeyCode::KeyW,
        KeyCode::ArrowLeft,
        KeyCode::Equal,
        KeyCode::Home,
    ] {
        input.keys.press(key);
    }
    input.capture.keyboard = true;
    input.step(&mut orbit);
    assert_eq!(orbit.pose(), before);
    input.capture.keyboard = false;
    input.window.focused = false;
    input.step(&mut orbit);
    assert_eq!(orbit.pose(), before);
    input.window.focused = true;
    for modifier in [
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ] {
        input.keys.reset_all();
        input.keys.press(KeyCode::KeyW);
        input.keys.press(modifier);
        assert!(!input.step(&mut orbit).manual_navigation);
        assert_eq!(orbit.pose(), before);
    }
}

#[test]
fn manual_navigation_overrides_following_and_freezes_pending_framing() {
    for mode in [CameraMode::Jacqueline, CameraMode::Tony] {
        for gesture in 0..5 {
            let mut orbit = Orbit::new(mode);
            let mut input = Inputs::default();
            orbit.set_following(true);
            orbit.update_follow_target(Some(layout::focus(Area::Scenario)));
            match gesture {
                0 => {
                    input.buttons.press(MouseButton::Right);
                    input.step(&mut orbit);
                    input.move_cursor(Vec2::splat(10.0));
                }
                1 => input.scroll_y = 1.0,
                2 => input.keys.press(KeyCode::ArrowLeft),
                3 => input.keys.press(KeyCode::KeyW),
                _ => input.keys.press(KeyCode::Home),
            }
            let step = input.step(&mut orbit);
            assert!(step.manual_navigation && step.stopped_following);
            assert!(!orbit.is_following());
            input.keys.reset_all();
            let stopped = orbit.pose();
            assert!(!input.step(&mut orbit).manual_navigation);
            assert_eq!(orbit.pose(), stopped);
        }
    }
}

#[test]
fn each_mode_preserves_startup_home_focus_and_camera_transform() {
    for (mode, target, distance) in [
        (CameraMode::Jacqueline, Vec3::new(0.0, 0.0, 13.0), 66.0),
        (CameraMode::Tony, Vec3::new(-18.6, 0.0, 23.2), 26.0),
    ] {
        let mut orbit = Orbit::new(mode);
        let home = orbit.pose();
        assert_eq!(
            home,
            Pose {
                yaw: 0.0,
                pitch: 1.0,
                target,
                distance
            }
        );
        assert!(!orbit.is_following());
        orbit.focus(layout::focus(Area::Test));
        assert_eq!(orbit.pose().target, Vec3::new(0.0, 0.0, 8.5));
        assert_eq!(orbit.pose().distance, 20.0);
        let mut input = Inputs::default();
        input.keys.press(KeyCode::Home);
        let step = input.step(&mut orbit);
        assert_eq!(orbit.pose(), home);
        assert!((step.transform.translation.distance(target) - distance).abs() < 0.00001);
        assert!(
            step.transform
                .forward()
                .dot((target - step.transform.translation).normalize())
                > 0.99999
        );
    }
}

#[test]
fn diagnostic_initial_views_apply_once_without_replacing_home_or_replay_following() {
    for mode in [CameraMode::Jacqueline, CameraMode::Tony] {
        let normal = Orbit::new(mode).pose();
        assert_eq!(Orbit::new(mode).with_initial_view(None).pose(), normal);
        for view in [
            Area::Overview,
            Area::Scenario,
            Area::Test,
            Area::Investigator(0),
            Area::Investigator(1),
            Area::Investigator(2),
        ] {
            let (target, distance) = layout::focus(view);
            let mut orbit = Orbit::new(mode).with_initial_view(Some(view));
            let mut input = Inputs::default();
            let first_frame = input.step(&mut orbit);
            assert_eq!(orbit.pose().target, target);
            assert_eq!(orbit.pose().distance, distance);
            assert!(!orbit.is_following());
            assert!(!orbit.is_moving());
            assert!(
                (first_frame.transform.translation.distance(target) - distance).abs() < 0.00001
            );
            assert!(
                first_frame
                    .transform
                    .forward()
                    .dot((target - first_frame.transform.translation).normalize())
                    > 0.99999
            );

            orbit.set_following(true);
            let replay_target = Vec3::new(3.0, 0.0, 7.0);
            orbit.update_follow_target(Some((replay_target, 15.0)));
            input.reduced_motion = true;
            input.dt = 1.0;
            input.step(&mut orbit);
            assert_eq!(orbit.pose().target, replay_target);
            assert!(orbit.is_following());

            input.keys.press(KeyCode::Home);
            input.step(&mut orbit);
            assert_eq!(orbit.pose(), normal);
            assert!(!orbit.is_following());
            input.keys.release(KeyCode::Home);
            input.step(&mut orbit);
            assert_eq!(
                orbit.pose(),
                normal,
                "startup framing must not be reapplied"
            );
        }
    }
}

#[test]
fn replay_motion_and_follow_off_keep_the_modes_distinct() {
    for mode in [CameraMode::Jacqueline, CameraMode::Tony] {
        let mut orbit = Orbit::new(mode);
        let mut input = Inputs::default();
        orbit.focus(layout::focus(Area::Overview));
        orbit.set_following(true);
        orbit.update_follow_target(Some(layout::focus(Area::Test)));
        let smooth_zoom = matches!(mode, CameraMode::Jacqueline);
        assert_eq!(orbit.pose().distance, if smooth_zoom { 66.0 } else { 20.0 });
        assert!(!input.step(&mut orbit).manual_navigation);
        assert!((orbit.pose().target.z - 11.2).abs() < 0.00001);
        assert!((orbit.pose().distance - if smooth_zoom { 47.6 } else { 20.0 }).abs() < 0.00001);
        orbit.set_following(false);
        orbit.update_follow_target(None);
        input.step(&mut orbit);
        assert!((orbit.pose().target.z - if smooth_zoom { 11.2 } else { 10.12 }).abs() < 0.00001);
        orbit.set_following(true);
        orbit.update_follow_target(Some(layout::focus(Area::Test)));
        input.reduced_motion = true;
        input.step(&mut orbit);
        if smooth_zoom {
            assert!((orbit.pose().target.z - 10.12).abs() < 0.00001);
        } else {
            assert_eq!(orbit.pose().target, layout::focus(Area::Test).0);
        }
    }
}

#[test]
fn wheel_capture_and_existing_clamp_order_are_preserved() {
    for (mode, maximum) in [(CameraMode::Jacqueline, 90.0), (CameraMode::Tony, 90.0)] {
        let mut orbit = Orbit::new(mode);
        let mut input = Inputs {
            dt: 0.0,
            ..default()
        };
        let initial = orbit.pose();
        for capture in [
            Capture {
                wheel: true,
                ..default()
            },
            Capture {
                over_ui: true,
                ..default()
            },
            Capture {
                pointer_blocked: true,
                ..default()
            },
        ] {
            input.capture = capture;
            input.scroll_y = 100.0;
            assert!(!input.step(&mut orbit).manual_navigation);
            assert_eq!(orbit.pose(), initial);
        }
        input.capture = default();
        input.scroll_y = -200.0;
        input.step(&mut orbit);
        assert_eq!(orbit.pose().distance, maximum);
        input.scroll_y = 300.0;
        input.step(&mut orbit);
        assert_eq!(orbit.pose().distance, 6.0);
        input.keys.press(KeyCode::ArrowUp);
        input.dt = 0.1;
        for _ in 0..20 {
            input.step(&mut orbit);
        }
        assert_eq!(orbit.pose().pitch, 1.35);
        input.keys.reset_all();
        input.keys.press(KeyCode::ArrowDown);
        for _ in 0..20 {
            input.step(&mut orbit);
        }
        assert_eq!(orbit.pose().pitch, 0.35);
    }
}
