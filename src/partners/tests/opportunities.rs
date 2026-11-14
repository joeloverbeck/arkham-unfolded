//! Seam: co-play's strict windows as a property of seeded play (#218 item 1). The
//! rules session's ownership of each listed option is the oracle; the partner
//! protocol and the human's controls, as in the tests above, are what is checked.
//! Each seeded play-through of The Gathering picks among the listed options at
//! random, and every assignment of the three investigators to the human and one,
//! two or three partners takes each choice through the players it belongs to.
//!
//! Expectations come from the play contract's "Shared decisions" (Joel's strict
//! windows, F1 and S1 of #202: whenever the pending decision lists an option of a
//! player other than the deciding investigator's, the deciding player's next
//! choice is held and applied only once every such player has passed there; a
//! player with no option there is never waited on; Undo and a player's part of a
//! shared decision are not held; an act's clue payment makes no one a
//! participant; a participant's option used instead releases the held choice
//! unapplied), from Rules Reference p.3 "Triggered Abilities", p.22 and p.24
//! (player windows) and p.26 ST.2 (commitments), and from #202's session 2
//! (comment 6867031634), where a partner's `test:reveal` was applied beside the
//! human's Jacqueline Fine reaction. At every pending decision, for every
//! assignment:
//! - each player is offered exactly the listed options its investigators own,
//!   its pass when it holds an option in another player's decision, and Undo
//!   only when it controls every investigator of the decision Undo reverts; each
//!   partner that acts in a step sees the same options in its view;
//! - the table waits on someone, on no player without an option there, and on
//!   exactly the players with an option there, except that an act's clue payment
//!   waits on its contributors only once one proposes a part, and a commitment
//!   whose Confirm waits for the tester's own part (Watch This!'s wager) is
//!   checked only for waiting on no one without an option; each shared decision
//!   names only players with an option in it;
//! - a decision listing another investigator's option and no pass of its own is
//!   the player window before an action, a chaos-token reveal or an encounter
//!   card's revelation, and its decider is, independently of ownership, the
//!   active investigator, the revealer or the drawer;
//! - when the list holds another player's option and no pass of its own, the
//!   deciding player's choice is held, not applied, until each such player has
//!   passed, and then it alone is applied (an act's clue payment counts as no
//!   one's option here, and a commitment awaiting the tester's own part is not
//!   held); a participant's option used instead
//!   applies at once and releases the held choice unapplied;
//! - a shared decision that spans players completes only with the last
//!   participating player's readiness.
//!
//! At about one decision in six for each assignment, one player who does not own
//! the step's option submits it: a partner's request is refused, and the human's
//! controls do not allow it.

use super::*;
use crate::gathering::{GroupCluePayment, OwnedOption, PREDICTION_MARGIN};
use std::rc::Rc;

/// The seeds of the play-throughs: each a fresh Gathering, its lead, mulligans and
/// play picked at random among the listed options. Of seeds 0 to 399 under this
/// random play, these three reach each kind of list that holds a choice, the
/// player window before an action, a chaos-token reveal and an encounter card's
/// revelation, within [`DECISIONS`] decisions. They are brittle: a change to the
/// rules or to this test's picks moves the random play, and the test's guard that
/// each kind held a choice can then fail with no defect. Choose seeds again from a
/// scan of the reference play-through when it does.
const SEEDS: [u64; 3] = [126, 235, 102];
/// How many decisions each play-through takes at most. Each assignment takes
/// every one, so 3 seeds give 270 decisions checked at 14 assignments each.
const DECISIONS: usize = 90;

/// Every investigator in [`Investigator::ALL`] order: `None` for the human,
/// `Some(k)` for partner `k`.
type Assignment = [Option<usize>; 3];

/// Every assignment of the investigators to the human and partners, at least one,
/// each partner holding at least one investigator, up to the partners' names: seven
/// with one partner, six with two, and one with three, each joined by naming its
/// investigator.
fn assignments() -> Vec<Assignment> {
    let mut all = Vec::new();
    for code in 0..64_usize {
        let assignment: Assignment =
            [code % 4, code / 4 % 4, code / 16].map(|player| player.checked_sub(1));
        // Partners are named in the order they first hold an investigator.
        let mut named = 0;
        let canonical = assignment.iter().flatten().all(|partner| {
            if *partner == named {
                named += 1;
            }
            *partner < named
        });
        if canonical && named > 0 {
            all.push(assignment);
        }
    }
    all
}

/// SplitMix64: deterministic picks.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, count: usize) -> usize {
        let count = u64::try_from(count).expect("A count fits");
        usize::try_from(self.next() % count).expect("Below a usize")
    }

    fn one_in(&mut self, odds: u64) -> bool {
        self.next().is_multiple_of(odds)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for last in (1..items.len()).rev() {
            items.swap(last, self.below(last + 1));
        }
    }
}

