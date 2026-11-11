//! Seam 2: the group clue payment chooser in the production registration.
//! Expectations come from the spec's chooser decisions and The Barrier's and
//! Trapped's requirements (3 and 2 clues per investigator who started).
use super::*;
use crate::gathering::{HostileFixture, Session};
use bevy::input::keyboard::Key;
use bevy::ui::InteractionDisabled;

const HALLWAY: &str = "01112";
const CELLAR: &str = "01114";
const STUDY: &str = "01111";
const BARRIER: &str = "Spend 9 clues as a group: advance The Barrier…";
const TRAPPED: &str = "Spend 6 clues as a group: advance Trapped…";

/// The Barrier's round-end Objective with these clues held in these rooms.
fn barrier(placed: [(u8, &str); 3]) -> Session {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    for (who, (clues, room)) in Investigator::ALL.into_iter().zip(placed) {
        session.place_for_test(who, clues, room);
    }
    session
}

/// Tony's first action choice of real play, with clues placed on the party.
fn trapped(clues: [u8; 3]) -> Session {
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
    for (who, clues) in Investigator::ALL.into_iter().zip(clues) {
        session.place_for_test(who, clues, STUDY);
    }
    session
}

/// A table for this session. A session with placed clues is not a replayable
/// record, so it is never saved: such a table can edit and cancel, not commit.
fn table(session: &Session) -> (tempfile::TempDir, std::path::PathBuf, App) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("payment.json");
    if session.validate().is_ok() {
        continuity::save_session(&path, session).unwrap();
    }
    let app = host_with_table(Table::new(session.clone(), path.clone()), false);
    (directory, path, app)
}

