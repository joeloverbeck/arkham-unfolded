//! Seam: the agent-partner protocol (#205) over the Bevy-free rules session, driven
//! as a partner drives it: one JSON request at a time, asserting the answers. The
//! table here is the rules session with a save file and a settled flag the test
//! sets; the window's own table is tested in `gathering_tabletop::partners`.
//! Expectations come from ticket #205, the play contract's "Agent partners" policy
//! and the Gathering's setup order (Rules Reference p.13: player order starts with
//! the lead), not from the implementation.

use arkham_unfolded_experiment::{
    gathering::{
        DrawFixture, EtherealFixture, HarmFixture, HostileFixture, Investigator, LifecycleFixture,
        Observation, RetryFixture, Session, SpellEventFixture, TokenFixture, continuity,
        inspection::{self, Sight},
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

/// Review fix E1 (stories 28-29: the lead's player decides who takes the next turn
/// "after hearing the table"). A plain wait answers the arbiter's own pending
/// decision at once, so it cannot listen. A listening wait is held while that
/// decision waits, and answers only another event (here another player's
/// message) or its timeout; the decision is still the arbiter's afterwards.
#[test]
fn a_listening_wait_lets_the_lead_hear_the_table_before_its_group_choice() {
    let mut sitting = Sitting::new(Session::with_seed(89), &[Patrice, Jacqueline]);
    sitting.human("lead:jacqueline");
    let lead = sitting.join(&["jacqueline"]);
    let other = sitting.join(&["patrice"]);
    sitting.partner(&lead, "confirm-mulligan");
    sitting.human("confirm-mulligan");
    sitting.partner(&other, "confirm-mulligan");
    let opened = sitting.ask(json!({"op": "wait", "partner": lead}));
    assert_eq!(opened["status"], "group_choice", "{opened}");

    let listening = sitting
        .send(&json!({"op": "wait", "partner": lead, "listen": true, "timeout_seconds": 60}));
    sitting.poll();
    assert!(
        sitting.take(listening).is_none(),
        "A listening wait does not answer its own decision"
    );
    assert!(
        !sitting
            .host
            .due(&sitting.rules.session, true, true, sitting.now),
        "Nothing is due while only its own decision waits"
    );
    let said = sitting.ask(json!({"op": "say", "partner": other, "text": "I can take the Attic."}));
    assert_eq!(said["status"], "said", "{said}");
    sitting.poll();
    let heard = sitting.take(listening).expect("Another player spoke");
    assert_eq!(heard["status"], "table_talk", "{heard}");
    assert_eq!(heard["view"]["decision"]["yours"], true);
    assert_eq!(
        heard["view"]["table_talk"][0]["text"],
        "I can take the Attic."
    );

    let quiet =
        sitting.send(&json!({"op": "wait", "partner": lead, "listen": true, "timeout_seconds": 1}));
    sitting.now += Duration::from_secs(2);
    sitting.poll();
    let ended = sitting.take(quiet).expect("Its period ran out");
    assert_eq!(ended["status"], "nothing_yet", "{ended}");
    let decision = sitting.ask(json!({"op": "wait", "partner": lead}));
    assert_eq!(decision["status"], "decision", "{decision}");
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
    let reason = late["error"].as_str().unwrap();
    assert!(
        reason.contains("No investigator is open to partners"),
        "{reason}"
    );
    // Review fix E3: a session that has not joined cannot speak at the table, so
    // the refusal sends it nowhere; it says there is nothing for it to play.
    assert!(!reason.contains("Ask at the table"), "{reason}");
    assert!(reason.contains("nothing for you to play"), "{reason}");
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
    // Only cards not yet sent, unless the complete view is asked for again.
    assert!(
        view["cards"].get(opening[0].code.as_str()).is_none(),
        "{}",
        view["cards"]
    );
    let all = sitting.ask(json!({"op": "observe", "partner": partner, "full": true}));
    assert!(all["view"]["cards"].get(opening[0].code.as_str()).is_some());
    assert_eq!(view["decision"]["yours"], false);
    assert_eq!(option_ids(view), Vec::<&str>::new());
    assert_eq!(
        view["decision"]["others_deciding"],
        json!(["Jacqueline Fine"])
    );

    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    // #206: later views report changes; the complete view holds the whole board.
    let view =
        sitting.ask(json!({"op": "observe", "partner": partner, "full": true}))["view"].clone();
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
    let theirs = sitting.ask(json!({"op": "observe", "partner": jacquelines, "full": true}));
    assert!(!theirs.to_string().contains("06017"), "{theirs}");
    assert!(!theirs.to_string().contains("Watcher"), "{theirs}");
    let hers = sitting.ask(json!({"op": "observe", "partner": patrices, "full": true}));
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

/// Every code of a card in `who`'s hand at `observation`.
fn hand_codes(observation: &Observation, who: Investigator) -> Vec<String> {
    observation
        .play
        .as_ref()
        .map_or_else(
            || &observation.investigators[who.index()].hand,
            |play| &play.players[who.index()].hand,
        )
        .iter()
        .map(|card| card.code.clone())
        .collect()
}

/// A Gathering in play after every mulligan, with the partner joined for `partners`.
fn in_play(partners: &[Investigator]) -> (Sitting, String) {
    let mut sitting = Sitting::after_lead(&[]);
    for _ in Investigator::ALL {
        sitting.human("confirm-mulligan");
    }
    for who in partners {
        sitting.host.set_selected(*who, true);
    }
    let partner = sitting.join(&[]);
    (sitting, partner)
}

/// Ticket #206, Inspect: a partner inspects any card it can see and receives the
/// text the table's Inspect shows; a card in another player's hand or only in a deck
/// is refused exactly as a code that names no card, so the refusal reveals nothing.
#[test]
fn inspect_gives_a_visible_cards_table_text_and_refuses_a_hidden_card_as_no_card() {
    let (mut sitting, partner) = in_play(&[Patrice]);
    let observation = sitting.rules.session.observe();
    let table = |code: &str| inspection::table_card(&observation, code, Sight::Table);
    let mine = hand_codes(&observation, Patrice);
    let visible: Vec<String> = mine
        .iter()
        .cloned()
        .chain(
            [
                "06003", "06005", "60401", "01104", "01105", "01108", "01111",
            ]
            .map(String::from),
        )
        .collect();
    for code in &visible {
        let answer = sitting.ask(json!({"op": "inspect", "partner": partner, "card": code}));
        assert_eq!(answer["status"], "inspected", "{code}: {answer}");
        assert_eq!(answer["ok"], true);
        assert_eq!(
            answer["text"].as_str(),
            table(code).as_deref(),
            "{code}: the table's own Inspect text"
        );
    }
    // Every investigator card carries its asset slots, as at the table.
    let tony = sitting.ask(json!({"op": "inspect", "partner": partner, "card": "06003"}));
    assert!(
        tony["text"].as_str().unwrap().contains("Asset slots"),
        "{tony}"
    );

    // At the start of play only hands and permanents hold player cards.
    let permanent = |code: &str| {
        observation
            .investigators
            .iter()
            .any(|setup| setup.permanents.iter().any(|card| card.code == code))
    };
    let theirs: Vec<String> = hand_codes(&observation, Tony)
        .into_iter()
        .chain(hand_codes(&observation, Jacqueline))
        .filter(|code| !mine.contains(code) && !permanent(code))
        .collect();
    assert!(!theirs.is_empty(), "Some cards are only in other hands");
    let in_hands: Vec<String> = Investigator::ALL
        .into_iter()
        .flat_map(|who| hand_codes(&observation, who))
        .collect();
    let in_deck: Vec<String> = observation.source.decks[Tony.index()]
        .inventory
        .iter()
        .map(|entry| entry.code.clone())
        .filter(|code| !in_hands.contains(code) && !permanent(code))
        .collect();
    assert!(
        !in_deck.is_empty(),
        "Some of Tony's cards are only in his deck"
    );
    let nothing = sitting.ask(json!({"op": "inspect", "partner": partner, "card": "99999"}));
    assert_eq!(nothing["status"], "refused", "{nothing}");
    for code in theirs.iter().chain(&in_deck) {
        let answer = sitting.ask(json!({"op": "inspect", "partner": partner, "card": code}));
        assert_eq!(answer["status"], "refused", "{code}: {answer}");
        assert_eq!(
            answer["error"], nothing["error"],
            "{code}: refused as a code that names no card"
        );
        let title = arkham_unfolded_experiment::gathering::card_title(code).unwrap();
        assert!(!answer.to_string().contains(title), "{code}: {answer}");
    }
}

/// Ticket #206, Inspect: the text in force, as the table shows it. Dark Prophecy
/// (04032) is on taboo list 10 and Katarina Sojka (11069) has Legacy FAQ 2.5 errata;
/// Dark Prophecy is in Jacqueline's hand, so only her player may read it, while
/// Katarina is in play for everyone.
#[test]
fn inspect_shows_the_text_in_force_with_its_taboo_and_errata_notes() {
    let session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::VoiceRaGuidance));
    let observation = session.observe();
    let mut sitting = Sitting::new(session, &[Patrice, Jacqueline]);
    let patrices = sitting.join(&["patrice"]);
    let jacquelines = sitting.join(&["jacqueline"]);
    let katarina = sitting.ask(json!({"op": "inspect", "partner": patrices, "card": "11069"}));
    assert_eq!(katarina["status"], "inspected", "{katarina}");
    assert!(
        katarina["text"]
            .as_str()
            .unwrap()
            .contains("Legacy FAQ 2.5 errata"),
        "{katarina}"
    );
    let hidden = sitting.ask(json!({"op": "inspect", "partner": patrices, "card": "04032"}));
    assert_eq!(hidden["status"], "refused", "{hidden}");
    let prophecy = sitting.ask(json!({"op": "inspect", "partner": jacquelines, "card": "04032"}));
    assert_eq!(prophecy["status"], "inspected", "{prophecy}");
    let text = prophecy["text"].as_str().unwrap();
    assert!(text.contains("taboo list 10"), "{text}");
    assert_eq!(
        Some(text),
        inspection::table_card(&observation, "04032", Sight::Table).as_deref()
    );
}

/// Ticket #206, Hidden information: a Watcher from Another Dimension hidden in
/// Patrice's hand cannot be inspected by another partner; her own player can.
#[test]
fn a_watcher_hidden_in_patrices_hand_cannot_be_inspected_by_another_partner() {
    let session = Session::with_hostile_fixture(HostileFixture::Retry(RetryFixture::HandWatcher));
    let mut sitting = Sitting::new(session, &[Patrice, Jacqueline]);
    let patrices = sitting.join(&["patrice"]);
    let jacquelines = sitting.join(&["jacqueline"]);
    let theirs = sitting.ask(json!({"op": "inspect", "partner": jacquelines, "card": "06017"}));
    assert_eq!(theirs["status"], "refused", "{theirs}");
    assert!(!theirs.to_string().contains("Watcher"), "{theirs}");
    let hers = sitting.ask(json!({"op": "inspect", "partner": patrices, "card": "06017"}));
    assert_eq!(hers["status"], "inspected", "{hers}");
    assert!(
        hers["text"]
            .as_str()
            .unwrap()
            .contains("Secretly add this enemy to your hand")
    );
}

/// Whether Watcher from Another Dimension is in Patrice's hand.
fn watcher_in_patrices_hand(session: &Session) -> bool {
    hand_codes(&session.observe(), Patrice).contains(&"06017".to_owned())
}

/// Ticket #206, Hidden information: in every prepared state where Watcher from
/// Another Dimension is hidden in Patrice's hand ("Secretly add this enemy to your
/// hand"; FAQ 2.5 Hidden rules: it counts toward her hand size), and through the
/// decisions that follow while it stays there, nothing Jacqueline's partner
/// receives names it: not its views, their changes or card text, nor inspection.
/// The table makes each following decision, preferring an option not yet taken:
/// the first listed, or in a second walk one that acts on Watcher. The walks fight
/// Watcher from Patrice's hand and take its attack when her deck runs out.
#[test]
fn a_watcher_hidden_in_patrices_hand_never_reaches_another_partner() {
    let fixtures = [
        HostileFixture::Retry(RetryFixture::HandWatcher),
        HostileFixture::Ethereal(EtherealFixture::Watcher),
        HostileFixture::Ethereal(EtherealFixture::WatcherAttack),
        HostileFixture::SpellEvents(SpellEventFixture::RazorWatcher),
        HostileFixture::Draws(DrawFixture::PatriceExhaustion),
        HostileFixture::Harm(HarmFixture::IdolAttack),
    ];
    for (fixture, on_watcher) in fixtures
        .into_iter()
        .flat_map(|fixture| [(fixture, false), (fixture, true)])
    {
        let session = Session::with_hostile_fixture(fixture);
        assert!(watcher_in_patrices_hand(&session), "{fixture:?}");
        let mut sitting = Sitting::new(session, &[Patrice, Jacqueline]);
        sitting.join(&["patrice"]);
        let jacquelines = sitting.ask(json!({"op": "join", "investigators": ["jacqueline"]}));
        let partner = jacquelines["partner"].as_str().unwrap().to_owned();
        let mut answers = vec![jacquelines];
        let mut step = 0;
        let mut taken: Vec<String> = Vec::new();
        loop {
            answers.push(sitting.ask(json!({"op": "observe", "partner": partner})));
            answers.push(sitting.ask(json!({"op": "observe", "partner": partner, "full": true})));
            answers
                .push(sitting.ask(json!({"op": "inspect", "partner": partner, "card": "06017"})));
            for answer in &answers {
                let text = answer.to_string();
                assert!(
                    !text.contains("06017") && !text.contains("Watcher"),
                    "{fixture:?} after {step} decisions: {answer}"
                );
            }
            answers.clear();
            let observation = sitting.rules.session.observe();
            // An option not taken before, so that toggles do not loop.
            let fresh = || {
                observation
                    .choices
                    .iter()
                    .filter(|choice| choice.id != "undo" && !taken.contains(&choice.id))
            };
            let Some(choice) = fresh()
                .find(|choice| on_watcher && choice.label.contains("Watcher"))
                .or_else(|| fresh().next())
                .or_else(|| {
                    observation
                        .choices
                        .iter()
                        .find(|choice| choice.id != "undo")
                })
            else {
                break;
            };
            taken.push(choice.id.clone());
            continuity::commit_choice(
                &sitting.rules.save,
                &mut sitting.rules.session,
                observation.revision,
                &choice.id,
            )
            .unwrap();
            step += 1;
            if step == 16 || !watcher_in_patrices_hand(&sitting.rules.session) {
                break;
            }
        }
    }
}

/// The entries of a view's changes at `at`.
fn changes_at<'a>(view: &'a Value, at: &str) -> Vec<&'a Value> {
    view["changes"]
        .as_array()
        .unwrap_or_else(|| panic!("no changes: {view}"))
        .iter()
        .filter(|change| change["at"] == at)
        .collect()
}

