//! Agent partners (#202, [ADR 0007]): players who play some investigators inside the
//! human's running game. This module is what a partner may do and see, without any
//! window: who controls each investigator, which options each player may submit, the
//! player-scoped view, and the partner protocol, one JSON request and one JSON answer
//! per line. The table hosts the transport and the presentation
//! (`gathering_tabletop::partners`). Deleting this module, that one, the tools'
//! `partner` command and the integration points that name them removes partners.
//!
//! A partner joins, which claims investigators the human handed to partners; waits
//! until one of them has a decision pending once the table has presented it (a
//! reading or the human's dialog may still be open, #210); observes its view,
//! complete once and then as changes (#206); inspects any card it can see; chooses
//! an option by identity at the current revision; and leaves. A choice is
//! applied only once the table settles, with nothing of the human's open over it,
//! through the table's own commit path. A group
//! choice is its arbiter's player's, and every partner's wait hears it open (#207).
//! In a shared decision that spans players, each player sets its own part and marks
//! itself ready, and the host completes it once every participating player is
//! ready (#208, `shared`). Every player may speak, and every other player hears
//! each message (#209, `talk`).
//!
//! [ADR 0007]: ../../docs/adr/0007-agent-partners-join-the-running-table.md

mod routing;
mod shared;
mod talk;
mod view;

pub use routing::{Routing, Shared, SharedKind};
pub use shared::{REOPENINGS, Readiness, SharedState};
pub use talk::{CHATTER_LIMIT, EXCHANGE_LIMIT, LONGEST_MESSAGE, PERIL, Said};

use crate::Choice;
use crate::gathering::{
    Investigator, OptionOwner, Session,
    inspection::{self, Sight},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// How long a wait is held before it answers `nothing_yet`, unless the partner asks
/// for another period: within Codex's five-minute polling window and Claude Code's
/// ten-minute command limit.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(240);
/// The longest wait a partner may ask for, under Claude Code's command limit.
pub const LONGEST_WAIT: Duration = Duration::from_secs(590);
/// How many of the latest messages heard since joining a complete view gives back,
/// for a partner whose answer was lost to a cut-off command.
pub const RECOVERED_TALK: usize = 20;

/// The player who controls an investigator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Controller {
    /// The human at the window.
    Human,
    /// The partner session with this identity.
    Partner(String),
    /// Handed to partners, but no partner holds it: its decisions wait for one to
    /// join, or for the human to take it over.
    Unclaimed,
}

/// Which investigators the human handed to partners, and which partner holds each.
/// Runtime state of the session at the table; never saved with the attempt.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Control {
    assigned: [bool; 3],
    claims: [Option<String>; 3],
}

impl Control {
    pub fn controller(&self, who: Investigator) -> Controller {
        let index = who.index();
        if !self.assigned[index] {
            return Controller::Human;
        }
        self.claims[index]
            .clone()
            .map_or(Controller::Unclaimed, Controller::Partner)
    }

    /// Whether the human plays every investigator, as without partners.
    pub fn solo(&self) -> bool {
        self.assigned == [false; 3]
    }

    /// Whether the human may submit `id` at the session's pending decision: its
    /// own option, or its part in a shared decision it takes part in. Without
    /// partners every option is the human's. With partners, Undo is the human's only
    /// when the decision it would revert belongs to investigators the human controls.
    pub fn human_may(&self, session: &Session, id: &str) -> bool {
        self.solo() || Routing::of(session, self).human_may(session, self, id)
    }

    /// The listed options of the session's pending decision that the human may
    /// submit, as the table offers them (#205): every one without partners, and
    /// with partners those [`Self::human_may`] allows, as a partner's view lists
    /// only that partner's options.
    pub fn human_choices(&self, session: &Session) -> Vec<Choice> {
        let choices = session.observe().choices;
        if self.solo() {
            return choices;
        }
        let routing = Routing::of(session, self);
        choices
            .into_iter()
            .filter(|choice| routing.human_may(session, self, &choice.id))
            .collect()
    }

    fn held_by(&self, partner: &str) -> Vec<Investigator> {
        Investigator::ALL
            .into_iter()
            .filter(|who| self.claims[who.index()].as_deref() == Some(partner))
            .collect()
    }
}

