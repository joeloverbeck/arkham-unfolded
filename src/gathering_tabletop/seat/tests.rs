//! The seat's protocol is the only seam: these tests send requests and assert on the
//! responses, the saved scenario attempt and written files. The no-GPU table host has
//! no mesh picking, so pointer activation of 3D components is covered only by the
//! ignored real-artwork run at the end of this file.

use super::super::tests::{click_source, host_with_pointer_input, host_with_table};
use super::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{BufRead, Write},
    os::unix::net::UnixStream,
    path::Path,
};

const LEAD_TONY: &str = "lead:tony";

struct Sitting {
    app: App,
    seat: Seat<Gathering>,
    path: PathBuf,
    _directory: tempfile::TempDir,
}

impl Sitting {
    fn ask(&mut self, request: Value) -> Value {
        self.seat.respond(&mut self.app, &request.to_string()).0
    }

    /// Mesh picking is absent here, so a synthetic intersection opens the menu.
    /// Its rows are then found and pressed through real UI layout and picking.
    fn open_menu(&mut self, who: Investigator) {
        click_source(&mut self.app, controls::Source::Investigator(who));
        let settled = self.ask(json!({"op": "settle"}));
        assert_eq!(settled["status"], "settled", "{settled}");
    }
}

fn sit_at(mut app: App, path: PathBuf, directory: tempfile::TempDir) -> Sitting {
    // Supply only renderer metadata; layout, picking and actions are production.
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    let world = app.world_mut();
    let camera = world
        .query_filtered::<Entity, With<MainCamera>>()
        .single(world)
        .unwrap();
    let seat = Seat::new(Gathering, &mut app, camera, None);
    let mut sitting = Sitting {
        seat,
        app,
        path,
        _directory: directory,
    };
    let settled = sitting.ask(json!({"op": "settle"}));
    assert_eq!(settled["status"], "settled", "{settled}");
    sitting
}

fn sit() -> Sitting {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("attempt.json");
    sit_at(host_with_pointer_input(path.clone(), true), path, directory)
}

fn sit_at_fixture(fixture: crate::gathering::HostileFixture) -> Sitting {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fixture.json");
    let table = Table::new(Session::with_hostile_fixture(fixture), path.clone());
    sit_at(host_with_table(table, true), path, directory)
}

fn rows(response: &Value) -> Vec<Value> {
    response["table"]["menu"]["rows"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn component<'a>(response: &'a Value, name: &str) -> &'a Value {
    response["table"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|component| component["name"] == name)
        .unwrap_or_else(|| panic!("{name} is not shown: {}", response["table"]["components"]))
}

fn saved(path: &std::path::Path) -> Value {
    json!(continuity::load_session(path).unwrap().observe())
}

#[test]
fn observation_is_the_headless_drivers_for_the_same_session() {
    let mut seat = sit();
    let response = seat.ask(json!({"op": "observe"}));
    assert_eq!(response["ok"], true);
    assert_eq!(response["mode"], "seat");
    assert_eq!(response["status"], "observed");
    assert_eq!(
        response["observation"],
        json!(Session::with_seed(19).observe())
    );
    assert_eq!(response["table"]["settled"], true);
    assert_eq!(response["table"]["pending"], json!([]));
    assert_eq!(response["table"]["modal"], Value::Null);
    assert_eq!(response["table"]["menu"], Value::Null);
}

#[test]
fn a_choice_control_pressed_through_ui_picking_commits_and_saves() {
    let mut seat = sit();
    seat.open_menu(Investigator::Tony);
    let menu = seat.ask(json!({"op": "observe"}));
    assert_eq!(menu["table"]["menu"]["component"], "investigator:tony");
    let lead = rows(&menu)
        .into_iter()
        .find(|row| row["choice"] == LEAD_TONY)
        .expect("the lead choice is a named menu row");
    assert_eq!(lead["label"], "Choose Tony Morgan as lead investigator");
    assert_eq!(lead["on_screen"], true);
    assert!(!seat.path.exists(), "selecting a component commits nothing");

    let activated = seat.ask(json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}));
    assert_eq!(activated["status"], "activated", "{activated}");
    assert_eq!(activated["position"], lead["position"]);
    assert_eq!(saved(&seat.path)["revision"], 1);
    // The observation is the driver's at once; the table is still dealing the hands.
    assert_eq!(activated["observation"], saved(&seat.path));
    assert_eq!(activated["table"]["shows_latest"], false);

    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(settled["status"], "settled", "{settled}");
    assert_eq!(settled["observation"]["revision"], 1);
    assert_eq!(settled["observation"]["lead"], "tony");
    assert_eq!(settled["observation"], saved(&seat.path));
    assert_eq!(settled["table"]["shows_latest"], true);
    assert_eq!(settled["table"]["menu"], Value::Null);
    assert_eq!(settled["table"]["notice"], Value::Null);

    // A decision-panel control carries its choice identity as a menu row does.
    let keep = settled["table"]["controls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|control| control["choice"] == "confirm-mulligan")
        .expect("the panel's confirmation is a named control");
    assert_eq!(keep["label"], "Keep this opening hand");
    let kept = seat.ask(json!({"op": "activate", "revision": 1, "choice": "confirm-mulligan"}));
    assert_eq!(kept["status"], "activated", "{kept}");
    assert_eq!(kept["observation"]["revision"], 2);
    assert_eq!(kept["observation"], saved(&seat.path));
}

#[test]
fn activation_is_refused_while_the_table_is_busy_and_settle_names_what_remains() {
    let mut seat = sit();
    seat.open_menu(Investigator::Tony);
    let activated = seat.ask(json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}));
    assert_eq!(activated["status"], "activated", "{activated}");
    assert_eq!(activated["table"]["settled"], false);

    let before = std::fs::read(&seat.path).unwrap();
    let busy = seat.ask(json!({"op": "activate", "revision": 0, "component": "investigator:tony"}));
    assert_eq!(busy["ok"], false);
    assert_eq!(busy["status"], "refused");
    assert!(
        busy["error"].as_str().unwrap().contains("busy"),
        "{}",
        busy["error"]
    );
    // A key pressed now could be dropped just as silently as a pointer press.
    let key = seat.ask(json!({"op": "key", "key": "Enter", "revision": 1, "focused": "Overview"}));
    assert_eq!(key["status"], "refused");
    assert!(key["error"].as_str().unwrap().contains("busy"));

    let limited = seat.ask(json!({"op": "settle", "steps": 1}));
    assert_eq!(limited["ok"], false);
    assert_eq!(limited["status"], "unsettled");
    assert_eq!(limited["steps"], 1);
    let remaining = limited["table"]["pending"].as_array().unwrap();
    assert!(!remaining.is_empty());
    for condition in remaining {
        assert!(
            limited["error"]
                .as_str()
                .unwrap()
                .contains(condition.as_str().unwrap()),
            "{limited}"
        );
    }

    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(settled["status"], "settled", "{settled}");
    assert!(settled["steps"].as_u64().unwrap() > 0);
    assert_eq!(settled["table"]["pending"], json!([]));
    assert_eq!(std::fs::read(&seat.path).unwrap(), before);
}

#[test]
fn an_outdated_choice_is_refused_without_touching_the_attempt() {
    let mut seat = sit();
    seat.open_menu(Investigator::Tony);
    let stale = seat.ask(json!({"op": "activate", "revision": 7, "choice": LEAD_TONY}));
    assert_eq!(stale["status"], "refused");
    assert!(
        stale["error"].as_str().unwrap().starts_with("Outdated"),
        "{}",
        stale["error"]
    );
    assert_eq!(stale["observation"]["revision"], 0);
    assert!(!seat.path.exists());

    seat.ask(json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}));
    seat.ask(json!({"op": "settle"}));
    let before = std::fs::read(&seat.path).unwrap();
    seat.open_menu(Investigator::Tony);
    let replayed = seat.ask(json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}));
    assert_eq!(replayed["status"], "refused");
    assert!(replayed["error"].as_str().unwrap().starts_with("Outdated"));
    assert_eq!(std::fs::read(&seat.path).unwrap(), before);
}

