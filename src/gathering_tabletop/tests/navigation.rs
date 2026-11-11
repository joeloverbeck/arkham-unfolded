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

fn press(app: &mut App, code: KeyCode, key: Key) {
    keyboard(app, code, key.clone(), true);
    keyboard(app, code, key, false);
}

fn shift_tab(app: &mut App) {
    keyboard(app, KeyCode::ShiftLeft, Key::Shift, true);
    press(app, KeyCode::Tab, Key::Tab);
    keyboard(app, KeyCode::ShiftLeft, Key::Shift, false);
}

/// Labels in Tab order from the first focusable control until focus returns there.
fn tab_cycle(app: &mut App) -> Vec<String> {
    let mut cycle = Vec::new();
    for _ in 0..80 {
        press(app, KeyCode::Tab, Key::Tab);
        let label = focused_label(app).expect("Tab focuses a labelled control");
        if cycle.first() == Some(&label) {
            return cycle;
        }
        cycle.push(label);
    }
    panic!("Tab never returned to its start: {cycle:?}");
}

/// Labels of the focusable buttons inside the non-modal tab group `order`, in tree order.
fn group_labels(app: &mut App, order: i32) -> Vec<String> {
    fn walk(world: &World, entity: Entity, labels: &mut Vec<String>) {
        if world
            .get::<bevy::input_focus::tab_navigation::TabIndex>(entity)
            .is_some()
            && let Some(label) = world.get::<AccessibleLabel>(entity)
        {
            labels.push(label.0.clone());
        }
        for child in world.get::<Children>(entity).into_iter().flatten() {
            walk(world, *child, labels);
        }
    }
    let world = app.world_mut();
    let groups: Vec<Entity> = world
        .query::<(Entity, &TabGroup)>()
        .iter(world)
        .filter(|(_, group)| !group.modal && group.order == order)
        .map(|(entity, _)| entity)
        .collect();
    let mut labels = Vec::new();
    for group in groups {
        walk(world, group, &mut labels);
    }
    labels
}

#[test]
fn the_menu_button_follows_the_existing_tab_groups_and_shift_tab_reaches_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("menu-tab.json");
    let mut app = host(path.clone());
    activate_choice(&mut app, "lead:tony");
    finish_presentation(&mut app);
    let saved = std::fs::read(&path).unwrap();
    let rail = group_labels(&mut app, 0);
    let decisions = group_labels(&mut app, 1);
    assert!(
        decisions.contains(&"Keep this opening hand".to_owned()),
        "The decision panel offers its controls: {decisions:?}"
    );
    assert_eq!(group_labels(&mut app, 3), ["Menu"]);

    app.world_mut().resource_mut::<InputFocus>().clear();
    let cycle = tab_cycle(&mut app);
    let first = cycle
        .iter()
        .position(|label| label == &rail[0])
        .expect("The portrait rail is in the Tab order");
    let cycle = [&cycle[first..], &cycle[..first]].concat();
    for (index, label) in [
        "Focus Tony Morgan",
        "Focus Patrice Hathaway",
        "Focus Jacqueline Fine",
        "Scenario",
        "Test",
        "Overview",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(cycle[index].starts_with(label), "{cycle:?}");
    }
    assert_eq!(
        cycle,
        [rail.clone(), decisions, vec!["Menu".to_owned()]].concat(),
        "Navigation, then decisions, then the Menu"
    );

    // Shift+Tab wraps from the first portrait to the Menu and back to its predecessor.
    while focused_label(&app).as_ref() != Some(&rail[0]) {
        press(&mut app, KeyCode::Tab, Key::Tab);
    }
    shift_tab(&mut app);
    assert_eq!(focused_label(&app).as_deref(), Some("Menu"));
    shift_tab(&mut app);
    assert_eq!(focused_label(&app), cycle.get(cycle.len() - 2).cloned());
    press(&mut app, KeyCode::Tab, Key::Tab);
    press(&mut app, KeyCode::Enter, Key::Enter);
    assert!(app.world().resource::<Table>().menu_open);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        saved,
        "Reaching the Menu decides nothing"
    );
}

