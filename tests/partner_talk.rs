//! Seam: table talk over the agent-partner protocol (#209), Bevy-free, driven as a
//! partner drives it: `say` requests and the waits that deliver them. The table is
//! the rules session with a save file and a settled flag the test sets, as in
//! `tests/partners.rs`; the window's speech boxes are tested at the table host.
//! Expectations come from ticket #209, the play contract's "Table talk" policy, the
//! Rules Reference (p.13 player order from the lead; p.16 Peril) and the Gathering's
//! seating order, Tony, Patrice, Jacqueline, which the rules session's player order
//! follows from the lead.

use arkham_unfolded_experiment::{
    gathering::{Investigator, Session, continuity},
    partners::{CHATTER_LIMIT, Host, LONGEST_MESSAGE, PERIL, Table},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf, time::Instant};

use Investigator::{Jacqueline, Patrice, Tony};

struct Rules {
    session: Session,
    save: PathBuf,
    settled: bool,
    _directory: tempfile::TempDir,
}

impl Table for Rules {
    fn session(&self) -> &Session {
        &self.session
    }

    fn settled(&self) -> bool {
        self.settled
    }

    fn commit(&mut self, revision: u64, id: &str, _: Investigator) -> Result<(), String> {
        continuity::commit_choice(&self.save, &mut self.session, revision, id)
            .map_err(|error| error.to_string())
    }
}

struct Sitting {
    host: Host<u32>,
    rules: Rules,
    now: Instant,
    next: u32,
    answers: BTreeMap<u32, Value>,
}

impl Sitting {
    /// A Gathering with `seed` and `lead`, partners enabled for `partners`.
    fn new(seed: u64, lead: Investigator, partners: &[Investigator]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let save = directory.path().join("talk.session.json");
        let session = Session::with_seed(seed);
        continuity::save_session(&save, &session).unwrap();
        let mut host = Host::new();
        for who in partners {
            host.set_selected(*who, true);
        }
        assert!(host.set_enabled(true).is_empty());
        let mut sitting = Self {
            host,
            rules: Rules {
                session,
                save,
                settled: true,
                _directory: directory,
            },
            now: Instant::now(),
            next: 0,
            answers: BTreeMap::new(),
        };
        sitting.human(&format!("lead:{}", lead.id()));
        sitting
    }

    fn send(&mut self, request: &Value) -> u32 {
        self.next += 1;
        let ticket = self.next;
        let answers = self
            .host
            .receive(&request.to_string(), ticket, &mut self.rules, self.now);
        self.answers.extend(answers);
        ticket
    }

    fn ask(&mut self, request: Value) -> Value {
        let ticket = self.send(&request);
        self.take(ticket)
            .unwrap_or_else(|| panic!("{request} is held, not answered"))
    }

    fn take(&mut self, ticket: u32) -> Option<Value> {
        self.answers.remove(&ticket)
    }

    fn poll(&mut self) {
        let answers = self.host.poll(&mut self.rules, self.now);
        self.answers.extend(answers);
    }