/// What the property met, so a passing run shows it was not vacuous.
#[derive(Debug, Default)]
struct Tally {
    /// Pending decisions of the reference play-throughs.
    decisions: usize,
    /// Those listing other investigators' options and no pass, by list.
    opportunities: BTreeMap<&'static str, usize>,
    /// Pending decisions checked, over every assignment.
    checked: usize,
    /// Choices held for other players' passes and then applied, by list, and how
    /// many of them were the human's and how many a partner's.
    held: BTreeMap<&'static str, usize>,
    held_for_human: usize,
    held_for_partner: usize,
    /// Held choices released when a participant used its option instead.
    released: usize,
    /// Shared decisions completed once every participating player was ready.
    shared: usize,
    /// Submissions of another player's option, refused.
    refused: usize,
}

impl Tally {
    fn add(&mut self, other: Self) {
        self.decisions += other.decisions;
        self.checked += other.checked;
        for (list, count) in other.opportunities {
            *self.opportunities.entry(list).or_default() += count;
        }
        for (list, count) in other.held {
            *self.held.entry(list).or_default() += count;
        }
        self.held_for_human += other.held_for_human;
        self.held_for_partner += other.held_for_partner;
        self.released += other.released;
        self.shared += other.shared;
        self.refused += other.refused;
    }
}

/// The pending decision as the rules session lists and owns it.
struct Listed {
    revision: u64,
    owned: Vec<OwnedOption>,
    /// The investigator whose choice moves the decision past the other
    /// investigators' options, in a list that gives other investigators options
    /// and lists no pass of its own. `None` in a list of one investigator's
    /// options, a group choice, a window or payment that lists its own pass, or
    /// a commitment.
    decider: Option<Investigator>,
    /// Which list the decider's decision is, when it has a decider.
    list: Option<&'static str>,
    /// A commitment window whose Confirm waits for the tester's own part, such as
    /// Watch This!'s wager (03233): every option is a player's part of it.
    unconfirmed: bool,
}

/// The lists that give other investigators options beside the decider's and list
/// no pass of their own (the play contract's "Shared decisions").
const TURN: &str = "the player window before an action";
const REVEAL: &str = "a chaos-token reveal";
const REVELATION: &str = "a drawn encounter card's revelation";

impl Listed {
    fn of(session: &Session) -> Self {
        let owned = session.ownership();
        let named = |option: &OwnedOption| routing::investigators(&option.owner);
        // An act's clue payment is a group payment, never a participant's option.
        let options = || owned.iter().filter(|option| !routing::pays_act(&option.id));
        let closing: Vec<Investigator> = options()
            .filter(|option| matches!(option.owner, OptionOwner::Shared { .. }))
            .flat_map(named)
            .collect();
        let mut everyone: Vec<Investigator> = options().flat_map(named).collect();
        everyone.sort_by_key(|who| who.index());
        everyone.dedup();
        // RR p.26 ST.2: each investigator's commitment is their own part of it.
        let unconfirmed =
            closing.is_empty() && owned.iter().any(|option| option.id.starts_with("commit:"));
        let decider = if !closing.is_empty() {
            assert!(
                everyone.iter().all(|who| closing.contains(who)),
                "A listed pass is shared by every investigator with an option: {owned:?}"
            );
            None
        } else if unconfirmed {
            None
        } else if everyone.len() > 1 {
            let decider = session.opportunity_window().unwrap_or_else(|| {
                panic!("Several investigators' options and no pass, outside an opportunity window: {owned:?}")
            });
            assert!(everyone.contains(&decider), "{decider:?}: {owned:?}");
            Some(decider)
        } else {
            None
        };
        let list = decider.map(|decider| {
            let (list, independent) = Self::opportunity(session, &owned);
            assert_eq!(
                decider, independent,
                "{list}: the decider ownership names, and the one the table shows: {owned:?}"
            );
            list
        });
        Self {
            revision: session.revision(),
            owned,
            decider,
            list,
            unconfirmed,
        }
    }

    /// Which list an opportunity list is, and its decider as the observation shows
    /// it, independently of ownership: the drawer resolves a drawn encounter card
    /// (RR p.24 1.4), the revealer reveals its chaos tokens, and the active
    /// investigator takes the turn's actions (RR p.24 2.2.1).
    fn opportunity(session: &Session, owned: &[OwnedOption]) -> (&'static str, Investigator) {
        let lists = |prefix: &str| owned.iter().any(|option| option.id.starts_with(prefix));
        let play = session.observe().play.expect("Scenario play");
        if lists("revelation:")
            && let Some(revelation) = &play.pending_revelation
        {
            (REVELATION, revelation.drawn_by)
        } else if lists("test:reveal")
            && let Some(tokens) = &play.tokens
        {
            (REVEAL, tokens.revealer)
        } else if lists("turn:end")
            && let Some(actor) = play.actor
        {
            (TURN, actor)
        } else {
            panic!("An opportunity list of no known kind: {owned:?}")
        }
    }

