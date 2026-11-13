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
    assert!(
        shown.contains(&line(&socket)),
        "The menu shows the line: {shown}"
    );

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

/// Ticket #211, the pasted line's final form: it points to the partner instructions
/// in this repository and gives the join command with the table's socket, and it
/// names no investigator and no human player (play contract, Agent partners).
#[test]
fn the_pasted_line_points_to_the_partner_instructions_and_names_no_player() {
    let instructions = "docs/agents/partner-instructions.md";
    let line = line(Path::new("/run/partners-elsewhere.sock"));
    assert!(line.contains(instructions), "{line}");
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(instructions)
            .is_file(),
        "The line points to a document in the repository"
    );
    assert!(
        line.contains("`scripts/tools partner --socket /run/partners-elsewhere.sock join`"),
        "{line}"
    );
    let words: Vec<String> = line
        .split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .collect();
    let investigators = Investigator::ALL
        .into_iter()
        .flat_map(|who| who.name().split(' ').chain([who.id()]));
    for named in investigators.chain(["i", "me", "my", "human", "joel", "owner"]) {
        assert!(
            !words.contains(&named.to_lowercase()),
            "The line names {named}: {line}"
        );
    }
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

/// Joel's co-play session 1 (#202, comment 6858608958): Jacqueline, Joel's,
/// investigated, and the table showed Patrice's partner thinking in two player
/// windows with nothing Joel could see she might do. The windows were legal and
/// hers: Patrice's Violin reads "[fast] Choose and discard 1 card from your hand
/// and exhaust Patrice's Violin: Choose an investigator at your location to either
/// gain 1 resource or draw 1 card" (06016). A [fast] ability may be used by its
/// controller in any player window (RR p.3 "Triggered Abilities", p.22), and a
/// player window follows ST.1 and ST.2 (RR p.26 "Skill Test Timing"). The window
/// closes when every player with an option has passed (play contract, Agent
/// partners, Shared decisions), so the table waits on her partner. A thinking cue
/// is there "so that I know why the table is waiting" (#202 story 70). Without
/// partners the table names the window's option ("Available now: ..."); with
/// Patrice handed to a partner it must still say what her partner may use.
///
/// Joel's seed and opening: he mulliganed Premonition away, so Jacqueline holds no
/// fast option in her test's windows; Patrice played her Violin on her turn.
#[test]
fn a_partners_player_window_in_the_humans_test_names_what_the_partner_may_use() {
    let mut session = Session::with_seed(12_374_354_593_772_059_857);
    for id in [
        "lead:jacqueline",
        "mulligan:212",
        "mulligan:210",
        "confirm-mulligan",
        "mulligan:28",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:patrice",
        "action:ordinary:play:127",
        "turn:end",
        "turn:jacqueline",
        "action:ordinary:investigate",
    ] {
        let revision = session.observe().revision;
        session
            .choose(revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("partners.sock");
    let table = Table::new(session, directory.path().join("partners.session.json"));
    let mut app = super::super::tests::host_with_table(table, false);
    app.insert_resource(PartnerSocket(socket.clone()));
    finish_presentation(&mut app);
    // The line naming her option; the asset's own label names only the card.
    let names_violin = |text: &str| {
        text.lines()
            .any(|line| line.contains("Patrice Hathaway") && line.contains("Patrice's Violin"))
    };
    let solo = rendered_text(&mut app);
    assert!(names_violin(&solo), "Without partners:\n{solo}");

    press(&mut app, "Menu");
    press(&mut app, "Agent partners");
    press(&mut app, "Patrice Hathaway: you play");
    press(&mut app, "Enable agent partners");
    press(&mut app, "Close menu");
    let joined = exchange(&mut app, &socket, &[json!({"op": "join"})]);
    assert_eq!(joined[0]["status"], "joined", "{}", joined[0]);
    let partner = joined[0]["partner"].as_str().unwrap();
    // Her partner is woken for the window, and its options are her Violin and Pass.
    let woken = exchange(
        &mut app,
        &socket,
        &[json!({"op": "wait", "partner": partner})],
    );
    assert_eq!(woken[0]["status"], "decision", "{}", woken[0]);
    let options: Vec<&str> = woken[0]["view"]["decision"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["id"].as_str().unwrap())
        .collect();
    assert!(options.contains(&"window:pass"), "{options:?}");
    assert!(
        options
            .iter()
            .all(|id| *id == "window:pass" || id.starts_with("violin:127:")),
        "{options:?}"
    );
    app.update();
    let shown = rendered_text(&mut app);
    assert!(
        shown.contains("Patrice Hathaway: partner thinking"),
        "{shown}"
    );
    assert!(
        names_violin(&shown),
        "The table waits on Patrice's partner without saying what she may use:\n{shown}"
    );
}

/// A table at Tony's commitment window in the Study, where every investigator
/// starts, reached by genuine decisions so it can commit and save: seed 89 deals
/// Patrice her Violin (06016, a wild icon, committable to any test). Patrice is
/// handed to partners. Returns the app, the socket, a card the human's controls
/// offer to commit, and her Violin.
fn shared_commitment(directory: &Path) -> (App, PathBuf, String, u16) {
    let mut session = Session::with_seed(89);
    for id in [
        "lead:jacqueline",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:tony",
        "action:ordinary:investigate",
    ] {
        session.choose(session.revision(), id).unwrap();
    }
    while session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        session.choose(session.revision(), "window:pass").unwrap();
    }
    let observation = session.observe();
    let play = observation.play.as_ref().unwrap();
    let violin = play.players[Patrice.index()]
        .hand
        .iter()
        .find(|card| card.code == "06016")
        .expect("Seed 89 deals Patrice her Violin")
        .id;
    let socket = directory.join("partners.sock");
    let table = Table::new(session, directory.join("shared.session.json"));
    let mut app = super::super::tests::host_with_table(table, false);
    app.insert_resource(PartnerSocket(socket.clone()));
    finish_presentation(&mut app);
    press(&mut app, "Menu");
    press(&mut app, "Agent partners");
    press(&mut app, "Patrice Hathaway: you play");
    press(&mut app, "Enable agent partners");
    press(&mut app, "Close menu");
    let humans = app
        .world()
        .resource::<Table>()
        .view()
        .choices
        .into_iter()
        .map(|choice| choice.id)
        .find(|id| id.starts_with("commit:") && id != "commit:confirm")
        .expect("The human has a card to commit");
    (app, socket, humans, violin)
}

fn readable(text: &str) -> String {
    crate::tabletop_support::readable_text(text.into())
}

fn revision(app: &App) -> u64 {
    app.world().resource::<Table>().session.revision()
}

/// The captions of the cards on the table.
fn captions(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    world
        .query::<(&Text, &motion::CardLabel)>()
        .iter(world)
        .map(|(text, _)| text.0.clone())
        .collect()
}

/// Ticket #208 and the presentation contract ("In a shared decision"): the
/// partner's pending card appears in the Test area, distinct from confirmed
/// commitments; each player's readiness is shown by investigator name; Confirm is
/// the human's readiness and, while it waits, names whose readiness is
/// outstanding; when the partner's change clears it, the control says so; the
/// test goes ahead once both are ready (RR p.26 ST.2).
#[test]
fn a_shared_commitment_shows_the_partners_pending_card_and_confirm_is_the_humans_readiness() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, socket, humans, violin) = shared_commitment(directory.path());
    let joined = exchange(&mut app, &socket, &[json!({"op": "join"})]);
    let partner = joined[0]["partner"].as_str().unwrap().to_owned();
    assert!(
        !offered(&mut app).contains(&format!("commit:{violin}")),
        "Her card is hers to commit"
    );
    let at = revision(&app);
    let chosen = exchange(
        &mut app,
        &socket,
        &[
            json!({"op": "choose", "partner": partner, "revision": at, "id": format!("commit:{violin}")}),
        ],
    );
    assert_eq!(chosen[0]["status"], "chosen", "{}", chosen[0]);
    finish_presentation(&mut app);
    let pending = captions(&mut app);
    assert!(
        pending
            .iter()
            .any(|caption| caption.contains("Patrice's Violin")
                && caption.contains("Patrice Hathaway: pending")),
        "{pending:?}"
    );
    assert!(
        !pending.iter().any(|caption| caption.contains("commitment")),
        "Nothing is confirmed yet: {pending:?}"
    );
    let shown = rendered_text(&mut app);
    assert!(
        shown.contains("Commitments: Tony Morgan") && shown.contains("Patrice Hathaway deciding"),
        "{shown}"
    );

    // The human's Confirm is his readiness: the test waits, and Confirm names
    // whose readiness is outstanding.
    choose(&mut app, &humans);
    finish_presentation(&mut app);
    let before = revision(&app);
    press(&mut app, "Confirm committed cards");
    assert_eq!(revision(&app), before, "The test waits for her");
    let shown = rendered_text(&mut app);
    assert!(
        button_with_label(
            &mut app,
            &readable("Confirmed · waiting for Patrice Hathaway")
        )
        .is_some(),
        "{shown}"
    );
    assert!(shown.contains(&readable(" ready ·")), "{shown}");
    assert!(shown.contains("Patrice Hathaway deciding"), "{shown}");
    assert!(shown.contains("Decision: Patrice Hathaway"), "{shown}");
    assert!(
        shown.contains("Patrice Hathaway: partner thinking"),
        "{shown}"
    );

    // Her change clears his readiness, and the control says so.
    let at = revision(&app);
    exchange(
        &mut app,
        &socket,
        &[
            json!({"op": "choose", "partner": partner, "revision": at, "id": format!("commit:{violin}")}),
        ],
    );
    finish_presentation(&mut app);
    assert!(
        button_with_label(
            &mut app,
            &readable("Confirm committed cards again · Patrice Hathaway changed it")
        )
        .is_some(),
        "{}",
        rendered_text(&mut app)
    );

    // Both ready on the same cards: the test goes ahead.
    let at = revision(&app);
    let ready = exchange(
        &mut app,
        &socket,
        &[
            json!({"op": "choose", "partner": partner, "revision": at, "id": format!("commit:{violin}")}),
            json!({"op": "ready", "partner": partner, "revision": at + 1}),
        ],
    );
    assert_eq!(ready[1]["status"], "ready", "{}", ready[1]);
    finish_presentation(&mut app);
    press(
        &mut app,
        &readable("Confirm committed cards again · Patrice Hathaway changed it"),
    );
    // The click rebuilt the scene; the next frame finds the table settled.
    app.update();
    finish_presentation(&mut app);
    let test = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap()
        .test
        .unwrap();
    let committed: Vec<u16> = test.committed.iter().map(|card| card.card.id).collect();
    assert!(
        committed.contains(&humans.strip_prefix("commit:").unwrap().parse().unwrap())
            && committed.contains(&violin),
        "{committed:?}"
    );
    let shown = captions(&mut app);
    assert!(
        shown
            .iter()
            .any(|caption| caption.contains("Patrice Hathaway's commitment")),
        "{shown:?}"
    );
}

