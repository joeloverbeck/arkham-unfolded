//! Seam: the agent-partner protocol over the Bevy-free rules session, for shared
//! decisions (#208). A partner sends JSON requests as it would over the socket;
//! the human acts through [`Host::human`], as the table's controls do. Setup edits
//! only place selected cards, clues and investigators where a witness needs them.
//! Expectations come from ticket #208, the play contract's "Shared decisions", and
//! the rules: Rules Reference p.26 ST.2 (each other investigator at the tester's
//! location may commit one card), p.7 "Clues" (any or all investigators contribute
//! to a group payment), p.3 "Triggered Abilities" and p.22 (a [fast] ability in
//! any player window), and FAQ 2.5 "Signature Cards".

use super::*;
use crate::gathering::Investigator::{Jacqueline, Patrice, Tony};
use std::collections::BTreeMap;

/// The rules session as a table, settled whenever the test says so; it records
/// whom each commit was attributed to.
struct Rules {
    session: Session,
    settled: bool,
    attributed: Vec<(String, Investigator)>,
}

impl Table for Rules {
    fn session(&self) -> &Session {
        &self.session
    }

    fn settled(&self) -> bool {
        self.settled
    }

    fn commit(&mut self, revision: u64, id: &str, by: Investigator) -> Result<(), String> {
        self.session
            .choose(revision, id)
            .map_err(|error| error.to_string())?;
        self.attributed.push((id.to_owned(), by));
        Ok(())
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
    fn new(session: Session, partners: &[Investigator]) -> Self {
        let mut host = Host::new();
        for who in partners {
            host.set_selected(*who, true);
        }
        assert!(host.set_enabled(true).is_empty());
        Self {
            host,
            rules: Rules {
                session,
                settled: true,
                attributed: Vec::new(),
            },
            now: Instant::now(),
            next: 0,
            answers: BTreeMap::new(),
        }
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

    /// One partner session playing `investigators` (all open ones when empty).
    fn join(&mut self, investigators: &[&str]) -> String {
        let answer = self.ask(json!({"op": "join", "investigators": investigators}));
        assert_eq!(answer["status"], "joined", "{answer}");
        answer["partner"].as_str().unwrap().to_owned()
    }

    /// The human's click, as the table's controls make it.
    fn human_try(&mut self, id: &str) -> Result<HumanChoice, String> {
        let session = &self.rules.session;
        assert!(
            self.host.control().human_may(session, id),
            "{id} is not the human's"
        );
        let revision = session.revision();
        let choice = self.host.human(session, revision, id)?;
        if choice == HumanChoice::Commit {
            self.rules
                .session
                .choose(revision, id)
                .map_err(|error| error.to_string())?;
        }
        self.poll();
        Ok(choice)
    }

    fn human(&mut self, id: &str) -> HumanChoice {
        self.human_try(id)
            .unwrap_or_else(|error| panic!("{id}: {error}"))
    }

    fn revision(&self) -> u64 {
        self.rules.session.revision()
    }

    fn offered(&self) -> Vec<String> {
        self.rules
            .session
            .observe()
            .choices
            .into_iter()
            .map(|choice| choice.id)
            .collect()
    }

    fn choose(&mut self, partner: &str, id: &str) -> Value {
        let revision = self.revision();
        self.ask(json!({"op": "choose", "partner": partner, "revision": revision, "id": id}))
    }

    fn ready(&mut self, partner: &str) -> Value {
        let revision = self.revision();
        self.ask(json!({"op": "ready", "partner": partner, "revision": revision}))
    }

    fn wait(&mut self, partner: &str) -> u32 {
        self.send(&json!({"op": "wait", "partner": partner, "timeout_seconds": 60}))
    }

    /// The pending shared decision of this kind, as the table shows it.
    fn shared(&self, kind: fn(&SharedKind) -> bool) -> SharedState {
        self.host
            .shared(&self.rules.session)
            .into_iter()
            .find(|state| kind(&state.kind))
            .unwrap_or_else(|| panic!("No such shared decision: {:?}", self.offered()))
    }
}

fn commitment(kind: &SharedKind) -> bool {
    *kind == SharedKind::Commitment
}

fn payment(kind: &SharedKind) -> bool {
    matches!(kind, SharedKind::Payment { .. })
}

fn window(kind: &SharedKind) -> bool {
    *kind == SharedKind::Window
}

fn option_ids(view: &Value) -> Vec<String> {
    view["decision"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["id"].as_str().unwrap().to_owned())
        .collect()
}

fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe().revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

/// The first turn of `of`, after a lead of Jacqueline and three kept hands. Every
/// investigator starts in the Study (The Gathering's setup).
fn turn(of: Investigator) -> Session {
    let mut session = Session::with_seed(89);
    for id in [
        "lead:jacqueline",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
    ] {
        choose(&mut session, id);
    }
    choose(&mut session, &format!("turn:{}", of.id()));
    session
}

/// Tony investigates the Study with Unexpected Courage in his hand (two wild
/// icons, 01093) and Patrice's Violin in hers (one wild icon, 06016): each fits
/// any test. Jacqueline holds no card, so she takes no part. Returns the session
/// at the commitment window and the two cards.
fn tonys_commitment_window() -> (Session, u16, u16) {
    let mut session = turn(Tony);
    session.empty_hand_for_test(Jacqueline);
    let tonys = session.hand_for_test(Tony, "01093");
    let patrices = session.hand_for_test(Patrice, "06016");
    choose(&mut session, "action:ordinary:investigate");
    while session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    (session, tonys, patrices)
}

/// Ticket #208, Commitments with two players: the window opens to both players at
/// once; each sets only its own investigators' cards (RR p.26 ST.2); each sees the
/// other's pending card; a change by either clears the other's readiness and wakes
/// it with a shared-decision event; the test goes ahead only when both are ready
/// on the same cards, and the decision names whose readiness is outstanding.
#[test]
fn a_commitment_shared_by_two_players_goes_ahead_only_when_both_are_ready_on_the_same_cards() {
    let (session, tonys, patrices) = tonys_commitment_window();
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    let opened = sitting.ask(json!({"op": "wait", "partner": partner}));
    assert_eq!(
        opened["status"], "decision",
        "Opened to her at once: {opened}"
    );
    assert_eq!(opened["view"]["decision"]["yours"], true);
    let ids = option_ids(&opened["view"]);
    assert!(ids.contains(&format!("commit:{patrices}")), "{ids:?}");
    assert!(
        !ids.contains(&format!("commit:{tonys}")),
        "Only her own cards: {ids:?}"
    );
    assert!(ids.contains(&"commit:confirm".to_owned()), "{ids:?}");
    let human = sitting.host.control();
    assert!(!human.human_may(&sitting.rules.session, &format!("commit:{patrices}")));

    // The human's pending card is visible to the partner: her next wait answers
    // with the change.
    assert_eq!(
        sitting.human(&format!("commit:{tonys}")),
        HumanChoice::Commit
    );
    let woken = sitting.ask(json!({"op": "wait", "partner": partner}));
    assert_eq!(woken["status"], "shared", "{woken}");
    let pending = &woken["view"]["test"]["others_pending"];
    assert_eq!(pending[0]["id"], tonys, "{pending}");
    assert_eq!(pending[0]["investigator"], "tony");
    assert_eq!(pending[0]["title"], "Unexpected Courage");

    // Her pending card is in the session the table shows the human.
    let chosen = sitting.choose(&partner, &format!("commit:{patrices}"));
    assert_eq!(chosen["status"], "chosen", "{chosen}");
    let selected = sitting
        .rules
        .session
        .observe()
        .play
        .unwrap()
        .test
        .unwrap()
        .selected;
    assert!(selected.contains(&tonys) && selected.contains(&patrices));
    let ready = sitting.ready(&partner);
    assert_eq!(ready["status"], "ready", "{ready}");
    let state = sitting.shared(commitment);
    assert_eq!(
        state.outstanding(),
        [Tony],
        "Tony's readiness is outstanding"
    );
    assert_eq!(
        ready["view"]["shared"][0]["waiting_for"],
        json!(["Tony Morgan"])
    );

    // The human's change clears her readiness and wakes her.
    let waiting = sitting.wait(&partner);
    sitting.poll();
    assert!(sitting.take(waiting).is_none(), "Nothing changed for her");
    sitting.human(&format!("commit:{tonys}"));
    let woken = sitting.take(waiting).expect("Woken by the change");
    assert_eq!(woken["status"], "shared", "{woken}");
    let shared = &woken["view"]["shared"][0];
    assert_eq!(shared["you_are_ready"], false, "{shared}");
    assert_eq!(shared["cleared_by"], json!(["Tony Morgan"]));

    // The human's Confirm is his readiness: the test waits for hers.
    sitting.human(&format!("commit:{tonys}"));
    assert_eq!(sitting.human("commit:confirm"), HumanChoice::Ready);
    assert!(sitting.offered().contains(&"commit:confirm".to_owned()));
    assert_eq!(sitting.shared(commitment).outstanding(), [Patrice]);

    // Her change clears his; both ready on the same cards completes it.
    let toggled = sitting.choose(&partner, &format!("commit:{patrices}"));
    assert_eq!(toggled["status"], "chosen", "{toggled}");
    let state = sitting.shared(commitment);
    let human = state.of(&Controller::Human).unwrap();
    assert!(!human.ready);
    assert_eq!(human.cleared_by.as_deref(), Some(&[Patrice][..]));
    sitting.choose(&partner, &format!("commit:{patrices}"));
    sitting.ready(&partner);
    assert!(
        sitting.offered().contains(&"commit:confirm".to_owned()),
        "Only she is ready"
    );
    sitting.human("commit:confirm");
    let test = sitting.rules.session.observe().play.unwrap().test.unwrap();
    let committed: Vec<u16> = test.committed.iter().map(|card| card.card.id).collect();
    assert!(
        committed.contains(&tonys) && committed.contains(&patrices),
        "{committed:?}"
    );
    assert_eq!(
        sitting.rules.attributed.last(),
        Some(&("commit:confirm".to_owned(), Tony))
    );
}

/// Ticket #208, Reopen bound: a player who has been ready may change its part only
/// a small fixed number of times; at the limit a change is refused with the
/// current view, readiness is still accepted, and the decision still completes.
#[test]
fn a_player_reopens_a_shared_commitment_only_up_to_the_bound_then_may_only_be_ready() {
    const { assert!(REOPENINGS >= 1 && REOPENINGS <= 5) };
    let (session, tonys, patrices) = tonys_commitment_window();
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    sitting.ask(json!({"op": "observe", "partner": partner}));
    let toggle = format!("commit:{patrices}");
    sitting.ready(&partner);
    for reopened in 1..=REOPENINGS {
        let changed = sitting.choose(&partner, &toggle);
        assert_eq!(
            changed["status"], "chosen",
            "Reopening {reopened}: {changed}"
        );
        let left = &changed["view"]["shared"][0]["reopenings_left"];
        assert_eq!(*left, REOPENINGS - reopened, "{changed}");
        assert_eq!(sitting.ready(&partner)["status"], "ready");
    }
    let revision = sitting.revision();
    let refused = sitting.choose(&partner, &toggle);
    assert_eq!(refused["status"], "refused", "{refused}");
    assert!(
        refused["error"].as_str().unwrap().contains("reopened"),
        "{refused}"
    );
    assert_eq!(
        refused["view"]["revision"], revision,
        "With the current view"
    );
    assert_eq!(sitting.revision(), revision, "Nothing changed");
    assert_eq!(sitting.ready(&partner)["status"], "ready");

    // The human is bound alike once he has been ready.
    sitting.human("commit:confirm");
    sitting.ready(&partner);
    assert!(
        !sitting.offered().contains(&"commit:confirm".to_owned()),
        "Both were ready: the test went ahead"
    );
    let _ = tonys;
    let (session, tonys, _) = tonys_commitment_window();
    let mut sitting = Sitting::new(session, &[Patrice]);
    let toggle = format!("commit:{tonys}");
    sitting.join(&[]);
    sitting.human("commit:confirm");
    for _ in 0..REOPENINGS {
        sitting.human(&toggle);
        sitting.human("commit:confirm");
    }
    let refused = sitting.human_try(&toggle);
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.contains("reopened")),
        "{refused:?}"
    );
}

