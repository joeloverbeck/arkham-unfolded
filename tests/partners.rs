//! Seam: the agent-partner protocol (#205) over the Bevy-free rules session, driven
//! as a partner drives it: one JSON request at a time, asserting the answers. The
//! table here is the rules session with a save file and a settled flag the test
//! sets; the window's own table is tested in `gathering_tabletop::partners`.
//! Expectations come from ticket #205, the play contract's "Agent partners" policy
//! and the Gathering's setup order (Rules Reference p.13: player order starts with
//! the lead), not from the implementation.

use arkham_unfolded_experiment::{
    gathering::{
        HostileFixture, Investigator, LifecycleFixture, RetryFixture, Session, continuity,
    },
    partners::{Controller, Host, Table},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use Investigator::{Jacqueline, Patrice, Tony};

/// The rules session as a table: it commits through the continuity path, is
/// settled whenever the test says so, and records whom each commit was attributed to.
struct Rules {
    session: Session,
    save: PathBuf,
    settled: bool,
    attributed: Vec<(String, Investigator)>,
    _directory: tempfile::TempDir,
}

impl Table for Rules {
    fn session(&self) -> &Session {
        &self.session
    }

    fn settled(&self) -> bool {
        self.settled
    }

    fn commit(&mut self, revision: u64, id: &str, by: Investigator) -> Result<(), String> {
        continuity::commit_choice(&self.save, &mut self.session, revision, id)
            .map_err(|error| error.to_string())?;
        self.attributed.push((id.to_owned(), by));
        Ok(())
    }
}

/// A table with partners enabled for `partners`, and every answer by request.
struct Sitting {
    host: Host<u32>,
    rules: Rules,
    now: Instant,
    next: u32,
    answers: BTreeMap<u32, Value>,
}

impl Sitting {
    fn new(session: Session, partners: &[Investigator]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let save = directory.path().join("partners.session.json");
        continuity::save_session(&save, &session).unwrap();
        let mut host = Host::new();
        for who in partners {
            host.set_selected(*who, true);
        }
        assert!(host.set_enabled(true).is_empty());
        Self {
            host,
            rules: Rules {
                session,
                save,
                settled: true,
                attributed: Vec::new(),
                _directory: directory,
            },
            now: Instant::now(),
            next: 0,
            answers: BTreeMap::new(),
        }
    }

    /// A fresh Gathering whose lead is Jacqueline: mulligans then go Jacqueline,
    /// Tony, Patrice (RR p.13, player order from the lead).
    fn after_lead(partners: &[Investigator]) -> Self {
        let mut sitting = Self::new(Session::with_seed(89), partners);
        sitting.human("lead:jacqueline");
        sitting
    }

    /// Send one request line; returns its ticket.
    fn send(&mut self, request: &Value) -> u32 {
        self.next += 1;
        let ticket = self.next;
        let answers = self
            .host
            .receive(&request.to_string(), ticket, &mut self.rules, self.now);
        self.answers.extend(answers);
        ticket
    }

    /// One request that must be answered at once.
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
        let control = self.host.control();
        assert!(
            control.human_may(&self.rules.session, id),
            "{id} is not the human's"
        );
        let revision = self.rules.session.observe().revision;
        continuity::commit_choice(&self.rules.save, &mut self.rules.session, revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }

    fn join(&mut self, investigators: &[&str]) -> String {
        let answer = self.ask(json!({"op": "join", "investigators": investigators}));
        assert_eq!(answer["status"], "joined", "{answer}");
        answer["partner"].as_str().unwrap().to_owned()
    }

    fn revision(&self) -> u64 {
        self.rules.session.observe().revision
    }

    /// A partner's choice at the current revision, accepted at once.
    fn partner(&mut self, partner: &str, id: &str) -> Value {
        let revision = self.revision();
        let answer =
            self.ask(json!({"op": "choose", "partner": partner, "revision": revision, "id": id}));
        assert_eq!(answer["status"], "chosen", "{id}: {answer}");
        answer
    }

    fn view(&mut self, partner: &str) -> Value {
        self.ask(json!({"op": "observe", "partner": partner}))["view"].clone()
    }

    fn human_may(&self, id: &str) -> bool {
        self.host.control().human_may(&self.rules.session, id)
    }

    /// The listed options of the pending decision, without Undo.
    fn listed(&self) -> Vec<String> {
        self.rules
            .session
            .observe()
            .choices
            .into_iter()
            .map(|choice| choice.id)
            .filter(|id| id != "undo")
            .collect()
    }

    /// Take `id` as whichever player it belongs to: the human, or the one of
    /// `partners` whose view lists it.
    fn play(&mut self, partners: &[&str], id: &str) {
        if self.human_may(id) {
            return self.human(id);
        }
        let partner = partners
            .iter()
            .find(|partner| option_ids(&self.view(partner)).contains(&id))
            .unwrap_or_else(|| panic!("No player lists {id}"));
        self.partner(partner, id);
    }

    /// Pass windows and confirm tests, by whichever player each belongs to, until
    /// `expected` is listed.
    fn reach(&mut self, partners: &[&str], expected: &str) {
        for _ in 0..12 {
            let listed = self.listed();
            if listed.iter().any(|id| id == expected) {
                return;
            }
            let id = [
                "window:pass",
                "commit:confirm",
                "test:reveal",
                "lucky:decline",
            ]
            .into_iter()
            .find(|id| listed.iter().any(|listed| listed == id))
            .unwrap_or_else(|| panic!("No progression toward {expected}: {listed:?}"));
            self.play(partners, id);
        }
        panic!("Did not reach {expected}");
    }
}

fn investigators(answer: &Value) -> Vec<&str> {
    answer["investigators"]
        .as_array()
        .unwrap_or_else(|| panic!("no investigators: {answer}"))
        .iter()
        .map(|who| who.as_str().unwrap())
        .collect()
}

fn option_ids(view: &Value) -> Vec<&str> {
    view["decision"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["id"].as_str().unwrap())
        .collect()
}

fn group_choice(answer: &Value) -> &Value {
    &answer["view"]["decision"]["group_choice"]
}

/// Ticket #207, Partner as lead and Group-choice events: the lead investigator's
/// player is the final arbiter of who takes the next turn (RR p.24 2.2; the play
/// contract's house practice). A group choice opening wakes every partner's wait,
/// the arbiter's and another partner's alike, once; only the arbiter's view lists
/// its options, and the human's controls do not offer them.
#[test]
fn a_partner_playing_the_lead_chooses_who_takes_the_next_turn_and_every_partner_hears_it_open() {
    let mut sitting = Sitting::new(Session::with_seed(89), &[Patrice, Jacqueline]);
    sitting.human("lead:jacqueline");
    let lead = sitting.join(&["jacqueline"]);
    let other = sitting.join(&["patrice"]);
    // Mulligans go in player order from the lead (RR p.13): Jacqueline, Tony, Patrice.
    sitting.partner(&lead, "confirm-mulligan");
    let leads = sitting.send(&json!({"op": "wait", "partner": lead, "timeout_seconds": 60}));
    sitting.human("confirm-mulligan");
    sitting.poll();
    assert!(sitting.take(leads).is_none(), "Patrice's mulligan is next");
    sitting.partner(&other, "confirm-mulligan");
    sitting.poll();

    let opened = sitting.take(leads).expect("The group choice opened");
    assert_eq!(opened["status"], "group_choice", "{opened}");
    assert_eq!(group_choice(&opened)["arbiter"], "jacqueline", "{opened}");
    assert_eq!(opened["view"]["decision"]["yours"], true);
    assert_eq!(
        option_ids(&opened["view"]),
        ["turn:tony", "turn:patrice", "turn:jacqueline"]
    );
    let heard = sitting.ask(json!({"op": "wait", "partner": other}));
    assert_eq!(heard["status"], "group_choice", "{heard}");
    assert_eq!(group_choice(&heard)["arbiter"], "jacqueline", "{heard}");
    assert_eq!(heard["view"]["decision"]["yours"], false);
    assert_eq!(option_ids(&heard["view"]), Vec::<&str>::new());
    assert_eq!(
        heard["view"]["decision"]["others_deciding"],
        json!(["Jacqueline Fine"])
    );
    for id in sitting.listed() {
        assert!(!sitting.human_may(&id), "{id} is the lead's player's");
    }
    // Each partner hears an opening once; the arbiter's next wait is its decision.
    let quiet = sitting.send(&json!({"op": "wait", "partner": other, "timeout_seconds": 60}));
    sitting.poll();
    assert!(sitting.take(quiet).is_none());
    let decision = sitting.ask(json!({"op": "wait", "partner": lead}));
    assert_eq!(decision["status"], "decision", "{decision}");

    sitting.partner(&lead, "turn:patrice");
    assert_eq!(
        sitting.rules.attributed.last(),
        Some(&("turn:patrice".to_owned(), Jacqueline))
    );
    sitting.poll();
    let turn = sitting.take(quiet).expect("Patrice's turn began");
    assert_eq!(turn["status"], "decision", "{turn}");
}

/// Ticket #207, Joel as lead: the human playing the lead makes the group choice as
/// before; a partner's wait hears it open, and its view names the arbiter but lists
/// no options.
#[test]
fn the_human_playing_the_lead_makes_the_group_choice_and_a_partner_sees_it_open_without_options() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan"); // Jacqueline
    sitting.human("confirm-mulligan"); // Tony
    sitting.partner(&partner, "confirm-mulligan");

    let heard = sitting.ask(json!({"op": "wait", "partner": partner}));
    assert_eq!(heard["status"], "group_choice", "{heard}");
    assert_eq!(group_choice(&heard)["arbiter"], "jacqueline", "{heard}");
    assert_eq!(heard["view"]["decision"]["yours"], false);
    assert_eq!(option_ids(&heard["view"]), Vec::<&str>::new());
    let listed = sitting.listed();
    assert_eq!(listed, ["turn:tony", "turn:patrice", "turn:jacqueline"]);
    for id in &listed {
        assert!(sitting.human_may(id), "{id}");
    }
    let held = sitting.send(&json!({"op": "wait", "partner": partner, "timeout_seconds": 60}));
    sitting.poll();
    assert!(sitting.take(held).is_none(), "Heard once");
    sitting.human("turn:patrice");
    sitting.poll();
    assert_eq!(sitting.take(held).unwrap()["status"], "decision");
}