/// The control directly below `who`'s portrait on the rail, when it is a button.
fn below_portrait(app: &mut App, who: Investigator) -> Option<(Entity, String)> {
    let world = app.world_mut();
    let portrait = world
        .query::<(Entity, &talk::Portrait)>()
        .iter(world)
        .find(|(_, portrait)| portrait.0 == who)
        .map(|(entity, _)| entity)
        .expect("The rail's portrait");
    let rail = world.get::<ChildOf>(portrait)?.parent();
    let children: Vec<Entity> = world.get::<Children>(rail)?.iter().collect();
    let next = *children.get(children.iter().position(|child| *child == portrait)? + 1)?;
    world.get::<Button>(next)?;
    let label = world.get::<AccessibleLabel>(next)?.0.clone();
    Some((next, crate::tabletop_support::readable_text(label)))
}

/// Review fix D1 (spec #202, Table: "The thinking cue and **Take over** on the
/// portrait and in the compact decision display"; presentation contract: while a
/// partner is deciding, its portrait offers Take over). The portrait's marks take
/// no input, so the rail holds a real control under the portrait, and only while
/// that investigator's partner decides.
#[test]
fn a_deciding_partners_portrait_offers_take_over() {
    let mut partnered = partnered(&[Patrice]);
    join(&mut partnered);
    let app = &mut partnered.app;
    let take_over = |app: &mut App, who: Investigator| {
        below_portrait(app, who)
            .filter(|(_, label)| *label == format!("Take over {}", who.name()))
            .map(|(entity, _)| entity)
    };
    assert_eq!(
        take_over(app, Patrice),
        None,
        "Jacqueline's mulligan is first"
    );
    choose(app, "confirm-mulligan"); // Jacqueline
    finish_presentation(app);
    choose(app, "confirm-mulligan"); // Tony
    finish_presentation(app);
    assert!(rendered_text(app).contains("Patrice Hathaway: partner thinking"));
    let control = take_over(app, Patrice).expect("Patrice's portrait offers Take over");
    assert_eq!(take_over(app, Investigator::Tony), None);
    assert_eq!(take_over(app, Jacqueline), None);
    app.world_mut().trigger(Activate { entity: control });
    app.update();
    assert!(
        offered(app).iter().any(|id| id == "confirm-mulligan"),
        "{:?}",
        offered(app)
    );
    assert!(!rendered_text(app).contains("partner thinking"));
    assert_eq!(take_over(app, Patrice), None);
}