/// Ticket #206, Changes: a partner's first view is complete; each later view keeps
/// the decision whole but reports the rest of the table only as what changed since
/// the partner's last view, never as history.
#[test]
fn after_the_complete_first_view_each_view_reports_only_what_changed() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let joined = sitting.ask(json!({"op": "join"}));
    let partner = joined["partner"].as_str().unwrap().to_owned();
    let complete = &joined["view"];
    assert!(complete["investigators"].is_array(), "{complete}");
    assert!(complete.get("changes").is_none(), "{complete}");

    let quiet = sitting.ask(json!({"op": "observe", "partner": partner}));
    let view = &quiet["view"];
    assert_eq!(view["changes"], json!([]), "Nothing happened: {view}");
    assert!(view.get("investigators").is_none(), "{view}");
    assert_eq!(view["revision"], complete["revision"]);
    assert_eq!(
        view["decision"], complete["decision"],
        "The decision is always whole"
    );

    // Jacqueline and Tony keep their opening hands.
    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    let later = sitting.ask(json!({"op": "observe", "partner": partner}));
    let view = &later["view"];
    for who in ["jacqueline", "tony"] {
        assert_eq!(
            changes_at(view, &format!("investigators.{who}.mulligan_complete")),
            [
                &json!({"at": format!("investigators.{who}.mulligan_complete"), "was": false, "now": true})
            ],
            "{view}"
        );
    }
    assert!(view.get("investigators").is_none(), "{view}");
    assert!(!later.to_string().contains("history"), "{later}");
    assert!(
        view["decision"]["yours"].as_bool().unwrap(),
        "Patrice's mulligan"
    );
}

