use super::*;

// Reproduce the ordinary seed-89 saves used in the 2026-11-03 native review.
// Load through native startup before installing the production scene and UI.
fn open_review_save(session: &Session) -> (tempfile::TempDir, App) {
    assert!(!session.is_verification_fixture());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("review.json");
    continuity::save_session(&path, session).unwrap();
    let saved = std::fs::read(&path).unwrap();
    let mut app = host_with_table(Table::open(path.clone()).unwrap(), false);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<Table>().session.recording(),
        session.recording()
    );
    assert_eq!(std::fs::read(path).unwrap(), saved);
    (directory, app)
}

fn assert_visible_button(app: &mut App, label: &str) {
    let entity = button_with_label(app, label).unwrap_or_else(|| panic!("Missing button: {label}"));
    assert!(
        app.world()
            .get::<InheritedVisibility>(entity)
            .unwrap()
            .get()
    );
    assert!(
        app.world()
            .get::<ComputedNode>(entity)
            .unwrap()
            .size()
            .min_element()
            > 0.
    );
}

#[test]
fn quick_thinking_review_saves_open_on_reveal_then_the_untimed_result() {
    // Reproduce the owner's ordinary recorded prefix used for the native review.
    let mut session = Session::with_seed(17_952_077_857_827_163_552);
    for id in [
        "lead:patrice",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:tony",
        "action:ordinary:play:23",
        "action:ordinary:investigate",
        "commit:6",
        "commit:106",
        "commit:confirm",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    assert_eq!(session.observe().revision, 10);
    let (_before_directory, mut before) = open_review_save(&session);
    assert!(!before.world().resource::<Table>().modal());
    assert_visible_button(&mut before, "Reveal chaos token");

    session.choose(10, "test:reveal").unwrap();
    let (_after_directory, mut after) = open_review_save(&session);
    let table = after.world().resource::<Table>();
    let result = table.results.current().expect("Saved result needs reading");
    assert_eq!((result.total, result.difficulty), (7, 2));
    assert!(result.breakdown.is_some());
    assert_visible_button(&mut after, "Continue");
    let button = button_with_label(&mut after, "Continue").unwrap();
    after.world_mut().trigger(Activate { entity: button });
    after.update();
    assert_visible_button(&mut after, "Take Tony's extra action (Quick Thinking)");
    assert_visible_button(&mut after, "Decline Quick Thinking");
    assert!(button_with_label(&mut after, "Continue").is_none());
    assert_eq!(
        after.world().resource::<Table>().session.recording(),
        session.recording()
    );
}

fn tony_turn() -> Session {
    let mut session = Session::with_seed(89);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:tony",
    ] {
        session
            .choose(session.observe_for_play().revision, id)
            .unwrap();
    }
    session
}

#[test]
fn initial_and_hard_knocks_review_saves_open_on_the_expected_actions() {
    let (_directory, mut initial) = open_review_save(&Session::with_seed(89));
    assert_initial_table(&mut initial);
    assert!(!initial.world().resource::<Table>().modal());
    assert!(button_with_label(&mut initial, "Continue").is_none());
    click_source(
        &mut initial,
        controls::Source::Investigator(Investigator::Tony),
    );
    for _ in 0..3 {
        initial.update();
    }
    assert_visible_button(&mut initial, "Choose Tony Morgan as lead investigator");

    let (_directory, mut ready) = open_review_save(&tony_turn());
    assert!(!ready.world().resource::<Table>().modal());
    assert!(!ready.world().resource::<Table>().is_presenting());
    assert!(button_with_label(&mut ready, "Continue").is_none());
    let view = ready.world().resource::<Table>().view();
    let tony = &view.play.as_ref().unwrap().players[0];
    assert_eq!(tony.resources, 5);
    let hard_knocks = tony.hand.iter().find(|card| card.code == "01049").unwrap();
    let choice = view
        .choices
        .iter()
        .find(|choice| choice.id == format!("action:ordinary:play:{}", hard_knocks.id))
        .unwrap();
    click_source(&mut ready, controls::Source::Card(hard_knocks.id));
    for _ in 0..3 {
        ready.update();
    }
    assert_visible_button(&mut ready, ui::choice_label(choice));
    assert_eq!(ready.world().resource::<Table>().view(), view);
}

fn expanded_map_transition() -> Session {
    let mut session = tony_turn();
    for _ in 0..200 {
        let view = session.observe_for_play();
        if view.play.as_ref().unwrap().scenario.locations.len() > 1 {
            assert_eq!(
                view.revision, 61,
                "Keep the native review's revealing decision"
            );
            return session;
        }
        let id = [
            "act:advance",
            "turn:tony",
            "turn:patrice",
            "turn:jacqueline",
            "action:ordinary:investigate",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
            "success:pass",
            "turn:end",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .or_else(|| {
            view.choices
                .iter()
                .find(|choice| choice.id.starts_with("act:contribute:"))
                .map(|choice| choice.id.as_str())
        })
        .or_else(|| view.choices.first().map(|choice| choice.id.as_str()))
        .expect("A legal decision must remain before the map expands");
        session.choose(view.revision, id).unwrap();
    }
    panic!("Expanded-map preparation reached its 200-decision bound");
}

#[test]
fn expanded_map_review_requires_a_clear_screen_after_the_story_transition() {
    let mut session = expanded_map_transition();
    let (_directory, mut obscured) = open_review_save(&session);
    assert_eq!(
        obscured
            .world()
            .resource::<Table>()
            .view()
            .play
            .unwrap()
            .scenario
            .locations
            .len(),
        4
    );
    assert!(obscured.world().resource::<Table>().modal());
    assert_visible_button(&mut obscured, "Continue");
    assert!(button_with_label(&mut obscured, "End turn").is_none());

    // The follow-up input is one legal decision later, not a forced dismissal or
    // a privileged fixture. Startup must now expose the map without a story panel.
    session
        .choose(session.observe_for_play().revision, "turn:end")
        .unwrap();
    let (_directory, mut ready) = open_review_save(&session);
    let table = ready.world().resource::<Table>();
    assert_eq!(table.view().revision, 62);
    assert!(!table.modal());
    assert!(!table.is_presenting());
    assert!(button_with_label(&mut ready, "Continue").is_none());
    let world = ready.world_mut();
    let locations: Vec<_> = world
        .query::<(&ContextTarget, &InheritedVisibility)>()
        .iter(world)
        .filter_map(|(target, visible)| match &target.source {
            controls::Source::Location(code) if visible.get() => Some(code.as_str()),
            _ => None,
        })
        .collect();
    for code in ["01112", "01113", "01114", "01115"] {
        assert!(
            locations.contains(&code),
            "Missing visible map location {code}"
        );
    }
}

#[test]
fn native_startup_rejects_privileged_review_saves() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("privileged.json");
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringFromSetup);
    continuity::save_session(&path, &session).unwrap();
    assert!(
        matches!(Table::open(path), Err(message) if message.contains("privileged verification fixture"))
    );
}
