//! Seam: table talk at the table host that runs without a graphics card (#209). A
//! partner speaks and chooses over the real socket; the human uses the production
//! controls, keys and pointer. Expectations come from ticket #209 and its amendment
//! (action lines), and the presentation contract's "Agent partners and table talk";
//! the decision display, the turn indicator and the rail are found by what they
//! show, not by the markers speech boxes read. Prior art: the phase-banner and
//! partner tests on this host.

use super::super::partners::{Input, PartnerSocket, PartnerWaker};
use super::super::tests::{button_with_label, finish_presentation, host_with_table, rendered_text};
use super::*;
use crate::gathering::Investigator::{Jacqueline, Patrice, Tony};
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    picking::{hover::HoverMap, pointer::PointerId},
    ui::ResolvedBorderRadius,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

struct Talking {
    app: App,
    socket: PathBuf,
    save: PathBuf,
    _directory: tempfile::TempDir,
}

/// A table on `seed` after `before` (the human's choices, partners still off),
/// with `partners` handed to partners and partners enabled.
fn talking(seed: u64, before: &[&str], partners: &[Investigator], pointer: bool) -> Talking {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("talk.session.json");
    let socket = directory.path().join("partners.sock");
    let mut app = host_with_table(Table::new(Session::with_seed(seed), save.clone()), pointer);
    // A camera with a real viewport: the pointer finds UI under it, and the Test
    // area projects onto the screen.
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.insert_resource(PartnerSocket(socket.clone()))
        .insert_resource(PartnerWaker(Arc::new(|| {})));
    for id in before {
        choose(&mut app, id);
        finish_presentation(&mut app);
    }
    let inputs = &mut app.world_mut().resource_mut::<Table>().partners.inputs;
    inputs.extend(partners.iter().map(|who| Input::Hand(*who)));
    inputs.push(Input::Enable(true));
    app.update();
    assert!(app.world().resource::<Table>().partners.enabled);
    Talking {
        app,
        socket,
        save,
        _directory: directory,
    }
}

fn choose(app: &mut App, id: &str) {
    let table = app.world().resource::<Table>();
    let action = Action::Choose(table.epoch, table.view().revision, id.into());
    app.world_mut().resource_mut::<Actions>().0.push(action);
    app.update();
}

fn press(app: &mut App, label: &str) {
    let entity = button_with_label(app, label)
        .unwrap_or_else(|| panic!("No button {label}:\n{}", rendered_text(app)));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

/// One partner connection: send `requests`, run frames until every answer is in.
/// The presentation clock does not move, so speech boxes keep their age.
fn exchange(app: &mut App, socket: &Path, requests: &[Value]) -> Vec<Value> {
    let socket = socket.to_owned();
    let lines: Vec<String> = requests.iter().map(Value::to_string).collect();
    let client = thread::spawn(move || {
        let stream = UnixStream::connect(&socket).expect("The partner socket answers");
        let mut writer = &stream;
        for line in &lines {
            writeln!(writer, "{line}").unwrap();
        }
        stream.shutdown(Shutdown::Write).unwrap();
        BufReader::new(&stream)
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect::<Vec<Value>>()
    });
    for _ in 0..1_000 {
        if client.is_finished() {
            break;
        }
        app.update();
        thread::sleep(Duration::from_millis(2));
    }
    client.join().unwrap()
}

fn join_as(talking: &mut Talking, investigators: &[&str]) -> String {
    let joined = exchange(
        &mut talking.app,
        &talking.socket,
        &[json!({"op": "join", "investigators": investigators})],
    );
    assert_eq!(joined[0]["status"], "joined", "{}", joined[0]);
    joined[0]["partner"].as_str().unwrap().to_owned()
}

fn say(talking: &mut Talking, partner: &str, text: &str) {
    let said = exchange(
        &mut talking.app,
        &talking.socket,
        &[json!({"op": "say", "partner": partner, "text": text})],
    );
    assert_eq!(said[0]["status"], "said", "{}", said[0]);
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// Advance the presentation clock by `seconds`, a tenth at a time.
fn wait(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(100));
    for _ in 0..(seconds * 10.).round() as usize {
        app.update();
    }
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::ZERO);
}

