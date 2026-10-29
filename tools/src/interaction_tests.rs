use super::*;
use anyhow::ensure;
use serde_json::json;
use std::{cell::RefCell, fs, rc::Rc};

struct State {
    geometry: Geometry,
    root: [f64; 2],
    mask: u16,
    focused: bool,
    presses: Vec<(bool, i32)>,
    moves: Vec<[f64; 2]>,
    starts: usize,
    closes: usize,
    identity: bool,
    extent: [f64; 2],
    start_error: bool,
    press_error: bool,
    release_error_once: bool,
    held: Option<i32>,
    change_on_move: Option<(usize, &'static str)>,
    change_on_press: Option<&'static str>,
}

fn state() -> Rc<RefCell<State>> {
    Rc::new(RefCell::new(State {
        geometry: Geometry {
            left: 100.0,
            top: 80.0,
            width: 1600.0,
            height: 900.0,
        },
        root: [0.0, 0.0],
        mask: 0,
        focused: false,
        presses: vec![],
        moves: vec![],
        starts: 0,
        closes: 0,
        identity: true,
        extent: [1920.0, 1080.0],
        start_error: false,
        press_error: false,
        release_error_once: false,
        held: None,
        change_on_move: None,
        change_on_press: None,
    }))
}
struct Screen(Rc<RefCell<State>>);
impl Desktop for Screen {
    fn focus(&mut self) -> Result<()> {
        self.0.borrow_mut().focused = true;
        Ok(())
    }
    fn verify(&mut self) -> Result<()> {
        ensure!(self.0.borrow().focused, "lost focus");
        ensure!(self.0.borrow().identity, "changed identity");
        Ok(())
    }
    fn geometry(&mut self) -> Result<Geometry> {
        Ok(self.0.borrow().geometry)
    }
    fn pointer(&mut self) -> Result<Pointer> {
        let state = self.0.borrow();
        Ok(Pointer {
            root: state.root,
            client: [
                state.root[0] - state.geometry.left,
                state.root[1] - state.geometry.top,
            ],
            mask: state.mask,
        })
    }
    fn capture(&mut self, path: &Path) -> Result<Value> {
        self.verify()?;
        fs::write(path, "test boundary image")?;
        Ok(json!({"path":path}))
    }
}
struct Grant(Rc<RefCell<State>>);
impl Portal for Grant {
    fn start(&mut self, _emit: &mut dyn FnMut(Value) -> Result<()>) -> Result<()> {
        self.0.borrow_mut().starts += 1;
        ensure!(!self.0.borrow().start_error, "partial startup error");
        Ok(())
    }
    fn size(&self) -> [f64; 2] {
        self.0.borrow().extent
    }
    fn move_to(&mut self, point: [f64; 2]) -> Result<()> {
        let mut state = self.0.borrow_mut();
        state.root = [point[0] * 1.5 - 100.0, point[1] * 1.25 - 50.0];
        state.moves.push(point);
        if let Some((count, change)) = state.change_on_move
            && count == state.moves.len()
        {
            change_state(&mut state, change);
        }
        Ok(())
    }
    fn button(&mut self, pressed: bool, button: i32) -> Result<()> {
        let mut state = self.0.borrow_mut();
        state.presses.push((pressed, button));
        if !pressed && state.release_error_once {
            state.release_error_once = false;
            bail!("release interrupted");
        }
        let bit = if button == 273 { 1 << 10 } else { 1 << 8 };
        if pressed {
            state.mask |= bit;
            state.held = Some(button);
            if let Some(change) = state.change_on_press {
                change_state(&mut state, change);
            }
            ensure!(!state.press_error, "press interrupted");
        } else {
            state.mask &= !bit;
            state.held = None;
        }
        Ok(())
    }
    fn close(&mut self) -> Result<()> {
        let held = self.0.borrow().held;
        if let Some(button) = held {
            self.button(false, button)?;
        }
        self.0.borrow_mut().closes += 1;
        Ok(())
    }
}

fn change_state(state: &mut State, change: &str) {
    match change {
        "focus" => state.focused = false,
        "identity" => state.identity = false,
        "geometry" => state.geometry.width += 1.0,
        "mapping" => state.root[0] += 30.0,
        "button" => state.mask |= 1 << 8,
        _ => panic!("unknown test change"),
    }
}

#[test]
fn release_failure_ends_session_and_retries_release_during_cleanup() {
    let shared = state();
    shared.borrow_mut().release_error_once = true;
    let directory = tempfile::tempdir().unwrap();
    let result = run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut [Ok(
            r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30}"#.to_owned(),
        )]
        .into_iter(),
        &mut |_| Ok(()),
        &mut |_| Ok(()),
    );
    assert!(result.is_err(), "release failure must end this session");
    let state = shared.borrow();
    assert_eq!(state.presses, [(true, 273), (false, 273), (false, 273)]);
    assert_eq!(state.mask, 0);
    assert_eq!(state.closes, 1);
}