/// Review fix D3 (story 13; presentation contract: "A stalled or disconnected
/// partner stays shown as such until it returns or is taken over"). A partner
/// that has sent nothing for minutes while its decision waits is marked with
/// those minutes on its portrait and in the decision display, and shows as
/// thinking again once the table hears from it.
#[test]
fn a_silent_partner_is_marked_with_its_minutes_until_it_is_heard_from() {
    let mut partnered = partnered(&[Patrice]);
    let ahead = Arc::new(Mutex::new(Duration::ZERO));
    let clock = ahead.clone();
    partnered
        .app
        .insert_resource(PartnerClock(Arc::new(move || {
            Instant::now() + *clock.lock().unwrap()
        })));
    let partner = join(&mut partnered);
    let app = &mut partnered.app;
    choose(app, "confirm-mulligan"); // Jacqueline
    finish_presentation(app);
    choose(app, "confirm-mulligan"); // Tony
    finish_presentation(app);
    assert!(rendered_text(app).contains("Patrice Hathaway: partner thinking"));

    *ahead.lock().unwrap() = Duration::from_secs(150);
    app.update();
    let shown = rendered_text(app);
    assert!(shown.contains("SILENT 2 MIN"), "{shown}");
    assert!(
        shown.contains("Patrice Hathaway: no word from its partner for 2 min"),
        "{shown}"
    );
    assert!(button_with_label(app, "Take over Patrice Hathaway").is_some());

    let observed = exchange(
        app,
        &partnered.socket,
        &[json!({"op": "observe", "partner": partner})],
    );
    assert_eq!(observed[0]["status"], "observed", "{}", observed[0]);
    app.update();
    let shown = rendered_text(app);
    assert!(!shown.contains("SILENT"), "{shown}");
    assert!(
        shown.contains("Patrice Hathaway: partner thinking"),
        "{shown}"
    );
}