/// Ticket #207: choices a card gives the lead investigator go to the lead's player.
/// Agenda 1b (01105b): "The lead investigator must decide"; the Hunters' order and
/// their movement among equally near investigators (RR p.12 "Hunter", p.17
/// "Priority of Simultaneous Resolution"). With a partner as lead, the partner
/// hears each one open with its options and the human may not take them; with the
/// human as lead, the human may, and a partner hears it open without options.
#[test]
fn the_leads_player_receives_the_choices_a_card_gives_the_lead() {
    for (fixture, first, then) in [
        (
            HostileFixture::GatheringDoomThreshold,
            "mythos:begin",
            "agenda:lead-horror",
        ),
        (
            HostileFixture::GatheringHunters,
            "enemy:hunter:400",
            "hunter:move:",
        ),
    ] {
        let session = Session::with_hostile_fixture(fixture);
        let lead = session.observe().lead.expect("A lead");
        let mut sitting = Sitting::new(session.clone(), &[lead]);
        let partner = sitting.join(&[]);
        let opened = sitting.ask(json!({"op": "wait", "partner": partner}));
        assert_eq!(opened["status"], "group_choice", "{fixture:?}: {opened}");
        assert_eq!(group_choice(&opened)["arbiter"], json!(lead));
        assert!(option_ids(&opened["view"]).contains(&first), "{opened}");
        assert!(!sitting.human_may(first), "{fixture:?}");
        sitting.partner(&partner, first);
        let next = sitting.ask(json!({"op": "wait", "partner": partner}));
        assert_eq!(next["status"], "group_choice", "{fixture:?}: {next}");
        let ids = option_ids(&next["view"]);
        assert!(ids.iter().any(|id| id.starts_with(then)), "{ids:?}");
        assert_eq!(ids, sitting.listed(), "{fixture:?}");
        for id in ids {
            assert!(!sitting.human_may(id), "{id}");
        }

        let another = Investigator::ALL
            .into_iter()
            .find(|who| *who != lead)
            .unwrap();
        let mut sitting = Sitting::new(session, &[another]);
        let partner = sitting.join(&[]);
        let heard = sitting.ask(json!({"op": "wait", "partner": partner}));
        assert_eq!(heard["status"], "group_choice", "{fixture:?}: {heard}");
        assert_eq!(group_choice(&heard)["arbiter"], json!(lead));
        assert_eq!(option_ids(&heard["view"]), Vec::<&str>::new());
        assert!(sitting.human_may(first), "{fixture:?}");
    }
}