    /// The players whose investigators the rules give `owner`.
    fn holders(owner: &OptionOwner, control: &Control) -> Vec<Controller> {
        if *owner == OptionOwner::Table {
            return vec![Controller::Human];
        }
        let mut players = Vec::new();
        for who in routing::investigators(owner) {
            let player = control.controller(who);
            if !players.contains(&player) {
                players.push(player);
            }
        }
        players
    }

    /// Whether `player` holds an option here, an act payment's part only when
    /// `payments`.
    fn holds(&self, control: &Control, player: &Controller, payments: bool) -> bool {
        self.owned.iter().any(|option| {
            (payments || !routing::pays_act(&option.id))
                && Self::holders(&option.owner, control).contains(player)
        })
    }

    /// The options only `player` holds, which it may use on its own.
    fn own(&self, control: &Control, player: &Controller) -> Vec<String> {
        self.owned
            .iter()
            .filter(|option| {
                !routing::pays_act(&option.id)
                    && !option.id.contains(PREDICTION_MARGIN)
                    && Self::holders(&option.owner, control) == [player.clone()]
            })
            .map(|option| option.id.clone())
            .collect()
    }

    /// The decider's player, in a list that has a decider.
    fn deciding(&self, control: &Control) -> Option<Controller> {
        self.decider.map(|decider| control.controller(decider))
    }

    /// The players other than the decider's with an option here: each must pass
    /// before the decider's player's choice applies.
    fn others(&self, control: &Control) -> Vec<Controller> {
        let Some(deciders) = self.deciding(control) else {
            return Vec::new();
        };
        let mut others = Vec::new();
        for option in self.owned.iter().filter(|o| !routing::pays_act(&o.id)) {
            for player in Self::holders(&option.owner, control) {
                if player != deciders && !others.contains(&player) {
                    others.push(player);
                }
            }
        }
        others
    }

    /// What `player` is offered: every listed option its investigators own, and
    /// its pass when it must pass before the decider's choice.
    fn expected(&self, control: &Control, player: &Controller) -> BTreeSet<String> {
        let mut expected: BTreeSet<String> = self
            .owned
            .iter()
            .filter(|option| Self::holders(&option.owner, control).contains(player))
            .map(|option| option.id.clone())
            .collect();
        if self.others(control).contains(player) {
            expected.insert(PASS_BEFORE.to_owned());
        }
        expected
    }

    /// Who the table waits on before anyone has acted here: every player with an
    /// option, an act payment's contributor only once a part is proposed. `None`
    /// while a commitment waits for the tester's part: the tester's player is
    /// waited on, and the others may set their parts meanwhile.
    fn awaited(&self, control: &Control) -> Option<BTreeSet<String>> {
        if self.unconfirmed {
            return None;
        }
        let mut players: BTreeSet<String> = BTreeSet::new();
        for payments in [false, true] {
            for option in &self.owned {
                if payments || !routing::pays_act(&option.id) {
                    players.extend(Self::holders(&option.owner, control).iter().map(name));
                }
            }
            if !players.is_empty() {
                break;
            }
        }
        Some(players)
    }
}

fn name(player: &Controller) -> String {
    match player {
        Controller::Human => "the human".into(),
        Controller::Partner(id) => format!("partner {id}"),
        Controller::Unclaimed => "no one".into(),
    }
}

fn names(players: &[Controller]) -> BTreeSet<String> {
    players.iter().map(name).collect()
}

/// One decision of the reference play-through, which every assignment takes.
struct Step {
    before: Listed,
    /// The option taken, its placeholder named, and its owner.
    intent: String,
    owner: OptionOwner,
    /// The session once the intent alone is applied, and its decision.
    applied: Rc<Session>,
    after: Listed,
}

/// The rules session as the table, repeating the tests' `Rules` table so it can
/// install the reference result below. Substituted infrastructure, in three ways:
/// - A commit of the step's intent installs the reference play-through's own
///   result of that `choose` from the same position, with Undo's owner already
///   derived, since deriving it replays the session from its start once per
///   revision and would otherwise be repeated at every assignment. Any other
///   commit is the session's own `choose`, and the step's final equality with the
///   reference catches it.
/// - It is always presented and settled, even straight after a commit. The
///   table's own sitting (`gathering_tabletop::partners`) clears both on every
///   commit until it has presented the consequences, so a partner's choice waits
///   there. Always settled is the worst case for applying a choice early, which is
///   what this test looks for.
/// - [`Play`] polls the host after every submission, where the table polls only
///   while [`Host::due`] says it has work. So the test cannot see a `due` that
///   wrongly answers false, which at the table would stall a held choice or a
///   shared decision rather than apply it early.
struct Lockstep {
    session: Session,
    foreseen: Option<(u64, String, Rc<Session>)>,
    attributed: Vec<(String, Investigator)>,
}