#[test]
fn names_are_exact_and_absent_ambiguous_or_off_screen_names_fail() {
    let mut seat = sit();
    for (request, expected) in [
        (
            json!({"op": "activate", "revision": 0, "component": "investigator:ton"}),
            "No shown component is named investigator:ton.",
        ),
        (
            json!({"op": "activate", "revision": 0, "component": "Tony Morgan"}),
            "No shown component is named Tony Morgan.",
        ),
        (
            json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}),
            "No shown control offers choice lead:tony.",
        ),
    ] {
        let response = seat.ask(request);
        assert_eq!(response["status"], "refused");
        assert_eq!(response["error"], expected);
    }

    // Zoomed on one investigator's area, another investigator's card leaves the image.
    seat.app
        .world_mut()
        .resource_mut::<Orbit>()
        .focus(sideboard::focus(Area::Investigator(0)));
    seat.app.update();
    let zoomed = seat.ask(json!({"op": "settle"}));
    assert_eq!(zoomed["status"], "settled", "{zoomed}");
    assert_eq!(component(&zoomed, "investigator:tony")["on_screen"], true);
    assert_eq!(
        component(&zoomed, "investigator:jacqueline")["on_screen"],
        false
    );
    let off_screen =
        seat.ask(json!({"op": "activate", "revision": 0, "component": "investigator:jacqueline"}));
    assert_eq!(off_screen["status"], "refused");
    let error = off_screen["error"].as_str().unwrap();
    assert!(
        error.contains("off screen") || error.contains("behind the camera"),
        "{error}"
    );

    // A second shown piece under the same name must fail, never pick one.
    let twin = {
        let world = seat.app.world_mut();
        let (entity, target) = world
            .query::<(Entity, &ContextTarget)>()
            .iter(world)
            .find(|(_, target)| target.source == controls::Source::Investigator(Investigator::Tony))
            .unwrap();
        let twin = ContextTarget {
            epoch: target.epoch,
            revision: target.revision,
            source: target.source.clone(),
        };
        let at = *world.get::<Transform>(entity).unwrap();
        world.spawn((twin, at)).id()
    };
    seat.app.update();
    let ambiguous =
        seat.ask(json!({"op": "activate", "revision": 0, "component": "investigator:tony"}));
    assert_eq!(ambiguous["status"], "refused");
    assert_eq!(
        ambiguous["error"],
        "2 shown components are named investigator:tony; the name is ambiguous."
    );
    seat.app.world_mut().despawn(twin);

    seat.open_menu(Investigator::Tony);
    for request in [
        json!({"op": "activate", "revision": 0}),
        json!({"op": "activate", "revision": 0, "component": "investigator:tony", "choice": LEAD_TONY}),
    ] {
        let response = seat.ask(request);
        assert_eq!(response["status"], "refused");
        assert_eq!(
            response["error"],
            "activate needs exactly one of component or choice"
        );
    }
    assert!(!seat.path.exists());
}

impl Sitting {
    /// The planted defect: another control drawn over the open menu's row for `choice`.
    fn cover_row(&mut self, choice: &str) {
        let menu = self.ask(json!({"op": "observe"}));
        let row = rows(&menu)
            .into_iter()
            .find(|row| row["choice"] == choice)
            .unwrap_or_else(|| panic!("no row for {choice}: {}", menu["table"]["menu"]));
        let at = Vec2::new(
            row["position"][0].as_f64().unwrap() as f32,
            row["position"][1].as_f64().unwrap() as f32,
        );
        self.app.world_mut().spawn((
            Button,
            AccessibleLabel::new("Planted cover"),
            Node {
                position_type: PositionType::Absolute,
                left: px(at.x - 60.),
                top: px(at.y - 20.),
                width: px(120),
                height: px(40),
                ..default()
            },
            GlobalZIndex(1_000),
        ));
        self.app.update();
    }
}

#[test]
fn a_covered_control_reports_what_covers_it_and_presses_nothing() {
    let mut seat = sit();
    seat.open_menu(Investigator::Tony);
    seat.cover_row(LEAD_TONY);

    let missed = seat.ask(json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}));
    assert_eq!(missed["ok"], false);
    assert_eq!(missed["status"], "missed", "{missed}");
    assert_eq!(missed["pressed"], false);
    assert_eq!(missed["hit"][0], json!({"control": "Planted cover"}));
    assert!(
        missed["error"].as_str().unwrap().contains("Planted cover"),
        "{}",
        missed["error"]
    );
    assert_eq!(missed["observation"]["revision"], 0);
    assert!(!seat.path.exists(), "a covered choice must not commit");
}

#[test]
fn the_listed_pendulum_prediction_opens_its_picker_by_identity_and_commits_nothing() {
    use crate::gathering::{HostileFixture, PREDICTION_MARGIN, TestCardFixture};
    let mut seat = sit_at_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalPrediction,
    ));
    let before = seat.ask(json!({"op": "observe"}));
    let listed = before["observation"]["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|choice| choice["id"].as_str().unwrap().to_owned())
        .find(|id| id.starts_with("test-card:predict:") && id.ends_with(PREDICTION_MARGIN))
        .expect("the session lists one direct prediction choice");
    let asset = listed.split(':').nth(2).unwrap().parse().unwrap();
    click_source(&mut seat.app, controls::Source::Card(asset));
    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(settled["status"], "settled", "{settled}");
    let menu = seat.ask(json!({"op": "observe"}));
    let row = rows(&menu)
        .into_iter()
        .find(|row| row["choice"] == listed.as_str())
        .unwrap_or_else(|| panic!("the menu names {listed}: {}", menu["table"]["menu"]));
    assert_eq!(row["label"], "Exhaust Crystal Pendulum");
    let revision = before["observation"]["revision"].clone();
    let opened =
        seat.ask(json!({"op": "activate", "revision": revision, "choice": listed.as_str()}));
    assert_eq!(opened["status"], "activated", "{opened}");
    assert_eq!(opened["table"]["modal"], "pendulum prediction");
    assert_eq!(opened["observation"], before["observation"]);
    assert!(!seat.path.exists(), "opening the picker commits nothing");
}

/// The keys are the table's own, named for the shared seat, which joins them into its
/// refusal of a key the table does not read.
#[test]
fn a_key_the_table_does_not_read_is_refused_with_the_eighteen_it_does() {
    let mut seat = sit();

    let unknown = seat.ask(json!({"op": "key", "key": "F13", "revision": 0}));

    assert_eq!(unknown["status"], "refused");
    assert_eq!(
        unknown["error"],
        "Unknown key \"F13\". Known keys: Escape, Enter, Space, Tab, Home, End, PageUp, PageDown, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, KeyW, KeyA, KeyS, KeyD, Minus, Equal."
    );
}

#[test]
fn keys_travel_the_keyboard_path_and_unknown_keys_are_refused() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let menu = seat.ask(json!({"op": "observe"}));
    assert_eq!(focused(&menu).as_deref(), Some("lead:patrice"), "{menu}");

    let unknown = seat.ask(json!({"op": "key", "key": "F13", "revision": 0}));
    assert_eq!(unknown["status"], "refused");
    assert!(unknown["error"].as_str().unwrap().contains("Unknown key"));

    let moved = seat.ask(json!({"op": "key", "key": "Tab", "revision": 0}));
    assert_eq!(moved["status"], "pressed", "{moved}");
    assert_eq!(focused(&moved).as_deref(), Some("Inspect"), "{moved}");
    let inspect = rows(&moved)
        .into_iter()
        .find(|row| row["label"] == "Inspect")
        .unwrap();
    assert_eq!(
        inspect["choice"],
        Value::Null,
        "Inspect is named by its label"
    );

    let dismissed = seat.ask(json!({"op": "key", "key": "Escape", "revision": 0}));
    assert_eq!(dismissed["status"], "pressed", "{dismissed}");
    assert_eq!(dismissed["table"]["menu"], Value::Null);
    assert!(!seat.path.exists());

    seat.open_menu(Investigator::Patrice);
    let chosen =
        seat.ask(json!({"op": "key", "key": "Enter", "revision": 0, "focused": "lead:patrice"}));
    assert_eq!(chosen["status"], "pressed", "{chosen}");
    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(settled["observation"]["revision"], 1);
    assert_eq!(settled["observation"]["lead"], "patrice");
    assert_eq!(settled["observation"], saved(&seat.path));
}

#[test]
fn a_key_names_its_revision_and_an_outdated_key_presses_nothing() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let menu = seat.ask(json!({"op": "observe"}));

    let stale = seat.ask(json!({"op": "key", "key": "Tab", "revision": 7}));
    assert_eq!(stale["status"], "refused", "{stale}");
    let stale_choice = seat.ask(json!({"op": "activate", "revision": 7, "choice": "lead:patrice"}));
    assert_eq!(
        stale["error"], stale_choice["error"],
        "an outdated key is refused as an outdated activation is"
    );
    assert!(stale["error"].as_str().unwrap().starts_with("Outdated"));
    assert_eq!(rows(&stale), rows(&menu), "Tab moved no focus");
    assert_eq!(stale["observation"]["revision"], 0);

    let unnamed = seat.ask(json!({"op": "key", "key": "Tab"}));
    assert_eq!(unnamed["status"], "refused", "{unnamed}");
    assert!(
        unnamed["error"].as_str().unwrap().contains("revision"),
        "{}",
        unnamed["error"]
    );
    assert_eq!(rows(&unnamed), rows(&menu));
    assert!(!seat.path.exists());
}

