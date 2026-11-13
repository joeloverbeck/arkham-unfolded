//! Seam: agent partners at the table host that runs without a graphics card (#205).
//! The human acts through the production controls (the menu's buttons, Take over);
//! a partner acts as one would, over the real socket. Expectations come from ticket
//! #205 and the presentation contract's "Agent partners and table talk".

use super::super::tests::{button_with_label, finish_presentation, host, rendered_text};
use super::*;
use crate::gathering::Investigator::{Jacqueline, Patrice};
use std::{fs, os::unix::fs::FileTypeExt, os::unix::fs::PermissionsExt};

struct Partnered {
    app: App,
    socket: PathBuf,
    save: PathBuf,
    wakes: Arc<AtomicUsize>,
    _directory: tempfile::TempDir,
}

/// A table whose lead is Jacqueline (mulligans then go Jacqueline, Tony, Patrice),
/// with `partners` handed to partners through the menu, and partners enabled.
fn partnered(partners: &[Investigator]) -> Partnered {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("partners.session.json");
    let socket = directory.path().join("partners.sock");
    let mut app = host(save.clone());
    let wakes = Arc::new(AtomicUsize::new(0));
    let counted = wakes.clone();
    app.insert_resource(PartnerSocket(socket.clone()))
        .insert_resource(PartnerWaker(Arc::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
        })));
    choose(&mut app, "lead:jacqueline");
    finish_presentation(&mut app);
    press(&mut app, "Menu");
    press(&mut app, "Agent partners");
    for who in partners {
        press(&mut app, &format!("{}: you play", who.name()));
    }
    press(&mut app, "Enable agent partners");
    Partnered {
        app,
        socket,
        save,
        wakes,
        _directory: directory,
    }
}

fn press(app: &mut App, label: &str) {
    let entity = button_with_label(app, label).unwrap_or_else(|| {
        panic!("No button {label}:\n{}", rendered_text(app));
    });
    app.world_mut().trigger(Activate { entity });
    app.update();
}

/// The human's choice through the decision display's commit action.
fn choose(app: &mut App, id: &str) {
    let table = app.world().resource::<Table>();
    let action = Action::Choose(table.epoch, table.view().revision, id.into());
    app.world_mut().resource_mut::<Actions>().0.push(action);
    app.update();
}

/// The choices the table's controls offer the human.
fn offered(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    world
        .query::<&ChoiceControl>()
        .iter(world)
        .map(|control| control.0.clone())
        .collect()
}

/// Run frames, advancing the presentation clock, until `done`.
fn pump(app: &mut App, done: impl Fn() -> bool) {
    for _ in 0..1_000 {
        if done() {
            return;
        }
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(Duration::from_millis(50));
        app.update();
        thread::sleep(Duration::from_millis(2));
    }
    panic!("The table never answered");
}

/// A partner connection sending `requests`, then reading every answer.
fn connect(socket: &Path, requests: &[Value]) -> JoinHandle<Vec<Value>> {
    let socket = socket.to_owned();
    let lines: Vec<String> = requests.iter().map(Value::to_string).collect();
    thread::spawn(move || {
        let stream = UnixStream::connect(&socket).expect("The partner socket answers");
        let mut writer = &stream;
        for line in &lines {
            writeln!(writer, "{line}").unwrap();
        }
        stream.shutdown(Shutdown::Write).unwrap();
        BufReader::new(&stream)
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect()
    })
}

fn exchange(app: &mut App, socket: &Path, requests: &[Value]) -> Vec<Value> {
    let client = connect(socket, requests);
    pump(app, || client.is_finished());
    client.join().unwrap()
}

fn join(partnered: &mut Partnered) -> String {
    let joined = exchange(
        &mut partnered.app,
        &partnered.socket,
        &[json!({"op": "join"})],
    );
    assert_eq!(joined[0]["status"], "joined", "{}", joined[0]);
    joined[0]["partner"].as_str().unwrap().to_owned()
}