impl Lockstep {
    fn apply(&mut self, revision: u64, id: &str) -> Result<(), String> {
        if let Some((at, intent, applied)) = &self.foreseen
            && *at == revision
            && intent == id
            && self.session.revision() == revision
        {
            self.session = Session::clone(applied);
            return Ok(());
        }
        self.session
            .choose(revision, id)
            .map_err(|error| error.to_string())
    }
}

impl Table for Lockstep {
    fn session(&self) -> &Session {
        &self.session
    }

    fn settled(&self) -> bool {
        true
    }

    fn commit(&mut self, revision: u64, id: &str, by: Investigator) -> Result<(), String> {
        self.apply(revision, id)?;
        self.attributed.push((id.to_owned(), by));
        Ok(())
    }
}

/// What one player's submission did.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Applied,
    Held,
    Ready,
    Refused,
}

/// One assignment's table, through one seeded play-through. It repeats the
/// request plumbing of the tests' `Sitting`; making `Sitting` generic over its
/// table could serve both, and was left for a later change.
struct Play {
    host: Host<u32>,
    table: Lockstep,
    /// The human, then each partner.
    players: Vec<Controller>,
    dice: Dice,
    seed: u64,
    assignment: Assignment,
    now: Instant,
    next: u32,
    answers: BTreeMap<u32, Value>,
}

impl Play {
    fn new(seed: u64, assignment: Assignment, session: &Session) -> Self {
        let mut host = Host::new();
        for who in Investigator::ALL {
            host.set_selected(who, assignment[who.index()].is_some());
        }
        assert!(host.set_enabled(true).is_empty());
        let mut play = Self {
            host,
            table: Lockstep {
                session: session.clone(),
                foreseen: None,
                attributed: Vec::new(),
            },
            players: vec![Controller::Human],
            dice: Dice(seed.rotate_left(17) ^ assignment_code(assignment)),
            seed,
            assignment,
            now: Instant::now(),
            next: 0,
            answers: BTreeMap::new(),
        };
        for partner in 0..Investigator::ALL.len() {
            let theirs: Vec<&str> = Investigator::ALL
                .into_iter()
                .filter(|who| assignment[who.index()] == Some(partner))
                .map(Investigator::id)
                .collect();
            if !theirs.is_empty() {
                let joined = play.ask(&json!({"op": "join", "investigators": theirs}));
                assert_eq!(joined["status"], "joined", "{joined}");
                let id = joined["partner"].as_str().unwrap().to_owned();
                play.players.push(Controller::Partner(id));
            }
        }
        play
    }

    fn session(&self) -> &Session {
        &self.table.session
    }

    fn revision(&self) -> u64 {
        self.table.session.revision()
    }

    fn at(&self) -> String {
        format!(
            "seed {}, assignment {:?}, revision {}",
            self.seed,
            self.assignment,
            self.revision()
        )
    }

    fn send(&mut self, request: &Value) -> u32 {
        self.next += 1;
        let ticket = self.next;
        let answers = self
            .host
            .receive(&request.to_string(), ticket, &mut self.table, self.now);
        self.answers.extend(answers);
        ticket
    }

    fn ask(&mut self, request: &Value) -> Value {
        let ticket = self.send(request);
        self.answers
            .remove(&ticket)
            .unwrap_or_else(|| panic!("{}: {request} is held, not answered", self.at()))
    }

    /// The table's frame: the host answers and completes what is due.
    fn poll(&mut self) {
        let answers = self.host.poll(&mut self.table, self.now);
        self.answers.extend(answers);
    }