#[test]
fn enter_presses_only_the_control_the_agent_names() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let menu = seat.ask(json!({"op": "observe"}));
    assert_eq!(focused(&menu).as_deref(), Some("lead:patrice"), "{menu}");
    let label = rows(&menu)
        .into_iter()
        .find(|row| row["choice"] == "lead:patrice")
        .unwrap()["label"]
        .clone();

    for request in [
        // The menu's other choice is shown, but focus is elsewhere.
        json!({"op": "key", "key": "Enter", "revision": 0, "focused": LEAD_TONY}),
        // A choice's label is not its name: the match is exact.
        json!({"op": "key", "key": "Enter", "revision": 0, "focused": label}),
        // Enter and Space can commit, so they must say what they expect to press.
        json!({"op": "key", "key": "Enter", "revision": 0}),
        json!({"op": "key", "key": "Space", "revision": 0}),
    ] {
        let refused = seat.ask(request.clone());
        assert_eq!(refused["status"], "refused", "{request}: {refused}");
        let error = refused["error"].as_str().unwrap();
        assert!(
            error.contains("lead:patrice"),
            "the refusal says what has focus: {request}: {error}"
        );
        assert_eq!(refused["observation"]["revision"], 0, "{request}");
        assert_eq!(
            rows(&refused),
            rows(&menu),
            "{request}: nothing was pressed"
        );
        assert!(!seat.path.exists(), "{request} committed nothing");
    }

    let chosen =
        seat.ask(json!({"op": "key", "key": "Enter", "revision": 0, "focused": "lead:patrice"}));
    assert_eq!(chosen["status"], "pressed", "{chosen}");
    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(settled["observation"]["revision"], 1);
    assert_eq!(settled["observation"]["lead"], "patrice");
    assert_eq!(settled["observation"], saved(&seat.path));
}

#[test]
fn enter_with_nothing_focused_presses_nothing() {
    let mut seat = sit();
    // The table keeps a control focused nearly always; clearing focus stands for a
    // table where nothing has it.
    seat.app.world_mut().resource_mut::<InputFocus>().clear();
    let unfocused = seat.ask(json!({"op": "observe"}));
    assert_eq!(
        focused(&unfocused),
        None,
        "{}",
        unfocused["table"]["controls"]
    );

    let refused =
        seat.ask(json!({"op": "key", "key": "Enter", "revision": 0, "focused": "Overview"}));
    assert_eq!(refused["status"], "refused", "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("nothing has focus"),
        "{}",
        refused["error"]
    );
    assert_eq!(refused["observation"]["revision"], 0);
    assert_eq!(refused["table"], unfocused["table"], "nothing was pressed");
    assert!(!seat.path.exists());
}

#[test]
fn enter_naming_continue_dismisses_a_test_result_and_decides_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result.json");
    let session = super::super::tests::results::quick_thinking_session();
    let table = Table::new(session, path.clone());
    let mut seat = sit_at(host_with_table(table, true), path, directory);
    let reading = seat.ask(json!({"op": "observe"}));
    assert_eq!(
        reading["table"]["modal"], "test result",
        "{}",
        reading["table"]
    );
    assert_eq!(focused(&reading).as_deref(), Some("Continue"));
    let revision = reading["observation"]["revision"].clone();

    let unnamed = seat.ask(json!({"op": "key", "key": "Enter", "revision": revision}));
    assert_eq!(unnamed["status"], "refused", "{unnamed}");
    assert!(
        unnamed["error"].as_str().unwrap().contains(r#""Continue""#),
        "the refusal says what has focus: {}",
        unnamed["error"]
    );
    assert_eq!(unnamed["table"]["modal"], "test result");

    let dismissed =
        seat.ask(json!({"op": "key", "key": "Enter", "revision": revision, "focused": "Continue"}));
    assert_eq!(dismissed["status"], "pressed", "{dismissed}");
    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(settled["status"], "settled", "{settled}");
    assert_ne!(
        settled["table"]["modal"], "test result",
        "{}",
        settled["table"]
    );
    assert_eq!(
        settled["observation"]["revision"], revision,
        "a test result's Continue commits no choice"
    );
}

#[test]
fn keys_that_cannot_commit_need_only_the_revision_and_a_named_focus_is_checked() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let moved = seat.ask(json!({"op": "key", "key": "Tab", "revision": 0}));
    assert_eq!(moved["status"], "pressed", "{moved}");
    assert_eq!(focused(&moved).as_deref(), Some("Inspect"));

    let wrong =
        seat.ask(json!({"op": "key", "key": "Tab", "revision": 0, "focused": "lead:patrice"}));
    assert_eq!(wrong["status"], "refused", "{wrong}");
    assert!(
        wrong["error"].as_str().unwrap().contains(r#""Inspect""#),
        "{}",
        wrong["error"]
    );
    assert_eq!(
        focused(&wrong).as_deref(),
        Some("Inspect"),
        "Tab moved no focus"
    );

    let named = seat.ask(json!({"op": "key", "key": "Tab", "revision": 0, "focused": "Inspect"}));
    assert_eq!(named["status"], "pressed", "{named}");
    assert_ne!(focused(&named).as_deref(), Some("Inspect"));
    assert!(!seat.path.exists());
}

/// Send lines over one connection, as batch form sends standard input, and return
/// the responses after `ready`.
fn converse(seat: &mut Sitting, lines: &[String]) -> Vec<Value> {
    let input: String = lines.iter().map(|line| format!("{line}\n")).collect();
    let mut output = Vec::new();
    seat.seat
        .serve(&mut seat.app, input.as_bytes(), &mut output)
        .unwrap();
    let mut responses: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.remove(0)["status"], "ready");
    responses
}

fn statuses(responses: &[Value]) -> Vec<&str> {
    responses
        .iter()
        .map(|response| response["status"].as_str().unwrap())
        .collect()
}

#[test]
fn after_a_refusal_the_connection_skips_its_activations_and_keys() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let menu = seat.ask(json!({"op": "observe"}));
    let capture = seat.path.with_extension("png");
    let lines: Vec<String> = [
        // Focus is on lead:patrice, so this Enter is refused.
        json!({"op": "key", "key": "Enter", "revision": 0, "focused": LEAD_TONY}),
        json!({"op": "activate", "revision": 0, "choice": "lead:patrice"}),
        json!({"op": "key", "key": "Enter", "revision": 0, "focused": "lead:patrice"}),
        json!({"op": "key", "key": "Tab", "revision": 0}),
        json!({"op": "observe"}),
        json!({"op": "settle"}),
        json!({"op": "capture", "path": capture}),
        json!({"op": "quit"}),
        json!({"op": "observe"}),
    ]
    .iter()
    .map(Value::to_string)
    .collect();
    let responses = converse(&mut seat, &lines);
    assert_eq!(
        statuses(&responses),
        [
            "refused", "skipped", "skipped", "skipped", "observed", "settled", "refused", "bye"
        ]
    );
    let reason = responses[0]["error"].as_str().unwrap();
    for skipped in &responses[1..4] {
        assert_eq!(skipped["ok"], false);
        let error = skipped["error"].as_str().unwrap();
        assert!(
            error.contains(&lines[0]) && error.contains(reason),
            "a skip names the refused request and its reason: {error}"
        );
        assert_eq!(skipped["observation"]["revision"], 0);
        assert_eq!(rows(skipped), rows(&menu), "nothing was pressed");
    }
    // Capture is served: this host refuses it for its own reason.
    assert!(
        responses[6]["error"]
            .as_str()
            .unwrap()
            .contains("renders no image"),
        "{}",
        responses[6]["error"]
    );
    assert_eq!(rows(&responses[5]), rows(&menu));
    assert!(
        !seat.path.exists(),
        "the skipped requests committed nothing"
    );
}

#[test]
fn a_malformed_line_stops_the_connection_and_the_next_connection_starts_afresh() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let activate = json!({"op": "activate", "revision": 0, "choice": "lead:patrice"}).to_string();
    let responses = converse(
        &mut seat,
        &[
            r#"{"op":"key","key":"Enter","revison":0}"#.to_owned(),
            activate.clone(),
            json!({"op": "observe"}).to_string(),
        ],
    );
    assert_eq!(statuses(&responses), ["refused", "skipped", "observed"]);
    assert!(
        responses[1]["error"]
            .as_str()
            .unwrap()
            .contains(r#"{"op":"key","key":"Enter","revison":0}"#),
        "{}",
        responses[1]["error"]
    );
    assert!(!seat.path.exists());

    let responses = converse(&mut seat, &[activate, json!({"op": "settle"}).to_string()]);
    assert_eq!(statuses(&responses), ["activated", "settled"]);
    assert_eq!(responses[1]["observation"]["lead"], "patrice");
    assert_eq!(responses[1]["observation"], saved(&seat.path));
}