/// Ticket #206, Changes: a partner's own action is reported as the changes it made.
/// A resource action gains one resource (Rules Reference p.17, Resource Action)
/// and is one of the three actions of the investigator's turn (p.24).
#[test]
fn a_partners_resource_action_is_answered_with_one_more_resource_and_one_action_fewer() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    let mulligan = sitting.ask(json!({"op": "wait", "partner": partner}));
    let revision = mulligan["view"]["revision"].as_u64().unwrap();
    sitting.ask(
        json!({"op": "choose", "partner": partner, "revision": revision, "id": "confirm-mulligan"}),
    );
    sitting.human("turn:patrice");
    let turn = sitting.ask(json!({"op": "wait", "partner": partner}));
    assert_eq!(turn["status"], "decision", "{turn}");
    let before = sitting.rules.session.observe().play.unwrap().players[Patrice.index()].resources;
    let revision = turn["view"]["revision"].as_u64().unwrap();
    let chosen = sitting.ask(json!({"op": "choose", "partner": partner, "revision": revision, "id": "action:ordinary:resource"}));
    assert_eq!(chosen["status"], "chosen", "{chosen}");
    let view = &chosen["view"];
    assert_eq!(
        changes_at(view, "investigators.patrice.resources"),
        [&json!({"at": "investigators.patrice.resources", "was": before, "now": before + 1})],
        "{view}"
    );
    assert_eq!(
        changes_at(view, "investigators.patrice.actions_remaining"),
        [&json!({"at": "investigators.patrice.actions_remaining", "was": 3, "now": 2})],
        "{view}"
    );
}