    /// The decision pending now: the step's, or once it is applied, the next.
    fn listed<'a>(&self, step: &'a Step) -> &'a Listed {
        if self.revision() == step.before.revision {
            &step.before
        } else {
            assert_eq!(self.revision(), step.after.revision, "{}", self.at());
            &step.after
        }
    }

    /// Every player is offered exactly what it owns, and Undo only when it
    /// controls every investigator of the decision Undo would revert.
    fn check_offers(&self, listed: &Listed) {
        let control = self.host.control();
        let undoer = self
            .session()
            .undo_owner()
            .map(|owner| Listed::holders(&owner, &control))
            .filter(|holders| holders.len() == 1)
            .and_then(|mut holders| holders.pop());
        for player in &self.players {
            let mut offered: BTreeSet<String> = control
                .choices_of(self.session(), player)
                .into_iter()
                .map(|choice| choice.id)
                .collect();
            let undo = offered.remove("undo");
            assert_eq!(
                offered,
                listed.expected(&control, player),
                "{}: what {} is offered",
                self.at(),
                name(player)
            );
            assert_eq!(
                undo,
                undoer.as_ref() == Some(player),
                "{}: Undo for {}",
                self.at(),
                name(player)
            );
        }
    }

    /// The table waits on `expected`, when given, and on no player without an
    /// option here, and each shared decision names only players with an option in
    /// it.
    fn check_waits(&self, listed: &Listed, expected: Option<&BTreeSet<String>>) {
        let control = self.host.control();
        let awaited = self.host.awaited(self.session());
        assert!(!awaited.is_empty(), "{}: no one is waited on", self.at());
        for (player, who) in &awaited {
            assert!(
                listed.holds(&control, player, true),
                "{}: {} is waited on with no option: {:?}",
                self.at(),
                name(player),
                listed.owned
            );
            if let Some(who) = who {
                assert_eq!(control.controller(*who), *player, "{}", self.at());
            }
        }
        if let Some(expected) = expected {
            let awaited: BTreeSet<String> =
                awaited.iter().map(|(player, _)| name(player)).collect();
            assert_eq!(
                awaited,
                *expected,
                "{}: who is waited on: {:?}",
                self.at(),
                listed.owned
            );
        }
        let others = listed.others(&control);
        let mut befores = 0;
        for state in self.host.shared(self.session()) {
            let before = matches!(state.kind, SharedKind::Before { .. });
            for readiness in &state.players {
                assert!(
                    listed.holds(&control, &readiness.player, !before),
                    "{}: {} takes part in {:?} with no option",
                    self.at(),
                    name(&readiness.player),
                    state.kind
                );
            }
            if before {
                befores += 1;
                let players: Vec<Controller> = state
                    .players
                    .iter()
                    .map(|readiness| readiness.player.clone())
                    .collect();
                assert_eq!(names(&players), names(&others), "{}", self.at());
            }
        }
        assert_eq!(
            befores,
            usize::from(!others.is_empty()),
            "{}: the passes before the decider's choice: {:?}",
            self.at(),
            listed.owned
        );
    }

    /// A partner's answer shows exactly the options that partner owns now, and
    /// says whether the table waits on it.
    fn check_view(&self, step: &Step, partner: &Controller, answer: &Value) {
        let listed = self.listed(step);
        let control = self.host.control();
        let view = &answer["view"];
        let shown: BTreeSet<String> = option_ids(view).into_iter().collect();
        assert_eq!(
            shown,
            listed.expected(&control, partner),
            "{}: {} is shown: {answer}",
            self.at(),
            name(partner)
        );
        let awaited = self.host.awaited(self.session());
        assert_eq!(
            view["decision"]["yours"],
            awaited.iter().any(|(player, _)| player == partner),
            "{}: {answer}",
            self.at()
        );
    }

    /// A partner's wait, answered at once: what became of its held choice, or the
    /// decision it is waited on in, whose view its readiness then rests on.
    fn wait(&mut self, step: &Step, partner: &Controller) -> Value {
        let Controller::Partner(id) = partner else {
            unreachable!("A partner")
        };
        let request = json!({"op": "wait", "partner": id, "timeout_seconds": 60});
        let ticket = self.send(&request);
        let answer = self.answers.remove(&ticket).unwrap_or_else(|| {
            panic!(
                "{}: {}'s wait is held. Intent {} by {:?}; listed {:?}; awaited {:?}; shared {:?}",
                self.at(),
                name(partner),
                step.intent,
                step.owner,
                self.listed(step).owned,
                self.host.awaited(self.session()),
                self.host.shared(self.session()),
            )
        });
        self.check_view(step, partner, &answer);
        answer
    }

    /// A partner's view before its readiness, which rests on it: its wait, answered
    /// at once while the table waits on it, or else, as before proposing the first
    /// part of a payment no one has proposed, its observation.
    fn wake(&mut self, step: &Step, player: &Controller) {
        let Controller::Partner(id) = player else {
            return;
        };
        let awaited = self.host.awaited(self.session());
        if !awaited.iter().any(|(waited, _)| waited == player) {
            let answer = self.ask(&json!({"op": "observe", "partner": id}));
            self.check_view(step, player, &answer);
            return;
        }
        let answer = self.wait(step, player);
        assert!(
            ["decision", "shared", "group_choice"]
                .contains(&answer["status"].as_str().unwrap_or_default()),
            "{}: {answer}",
            self.at()
        );
    }

    fn submit(&mut self, step: &Step, player: &Controller, id: &str) -> Outcome {
        let revision = self.revision();
        match player {
            Controller::Human => {
                assert!(
                    self.host.control().human_may(self.session(), id),
                    "{}: {id} is the human's",
                    self.at()
                );
                let outcome = match self.host.human(&self.table.session, revision, id) {
                    Ok(HumanChoice::Commit) => {
                        self.table
                            .apply(revision, id)
                            .unwrap_or_else(|error| panic!("{}: {id}: {error}", self.at()));
                        Outcome::Applied
                    }
                    Ok(HumanChoice::Ready) if self.host.human_held(self.session()) == Some(id) => {
                        Outcome::Held
                    }
                    Ok(HumanChoice::Ready) => Outcome::Ready,
                    Err(_) => Outcome::Refused,
                };
                self.poll();
                outcome
            }
            Controller::Partner(partner) => {
                let request =
                    json!({"op": "choose", "partner": partner, "revision": revision, "id": id});
                let answer = self.ask(&request);
                self.check_view(step, player, &answer);
                self.poll();
                match answer["status"].as_str() {
                    Some("chosen") => Outcome::Applied,
                    Some("held") => Outcome::Held,
                    Some("ready") => Outcome::Ready,
                    Some("refused") => Outcome::Refused,
                    _ => panic!("{}: {id}: {answer}", self.at()),
                }
            }
            Controller::Unclaimed => unreachable!("Every partner has joined"),
        }
    }

    /// Take the step's decision through the players it belongs to; it ends where
    /// the reference play-through does.
    fn take(&mut self, step: &Step, tally: &mut Tally) {
        assert_eq!(self.revision(), step.before.revision, "{}", self.at());
        tally.checked += 1;
        let control = self.host.control();
        self.check_offers(&step.before);
        self.check_waits(&step.before, step.before.awaited(&control).as_ref());
        if self.dice.one_in(6) {
            self.refusal(step, tally);
        }
        self.table.foreseen = Some((
            step.before.revision,
            step.intent.clone(),
            Rc::clone(&step.applied),
        ));
        let shared = Routing::of(self.session(), &control)
            .shared_of(&step.intent)
            .cloned();
        match shared {
            Some(shared) => self.together(step, &shared, tally),
            None => self.alone(step, tally),
        }
        assert!(
            *self.session() == *step.applied,
            "{}: the step applied {} and nothing else",
            self.at(),
            step.intent
        );
        self.table.foreseen = None;
    }

    /// A player who does not own the intent cannot submit it.
    fn refusal(&mut self, step: &Step, tally: &mut Tally) {
        let control = self.host.control();
        let holders = Listed::holders(&step.owner, &control);
        let Some(stranger) = self
            .players
            .iter()
            .find(|player| !holders.contains(player))
            .cloned()
        else {
            return;
        };
        let revision = self.revision();
        match &stranger {
            Controller::Human => assert!(
                !control.human_may(self.session(), &step.intent),
                "{}: the human may submit {}, owned by {:?}",
                self.at(),
                step.intent,
                step.owner
            ),
            Controller::Partner(_) => {
                let outcome = self.submit(step, &stranger, &step.intent);
                assert_eq!(
                    outcome,
                    Outcome::Refused,
                    "{}: {} submitted {}, owned by {:?}",
                    self.at(),
                    name(&stranger),
                    step.intent,
                    step.owner
                );
            }
            Controller::Unclaimed => unreachable!("Every partner has joined"),
        }
        assert_eq!(self.revision(), revision, "{}", self.at());
        tally.refused += 1;
    }

    /// A shared decision that spans players: each participating player marks
    /// itself ready, a payment's contributor on its part, and it completes only
    /// with the last of them.
    fn together(&mut self, step: &Step, shared: &Shared, tally: &mut Tally) {
        let revision = self.revision();
        let control = self.host.control();
        let target = GroupCluePayment::allocation_of(&step.intent);
        let mut players = shared.players(&control);
        self.dice.shuffle(&mut players);
        for _round in 0..3 {
            for player in &players {
                let state = self
                    .host
                    .shared(self.session())
                    .into_iter()
                    .find(|state| state.close == shared.close)
                    .unwrap_or_else(|| panic!("{}: {} is pending", self.at(), shared.close));
                let mine = state.of(player).expect("A participating player");
                if mine.ready {
                    continue;
                }
                let outstanding: Vec<Controller> = state
                    .players
                    .iter()
                    .filter(|readiness| !readiness.ready)
                    .map(|readiness| readiness.player.clone())
                    .collect();
                let part = match target {
                    Some(target) => {
                        let mut clues = state.clues;
                        for who in &mine.investigators {
                            clues[*who] = target[*who];
                        }
                        self.session()
                            .observe()
                            .group_clue_payment
                            .expect("A pending payment")
                            .part_for(clues)
                            .unwrap_or_else(|| panic!("{}: {}'s part", self.at(), step.intent))
                    }
                    None => step.intent.clone(),
                };
                self.wake(step, player);
                let outcome = self.submit(step, player, &part);
                assert_eq!(outcome, Outcome::Ready, "{}: {part}", self.at());
                if self.revision() != revision {
                    assert_eq!(
                        names(&outstanding),
                        names(std::slice::from_ref(player)),
                        "{}: {} completed before every player was ready",
                        self.at(),
                        step.intent
                    );
                    tally.shared += 1;
                    return;
                }
                self.check_waits(&step.before, None);
            }
        }
        panic!("{}: {} did not complete", self.at(), step.intent);
    }

    /// An option one player owns: applied at once, unless its player decides here
    /// while another player holds an option, when it is held for their passes.
    fn alone(&mut self, step: &Step, tally: &mut Tally) {
        let listed = &step.before;
        let control = self.host.control();
        let id = step.intent.as_str();
        let holders = Listed::holders(&step.owner, &control);
        let [player] = holders.as_slice() else {
            panic!(
                "{}: {id} is owned by several players outside a shared decision: {:?}",
                self.at(),
                step.owner
            );
        };
        let player = player.clone();
        let others = listed.others(&control);
        let deciding = listed.deciding(&control);
        let revision = self.revision();
        if deciding.as_ref() != Some(&player) || others.is_empty() || routing::pays_act(id) {
            if let Some(decider) = deciding.filter(|_| others.contains(&player))
                && self.dice.one_in(2)
                && self.instead(step, &decider, &player, tally)
            {
                return;
            }
            let outcome = self.submit(step, &player, id);
            assert_eq!(outcome, Outcome::Applied, "{}: {id}", self.at());
            return;
        }
        let outcome = self.submit(step, &player, id);
        assert_eq!(
            outcome,
            Outcome::Held,
            "{}: {id} beside {:?}'s options",
            self.at(),
            names(&others)
        );
        assert_eq!(self.revision(), revision, "{}: {id}", self.at());
        self.check_waits(listed, Some(&names(&others)));
        let mut waiting = others.clone();
        self.dice.shuffle(&mut waiting);
        while let Some(other) = waiting.pop() {
            self.wake(step, &other);
            let outcome = self.submit(step, &other, PASS_BEFORE);
            assert_eq!(outcome, Outcome::Ready, "{}: {}", self.at(), name(&other));
            if !waiting.is_empty() {
                assert_eq!(
                    self.revision(),
                    revision,
                    "{}: {id} applied before {:?} passed",
                    self.at(),
                    names(&waiting)
                );
                self.check_waits(listed, Some(&names(&waiting)));
            }
        }
        assert_ne!(
            self.revision(),
            revision,
            "{}: {id} after the passes",
            self.at()
        );
        if let Controller::Partner(_) = &player {
            let told = self.wait(step, &player);
            assert_eq!(told["status"], "chosen", "{}: {told}", self.at());
            tally.held_for_partner += 1;
        } else {
            assert_eq!(self.host.human_held(self.session()), None, "{}", self.at());
            tally.held_for_human += 1;
        }
        if let OptionOwner::Investigator(who) = step.owner {
            assert_eq!(
                self.table.attributed.last(),
                Some(&(id.to_owned(), who)),
                "{}",
                self.at()
            );
        }
        *tally
            .held
            .entry(listed.list.expect("A list with a decider"))
            .or_default() += 1;
    }

    /// The decider's player chooses first, and `participant` uses the intent, its
    /// own option there, instead of passing: the intent is applied at once and the
    /// held choice is released, not applied. Whether the decider's player had a
    /// choice to hold.
    fn instead(
        &mut self,
        step: &Step,
        decider: &Controller,
        participant: &Controller,
        tally: &mut Tally,
    ) -> bool {
        let control = self.host.control();
        let theirs = step.before.own(&control, decider);
        if theirs.is_empty() {
            return false;
        }
        let first = theirs[self.dice.below(theirs.len())].clone();
        let revision = self.revision();
        let outcome = self.submit(step, decider, &first);
        assert_eq!(outcome, Outcome::Held, "{}: {first}", self.at());
        assert_eq!(self.revision(), revision, "{}: {first}", self.at());
        let outcome = self.submit(step, participant, &step.intent);
        assert_eq!(outcome, Outcome::Applied, "{}: {}", self.at(), step.intent);
        assert!(
            *self.session() == *step.applied,
            "{}: {} is applied, and {first} is not",
            self.at(),
            step.intent
        );
        if let Controller::Partner(_) = decider {
            let told = self.wait(step, decider);
            assert_eq!(told["status"], "released", "{}: {told}", self.at());
        } else {
            assert!(
                self.host.human_released(self.session()).is_some(),
                "{}: the human is told {first} was released",
                self.at()
            );
            assert_eq!(self.host.human_held(self.session()), None, "{}", self.at());
        }
        tally.released += 1;
        true
    }
}