fn screen_rect(app: &App, entity: Entity) -> Rect {
    let world = app.world();
    rect(
        world.get::<ComputedNode>(entity).unwrap(),
        world.get::<UiGlobalTransform>(entity).unwrap(),
    )
}

/// The visible slot beside `who`'s portrait.
fn slot(app: &mut App, who: Investigator) -> Option<Entity> {
    let world = app.world_mut();
    world
        .query::<(Entity, &SpeechSlot, &InheritedVisibility)>()
        .iter(world)
        .find(|(_, slot, visible)| slot.0 == who && visible.get())
        .map(|(entity, ..)| entity)
}

/// The boxes shown beside `who`'s portrait: (kind, text, entity).
fn shown(app: &mut App, who: Investigator) -> Vec<(Kind, String, Entity)> {
    let Some(slot) = slot(app, who) else {
        return Vec::new();
    };
    let world = app.world();
    world
        .get::<Children>(slot)
        .into_iter()
        .flatten()
        .map(|child| {
            let kind = *world.get::<Kind>(*child).unwrap();
            let text = world
                .get::<Children>(*child)
                .unwrap()
                .iter()
                .find_map(|part| world.get::<Text>(part).map(|text| text.0.clone()))
                .unwrap();
            (kind, text, *child)
        })
        .collect()
}

fn portrait(app: &mut App, who: Investigator) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<(Entity, &AccessibleLabel), With<Button>>()
        .iter(world)
        .find(|(_, label)| label.0.starts_with(&format!("Focus {}", who.name())))
        .map(|(entity, _)| entity)
        .expect("The rail's portrait")
}

/// The panel holding the text `shown`: the decision display holds "LIVE
/// DECISION", the turn indicator "The Gathering · Standard".
fn panel_showing(app: &mut App, shown: &str) -> Rect {
    let world = app.world_mut();
    let text = world
        .query::<(Entity, &Text, Option<&AccessibleLabel>)>()
        .iter(world)
        .find(|(_, text, label)| {
            text.0.starts_with(shown) || label.is_some_and(|label| label.0.starts_with(shown))
        })
        .map(|(entity, ..)| entity)
        .unwrap_or_else(|| panic!("Nothing shows {shown}"));
    let panel = world.get::<ChildOf>(text).unwrap().parent();
    screen_rect(app, panel)
}

fn subtree(app: &App, root: Entity) -> Vec<Entity> {
    let world = app.world();
    let mut all = vec![root];
    let mut index = 0;
    while index < all.len() {
        all.extend(world.get::<Children>(all[index]).into_iter().flatten());
        index += 1;
    }
    all
}

fn key(app: &mut App, code: KeyCode, logical: Key, pressed: bool) {
    let world = app.world_mut();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    let text = match (&logical, pressed) {
        (Key::Character(text), true) => Some(text.clone()),
        _ => None,
    };
    world.write_message(KeyboardInput {
        key_code: code,
        logical_key: logical,
        state: if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
        text,
        repeat: false,
        window,
    });
    app.update();
}

fn tap(app: &mut App, code: KeyCode, logical: Key) {
    key(app, code, logical.clone(), true);
    key(app, code, logical, false);
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        let code = match character.to_ascii_lowercase() {
            'w' => KeyCode::KeyW,
            'a' => KeyCode::KeyA,
            's' => KeyCode::KeyS,
            'd' => KeyCode::KeyD,
            'h' => KeyCode::KeyH,
            'i' => KeyCode::KeyI,
            _ => KeyCode::KeyX,
        };
        tap(app, code, Key::Character(character.to_string().into()));
    }
}

fn move_pointer(app: &mut App, position: Vec2) {
    let world = app.world_mut();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    world.write_message(bevy::window::WindowEvent::CursorMoved(
        bevy::window::CursorMoved {
            window,
            position,
            delta: None,
        },
    ));
    app.update();
    app.update();
}