#[test]
fn session_measures_mapping_refuses_unsafe_click_and_recovers_with_one_grant_until_eof() {
    let shared = state();
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    let commands = [
        r#"{"action":"click","x":-1,"y":300}"#,
        r#"{"action":"click","x":400,"y":300}"#,
        r#"{"action":"capture","name":"after.png"}"#,
    ];
    run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut commands.into_iter().map(|line| Ok(line.to_owned())),
        &mut |event| {
            events.push(event);
            Ok(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
    let statuses: Vec<_> = events
        .iter()
        .map(|e| e["status"].as_str().unwrap())
        .collect();
    assert_eq!(
        statuses,
        [
            "capture",
            "ready",
            "refused",
            "clicked",
            "capture",
            "session-closed"
        ]
    );
    let state = shared.borrow();
    assert_eq!((state.starts, state.closes), (1, 1));
    assert_eq!(state.presses, [(true, 272), (false, 272)]);
    assert_eq!(state.root, [500.0, 380.0]);
    assert!(directory.path().join("initial.png").is_file());
    assert!(directory.path().join("after.png").is_file());
}

#[test]
fn bounded_right_drag_reaches_the_endpoint_and_releases_the_right_button() {
    let shared = state();
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    let commands =
        [r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30,"duration_seconds":0.6}"#];
    run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut commands.into_iter().map(|line| Ok(line.to_owned())),
        &mut |event| {
            events.push(event);
            Ok(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
    assert!(
        events.iter().any(|e| e["status"] == "right-dragged"),
        "{events:?}"
    );
    let state = shared.borrow();
    assert_eq!(state.presses, [(true, 273), (false, 273)]);
    assert_eq!(state.moves.len(), 4 + 1 + 20);
    assert_eq!(state.root, [620.0, 410.0]);
    assert_eq!(state.mask, 0);
    assert_eq!(state.closes, 1);
}

#[test]
fn malformed_nonfinite_and_unsafe_requests_do_not_move_or_press() {
    let shared = state();
    let directory = tempfile::tempdir().unwrap();
    let commands = [
        "{",
        "null",
        "[]",
        r#"{"action":"click","x":true,"y":20}"#,
        r#"{"action":"click","x":NaN,"y":20}"#,
        r#"{"action":"right-drag","x":400,"y":300,"dx":"120","dy":30}"#,
        r#"{"action":"right-drag","x":400,"y":300,"dx":0,"dy":0}"#,
        r#"{"action":"right-drag","x":400,"y":300,"dx":601,"dy":0}"#,
        r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30,"duration_seconds":0.19}"#,
        r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30,"duration_seconds":2.01}"#,
        r#"{"action":"right-drag","x":1500,"y":300,"dx":120,"dy":30}"#,
        r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":-301}"#,
        r#"{"action":"capture","name":"../outside.png"}"#,
        r#"{"action":"capture","name":"initial.png"}"#,
        r#"{"action":"capture","name":"bad.txt"}"#,
    ];
    let mut refusals = 0;
    run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut commands.into_iter().map(|line| Ok(line.to_owned())),
        &mut |event| {
            if event["status"] == "refused" {
                refusals += 1;
            }
            Ok(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(refusals, commands.len());
    assert_eq!(shared.borrow().moves.len(), 4);
    assert!(shared.borrow().presses.is_empty());
    assert_eq!(
        fs::read_to_string(directory.path().join("initial.png")).unwrap(),
        "test boundary image"
    );
}

#[test]
fn changed_focus_identity_geometry_mapping_or_buttons_prevents_a_press() {
    for action in [
        r#"{"action":"click","x":400,"y":300}"#,
        r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30}"#,
    ] {
        for change in ["focus", "identity", "geometry", "mapping", "button"] {
            let shared = state();
            shared.borrow_mut().change_on_move = Some((5, change));
            let directory = tempfile::tempdir().unwrap();
            let mut refused = false;
            run_session(
                &mut Screen(shared.clone()),
                &mut Grant(shared.clone()),
                directory.path(),
                &mut [Ok(action.to_owned())].into_iter(),
                &mut |event| {
                    refused |= event["status"] == "refused";
                    Ok(())
                },
                &mut |_| Ok(()),
            )
            .unwrap();
            assert!(refused, "{change}");
            assert!(shared.borrow().presses.is_empty(), "{change}");
            assert_eq!(shared.borrow().closes, 1);
        }
    }
}

#[test]
fn changed_state_after_press_or_mid_drag_stops_motion_and_releases() {
    for change in ["focus", "identity", "geometry", "mapping", "button"] {
        for after_press in [true, false] {
            let shared = state();
            if after_press {
                shared.borrow_mut().change_on_press = Some(change);
            } else {
                shared.borrow_mut().change_on_move = Some((6, change));
            }
            let directory = tempfile::tempdir().unwrap();
            let mut completed = false;
            run_session(
                &mut Screen(shared.clone()),
                &mut Grant(shared.clone()),
                directory.path(),
                &mut [Ok(
                    r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30}"#.to_owned(),
                )]
                .into_iter(),
                &mut |event| {
                    completed |= event["status"] == "right-dragged";
                    Ok(())
                },
                &mut |_| Ok(()),
            )
            .unwrap();
            let state = shared.borrow();
            assert!(!completed, "{change}");
            assert_eq!(state.moves.len(), if after_press { 5 } else { 6 });
            assert_eq!(state.presses, [(true, 273), (false, 273)]);
            assert_eq!(state.mask & (1 << 10), 0);
            assert_eq!(state.closes, 1);
        }
    }
}

#[test]
fn interrupted_press_or_pause_releases_and_partial_start_or_input_error_closes() {
    for point in ["press", "pause", "start", "input"] {
        let shared = state();
        shared.borrow_mut().press_error = point == "press";
        shared.borrow_mut().start_error = point == "start";
        let directory = tempfile::tempdir().unwrap();
        let mut commands = [if point == "input" {
            Err(anyhow::anyhow!("input interrupted"))
        } else {
            Ok(r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30}"#.to_owned())
        }]
        .into_iter();
        let result = run_session(
            &mut Screen(shared.clone()),
            &mut Grant(shared.clone()),
            directory.path(),
            &mut commands,
            &mut |_| Ok(()),
            &mut |_| {
                ensure!(
                    !(point == "pause" && shared.borrow().held.is_some()),
                    "pause interrupted"
                );
                Ok(())
            },
        );
        if ["start", "input"].contains(&point) {
            assert!(result.is_err());
        } else {
            assert_eq!(shared.borrow().presses, [(true, 273), (false, 273)]);
        }
        assert_eq!(shared.borrow().mask, 0);
        assert_eq!(shared.borrow().closes, 1);
    }
}

#[test]
fn failed_calibration_requires_recalibration_but_keeps_the_same_grant() {
    for independent_failure in [false, true] {
        let shared = state();
        if independent_failure {
            shared.borrow_mut().change_on_move = Some((4, "mapping"));
        } else {
            shared.borrow_mut().extent = [0.0, 0.0];
        }
        let directory = tempfile::tempdir().unwrap();
        let mut events = Vec::new();
        let mut commands = [
            r#"{"action":"calibrate"}"#,
            r#"{"action":"click","x":400,"y":300}"#,
            r#"{"action":"stop"}"#,
        ]
        .into_iter()
        .map(|line| {
            shared.borrow_mut().extent = [1920.0, 1080.0];
            Ok(line.to_owned())
        });
        run_session(
            &mut Screen(shared.clone()),
            &mut Grant(shared.clone()),
            directory.path(),
            &mut commands,
            &mut |event| {
                events.push(event);
                Ok(())
            },
            &mut |_| Ok(()),
        )
        .unwrap();
        assert_eq!(
            events
                .iter()
                .map(|e| e["status"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["capture", "refused", "ready", "clicked", "session-closed"]
        );
        assert_eq!((shared.borrow().starts, shared.borrow().closes), (1, 1));
        assert_eq!(shared.borrow().presses, [(true, 272), (false, 272)]);
    }
}

#[test]
fn held_buttons_or_off_monitor_targets_refuse_before_movement() {
    for held in [true, false] {
        let shared = state();
        let directory = tempfile::tempdir().unwrap();
        let mut commands = [r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30}"#]
            .into_iter()
            .map(|line| {
                if held {
                    shared.borrow_mut().mask = 256;
                } else {
                    shared.borrow_mut().geometry.left = 4000.0;
                }
                Ok(line.to_owned())
            });
        run_session(
            &mut Screen(shared.clone()),
            &mut Grant(shared.clone()),
            directory.path(),
            &mut commands,
            &mut |_| Ok(()),
            &mut |_| Ok(()),
        )
        .unwrap();
        assert_eq!(shared.borrow().moves.len(), 4);
        assert!(shared.borrow().presses.is_empty());
    }
}