fn assignment_code(assignment: Assignment) -> u64 {
    assignment.iter().fold(0, |code, held| {
        code * 4 + held.map_or(0, |partner| partner as u64 + 1)
    })
}

/// The reference play-through's next decision: an option picked at random, its
/// placeholder named, and the session once it is applied. `None` once nothing is
/// listed.
fn pick(session: &Session, before: Listed, dice: &mut Dice) -> Option<Step> {
    if before.owned.is_empty() {
        return None;
    }
    let observation = session.observe();
    let intent = loop {
        let id = before.owned[dice.below(before.owned.len())].id.clone();
        if GroupCluePayment::is_listed(&id) {
            let payment = observation
                .group_clue_payment
                .as_ref()
                .expect("The listed payment");
            if let Some(paid) = allocation(payment, dice) {
                break paid;
            }
            continue;
        }
        if let Some(template) = id.strip_suffix(PREDICTION_MARGIN) {
            break format!("{template}{}", dice.below(6));
        }
        break id;
    };
    let owner = session
        .option_owner(&intent)
        .unwrap_or_else(|| panic!("{intent} is accepted"));
    let mut applied = session.clone();
    applied
        .choose(applied.revision(), &intent)
        .unwrap_or_else(|error| panic!("{intent}: {error}"));
    // Derived once here; each assignment's table installs this session.
    let _ = applied.undo_owner();
    let after = Listed::of(&applied);
    Some(Step {
        before,
        intent,
        owner,
        applied: Rc::new(applied),
        after,
    })
}