/// Ticket #206, Changes: `observe` with `full` gives the complete view and every
/// visible card's text again, after which views report changes from it; a partner
/// that leaves and joins again is a new session and starts with a complete view.
#[test]
fn a_full_observe_and_a_new_session_each_give_the_complete_view_once() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let joined = sitting.ask(json!({"op": "join"}));
    let partner = joined["partner"].as_str().unwrap().to_owned();
    let complete = joined["view"].clone();
    sitting.ask(json!({"op": "observe", "partner": partner}));
    let full = sitting.ask(json!({"op": "observe", "partner": partner, "full": true}));
    assert_eq!(full["view"], complete, "Nothing changed since the join");
    let next = sitting.ask(json!({"op": "observe", "partner": partner}));
    assert_eq!(next["view"]["changes"], json!([]), "{next}");
    assert_eq!(next["view"]["cards"], json!({}), "{next}");

    sitting.ask(json!({"op": "leave", "partner": partner}));
    let again = sitting.ask(json!({"op": "join"}));
    assert_eq!(again["status"], "joined", "{again}");
    assert_eq!(again["view"], complete, "A new session starts complete");
}

/// Ticket #206, Hidden information: another player's mulligan changes no card a
/// partner can see. Neither Jacqueline's opening hand nor her replacements appear in
/// the changes Patrice's partner receives (Rules Reference p.15, Mulligan: a player
/// mulligans the cards they do not wish to keep in their own starting hand).
#[test]
fn another_players_mulligan_reaches_a_partners_changes_without_naming_a_card() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    let opening = sitting.rules.session.observe().investigators[Jacqueline.index()]
        .hand
        .clone();
    let mut answers = Vec::new();
    for card in opening.iter().take(2) {
        sitting.human(&format!("mulligan:{}", card.id));
        answers.push(sitting.ask(json!({"op": "observe", "partner": partner})));
    }
    sitting.human("confirm-mulligan");
    answers.push(sitting.ask(json!({"op": "observe", "partner": partner})));
    let replaced = sitting.rules.session.observe().investigators[Jacqueline.index()]
        .hand
        .clone();
    assert_ne!(opening, replaced, "Jacqueline replaced two cards");
    for answer in &answers {
        for card in opening.iter().chain(&replaced) {
            assert!(
                !names_card(answer, card.id, &card.code),
                "{} in {answer}",
                card.title
            );
            assert!(
                !answer.to_string().contains(&card.title),
                "{} in {answer}",
                card.title
            );
        }
    }
    let last = &answers[2]["view"];
    assert_eq!(
        last["changes"],
        json!([{"at": "investigators.jacqueline.mulligan_complete", "was": false, "now": true}]),
        "{last}"
    );
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

