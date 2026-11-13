//! Seam: partner pacing at the table host that runs without a graphics card (#210).
//! The human reads through the production reading panels and pointer picking; a
//! partner acts over the real socket. Expectations come from ticket #210 and the
//! presentation contract's "Agent partners and table talk".

use super::super::super::tests::{
    button_with_label, rendered_text, results::quick_thinking_session,
};
use super::*;
use crate::gathering::{
    HostileFixture,
    Investigator::{Jacqueline, Patrice, Tony},
};
use bevy::{
    camera::RenderTarget,
    picking::pointer::{
        Location, PointerAction, PointerButton, PointerId, PointerInput, PointerLocation,
    },
    window::WindowRef,
};

struct Paced {
    app: App,
    socket: PathBuf,
    _directory: tempfile::TempDir,
}

/// A table opened on `session`, saved first as the window saves it, whose
/// `partners` are handed to agent partners with partners enabled. Its viewport is
/// laid out so that the pointer can reach its panels.
fn open(session: Session, partners: &[Investigator]) -> Paced {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("pacing.session.json");
    continuity::save_session(&save, &session).unwrap();
    let socket = directory.path().join("partners.sock");
    let mut app = super::super::super::tests::host_with_table(Table::new(session, save), false);
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    app.insert_resource(PartnerSocket(socket.clone()));
    // A reading can be up from the start, which keeps the menu out of reach: the
    // menu's inputs go to the table directly.
    let table = &mut app.world_mut().resource_mut::<Table>();
    table
        .partners
        .inputs
        .extend(partners.iter().map(|who| Input::Hand(*who)));
    table.partners.inputs.push(Input::Enable(true));
    table.dirty = true;
    app.update();
    app.update();
    Paced {
        app,
        socket,
        _directory: directory,
    }
}

/// Let `seconds` of presentation time pass in one frame, then run one more frame
/// with no time passing, so a queued Continue is handled.
fn wait(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_secs_f32(seconds));
    app.update();
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::ZERO);
    app.update();
}

fn result_up(app: &App) -> bool {
    app.world()
        .resource::<Table>()
        .presentation
        .result()
        .is_some()
}

/// The reading's countdown line, as the human reads it.
fn countdown(app: &mut App) -> Option<String> {
    rendered_text(app)
        .lines()
        .find(|line| line.starts_with("Continues on its own") || line.starts_with("Held"))
        .map(str::to_owned)
}

/// The center of the reading's heading, where the pointer is over the reading.
fn frame_center(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    let heading = world
        .query_filtered::<Entity, With<pace::Frame>>()
        .iter(world)
        .next()
        .expect("A reading is up");
    world
        .get::<bevy::ui::UiGlobalTransform>(heading)
        .unwrap()
        .translation
}

fn location(app: &mut App, position: Vec2) -> Location {
    let world = app.world_mut();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    Location {
        target: RenderTarget::Window(WindowRef::Entity(window))
            .normalize(Some(window))
            .unwrap(),
        position,
    }
}

/// Put the mouse pointer at `position`, as the window reports it.
fn point(app: &mut App, position: Vec2) {
    let location = location(app, position);
    let world = app.world_mut();
    let existing = world
        .query::<(Entity, &PointerId)>()
        .iter(world)
        .find_map(|(entity, id)| (*id == PointerId::Mouse).then_some(entity));
    match existing {
        Some(pointer) => {
            world.get_mut::<PointerLocation>(pointer).unwrap().location = Some(location);
        }
        None => {
            world.spawn((PointerId::Mouse, PointerLocation::new(location)));
        }
    }
    app.update();
}

/// Press and release the primary button where the pointer is.
fn click(app: &mut App, position: Vec2) {
    point(app, position);
    for action in [
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        let location = location(app, position);
        app.world_mut()
            .write_message(PointerInput::new(PointerId::Mouse, location, action));
        app.update();
    }
}

/// Ticket #210, Partner readings: a partner's own test result is shown as usual,
/// then continues after its reading delay without the human's input, with visible
/// progress. Continuing it decides nothing for the rules session.
#[test]
fn a_partners_own_result_continues_after_its_delay_with_visible_progress() {
    let mut paced = open(quick_thinking_session(), &[Tony]);
    let app = &mut paced.app;
    let before = app.world().resource::<Table>().session.clone();
    assert!(result_up(app), "The result is shown as usual");
    let first = countdown(app).expect("A partner's result shows its progress");
    wait(app, 1.);
    assert!(result_up(app), "It is readable for more than a second");
    let later = countdown(app).unwrap();
    assert_ne!(first, later, "Its progress is visible as it runs");
    wait(app, 1.);
    assert!(result_up(app), "and for more than two");
    wait(app, 60.);
    assert!(
        !result_up(app),
        "It continues on its own: {}",
        rendered_text(app)
    );
    assert_eq!(app.world().resource::<Table>().session, before);
}