/// Ticket #205, The socket: it exists only while partners are enabled, only its
/// owner can reach it, several connections are served at once while a wait is
/// held, and each request wakes the frame loop.
#[test]
fn the_socket_lives_only_while_partners_are_enabled_and_serves_several_partners_at_once() {
    let mut partnered = partnered(&[Patrice, Jacqueline]);
    let socket = partnered.socket.clone();
    let file = fs::symlink_metadata(&socket).expect("Enabling partners opens the socket");
    assert!(file.file_type().is_socket());
    assert_eq!(file.permissions().mode() & 0o777, 0o600);
    let shown = rendered_text(&mut partnered.app);
    assert!(shown.contains("scripts/tools partner"), "{shown}");
    for who in Investigator::ALL {
        assert!(
            !shown.contains(&format!("partner join {}", who.id())),
            "The line names no investigator"
        );
    }

    let first = exchange(
        &mut partnered.app,
        &socket,
        &[json!({"op": "join", "investigators": ["patrice"]})],
    );
    let patrices = first[0]["partner"].as_str().unwrap().to_owned();
    let waiting = connect(
        &socket,
        &[json!({"op": "wait", "partner": patrices, "timeout_seconds": 60})],
    );
    let wakes = partnered.wakes.load(Ordering::SeqCst);
    // A second partner is served while the first one's wait is held.
    let second = exchange(&mut partnered.app, &socket, &[json!({"op": "join"})]);
    assert_eq!(second[0]["status"], "joined", "{}", second[0]);
    assert_eq!(second[0]["investigators"], json!(["jacqueline"]));
    assert!(!waiting.is_finished(), "Nothing concerns Patrice yet");
    assert!(
        partnered.wakes.load(Ordering::SeqCst) > wakes,
        "A request wakes the frame loop"
    );

    press(&mut partnered.app, "Turn agent partners off");
    pump(&mut partnered.app, || waiting.is_finished());
    let ended = waiting.join().unwrap();
    assert_eq!(ended[0]["status"], "closed", "{}", ended[0]);
    assert!(
        fs::symlink_metadata(&socket).is_err(),
        "Turning partners off removes the socket"
    );
    assert!(
        partnered
            .app
            .world()
            .resource::<Table>()
            .presentation
            .control()
            .solo()
    );
}

/// Ticket #205: the human's controls offer only options of investigators the
/// human controls; while a partner decides, its portrait and the decision display
/// say so by name and offer Take over; a decision with no partner holding it waits,
/// marked.
#[test]
fn the_humans_controls_hold_only_the_humans_options_and_a_cue_names_the_deciding_partner() {
    let mut partnered = partnered(&[Patrice]);
    let app = &mut partnered.app;
    choose(app, "confirm-mulligan"); // Jacqueline
    finish_presentation(app);
    choose(app, "confirm-mulligan"); // Tony
    finish_presentation(app);
    // Patrice's mulligan, with no partner yet.
    assert!(
        offered(app)
            .iter()
            .all(|id| id != "confirm-mulligan" && !id.starts_with("mulligan:")),
        "{:?}",
        offered(app)
    );
    let shown = rendered_text(app);
    assert!(
        shown.contains("Patrice Hathaway: waiting for a partner to join"),
        "{shown}"
    );
    assert!(shown.contains("AWAITING PARTNER"), "{shown}");
    assert!(button_with_label(app, "Take over Patrice Hathaway").is_some());
    // The human's own click on her option is refused as well.
    let revision = app.world().resource::<Table>().view().revision;
    choose(app, "confirm-mulligan");
    assert_eq!(app.world().resource::<Table>().session.revision(), revision);

    let partner = join(&mut partnered);
    let app = &mut partnered.app;
    app.update();
    let shown = rendered_text(app);
    assert!(
        shown.contains("Patrice Hathaway: partner thinking"),
        "{shown}"
    );
    assert!(
        shown.contains("PARTNER") && shown.contains("THINKING"),
        "{shown}"
    );
    assert!(offered(app).iter().all(|id| id != "confirm-mulligan"));
    let _ = partner;
}

/// Ticket #205, Choosing: a partner's choice is applied only once the table has
/// settled, commits and autosaves through the human's commit path, leaves the
/// camera where it was, and the action history attributes it to Patrice.
#[test]
fn a_partners_choice_waits_for_the_table_to_settle_and_commits_like_a_click() {
    let mut partnered = partnered(&[Patrice]);
    let partner = join(&mut partnered);
    let app = &mut partnered.app;
    choose(app, "confirm-mulligan"); // Jacqueline
    finish_presentation(app);
    choose(app, "confirm-mulligan"); // Tony
    finish_presentation(app);
    // The human reads the action history: a dialog the table waits behind.
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::History);
    app.update();
    let revision = app.world().resource::<Table>().session.revision();
    let camera = app.world().resource::<Orbit>().pose();
    let chosen = connect(
        &partnered.socket,
        &[
            json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}),
        ],
    );
    for _ in 0..20 {
        app.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!chosen.is_finished());
    assert_eq!(
        app.world().resource::<Table>().session.revision(),
        revision,
        "Held while the human reads"
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    pump(app, || chosen.is_finished());
    let answer = chosen.join().unwrap();
    assert_eq!(answer[0]["status"], "chosen", "{}", answer[0]);
    assert_eq!(
        app.world().resource::<Table>().session.revision(),
        revision + 1
    );
    let saved = continuity::load_session(&partnered.save).unwrap();
    assert_eq!(saved.revision(), revision + 1, "Autosaved");
    assert_eq!(
        app.world().resource::<Orbit>().pose(),
        camera,
        "A partner's decision does not move the camera"
    );
    finish_presentation(app);
    // The table settles after the partner's decision: its controls are not
    // rebuilt under the human's pointer frame after frame (found by the seat run).
    let control = button_with_label(app, "Overview").expect("The rail");
    app.update();
    app.update();
    assert_eq!(button_with_label(app, "Overview"), Some(control));
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::History);
    app.update();
    let shown = rendered_text(app);
    assert!(
        shown.contains("Patrice Hathaway: Keep this opening hand"),
        "{shown}"
    );
}