/// Ticket #208, Clue payments (RR p.7, Trapped 01108): each player sets only its
/// own investigators' clues; nothing is paid until the requirement is met and
/// every contributing player is ready on the same contributions.
#[test]
fn a_group_clue_payment_spends_once_the_requirement_is_met_and_every_contributor_is_ready() {
    let mut session = turn(Tony);
    session.place_for_test(Tony, 4, "01111");
    session.place_for_test(Patrice, 2, "01111");
    session.place_for_test(Jacqueline, 0, "01111");
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    assert!(
        sitting
            .offered()
            .contains(&"act:pay:<contributions>".to_owned())
    );
    // A payment no one has proposed does not wake her during Tony's turn.
    let waiting = sitting.wait(&partner);
    sitting.poll();
    assert!(sitting.take(waiting).is_none());

    // She proposes her part; it wakes no one by itself but is visible.
    let proposed = sitting.choose(&partner, "act:pay:0:1:0");
    assert_eq!(proposed["status"], "ready", "{proposed}");
    let contributions = &proposed["view"]["shared"][0]["contributions"];
    assert_eq!(
        contributions[Patrice.index()]["clues"],
        1,
        "{contributions}"
    );
    assert_eq!(contributions[Patrice.index()]["yours"], true);
    let state = sitting.shared(payment);
    assert_eq!(state.clues[Patrice], 1);
    assert_eq!(state.outstanding(), [Tony]);

    // She cannot set Tony's clues, and the human cannot set hers.
    let theirs = sitting.choose(&partner, "act:pay:4:2:0");
    assert_eq!(theirs["status"], "refused", "{theirs}");
    assert!(
        sitting.human_try("act:pay:4:2:0").is_err(),
        "Patrice's pending part is 1"
    );

    // His part clears her readiness and wakes her; short of the requirement,
    // both ready pays nothing.
    let waiting = sitting.wait(&partner);
    assert_eq!(sitting.human("act:pay:4:1:0"), HumanChoice::Ready);
    let woken = sitting.take(waiting).expect("Woken by his part");
    assert_eq!(woken["status"], "shared", "{woken}");
    assert_eq!(sitting.ready(&partner)["status"], "ready");
    let clues = |sitting: &Sitting| -> Vec<u8> {
        sitting
            .rules
            .session
            .observe()
            .play
            .unwrap()
            .players
            .iter()
            .map(|player| player.clues)
            .collect()
    };
    assert_eq!(clues(&sitting), [4, 2, 0], "Five of six clues pay nothing");

    // Her change meets the requirement and clears his readiness; his readiness
    // on the same contributions pays.
    assert_eq!(sitting.choose(&partner, "act:pay:4:2:0")["status"], "ready");
    assert_eq!(clues(&sitting), [4, 2, 0], "He is not ready on her change");
    sitting.human("act:pay:4:2:0");
    assert_eq!(clues(&sitting), [0, 0, 0]);
    assert_eq!(
        sitting
            .rules
            .attributed
            .iter()
            .find(|(id, _)| id.starts_with("act:pay:"))
            .map(|(id, _)| id.as_str()),
        Some("act:pay:4:2:0")
    );
}

