use super::*;

/// The session lists one prediction choice per ready Pendulum; the table opens
/// its local picker from that entry without committing anything.
fn open_picker(app: &mut App, asset: u16) {
    activate_choice(
        app,
        &format!(
            "test-card:predict:{asset}:{}",
            crate::gathering::PREDICTION_MARGIN
        ),
    );
}

fn press(app: &mut App, label: &str) {
    let entity = choice_button(app, label).unwrap_or_else(|| panic!("Missing {label}"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

#[test]
fn picker_dismissal_is_free_and_other_margin_commits_once() {
    use crate::gathering::{HostileFixture, TestCardFixture};
    use bevy::input::keyboard::Key;
    let session = Session::with_hostile_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalPrediction,
    ));
    let asset = session.observe().play.unwrap().players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60411")
        .unwrap()
        .card
        .id;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("picker.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session.clone(), path.clone()), false);
    open_picker(&mut app, asset);
    assert_eq!(
        app.world().resource::<Table>().session,
        session,
        "Opening is local"
    );
    for number in 0..=10 {
        assert!(choice_button(&mut app, &format!("Predict {number}")).is_some());
    }
    press(&mut app, "Dismiss");
    assert_eq!(app.world().resource::<Table>().session, session);
    assert_eq!(continuity::load_session(&path).unwrap(), session);
    open_picker(&mut app, asset);
    press(&mut app, "Other margin…");
    keyboard(&mut app, KeyCode::Digit1, Key::Character("1".into()), true);
    keyboard(&mut app, KeyCode::Digit2, Key::Character("2".into()), true);
    assert_eq!(
        app.world().resource::<Table>().session,
        session,
        "Typing is local"
    );
    let entity = choice_button(&mut app, "Predict 12").expect("Typed number is confirmable");
    app.world_mut().trigger(Activate { entity });
    app.world_mut().trigger(Activate { entity });
    app.update();
    let saved = continuity::load_session(&path).unwrap();
    assert_eq!(&saved, &app.world().resource::<Table>().session);
    assert_eq!(saved.observe().revision, session.observe().revision + 1);
    assert!(
        saved
            .observe()
            .history
            .last()
            .unwrap()
            .contains("predict 12")
    );
    assert!(
        !saved
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "test-card:pass")
    );
}

#[test]
fn undo_after_the_final_reaction_recovers_the_wager_without_an_automatic_loop() {
    use crate::gathering::{HostileFixture, TestCardFixture};
    let session = Session::with_hostile_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalWithGregory,
    ));
    let asset = session.pendulum_prompts()[0].asset;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reaction-undo.json");
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    open_picker(&mut app, asset);
    press(&mut app, "Predict 3");
    let before = app.world().resource::<Table>().session.observe();
    let wager = before
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("tony:gregory:"))
        .unwrap()
        .id
        .clone();
    activate_choice(&mut app, &wager);
    settle_automatic_resolution(&mut app);
    let committed = app.world().resource::<Table>().session.clone();
    assert_eq!(continuity::load_session(&path).unwrap(), committed);
    drop(app);
    let mut app = host_with_table(
        Table::new(continuity::load_session(&path).unwrap(), path.clone()),
        false,
    );
    activate_choice(&mut app, "undo");
    settle_automatic_resolution(&mut app);
    assert_eq!(
        app.world().resource::<Table>().session.observe().play,
        before.play
    );
    assert!(
        app.world()
            .resource::<Table>()
            .session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == wager)
    );
    assert_eq!(
        continuity::load_session(path).unwrap(),
        app.world().resource::<Table>().session
    );
}

#[test]
fn prediction_choices_form_a_compact_grid_and_stay_inside_the_window() {
    use crate::gathering::{HostileFixture, TestCardFixture};
    let session = Session::with_hostile_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalPrediction,
    ));
    let asset = session.pendulum_prompts()[0].asset;
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("grid.json")),
        false,
    );
    let size = Vec2::new(1280., 800.);
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
    open_picker(&mut app, asset);
    for other in [false, true] {
        if other {
            press(&mut app, "Other margin…");
        }
        for _ in 0..5 {
            app.update();
        }
        let mut centers = Vec::new();
        for number in 0..=10 {
            let button = choice_button(&mut app, &format!("Predict {number}")).unwrap();
            let world = app.world();
            let node = world.get::<ComputedNode>(button).unwrap();
            let at = world.get::<UiGlobalTransform>(button).unwrap();
            let min = at.transform_point2(-node.size() / 2.);
            let max = at.transform_point2(node.size() / 2.);
            assert!(
                min.cmpge(Vec2::ZERO).all() && max.cmple(size).all(),
                "Predict {number}: {min:?}..{max:?}"
            );
            centers.push(at.transform_point2(Vec2::ZERO));
        }
        assert!(
            centers.iter().any(|at| (at.x - centers[0].x).abs() > 20.),
            "Choices must occupy multiple columns"
        );
        let dismiss = choice_button(&mut app, "Dismiss").unwrap();
        let world = app.world();
        let node = world.get::<ComputedNode>(dismiss).unwrap();
        let at = world.get::<UiGlobalTransform>(dismiss).unwrap();
        assert!(at.transform_point2(node.size() / 2.).cmple(size).all());
    }
}

#[test]
fn enter_on_a_focused_prediction_button_does_not_confirm_the_other_field() {
    use crate::gathering::{HostileFixture, TestCardFixture};
    use bevy::input::keyboard::Key;
    let session = Session::with_hostile_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalPrediction,
    ));
    let asset = session.pendulum_prompts()[0].asset;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("keyboard.json");
    let mut app = host_with_table(Table::new(session.clone(), path.clone()), false);
    open_picker(&mut app, asset);
    press(&mut app, "Other margin…");
    keyboard(&mut app, KeyCode::Digit1, Key::Character("1".into()), true);
    keyboard(&mut app, KeyCode::Digit2, Key::Character("2".into()), true);
    let button = choice_button(&mut app, "Predict 3").unwrap();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(button, bevy::input_focus::FocusCause::Navigated);
    keyboard(&mut app, KeyCode::Enter, Key::Enter, true);
    keyboard(&mut app, KeyCode::Enter, Key::Enter, false);
    let saved = continuity::load_session(path).unwrap();
    assert_eq!(saved.observe().revision, session.observe().revision + 1);
    assert!(
        saved
            .observe()
            .history
            .last()
            .unwrap()
            .contains("predict 3")
    );
}
