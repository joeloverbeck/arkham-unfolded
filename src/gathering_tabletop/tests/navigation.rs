use super::*;
use bevy::input::keyboard::Key;
use std::time::Duration;

fn focused_label(app: &App) -> Option<String> {
    let world = app.world();
    let entity = world.resource::<InputFocus>().get()?;
    if let Some(label) = world.get::<AccessibleLabel>(entity) {
        return Some(label.0.clone());
    }
    world
        .get::<Children>(entity)?
        .iter()
        .find_map(|child| world.get::<Text>(child).map(|text| text.0.clone()))
}

#[test]
fn held_wasd_pans_continuously_from_load_without_a_background_click() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("navigation.json");
    let mut app = host(path.clone());
    let view = app.world().resource::<Table>().view();
    let initial = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(100));
    keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), true);
    let mut previous = initial;
    for frame in 0..5 {
        let current = app.world().resource::<Orbit>().pose();
        assert!(
            current.target.z < previous.target.z,
            "held W must keep panning at frame {frame}: {previous:?} -> {current:?}"
        );
        assert_eq!(current.yaw, initial.yaw);
        assert_eq!(current.pitch, initial.pitch);
        assert_eq!(current.distance, initial.distance);
        previous = current;
        if frame < 4 {
            app.update();
        }
    }
    keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), false);
    app.update();
    assert_eq!(app.world().resource::<Orbit>().pose(), previous);
    assert_eq!(app.world().resource::<Table>().view(), view);
    assert!(!path.exists(), "camera input cannot save a game decision");
}

#[test]
fn panning_after_playing_azure_flame_preserves_keyboard_navigation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("azure-flame.json");
    let mut app = host(path.clone());
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session =
            Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringFromSetup);
        table.dirty = true;
    }
    app.update();
    activate_choice(&mut app, "lead:patrice");
    for _ in 0..3 {
        activate_choice(&mut app, "confirm-mulligan");
    }
    activate_choice(&mut app, "turn:patrice");
    finish_presentation(&mut app);
    let flame = app.world().resource::<Table>().view().play.unwrap().players[1]
        .hand
        .iter()
        .find(|card| card.code == "60407")
        .unwrap()
        .id;
    activate_choice(&mut app, &format!("action:ordinary:play:{flame}"));
    let saved = std::fs::read(&path).unwrap();
    let live = continuity::load_session(&path).unwrap().observe();
    let patrice = &live.play.as_ref().unwrap().players[1];
    assert_eq!(patrice.resources, 2);
    assert!(
        patrice
            .assets
            .iter()
            .any(|asset| asset.card.id == flame && asset.uses == 4)
    );
    assert!(app.world().resource::<Table>().is_presenting());

    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(100));
    let before = app.world().resource::<Orbit>().pose();
    keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), true);
    assert!(app.world().resource::<Orbit>().pose().target.z < before.target.z);
    assert!(button_with_label(&mut app, "Resume presentation").is_none());

    let initial_focus = focused_label(&app);
    keyboard(&mut app, KeyCode::Tab, Key::Tab, true);
    let selected = focused_label(&app);
    assert_ne!(
        selected, initial_focus,
        "Tab must move focus while W remains held"
    );
    keyboard(&mut app, KeyCode::Tab, Key::Tab, false);
    for _ in 0..5 {
        let before = app.world().resource::<Orbit>().pose();
        let view = app.world().resource::<Table>().view();
        let selected = focused_label(&app);
        app.update();
        assert!(app.world().resource::<Orbit>().pose().target.z < before.target.z);
        if app.world().resource::<Table>().view() == view {
            assert_eq!(
                focused_label(&app),
                selected,
                "panning must not steal keyboard focus"
            );
        }
    }
    for _ in 0..10 {
        app.update();
    }
    keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), false);
    assert!(
        !app.world().resource::<Table>().is_presenting(),
        "ordinary card placement must finish automatically even after camera navigation"
    );
    assert_eq!(app.world().resource::<Table>().view(), live);
    for label in ["Next event", "Pause presentation", "Show pending choice"] {
        assert!(button_with_label(&mut app, label).is_none());
    }
    assert_eq!(app.world().resource::<Table>().session.observe(), live);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
}

#[test]
fn inspect_history_and_menu_block_wasd_until_closed_without_a_table_click() {
    for dialog in ["Inspect", "History", "Menu"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("modal.json");
        let mut app = host(path.clone());
        let initial = app.world().resource::<Table>().view();
        let label = if dialog == "Inspect" {
            click_source(&mut app, controls::Source::Investigator(Investigator::Tony));
            "Inspect"
        } else {
            let menu = button_with_label(&mut app, "Menu").unwrap();
            app.world_mut().trigger(Activate { entity: menu });
            app.update();
            "Action history and rules reasons"
        };
        if dialog != "Menu" {
            let entity = button_with_label(&mut app, label).unwrap();
            app.world_mut().trigger(Activate { entity });
            app.update();
        }
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(Duration::from_millis(100));
        let pose = app.world().resource::<Orbit>().pose();
        keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), true);
        app.update();
        assert_eq!(app.world().resource::<Orbit>().pose(), pose, "{dialog}");
        keyboard(&mut app, KeyCode::Escape, Key::Escape, true);
        assert!(
            app.world().resource::<Orbit>().pose().target.z < pose.target.z,
            "{dialog}"
        );
        keyboard(&mut app, KeyCode::Escape, Key::Escape, false);
        keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), false);
        assert_eq!(app.world().resource::<Table>().view(), initial);
        assert!(!path.exists());
    }
}

#[test]
fn wasd_dismisses_a_card_action_list_without_choosing_and_ordinary_tab_focus_does_not_capture_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("context.json");
    let mut app = host(path.clone());
    let view = app.world().resource::<Table>().view();
    click_source(&mut app, controls::Source::Investigator(Investigator::Tony));
    assert!(button_with_label(&mut app, "Choose Tony Morgan as lead investigator").is_some());
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(100));
    let pose = app.world().resource::<Orbit>().pose();
    keyboard(&mut app, KeyCode::KeyD, Key::Character("d".into()), true);
    assert!(app.world().resource::<Orbit>().pose().target.x > pose.target.x);
    assert!(button_with_label(&mut app, "Choose Tony Morgan as lead investigator").is_none());
    keyboard(&mut app, KeyCode::KeyD, Key::Character("d".into()), false);
    keyboard(&mut app, KeyCode::Tab, Key::Tab, true);
    keyboard(&mut app, KeyCode::Tab, Key::Tab, false);
    let selected = focused_label(&app).expect("Tab focuses an ordinary control");
    let pose = app.world().resource::<Orbit>().pose();
    keyboard(&mut app, KeyCode::KeyA, Key::Character("a".into()), true);
    for _ in 0..3 {
        app.update();
        assert_eq!(focused_label(&app).as_deref(), Some(selected.as_str()));
    }
    assert!(app.world().resource::<Orbit>().pose().target.x < pose.target.x);
    keyboard(&mut app, KeyCode::KeyA, Key::Character("a".into()), false);
    assert_eq!(app.world().resource::<Table>().view(), view);
    assert!(!path.exists());
}