/// A complete allocation of the payment, its contributors in a random order.
fn allocation(payment: &GroupCluePayment, dice: &mut Dice) -> Option<String> {
    if let Some(forced) = payment.forced {
        return payment.choice_for(forced);
    }
    let mut order: Vec<_> = payment.contributors.iter().collect();
    dice.shuffle(&mut order);
    let (mut clues, mut left) = ([0; 3], payment.required);
    for contributor in order {
        let given = contributor.limit(payment.required).min(left);
        clues[contributor.investigator.index()] = given;
        left -= given;
    }
    payment.choice_for(clues)
}

/// One seed's play-through, taken at every assignment.
fn play_through(seed: u64, decisions: usize) -> Tally {
    let mut tally = Tally::default();
    let mut session = Session::with_seed(seed);
    let mut plays: Vec<Play> = assignments()
        .into_iter()
        .map(|assignment| Play::new(seed, assignment, &session))
        .collect();
    let mut dice = Dice(seed);
    let mut before = Listed::of(&session);
    for _ in 0..decisions {
        let Some(step) = pick(&session, before, &mut dice) else {
            break;
        };
        tally.decisions += 1;
        if let Some(list) = step.before.list {
            *tally.opportunities.entry(list).or_default() += 1;
        }
        for play in &mut plays {
            play.take(&step, &mut tally);
        }
        session = Session::clone(&step.applied);
        before = step.after;
    }
    tally
}