    /// The human's click, through the same continuity commit.
    fn human(&mut self, id: &str) {
        let revision = self.rules.session.observe().revision;
        continuity::commit_choice(&self.rules.save, &mut self.rules.session, revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }

    fn join(&mut self, investigators: &[&str]) -> String {
        let answer = self.ask(json!({"op": "join", "investigators": investigators}));
        assert_eq!(answer["status"], "joined", "{answer}");
        answer["partner"].as_str().unwrap().to_owned()
    }

    fn wait(&mut self, partner: &str) -> u32 {
        self.send(&json!({"op": "wait", "partner": partner, "timeout_seconds": 60}))
    }

    fn say(&mut self, partner: &str, text: &str) -> Value {
        self.ask(json!({"op": "say", "partner": partner, "text": text}))
    }
}

/// The messages an answer delivers, as (speaker, text).
fn heard(answer: &Value) -> Vec<(Vec<String>, String)> {
    answer["view"]["table_talk"]
        .as_array()
        .map(|said| {
            said.iter()
                .map(|said| {
                    (
                        said["speaker"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|name| name.as_str().unwrap().to_owned())
                            .collect(),
                        said["text"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// Ticket #209, Delivery and naming: a partner's message wakes every other
/// partner's wait, the human's message reaches every partner, and nobody hears
/// their own message back. Each is named only by its speaker's investigators.
#[test]
fn every_message_reaches_every_other_player_named_by_investigator() {
    // Jacqueline leads and is the human's; her mulligan is pending, so nothing but
    // table talk concerns the partners.
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join(&["tony"]);
    let patrices = sitting.join(&["patrice"]);
    let tony_waits = sitting.wait(&tonys);
    let patrice_waits = sitting.wait(&patrices);
    assert!(sitting.take(tony_waits).is_none());

    let said = sitting.say(&patrices, "I'll take the Attic.");
    assert_eq!(said["status"], "said", "{said}");
    assert_eq!(said["ok"], true);
    sitting.poll();
    let woken = sitting
        .take(tony_waits)
        .expect("Patrice's message wakes Tony's player");
    assert_eq!(woken["status"], "table_talk", "{woken}");
    assert_eq!(
        heard(&woken),
        [(
            names(&["Patrice Hathaway"]),
            "I'll take the Attic.".to_owned()
        )]
    );
    assert!(
        sitting.take(patrice_waits).is_none(),
        "A speaker does not hear its own message back"
    );

    sitting
        .host
        .human_says("Good plan.", &sitting.rules)
        .unwrap();
    sitting.poll();
    let woken = sitting
        .take(patrice_waits)
        .expect("The human's message wakes her");
    assert_eq!(
        heard(&woken),
        [(names(&["Jacqueline Fine"]), "Good plan.".to_owned())]
    );
    let observed = sitting.ask(json!({"op": "observe", "partner": tonys}));
    assert_eq!(
        heard(&observed),
        [(names(&["Jacqueline Fine"]), "Good plan.".to_owned())],
        "A message is delivered once, with the next answer that carries a view"
    );
    let again = sitting.ask(json!({"op": "observe", "partner": tonys}));
    assert!(heard(&again).is_empty(), "{}", again["view"]);
    for answer in [&woken, &observed] {
        let talk = answer["view"]["table_talk"].to_string();
        for absent in ["human", "agent", "partner", "player"] {
            assert!(!talk.to_lowercase().contains(absent), "{absent} in {talk}");
        }
    }
    assert_eq!(
        sitting
            .host
            .table_talk()
            .iter()
            .map(|said| (said.speaker.clone(), said.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (vec![Patrice], "I'll take the Attic."),
            (vec![Jacqueline], "Good plan.")
        ]
    );
}

/// Ticket #209, Naming: a player who plays several investigators is named by all of
/// them in player order, from the lead (RR p.13), and a message in play carries
/// its round.
#[test]
fn a_player_of_several_investigators_is_named_by_them_in_player_order() {
    // Patrice leads: player order is Patrice, Jacqueline, Tony.
    let mut sitting = Sitting::new(89, Patrice, &[Tony, Patrice]);
    let partner = sitting.join(&[]);
    let said = sitting.say(&partner, "Setting up.");
    assert_eq!(said["status"], "said", "{said}");
    for _ in 0..3 {
        sitting.human("confirm-mulligan");
    }
    let said = sitting.say(&partner, "Round one.");
    assert_eq!(said["status"], "said", "{said}");
    let talk = sitting.host.table_talk();
    assert_eq!(talk[0].speaker, [Patrice, Tony]);
    assert_eq!(talk[0].round, None, "Said during setup");
    assert_eq!(talk[1].round, Some(1));
}

/// Seed 224 deals Watcher from Another Dimension on top of Patrice's deck after the
/// mulligans: her first draw action draws it, and its revelation resolves at once.
fn watcher_drawn() -> (Sitting, String, String) {
    let mut sitting = Sitting::new(224, Jacqueline, &[Patrice, Jacqueline]);
    let patrices = sitting.join(&["patrice"]);
    let jacquelines = sitting.join(&["jacqueline"]);
    for _ in 0..3 {
        sitting.human("confirm-mulligan");
    }
    sitting.human("turn:patrice");
    let revision = sitting.rules.session.observe().revision;
    let drawn = sitting.ask(json!({
        "op": "choose", "partner": patrices, "revision": revision, "id": "action:ordinary:draw",
    }));
    assert_eq!(drawn["status"], "chosen", "{drawn}");
    let play = sitting.rules.session.observe().play.unwrap();
    assert!(
        play.players[Patrice.index()]
            .hand
            .iter()
            .any(|card| card.code == "06017"),
        "Patrice drew Watcher"
    );
    (sitting, patrices, jacquelines)
}

/// Ticket #209, Peril silence: while the table presents the drawing of Watcher from
/// Another Dimension, a peril encounter (06017: "Peril"; RR p.16), every `say` is
/// refused with the reason the message box gives, and the human cannot speak. The
/// reason names neither the card nor its investigator. Talk opens again once the
/// table has settled.
#[test]
fn table_talk_closes_while_a_peril_encounter_resolves() {
    let (mut sitting, patrices, jacquelines) = watcher_drawn();
    // The table presents the draw.
    sitting.rules.settled = false;
    for partner in [&jacquelines, &patrices] {
        let refused = sitting.say(partner, "Careful in the Attic.");
        assert_eq!(refused["status"], "refused", "{refused}");
        assert_eq!(refused["error"], PERIL);
        assert!(refused["view"].is_object(), "A refusal carries the view");
    }
    let theirs = sitting.ask(json!({"op": "observe", "partner": jacquelines}));
    assert!(!theirs.to_string().contains("Watcher"), "{theirs}");
    assert!(PERIL.contains("peril encounter") && PERIL.contains("p. 16"));
    for hidden in ["Watcher", "Patrice"] {
        assert!(!PERIL.contains(hidden));
    }
    assert_eq!(sitting.host.talk_closed(&sitting.rules), Some(PERIL));
    assert_eq!(
        sitting.host.human_says("Everyone fine?", &sitting.rules),
        Err(PERIL.to_owned())
    );
    assert!(sitting.host.table_talk().is_empty());

    sitting.rules.settled = true;
    sitting.poll();
    assert_eq!(sitting.host.talk_closed(&sitting.rules), None);
    // A later unsettled moment of the same decision is no longer the encounter.
    sitting.rules.settled = false;
    let said = sitting.say(&jacquelines, "Careful in the Attic.");
    assert_eq!(said["status"], "said", "{said}");
}

/// Ticket #209, Chatter limit: a partner sends at most CHATTER_LIMIT messages in a
/// row between committed decisions; the refusal names the limit. Another player's
/// message, or a committed decision, lets it speak again.
#[test]
fn a_partner_speaks_at_most_the_chatter_limit_in_a_row_between_decisions() {
    assert!((2..=5).contains(&CHATTER_LIMIT), "A small fixed number");
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join(&["tony"]);
    let patrices = sitting.join(&["patrice"]);
    let talk = |sitting: &mut Sitting, partner: &str, times: usize| {
        for time in 0..times {
            let said = sitting.say(partner, &format!("Thought {time}."));
            assert_eq!(said["status"], "said", "{said}");
        }
    };
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    let refused = sitting.say(&tonys, "One more thing.");
    assert_eq!(refused["status"], "refused", "{refused}");
    let reason = refused["error"].as_str().unwrap();
    assert!(
        reason.contains("chatter limit") && reason.contains(&CHATTER_LIMIT.to_string()),
        "{reason}"
    );
    assert!(refused["view"].is_object());

    // Another partner speaks: Tony's player may answer.
    talk(&mut sitting, &patrices, 1);
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    assert_eq!(sitting.say(&tonys, "Again.")["status"], "refused");
    // The human speaks.
    sitting.host.human_says("Go on.", &sitting.rules).unwrap();
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    assert_eq!(sitting.say(&tonys, "Again.")["status"], "refused");
    // Play moves on: the human's mulligan is committed.
    sitting.human("confirm-mulligan");
    talk(&mut sitting, &tonys, CHATTER_LIMIT);
    assert_eq!(sitting.say(&tonys, "Again.")["status"], "refused");
    // The human is not limited.
    for _ in 0..CHATTER_LIMIT + 2 {
        sitting.host.human_says("Noted.", &sitting.rules).unwrap();
    }
}

/// Ticket #209: a message is one short line, a partner with no investigator has no
/// seat to speak from, and neither does the human when partners play all three. A
/// refused `say` leaves the connection usable. Talk never makes a decision and is
/// never saved with the attempt.
#[test]
fn talk_is_one_short_message_from_a_seat_and_never_enters_the_attempt() {
    let mut sitting = Sitting::new(89, Jacqueline, &[Tony, Patrice]);
    let tonys = sitting.join(&["tony"]);
    let saved = std::fs::read(&sitting.rules.save).unwrap();
    let revision = sitting.rules.session.observe().revision;
    let history = sitting.rules.session.observe().history;

    let empty = sitting.say(&tonys, "  \n ");
    assert_eq!(empty["status"], "refused", "{empty}");
    let long = "word ".repeat(LONGEST_MESSAGE / 4);
    let refused = sitting.say(&tonys, &long);
    assert_eq!(refused["status"], "refused", "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains(&LONGEST_MESSAGE.to_string())
    );
    let said = sitting.say(&tonys, "Two\nlines   become one.");
    assert_eq!(said["status"], "said", "{said}");
    assert_eq!(sitting.host.table_talk()[0].text, "Two lines become one.");
    let fight = sitting.say(&tonys, "turn:tony");
    assert_eq!(fight["status"], "said", "Words are never a choice");
    assert_eq!(sitting.rules.session.observe().revision, revision);
    assert_eq!(sitting.rules.session.observe().history, history);
    assert_eq!(std::fs::read(&sitting.rules.save).unwrap(), saved);

    sitting.host.set_selected(Tony, false);
    let seatless = sitting.say(&tonys, "Hello?");
    assert_eq!(seatless["status"], "refused", "{seatless}");
    assert!(
        seatless["error"]
            .as_str()
            .unwrap()
            .contains("no investigator")
    );
    assert_eq!(
        sitting.ask(json!({"op": "observe", "partner": tonys}))["status"],
        "observed"
    );

    let mut all = Sitting::new(89, Jacqueline, &[Tony, Patrice, Jacqueline]);
    assert!(all.host.human_says("Hello.", &all.rules).is_err());
}