#[test]
fn a_missed_activation_stops_the_connection_as_a_refusal_does() {
    let mut seat = sit();
    seat.open_menu(Investigator::Tony);
    seat.cover_row(LEAD_TONY);
    let menu = seat.ask(json!({"op": "observe"}));
    assert_eq!(focused(&menu).as_deref(), Some(LEAD_TONY), "{menu}");
    // The agent planned Enter on the focused row after the activation; the miss shows
    // the table was not as assumed, so the Enter must not commit Tony as lead.
    let activate = json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}).to_string();
    let enter =
        json!({"op": "key", "key": "Enter", "revision": 0, "focused": LEAD_TONY}).to_string();
    let responses = converse(
        &mut seat,
        &[
            activate.clone(),
            enter.clone(),
            json!({"op": "observe"}).to_string(),
        ],
    );
    assert_eq!(statuses(&responses), ["missed", "skipped", "observed"]);
    let error = responses[1]["error"].as_str().unwrap();
    assert!(
        error.contains(&activate) && error.contains("Planted cover"),
        "a skip names the missed request and why it missed: {error}"
    );
    assert_eq!(responses[1]["observation"]["revision"], 0);
    assert_eq!(rows(&responses[1]), rows(&menu), "nothing was pressed");
    assert!(!seat.path.exists(), "the skipped Enter committed nothing");

    // On a connection of its own the same Enter presses the row it names.
    let responses = converse(&mut seat, &[enter, json!({"op": "settle"}).to_string()]);
    assert_eq!(statuses(&responses), ["pressed", "settled"]);
    assert_eq!(responses[1]["observation"]["lead"], "tony");
    assert_eq!(responses[1]["observation"], saved(&seat.path));
}

/// The one focused control or contextual-menu row as a guarded key names it: its
/// choice identity, or its label when it has none. `None` when neither has focus.
fn focused(response: &Value) -> Option<String> {
    let table = &response["table"];
    let controls = table["controls"].as_array().into_iter().flatten();
    let rows = table["menu"]["rows"].as_array().into_iter().flatten();
    let mut focused = controls
        .chain(rows)
        .filter(|control| control["focused"] == true);
    let control = focused.next()?;
    assert!(focused.next().is_none(), "one control has focus: {table}");
    control["choice"]
        .as_str()
        .or_else(|| control["label"].as_str())
        .map(str::to_owned)
}

impl Sitting {
    /// Press Tab until the control a guarded key names `name` has focus, as an agent
    /// without a pointer would.
    fn tab_to(&mut self, name: &str) -> Value {
        let mut response = self.ask(json!({"op": "observe"}));
        for _ in 0..40 {
            if focused(&response).as_deref() == Some(name) {
                return response;
            }
            let revision = &response["observation"]["revision"];
            response = self.ask(json!({"op": "key", "key": "Tab", "revision": revision}));
            assert_eq!(response["status"], "pressed", "{response}");
        }
        panic!(
            "Tab never reached {name}: {}",
            response["table"]["controls"]
        );
    }

    /// Press Enter on the focused control, named as a guarded key names it.
    fn enter(&mut self, name: &str) -> Value {
        let revision = self.ask(json!({"op": "observe"}))["observation"]["revision"].clone();
        let pressed =
            self.ask(json!({"op": "key", "key": "Enter", "revision": revision, "focused": name}));
        assert_eq!(pressed["status"], "pressed", "{pressed}");
        let settled = self.ask(json!({"op": "settle"}));
        assert_eq!(settled["status"], "settled", "{settled}");
        settled
    }
}

#[test]
fn the_seat_reaches_the_menu_by_tab_and_refuses_to_open_external_links() {
    let mut seat = sit();
    seat.tab_to("Menu");
    let menu = seat.enter("Menu");
    assert_eq!(menu["table"]["modal"], "utilities menu", "{menu}");
    seat.tab_to("Credits");
    let credits = seat.enter("Credits");
    assert_eq!(credits["table"]["modal"], "inspection", "{credits}");
    seat.tab_to("Chalice drops — source");
    let refused = seat.enter("Chalice drops — source");
    let notice = refused["table"]["notice"].as_str().unwrap_or_default();
    assert!(
        notice.contains("agent seat")
            && notice.contains("https://game-icons.net/1x1/lorc/chalice-drops.html"),
        "The refusal names the address it did not open: {refused}"
    );
    assert_eq!(refused["table"]["modal"], "inspection");
    assert_eq!(refused["observation"]["revision"], 0);
    assert!(!seat.path.exists(), "Credits decide nothing");
}

/// Cards that are still face down in a deck now, proven by drawing them later: the
/// replacements a full mulligan would deal to the investigator deciding it.
fn undrawn_cards(session: &Session) -> Vec<(u16, String)> {
    let now = session.observe();
    let owner = now.decision_owner.expect("a mulligan owner").index();
    let visible: BTreeSet<u16> = now
        .investigators
        .iter()
        .flat_map(|investigator| investigator.hand.iter().map(|card| card.id))
        .collect();
    let mut future = session.clone();
    for card in &now.investigators[owner].hand {
        let revision = future.observe().revision;
        future
            .choose(revision, &format!("mulligan:{}", card.id))
            .unwrap();
    }
    let revision = future.observe().revision;
    future.choose(revision, "confirm-mulligan").unwrap();
    future.observe().investigators[owner]
        .hand
        .iter()
        .filter(|card| !visible.contains(&card.id))
        .map(|card| (card.id, card.title.clone()))
        .collect()
}

#[test]
fn nothing_hidden_appears_in_what_the_seat_reports() {
    let mut seat = sit();
    seat.open_menu(Investigator::Tony);
    seat.ask(json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}));
    let dealt = seat.ask(json!({"op": "settle"}));
    assert_eq!(dealt["status"], "settled", "{dealt}");

    let session = continuity::load_session(&seat.path).unwrap();
    assert_eq!(dealt["observation"], json!(session.observe()));
    let hidden = undrawn_cards(&session);
    assert_eq!(hidden.len(), 5, "a full mulligan draws five unseen cards");
    let public = dealt["observation"].to_string();
    let shown = dealt["table"].to_string();
    for (id, title) in &hidden {
        assert!(
            !shown.contains(&format!("\"card:{id}\"")),
            "the table names undrawn card {id}"
        );
        // A second copy of a visible title is not a secret.
        assert!(
            public.contains(title.as_str()) || !shown.contains(title.as_str()),
            "the table shows the undrawn {title}"
        );
    }
    assert_eq!(
        dealt
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["mode", "observation", "ok", "status", "steps", "table"])
    );
}

#[test]
fn pieces_sharing_a_source_keep_distinct_names_in_play() {
    let mut seat = sit_at_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
    let response = seat.ask(json!({"op": "observe"}));
    let names: Vec<&str> = response["table"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|component| component["name"].as_str().unwrap())
        .collect();
    let distinct: BTreeSet<&str> = names.iter().copied().collect();
    assert_eq!(names.len(), distinct.len(), "{names:?}");
    for expected in [
        "investigator:tony",
        "investigator:tony/standee",
        "location:01114",
        "location:01114/clues",
        "deck:tony",
        "encounter-deck",
        "chaos-bag",
        "act",
        "agenda",
    ] {
        assert!(distinct.contains(expected), "{expected} missing: {names:?}");
    }
    assert_eq!(
        component(&response, "investigator:tony")["label"],
        "Tony Morgan"
    );
}

#[test]
fn an_attached_card_has_its_own_marker_and_heading_names() {
    // This position starts with Obscuring Fog attached to the Cellar.
    let mut seat = sit_at_fixture(crate::gathering::HostileFixture::Search(
        crate::gathering::SearchFixture::TonyEvents,
    ));
    // Scenario frames both sideboards, so the heading is shown.
    seat.tab_to("Scenario");
    let response = seat.enter("Scenario");
    let names: Vec<&str> = response["table"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|component| component["name"].as_str().unwrap())
        .collect();
    let distinct: BTreeSet<&str> = names.iter().copied().collect();
    assert_eq!(names.len(), distinct.len(), "{names:?}");
    assert_eq!(
        component(&response, "location:01114/attachments")["label"],
        "Attachment Cellar"
    );
    assert_eq!(
        component(&response, "location:01114/attachment-sideboard")["label"],
        "Attached to CELLAR · 1"
    );
}