/// Jacqueline's turn with Patrice's Violin ready in play (06016: "[fast] ...
/// usable in any player window"), saved as the window saves it, opened at the
/// table with `partners` handed to agent partners and partners enabled through the
/// menu, as `partnered` does, and the menu closed again.
fn violin_in_jacquelines_turn(directory: &Path, partners: &[Investigator]) -> (App, PathBuf) {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Assets(
        crate::gathering::AssetFixture::RobesOpportunity,
    ));
    let save = directory.join("turn.session.json");
    continuity::save_session(&save, &session).unwrap();
    let socket = directory.join("partners.sock");
    let mut app = super::super::tests::host_with_table(Table::new(session, save), false);
    app.insert_resource(PartnerSocket(socket.clone()));
    finish_presentation(&mut app);
    if !partners.is_empty() {
        press(&mut app, "Menu");
        press(&mut app, "Agent partners");
        for who in partners {
            press(&mut app, &format!("{}: you play", who.name()));
        }
        press(&mut app, "Enable agent partners");
        // The human closes the menu, as Escape does.
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::Close);
        app.update();
    }
    (app, socket)
}

/// F1, Joel's decision of 2026-11-13 (strict), over the table host. In Jacqueline's
/// partner's turn the human holds Patrice's Violin, usable in the player window
/// before each action (RR p.22, p.24 2.2.1): the decision display names what he may
/// use and offers Pass, and the partner's choice waits until he passes.
#[test]
fn the_human_passes_before_a_partners_action_while_holding_a_fast_option() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, socket) = violin_in_jacquelines_turn(directory.path(), &[Jacqueline]);
    let joined = exchange(&mut app, &socket, &[json!({"op": "join"})]);
    assert_eq!(
        joined[0]["investigators"],
        json!(["jacqueline"]),
        "{}",
        joined[0]
    );
    let partner = joined[0]["partner"].as_str().unwrap().to_owned();
    app.update();
    let shown = rendered_text(&mut app);
    assert!(
        shown.contains("You may use") && shown.contains("Patrice's Violin"),
        "{shown}"
    );
    // Review fix G3: his pass is one of the options the partner library gives the
    // table, rendered as any other choice.
    let pass = "Pass before Jacqueline Fine's action";
    assert!(button_with_label(&mut app, pass).is_some(), "{shown}");
    assert!(
        offered(&mut app)
            .iter()
            .any(|id| id == crate::partners::TURN_PASS),
        "{:?}",
        offered(&mut app)
    );

    // Review fix G1: the held choice is answered at once, and a wait tells the
    // partner once it is applied.
    let at = revision(&app);
    let held = exchange(
        &mut app,
        &socket,
        &[json!({"op": "choose", "partner": partner, "revision": at, "id": "turn:end"})],
    );
    assert_eq!(held[0]["status"], "held", "{}", held[0]);
    let told = connect(
        &socket,
        &[json!({"op": "wait", "partner": partner, "timeout_seconds": 60})],
    );
    for _ in 0..20 {
        app.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert!(!told.is_finished(), "Held for the human's pass");
    assert_eq!(revision(&app), at);
    press(&mut app, pass);
    pump(&mut app, || told.is_finished());
    let answer = told.join().unwrap();
    assert_eq!(answer[0]["status"], "chosen", "{}", answer[0]);
    assert_eq!(revision(&app), at + 1);
}