/// The Undo a partner's view offers, if any.
fn undo_offered(view: &Value) -> Option<&Value> {
    view["decision"].get("undo")
}

/// Ticket #207, Undo (play contract, "Undo and pacing"): with partners enabled each
/// player may undo only their own latest decision, before anything follows it, and
/// never across revealed information; Take over makes the investigator's latest
/// decision the human's to undo; without partners, Undo is the human's as before.
#[test]
fn each_player_may_undo_only_their_own_latest_decision() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan"); // Jacqueline
    sitting.human("confirm-mulligan"); // Tony
    sitting.partner(&partner, "confirm-mulligan");
    sitting.human("turn:patrice");
    // The human's group choice came last.
    assert!(sitting.listed().len() > 1 && sitting.human_may("undo"));
    assert_eq!(undo_offered(&sitting.view(&partner)), None);
    let revision = sitting.revision();
    let refused = sitting
        .ask(json!({"op": "choose", "partner": partner, "revision": revision, "id": "undo"}));
    assert_eq!(refused["status"], "refused", "{refused}");
    assert!(
        refused["error"]
            .as_str()
            .unwrap()
            .contains("your own latest decision"),
        "{refused}"
    );
    assert_eq!(sitting.revision(), revision);

    // Patrice's resource came last: only her partner may undo it.
    let before = sitting.rules.session.observe().play.unwrap().players[Patrice.index()].resources;
    sitting.partner(&partner, "action:ordinary:resource");
    assert!(!sitting.human_may("undo"));
    let view = sitting.view(&partner);
    let offered = undo_offered(&view).unwrap_or_else(|| panic!("{}", view["decision"]));
    assert_eq!(offered["id"], "undo");
    assert_eq!(offered["investigator"], "patrice");
    sitting.partner(&partner, "undo");
    let play = sitting.rules.session.observe().play.unwrap();
    assert_eq!(play.players[Patrice.index()].resources, before);
    assert_eq!(
        sitting.rules.attributed.last(),
        Some(&("undo".to_owned(), Patrice))
    );
    // The human's group choice is the latest again.
    assert!(sitting.human_may("undo"));
    assert_eq!(undo_offered(&sitting.view(&partner)), None);

    // A draw reveals her next card: nobody may undo it.
    sitting.partner(&partner, "action:ordinary:draw");
    assert!(!sitting.listed().is_empty());
    assert!(!sitting.human_may("undo"));
    assert_eq!(undo_offered(&sitting.view(&partner)), None);

    // Take over gives the human Patrice's latest decision; handed back to partners
    // and unclaimed, it is nobody's to undo; without partners, it is the human's.
    sitting.partner(&partner, "action:ordinary:resource");
    assert!(!sitting.human_may("undo"));
    sitting.host.set_selected(Patrice, false);
    assert!(sitting.human_may("undo"));
    sitting.host.set_selected(Patrice, true);
    assert!(!sitting.human_may("undo"));
    let _ = sitting.host.set_enabled(false);
    assert!(sitting.human_may("undo"));
}