#[test]
fn the_protocol_refuses_what_it_cannot_do_and_ends_cleanly() {
    let mut seat = sit();
    for (request, expected) in [
        ("not json", "expected"),
        (r#"{"op":"teleport"}"#, "unknown variant"),
        (r#"{"op":"observe","extra":1}"#, "unknown field"),
        (r#"{"op":"activate","choice":"lead:tony"}"#, "revision"),
    ] {
        let (response, proceed) = seat.seat.respond(&mut seat.app, request);
        assert!(proceed);
        assert_eq!(response["status"], "refused", "{request}");
        assert!(
            response["error"].as_str().unwrap().contains(expected),
            "{request}: {}",
            response["error"]
        );
        assert_eq!(response["observation"]["revision"], 0);
    }
    let capture = seat.ask(json!({"op": "capture", "path": seat.path.with_extension("png")}));
    assert_eq!(capture["status"], "refused");
    assert!(!seat.path.with_extension("png").exists());

    let mut output = Vec::new();
    seat.seat
        .serve(
            &mut seat.app,
            &b"{\"op\":\"observe\"}\n\n{\"op\":\"quit\"}\n{\"op\":\"observe\"}\n"[..],
            &mut output,
        )
        .unwrap();
    let lines: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        lines
            .iter()
            .map(|line| line["status"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["ready", "observed", "bye"]
    );
    assert!(lines.iter().all(|line| line["mode"] == "seat"));
    assert_eq!(
        lines[0]["adapter"],
        Value::Null,
        "this host has no renderer"
    );
    assert!(!seat.path.exists(), "observing and quitting commit nothing");
}

/// One client connection to a held seat, asking one request at a time.
struct Connection {
    reader: std::io::BufReader<UnixStream>,
    writer: UnixStream,
}

impl Connection {
    fn open(socket: &Path) -> Self {
        let writer = UnixStream::connect(socket).unwrap();
        Self {
            reader: std::io::BufReader::new(writer.try_clone().unwrap()),
            writer,
        }
    }

    fn ask(&mut self, request: Value) -> Value {
        writeln!(self.writer, "{request}").unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("no response to {request}"))
    }
}

impl Sitting {
    /// Serve this table on a real socket while `client` talks to it from another
    /// thread. A client that fails before quitting gets a last connection that quits,
    /// so the session always ends.
    fn hold<T: Send>(&mut self, socket: &Path, client: impl FnOnce(&Path) -> T + Send) -> T {
        self.hold_idle(socket, crate::agent_seat::socket::IDLE, client)
    }

    /// [`Sitting::hold`], ending the session after `idle` without a request.
    fn hold_idle<T: Send>(
        &mut self,
        socket: &Path,
        idle: Duration,
        client: impl FnOnce(&Path) -> T + Send,
    ) -> T {
        let listener = crate::agent_seat::socket::Listener::claim(socket).unwrap();
        std::thread::scope(|scope| {
            let talking = scope.spawn(|| {
                let outcome =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client(socket)));
                if let Ok(mut last) = UnixStream::connect(socket) {
                    let _ = writeln!(last, r#"{{"op":"quit"}}"#);
                    let _ = std::io::Read::read_to_end(&mut last, &mut Vec::new());
                }
                outcome
            });
            let served = listener.serve(&mut self.seat, &mut self.app, idle);
            let outcome = talking.join().unwrap();
            served.unwrap();
            outcome.unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        })
    }
}

#[test]
fn a_held_session_keeps_a_dialog_opened_through_one_connection_until_quit_removes_its_socket() {
    // This position has Obscuring Fog attached to the Cellar; the Scenario view frames
    // the attachment sideboard's heading, a piece the no-GPU host's UI picking reaches.
    // Its only card is inspected at once.
    let mut seat = sit_at_fixture(crate::gathering::HostileFixture::Search(
        crate::gathering::SearchFixture::TonyEvents,
    ));
    seat.tab_to("Scenario");
    seat.enter("Scenario");
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let heading = "location:01114/attachment-sideboard";

    let (activated, opened, held, bye) = seat.hold(&socket, |socket| {
        let mut first = Connection::open(socket);
        let observed = first.ask(json!({"op": "observe"}));
        let revision = observed["observation"]["revision"].clone();
        let activated =
            first.ask(json!({"op": "activate", "revision": revision, "component": heading}));
        let opened = first.ask(json!({"op": "settle"}));
        drop(first);
        let mut second = Connection::open(socket);
        let held = second.ask(json!({"op": "observe"}));
        let bye = second.ask(json!({"op": "quit"}));
        (activated, opened, held, bye)
    });
    assert_eq!(activated["status"], "activated", "{activated}");
    assert_eq!(opened["status"], "settled", "{opened}");
    assert_eq!(
        opened["table"]["modal"], "inspection",
        "{}",
        opened["table"]
    );
    assert_eq!(held["status"], "observed");
    assert_eq!(
        held["table"], opened["table"],
        "the next connection finds the same table"
    );
    assert_eq!(held["observation"], opened["observation"]);
    assert_eq!(bye["status"], "bye");
    assert!(!socket.exists(), "quit removes the socket");
}

#[test]
fn a_held_session_keeps_an_open_menu_and_its_focus_between_connections() {
    // The no-GPU host has no mesh picking, so a synthetic click opens the menu; the
    // connections then move its focus by keyboard and find it where they left it.
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let (moved, held) = seat.hold(&socket, |socket| {
        let moved = Connection::open(socket).ask(json!({"op": "key", "key": "Tab", "revision": 0}));
        let mut second = Connection::open(socket);
        let held = second.ask(json!({"op": "observe"}));
        second.ask(json!({"op": "quit"}));
        (moved, held)
    });
    assert_eq!(moved["status"], "pressed", "{moved}");
    assert_eq!(
        focused(&moved).as_deref(),
        Some("Inspect"),
        "{}",
        moved["table"]["menu"]
    );
    assert_eq!(held["table"]["menu"], moved["table"]["menu"]);
    assert_eq!(focused(&held).as_deref(), Some("Inspect"));
    assert!(!seat.path.exists(), "moving focus commits nothing");
}

#[test]
fn the_tables_own_exit_ends_a_held_session_and_removes_its_socket() {
    let mut seat = sit();
    seat.tab_to("Menu");
    seat.enter("Menu");
    seat.tab_to("Save and return to scenario selection");
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let (exited, after) = seat.hold(&socket, |socket| {
        let mut connection = Connection::open(socket);
        let exited = connection.ask(json!({
            "op": "key", "key": "Enter", "revision": 0,
            "focused": "Save and return to scenario selection",
        }));
        // The session is over: the seat closes the connection.
        let mut after = String::new();
        connection.reader.read_line(&mut after).unwrap();
        (exited, after)
    });
    assert_eq!(exited["status"], "exited", "{exited}");
    assert_eq!(exited["ok"], true);
    assert_eq!(after, "");
    assert!(!socket.exists(), "the table's exit removes the socket");
    assert_eq!(
        saved(&seat.path)["revision"],
        0,
        "returning saves the attempt"
    );
}

#[test]
fn the_held_socket_is_accessible_only_to_its_owner_and_removed_with_its_listener() {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let listener = crate::agent_seat::socket::Listener::claim(&socket).unwrap();
    let file = std::fs::symlink_metadata(&socket).unwrap();
    assert!(file.file_type().is_socket());
    assert_eq!(file.permissions().mode() & 0o777, 0o600);
    // Nothing else is left beside it.
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    drop(listener);
    assert!(
        !socket.exists(),
        "a seat that never served leaves no socket"
    );
}

#[test]
fn a_path_that_is_not_a_socket_is_refused_and_left_unchanged() {
    use std::os::unix::fs::{FileTypeExt, symlink};
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("notes.sock");
    std::fs::write(&file, "an agent's notes").unwrap();
    let folder = directory.path().join("folder.sock");
    std::fs::create_dir(&folder).unwrap();
    // A link to a socket nobody answers is still not a socket at the path.
    let left = directory.path().join("left.sock");
    drop(std::os::unix::net::UnixListener::bind(&left).unwrap());
    let link = directory.path().join("link.sock");
    symlink(&left, &link).unwrap();
    for path in [&file, &folder, &link] {
        let error = crate::agent_seat::socket::Listener::claim(path)
            .err()
            .unwrap_or_else(|| panic!("{} was claimed", path.display()));
        assert!(error.contains(path.to_str().unwrap()), "{error}");
        assert!(error.contains("not a socket"), "{error}");
        assert!(error.contains("left unchanged"), "{error}");
    }
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "an agent's notes");
    assert!(folder.is_dir());
    assert_eq!(std::fs::read_link(&link).unwrap(), left);
    assert!(
        std::fs::symlink_metadata(&left)
            .unwrap()
            .file_type()
            .is_socket()
    );
    assert_eq!(
        std::fs::read_dir(directory.path()).unwrap().count(),
        4,
        "nothing is left beside them"
    );
}

#[test]
fn a_socket_left_by_a_crashed_seat_is_replaced() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    // A seat that crashed left its socket file, and nothing answers there.
    drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
    assert!(UnixStream::connect(&socket).is_err());
    let mut seat = sit();
    let (observed, bye) = seat.hold(&socket, |socket| {
        let mut connection = Connection::open(socket);
        (
            connection.ask(json!({"op": "observe"})),
            connection.ask(json!({"op": "quit"})),
        )
    });
    assert_eq!(observed["status"], "observed", "{observed}");
    assert_eq!(bye["status"], "bye", "{bye}");
    assert!(!socket.exists(), "quit removes the replacement");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn seats_starting_together_on_one_leftover_socket_leave_exactly_one_listening() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
    let starting = std::sync::Barrier::new(8);
    let claims: Vec<_> = std::thread::scope(|scope| {
        let claimants: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    starting.wait();
                    crate::agent_seat::socket::Listener::claim(&socket)
                })
            })
            .collect();
        claimants
            .into_iter()
            .map(|claimant| claimant.join().unwrap())
            .collect()
    });
    let (listening, refused): (Vec<_>, Vec<_>) = claims.into_iter().partition(Result::is_ok);
    let refused: Vec<String> = refused.into_iter().filter_map(Result::err).collect();
    assert_eq!(listening.len(), 1, "{refused:?}");
    for error in &refused {
        assert!(error.contains("already answers"), "{error}");
    }
    assert!(UnixStream::connect(&socket).is_ok(), "the one seat answers");
    drop(listening);
    assert!(!socket.exists());
}