/// The table a partner plays at, as the host reaches it.
pub trait Table {
    fn session(&self) -> &Session;
    /// The table has finished presenting the pending decision, so a wait may tell
    /// its players of it, even while a reading or the human's dialog is still open
    /// (#210). By default, only once the table has settled.
    fn presented(&self) -> bool {
        self.settled()
    }
    /// The table has finished presenting, would take the human's click now, and
    /// shows nothing of the human's over it: a choice applied now presents its
    /// consequences at once.
    fn settled(&self) -> bool;
    /// Commit `id` through the human's commit path, saving before presenting. The
    /// table attributes the decision to `by` in its action history.
    fn commit(&mut self, revision: u64, id: &str, by: Investigator) -> Result<(), String>;
}

/// One answer to a request, for the connection that sent it.
pub type Answer<R> = (R, Value);

/// What the human's choice does while partners play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HumanChoice {
    /// Commit it through the table's commit path, as without partners.
    Commit,
    /// It was the human's readiness in a shared decision, now recorded: Confirm,
    /// a window's pass, or a clue contribution. The host completes the decision
    /// once every participating player is ready.
    Ready,
}

/// The partner sessions at one table and their requests in progress. `R` is how
/// the transport answers a request: it travels with the request and comes back
/// with its answer.
pub struct Host<R> {
    enabled: bool,
    /// The investigators the human hands to partners while partners are enabled.
    selected: [bool; 3],
    claims: [Option<String>; 3],
    partners: Vec<Partner>,
    held: Vec<Held<R>>,
    /// Readiness, reopenings and pending contributions in shared decisions.
    sharing: shared::Sharing,
    talk: talk::Talk,
}

struct Partner {
    id: String,
    /// The investigators this partner was last told it plays.
    reported: Vec<Investigator>,
    /// Every investigator this partner has played, to say when one was taken over.
    played: Vec<Investigator>,
    /// Card text already sent, by code and face.
    sent: BTreeSet<String>,
    /// The board of the last view sent, which the next view's changes start from.
    board: Option<serde_json::Map<String, Value>>,
    heard: Instant,
    /// The revision of the last group choice a wait reported to this partner.
    told_group: Option<u64>,
    /// The shared decisions this partner's last view showed, and their state then.
    seen: Vec<(shared::Key, shared::Stamp)>,
    /// The table talk this partner has been told: messages before this index.
    told_talk: usize,
    /// The table talk said before this partner sat down: messages before this index.
    joined_talk: usize,
}

struct Held<R> {
    reply: R,
    partner: String,
    pending: Pending,
}

enum Pending {
    Wait { deadline: Instant },
    Choose { revision: u64, id: String },
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Join {
        #[serde(default)]
        investigators: Vec<Investigator>,
    },
    Wait {
        partner: String,
        #[serde(default)]
        timeout_seconds: Option<u64>,
    },
    Observe {
        partner: String,
        /// The complete view, every visible card's text and the latest table talk
        /// heard since joining, again.
        #[serde(default)]
        full: bool,
    },
    Inspect {
        partner: String,
        card: String,
    },
    Choose {
        partner: String,
        revision: u64,
        id: String,
    },
    /// Readiness in the pending shared decision, on the state of the last view.
    Ready {
        partner: String,
        revision: u64,
    },
    Leave {
        partner: String,
    },
    Say {
        partner: String,
        text: String,
    },
}

impl<R> Default for Host<R> {
    fn default() -> Self {
        Self {
            enabled: false,
            selected: [false; 3],
            claims: [None, None, None],
            partners: Vec::new(),
            held: Vec::new(),
            sharing: shared::Sharing::default(),
            talk: talk::Talk::default(),
        }
    }
}