/// Ticket #207, New lead: when the lead is eliminated, the player of the next
/// investigator in player order chooses the new lead (RR p.14 leaves it to the
/// remaining players; the play contract's house practice names the arbiter; RR
/// p.13 player order is clockwise Tony, Patrice, Jacqueline). Jacqueline leads and
/// is defeated, so Tony's partner chooses; Patrice's partner hears it open.
#[test]
fn eliminating_the_lead_gives_the_new_lead_choice_to_the_next_investigators_player() {
    let session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::EndTurnDefeat));
    assert_eq!(session.observe().lead, Some(Jacqueline));
    let mut sitting = Sitting::new(session, &[Tony, Patrice]);
    let tonys = sitting.join(&["tony"]);
    let patrices = sitting.join(&["patrice"]);
    let partners = [tonys.as_str(), patrices.as_str()];
    // Jacqueline takes a resource and the engaged Priest's attack defeats her.
    for id in [
        "action:ordinary:resource",
        "harm:damage:investigator",
        "harm:damage:investigator",
    ] {
        sitting.play(&partners, id);
    }
    sitting.reach(&partners, "elimination:lead:tony");

    let chooses = sitting.ask(json!({"op": "wait", "partner": tonys}));
    assert_eq!(chooses["status"], "group_choice", "{chooses}");
    assert_eq!(group_choice(&chooses)["arbiter"], "tony");
    let ids = option_ids(&chooses["view"]);
    assert!(ids.contains(&"elimination:lead:tony"), "{ids:?}");
    assert!(
        ids.iter().all(|id| id.starts_with("elimination:lead:")),
        "{ids:?}"
    );
    let hears = sitting.ask(json!({"op": "wait", "partner": patrices}));
    assert_eq!(hears["status"], "group_choice", "{hears}");
    assert_eq!(option_ids(&hears["view"]), Vec::<&str>::new());
    assert!(!sitting.human_may("elimination:lead:tony"));
    sitting.partner(&tonys, "elimination:lead:patrice");
    assert_eq!(sitting.rules.session.observe().lead, Some(Patrice));
    assert_eq!(
        sitting.rules.attributed.last(),
        Some(&("elimination:lead:patrice".to_owned(), Tony))
    );
}