/// Ticket #209, Speech boxes: a partner's message shows to the right of its
/// investigator's portrait. It takes no focus or pointer, never blocks the camera,
/// stays clear of the decision display and the turn indicator, and a rebuild of
/// the table's panels leaves it in place.
#[test]
fn a_partners_message_shows_beside_its_portrait_and_takes_no_input() {
    // Jacqueline leads; Patrice is the partner's.
    let mut talking = talking(89, &["lead:jacqueline"], &[Patrice], true);
    let partner = join_as(&mut talking, &[]);
    let focus_before = talking.app.world().resource::<InputFocus>().get();
    say(&mut talking, &partner, "I'll take the Attic.");
    let app = &mut talking.app;
    settle(app);
    let boxes = shown(app, Patrice);
    assert_eq!(boxes.len(), 1, "{}", rendered_text(app));
    assert_eq!(boxes[0].0, Kind::Speech);
    assert!(
        boxes[0].1.contains("I'll take the Attic."),
        "{}",
        boxes[0].1
    );
    let label = app.world().get::<AccessibleLabel>(boxes[0].2).unwrap();
    assert!(label.0.contains("Patrice Hathaway"), "{}", label.0);

    let slot = slot(app, Patrice).unwrap();
    let speech = screen_rect(app, slot);
    let portrait = portrait(app, Patrice);
    let beside = screen_rect(app, portrait);
    assert!(
        speech.min.x >= beside.max.x,
        "{speech:?} right of {beside:?}"
    );
    assert!(
        speech.min.y < beside.max.y && speech.max.y > beside.min.y,
        "{speech:?} level with {beside:?}"
    );
    for (panel, shows) in [
        ("decision display", "LIVE DECISION"),
        ("turn indicator", "The Gathering · Standard"),
    ] {
        let covered = panel_showing(app, shows);
        assert!(
            covered.intersect(speech).is_empty(),
            "The speech box {speech:?} covers the {panel} {covered:?}"
        );
    }

    // No focus, no pointer, no keyboard.
    assert_eq!(app.world().resource::<InputFocus>().get(), focus_before);
    for entity in subtree(app, slot) {
        let world = app.world();
        assert!(world.get::<Button>(entity).is_none());
        assert!(
            world
                .get::<bevy::input_focus::tab_navigation::TabIndex>(entity)
                .is_none()
        );
        let pickable = world
            .get::<Pickable>(entity)
            .expect("Every part ignores the pointer");
        assert!(!pickable.should_block_lower && !pickable.is_hoverable);
    }
    move_pointer(app, speech.center());
    let hits: Vec<Entity> = app.world().resource::<HoverMap>()[&PointerId::Mouse]
        .keys()
        .copied()
        .collect();
    let parts = subtree(app, slot);
    assert!(
        hits.iter().all(|hit| !parts.contains(hit)),
        "The pointer passes through the speech box: {hits:?}"
    );
    // WASD still pans the camera while it shows.
    let pose = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(16));
    key(app, KeyCode::KeyW, Key::Character("w".into()), true);
    app.update();
    key(app, KeyCode::KeyW, Key::Character("w".into()), false);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::ZERO);
    assert_ne!(app.world().resource::<Orbit>().pose(), pose);

    // A rebuild of the table's panels keeps it.
    app.world_mut().resource_mut::<Table>().dirty = true;
    app.update();
    app.update();
    assert_eq!(self::slot(app, Patrice), Some(slot));
    assert_eq!(shown(app, Patrice)[0].1, boxes[0].1);
}

