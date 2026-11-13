//! Agent partners (#202, [ADR 0007]): players who play some investigators inside the
//! human's running game. This module is what a partner may do and see, without any
//! window: who controls each investigator, which options each player may submit, the
//! player-scoped view, and the partner protocol, one JSON request and one JSON answer
//! per line. The table hosts the transport and the presentation
//! (`gathering_tabletop::partners`). Deleting this module, that one, the tools'
//! `partner` command and the integration points that name them removes partners.
//!
//! A partner joins, which claims investigators the human handed to partners; waits
//! until one of them has a decision pending at a settled table; observes its view;
//! chooses an option by identity at the current revision; and leaves. A choice is
//! applied only once the table settles, through the table's own commit path. Group
//! choices stay with the human in this slice (#205). In a shared decision that
//! spans players, each player sets its own part and marks itself ready, and the
//! host completes it once every participating player is ready (#208, `shared`).
//!
//! [ADR 0007]: ../../docs/adr/0007-agent-partners-join-the-running-table.md

mod routing;
mod shared;
mod view;

pub use routing::{Routing, Shared, SharedKind};
pub use shared::{REOPENINGS, Readiness, SharedState};

use crate::gathering::{Investigator, OptionOwner, Session};
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
    /// partners every option is the human's.
    pub fn human_may(&self, session: &Session, id: &str) -> bool {
        if self.solo() {
            return true;
        }
        let routing = Routing::of(session, self);
        if routing.shared_of(id).is_some() {
            return routing.may(session, self, &Controller::Human, id);
        }
        routing
            .controller(session, self, id)
            .is_none_or(|controller| controller == Controller::Human)
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
    /// The table has finished presenting and would take the human's click now.
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
}

struct Partner {
    id: String,
    /// The investigators this partner was last told it plays.
    reported: Vec<Investigator>,
    /// Every investigator this partner has played, to say when one was taken over.
    played: Vec<Investigator>,
    /// Card text already sent, by code and face.
    sent: BTreeSet<String>,
    heard: Instant,
    /// The shared decisions this partner's last view showed, and their state then.
    seen: Vec<(shared::Key, shared::Stamp)>,
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
        #[serde(default)]
        all_cards: bool,
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

    /// Whether [`Self::poll`] would answer a held request, or complete a shared
    /// decision, now.
    pub fn due(&self, session: &Session, settled: bool, now: Instant) -> bool {
        if settled && self.completion(session).is_some() {
            return true;
        }
        let mut moment: Option<Moment> = None;
        self.held.iter().any(|held| match &held.pending {
            Pending::Choose { .. } => settled,
            Pending::Wait { deadline } => {
                now >= *deadline
                    || self.changed(&held.partner)
                    || (settled
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
                    "Not a partner request ({error}). A partner may join, wait, observe, choose, ready or leave."
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
            | Request::Choose { partner, .. }
            | Request::Ready { partner, .. }
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
            Request::Observe { all_cards, .. } => {
                vec![(
                    reply,
                    self.answer(&id, "observed", None, table, all_cards, now),
                )]
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
    /// held request whose moment has come: a wait whose partner has a decision or
    /// news of a shared decision at a settled table, whose control changed or
    /// whose period ran out, and a choice once the table has settled.
    pub fn poll(&mut self, table: &mut impl Table, now: Instant) -> Vec<Answer<R>> {
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
        // Routed only once the table has settled, and again after a commit.
        let mut moment: Option<Moment> = None;
        while index < self.held.len() {
            let partner = self.held[index].partner.clone();
            let settled = table.settled();
            let event = match &self.held[index].pending {
                Pending::Wait { .. } if settled && !self.changed(&partner) => moment
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
                "No investigator is open to partners right now. Ask at the table for one to be handed to partners (Menu, Agent partners)."
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
            heard: now,
            seen: Vec::new(),
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
        let Some(controller) = routing.controller(session, &control, id) else {
            return Err("That choice is not available at the current decision.".into());
        };
        let owner = routing.owner(session, id);
        let named = owner
            .as_ref()
            .map(routing::investigators)
            .unwrap_or_default();
        if controller != me {
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
            return Err(match owner {
                Some(OptionOwner::Table) => {
                    "That choice is made at the table: the lead investigator is chosen there."
                        .into()
                }
                Some(OptionOwner::Group { .. }) => {
                    "That choice is a group choice, made at the table in this version.".into()
                }
                _ => format!(
                    "That choice belongs to {}, whom you do not play.",
                    named
                        .iter()
                        .map(|who| who.name())
                        .collect::<Vec<_>>()
                        .join(" and ")
                ),
            });
        }
        Ok(match owner {
            Some(OptionOwner::Investigator(who)) => who,
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
        all_cards: bool,
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
        let view = view::build(
            table.session(),
            &control,
            &moment.routing,
            view::Viewer {
                partner: id,
                sent: &mut partner.sent,
                all_cards,
            },
            &moment.awaited,
            &shared,
        );
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
    /// What a held wait of `partner` answers now, at a settled table: news of a
    /// shared decision it takes part in, changed since its last view, or a
    /// decision of its own.
    fn event<R>(&self, host: &Host<R>, partner: &str) -> Option<&'static str> {
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