#[test]
fn a_second_seat_on_a_live_seats_path_is_refused_and_the_live_seat_is_untouched() {
    use std::os::unix::fs::MetadataExt;
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let file = |socket: &Path| {
        let file = std::fs::symlink_metadata(socket).unwrap();
        (file.dev(), file.ino())
    };
    let (moved, before, refused, after, held) = seat.hold(&socket, |socket| {
        let moved = Connection::open(socket).ask(json!({"op": "key", "key": "Tab", "revision": 0}));
        let before = file(socket);
        let refused = crate::agent_seat::socket::Listener::claim(socket).err();
        let after = file(socket);
        let mut next = Connection::open(socket);
        let held = next.ask(json!({"op": "observe"}));
        next.ask(json!({"op": "quit"}));
        (moved, before, refused, after, held)
    });
    let error = refused.expect("the second seat was refused");
    assert!(error.contains(socket.to_str().unwrap()), "{error}");
    assert!(error.contains("already answers"), "{error}");
    assert_eq!(before, after, "the live seat's socket stays in place");
    assert_eq!(moved["status"], "pressed", "{moved}");
    assert_eq!(held["status"], "observed", "{held}");
    assert_eq!(held["table"], moved["table"], "the live table is unchanged");
    assert_eq!(held["observation"], moved["observation"]);
    assert!(!socket.exists(), "quit removes the live seat's socket");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    assert!(!seat.path.exists(), "nothing was committed");
}

#[test]
fn a_refusal_skips_only_the_rest_of_its_own_connection_on_a_held_socket() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let choose = json!({"op": "activate", "revision": 0, "choice": "lead:patrice"});
    let (first, second) = seat.hold(&socket, |socket| {
        let mut first = Connection::open(socket);
        // Focus is on lead:patrice, so this Enter is refused.
        let refused =
            first.ask(json!({"op": "key", "key": "Enter", "revision": 0, "focused": LEAD_TONY}));
        let skipped = first.ask(choose.clone());
        drop(first);
        let mut second = Connection::open(socket);
        let activated = second.ask(choose.clone());
        let settled = second.ask(json!({"op": "settle"}));
        second.ask(json!({"op": "quit"}));
        ([refused, skipped], [activated, settled])
    });
    assert_eq!(statuses(&first), ["refused", "skipped"], "{first:?}");
    assert_eq!(first[1]["observation"]["revision"], 0);
    assert_eq!(statuses(&second), ["activated", "settled"], "{second:?}");
    assert_eq!(second[1]["observation"]["lead"], "patrice");
    assert_eq!(second[1]["observation"], saved(&seat.path));
}

#[test]
fn a_held_session_ends_as_quit_does_after_its_idle_period_without_a_request() {
    let idle = Duration::from_millis(1_500);
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let (answered, quiet) = seat.hold_idle(&socket, idle, |socket| {
        // Requests closer together than the idle period keep the session, across
        // connections, for longer than one period.
        let gap = idle * 3 / 5;
        let mut first = Connection::open(socket);
        let observed = first.ask(json!({"op": "observe"}));
        std::thread::sleep(gap);
        let activated =
            first.ask(json!({"op": "activate", "revision": 0, "choice": "lead:patrice"}));
        drop(first);
        std::thread::sleep(gap);
        let settled = Connection::open(socket).ask(json!({"op": "settle"}));
        let last = std::time::Instant::now();
        while socket.exists() {
            assert!(last.elapsed() < idle * 10, "the idle session did not end");
            std::thread::sleep(Duration::from_millis(10));
        }
        ([observed, activated, settled], last.elapsed())
    });
    assert_eq!(
        statuses(&answered),
        ["observed", "activated", "settled"],
        "{answered:?}"
    );
    assert!(
        quiet >= idle * 2 / 3,
        "the period counts from the last request: {quiet:?}"
    );
    assert!(!socket.exists(), "the idle end removes the socket");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    assert_eq!(answered[2]["observation"]["lead"], "patrice");
    assert_eq!(
        saved(&seat.path),
        answered[2]["observation"],
        "the attempt is saved and resumes where the session ended"
    );
}

#[test]
fn an_open_connection_without_requests_does_not_keep_a_held_session() {
    let idle = Duration::from_millis(500);
    let mut seat = sit();
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let (end, waited) = seat.hold_idle(&socket, idle, |socket| {
        let opened = std::time::Instant::now();
        let mut silent = Connection::open(socket);
        silent.writer.set_read_timeout(Some(idle * 10)).unwrap();
        let mut end = String::new();
        silent
            .reader
            .read_line(&mut end)
            .expect("the idle session did not end");
        (end, opened.elapsed())
    });
    assert_eq!(end, "", "the seat closes the connection without a response");
    assert!(waited >= idle * 2 / 3, "{waited:?}");
    assert!(!socket.exists(), "the idle end removes the socket");
}

fn parse(directory: &std::path::Path, arguments: &[&str]) -> Result<Option<Options>, String> {
    Options::parse(
        AssetConfig::load_at(directory.join("gathering.toml"), directory.join("assets")),
        arguments.iter().map(|argument| (*argument).to_owned()),
    )
}

#[test]
fn a_held_session_waits_thirty_minutes_for_a_request_unless_started_with_another_period() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("seat.sock");
    let save = directory.path().join("attempt.json");
    let held = [
        "--experiment",
        "gathering",
        "--agent-seat",
        "--save",
        save.to_str().unwrap(),
        "--socket",
        socket.to_str().unwrap(),
    ];
    let period = |extra: &[&str]| {
        let arguments: Vec<&str> = held.iter().chain(extra).copied().collect();
        idle(&parse(directory.path(), &arguments).unwrap().unwrap())
    };
    assert_eq!(period(&[]), Duration::from_secs(30 * 60));
    assert_eq!(
        period(&["--idle-minutes", "90"]),
        Duration::from_secs(90 * 60)
    );
    assert_eq!(period(&["--idle-minutes", "1"]), Duration::from_secs(60));
    assert_eq!(
        period(&["--idle-minutes", "1440"]),
        Duration::from_secs(24 * 60 * 60)
    );
}

fn tell(seat: &mut Seat<Gathering>, app: &mut App, request: Value) -> Value {
    seat.respond(app, &request.to_string()).0
}

fn not_blank(path: &std::path::Path) -> bool {
    let picture = image::open(path).unwrap().to_rgb8();
    let first = *picture.get_pixel(0, 0);
    picture.pixels().filter(|pixel| **pixel != first).count() > 10_000
}