/// Ticket #209: each box stays for a reading delay proportional to its length,
/// then fades; reduced motion removes the fade, not the delay. A speaker's
/// messages queue, and hovering its portrait shows its latest message again.
#[test]
fn speech_boxes_read_queue_fade_and_are_recalled_by_hovering() {
    let mut talking = talking(89, &["lead:jacqueline"], &[Tony, Patrice], true);
    let tonys = join_as(&mut talking, &["tony"]);
    let patrices = join_as(&mut talking, &["patrice"]);
    say(&mut talking, &tonys, "Short.");
    say(
        &mut talking,
        &patrices,
        "A much longer message that takes a good while longer to read through.",
    );
    say(&mut talking, &patrices, "Second thought.");
    let app = &mut talking.app;
    settle(app);
    assert_eq!(shown(app, Tony).len(), 1);
    let patrices_boxes = shown(app, Patrice);
    assert_eq!(patrices_boxes.len(), 1, "Her second message queues");
    assert!(patrices_boxes[0].1.contains("much longer"));

    // Proportional to length: Tony's short message goes first, after fading.
    let mut tony_gone = None;
    let mut faded = false;
    for tenth in 1..400 {
        wait(app, 0.1);
        if let Some((_, _, part)) = shown(app, Tony).first() {
            let alpha = app.world().get::<BackgroundColor>(*part).unwrap().0.alpha();
            faded |= alpha > 0. && alpha < 1.;
        } else if tony_gone.is_none() {
            tony_gone = Some(tenth);
            assert!(
                shown(app, Patrice)[0].1.contains("much longer"),
                "The longer message still shows"
            );
        }
        if shown(app, Patrice)
            .first()
            .is_some_and(|(_, text, _)| text.contains("Second thought"))
        {
            break;
        }
    }
    let animated = tony_gone.expect("Tony's message ends");
    assert!(faded, "Without reduced motion a box fades out");

    // Hovering Tony's portrait recalls his latest message; leaving it hides it.
    let tony = portrait(app, Tony);
    let center = screen_rect(app, tony).center();
    move_pointer(app, center);
    settle(app);
    assert!(
        shown(app, Tony)
            .iter()
            .any(|(kind, text, _)| *kind == Kind::Speech && text.contains("Short.")),
        "{}",
        rendered_text(app)
    );
    move_pointer(app, Vec2::new(900., 400.));
    settle(app);
    assert!(shown(app, Tony).is_empty());

    // Reduced motion: no fade, and the same reading delay before it goes.
    app.world_mut().resource_mut::<Table>().reduced_motion = true;
    say(&mut talking, &tonys, "Short.");
    let app = &mut talking.app;
    settle(app);
    let mut reduced = 0;
    while let Some((_, _, part)) = shown(app, Tony).first().cloned() {
        let alpha = app.world().get::<BackgroundColor>(part).unwrap().0.alpha();
        assert_eq!(alpha, 1., "Reduced motion never fades a box");
        wait(app, 0.1);
        reduced += 1;
        assert!(reduced < 200);
    }
    let fade = (FADE * 10.).round() as usize;
    assert!(
        reduced < animated && reduced + fade + 1 >= animated,
        "Reduced motion keeps the delay ({reduced} tenths) and drops only the fade ({animated} tenths)"
    );
}