/// Ticket #208, Fast-card windows (RR p.3, p.22): a window closes only after every
/// player with an option has passed in succession, whichever order they pass in.
#[test]
fn a_window_closes_only_after_every_player_with_an_option_has_passed() {
    let mut session = turn(Tony);
    session.asset_for_test(Patrice, "06016");
    session.hand_for_test(Jacqueline, "04199");
    choose(&mut session, "action:ordinary:investigate");
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    let opened = sitting.ask(json!({"op": "wait", "partner": partner}));
    assert_eq!(opened["status"], "decision", "{opened}");
    let ids = option_ids(&opened["view"]);
    assert!(ids.iter().any(|id| id.starts_with("violin:")), "{ids:?}");
    assert!(ids.contains(&"window:pass".to_owned()), "{ids:?}");
    assert!(!ids.iter().any(|id| id.starts_with("premonition:")));

    // The human passes first: the window waits for her.
    assert_eq!(sitting.human("window:pass"), HumanChoice::Ready);
    assert!(sitting.offered().contains(&"window:pass".to_owned()));
    assert_eq!(sitting.shared(window).outstanding(), [Patrice]);
    let passed = sitting.choose(&partner, "window:pass");
    assert_eq!(passed["status"], "ready", "{passed}");
    assert!(
        !sitting.offered().contains(&"window:pass".to_owned()),
        "Both passed: the window closed"
    );
    assert!(
        sitting
            .offered()
            .iter()
            .any(|id| id == "commit:confirm" || id.starts_with("commit:")),
        "{:?}",
        sitting.offered()
    );
}