/// The real table, real artwork and a real renderer, with no window. Run on the
/// owner's machine after preparing the collection; CI has neither artwork nor a GPU.
#[test]
#[ignore = "requires ARKHAM_GATHERING_TEST_ASSETS pointing at a complete prepared collection and a graphics adapter"]
fn the_real_table_is_admitted_and_played_without_a_window() {
    let root = std::env::var("ARKHAM_GATHERING_TEST_ASSETS")
        .expect("set ARKHAM_GATHERING_TEST_ASSETS to the prepared collection");
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let options = parse(
        directory.path(),
        &[
            "--experiment",
            "gathering",
            "--agent-seat",
            "--save",
            save.to_str().unwrap(),
            "--art-root",
            &root,
        ],
    )
    .unwrap()
    .unwrap();
    let Seated {
        mut app, mut seat, ..
    } = open(&options).unwrap();

    // Admission: a fresh attempt is saved only after the real table rendered.
    let mut greeting = Vec::new();
    seat.serve(&mut app, &b""[..], &mut greeting).unwrap();
    let ready: Value = serde_json::from_slice(&greeting).unwrap();
    assert_eq!(ready["status"], "ready");
    assert!(ready["adapter"]["name"].is_string(), "{}", ready["adapter"]);
    eprintln!("Seat adapter: {}", ready["adapter"]);
    assert_eq!(ready["table"]["settled"], true, "{}", ready["table"]);
    assert_eq!(ready["table"]["modal"], Value::Null);
    assert_eq!(ready["observation"], saved(&save));

    let before = directory.path().join("before.png");
    let captured = tell(
        &mut seat,
        &mut app,
        json!({"op": "capture", "path": before}),
    );
    assert_eq!(captured["status"], "captured", "{captured}");
    assert!(not_blank(&before));

    // The Study's clue counter rests on the card's middle. The seat presses beside it
    // and says what it found on the middle instead of hiding it behind the success.
    let study = tell(
        &mut seat,
        &mut app,
        json!({"op": "activate", "revision": 0, "component": "location:01111"}),
    );
    assert_eq!(study["status"], "activated", "{study}");
    assert_eq!(
        study["covered"]["hit"][0]["component"],
        "location:01111/clues"
    );
    assert_ne!(study["covered"]["position"], study["position"]);
    assert_eq!(study["table"]["modal"], "inspection");
    let closed = tell(
        &mut seat,
        &mut app,
        json!({"op": "key", "key": "Escape", "revision": 0}),
    );
    assert_eq!(closed["status"], "pressed", "{closed}");
    assert_eq!(closed["table"]["modal"], Value::Null);
    assert_eq!(closed["observation"]["revision"], 0);

    // A card is activated by pointer through mesh picking, then its menu choice.
    let tony = component(&ready, "investigator:tony").clone();
    assert_eq!(tony["available"], true);
    let selected = tell(
        &mut seat,
        &mut app,
        json!({"op": "activate", "revision": 0, "component": "investigator:tony"}),
    );
    assert_eq!(selected["status"], "activated", "{selected}");
    assert_eq!(selected["table"]["menu"]["component"], "investigator:tony");
    assert!(rows(&selected).iter().any(|row| row["choice"] == LEAD_TONY));

    // The planted defect: a panel over the whole table, above the open menu, hides a
    // component from the pointer.
    let cover = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::srgb(0.4, 0.1, 0.1)),
            GlobalZIndex(60),
        ))
        .with_child(Text::new("Planted panel"))
        .id();
    app.update();
    let covered = tell(
        &mut seat,
        &mut app,
        json!({"op": "activate", "revision": 0, "component": "investigator:patrice"}),
    );
    assert_eq!(covered["status"], "missed", "{covered}");
    assert_eq!(covered["pressed"], false);
    assert_eq!(
        covered["hit"][0],
        json!({"surface": "overlay", "text": "Planted panel"})
    );
    assert_eq!(covered["table"]["menu"]["component"], "investigator:tony");
    app.world_mut().despawn(cover);
    app.update();

    let chosen = tell(
        &mut seat,
        &mut app,
        json!({"op": "activate", "revision": 0, "choice": LEAD_TONY}),
    );
    assert_eq!(chosen["status"], "activated", "{chosen}");
    let settled = tell(&mut seat, &mut app, json!({"op": "settle"}));
    assert_eq!(settled["status"], "settled", "{settled}");
    assert_eq!(settled["observation"]["revision"], 1);
    assert_eq!(settled["observation"]["lead"], "tony");
    assert_eq!(settled["observation"], saved(&save));

    let after = directory.path().join("after.png");
    let captured = tell(&mut seat, &mut app, json!({"op": "capture", "path": after}));
    assert_eq!(captured["status"], "captured", "{captured}");
    assert!(not_blank(&after));
    assert_ne!(
        std::fs::read(&before).unwrap(),
        std::fs::read(&after).unwrap(),
        "the capture must show the dealt opening hands"
    );
    if let Some(keep) = std::env::var_os("ARKHAM_SEAT_CAPTURE_DIR") {
        std::fs::create_dir_all(&keep).unwrap();
        for picture in [&before, &after] {
            std::fs::copy(
                picture,
                PathBuf::from(&keep).join(picture.file_name().unwrap()),
            )
            .unwrap();
        }
    }

    let (bye, proceed) = seat.respond(&mut app, r#"{"op":"quit"}"#);
    assert_eq!(bye["status"], "bye");
    assert!(!proceed);
    drop(app);
    // The attempt stays resumable through the ordinary driver's reader.
    assert_eq!(saved(&save)["revision"], 1);

    // A later session resumes the same attempt where this one left it.
    let Seated {
        mut app, mut seat, ..
    } = open(&options).unwrap();
    let resumed = tell(&mut seat, &mut app, json!({"op": "observe"}));
    assert_eq!(resumed["observation"], saved(&save));
    assert_eq!(resumed["observation"]["revision"], 1);
    assert_eq!(resumed["table"]["settled"], true, "{}", resumed["table"]);

    // Required artwork that becomes unavailable ends the session with its reason,
    // as native play stops, and leaves the attempt as it was.
    let before = std::fs::read(&save).unwrap();
    let study = app
        .world()
        .resource::<art::Art>()
        .card("01111", false)
        .unwrap();
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .remove(study.id());
    let (ended, proceed) = seat.respond(&mut app, r#"{"op":"key","key":"Home","revision":1}"#);
    assert_eq!(ended["status"], "exited", "{ended}");
    assert_eq!(ended["ok"], false);
    assert!(
        ended["error"]
            .as_str()
            .unwrap()
            .contains("became unavailable"),
        "{}",
        ended["error"]
    );
    assert!(!proceed);
    assert_eq!(std::fs::read(&save).unwrap(), before);
}

fn control_choice(response: &Value, label: &str) -> Option<String> {
    response["table"]["controls"]
        .as_array()?
        .iter()
        .find(|control| control["label"] == label)
        .and_then(|control| control["choice"].as_str())
        .map(str::to_owned)
}

#[test]
fn the_seat_pays_the_group_clue_payment_by_choice_identity_and_keyboard() {
    let mut seat = sit_at_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
    let observed = seat.ask(json!({"op": "observe"}));
    let payment = &observed["observation"]["group_clue_payment"];
    assert_eq!(payment["required"], 9);
    assert_eq!(payment["forced"], json!([3, 3, 3]));
    let revision = observed["observation"]["revision"].as_u64().unwrap();
    // The Act card's menu names the listed payment, which opens the chooser.
    click_source(&mut seat.app, controls::Source::Act);
    let menu = seat.ask(json!({"op": "settle"}));
    let row = rows(&menu)
        .into_iter()
        .find(|row| row["choice"] == "act:pay:<contributions>")
        .unwrap_or_else(|| panic!("the Act menu names the payment: {}", menu["table"]["menu"]));
    assert_eq!(
        row["label"],
        "Spend 9 clues as a group: advance The Barrier…"
    );
    let opened = seat
        .ask(json!({"op": "activate", "revision": revision, "choice": "act:pay:<contributions>"}));
    assert_eq!(opened["status"], "activated", "{opened}");
    let opened = seat.ask(json!({"op": "settle"}));
    assert_eq!(
        opened["observation"], observed["observation"],
        "Opening commits nothing"
    );
    assert_eq!(opened["table"]["modal"], "group clue payment", "{opened}");
    assert_eq!(
        control_choice(&opened, "Do not advance").as_deref(),
        Some("round-end:decline-act")
    );
    assert_eq!(
        control_choice(&opened, "Spend 9 clues and advance").as_deref(),
        Some("act:pay:3:3:3")
    );
    let tony = "Tony Morgan · 3 clues held · Hallway · pays 3";
    assert_eq!(focused(&opened).as_deref(), Some(tony));
    // The arrow keys change the focused row; an incomplete total offers no payment.
    let lowered = seat.ask(json!({"op": "key", "key": "ArrowDown", "revision": revision}));
    assert_eq!(
        focused(&lowered).as_deref(),
        Some("Tony Morgan · 3 clues held · Hallway · pays 2")
    );
    assert_eq!(control_choice(&lowered, "Spend 9 clues and advance"), None);
    let restored = seat.ask(json!({"op": "key", "key": "ArrowUp", "revision": revision}));
    assert_eq!(focused(&restored).as_deref(), Some(tony));
    let revision = restored["observation"]["revision"].as_u64().unwrap();
    assert_eq!(
        revision,
        observed["observation"]["revision"].as_u64().unwrap()
    );
    let paid = seat.ask(json!({"op": "activate", "revision": revision, "choice": "act:pay:3:3:3"}));
    assert_eq!(
        paid["status"], "activated",
        "{}",
        restored["table"]["controls"]
    );
    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(
        settled["observation"]["play"]["act"], 3,
        "{}",
        settled["observation"]["prompt"]
    );
    assert_eq!(settled["observation"], saved(&seat.path));
}

#[test]
fn a_blind_enter_at_round_end_commits_nothing_and_a_named_chooser_row_pays() {
    let mut seat = sit_at_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
    let observed = seat.ask(json!({"op": "observe"}));
    let revision = observed["observation"]["revision"].clone();
    // #173: Enter meant for a reading pressed whatever had focus instead, here a
    // decision-bar choice.
    let decline = focused(&observed).expect("a decision-bar control has focus");
    assert!(
        observed["table"]["controls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|control| control["choice"] == decline.as_str()),
        "Enter would commit a choice: {}",
        observed["table"]["controls"]
    );
    let blind =
        seat.ask(json!({"op": "key", "key": "Enter", "revision": revision, "focused": "Continue"}));
    assert_eq!(blind["status"], "refused", "{blind}");
    assert!(
        blind["error"].as_str().unwrap().contains(&decline),
        "{}",
        blind["error"]
    );
    assert_eq!(blind["observation"], observed["observation"]);
    assert!(!seat.path.exists(), "the blind Enter committed nothing");

    click_source(&mut seat.app, controls::Source::Act);
    seat.ask(json!({"op": "settle"}));
    let opened = seat
        .ask(json!({"op": "activate", "revision": revision, "choice": "act:pay:<contributions>"}));
    assert_eq!(opened["status"], "activated", "{opened}");
    let opened = seat.ask(json!({"op": "settle"}));
    assert_eq!(opened["table"]["modal"], "group clue payment", "{opened}");
    // Enter on a focused investigator row spends the clues, so the row is named by
    // its label, which carries the amounts it would spend.
    let row = focused(&opened).expect("an investigator row has focus");
    assert_eq!(row, "Tony Morgan · 3 clues held · Hallway · pays 3");
    let elsewhere = seat.ask(
        json!({"op": "key", "key": "Enter", "revision": revision, "focused": "act:pay:3:3:3"}),
    );
    assert_eq!(elsewhere["status"], "refused", "{elsewhere}");
    assert!(elsewhere["error"].as_str().unwrap().contains(&row));
    assert_eq!(elsewhere["table"]["modal"], "group clue payment");

    let paid = seat.ask(json!({"op": "key", "key": "Enter", "revision": revision, "focused": row}));
    assert_eq!(paid["status"], "pressed", "{paid}");
    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(
        settled["observation"]["play"]["act"], 3,
        "{}",
        settled["observation"]["prompt"]
    );
    assert_eq!(settled["observation"], saved(&seat.path));
}

/// Review fix A2 (#202: "The agent seat, the headless driver and their protocols
/// are unchanged"): without partners, `shows_latest` compares the table's view
/// with the driver's observation exactly. A map beat shows the table with no
/// controls; when everything else it shows is already the latest, here the next
/// turn's group choice after Patrice's move from the Cellar into the Hallway, the
/// table still does not show the latest observation until the beat has played.
///
/// A live move presents through the rules session's own moments, whose captions
/// differ from the observation, so this test gives the production transition the
/// position Patrice left as its starting view.
#[test]
fn without_partners_a_beat_with_cleared_choices_does_not_show_the_latest() {
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringArrival);
    let violin = session
        .observe()
        .choices
        .into_iter()
        .map(|choice| choice.id)
        .find(|id| id.starts_with("violin:"))
        .expect("Patrice may use her Violin");
    // Her third action, back into the Hallway, ends her turn.
    for id in [
        violin.as_str(),
        "action:ordinary:move:01114",
        "action:ordinary:resource",
        "action:ordinary:move:01112",
    ] {
        let revision = session.revision();
        session.choose(revision, id).unwrap();
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("move.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session, path), false);
    super::super::tests::finish_presentation(&mut app);
    let latest = app.world().resource::<Table>().session.observe();
    let next: Vec<&str> = latest
        .choices
        .iter()
        .map(|choice| choice.id.as_str())
        .collect();
    assert!(next.contains(&"turn:tony"), "{next:?}");
    assert_eq!(
        Gathering.observe(app.world_mut()).table["shows_latest"],
        true
    );

    let mut before = latest.clone();
    before.play.as_mut().unwrap().players[crate::gathering::Investigator::Patrice.index()]
        .location = "01114".into();
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        let table = &mut *table;
        table.presentation.committed(
            &table.session,
            &before,
            "action:ordinary:move:01112",
            Vec::new(),
            false,
        );
        table.dirty = true;
    }
    app.update();
    let mut settling = 0;
    for _ in 0..400 {
        let table = app.world().resource::<Table>();
        if !table.presentation.map().active() {
            break;
        }
        let shown = table.view();
        assert!(shown.choices.is_empty(), "A map beat has no controls");
        let mut without_choices = latest.clone();
        without_choices.choices.clear();
        if shown == without_choices {
            settling += 1;
        }
        assert_eq!(
            Gathering.observe(app.world_mut()).table["shows_latest"],
            false,
            "A map beat is not the latest observation"
        );
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(Duration::from_millis(50));
        app.update();
    }
    assert!(
        settling > 0,
        "The move settled in a beat with cleared choices"
    );
    super::super::tests::finish_presentation(&mut app);
    assert_eq!(
        Gathering.observe(app.world_mut()).table["shows_latest"],
        true
    );
}

/// #218: `seat-wait` (`tools/src/seat.rs`) learns which partner-played
/// investigators the table waits on only from the seat's `controls` labels: one
/// `Take over NAME` wherever the table offers Take over, and the portrait's
/// `Focus NAME · partner · CUE`. This pins those exact shapes through the seat's
/// own observation at a table hosting a partner, while that partner is awaited,
/// unclaimed and then thinking, and once none is. A renamed label would make
/// `seat-wait` return at once, as if the seat had to decide.
#[test]
fn a_hosted_partner_is_shown_to_seat_wait_by_take_over_and_its_cue() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("partners.sock");
    let mut seat = sit();
    seat.app
        .insert_resource(partners::PartnerSocket(socket.clone()));
    // Jacqueline leads, so her mulligan comes first. She is handed to partners
    // through the menu, as an agent at the seat does it.
    seat.open_menu(Investigator::Jacqueline);
    let lead = seat.ask(json!({"op": "activate", "revision": 0, "choice": "lead:jacqueline"}));
    assert_eq!(lead["status"], "activated", "{lead}");
    let dealt = seat.ask(json!({"op": "settle"}));
    assert_eq!(dealt["status"], "settled", "{dealt}");
    for name in [
        "Menu",
        "Agent partners",
        "Jacqueline Fine: you play",
        "Enable agent partners",
    ] {
        seat.tab_to(name);
        seat.enter(name);
    }
    let closed = seat.ask(json!({"op": "key", "revision": 1, "key": "Escape"}));
    assert_eq!(closed["status"], "pressed", "{closed}");

    // What `seat-wait` reads: every Take over, and the partner's portrait.
    let read = |response: &Value| -> Vec<String> {
        let mut labels: Vec<String> = response["table"]["controls"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|control| control["label"].as_str())
            .filter(|label| {
                label.starts_with("Take over ") || label.starts_with("Focus Jacqueline Fine")
            })
            .map(str::to_owned)
            .collect();
        labels.sort();
        labels
    };
    let unclaimed = seat.ask(json!({"op": "settle"}));
    assert_eq!(unclaimed["status"], "settled", "{unclaimed}");
    assert_eq!(unclaimed["table"]["modal"], Value::Null);
    assert_eq!(
        read(&unclaimed),
        [
            "Focus Jacqueline Fine · partner · awaiting partner",
            "Take over Jacqueline Fine",
            "Take over Jacqueline Fine",
        ]
    );

    let joined = hosted(&mut seat, &socket, json!({"op": "join"}));
    assert_eq!(joined["status"], "joined", "{joined}");
    let thinking = seat.ask(json!({"op": "settle"}));
    assert_eq!(
        read(&thinking),
        [
            "Focus Jacqueline Fine · partner · thinking",
            "Take over Jacqueline Fine",
            "Take over Jacqueline Fine",
        ]
    );

    let choose = json!({
        "op": "choose", "partner": joined["partner"], "revision": 1, "id": "confirm-mulligan",
    });
    let chosen = hosted(&mut seat, &socket, choose);
    assert_eq!(chosen["status"], "chosen", "{chosen}");
    let seats_own = seat.ask(json!({"op": "settle"}));
    assert_eq!(seats_own["status"], "settled", "{seats_own}");
    assert_eq!(seats_own["observation"]["decision_owner"], "tony");
    assert_eq!(read(&seats_own), ["Focus Jacqueline Fine · partner"]);
}

/// One partner request on the hosted socket, answered while the seat settles: the
/// seat runs frames only inside its own requests.
fn hosted(seat: &mut Sitting, socket: &Path, request: Value) -> Value {
    let socket = socket.to_owned();
    let partner = std::thread::spawn(move || {
        let stream = UnixStream::connect(&socket).expect("The partner socket answers");
        let mut writer = &stream;
        writeln!(writer, "{request}").unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut answer = String::new();
        std::io::BufReader::new(&stream)
            .read_line(&mut answer)
            .unwrap();
        serde_json::from_str::<Value>(&answer).unwrap()
    });
    for _ in 0..1_000 {
        if partner.is_finished() {
            return partner.join().unwrap();
        }
        seat.ask(json!({"op": "settle"}));
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("The table never answered the partner");
}