/// Ticket #209 amendment: a partner's committed decision shows its choice label
/// beside the acting investigator's portrait as a record (square, edged, no tail,
/// no quotation marks), stacked with that speaker's message, clear of the decision
/// display and kept through rebuilds. The human's own decisions get no line.
#[test]
fn a_partners_decision_shows_as_an_action_line_and_the_humans_does_not() {
    let mut talking = talking(89, &["lead:jacqueline"], &[Patrice], false);
    let partner = join_as(&mut talking, &[]);
    let app = &mut talking.app;
    choose(app, "confirm-mulligan"); // Jacqueline, the human's
    finish_presentation(app);
    choose(app, "confirm-mulligan"); // Tony, the human's
    finish_presentation(app);
    settle(app);
    for who in Investigator::ALL {
        assert!(
            shown(app, who).is_empty(),
            "The human's decisions get no line"
        );
    }
    let view = app.world().resource::<Table>().session.observe();
    let label = view
        .choices
        .iter()
        .find(|choice| choice.id == "confirm-mulligan")
        .unwrap()
        .label
        .clone();
    let chosen = exchange(
        app,
        &talking.socket,
        &[json!({
            "op": "choose", "partner": partner, "revision": view.revision, "id": "confirm-mulligan",
        })],
    );
    assert_eq!(chosen[0]["status"], "chosen", "{}", chosen[0]);
    say(&mut talking, &partner, "Keeping it.");
    let app = &mut talking.app;
    settle(app);
    let boxes = shown(app, Patrice);
    let line = boxes
        .iter()
        .find(|(kind, ..)| *kind == Kind::Line)
        .unwrap_or_else(|| panic!("{boxes:?}"));
    assert_eq!(line.1, label);
    assert!(!line.1.contains(['“', '”', '"']));
    let speech = boxes
        .iter()
        .find(|(kind, ..)| *kind == Kind::Speech)
        .expect("The line stacks with her message");
    let world = app.world();
    let plate = world.get::<ComputedNode>(line.2).unwrap();
    assert_eq!(plate.border_radius, ResolvedBorderRadius::ZERO, "Square");
    assert!(plate.border.min_inset.x > 0., "Edged");
    assert_eq!(
        world.get::<Children>(line.2).unwrap().len(),
        1,
        "No tail: only its text"
    );
    let bubble = world.get::<ComputedNode>(speech.2).unwrap();
    assert_ne!(bubble.border_radius, ResolvedBorderRadius::ZERO, "Rounded");
    assert!(
        world.get::<Children>(speech.2).unwrap().len() > 1,
        "Speech has a tail"
    );
    let beside = slot(app, Patrice).unwrap();
    let column = screen_rect(app, beside);
    let decision = panel_showing(app, "LIVE DECISION");
    assert!(
        decision.intersect(column).is_empty(),
        "{column:?} {decision:?}"
    );
    app.world_mut().resource_mut::<Table>().dirty = true;
    settle(app);
    assert!(
        shown(app, Patrice)
            .iter()
            .any(|(kind, text, _)| *kind == Kind::Line && *text == label)
    );
}

/// Ticket #209, Message box: T opens it; while open it holds the keyboard, so WASD
/// and Tab type or stay in it; Escape closes it without sending. The rail's button
/// opens it too, Enter sends, and the message shows as the human's speech box
/// beside the first of the human's investigators in player order, and in Menu →
/// Table talk with its speaker and round. Talk never touches the save.
#[test]
fn the_message_box_holds_the_keyboard_and_sends_with_enter() {
    let mut talking = talking(89, &["lead:jacqueline"], &[Patrice], false);
    let partner = join_as(&mut talking, &[]);
    say(&mut talking, &partner, "Hello, table.");
    let saved = std::fs::read(&talking.save).unwrap();
    let app = &mut talking.app;
    tap(app, KeyCode::KeyT, Key::Character("t".into()));
    app.update();
    assert!(app.world().resource::<Table>().talk.open);
    let message_box = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<MessageBox>>()
            .single(world)
            .unwrap()
    };
    let pose = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(16));
    key(app, KeyCode::KeyW, Key::Character("w".into()), true);
    app.update();
    app.update();
    key(app, KeyCode::KeyW, Key::Character("w".into()), false);
    type_text(app, "asd");
    tap(app, KeyCode::Tab, Key::Tab);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::ZERO);
    assert_eq!(app.world().resource::<Orbit>().pose(), pose, "WASD typed");
    assert_eq!(app.world().resource::<Table>().talk.draft, "wasd");
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(message_box),
        "Tab stays in the box"
    );
    assert!(rendered_text(app).contains("wasd|"));
    tap(app, KeyCode::Escape, Key::Escape);
    assert!(!app.world().resource::<Table>().talk.open);
    assert_eq!(
        app.world().resource::<Table>().talk.log.len(),
        1,
        "Nothing sent"
    );

    // Labels show "·" as " / ".
    press(app, &crate::tabletop_support::readable_text(SPEAK.into()));
    app.update();
    assert!(app.world().resource::<Table>().talk.open);
    for _ in 0..4 {
        tap(app, KeyCode::Backspace, Key::Backspace);
    }
    type_text(app, "Hi");
    tap(app, KeyCode::Enter, Key::Enter);
    settle(app);
    let table = app.world().resource::<Table>();
    assert!(!table.talk.open, "Sent: the box closes");
    let said = table.talk.log.last().unwrap();
    assert_eq!(said.text, "Hi");
    assert_eq!(said.speaker, [Jacqueline, Tony], "From the lead");
    assert!(
        shown(app, Jacqueline)
            .iter()
            .any(|(kind, text, _)| *kind == Kind::Speech && text.contains("Hi")),
        "{}",
        rendered_text(app)
    );
    assert!(shown(app, Tony).is_empty());
    let heard = exchange(
        app,
        &talking.socket,
        &[json!({"op": "observe", "partner": partner})],
    );
    assert_eq!(heard[0]["view"]["table_talk"][0]["text"], "Hi");

    let app = &mut talking.app;
    press(app, "Menu");
    press(app, "Table talk");
    let listed = rendered_text(app);
    for entry in [
        "Setup · Patrice Hathaway: Hello, table.",
        "Setup · Jacqueline Fine and Tony Morgan: Hi",
    ] {
        let shown = crate::tabletop_support::readable_text(entry.into());
        assert!(listed.contains(&shown), "{shown} in {listed}");
    }
    assert_eq!(std::fs::read(&talking.save).unwrap(), saved, "Never saved");
}