/// Ticket #205, Joining: by default a join takes every investigator handed to
/// partners that no partner holds; a later join finds none left.
#[test]
fn joining_by_default_takes_every_unclaimed_investigator_handed_to_partners() {
    let mut sitting = Sitting::after_lead(&[Patrice, Jacqueline]);
    let joined = sitting.ask(json!({"op": "join"}));
    assert_eq!(joined["status"], "joined", "{joined}");
    assert_eq!(investigators(&joined), ["patrice", "jacqueline"]);
    assert_eq!(joined["view"]["you_play"], json!(["patrice", "jacqueline"]));
    let partner = joined["partner"].as_str().unwrap().to_owned();
    assert_eq!(
        sitting.host.control().controller(Patrice),
        Controller::Partner(partner)
    );
    assert_eq!(sitting.host.control().controller(Tony), Controller::Human);

    let late = sitting.ask(json!({"op": "join"}));
    assert_eq!(late["status"], "refused", "{late}");
    assert_eq!(late["ok"], false);
    assert!(
        late["error"]
            .as_str()
            .unwrap()
            .contains("No investigator is open to partners")
    );
}

/// Ticket #205, Joining: a join naming investigators takes only those no partner
/// holds; naming one another partner holds, or one not handed to partners, is
/// refused and claims nothing.
#[test]
fn a_named_join_takes_only_unclaimed_investigators_and_a_conflicting_claim_is_refused() {
    let mut sitting = Sitting::after_lead(&[Patrice, Jacqueline]);
    let first = sitting.ask(json!({"op": "join", "investigators": ["patrice"]}));
    assert_eq!(investigators(&first), ["patrice"]);

    let conflict = sitting.ask(json!({"op": "join", "investigators": ["jacqueline", "patrice"]}));
    assert_eq!(conflict["status"], "refused", "{conflict}");
    assert!(
        conflict["error"]
            .as_str()
            .unwrap()
            .contains("Patrice Hathaway is already played")
    );
    let closed = sitting.ask(json!({"op": "join", "investigators": ["tony"]}));
    assert_eq!(closed["status"], "refused", "{closed}");
    assert_eq!(
        sitting.host.control().controller(Jacqueline),
        Controller::Unclaimed
    );

    let second = sitting.ask(json!({"op": "join"}));
    assert_eq!(investigators(&second), ["jacqueline"]);
    assert_ne!(first["partner"], second["partner"]);
}