/// Ticket #207, Undo: a later decision by another player puts a partner's own
/// decision out of its reach; the human's turn choice now comes last.
#[test]
fn a_partner_cannot_undo_its_decision_once_another_players_follows_it() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    sitting.partner(&partner, "confirm-mulligan");
    sitting.human("turn:patrice");
    sitting.partner(&partner, "turn:end");
    assert!(undo_offered(&sitting.view(&partner)).is_some());
    assert!(!sitting.human_may("undo"));
    sitting.human("turn:tony");
    assert_eq!(undo_offered(&sitting.view(&partner)), None);
    assert!(sitting.human_may("undo"));
}

/// Ticket #207, Undo: whose decision came last is derived, not saved. After the
/// attempt is reopened and partners are enabled again, Patrice's latest decision is
/// still only her player's to undo.
#[test]
fn a_reopened_attempt_still_knows_whose_latest_decision_undo_would_revert() {
    let mut sitting = Sitting::after_lead(&[Patrice]);
    let partner = sitting.join(&[]);
    sitting.human("confirm-mulligan");
    sitting.human("confirm-mulligan");
    sitting.partner(&partner, "confirm-mulligan");
    sitting.human("turn:patrice");
    sitting.partner(&partner, "action:ordinary:resource");

    let reopened = continuity::load_session(&sitting.rules.save).unwrap();
    assert_eq!(reopened, sitting.rules.session);
    let mut sitting = Sitting::new(reopened, &[Patrice]);
    let partner = sitting.join(&[]);
    assert!(!sitting.human_may("undo"));
    assert!(undo_offered(&sitting.view(&partner)).is_some());
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

/// Ticket #206: the sizes of the answers a partner receives over a real Gathering
/// session. The save names its seed and decisions; the session is replayed, and a
/// partner playing one investigator waits for and makes each of that
/// investigator's decisions through the protocol while every other decision is
/// committed for the table. Prints, in bytes of the JSON line, each partner's join
/// answer, the median, 90th percentile and largest of its later answers, and the
/// complete view a partner joining at the save's position would receive.
#[test]
#[ignore = "measures partner answers over ARKHAM_PARTNER_VIEW_SAVE, a real Gathering session save"]
fn partner_answer_sizes_over_a_real_session() {
    let path = std::env::var("ARKHAM_PARTNER_VIEW_SAVE")
        .expect("ARKHAM_PARTNER_VIEW_SAVE names a Gathering session save");
    let saved: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let payload = &saved["payload"];
    let seed = payload["seed"].as_u64().unwrap();
    let decisions: Vec<String> = serde_json::from_value(payload["decisions"].clone()).unwrap();
    for who in Investigator::ALL {
        let mut sitting = Sitting::new(Session::with_seed(seed), &[who]);
        let joined = sitting.ask(json!({"op": "join"}));
        let partner = joined["partner"].as_str().unwrap().to_owned();
        let mut sizes = Vec::new();
        for id in &decisions {
            let control = sitting.host.control();
            let revision = sitting.revision();
            if control.human_may(&sitting.rules.session, id) {
                continuity::commit_choice(
                    &sitting.rules.save,
                    &mut sitting.rules.session,
                    revision,
                    id,
                )
                .unwrap_or_else(|error| panic!("{id}: {error}"));
                continue;
            }
            // What its wait would answer: the view at its decision.
            let observed = sitting.ask(json!({"op": "observe", "partner": partner}));
            assert_eq!(observed["status"], "observed", "{id}: {observed}");
            sizes.push(observed.to_string().len());
            let chosen = sitting
                .ask(json!({"op": "choose", "partner": partner, "revision": revision, "id": id}));
            assert_eq!(chosen["status"], "chosen", "{id}: {chosen}");
            sizes.push(chosen.to_string().len());
        }
        let rejoined = sitting.ask(json!({"op": "observe", "partner": partner, "full": true}));
        let total: usize = sizes.iter().sum();
        let mut sorted = sizes.clone();
        sorted.sort_unstable();
        let at = |share: usize| sorted.get(sorted.len() * share / 100).copied().unwrap_or(0);
        println!(
            "{}: join {} bytes; {} later answers, median {} bytes, 90th percentile {} bytes, largest {} bytes, total {} bytes; complete view at the save's position {} bytes",
            who.name(),
            joined.to_string().len(),
            sizes.len(),
            at(50),
            at(90),
            sorted.last().copied().unwrap_or(0),
            total,
            rejoined.to_string().len(),
        );
    }
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