/// Ticket #208: an action by any player restarts the succession. A pass made
/// before another player's action no longer counts; the passing player is woken
/// to pass again.
#[test]
fn an_action_in_a_window_restarts_the_succession_of_passes() {
    let mut session = turn(Tony);
    session.asset_for_test(Patrice, "06016");
    let premonition = session.hand_for_test(Jacqueline, "04199");
    choose(&mut session, "action:ordinary:investigate");
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    sitting.ask(json!({"op": "observe", "partner": partner}));
    assert_eq!(sitting.ready(&partner)["status"], "ready");
    let waiting = sitting.wait(&partner);
    sitting.poll();
    assert!(sitting.take(waiting).is_none(), "She has passed");
    assert_eq!(
        sitting.human(&format!("premonition:{premonition}")),
        HumanChoice::Commit
    );
    while sitting.offered().iter().any(|id| id.starts_with("tokens:")) {
        let id = sitting.offered()[0].clone();
        sitting.human(&id);
    }
    let woken = sitting.take(waiting).expect("The action woke her");
    assert_eq!(woken["status"], "decision", "{woken}");
    assert!(
        sitting.offered().contains(&"window:pass".to_owned()),
        "Her earlier pass no longer closes the window"
    );
    let passed = sitting.choose(&partner, "window:pass");
    assert!(
        matches!(passed["status"].as_str(), Some("ready" | "chosen")),
        "{passed}"
    );
    assert!(
        !sitting.offered().contains(&"window:pass".to_owned()),
        "{:?}",
        sitting.offered()
    );
}