/// Ticket #205, Recovery: Take over in the middle of a partner's decision returns
/// Patrice to the human at once, and the partner's later choice is refused as no
/// longer its own.
#[test]
fn take_over_returns_the_decision_to_the_human_and_refuses_the_partners_later_choice() {
    let mut partnered = partnered(&[Patrice]);
    let partner = join(&mut partnered);
    let app = &mut partnered.app;
    choose(app, "confirm-mulligan");
    finish_presentation(app);
    choose(app, "confirm-mulligan");
    finish_presentation(app);
    let revision = app.world().resource::<Table>().session.revision();
    press(app, "Take over Patrice Hathaway");
    assert!(
        offered(app).iter().any(|id| id == "confirm-mulligan"),
        "{:?}",
        offered(app)
    );
    assert!(!rendered_text(app).contains("partner thinking"));
    let refused = exchange(
        app,
        &partnered.socket,
        &[
            json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}),
            json!({"op": "observe", "partner": partner}),
        ],
    );
    assert_eq!(refused[0]["status"], "refused", "{}", refused[0]);
    assert!(
        refused[0]["error"]
            .as_str()
            .unwrap()
            .contains("no longer yours")
    );
    assert_eq!(
        refused[1]["status"], "observed",
        "The connection stays usable"
    );
    assert_eq!(app.world().resource::<Table>().session.revision(), revision);
}

/// A partner's choice over the socket, applied once the table settles.
fn partner_chooses(partnered: &mut Partnered, partner: &str, id: &str) {
    let revision = partnered.app.world().resource::<Table>().session.revision();
    let answer = exchange(
        &mut partnered.app,
        &partnered.socket,
        &[json!({"op": "choose", "partner": partner, "revision": revision, "id": id})],
    );
    assert_eq!(answer[0]["status"], "chosen", "{id}: {}", answer[0]);
    settle(&mut partnered.app);
}

/// Finish the presentation, continuing every story reading as the human would.
fn settle(app: &mut App) {
    loop {
        finish_presentation(app);
        if app
            .world()
            .resource::<Table>()
            .presentation
            .story()
            .is_none()
        {
            return;
        }
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::ContinueStory);
        app.update();
    }
}

/// The choice control holding keyboard focus, if any.
fn focused_choice(app: &App) -> Option<String> {
    let world = app.world();
    let focus = world.resource::<InputFocus>().get()?;
    world
        .get::<ChoiceControl>(focus)
        .map(|control| control.0.clone())
}

/// Ticket #207: with a partner playing the lead, the human's controls do not offer
/// the group choice of who takes the next turn, and the lead's portrait shows its
/// partner deciding. Undo is offered to the human only for the human's own latest
/// decision, and does not take keyboard focus while a partner decides.
#[test]
fn the_humans_controls_leave_a_partner_leads_group_choice_and_its_undo_to_that_partner() {
    let mut partnered = partnered(&[Jacqueline]);
    // An open menu holds the table's presentation, and with it a partner's choice.
    press(&mut partnered.app, "Close menu");
    let partner = join(&mut partnered);
    partner_chooses(&mut partnered, &partner, "confirm-mulligan"); // Jacqueline
    let app = &mut partnered.app;
    choose(app, "confirm-mulligan"); // Tony
    settle(app);
    choose(app, "confirm-mulligan"); // Patrice
    settle(app);
    let listed = app.world().resource::<Table>().session.observe().choices;
    assert!(listed.iter().any(|choice| choice.id == "turn:tony"));
    assert!(
        offered(app).iter().all(|id| !id.starts_with("turn:")),
        "{:?}",
        offered(app)
    );
    let shown = rendered_text(app);
    assert!(
        shown.contains("Jacqueline Fine: partner thinking"),
        "{shown}"
    );

    partner_chooses(&mut partnered, &partner, "turn:tony");
    let app = &mut partnered.app;
    let listed = app.world().resource::<Table>().session.observe().choices;
    assert!(
        listed.iter().any(|choice| choice.id == "undo"),
        "The partner's group choice can be undone"
    );
    assert!(
        !offered(app).iter().any(|id| id == "undo"),
        "Not by the human"
    );
    assert!(offered(app).iter().any(|id| id == "turn:end"));
    // The human's own latest decision is the human's to undo.
    choose(app, "action:ordinary:resource");
    settle(app);
    assert!(offered(app).iter().any(|id| id == "undo"));
    choose(app, "turn:end");
    settle(app);
    assert!(
        rendered_text(app).contains("Jacqueline Fine: partner thinking"),
        "The lead's partner chooses the next turn"
    );
    assert!(offered(app).iter().any(|id| id == "undo"));
    assert_ne!(focused_choice(app).as_deref(), Some("undo"));
}