/// S2, Joel's co-play session 2 (#202): the message box's header showed empty boxes
/// where its separators were, and a partner's speech box showed boxes at the start
/// and end of its message. The interface's face draws only what its character map
/// has (`font_checks` reads it), so everything table talk shows, the message box,
/// speech boxes and Menu → Table talk, is drawn in those characters: typographic
/// quotes, apostrophes, dashes and ellipses as their plain forms, anything else
/// the face lacks as "?". What the human types is drawn so as it is typed, and
/// once drawn is left alone; the draft, the talk log and the sent message keep it
/// as typed.
#[test]
fn table_talk_draws_only_characters_its_face_has() {
    let mut talking = talking(89, &["lead:jacqueline"], &[Patrice], false);
    let partner = join_as(&mut talking, &[]);
    say(&mut talking, &partner, "“Don’t go — it’s cold…” naïve ✓");
    let app = &mut talking.app;
    settle(app);
    let boxes = shown(app, Patrice);
    let spoken = &boxes.first().expect("Her speech box").1;
    for part in ["\"\"Don't go", "it's cold...\"", "na?ve ?\""] {
        assert!(spoken.contains(part), "{part} in {spoken}");
    }
    tap(app, KeyCode::KeyT, Key::Character("t".into()));
    type_text(app, "Hi é’");
    settle(app);
    assert!(app.world().resource::<Table>().talk.open);
    assert_eq!(app.world().resource::<Table>().talk.draft, "Hi é’");
    let draft = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<Draft>>()
            .single(world)
            .unwrap()
    };
    assert_eq!(app.world().get::<Text>(draft).unwrap().0, "Hi ?'|");
    crate::tabletop_layout::font_checks::assert_supported(app.world_mut());
    let changed = |app: &App| {
        app.world()
            .entity(draft)
            .get_change_ticks::<Text>()
            .unwrap()
            .changed
    };
    let drawn = changed(app);
    app.update();
    assert_eq!(
        changed(app),
        drawn,
        "Nothing changed, so the draft is not rewritten"
    );

    tap(app, KeyCode::Enter, Key::Enter);
    settle(app);
    let table = app.world().resource::<Table>();
    assert!(!table.talk.open, "Sent: the box closes");
    let sent = &table.talk.log.last().unwrap().text;
    assert_eq!(sent, "Hi é’", "Sent as typed");
    press(app, "Menu");
    press(app, "Table talk");
    let listed = rendered_text(app);
    for entry in [
        "Patrice Hathaway: \"Don't go",
        "Jacqueline Fine and Tony Morgan: Hi ?'",
    ] {
        assert!(listed.contains(entry), "{entry} in {listed}");
    }
    crate::tabletop_layout::font_checks::assert_supported(app.world_mut());
}