/// Ticket #205, Waiting: a wait returns when the player's decision is pending at a
/// settled table, and not before the table settles; otherwise it answers
/// "nothing yet" once its period runs out.
#[test]
fn a_wait_returns_the_players_decision_once_the_table_settles_or_nothing_yet_at_its_timeout() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);

    // Jacqueline's mulligan is the human's; nothing concerns the partner yet.
    let quiet = sitting.send(&json!({"op": "wait", "partner": partner, "timeout_seconds": 30}));
    sitting.poll();
    assert!(sitting.take(quiet).is_none());
    sitting.now += Duration::from_secs(31);
    sitting.poll();
    let nothing = sitting.take(quiet).expect("The period ran out");
    assert_eq!(nothing["status"], "nothing_yet", "{nothing}");
    assert_eq!(nothing["ok"], true);

    let held = sitting.send(&json!({"op": "wait", "partner": partner, "timeout_seconds": 30}));
    sitting.human("confirm-mulligan"); // Jacqueline
    sitting.human("confirm-mulligan"); // Tony
    sitting.rules.settled = false;
    sitting.poll();
    assert!(
        sitting.take(held).is_none(),
        "The table is still presenting"
    );
    sitting.rules.settled = true;
    sitting.poll();
    let decision = sitting.take(held).expect("Patrice's mulligan is pending");
    assert_eq!(decision["status"], "decision", "{decision}");
    assert_eq!(decision["view"]["decision"]["yours"], true);
    assert!(option_ids(&decision["view"]).contains(&"confirm-mulligan"));

    // Still pending: the next wait answers at once.
    let again = sitting.ask(json!({"op": "wait", "partner": partner}));
    assert_eq!(again["status"], "decision", "{again}");
}