/// F1: without partners the human plays every investigator, so no player window
/// waits on anyone: no Pass is offered and the turn's choice commits at once.
#[test]
fn without_partners_a_turns_choice_commits_at_once() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, _) = violin_in_jacquelines_turn(directory.path(), &[]);
    let shown = rendered_text(&mut app);
    assert!(!shown.contains("You may use"), "{shown}");
    assert!(button_with_label(&mut app, "Pass before Jacqueline Fine's action").is_none());
    let at = revision(&app);
    choose(&mut app, "turn:end");
    assert_eq!(revision(&app), at + 1);
}

/// Jacqueline's turn, Joel's, with Patrice and her Violin handed to a partner that
/// has joined: the partner's identity and the Violin's option for Patrice herself.
fn patrices_partner_in_jacquelines_turn(directory: &Path) -> (App, PathBuf, String, String) {
    let (mut app, socket) = violin_in_jacquelines_turn(directory, &[Patrice]);
    let joined = exchange(&mut app, &socket, &[json!({"op": "join"})]);
    assert_eq!(
        joined[0]["investigators"],
        json!(["patrice"]),
        "{}",
        joined[0]
    );
    let partner = joined[0]["partner"].as_str().unwrap().to_owned();
    let violin = joined[0]["view"]["decision"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|option| option["id"].as_str())
        .find(|id| id.starts_with("violin:") && id.ends_with(":target:patrice:draw"))
        .unwrap_or_else(|| panic!("Her Violin: {}", joined[0]))
        .to_owned();
    app.update();
    (app, socket, partner, violin)
}