/// Ticket #209, Peril silence: while the table presents Patrice drawing Watcher
/// from Another Dimension (seed 224's first draw; "Peril", RR p.16), the message
/// box says why it cannot send, and sends nothing; once the encounter is over it
/// sends.
#[test]
fn the_message_box_is_unavailable_during_a_peril_encounter_and_says_why() {
    let before = [
        "lead:jacqueline",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:patrice",
    ];
    let mut talking = talking(224, &before, &[Tony], false);
    let app = &mut talking.app;
    choose(app, "action:ordinary:draw");
    assert!(app.world().resource::<Table>().is_presenting());
    tap(app, KeyCode::KeyT, Key::Character("t".into()));
    type_text(app, "Hi");
    tap(app, KeyCode::Enter, Key::Enter);
    let table = app.world().resource::<Table>();
    assert!(table.talk.open);
    assert_eq!(table.talk.closed, Some(crate::partners::PERIL));
    assert!(table.talk.log.is_empty(), "Nothing is said");
    assert!(rendered_text(app).contains(crate::partners::PERIL));

    finish_presentation(app);
    // The drawn card reaches Patrice's hand; the table settles.
    wait(app, 2.);
    settle(app);
    assert_eq!(app.world().resource::<Table>().talk.closed, None);
    tap(app, KeyCode::Enter, Key::Enter);
    settle(app);
    assert_eq!(app.world().resource::<Table>().talk.log.len(), 1);
}

/// Ticket #209: a speech box never covers a reading or an open menu. Under the
/// action history's reading it is hidden and its reading time waits; it shows
/// again once the reading closes. Beside an open menu it stays clear of it.
#[test]
fn a_reading_hides_a_speech_box_and_holds_its_time() {
    let mut talking = talking(89, &["lead:jacqueline"], &[Patrice], false);
    let partner = join_as(&mut talking, &[]);
    say(&mut talking, &partner, "Look at this.");
    let app = &mut talking.app;
    settle(app);
    assert_eq!(shown(app, Patrice).len(), 1);
    press(app, "Menu");
    settle(app);
    let menu = {
        let close = button_with_label(app, "Close menu").unwrap();
        let panel = app.world().get::<ChildOf>(close).unwrap().parent();
        screen_rect(app, panel)
    };
    let beside = slot(app, Patrice).expect("Still shown beside the open menu");
    assert!(menu.intersect(screen_rect(app, beside)).is_empty());
    press(app, "Action history and rules reasons");
    settle(app);
    assert!(app.world().resource::<Table>().history_open);
    assert!(shown(app, Patrice).is_empty(), "Hidden under the reading");
    wait(app, 30.);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    settle(app);
    assert_eq!(
        shown(app, Patrice)
            .first()
            .map(|(_, text, _)| text.contains("Look at this.")),
        Some(true),
        "Its reading time waited"
    );
}

/// S3, Joel's co-play session 2 (#202): as the camera neared the Test area during a
/// test, speech boxes narrowed and then hid ("I don't think we want that to happen
/// at any point"). A box avoids only the interface it must never cover, never the
/// scene: wherever the camera is over the table during a test, a long message's box
/// keeps its width and shows, and it stays clear of the decision display.
#[test]
fn a_speech_box_keeps_its_width_and_shows_wherever_the_camera_is() {
    let directory = tempfile::tempdir().unwrap();
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::JacquelineCancellation);
    let mut app = host_with_table(
        Table::new(session, directory.path().join("test-area.json")),
        false,
    );
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.insert_resource(PartnerSocket(directory.path().join("partners.sock")))
        .insert_resource(PartnerWaker(Arc::new(|| {})));
    // The human plays Jacqueline alone: her portrait is the rail's lowest.
    app.world_mut()
        .resource_mut::<Table>()
        .partners
        .inputs
        .extend([Input::Hand(Tony), Input::Hand(Patrice), Input::Enable(true)]);
    app.update();
    let view = app.world().resource::<Table>().view();
    assert!(view.play.as_ref().unwrap().test.is_some(), "A test shows");
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.talk.open();
        table.talk.draft = "We need a better roll here. I am committing what I can, and if it still fails we fall back to the Hallway and regroup before the agenda advances; nobody should spend their last resources on this one.".into();
        table.talk.outgoing = Some(table.talk.draft.clone());
    }
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Table>().talk.log[0].speaker,
        [Jacqueline]
    );
    for area in [
        Area::Test,
        Area::Overview,
        Area::Investigator(Jacqueline.index()),
    ] {
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::Focus(area));
        wait(&mut app, 2.);
        settle(&mut app);
        let beside = slot(&mut app, Jacqueline)
            .unwrap_or_else(|| panic!("Shown with the camera on {area:?}"));
        let speech = screen_rect(&app, beside);
        assert_eq!(
            speech.width(),
            WIDEST,
            "Its width with the camera on {area:?}"
        );
        let decision = panel_showing(&mut app, "LIVE DECISION");
        assert!(
            decision.intersect(speech).is_empty(),
            "With the camera on {area:?}, the speech box {speech:?} covers the decision display {decision:?}"
        );
    }
}