/// Ticket #207, Joel as lead: with a partner playing another investigator, the
/// human playing the lead receives the next-turn group choice at the table as
/// before, and Undo of that partner's decision is not the human's.
#[test]
fn the_human_playing_the_lead_keeps_the_group_choice_at_the_table() {
    let mut partnered = partnered(&[Patrice]);
    press(&mut partnered.app, "Close menu");
    let partner = join(&mut partnered);
    let app = &mut partnered.app;
    choose(app, "confirm-mulligan"); // Jacqueline
    settle(app);
    choose(app, "confirm-mulligan"); // Tony
    settle(app);
    partner_chooses(&mut partnered, &partner, "confirm-mulligan"); // Patrice
    let app = &mut partnered.app;
    let table = app.world().resource::<Table>();
    let shown: Vec<_> = table.view().choices.into_iter().map(|c| c.id).collect();
    for id in ["turn:tony", "turn:patrice", "turn:jacqueline"] {
        assert!(shown.iter().any(|offer| offer == id), "{shown:?}");
        assert!(table.human_may(id));
    }
    assert!(!rendered_text(app).contains("partner thinking"));

    choose(app, "turn:patrice");
    settle(app);
    partner_chooses(&mut partnered, &partner, "action:ordinary:resource");
    let app = &mut partnered.app;
    assert!(
        app.world()
            .resource::<Table>()
            .session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "undo")
    );
    assert!(!offered(app).iter().any(|id| id == "undo"));
}

/// Ticket #205: leaving the table ends every partner session and its socket.
#[test]
fn leaving_the_table_ends_every_partner_session() {
    let mut partnered = partnered(&[Patrice]);
    let partner = join(&mut partnered);
    let waiting = connect(
        &partnered.socket,
        &[json!({"op": "wait", "partner": partner})],
    );
    let app = &mut partnered.app;
    for _ in 0..5 {
        app.update();
        thread::sleep(Duration::from_millis(5));
    }
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::ReturnToSelection);
    pump(app, || waiting.is_finished());
    let ended = waiting.join().unwrap();
    assert_eq!(ended[0]["status"], "closed", "{}", ended[0]);
    assert!(fs::symlink_metadata(&partnered.socket).is_err());
}

/// An encounter card a partner's investigator draws is read at the table as usual.
/// Its Continue only closes the reading for the human: resolving the card is the
/// partner's decision, taken once the table has settled (RR p.24 1.4).
#[test]
fn the_human_continues_a_partners_encounter_reading_without_resolving_it() {
    let directory = tempfile::tempdir().unwrap();
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::PatriceCancellationVision);
    let table = Table::new(session, directory.path().join("encounter.session.json"));
    let mut app = super::super::tests::host_with_table(table, false);
    app.insert_resource(PartnerSocket(directory.path().join("partners.sock")));
    // The reading is up from the start, so the menu cannot open: the menu's
    // inputs go to the table directly.
    let inputs = &mut app.world_mut().resource_mut::<Table>().partners.inputs;
    inputs.extend([Input::Hand(Patrice), Input::Enable(true)]);
    app.update();
    assert!(
        app.world()
            .resource::<Table>()
            .partners
            .partner_plays(Patrice)
    );
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .encounter()
            .current()
            .is_some()
    );
    let revision = app.world().resource::<Table>().session.revision();
    press(&mut app, "Continue");
    let table = app.world().resource::<Table>();
    assert!(
        table.presentation.encounter().current().is_none(),
        "{}",
        table.notice
    );
    assert_eq!(
        table.session.revision(),
        revision,
        "Only the partner resolves it"
    );
    assert!(!table.human_may("revelation:resolve"));
}