/// Joel's co-play session 1 (#202): a player window in the human's test waits on
/// a partner only when that partner's investigator has an option in it. Patrice's
/// Violin is a [fast] ability usable in any player window (06016); Tony holds no
/// fast option, so his partner is not woken.
#[test]
fn a_window_wakes_only_the_partners_whose_investigators_have_an_option_in_it() {
    let mut session = turn(Jacqueline);
    session.asset_for_test(Patrice, "06016");
    session.empty_hand_for_test(Tony);
    session.empty_hand_for_test(Jacqueline);
    session.hand_for_test(Patrice, "01093");
    choose(&mut session, "action:ordinary:investigate");
    let mut sitting = Sitting::new(session, &[Tony, Patrice]);
    let tonys = sitting.join(&["tony"]);
    let patrices = sitting.join(&["patrice"]);
    let tony_waits = sitting.wait(&tonys);
    let patrice_waits = sitting.wait(&patrices);
    sitting.poll();
    assert!(sitting.take(tony_waits).is_none(), "Tony has no option");
    let woken = sitting
        .take(patrice_waits)
        .expect("Patrice may use her Violin");
    assert_eq!(woken["status"], "decision", "{woken}");
    assert_eq!(
        sitting.host.awaited(&sitting.rules.session),
        [(Controller::Partner(patrices.clone()), Some(Patrice))]
    );
}

/// Ticket #208, Commit limits unchanged (RR p.26 ST.2; FAQ 2.5 "Signature Cards":
/// an investigator cannot commit another investigator's signature cards): another
/// investigator at the location commits one card; no player commits another
/// player's investigators' cards, Patrice's Violin (her signature, 06016) included.
#[test]
fn co_play_adds_no_commitment_the_rules_forbid() {
    let mut session = turn(Tony);
    let tonys = session.hand_for_test(Tony, "01093");
    let first = session.hand_for_test(Patrice, "01093");
    let violin = session.hand_for_test(Patrice, "06016");
    choose(&mut session, "action:ordinary:investigate");
    while session
        .observe()
        .choices
        .iter()
        .any(|choice| choice.id == "window:pass")
    {
        choose(&mut session, "window:pass");
    }
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    let observed = sitting.ask(json!({"op": "observe", "partner": partner}));
    let ids = option_ids(&observed["view"]);
    assert!(ids.contains(&format!("commit:{violin}")), "{ids:?}");
    let theirs = sitting.choose(&partner, &format!("commit:{tonys}"));
    assert_eq!(theirs["status"], "refused", "{theirs}");
    assert!(
        !sitting
            .host
            .control()
            .human_may(&sitting.rules.session, &format!("commit:{violin}")),
        "The human cannot commit her signature Violin"
    );
    assert_eq!(
        sitting.choose(&partner, &format!("commit:{first}"))["status"],
        "chosen"
    );
    let after = sitting.ask(json!({"op": "observe", "partner": partner}));
    let ids = option_ids(&after["view"]);
    assert!(
        !ids.contains(&format!("commit:{violin}")),
        "One card from another investigator: {ids:?}"
    );
    let second = sitting.choose(&partner, &format!("commit:{violin}"));
    assert_eq!(second["status"], "refused", "{second}");
}