/// S4, Joel's co-play session 2 (#202): "When the speech dialogue faded away, it
/// fluttered/flickered; it didn't go away smoothly, but as if it wasn't rendering
/// fully well." Frame by frame at 60 frames a second through its fade, a speech box
/// stays the same node, laid out at its size on every frame, and its box, border,
/// tail and text fade together: their opacity is one value, which never rises and
/// passes through the fade before the box goes.
#[test]
fn a_speech_box_fades_as_one_node_whose_opacity_only_falls() {
    let mut talking = talking(89, &["lead:jacqueline"], &[Patrice], false);
    let partner = join_as(&mut talking, &[]);
    say(&mut talking, &partner, "Short.");
    let app = &mut talking.app;
    settle(app);
    let (_, _, first) = shown(app, Patrice)[0];
    let parts_of = |app: &App, boxed: Entity| -> Vec<Entity> {
        app.world().get::<Children>(boxed).unwrap().to_vec()
    };
    let parts = parts_of(app, first);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(16));
    let mut opacities = Vec::new();
    for frame in 0..400 {
        app.update();
        let Some(boxed) = shown(app, Patrice).first().map(|(_, _, boxed)| *boxed) else {
            break;
        };
        assert_eq!(boxed, first, "Frame {frame}: the box was respawned");
        assert_eq!(
            parts_of(app, boxed),
            parts,
            "Frame {frame}: its parts were respawned"
        );
        let world = app.world();
        let size = world.get::<ComputedNode>(boxed).unwrap().size();
        assert!(size.x > 0. && size.y > 0., "Frame {frame}: drawn at {size}");
        let mut alphas = vec![
            world.get::<BackgroundColor>(boxed).unwrap().0.alpha(),
            world.get::<BorderColor>(boxed).unwrap().top.alpha(),
        ];
        for part in &parts {
            let laid = world.get::<ComputedNode>(*part).unwrap().size();
            assert!(
                laid.x > 0. && laid.y > 0.,
                "Frame {frame}: a part at {laid}"
            );
            alphas.push(match world.get::<TextColor>(*part) {
                Some(color) => color.0.alpha(),
                None => world.get::<BackgroundColor>(*part).unwrap().0.alpha(),
            });
        }
        assert!(
            alphas.iter().all(|alpha| (alpha - alphas[0]).abs() < 1e-6),
            "Frame {frame}: box, border, tail and text out of step: {alphas:?}"
        );
        opacities.push(alphas[0]);
    }
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::ZERO);
    assert!(shown(app, Patrice).is_empty(), "It went");
    assert!(
        opacities.windows(2).all(|pair| pair[1] <= pair[0]),
        "Its opacity only falls: {opacities:?}"
    );
    let fading = opacities.iter().filter(|alpha| **alpha < 1.).count();
    assert!(
        fading >= (FADE / 0.016) as usize - 2,
        "It fades over {FADE} s, one step a frame: {opacities:?}"
    );
}
