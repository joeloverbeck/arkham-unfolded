//! Arkham's framing over the shared camera. The camera's motion, capture and
//! follow behaviour is held in unfolded-tabletop's `tests/camera.rs`.

use super::*;

struct Inputs {
    keys: ButtonInput<KeyCode>,
    buttons: ButtonInput<MouseButton>,
    window: Window,
    capture: Capture,
    scroll_y: f32,
    dt: f32,
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
        });
        self.buttons.clear();
        self.keys.clear();
        self.scroll_y = 0.0;
        step
    }
}

#[test]
fn gathering_preserves_startup_home_focus_and_camera_transform() {
    let target = Vec3::new(0.0, 0.0, 7.0);
    let distance = 72.0;
    let mut orbit = overview();
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

#[test]
fn diagnostic_initial_views_apply_once_without_replacing_home_or_live_following() {
    let normal = overview().pose();
    for view in [
        Area::Overview,
        Area::Scenario,
        Area::Test,
        Area::Investigator(0),
        Area::Investigator(1),
        Area::Investigator(2),
    ] {
        let (target, distance) = layout::focus(view);
        let mut orbit = overview();
        orbit.focus(layout::focus(view));
        let mut input = Inputs::default();
        let first_frame = input.step(&mut orbit);
        assert_eq!(orbit.pose().target, target);
        assert_eq!(orbit.pose().distance, distance);
        assert!(!orbit.is_following());
        assert!(!orbit.is_moving());
        assert!((first_frame.transform.translation.distance(target) - distance).abs() < 0.00001);
        assert!(
            first_frame
                .transform
                .forward()
                .dot((target - first_frame.transform.translation).normalize())
                > 0.99999
        );

        orbit.set_following(true);
        let follow_target = Vec3::new(3.0, 0.0, 7.0);
        orbit.update_follow_target(Some((follow_target, 15.0).into()));
        input.dt = 1.0;
        input.step(&mut orbit);
        assert_eq!(orbit.pose().target, follow_target);
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