/// Ticket #208, Concurrency: a submission against an older revision, or readiness
/// on contributions the player has not seen, is refused with the current view;
/// nothing is silently rebased.
#[test]
fn a_stale_submission_in_a_shared_decision_is_refused_with_the_current_view() {
    let (session, tonys, patrices) = tonys_commitment_window();
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    let seen = sitting.ask(json!({"op": "observe", "partner": partner}));
    let old = seen["view"]["revision"].as_u64().unwrap();
    sitting.human(&format!("commit:{tonys}"));
    for request in [
        json!({"op": "ready", "partner": partner, "revision": old}),
        json!({"op": "choose", "partner": partner, "revision": old, "id": "commit:confirm"}),
        json!({"op": "choose", "partner": partner, "revision": old, "id": format!("commit:{patrices}")}),
    ] {
        let refused = sitting.ask(request.clone());
        assert_eq!(refused["status"], "refused", "{request}: {refused}");
        assert!(
            refused["error"].as_str().unwrap().contains("stale"),
            "{refused}"
        );
        assert_eq!(refused["view"]["revision"], sitting.revision());
    }
    assert!(!sitting.shared(commitment).players[1].ready);

    // A payment's contribution changes without a new revision: readiness after
    // a change the partner has not seen is refused too.
    let mut session = turn(Tony);
    session.place_for_test(Tony, 4, "01111");
    session.place_for_test(Patrice, 2, "01111");
    let mut sitting = Sitting::new(session, &[Patrice]);
    let partner = sitting.join(&[]);
    sitting.ask(json!({"op": "observe", "partner": partner}));
    sitting.human("act:pay:4:0:0");
    let refused = sitting.ready(&partner);
    assert_eq!(refused["status"], "refused", "{refused}");
    assert!(
        refused["error"].as_str().unwrap().contains("changed"),
        "{refused}"
    );
    assert_eq!(
        sitting.ready(&partner)["status"],
        "ready",
        "Once it has seen it"
    );
}

/// Review fix A1: the library, not the table, computes the human's options, with
/// one rule for every listed option. Play contract, Shared decisions: in a group
/// clue payment each player decides only their own investigators' part, and The
/// Barrier's round-end decline is that payment's other outcome (01109, RR p.7). The
/// human may pay or decline exactly when one of the human's investigators can
/// contribute; Undo is never among the options of a decision.
#[test]
fn the_human_may_decline_the_barrier_only_when_taking_part_in_its_payment() {
    let ids = |sitting: &Sitting| -> Vec<String> {
        sitting
            .host
            .control()
            .human_choices(&sitting.rules.session)
            .into_iter()
            .map(|choice| choice.id)
            .collect()
    };
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
    let listed = crate::gathering::GroupCluePayment::is_listed;

    // Every investigator is in the Hallway with clues: Tony's and Jacqueline's
    // player takes part beside Patrice's partner.
    let mut sitting = Sitting::new(session.clone(), &[Patrice]);
    sitting.join(&[]);
    let human = ids(&sitting);
    assert!(
        human.iter().any(|id| id == "round-end:decline-act"),
        "{human:?}"
    );
    assert!(human.iter().any(|id| listed(id)), "{human:?}");

    // Tony waits in the Cellar: only the partners' investigators, in the Hallway
    // with the nine clues The Barrier requires, can contribute.
    let mut session = session;
    session.place_for_test(Tony, 3, "01114");
    session.place_for_test(Patrice, 5, "01112");
    session.place_for_test(Jacqueline, 4, "01112");
    let mut sitting = Sitting::new(session, &[Patrice, Jacqueline]);
    sitting.join(&["patrice"]);
    sitting.join(&["jacqueline"]);
    let human = ids(&sitting);
    assert!(
        !human
            .iter()
            .any(|id| id == "round-end:decline-act" || listed(id)),
        "{human:?}"
    );
    for id in ["round-end:decline-act", "act:pay:0:5:4"] {
        assert!(
            !sitting.host.control().human_may(&sitting.rules.session, id),
            "{id}"
        );
    }
    assert!(!human.iter().any(|id| id == "undo"), "{human:?}");
}