fn press(app: &mut App, label: &str) {
    let entity = choice_button(app, label).unwrap_or_else(|| panic!("Missing {label}"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

fn key(app: &mut App, code: KeyCode, logical: Key) {
    keyboard(app, code, logical.clone(), true);
    keyboard(app, code, logical, false);
}

fn open(app: &mut App) -> bool {
    app.world().resource::<Table>().clue_payment.is_some()
}

fn enabled(app: &mut App, label: &str) -> bool {
    let entity = choice_button(app, label).unwrap_or_else(|| panic!("Missing {label}"));
    app.world().get::<InteractionDisabled>(entity).is_none()
}

fn identity(app: &mut App, label: &str) -> Option<String> {
    let entity = choice_button(app, label).unwrap_or_else(|| panic!("Missing {label}"));
    app.world()
        .get::<ChoiceControl>(entity)
        .map(|choice| choice.0.clone())
}

fn row_label(who: Investigator, held: u8, room: &str, pays: u8) -> String {
    format!(
        "{} · {held} clue{} held · {room} · pays {pays}",
        who.name(),
        if held == 1 { "" } else { "s" }
    )
}

fn add(who: Investigator) -> String {
    format!("Add 1 clue from {}", who.name())
}

fn remove(who: Investigator) -> String {
    format!("Remove 1 clue from {}", who.name())
}

#[test]
fn the_barrier_opens_from_the_decision_bar_prefilled_with_its_only_split_and_pays_once() {
    let session = barrier([(3, HALLWAY), (3, HALLWAY), (3, HALLWAY)]);
    let (_directory, path, mut app) = table(&session);
    assert!(choice_button(&mut app, "Do not advance The Barrier at this round end").is_some());
    press(&mut app, BARRIER);
    assert!(open(&mut app));
    let text = rendered_text(&mut app);
    assert!(text.contains("Paid 9 / 9 clues"), "{text}");
    for who in Investigator::ALL {
        assert!(choice_button(&mut app, &row_label(who, 3, "Hallway", 3)).is_some());
        assert!(!enabled(&mut app, &add(who)), "The requirement is met");
    }
    assert!(choice_button(&mut app, "Cancel").is_none());
    assert_eq!(
        identity(&mut app, "Do not advance").as_deref(),
        Some("round-end:decline-act")
    );
    assert_eq!(
        identity(&mut app, "Spend 9 clues and advance").as_deref(),
        Some("act:pay:3:3:3")
    );
    assert_eq!(app.world().resource::<Table>().session, session);

    // Escape closes the chooser; it also opens from the Act card.
    key(&mut app, KeyCode::Escape, Key::Escape);
    assert!(!open(&mut app));
    assert_eq!(app.world().resource::<Table>().session, session);
    click_source(&mut app, controls::Source::Act);
    press(&mut app, BARRIER);
    assert!(open(&mut app));

    press(&mut app, "Spend 9 clues and advance");
    let committed = app.world().resource::<Table>().session.clone();
    let after = committed.observe();
    assert_eq!(after.revision, session.observe().revision + 1);
    assert_eq!(after.play.as_ref().unwrap().act, 3);
    assert!(after.play.unwrap().players.iter().all(|p| p.clues == 0));
    assert!(!open(&mut app));
    assert_eq!(continuity::load_session(path).unwrap(), committed);
}

#[test]
fn do_not_advance_declines_the_barrier_without_spending() {
    let session = barrier([(3, HALLWAY), (3, HALLWAY), (3, HALLWAY)]);
    let (_directory, _path, mut app) = table(&session);
    press(&mut app, BARRIER);
    press(&mut app, "Do not advance");
    let after = app.world().resource::<Table>().session.observe();
    assert_eq!(after.revision, session.observe().revision + 1);
    let play = after.play.unwrap();
    assert_eq!(play.act, 2);
    assert!(play.players.iter().all(|p| p.clues == 3));
    assert!(!open(&mut app));
}

#[test]
fn steppers_stop_at_holdings_and_the_requirement_and_spend_needs_the_exact_total() {
    use Investigator::{Jacqueline, Patrice, Tony};
    let session = barrier([(5, HALLWAY), (4, HALLWAY), (2, HALLWAY)]);
    let (_directory, _path, mut app) = table(&session);
    press(&mut app, BARRIER);
    // Several splits pay, so the chooser starts empty.
    assert!(rendered_text(&mut app).contains("Paid 0 / 9 clues"));
    assert!(!enabled(&mut app, &remove(Tony)));
    assert!(!enabled(&mut app, "Spend 9 clues and advance"));
    assert_eq!(identity(&mut app, "Spend 9 clues and advance"), None);
    for _ in 0..3 {
        press(&mut app, &add(Jacqueline));
    }
    assert!(choice_button(&mut app, &row_label(Jacqueline, 2, "Hallway", 2)).is_some());
    assert!(
        !enabled(&mut app, &add(Jacqueline)),
        "Holdings cap the stepper"
    );
    for _ in 0..5 {
        press(&mut app, &add(Tony));
    }
    assert!(!enabled(&mut app, &add(Tony)));
    press(&mut app, &add(Patrice));
    assert!(rendered_text(&mut app).contains("Paid 8 / 9 clues"));
    assert!(!enabled(&mut app, "Spend 9 clues and advance"));
    press(&mut app, &add(Patrice));
    assert!(rendered_text(&mut app).contains("Paid 9 / 9 clues"));
    assert!(
        !enabled(&mut app, &add(Patrice)),
        "The remaining requirement caps the stepper below Patrice's holdings"
    );
    assert!(choice_button(&mut app, &row_label(Patrice, 4, "Hallway", 2)).is_some());
    assert_eq!(
        identity(&mut app, "Spend 9 clues and advance").as_deref(),
        Some("act:pay:5:2:2")
    );
    press(&mut app, &remove(Tony));
    assert!(!enabled(&mut app, "Spend 9 clues and advance"));
    assert_eq!(
        app.world().resource::<Table>().session,
        session,
        "Editing spends nothing"
    );
}

#[test]
fn an_ineligible_investigator_is_shown_disabled_with_the_reason() {
    use Investigator::{Jacqueline, Patrice, Tony};
    let session = barrier([(6, HALLWAY), (3, HALLWAY), (4, CELLAR)]);
    let (_directory, _path, mut app) = table(&session);
    press(&mut app, BARRIER);
    let row = format!(
        "{} · 4 clues held · Cellar · Not in the Hallway",
        Jacqueline.name()
    );
    assert!(!enabled(&mut app, &row));
    assert!(choice_button(&mut app, &add(Jacqueline)).is_none());
    // The only split from the Hallway is pre-filled.
    assert!(choice_button(&mut app, &row_label(Tony, 6, "Hallway", 6)).is_some());
    assert!(choice_button(&mut app, &row_label(Patrice, 3, "Hallway", 3)).is_some());
    assert_eq!(
        identity(&mut app, "Spend 9 clues and advance").as_deref(),
        Some("act:pay:6:3:0")
    );
}

#[test]
fn the_keyboard_tabs_between_rows_changes_the_focused_stepper_and_pays_with_enter() {
    use Investigator::{Patrice, Tony};
    let session = barrier([(3, HALLWAY), (3, HALLWAY), (3, HALLWAY)]);
    let (_directory, path, mut app) = table(&session);
    press(&mut app, BARRIER);
    let focused = |app: &mut App| {
        let entity = app.world().resource::<InputFocus>().get().unwrap();
        app.world()
            .get::<AccessibleLabel>(entity)
            .unwrap()
            .0
            .clone()
    };
    assert_eq!(focused(&mut app), row_label(Tony, 3, "Hallway", 3));
    key(&mut app, KeyCode::ArrowDown, Key::ArrowDown);
    key(&mut app, KeyCode::ArrowLeft, Key::ArrowLeft);
    assert_eq!(focused(&mut app), row_label(Tony, 3, "Hallway", 1));
    key(&mut app, KeyCode::Tab, Key::Tab);
    assert_eq!(focused(&mut app), row_label(Patrice, 3, "Hallway", 3));
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    assert_eq!(
        focused(&mut app),
        row_label(Patrice, 3, "Hallway", 3),
        "Holdings cap the focused stepper"
    );
    key(&mut app, KeyCode::Enter, Key::Enter);
    assert_eq!(
        app.world().resource::<Table>().session,
        session,
        "Enter pays only the exact requirement"
    );
    assert!(open(&mut app));
    // Tab cycles through the remaining row and the buttons back to Tony.
    let tony = row_label(Tony, 3, "Hallway", 1);
    for _ in 0..6 {
        if focused(&mut app) == tony {
            break;
        }
        key(&mut app, KeyCode::Tab, Key::Tab);
    }
    assert_eq!(focused(&mut app), tony);
    key(&mut app, KeyCode::ArrowRight, Key::ArrowRight);
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    key(&mut app, KeyCode::ArrowUp, Key::ArrowUp);
    assert_eq!(focused(&mut app), row_label(Tony, 3, "Hallway", 3));
    key(&mut app, KeyCode::Enter, Key::Enter);
    let committed = app.world().resource::<Table>().session.clone();
    let play = committed.observe().play.unwrap();
    assert_eq!(play.act, 3);
    assert!(play.players.iter().all(|player| player.clues == 0));
    assert_eq!(continuity::load_session(path).unwrap(), committed);
}

#[test]
fn trapped_opens_from_the_act_card_and_cancel_or_escape_spends_nothing() {
    use Investigator::{Jacqueline, Tony};
    let session = trapped([4, 2, 1]);
    let (_directory, path, mut app) = table(&session);
    let view = session.observe();
    assert!(
        controls::source_choices(&view, &controls::Source::Investigator(Tony))
            .iter()
            .all(|choice| !choice.id.starts_with("act:")),
        "Investigator menus list no clue rows"
    );
    click_source(&mut app, controls::Source::Act);
    press(&mut app, TRAPPED);
    assert!(rendered_text(&mut app).contains("Paid 0 / 6 clues"));
    assert!(choice_button(&mut app, "Do not advance").is_none());
    press(&mut app, &add(Tony));
    press(&mut app, &add(Jacqueline));
    press(&mut app, "Cancel");
    assert!(!open(&mut app));
    assert_eq!(app.world().resource::<Table>().session, session);
    assert!(!path.exists(), "Nothing was saved");

    click_source(&mut app, controls::Source::Act);
    press(&mut app, TRAPPED);
    assert!(
        rendered_text(&mut app).contains("Paid 0 / 6 clues"),
        "A reopened chooser starts empty again"
    );
    press(&mut app, &add(Tony));
    key(&mut app, KeyCode::Escape, Key::Escape);
    assert!(!open(&mut app));
    assert_eq!(app.world().resource::<Table>().session, session);
    assert!(!path.exists(), "Nothing was saved");
}

#[test]
fn the_chooser_with_portraits_fits_the_viewport_and_keeps_its_buttons_visible() {
    use Investigator::{Jacqueline, Patrice, Tony};
    for size in [
        Vec2::new(1600., 1000.),
        Vec2::new(1280., 800.),
        Vec2::new(960., 640.),
    ] {
        let session = barrier([(5, HALLWAY), (4, HALLWAY), (2, CELLAR)]);
        let (_directory, _path, mut app) = table(&session);
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.insert_resource(art::Art::with_images_for_test(
            ["standee/06003", "standee/06005", "standee/60401"].map(|role| (role, image.clone())),
        ));
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
        press(&mut app, BARRIER);
        for _ in 0..5 {
            app.update();
        }
        for label in [
            row_label(Tony, 5, "Hallway", 5),
            row_label(Patrice, 4, "Hallway", 4),
            format!(
                "{} · 2 clues held · Cellar · Not in the Hallway",
                Jacqueline.name()
            ),
            add(Tony),
            remove(Patrice),
            "Do not advance".into(),
            "Spend 9 clues and advance".into(),
        ] {
            assert_unclipped_button(&mut app, &label, size, 1.);
        }
    }
}
