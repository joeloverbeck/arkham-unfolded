use super::*;
use anyhow::ensure;
use serde_json::json;
use std::{cell::RefCell, fs, rc::Rc};

fn run_session(
    desktop: &mut dyn Desktop,
    portal: &mut dyn Portal,
    directory: &Path,
    commands: &mut dyn Iterator<Item = Result<String>>,
    emit: &mut dyn FnMut(Value) -> Result<()>,
    pause: &mut dyn FnMut(Duration) -> Result<()>,
) -> Result<()> {
    super::run_session(
        desktop,
        portal,
        directory,
        commands,
        emit,
        pause,
        SessionTiming {
            deadline: Instant::now() + Duration::from_secs(600),
            command_timeout: Duration::from_secs(15),
            now: &Instant::now,
        },
    )
}

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
    elapsed: Duration,
    start_delay: Duration,
    focus_delay: Duration,
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
        elapsed: Duration::ZERO,
        start_delay: Duration::ZERO,
        focus_delay: Duration::ZERO,
    }))
}
struct Screen(Rc<RefCell<State>>);
impl Desktop for Screen {
    fn focus(&mut self) -> Result<()> {
        let mut state = self.0.borrow_mut();
        state.focused = true;
        let delay = state.focus_delay;
        state.elapsed += delay;
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
        let mut state = self.0.borrow_mut();
        state.starts += 1;
        let delay = state.start_delay;
        state.elapsed += delay;
        drop(state);
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
fn each_command_is_acknowledged_and_has_a_numbered_terminal_response() {
    let shared = state();
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared),
        directory.path(),
        &mut [Ok("not json".into()), Ok(r#"{"action":"stop"}"#.into())].into_iter(),
        &mut |event| {
            events.push(event);
            Ok(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
    for (request, terminal) in [(1, "refused"), (2, "stopped")] {
        let responses: Vec<_> = events
            .iter()
            .filter(|event| event["request"] == request)
            .collect();
        assert_eq!(responses.len(), 2, "{events:?}");
        assert_eq!(responses[0]["status"], "command-started");
        assert_eq!(responses[1]["status"], terminal);
        assert!(responses[1]["elapsed_ms"].is_number());
    }
    assert_eq!(events.last().unwrap()["status"], "session-closed");
}

#[test]
fn delayed_consent_cannot_start_calibration_in_the_cleanup_reserve() {
    let shared = state();
    shared.borrow_mut().start_delay = Duration::from_secs(90);
    let origin = Instant::now();
    let now = || origin + shared.borrow().elapsed;
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    let result = super::run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut [Ok(
            r#"{"action":"click","focus":true,"x":400,"y":300}"#.into()
        )]
        .into_iter(),
        &mut |event| {
            events.push(event);
            Ok(())
        },
        &mut |duration| {
            shared.borrow_mut().elapsed += duration;
            Ok(())
        },
        SessionTiming {
            deadline: origin + Duration::from_secs(100),
            command_timeout: Duration::from_secs(15),
            now: &now,
        },
    );
    assert!(
        result.is_err(),
        "late consent must not restart the interaction budget"
    );
    let state = shared.borrow();
    assert!(
        state.moves.is_empty(),
        "calibration must not consume cleanup time"
    );
    assert!(state.presses.is_empty());
    assert_eq!(state.closes, 1);
    assert_eq!(events.last().unwrap()["status"], "session-closed");
}

#[test]
fn late_command_refuses_before_focus_or_input_and_closes_the_grant() {
    let shared = state();
    let origin = Instant::now();
    let now = || origin + shared.borrow().elapsed;
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    let result = super::run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut [Ok(
            r#"{"action":"click","focus":true,"x":400,"y":300}"#.into()
        )]
        .into_iter(),
        &mut |event| {
            if event["status"] == "ready" {
                let mut state = shared.borrow_mut();
                state.elapsed = Duration::from_secs(71);
                state.focused = false;
            }
            events.push(event);
            Ok(())
        },
        &mut |duration| {
            shared.borrow_mut().elapsed += duration;
            Ok(())
        },
        SessionTiming {
            deadline: origin + Duration::from_secs(100),
            command_timeout: Duration::from_secs(15),
            now: &now,
        },
    );
    assert!(result.is_err());
    let state = shared.borrow();
    assert!(
        !state.focused,
        "refusal must precede optional focus acquisition"
    );
    assert!(state.presses.is_empty());
    assert_eq!(
        state.moves.len(),
        4,
        "only the initial calibration may move the pointer"
    );
    assert_eq!(state.closes, 1);
    assert!(events.iter().any(
        |event| event["status"] == "command-started" && event["session_remaining_ms"] == 29000
    ));
    assert!(
        events
            .iter()
            .any(|event| event["status"] == "refused" && event["request"] == 1)
    );
    assert_eq!(events.last().unwrap()["status"], "session-closed");
}

#[test]
fn slow_initial_focus_cannot_start_a_capture_or_consent_after_its_budget() {
    let shared = state();
    shared.borrow_mut().focus_delay = Duration::from_secs(86);
    let origin = Instant::now();
    let now = || origin + shared.borrow().elapsed;
    let directory = tempfile::tempdir().unwrap();
    let result = super::run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut std::iter::empty(),
        &mut |_| Ok(()),
        &mut |_| Ok(()),
        SessionTiming {
            deadline: origin + Duration::from_secs(100),
            command_timeout: Duration::from_secs(15),
            now: &now,
        },
    );
    assert!(result.is_err());
    assert!(
        !directory.path().join("initial.png").exists(),
        "expired startup must not capture"
    );
    let state = shared.borrow();
    assert_eq!(state.starts, 0);
    assert!(state.moves.is_empty());
    assert_eq!(state.closes, 1);
}

#[test]
fn slow_command_acknowledgement_cannot_renew_the_input_budget() {
    let shared = state();
    let origin = Instant::now();
    let now = || origin + shared.borrow().elapsed;
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    let result = super::run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut [Ok(
            r#"{"action":"click","focus":true,"x":400,"y":300}"#.into()
        )]
        .into_iter(),
        &mut |event| {
            if event["status"] == "ready" {
                shared.borrow_mut().focused = false;
            }
            if event["status"] == "command-started" {
                shared.borrow_mut().elapsed += Duration::from_secs(16);
            }
            events.push(event);
            Ok(())
        },
        &mut |duration| {
            shared.borrow_mut().elapsed += duration;
            Ok(())
        },
        SessionTiming {
            deadline: origin + Duration::from_secs(100),
            command_timeout: Duration::from_secs(15),
            now: &now,
        },
    );
    assert!(
        result.is_err(),
        "an acknowledgement delay must consume the command budget"
    );
    let state = shared.borrow();
    assert!(
        !state.focused,
        "the expired command must not reacquire focus"
    );
    assert!(state.presses.is_empty());
    assert_eq!(
        state.moves.len(),
        4,
        "only calibration may have moved the pointer"
    );
    assert_eq!(state.closes, 1);
    assert!(
        events
            .iter()
            .any(|event| event["status"] == "refused" && event["request"] == 1)
    );
    assert_eq!(events.last().unwrap()["status"], "session-closed");
}

#[test]
fn command_timeout_releases_a_pressed_button_and_ends_before_the_next_command() {
    let shared = state();
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    let result = super::run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut [
            Ok(r#"{"action":"right-drag","x":400,"y":300,"dx":120,"dy":30}"#.into()),
            Ok(r#"{"action":"click","x":400,"y":300}"#.into()),
        ]
        .into_iter(),
        &mut |event| {
            events.push(event);
            Ok(())
        },
        &mut |_| {
            if shared.borrow().held.is_some() {
                std::thread::sleep(Duration::from_millis(1100));
            }
            Ok(())
        },
        SessionTiming {
            deadline: Instant::now() + Duration::from_secs(600),
            command_timeout: Duration::from_secs(1),
            now: &Instant::now,
        },
    );
    assert!(result.unwrap_err().is::<CommandTimedOut>());
    assert!(events.iter().any(|event| event["request"] == 1
        && event["status"] == "refused"
        && event["reason"].as_str().unwrap().contains("timed out")));
    assert!(!events.iter().any(|event| event["request"] == 2));
    assert_eq!(events.last().unwrap()["status"], "session-closed");
    let state = shared.borrow();
    assert_eq!(state.presses, [(true, 273), (false, 273)]);
    assert_eq!(
        state.moves.len(),
        5,
        "timeout must prevent the first held movement"
    );
    assert_eq!(state.mask, 0);
    assert_eq!(state.closes, 1);
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
            "command-started",
            "refused",
            "command-started",
            "clicked",
            "command-started",
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
fn explicitly_focused_drag_recovers_between_commands_but_refuses_lost_focus_mid_drag() {
    for interrupted in [false, true] {
        let shared = state();
        let directory = tempfile::tempdir().unwrap();
        let mut events = Vec::new();
        let command = r#"{"action":"right-drag","focus":true,"x":400,"y":300,"dx":120,"dy":30,"duration_seconds":0.6}"#;
        let mut commands = std::iter::once_with(|| {
            let mut state = shared.borrow_mut();
            state.focused = false;
            if interrupted {
                state.change_on_move = Some((6, "focus"));
            }
            Ok(command.to_owned())
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
        let statuses: Vec<_> = events
            .iter()
            .filter_map(|event| event["status"].as_str())
            .collect();
        assert!(
            statuses.contains(&if interrupted {
                "refused"
            } else {
                "right-dragged"
            }),
            "{events:?}"
        );
        let state = shared.borrow();
        assert_eq!(state.presses, [(true, 273), (false, 273)]);
        assert_eq!(state.mask, 0);
        assert_eq!(state.closes, 1);
    }
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
            [
                "capture",
                "refused",
                "command-started",
                "ready",
                "command-started",
                "clicked",
                "command-started",
                "stopped",
                "session-closed"
            ]
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
