//! The seat's protocol is the only seam: these tests send requests and assert on the
//! responses, the saved scenario attempt and written files. The no-GPU table host has
//! no mesh picking, so pointer activation of 3D components is covered only by the
//! ignored real-artwork run at the end of this file.

use super::super::tests::{click_source, host_with_pointer_input, host_with_table};
use super::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;

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
    let mut sitting = Sitting {
        seat: Seat::new(Gathering, camera, None),
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
    let key = seat.ask(json!({"op": "key", "key": "Enter"}));
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
        .focus(enemy_board::focus(Area::Investigator(0)));
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

#[test]
fn a_covered_control_reports_what_covers_it_and_presses_nothing() {
    let mut seat = sit();
    seat.open_menu(Investigator::Tony);
    let menu = seat.ask(json!({"op": "observe"}));
    let lead = rows(&menu)
        .into_iter()
        .find(|row| row["choice"] == LEAD_TONY)
        .unwrap();
    let at = Vec2::new(
        lead["position"][0].as_f64().unwrap() as f32,
        lead["position"][1].as_f64().unwrap() as f32,
    );
    // The planted defect: another control drawn over the lead choice.
    seat.app.world_mut().spawn((
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
    seat.app.update();

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

#[test]
fn keys_travel_the_keyboard_path_and_unknown_keys_are_refused() {
    let mut seat = sit();
    seat.open_menu(Investigator::Patrice);
    let menu = seat.ask(json!({"op": "observe"}));
    let focused: Vec<_> = rows(&menu)
        .into_iter()
        .filter(|row| row["focused"] == true)
        .collect();
    assert_eq!(focused.len(), 1, "{menu}");
    assert_eq!(focused[0]["choice"], "lead:patrice");

    let unknown = seat.ask(json!({"op": "key", "key": "F13"}));
    assert_eq!(unknown["status"], "refused");
    assert!(unknown["error"].as_str().unwrap().contains("Unknown key"));

    let moved = seat.ask(json!({"op": "key", "key": "Tab"}));
    assert_eq!(moved["status"], "pressed", "{moved}");
    let focused: Vec<_> = rows(&moved)
        .into_iter()
        .filter(|row| row["focused"] == true)
        .collect();
    assert_eq!(focused.len(), 1, "{moved}");
    assert_eq!(focused[0]["label"], "Inspect");
    assert_eq!(focused[0]["choice"], Value::Null);

    let dismissed = seat.ask(json!({"op": "key", "key": "Escape"}));
    assert_eq!(dismissed["status"], "pressed", "{dismissed}");
    assert_eq!(dismissed["table"]["menu"], Value::Null);
    assert!(!seat.path.exists());

    seat.open_menu(Investigator::Patrice);
    let chosen = seat.ask(json!({"op": "key", "key": "Enter"}));
    assert_eq!(chosen["status"], "pressed", "{chosen}");
    let settled = seat.ask(json!({"op": "settle"}));
    assert_eq!(settled["observation"]["revision"], 1);
    assert_eq!(settled["observation"]["lead"], "patrice");
    assert_eq!(settled["observation"], saved(&seat.path));
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

fn parse(directory: &std::path::Path, arguments: &[&str]) -> Result<Option<Options>, String> {
    Options::parse(
        AssetConfig::load_at(directory.join("gathering.toml"), directory.join("assets")),
        arguments.iter().map(|argument| (*argument).to_owned()),
    )
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
    let closed = tell(&mut seat, &mut app, json!({"op": "key", "key": "Escape"}));
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
    let (ended, proceed) = seat.respond(&mut app, r#"{"op":"key","key":"Home"}"#);
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