/// Ticket #205, The view: the partner sees its own hand and public information;
/// other hands, deck order and the encounter deck are absent; it lists only its
/// own options, names who else is deciding, and names players only by their
/// investigators.
#[test]
fn the_view_holds_only_its_players_hidden_information_and_names_others_by_investigator() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let joined = sitting.ask(json!({"op": "join"}));
    let partner = joined["partner"].as_str().unwrap().to_owned();
    // Full text on first sight: Patrice's opening hand.
    let opening = sitting.rules.session.observe().investigators[Patrice.index()]
        .hand
        .clone();
    for card in &opening {
        let text = joined["view"]["cards"][card.code.as_str()].as_str();
        assert!(
            text.is_some_and(|text| text.contains(&card.title)),
            "{}",
            card.title
        );
    }
    let observed = sitting.ask(json!({"op": "observe", "partner": partner}));
    assert_eq!(observed["status"], "observed", "{observed}");
    let view = &observed["view"];
    // Only cards not yet sent, unless all are asked for again.
    assert!(
        view["cards"].get(opening[0].code.as_str()).is_none(),
        "{}",
        view["cards"]
    );
    let all = sitting.ask(json!({"op": "observe", "partner": partner, "all_cards": true}));
    assert!(all["view"]["cards"].get(opening[0].code.as_str()).is_some());
    assert_eq!(view["decision"]["yours"], false);
    assert_eq!(option_ids(view), Vec::<&str>::new());
    assert_eq!(
        view["decision"]["others_deciding"],
        json!(["Jacqueline Fine"])
    );

    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    let view = sitting.ask(json!({"op": "observe", "partner": partner}))["view"].clone();
    let observation = sitting.rules.session.observe();
    let hand = &observation.investigators[Patrice.index()].hand;
    assert_eq!(
        view["investigators"][1]["hand"].as_array().unwrap().len(),
        hand.len()
    );
    assert!(
        option_ids(&view)
            .iter()
            .all(|id| *id == "confirm-mulligan" || id.starts_with("mulligan:"))
    );
    for who in [Tony, Jacqueline] {
        let entry = &view["investigators"][who.index()];
        assert!(entry.get("hand").is_none(), "{entry}");
        assert_eq!(
            entry["hand_count"],
            observation.investigators[who.index()].hand.len()
        );
        for card in &observation.investigators[who.index()].hand {
            assert!(
                !names_card(&view, card.id, &card.code),
                "{} in {}'s hand is visible",
                card.title,
                who.name()
            );
        }
    }
    let text = view.to_string();
    for absent in ["\"deck\"", "encounter_deck\"", "human", "agent", "partner"] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
}

/// Watcher from Another Dimension (06017): "Hidden. ... Secretly add this enemy to
/// your hand." Only Patrice's player sees it.
#[test]
fn a_watcher_hidden_in_patrices_hand_is_absent_from_another_partners_view() {
    let session = Session::with_hostile_fixture(HostileFixture::Retry(RetryFixture::HandWatcher));
    let observation = session.observe();
    assert!(
        observation.play.as_ref().unwrap().players[Patrice.index()]
            .hand
            .iter()
            .any(|card| card.code == "06017"),
        "The fixture holds Watcher in Patrice's hand"
    );
    let mut sitting = Sitting::new(session, &[Patrice, Jacqueline]);
    let patrices = sitting.join(&["patrice"]);
    let jacquelines = sitting.join(&["jacqueline"]);
    let theirs = sitting.ask(json!({"op": "observe", "partner": jacquelines}));
    assert!(!theirs.to_string().contains("06017"), "{theirs}");
    assert!(!theirs.to_string().contains("Watcher"), "{theirs}");
    let hers = sitting.ask(json!({"op": "observe", "partner": patrices}));
    assert!(
        hers["view"]["investigators"][1]["hand"]
            .to_string()
            .contains("06017")
    );
}

/// Whether any object in `value` names this physical card.
fn names_card(value: &Value, id: u16, code: &str) -> bool {
    match value {
        Value::Object(fields) => {
            (fields.get("id") == Some(&json!(id)) && fields.get("code") == Some(&json!(code)))
                || fields.values().any(|field| names_card(field, id, code))
        }
        Value::Array(items) => items.iter().any(|item| names_card(item, id, code)),
        _ => false,
    }
}

/// Ticket #205, Choosing: a stale, unlisted or unowned choice is refused with its
/// reason and the current view, and the partner can go on asking.
#[test]
fn a_stale_unlisted_or_unowned_choice_is_refused_with_its_reason_and_the_view() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    let revision = sitting.revision();
    let unowned = sitting.ask(
        json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}),
    );
    assert_eq!(unowned["status"], "refused", "{unowned}");
    assert!(
        unowned["error"]
            .as_str()
            .unwrap()
            .contains("belongs to Jacqueline Fine"),
        "{unowned}"
    );
    assert_eq!(unowned["view"]["revision"], revision);

    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    let stale = sitting.ask(
        json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}),
    );
    assert_eq!(stale["status"], "refused", "{stale}");
    assert!(
        stale["error"].as_str().unwrap().contains("stale"),
        "{stale}"
    );
    assert_eq!(stale["view"]["revision"], sitting.revision());

    let current = sitting.revision();
    let unlisted = sitting.ask(
        json!({"op": "choose", "partner": partner, "revision": current, "id": "turn:patrice"}),
    );
    assert_eq!(unlisted["status"], "refused", "{unlisted}");
    assert!(
        unlisted["error"]
            .as_str()
            .unwrap()
            .contains("not available"),
        "{unlisted}"
    );
    let undo =
        sitting.ask(json!({"op": "choose", "partner": partner, "revision": current, "id": "undo"}));
    assert_eq!(undo["status"], "refused", "{undo}");
    assert_eq!(sitting.revision(), current, "Nothing was committed");

    let observed = sitting.ask(json!({"op": "observe", "partner": partner}));
    assert_eq!(observed["status"], "observed");
}

