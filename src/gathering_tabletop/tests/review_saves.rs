use super::*;

#[path = "../../../tests/support/gathering_draw_review.rs"]
mod draw_review;

#[path = "../../../tests/support/gathering_resolution_review.rs"]
mod resolution_review;

#[test]
fn patrice_upkeep_orders_one_physical_hand_in_her_drawer_and_reopens_there() {
    use super::draws::{physical, tick};
    let point = resolution_review::checkpoints()
        .unwrap()
        .into_iter()
        .find(|point| point.name == "patrice-upkeep")
        .unwrap();
    let cards: Vec<_> = point.session.observe().investigators[1]
        .hand
        .iter()
        .filter(|card| !card.weakness)
        .map(|card| card.id)
        .collect();
    assert_eq!(cards.len(), 5);
    let mut expected = point.session.clone();
    let (_directory, mut app) = open_review_save(&point.session);
    for id in &cards {
        let at = physical(&mut app, *id).translation;
        assert!(
            at.x.abs() < 7. && at.z > 30.,
            "Patrice orders each non-weakness card in her own drawer: {at:?}"
        );
    }
    activate_choice(&mut app, &point.trigger);
    expected
        .choose(expected.observe().revision, &point.trigger)
        .unwrap();
    tick(&mut app, 0.5);
    let committed = app.world().resource::<Table>().session.clone();
    assert_eq!(committed, expected);
    assert_eq!(committed.observe().investigators[1].hand.len(), 5);
    let (_reopened_directory, mut app) = open_review_save(&committed);
    for id in &cards {
        assert!(physical(&mut app, *id).translation.z > 30.);
    }
    for _ in 1..cards.len() {
        let view = expected.observe();
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with("upkeep:patrice-discard:"))
            .unwrap()
            .id
            .clone();
        expected.choose(view.revision, &choice).unwrap();
        activate_choice(&mut app, &choice);
        tick(&mut app, 0.5);
    }
    expected
        .choose(expected.observe().revision, "upkeep:patrice-refill")
        .unwrap();
    activate_choice(&mut app, "upkeep:patrice-refill");
    for id in cards {
        assert!(
            physical(&mut app, id).translation.z > 30.,
            "Every ordered card starts its discard travel from the drawer"
        );
    }
    finish_presentation(&mut app);
    assert_eq!(app.world().resource::<Table>().session, expected);
    assert!(expected.observe().play.unwrap().pending.discard.is_none());
}

#[test]
fn resolution_review_saves_open_as_ordinary_sessions_with_the_expected_decisions() {
    for point in resolution_review::checkpoints().unwrap() {
        let (_directory, mut app) = open_review_save(&point.session);
        let view = app.world().resource::<Table>().view();
        assert!(
            view.choices.iter().any(|choice| choice.id == point.trigger),
            "{}",
            point.name
        );
        assert!(parse_initial_view(point.initial_view).is_ok());
        if point.name == "revelation-pending" {
            let play = view.play.unwrap();
            let id = play.pending_revelation.unwrap().card.id;
            assert_eq!(play.test.unwrap().source_card.unwrap().id, id);
            let world = app.world_mut();
            let at = world
                .query::<(&motion::PhysicalCard, &Transform)>()
                .iter(world)
                .filter(|(card, _)| card.0 == id)
                .map(|(_, at)| at.translation)
                .collect::<Vec<_>>();
            assert_eq!(at.len(), 1, "The revelation has one physical test source");
            assert!(at[0].abs_diff_eq(layout::test_source(), 0.001));
        }
    }
}