impl<R> Host<R> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Who controls each investigator now.
    pub fn control(&self) -> Control {
        Control {
            assigned: if self.enabled {
                self.selected
            } else {
                [false; 3]
            },
            claims: self.claims.clone(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether the human hands `who` to partners when partners are enabled.
    pub fn selected(&self, who: Investigator) -> bool {
        self.selected[who.index()]
    }

    /// Enable partners, or turn them off: every investigator returns to the human
    /// and every partner session ends.
    pub fn set_enabled(&mut self, enabled: bool) -> Vec<Answer<R>> {
        self.enabled = enabled;
        if enabled {
            return Vec::new();
        }
        self.close("Agent partners were turned off.")
    }

    /// Hand `who` to partners, or return it to the human (Take over). A partner
    /// holding it loses it at once; its waits answer that its control changed.
    pub fn set_selected(&mut self, who: Investigator, selected: bool) {
        self.selected[who.index()] = selected;
        if !selected {
            self.claims[who.index()] = None;
        }
    }

    /// End every partner session: the table closed or partners were turned off.
    pub fn close(&mut self, reason: &str) -> Vec<Answer<R>> {
        self.partners.clear();
        self.claims = [None, None, None];
        self.held
            .drain(..)
            .map(|held| (held.reply, closed(reason)))
            .collect()
    }

    /// How long since the partner holding `who` was last heard from; zero while one
    /// of its requests is held. `None` when no partner holds `who`.
    pub fn silence(&self, who: Investigator, now: Instant) -> Option<Duration> {
        let id = self.claims[who.index()].as_deref()?;
        if self.held.iter().any(|held| held.partner == id) {
            return Some(Duration::ZERO);
        }
        let partner = self.partners.iter().find(|partner| partner.id == id)?;
        Some(now.saturating_duration_since(partner.heard))
    }

    /// The sitting's table talk, oldest first: kept while the table is open, and
    /// never saved with the attempt.
    pub fn table_talk(&self) -> &[Said] {
        self.talk.said()
    }

    /// Why no one may speak now, if no one may.
    pub fn talk_closed(&mut self, table: &impl Table) -> Option<&'static str> {
        self.talk.closed(table)
    }

    /// The human at the window speaks, as the investigators the human plays. Every
    /// partner's wait hears it at the next [`Self::poll`].
    pub fn human_says(&mut self, text: &str, table: &impl Table) -> Result<(), String> {
        let control = self.control();
        let speaker = Investigator::ALL
            .into_iter()
            .filter(|who| control.controller(*who) == Controller::Human)
            .collect();
        self.talk.say(None, speaker, text, table)
    }

    /// Whether [`Self::poll`] would answer a held request, or complete a shared
    /// decision, now, at a table that has `presented` its decision and is `settled`
    /// or not, as [`Table`] names them.
    pub fn due(&self, session: &Session, presented: bool, settled: bool, now: Instant) -> bool {
        if settled && self.completion(session).is_some() {
            return true;
        }
        let mut moment: Option<Moment> = None;
        self.held.iter().any(|held| match &held.pending {
            Pending::Choose { .. } => settled,
            Pending::Wait { deadline } => {
                now >= *deadline
                    || self.changed(&held.partner)
                    || self.talk_untold(&held.partner)
                    || (presented
                        && moment
                            .get_or_insert_with(|| self.moment(session))
                            .event(self, &held.partner)
                            .is_some())
            }
        })
    }

    /// The players the table waits for, each with the investigator it answers
    /// for: the player of the pending decision, and every participating player
    /// not yet ready in a shared decision that waits on its players.
    pub fn awaited(&self, session: &Session) -> Vec<(Controller, Option<Investigator>)> {
        self.moment(session).awaited
    }

    /// The pending shared decisions that span players, as the table shows them.
    pub fn shared(&self, session: &Session) -> Vec<SharedState> {
        let control = self.control();
        self.moment(session)
            .pending
            .iter()
            .map(|pending| self.sharing.state(pending, &control))
            .collect()
    }

    /// Moves whenever readiness or a pending contribution changes.
    pub fn sharing_version(&self) -> u64 {
        self.sharing.version()
    }

    /// What the human's choice of `id` does now: commit it, or record the human's
    /// part in a shared decision. A change to the human's part beyond the reopen
    /// bound, or a contribution against another player's changed one, is refused.
    pub fn human(
        &mut self,
        session: &Session,
        revision: u64,
        id: &str,
    ) -> Result<HumanChoice, String> {
        let control = self.control();
        if control.solo() || revision != session.revision() {
            // A stale click is refused by the session itself.
            return Ok(HumanChoice::Commit);
        }
        let moment = self.moment(session);
        let human = Controller::Human;
        if let Some(pending) = moment
            .pending
            .iter()
            .find(|pending| pending.decision.includes(id))
        {
            if !pending.decision.players(&control).contains(&human) {
                return Err("That shared decision belongs to the other players.".into());
            }
            self.mark_ready(pending, &human, Some(id), &moment.observation)?;
            return Ok(HumanChoice::Ready);
        }
        if let Some(pending) = moment.changes(&control, &human)
            && moment.routing.controller(session, &control, id) == Some(human.clone())
        {
            self.sharing.change(pending, &human)?;
        }
        Ok(HumanChoice::Commit)
    }

    /// Answer one request line now, or hold it. Answers to held requests come from
    /// [`Self::poll`].
    pub fn receive(
        &mut self,
        line: &str,
        reply: R,
        table: &mut impl Table,
        now: Instant,
    ) -> Vec<Answer<R>> {
        let request = match serde_json::from_str::<Request>(line) {
            Ok(request) => request,
            Err(error) => {
                let partner = serde_json::from_str::<Value>(line)
                    .ok()
                    .and_then(|value| value["partner"].as_str().map(str::to_owned));
                let reason = format!(
                    "Not a partner request ({error}). A partner may join, wait, observe, inspect, choose, ready, say or leave."
                );
                let answer = match partner.filter(|id| self.known(id)) {
                    Some(id) => self.answer(&id, "refused", Some(reason), table, false, now),
                    None => refused(reason),
                };
                return vec![(reply, answer)];
            }
        };
        let id = match &request {
            Request::Join { investigators } => {
                return vec![(reply, self.join(investigators, table, now))];
            }
            Request::Wait { partner, .. }
            | Request::Observe { partner, .. }
            | Request::Inspect { partner, .. }
            | Request::Choose { partner, .. }
            | Request::Ready { partner, .. }
            | Request::Say { partner, .. }
            | Request::Leave { partner } => partner.clone(),
        };
        if !self.known(&id) {
            return vec![(
                reply,
                refused(format!(
                    "No partner {id} plays at this table. Join first; a partner identity ends when its session does."
                )),
            )];
        }
        match request {
            Request::Join { .. } => unreachable!("Answered above"),
            Request::Observe { full, .. } => {
                vec![(reply, self.answer(&id, "observed", None, table, full, now))]
            }
            Request::Inspect { card, .. } => vec![(reply, self.inspect(&id, &card, table, now))],
            Request::Say { text, .. } => {
                self.hear(&id, now);
                let speaker = self.control().held_by(&id);
                let answer = match self.talk.say(Some(&id), speaker, &text, &*table) {
                    Ok(()) => self.answer(&id, "said", None, table, false, now),
                    Err(reason) => self.answer(&id, "refused", Some(reason), table, false, now),
                };
                // Every other partner's wait hears it now.
                let mut answers = vec![(reply, answer)];
                answers.extend(self.poll(table, now));
                answers
            }
            Request::Leave { .. } => {
                self.partners.retain(|partner| partner.id != id);
                for claim in &mut self.claims {
                    if claim.as_deref() == Some(id.as_str()) {
                        *claim = None;
                    }
                }
                let mut answers: Vec<_> = self
                    .held
                    .extract_if(.., |held| held.partner == id)
                    .map(|held| (held.reply, closed("You left the table.")))
                    .collect();
                answers.push((
                    reply,
                    json!({"ok": true, "mode": "partner", "status": "left", "partner": id}),
                ));
                answers
            }
            Request::Wait {
                timeout_seconds, ..
            } => {
                self.hear(&id, now);
                let period = timeout_seconds
                    .map_or(DEFAULT_WAIT, Duration::from_secs)
                    .min(LONGEST_WAIT);
                self.held.push(Held {
                    reply,
                    partner: id,
                    pending: Pending::Wait {
                        deadline: now + period,
                    },
                });
                self.poll(table, now)
            }
            Request::Ready { revision, .. } => self.ready(&id, revision, None, reply, table, now),
            Request::Choose {
                revision,
                id: choice,
                ..
            } => {
                let moment = self.moment(table.session());
                if moment.routing.shared_of(&choice).is_some() {
                    return self.ready(&id, revision, Some(&choice), reply, table, now);
                }
                let me = Controller::Partner(id.clone());
                let checked = self
                    .check(&id, revision, &choice, &moment.routing, table.session())
                    .and_then(|_| match moment.changes(&self.control(), &me) {
                        Some(pending) => self.sharing.may_change(pending, &me),
                        None => Ok(()),
                    });
                if let Err(reason) = checked {
                    return vec![(
                        reply,
                        self.answer(&id, "refused", Some(reason), table, false, now),
                    )];
                }
                self.hear(&id, now);
                self.held.push(Held {
                    reply,
                    partner: id,
                    pending: Pending::Choose {
                        revision,
                        id: choice,
                    },
                });
                self.poll(table, now)
            }
        }
    }

    /// Complete a shared decision whose players are all ready, then answer every
    /// held request whose moment has come: a wait whose control changed, or whose
    /// partner hears a group choice open, has news of a shared decision or has a
    /// decision, once the table has presented it, has table talk to hear, or whose
    /// period ran out; and a choice once the table has settled.
    pub fn poll(&mut self, table: &mut impl Table, now: Instant) -> Vec<Answer<R>> {
        self.talk.watch(&*table);
        if table.settled()
            && let Some((pending, id, by)) = self.completion(table.session())
        {
            let revision = table.session().revision();
            if let Err(error) = table.commit(revision, &id, by) {
                eprintln!("Agent partners: the shared decision did not complete: {error}");
                self.sharing.unready(&pending);
            }
        }
        let mut answers = Vec::new();
        let mut index = 0;
        // Routed only once the table has presented its decision, and again after a
        // commit.
        let mut moment: Option<Moment> = None;
        while index < self.held.len() {
            let partner = self.held[index].partner.clone();
            let (presented, settled) = (table.presented(), table.settled());
            let revision = table.session().revision();
            let event = match &self.held[index].pending {
                Pending::Wait { .. } if presented && !self.changed(&partner) => moment
                    .get_or_insert_with(|| self.moment(table.session()))
                    .event(self, &partner),
                _ => None,
            };
            let outcome = match &self.held[index].pending {
                Pending::Wait { deadline } => {
                    if self.changed(&partner) {
                        Some(("control_changed", None))
                    } else if let Some(event) = event {
                        Some((event, None))
                    } else if self.talk_untold(&partner) {
                        // After the table's own events, whose answers carry the
                        // talk as well.
                        Some(("table_talk", None))
                    } else if now >= *deadline {
                        let held = self.held.remove(index);
                        answers.push((
                            held.reply,
                            json!({
                                "ok": true, "mode": "partner", "status": "nothing_yet",
                                "partner": partner,
                                "investigators": self.control().held_by(&partner),
                            }),
                        ));
                        continue;
                    } else {
                        None
                    }
                }
                Pending::Choose { revision, id } if settled => {
                    let (revision, id) = (*revision, id.clone());
                    let current = moment
                        .take()
                        .unwrap_or_else(|| self.moment(table.session()));
                    let me = Controller::Partner(partner.clone());
                    // A partner's option in a commitment window changes its part.
                    let change = current.changes(&self.control(), &me).cloned();
                    let result = self
                        .check(&partner, revision, &id, &current.routing, table.session())
                        .and_then(|by| match &change {
                            Some(pending) => self.sharing.may_change(pending, &me).map(|()| by),
                            None => Ok(by),
                        })
                        .and_then(|by| table.commit(revision, &id, by));
                    if result.is_ok()
                        && let Some(pending) = &change
                    {
                        self.sharing
                            .change(pending, &me)
                            .expect("Checked before committing");
                    }
                    match result {
                        Ok(()) => Some(("chosen", None)),
                        Err(reason) => Some(("refused", Some(reason))),
                    }
                }
                Pending::Choose { .. } => None,
            };
            match outcome {
                Some((status, error)) => {
                    let held = self.held.remove(index);
                    if status == "group_choice"
                        && let Some(known) = self.partners.iter_mut().find(|p| p.id == partner)
                    {
                        known.told_group = Some(revision);
                    }
                    let answer = self.answer(&partner, status, error, table, false, now);
                    answers.push((held.reply, answer));
                }
                None => index += 1,
            }
        }
        answers
    }

    fn known(&self, id: &str) -> bool {
        self.partners.iter().any(|partner| partner.id == id)
    }

    fn hear(&mut self, id: &str, now: Instant) {
        if let Some(partner) = self.partners.iter_mut().find(|partner| partner.id == id) {
            partner.heard = now;
        }
    }

    /// Whether another player has said something `id` has not been told.
    fn talk_untold(&self, id: &str) -> bool {
        self.partners
            .iter()
            .find(|partner| partner.id == id)
            .is_some_and(|partner| self.talk.unheard(partner.told_talk, id))
    }

    /// Whether a group choice is open at `revision` and no wait has told `id` yet.
    fn group_untold(&self, id: &str, routing: &Routing, revision: u64) -> bool {
        routing.group().is_some()
            && self
                .partners
                .iter()
                .any(|partner| partner.id == id && partner.told_group != Some(revision))
    }

    fn changed(&self, id: &str) -> bool {
        self.partners
            .iter()
            .find(|partner| partner.id == id)
            .is_none_or(|partner| partner.reported != self.control().held_by(id))
    }

    fn join(&mut self, named: &[Investigator], table: &impl Table, now: Instant) -> Value {
        let control = self.control();
        let open = |who: &Investigator| control.controller(*who) == Controller::Unclaimed;
        let taking: Vec<Investigator> = if named.is_empty() {
            Investigator::ALL.into_iter().filter(open).collect()
        } else {
            for who in named {
                match control.controller(*who) {
                    Controller::Unclaimed => {}
                    Controller::Partner(_) => {
                        return refused(format!(
                            "{} is already played by another partner.",
                            who.name()
                        ));
                    }
                    Controller::Human => {
                        return refused(format!("{} is not open to partners.", who.name()));
                    }
                }
            }
            let mut taking = named.to_vec();
            taking.sort_by_key(|who| who.index());
            taking.dedup();
            taking
        };
        if taking.is_empty() {
            return refused(
                "No investigator is open to partners right now: the table has handed none to partners, or other partner sessions play every one it handed. There is nothing for you to play, so stop."
                    .into(),
            );
        }
        let id = token();
        for who in &taking {
            self.claims[who.index()] = Some(id.clone());
        }
        self.partners.push(Partner {
            id: id.clone(),
            reported: Vec::new(),
            played: Vec::new(),
            sent: BTreeSet::new(),
            board: None,
            heard: now,
            told_group: None,
            seen: Vec::new(),
            // A player hears what is said from when it sits down.
            told_talk: self.talk.said().len(),
            joined_talk: self.talk.said().len(),
        });
        self.answer(&id, "joined", None, table, false, now)
    }

    /// The pending decision as routed now, with its shared decisions and the
    /// players it waits for.
    fn moment(&self, session: &Session) -> Moment {
        let control = self.control();
        let observation = session.observe();
        let routing = Routing::of(session, &control);
        let pending = self.sharing.pending(routing.shared(), &observation);
        let mut awaited: Vec<(Controller, Option<Investigator>)> = routing
            .awaiting()
            .map(|player| (player.clone(), routing.awaited_investigator()))
            .into_iter()
            .collect();
        for decision in &pending {
            for (player, who) in self.sharing.waiting(decision, &control) {
                if !awaited.iter().any(|(listed, _)| *listed == player) {
                    awaited.push((player, Some(who)));
                }
            }
        }
        Moment {
            observation,
            routing,
            pending,
            awaited,
        }
    }

    /// A shared decision whose participating players are all ready, with its
    /// completing choice and the investigator it is attributed to.
    fn completion(&self, session: &Session) -> Option<(shared::Pending, String, Investigator)> {
        if !self.sharing.ready_at(session.revision()) {
            return None;
        }
        let control = self.control();
        let moment = self.moment(session);
        moment.pending.into_iter().find_map(|pending| {
            let (id, by) = self
                .sharing
                .completion(&pending, &control, &moment.observation)?;
            Some((pending, id, by))
        })
    }

    /// A partner's readiness in a shared decision: through `ready`, or by choosing
    /// the decision's completing option, a named clue allocation or The Barrier's
    /// decline. The decision completes here when it was the last one awaited.
    fn ready(
        &mut self,
        partner: &str,
        revision: u64,
        id: Option<&str>,
        reply: R,
        table: &mut impl Table,
        now: Instant,
    ) -> Vec<Answer<R>> {
        let me = Controller::Partner(partner.to_owned());
        let moment = self.moment(table.session());
        let result = if revision != moment.observation.revision {
            Err(format!(
                "This readiness is stale: it names revision {revision}, and the decision is now at revision {}. Observe the current decision.",
                moment.observation.revision
            ))
        } else {
            let control = self.control();
            let mine =
                |pending: &&shared::Pending| pending.decision.players(&control).contains(&me);
            let target = match id {
                Some(id) => moment
                    .pending
                    .iter()
                    .find(|pending| pending.decision.includes(id)),
                // A commitment or a window before a payment proposed beside it.
                None => moment.pending.iter().filter(mine).min_by_key(|pending| {
                    matches!(pending.decision.kind, SharedKind::Payment { .. })
                }),
            };
            match target {
                None => Err(
                    "No shared decision is pending for you to be ready in. Observe the current decision."
                        .into(),
                ),
                Some(pending) if !mine(&pending) => {
                    Err("You take no part in that shared decision.".into())
                }
                Some(pending) => {
                    let seen = self
                        .partners
                        .iter()
                        .find(|known| known.id == partner)
                        .and_then(|known| known.seen.iter().find(|(key, _)| *key == pending.key))
                        .map(|(_, stamp)| *stamp);
                    if seen == Some(pending.stamp) {
                        self.mark_ready(pending, &me, id, &moment.observation)
                    } else {
                        Err("The shared decision changed since your last view; nothing was rebased. Observe it and decide again.".into())
                    }
                }
            }
        };
        if let Err(reason) = result {
            return vec![(
                reply,
                self.answer(partner, "refused", Some(reason), table, false, now),
            )];
        }
        self.hear(partner, now);
        let mut answers = self.poll(table, now);
        let answer = self.answer(partner, "ready", None, table, false, now);
        answers.insert(0, (reply, answer));
        answers
    }

    /// Record `player`'s readiness, setting its part of a payment first when `id`
    /// names an allocation or The Barrier's decline.
    fn mark_ready(
        &mut self,
        pending: &shared::Pending,
        player: &Controller,
        id: Option<&str>,
        observation: &crate::gathering::Observation,
    ) -> Result<(), String> {
        let control = self.control();
        match (pending.decision.kind, id) {
            (SharedKind::Payment { .. }, Some(id)) if id != pending.decision.close => {
                let payment = observation
                    .group_clue_payment
                    .as_ref()
                    .ok_or("No group clue payment is pending.")?;
                self.sharing
                    .contribute(pending, payment, &control, player, id)
            }
            _ => {
                self.sharing.ready(pending, player);
                Ok(())
            }
        }
    }

    /// A visible card's full text, as the table's Inspect shows it. A card this
    /// partner cannot see is refused exactly as a code that names no card.
    fn inspect(&mut self, id: &str, card: &str, table: &impl Table, now: Instant) -> Value {
        self.hear(id, now);
        let mine = self.control().held_by(id);
        let observation = table.session().observe();
        match inspection::table_card(&observation, card, Sight::Player(&mine)) {
            Some(text) => json!({
                "ok": true, "mode": "partner", "status": "inspected", "partner": id,
                "investigators": mine, "card": card, "text": text,
            }),
            None => {
                let reason = "No card with that code is in your sight. Inspect a card you can see by its code: in your investigators' hands, in play, being resolved or in a public pile.".to_owned();
                self.answer(id, "refused", Some(reason), table, false, now)
            }
        }
    }

    /// Whether `partner` may submit `id` at `revision` now, and as which investigator.
    fn check(
        &self,
        partner: &str,
        revision: u64,
        id: &str,
        routing: &Routing,
        session: &Session,
    ) -> Result<Investigator, String> {
        let current = session.observe().revision;
        if revision != current {
            return Err(format!(
                "This choice is stale: it names revision {revision}, and the decision is now at revision {current}. Observe the current decision."
            ));
        }
        let control = self.control();
        let me = Controller::Partner(partner.to_owned());
        // Undo's owner is that of the decision it would revert.
        let Some(owner) = routing.owner(session, id) else {
            return Err("That choice is not available at the current decision.".into());
        };
        let named = routing::investigators(&owner);
        let names = named
            .iter()
            .map(|who| who.name())
            .collect::<Vec<_>>()
            .join(" and ");
        if routing.controller(session, &control, id).as_ref() != Some(&me) {
            let played = self
                .partners
                .iter()
                .find(|known| known.id == partner)
                .map(|known| known.played.clone())
                .unwrap_or_default();
            if let Some(lost) = named.iter().find(|who| played.contains(who)) {
                return Err(format!(
                    "{} is no longer yours: the table took it over. Observe your current view.",
                    lost.name()
                ));
            }
            if id == "undo" {
                return Err(match owner {
                    OptionOwner::Table => "Undo reverts only your own latest decision; the latest was made at the table.".into(),
                    _ => format!("Undo reverts only your own latest decision; the latest belongs to {names}."),
                });
            }
            return Err(match owner {
                OptionOwner::Table => {
                    "That choice is made at the table: the lead investigator is chosen there."
                        .into()
                }
                OptionOwner::Group { arbiter } => format!(
                    "That is a group choice: the player of {}, its arbiter, makes it.",
                    arbiter.name()
                ),
                // A shared decision that spans players is readiness, answered
                // before this check; this one is another player's alone.
                OptionOwner::Shared { .. } => {
                    format!("That choice belongs to the player of {names}, whom you do not play.")
                }
                OptionOwner::Investigator(_) => {
                    format!("That choice belongs to {names}, whom you do not play.")
                }
            });
        }
        Ok(match owner {
            OptionOwner::Investigator(who) => who,
            // Undo answers for the investigator whose decision it reverts.
            _ if id == "undo" => *named
                .first()
                .expect("A partner's latest decision names its investigator"),
            _ => routing
                .awaited_investigator()
                .filter(|who| control.controller(*who) == me)
                .or_else(|| named.first().copied())
                .or_else(|| control.held_by(partner).first().copied())
                .expect("A partner's option names one of its investigators"),
        })
    }

    /// An answer for a known partner, with its investigators and view.
    fn answer(
        &mut self,
        id: &str,
        status: &str,
        error: Option<String>,
        table: &impl Table,
        full: bool,
        now: Instant,
    ) -> Value {
        let control = self.control();
        let moment = self.moment(table.session());
        let me = Controller::Partner(id.to_owned());
        let shared: Vec<view::SharedPart> = moment
            .pending
            .iter()
            .filter(|pending| pending.decision.players(&control).contains(&me))
            .map(|pending| view::SharedPart {
                decision: &pending.decision,
                view: self
                    .sharing
                    .view(pending, &control, &me, &moment.observation),
                waits: self.sharing.active(pending),
            })
            .collect();
        let mine = control.held_by(id);
        let partner = self
            .partners
            .iter_mut()
            .find(|partner| partner.id == id)
            .expect("A known partner");
        partner.heard = now;
        partner.reported = mine.clone();
        partner.seen = moment
            .pending
            .iter()
            .map(|pending| (pending.key.clone(), pending.stamp))
            .collect();
        for who in &mine {
            if !partner.played.contains(who) {
                partner.played.push(*who);
            }
        }
        let mut view = view::build(
            table.session(),
            &control,
            &moment.routing,
            view::Viewer {
                partner: id,
                sent: &mut partner.sent,
                board: &mut partner.board,
                full,
            },
            &moment.awaited,
            &shared,
        );
        // Table talk is outside the board: each message from another player once,
        // with the first answer after it that carries a view. A complete view gives
        // back the latest heard since joining, which a lost answer may have carried.
        let talk = if full {
            let mut talk = self.talk.since(partner.joined_talk, id);
            talk.drain(..talk.len().saturating_sub(RECOVERED_TALK));
            talk
        } else {
            self.talk.since(partner.told_talk, id)
        };
        partner.told_talk = self.talk.said().len();
        if !talk.is_empty() {
            view["table_talk"] = Value::Array(talk);
        }
        let mut answer = json!({
            "ok": error.is_none(),
            "mode": "partner",
            "status": status,
            "partner": id,
            "investigators": mine,
            "view": view,
        });
        if let Some(error) = error {
            answer["error"] = json!(error);
        }
        answer
    }
}

/// The pending decision at one revision as the host routes it.
struct Moment {
    observation: crate::gathering::Observation,
    routing: Routing,
    /// Its shared decisions that span players.
    pending: Vec<shared::Pending>,
    /// The players it waits for, each with the investigator it answers for.
    awaited: Vec<(Controller, Option<Investigator>)>,
}

impl Moment {
    /// What a held wait of `partner` answers now, at a table that has presented its
    /// decision: a group
    /// choice it has not been told of, news of a shared decision it takes part in
    /// that changed since its last view, or a decision of its own.
    fn event<R>(&self, host: &Host<R>, partner: &str) -> Option<&'static str> {
        if host.group_untold(partner, &self.routing, self.observation.revision) {
            return Some("group_choice");
        }
        let me = Controller::Partner(partner.to_owned());
        let control = host.control();
        let seen = &host.partners.iter().find(|known| known.id == partner)?.seen;
        let news = self.pending.iter().any(|pending| {
            pending.decision.players(&control).contains(&me)
                && seen
                    .iter()
                    .any(|(key, stamp)| *key == pending.key && *stamp != pending.stamp)
        });
        if news {
            Some("shared")
        } else if self.awaited.iter().any(|(player, _)| *player == me) {
            Some("decision")
        } else {
            None
        }
    }

    /// The commitment window `player` takes part in, where each of its options
    /// changes its part.
    fn changes(&self, control: &Control, player: &Controller) -> Option<&shared::Pending> {
        self.pending.iter().find(|pending| {
            pending.decision.kind == SharedKind::Commitment
                && pending.decision.players(control).contains(player)
        })
    }
}

fn refused(reason: String) -> Value {
    json!({"ok": false, "mode": "partner", "status": "refused", "error": reason})
}

fn closed(reason: &str) -> Value {
    json!({"ok": true, "mode": "partner", "status": "closed", "reason": reason})
}

/// A partner identity no earlier session at this table can have held.
fn token() -> String {
    use std::io::Read;
    let mut bytes = [0u8; 6];
    let random =
        std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes));
    if random.is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        bytes.copy_from_slice(&nanos.to_le_bytes()[..6]);
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