/// Ticket #205, Choosing: an accepted choice is applied only once the table
/// settles, commits and autosaves through the human's commit path, and the history
/// names its investigator.
#[test]
fn an_accepted_choice_waits_for_the_table_then_commits_autosaves_and_is_attributed() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    let revision = sitting.revision();
    sitting.rules.settled = false;
    let chosen = sitting.send(&json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}));
    sitting.poll();
    assert!(sitting.take(chosen).is_none());
    assert_eq!(
        sitting.revision(),
        revision,
        "Not applied while the table presents"
    );

    sitting.rules.settled = true;
    sitting.poll();
    let answer = sitting.take(chosen).expect("Applied once settled");
    assert_eq!(answer["status"], "chosen", "{answer}");
    assert_eq!(answer["view"]["revision"], revision + 1);
    let saved = continuity::load_session(&sitting.rules.save).unwrap();
    assert_eq!(saved.observe().revision, revision + 1, "Autosaved");
    assert_eq!(
        sitting.rules.attributed,
        [("confirm-mulligan".to_owned(), Patrice)],
        "The table attributes the decision to Patrice"
    );
}

/// Ticket #205, Limits: a partner cannot save, restart, change settings, leave the
/// table for the human or operate the table's controls. Every such request is
/// refused, changes nothing, and leaves the partner able to go on.
#[test]
fn a_partner_can_only_play_and_every_other_request_is_refused() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    let revision = sitting.revision();
    let saved = std::fs::read(&sitting.rules.save).unwrap();
    for op in [
        "save",
        "reopen",
        "restart",
        "quit",
        "settings",
        "activate",
        "press",
        "settle",
        "capture",
        "take_over",
    ] {
        let answer = sitting.ask(json!({"op": op, "partner": partner}));
        assert_eq!(answer["status"], "refused", "{op}: {answer}");
        assert!(
            answer["error"]
                .as_str()
                .unwrap()
                .contains("Not a partner request"),
            "{answer}"
        );
        assert!(
            answer["view"].is_object(),
            "A known partner's refusal carries its view"
        );
    }
    let garbled = sitting.ask(json!({"op": "observe"}));
    assert_eq!(garbled["status"], "refused");
    let stranger = sitting.ask(json!({"op": "observe", "partner": "0123456789ab"}));
    assert_eq!(stranger["status"], "refused");
    assert_eq!(sitting.revision(), revision);
    assert_eq!(std::fs::read(&sitting.rules.save).unwrap(), saved);
    assert_eq!(
        sitting.ask(json!({"op": "observe", "partner": partner}))["status"],
        "observed"
    );
}

/// Ticket #205, Recovery: Take over in the middle of a partner's decision returns
/// the investigator to the human at once; the partner's later choice is refused as
/// no longer its own, and its wait answers that its control changed.
#[test]
fn take_over_mid_decision_refuses_the_partners_later_choice_as_no_longer_its_own() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    let revision = sitting.revision();
    let waiting = sitting.send(&json!({"op": "wait", "partner": partner}));
    assert_eq!(sitting.take(waiting).unwrap()["status"], "decision");
    sitting.rules.settled = false;
    let chosen = sitting.send(&json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}));
    let waiting = sitting.send(&json!({"op": "wait", "partner": partner}));

    sitting.host.set_selected(Patrice, false);
    assert_eq!(
        sitting.host.control().controller(Patrice),
        Controller::Human
    );
    assert!(
        sitting
            .host
            .control()
            .human_may(&sitting.rules.session, "confirm-mulligan")
    );
    sitting.poll();
    let changed = sitting.take(waiting).expect("Control changed");
    assert_eq!(changed["status"], "control_changed", "{changed}");
    assert_eq!(changed["ok"], true, "A change of control is not a refusal");
    assert_eq!(changed["investigators"], json!([]));
    sitting.rules.settled = true;
    sitting.poll();
    let refused = sitting.take(chosen).expect("Answered once settled");
    assert_eq!(refused["status"], "refused", "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("Patrice Hathaway is no longer yours"),
        "{refused}"
    );
    assert_eq!(sitting.revision(), revision);
}