/// Review fix G7 (F1; presentation contract: in the human's own turn, his choice
/// waits, named, until every other player with a fast option has passed). Joel
/// ends Jacqueline's turn while Patrice's partner holds her Violin, usable in the
/// player window before each action (06016; RR p.22, p.24 2.2.1): the decision
/// display names his waiting choice and whose pass is outstanding, and his choice
/// is applied once she passes.
#[test]
fn the_humans_own_turn_choice_is_shown_waiting_until_the_partner_passes() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, socket, partner, _) = patrices_partner_in_jacquelines_turn(directory.path());
    let at = revision(&app);
    choose(&mut app, "turn:end");
    app.update();
    assert_eq!(revision(&app), at, "Held for her pass");
    let shown = rendered_text(&mut app);
    assert!(
        shown.contains("End this investigator's turn: waits for the other players to pass"),
        "{shown}"
    );
    assert!(shown.contains("Patrice Hathaway may act"), "{shown}");

    let passed = exchange(
        &mut app,
        &socket,
        &[json!({"op": "choose", "partner": partner, "revision": at, "id": "turn-window:pass"})],
    );
    assert_eq!(passed[0]["status"], "ready", "{}", passed[0]);
    finish_presentation(&mut app);
    assert_eq!(revision(&app), at + 1, "Applied once she passed");
    assert!(!rendered_text(&mut app).contains("waits for the other players to pass"));
}

/// Review fix G2 (F1: a fast play moves the decision on, so the active player
/// decides again). Joel's held click in his own turn is released when Patrice's
/// partner plays her Violin instead of passing: his decision display says that his
/// choice was not applied because another player acted first, and asks him to
/// choose again.
#[test]
fn the_humans_held_choice_released_by_a_fast_play_is_announced() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, socket, partner, violin) = patrices_partner_in_jacquelines_turn(directory.path());
    let at = revision(&app);
    choose(&mut app, "turn:end");
    app.update();
    assert_eq!(revision(&app), at, "Held for her pass");

    let played = exchange(
        &mut app,
        &socket,
        &[json!({"op": "choose", "partner": partner, "revision": at, "id": violin})],
    );
    assert_eq!(played[0]["status"], "chosen", "{}", played[0]);
    finish_presentation(&mut app);
    assert_eq!(revision(&app), at + 1, "Her Violin, not his held choice");
    let shown = rendered_text(&mut app);
    assert!(
        !shown.contains("waits for the other players to pass"),
        "{shown}"
    );
    assert!(
        shown.contains("End this investigator's turn, was not applied: another player acted"),
        "{shown}"
    );
    assert!(shown.contains("Choose again"), "{shown}");
    assert!(
        offered(&mut app).iter().any(|id| id == "turn:end"),
        "He decides again: {:?}",
        offered(&mut app)
    );
}

/// Review fix G6 (F1 under the stall policy: a decision that waits on a partner
/// waits, marked, until one joins or the human takes over; nothing is decided for
/// him). Patrice is handed to partners and holds her Violin in Jacqueline's turn,
/// Joel's, but no partner has joined: ending the turn waits for her seat's pass,
/// the decision display names whose pass is outstanding and marks the seat as
/// awaiting a partner, and Take over applies his choice.
#[test]
fn an_unclaimed_seat_with_a_fast_option_holds_the_humans_turn_until_taken_over() {
    let directory = tempfile::tempdir().unwrap();
    let (mut app, _) = violin_in_jacquelines_turn(directory.path(), &[Patrice]);
    let at = revision(&app);
    choose(&mut app, "turn:end");
    app.update();
    assert_eq!(revision(&app), at, "Held for Patrice's seat");
    let shown = rendered_text(&mut app);
    assert!(
        shown.contains("End this investigator's turn: waits for the other players to pass"),
        "{shown}"
    );
    assert!(shown.contains("Patrice Hathaway may act"), "{shown}");
    assert!(
        shown.contains("Patrice Hathaway: waiting for a partner to join"),
        "{shown}"
    );
    assert!(shown.contains("AWAITING PARTNER"), "{shown}");

    press(&mut app, "Take over Patrice Hathaway");
    // Applied on a frame at which the table has settled from the Take over.
    for _ in 0..5 {
        app.update();
    }
    finish_presentation(&mut app);
    assert_eq!(
        revision(&app),
        at + 1,
        "Applied once no other player holds a fast option"
    );
}

mod pacing;