/// #218 item 1: across seeded play of The Gathering and every assignment of the
/// investigators to the human and one, two or three partners, no choice skips another
/// player's opportunity, and no player is waited on, offered or allowed an option
/// that is not theirs.
#[test]
fn seeded_co_play_never_skips_another_players_opportunity() {
    assert_eq!(assignments().len(), 14);
    let tallies: Vec<Tally> = std::thread::scope(|scope| {
        let runs: Vec<_> = SEEDS
            .map(|seed| scope.spawn(move || play_through(seed, DECISIONS)))
            .into_iter()
            .collect();
        runs.into_iter()
            .map(|run| {
                run.join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    });
    let mut tally = Tally::default();
    for (seed, run) in SEEDS.iter().zip(tallies) {
        println!("seed {seed}: {run:?}");
        tally.add(run);
    }
    println!("{tally:#?}");
    for list in [TURN, REVEAL, REVELATION] {
        assert!(
            tally.held.get(list).is_some_and(|held| *held > 0),
            "{list}: {tally:?}"
        );
    }
    assert!(
        tally.held_for_human > 0 && tally.held_for_partner > 0,
        "{tally:?}"
    );
    assert!(
        tally.released > 0 && tally.shared > 0 && tally.refused > 0,
        "{tally:?}"
    );
}