/// Ticket #205, Recovery and Waiting: turning partners off returns every
/// investigator to the human and ends every partner session; a held wait answers
/// that the table closed for it.
#[test]
fn turning_partners_off_returns_every_investigator_and_ends_held_waits() {
    // Jacqueline leads and is the human's: her mulligan comes first.
    let mut sitting = Sitting::after_lead(&[Tony, Patrice]);
    let partner = sitting.join(&[]);
    let waiting = sitting.send(&json!({"op": "wait", "partner": partner}));
    assert!(sitting.take(waiting).is_none());
    let answers = sitting.host.set_enabled(false);
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].0, waiting);
    assert_eq!(answers[0].1["status"], "closed", "{}", answers[0].1);
    assert!(sitting.host.control().solo());
    let after = sitting.ask(json!({"op": "observe", "partner": partner}));
    assert_eq!(after["status"], "refused", "{after}");
}

/// The tracer's end-to-end path at the protocol: the partner takes Patrice's
/// mulligan and her whole turn, waiting between decisions, while the human makes
/// the lead's group choice of who takes the next turn.
#[test]
fn a_partner_plays_patrices_mulligan_and_turn_between_waits() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    let decision = sitting.ask(json!({"op": "wait", "partner": partner}));
    let revision = decision["view"]["revision"].as_u64().unwrap();
    let chosen = sitting.ask(
        json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}),
    );
    assert_eq!(chosen["status"], "chosen", "{chosen}");

    // Who takes the next turn is a group choice, the human's as the lead's player.
    let view = sitting.ask(json!({"op": "observe", "partner": partner}))["view"].clone();
    assert_eq!(view["decision"]["yours"], false, "{}", view["decision"]);
    assert_eq!(view["decision"]["group_choice"]["arbiter"], "jacqueline");
    sitting.human("turn:patrice");

    let mut actions = 0;
    let waiting = loop {
        let waiting =
            sitting.send(&json!({"op": "wait", "partner": partner, "timeout_seconds": 60}));
        let Some(answer) = sitting.take(waiting) else {
            break waiting;
        };
        if answer["status"] == "group_choice" {
            // Her turn has ended: she hears the human's next-turn choice open.
            assert_eq!(answer["view"]["decision"]["yours"], false, "{answer}");
            continue;
        }
        assert_eq!(answer["status"], "decision", "{answer}");
        let view = &answer["view"];
        let ids = option_ids(view);
        assert!(!ids.is_empty(), "{view}");
        let id = ["action:ordinary:resource", "turn:end"]
            .into_iter()
            .find(|wanted| ids.contains(wanted))
            .unwrap_or_else(|| panic!("{ids:?}"));
        let revision = view["revision"].as_u64().unwrap();
        let chosen = sitting
            .ask(json!({"op": "choose", "partner": partner, "revision": revision, "id": id}));
        assert_eq!(chosen["status"], "chosen", "{chosen}");
        actions += 1;
        assert!(actions < 10, "Patrice's turn ends");
    };
    // Her turn is over: the wait is held until the next decision of hers.
    sitting.now += Duration::from_secs(61);
    sitting.poll();
    assert_eq!(sitting.take(waiting).unwrap()["status"], "nothing_yet");
    let observation = sitting.rules.session.observe();
    let play = observation.play.unwrap();
    assert!(play.players[Patrice.index()].presentation.turn_finished);
    assert!(
        sitting
            .rules
            .attributed
            .iter()
            .all(|(_, by)| *by == Patrice)
    );
    assert!(
        sitting.rules.attributed.len() >= 4,
        "{:?}",
        sitting.rules.attributed
    );
}