#[test]
fn draw_review_checkpoints_open_on_visible_triggers_and_preserve_manual_framing() {
    let checkpoints = draw_review::checkpoints().unwrap();
    assert_eq!(
        checkpoints
            .iter()
            .map(|point| point.name)
            .collect::<Vec<_>>(),
        ["opening", "mulligan", "action", "upkeep", "encounter"]
    );
    for point in checkpoints {
        let (_directory, mut app) = open_review_save(&point.session);
        let area = parse_initial_view(point.initial_view).unwrap();
        app.world_mut()
            .insert_resource(Orbit::new(CameraMode::Jacqueline).with_initial_view(Some(area)));
        app.update();
        let pose = app.world().resource::<Orbit>().pose();
        let table = app.world().resource::<Table>();
        assert!(
            !table.modal(),
            "{} opens without a blocking dialog",
            point.name
        );
        assert!(
            !table.is_presenting(),
            "{} opens before its draw",
            point.name
        );
        let view = table.view();
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id == point.trigger)
            .unwrap();
        if let Some(source) = choice_source(&mut app, &point.trigger) {
            click_source(&mut app, source);
        }
        for _ in 0..3 {
            app.update();
        }
        assert_visible_button(&mut app, ui::choice_label(choice));
        activate_choice(&mut app, &point.trigger);
        assert!(
            app.world().resource::<Table>().is_presenting(),
            "{} starts its draw",
            point.name
        );
        assert_eq!(app.world().resource::<Orbit>().pose(), pose);
        finish_presentation(&mut app);
        assert_eq!(app.world().resource::<Orbit>().pose(), pose);
        for size in [Vec2::new(1600., 1000.), Vec2::new(3200., 2000.)] {
            crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
            for _ in 0..3 {
                app.update();
            }
            let anchors = if point.name == "encounter" {
                vec![layout::ENCOUNTER_DECK, layout::test_source()]
            } else {
                let view = app.world().resource::<Table>().view();
                let count = view.investigators[0].hand.len();
                let mut anchors = vec![layout::investigator_deck(0)];
                anchors.extend((0..count).map(|slot| layout::hand_position(0, count, slot)));
                anchors
            };
            assert_draw_anchors_unobscured(&mut app, &anchors, size, point.name);
        }
        let committed = app.world().resource::<Table>().session.clone();
        let (_reopened_directory, reopened) = open_review_save(&committed);
        assert!(
            !reopened.world().resource::<Table>().is_presenting(),
            "Reopening shows the saved endpoint"
        );
    }
}

pub(super) fn assert_draw_anchors_unobscured(
    app: &mut App,
    anchors: &[Vec3],
    size: Vec2,
    name: &str,
) {
    let world = app.world_mut();
    let panels = world
        .query::<(&ScrollablePanel, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
        .filter(|(panel, _, _)| !panel.inspection)
        .map(|(_, node, transform)| {
            Rect::from_corners(
                transform.transform_point2(-node.size() / 2.),
                transform.transform_point2(node.size() / 2.),
            )
        })
        .collect::<Vec<_>>();
    let (camera, transform) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
        .single(world)
        .unwrap();
    for anchor in anchors {
        let point = camera.world_to_viewport(transform, *anchor).unwrap();
        assert!(
            point.cmpge(Vec2::splat(24.)).all() && point.cmple(size - Vec2::splat(24.)).all(),
            "{name} draw anchor must remain in frame at {size:?}: {point:?}"
        );
        assert!(
            !panels.iter().any(|panel| panel.contains(point)),
            "{name} draw anchor is behind the decision panel at {size:?}: {point:?}"
        );
    }
}

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
        app.world().resource::<Table>().session.clone(),
        session.clone()
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
    let result = table
        .presentation
        .result()
        .expect("Saved result needs reading");
    assert_eq!((result.total, result.difficulty), (7, 2));
    assert_eq!(
        result.breakdown.base
            + result
                .breakdown
                .cards
                .iter()
                .map(|row| row.amount)
                .sum::<i32>()
            + result
                .breakdown
                .bonuses
                .iter()
                .map(|row| row.amount)
                .sum::<i32>()
            + result.breakdown.token_total,
        result.total
    );
    assert_visible_button(&mut after, "Continue");
    let button = button_with_label(&mut after, "Continue").unwrap();
    after.world_mut().trigger(Activate { entity: button });
    after.update();
    assert_visible_button(&mut after, "Take Tony's extra action (Quick Thinking)");
    assert_visible_button(&mut after, "Decline Quick Thinking");
    assert!(button_with_label(&mut after, "Continue").is_none());
    assert_eq!(
        after.world().resource::<Table>().session.clone(),
        session.clone()
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
        session.choose(session.observe().revision, id).unwrap();
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
        let view = session.observe();
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

    let path = obscured.world().resource::<Table>().save_path.clone();
    let saved = std::fs::read(&path).unwrap();
    let continue_button = button_with_label(&mut obscured, "Continue").unwrap();
    obscured.world_mut().trigger(Activate {
        entity: continue_button,
    });
    obscured.update();
    assert!(button_with_label(&mut obscured, "Continue").is_none());
    obscured
        .world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Reopen);
    obscured.update();
    assert!(
        button_with_label(&mut obscured, "Continue").is_none(),
        "In-place reopen of the same Session must retain the story acknowledgment"
    );
    assert_eq!(std::fs::read(path).unwrap(), saved);
    assert_eq!(obscured.world().resource::<Table>().session, session);

    // The follow-up input is one legal decision later, not a forced dismissal or
    // a privileged fixture. Startup must now expose the map without a story panel.
    session
        .choose(session.observe().revision, "turn:end")
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
