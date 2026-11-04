use super::*;

#[test]
fn click_capture_reports_both_results_with_one_click() {
    let shared = state();
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut [Ok(r#"{"action":"click","x":400,"y":300,"capture_after":{"name":"after.png","delay_ms":250}}"#.into())].into_iter(),
        &mut |event| { events.push(event); Ok(()) },
        &mut |_| Ok(()),
    ).unwrap();
    let result = events
        .iter()
        .find(|event| event["status"] == "clicked")
        .unwrap();
    assert_eq!(result["request"], 1);
    assert_eq!(result["capture"]["status"], "capture");
    assert!(directory.path().join("after.png").is_file());
    assert_eq!(shared.borrow().presses, [(true, 272), (false, 272)]);
    assert_eq!(shared.borrow().closes, 1);
}

#[test]
fn click_capture_rejects_bad_capture_options_before_input() {
    for after in [
        json!({"name":"../outside.png"}),
        json!({"name":"initial.png"}),
        json!({"name":"bad.txt"}),
        json!({"name":"after.png","delay_ms":12001}),
    ] {
        let shared = state();
        let directory = tempfile::tempdir().unwrap();
        let mut refused = false;
        run_session(
            &mut Screen(shared.clone()),
            &mut Grant(shared.clone()),
            directory.path(),
            &mut [Ok(
                json!({"action":"click","x":400,"y":300,"capture_after":after}).to_string(),
            )]
            .into_iter(),
            &mut |event| {
                refused |= event["status"] == "refused";
                Ok(())
            },
            &mut |_| Ok(()),
        )
        .unwrap();
        assert!(refused);
        assert!(shared.borrow().presses.is_empty());
        assert_eq!(shared.borrow().moves.len(), 4, "Only calibration moves");
    }
}

#[test]
fn click_capture_failure_preserves_input_success_without_retry() {
    let shared = state();
    shared.borrow_mut().capture_error = true;
    let directory = tempfile::tempdir().unwrap();
    let mut events = Vec::new();
    run_session(
        &mut Screen(shared.clone()),
        &mut Grant(shared.clone()),
        directory.path(),
        &mut [Ok(
            r#"{"action":"click","x":400,"y":300,"capture_after":{"name":"after.png"}}"#.into(),
        )]
        .into_iter(),
        &mut |event| {
            events.push(event);
            Ok(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
    let refusal = events
        .iter()
        .find(|event| event["status"] == "refused")
        .unwrap();
    assert_eq!(refusal["request"], 1);
    assert_eq!(refusal["input"]["status"], "clicked");
    assert_eq!(refusal["capture"]["status"], "failed");
    assert_eq!(shared.borrow().presses, [(true, 272), (false, 272)]);
    assert_eq!(shared.borrow().closes, 1);
    assert!(!directory.path().join("after.png").exists());
}

#[test]
fn click_capture_keeps_the_original_deadlines() {
    for phase in ["admission", "acknowledgement", "settle", "capture"] {
        let shared = state();
        if phase == "capture" {
            shared.borrow_mut().capture_delay = Duration::from_secs(16);
        }
        let origin = Instant::now();
        let now = || origin + shared.borrow().elapsed;
        let directory = tempfile::tempdir().unwrap();
        let mut events = Vec::new();
        let result = super::super::run_session(
            &mut Screen(shared.clone()), &mut Grant(shared.clone()), directory.path(),
            &mut [
                Ok(r#"{"action":"click","focus":true,"x":400,"y":300,"capture_after":{"name":"after.png","delay_ms":500}}"#.into()),
                Ok(r#"{"action":"click","x":400,"y":300}"#.into()),
            ].into_iter(),
            &mut |event| {
                if phase == "admission" && event["status"] == "ready" {
                    let mut state = shared.borrow_mut();
                    state.elapsed = Duration::from_secs(71);
                    state.focused = false;
                }
                if phase == "acknowledgement" && event["status"] == "command-started" {
                    let mut state = shared.borrow_mut();
                    state.elapsed += Duration::from_secs(16);
                    state.focused = false;
                }
                events.push(event); Ok(())
            },
            &mut |duration| {
                shared.borrow_mut().elapsed += if phase == "settle" && duration == Duration::from_millis(500) {
                    Duration::from_secs(16)
                } else { duration };
                Ok(())
            },
            SessionTiming { deadline: origin + Duration::from_secs(100), command_timeout: Duration::from_secs(15), now: &now },
        );
        assert!(result.is_err(), "{phase}");
        let refusal = events
            .iter()
            .find(|event| event["status"] == "refused")
            .unwrap();
        let delivered = matches!(phase, "settle" | "capture");
        assert_eq!(
            refusal["input"]["status"] == "clicked",
            delivered,
            "{phase}"
        );
        assert_eq!(
            shared.borrow().presses.len(),
            if delivered { 2 } else { 0 },
            "{phase}"
        );
        if !delivered {
            assert!(!shared.borrow().focused, "Expired input cannot focus");
        }
        assert!(
            !events.iter().any(|event| event["request"] == 2),
            "Timeout ends the session"
        );
        assert_eq!(shared.borrow().closes, 1);
    }
}
