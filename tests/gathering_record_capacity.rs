//! Storage-capacity witness through actual preparation and legal public choices.
//! This deliberately passive policy is not a gameplay or strategy oracle.
use arkham_unfolded_experiment::gathering::{
    HostileFixture, Observation, Session,
    continuity::{self},
};

fn next_choice(view: &Observation) -> String {
    let preferred = [
        "lead:tony",
        "confirm-mulligan",
        "test-card:pass",
        "phase-window:pass",
        "window:pass",
        "commit:confirm",
        "test:reveal",
        "lucky:decline",
        "success:pass",
        "after-test:pass",
        "harm:confirm",
        "harm:damage:investigator",
        "harm:horror:investigator",
        "action:ordinary:resource",
        "turn:end",
        "enemy:attacks",
        "round:end",
        "round-end:decline-act",
        "mythos:begin",
        "mythos:encounters",
        "revelation:resolve",
        "agenda:discard",
        "peter:decline",
    ];
    preferred
        .into_iter()
        .find_map(|id| view.choices.iter().find(|choice| choice.id == id))
        .or_else(|| {
            view.choices
                .iter()
                .find(|choice| choice.id.contains("decline") || choice.id.ends_with(":pass"))
        })
        .or_else(|| {
            view.choices.iter().find(|choice| {
                [
                    "turn:",
                    "upkeep:",
                    "result:",
                    "enemy:",
                    "mythos:",
                    "turn-end:",
                    "round-end:",
                    "revelation-order:",
                    "agenda:",
                    "discard-card:",
                    "quarry:",
                    "elimination:",
                ]
                .iter()
                .any(|prefix| choice.id.starts_with(prefix))
            })
        })
        .or_else(|| view.choices.iter().find(|choice| choice.id != "undo"))
        .expect(
            "The capacity witness must reach a real outcome rather than an unsupported boundary",
        )
        .id
        .clone()
}

fn round_trip(session: &Session, path: &std::path::Path) {
    continuity::save_session(path, session)
        .expect("This actual Gathering continuation must fit the existing record bound");
    assert_eq!(continuity::load_session(path).unwrap(), *session);
    assert!(std::fs::metadata(path).unwrap().len() <= 8 * 1024 * 1024);

    let bytes = std::fs::read(path).unwrap();
    assert!(
        bytes.len() < 256 * 1024,
        "Current-state saves should stay small: {} bytes",
        bytes.len()
    );
}

#[test]
fn actual_setup_through_terminal_defeat_keeps_compact_current_saves() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gathering.session.json");
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
    let mut checked = Vec::new();
    for _ in 0..400 {
        let view = session.observe();
        if matches!(view.revision, 213..=215) {
            round_trip(&session, &path);
            checked.push(view.revision);
        }
        if view
            .play
            .as_ref()
            .is_some_and(|play| play.scenario.outcome.is_some())
        {
            assert_eq!(
                checked,
                vec![213, 214, 215],
                "Exercise both sides of the measured old size failure"
            );
            round_trip(&session, &path);
            // Formatting is not record identity: a preserved pretty JSON file
            // with the same payload/checksum remains readable.
            let pretty_path = directory.path().join("small-old-style.session.json");
            let mut fresh = Session::with_hostile_fixture(HostileFixture::GatheringFromSetup);
            fresh.choose(0, "lead:tony").unwrap();
            continuity::save_session(&pretty_path, &fresh).unwrap();
            let fresh_value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&pretty_path).unwrap()).unwrap();
            std::fs::write(
                &pretty_path,
                serde_json::to_vec_pretty(&fresh_value).unwrap(),
            )
            .unwrap();
            assert_eq!(continuity::load_session(&pretty_path).unwrap(), fresh);
            return;
        }
        let id = next_choice(&view);
        session.choose(view.revision, &id).unwrap();
    }
    panic!("The bounded real-setup capacity witness did not reach its observed terminal outcome");
}