#[test]
fn every_modal_dialog_offers_focus_when_opened() {
    fn activate(app: &mut App, label: &str) {
        let entity = button_with_label(app, label).unwrap_or_else(|| panic!("{label}"));
        app.world_mut().trigger(Activate { entity });
        app.update();
    }
    let directory = tempfile::tempdir().unwrap();
    for name in [
        "inspection",
        "history",
        "credits",
        "restart confirmation",
        "encounter reading",
        "group clue payment",
    ] {
        let path = directory.path().join(format!("{name}.json"));
        let mut app = if name == "encounter reading" {
            let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Harm(
                crate::gathering::HarmFixture::WardCancellation,
            ));
            host_with_table(Table::new(session, path), false)
        } else if name == "group clue payment" {
            let session =
                Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
            host_with_table(Table::new(session, path), false)
        } else {
            host(path)
        };
        match name {
            "inspection" => {
                click_source(&mut app, controls::Source::Investigator(Investigator::Tony));
                activate(&mut app, "Inspect");
            }
            "history" | "credits" | "restart confirmation" => {
                activate(&mut app, "Menu");
                activate(
                    &mut app,
                    match name {
                        "history" => "Action history and rules reasons",
                        "credits" => "Credits",
                        _ => "Restart The Gathering...",
                    },
                );
            }
            "group clue payment" => {
                let entity =
                    choice_button(&mut app, "Spend 9 clues as a group: advance The Barrier…")
                        .expect("The decision bar opens the payment at round end");
                app.world_mut().trigger(Activate { entity });
                app.update();
            }
            _ => {}
        }
        let dialogs = modal_dialogs_and_focus(&mut app);
        assert!(!dialogs.is_empty(), "{name} opens a modal dialog");
        assert!(
            dialogs.iter().all(|(_, focusable)| *focusable),
            "{name}: {dialogs:?}"
        );
    }
}

#[test]
fn sideboard_headings_are_reached_by_tab_between_decisions_and_the_menu() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::QuickThinking,
    ));
    let mut play = session.observe().play.unwrap();
    for enemy in &mut play.scenario.enemies {
        enemy.engaged = None;
    }
    play.scenario.locations[0].fog = Some(crate::gathering::CardView {
        id: 63000,
        code: "01168".into(),
        title: "Obscuring Fog".into(),
        weakness: false,
    });
    let mut app = super::cards::at_public_moment(session, play, PathBuf::new(), false);
    // World labels are shown only once the camera can project them.
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.update();
    let headings = group_labels(&mut app, 2);
    assert!(
        !headings.is_empty() && headings.iter().all(|label| label.contains(" · ")),
        "The sideboards offer their location headings: {headings:?}"
    );
    // Enemy headings come first, then the attachment sideboard's.
    let attached = headings
        .iter()
        .position(|label| label.starts_with("Attached to "))
        .expect("the attachment sideboard offers its heading");
    assert!(
        attached > 0
            && headings[attached..]
                .iter()
                .all(|label| label.starts_with("Attached to ")),
        "{headings:?}"
    );
    app.world_mut().resource_mut::<InputFocus>().clear();
    let cycle = tab_cycle(&mut app);
    let start = cycle
        .iter()
        .position(|label| label == &headings[0])
        .unwrap_or_else(|| panic!("Tab reaches the sideboard headings: {cycle:?}"));
    let after = (start + headings.len()) % cycle.len();
    let reached: Vec<_> = (start..start + headings.len())
        .map(|index| cycle[index % cycle.len()].clone())
        .collect();
    assert_eq!(reached, headings);
    assert_eq!(cycle[after], "Menu", "Headings precede the Menu: {cycle:?}");
}