/// Ticket #210, Partner readings: the pointer over the reading holds it, and it runs
/// on once the pointer leaves; a click on it holds it until the human continues;
/// Enter continues it at once.
#[test]
fn hovering_or_clicking_holds_a_partners_result_and_enter_continues_it() {
    use bevy::input::keyboard::Key;
    let mut paced = open(quick_thinking_session(), &[Tony]);
    let app = &mut paced.app;
    let center = frame_center(app);
    point(app, center);
    wait(app, 60.);
    assert!(result_up(app), "Held while the pointer is over it");
    assert!(
        countdown(app).unwrap().contains("Held"),
        "{:?}",
        countdown(app)
    );
    point(app, Vec2::new(4., 4.));
    wait(app, 60.);
    assert!(!result_up(app), "It runs on once the pointer leaves");

    let mut paced = open(quick_thinking_session(), &[Tony]);
    let app = &mut paced.app;
    let center = frame_center(app);
    click(app, center + Vec2::new(0., -10.));
    point(app, Vec2::new(4., 4.));
    wait(app, 60.);
    assert!(result_up(app), "A click holds it until Continue");
    let entity = button_with_label(app, "Continue").unwrap();
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(entity),
        "Its Continue holds keyboard focus"
    );
    keyboard(app, KeyCode::Enter, Key::Enter, true);
    keyboard(app, KeyCode::Enter, Key::Enter, false);
    assert!(!result_up(app), "Enter continues it at once");
}

fn keyboard(app: &mut App, code: KeyCode, logical_key: bevy::input::keyboard::Key, pressed: bool) {
    let world = app.world_mut();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    world.write_message(bevy::input::keyboard::KeyboardInput {
        key_code: code,
        logical_key,
        state: if pressed {
            bevy::input::ButtonState::Pressed
        } else {
            bevy::input::ButtonState::Released
        },
        text: None,
        repeat: false,
        window,
    });
    app.update();
}

/// How long the result stays up without input, to the quarter second.
fn delay(reduced_motion: bool) -> f32 {
    let mut paced = open(quick_thinking_session(), &[Tony]);
    let app = &mut paced.app;
    app.world_mut().resource_mut::<Table>().reduced_motion = reduced_motion;
    app.world_mut().resource_mut::<Table>().dirty = true;
    app.update();
    let mut shown = 0.;
    while result_up(app) {
        assert!(shown < 120., "The reading never continued");
        assert!(
            countdown(app).is_some(),
            "Its progress stays visible with reduced motion too"
        );
        wait(app, 0.25);
        shown += 0.25;
    }
    shown
}

/// Ticket #210, Partner readings: with reduced motion the delay is unchanged.
#[test]
fn reduced_motion_keeps_a_partners_reading_delay() {
    let animated = delay(false);
    assert!(animated > 2., "{animated}");
    assert_eq!(delay(true), animated);
}

/// Ticket #210, Readings that still wait: a test result involving the human's
/// investigator, a story reading and the scenario's end each wait for the human's
/// Continue, even when partners play every other investigator.
#[test]
fn the_humans_results_story_readings_and_the_scenarios_end_wait_for_continue() {
    // Tony's own result, with partners playing the others.
    let mut paced = open(quick_thinking_session(), &[Patrice, Jacqueline]);
    wait(&mut paced.app, 120.);
    assert!(result_up(&paced.app), "The human's own result waits");
    assert!(countdown(&mut paced.app).is_none());

    // Act 2's story, with partners playing every investigator.
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    let revision = session.observe().revision;
    session.choose(revision, "act:pay:3:3:3").unwrap();
    let mut paced = open(session, &Investigator::ALL);
    let app = &mut paced.app;
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .story()
            .is_some()
    );
    wait(app, 120.);
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .story()
            .is_some(),
        "A story reading waits"
    );

    // The scenario's end: the Ghoul Priest falls and Act 3 ends the scenario.
    let mut ending = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    defeat_the_priest(&mut ending);
    let mut paced = open(ending, &Investigator::ALL);
    let app = &mut paced.app;
    for _ in 0..4 {
        wait(app, 60.);
    }
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .story()
            .is_some(),
        "The scenario's end waits: {}",
        rendered_text(app)
    );
}

/// Defeat the Ghoul Priest with Tony's basic Fight, to the scenario's end.
fn defeat_the_priest(session: &mut Session) {
    let choose = |session: &mut Session, id: &str| {
        let revision = session.observe().revision;
        session.choose(revision, id).unwrap();
    };
    choose(session, "action:ordinary:fight:400");
    for _ in 0..8 {
        let choices = session.observe().choices;
        if choices.iter().any(|choice| choice.id == "resolution:burn") {
            return;
        }
        let id = [
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| choices.iter().any(|choice| choice.id == *id))
        .expect("A basic fight has a pending test step");
        choose(session, id);
    }
    panic!("The Priest's defeat never reached the scenario's end");
}

/// Ticket #210, Encounter cards: a partner's choice to resolve the encounter card its
/// investigator drew is held while the card is read, and applied no earlier than the
/// reading delay after the card is shown.
#[test]
fn a_partners_encounter_choice_waits_for_the_reading_delay() {
    let mut paced = open(
        Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision),
        &[Patrice],
    );
    let joined = exchange(&mut paced.app, &paced.socket, &[json!({"op": "join"})]);
    let partner = joined[0]["partner"].as_str().unwrap().to_owned();
    let app = &mut paced.app;
    assert!(rendered_text(app).contains("ENCOUNTER CARD"));
    assert!(countdown(app).is_some(), "Its reading shows progress");
    let revision = app.world().resource::<Table>().session.revision();
    let chosen = connect(
        &paced.socket,
        &[
            json!({"op": "choose", "partner": partner, "revision": revision, "id": "revelation:resolve"}),
        ],
    );
    for _ in 0..10 {
        wait(app, 0.2);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        app.world().resource::<Table>().session.revision(),
        revision,
        "Held while the card is read"
    );
    assert!(!chosen.is_finished());
    pump(app, || chosen.is_finished());
    let answer = chosen.join().unwrap();
    assert_eq!(answer[0]["status"], "chosen", "{}", answer[0]);
    assert_eq!(
        app.world().resource::<Table>().session.revision(),
        revision + 1
    );
}
