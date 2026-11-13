//! Shared decisions between players (#208, play contract "Shared decisions"). The
//! rules session keeps its own representation of a commitment window, a window
//! for fast cards and abilities, and a group clue payment. Co-play adds each
//! participating player's readiness, a bound on reopenings, and a payment's
//! pending clue contributions until it is paid in one choice.
//!
//! Readiness is held against a stamp: the session's revision, and for a payment
//! the number of contribution changes at that revision. Every change moves the
//! stamp, so it clears the other players' readiness at once, and a decision
//! completes only when every participating player is ready on the same stamp.
//! An action in a window moves the revision too, which restarts its succession of
//! passes.

use super::routing::{Shared, SharedKind};
use super::{Control, Controller};
use crate::gathering::{Allocation, GroupCluePayment, Investigator, Observation};
use serde_json::{Value, json};

/// How many times one player may reopen a shared decision: change its part after
/// having been ready in it. At the limit it may only mark itself ready, so two
/// players cannot keep answering each other's changes.
pub const REOPENINGS: u8 = 3;

/// The Barrier's round-end alternative to paying its clues.
const DECLINE: &str = "round-end:decline-act";

/// Which shared decision a record belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Key {
    /// The commitment window of the next test to complete; it lasts while its
    /// cards are toggled.
    Commitment { test: usize },
    /// A window at one revision: an action ends it and opens the next.
    Window { close: String, revision: u64 },
    /// A payment's contributions at one revision.
    Payment { revision: u64 },
}

impl Key {
    fn of(decision: &Shared, observation: &Observation) -> Self {
        match decision.kind {
            SharedKind::Commitment => Self::Commitment {
                test: observation
                    .play
                    .as_ref()
                    .map_or(0, |play| play.completed_tests.len()),
            },
            SharedKind::Window => Self::Window {
                close: decision.close.clone(),
                revision: observation.revision,
            },
            SharedKind::Payment { .. } => Self::Payment {
                revision: observation.revision,
            },
        }
    }

    fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// The state a player is ready on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    revision: u64,
    changes: u64,
}

/// One shared decision's readiness, reopenings and pending contributions.
#[derive(Debug)]
struct Record {
    key: Key,
    ready: Vec<(Controller, Stamp)>,
    /// Every player who has been ready in it, and how often it reopened it since.
    reopened: Vec<(Controller, u8)>,
    /// The player whose change came last.
    changed_by: Option<Controller>,
    /// A payment's pending clues, in player-card order.
    clues: [u8; 3],
    /// Contribution changes at this revision.
    changes: u64,
}

impl Record {
    fn new(key: Key) -> Self {
        Self {
            key,
            ready: Vec::new(),
            reopened: Vec::new(),
            changed_by: None,
            clues: [0; 3],
            changes: 0,
        }
    }

    fn stamp(&self, revision: u64) -> Stamp {
        Stamp {
            revision,
            changes: self.changes,
        }
    }

    fn is_ready(&self, player: &Controller, stamp: Stamp) -> bool {
        self.ready
            .iter()
            .any(|(ready, at)| ready == player && *at == stamp)
    }

    fn reopenings(&self, player: &Controller) -> Option<u8> {
        self.reopened
            .iter()
            .find(|(reopener, _)| reopener == player)
            .map(|(_, count)| *count)
    }
}

/// One pending shared decision, with its record's identity and current stamp.
#[derive(Clone, Debug)]
pub(super) struct Pending {
    pub decision: Shared,
    pub key: Key,
    pub stamp: Stamp,
}

/// What one participating player has done in a shared decision, for the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Readiness {
    pub player: Controller,
    /// That player's investigators taking part, in player-card order.
    pub investigators: Vec<Investigator>,
    pub ready: bool,
    /// It was ready, and another player's change cleared it: whose investigators.
    pub cleared_by: Option<Vec<Investigator>>,
    pub reopenings_left: u8,
}

/// A pending shared decision as the table shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedState {
    pub kind: SharedKind,
    pub close: String,
    pub players: Vec<Readiness>,
    /// A payment's pending clues, in player-card order.
    pub clues: Allocation,
    /// Whether it waits on its players: a commitment or a window always does, a
    /// payment during a turn once anyone has proposed a contribution.
    pub active: bool,
}

impl SharedState {
    /// The participating players' investigators who are not yet ready.
    pub fn outstanding(&self) -> Vec<Investigator> {
        self.players
            .iter()
            .filter(|player| !player.ready)
            .flat_map(|player| player.investigators.iter().copied())
            .collect()
    }

    pub fn of(&self, player: &Controller) -> Option<&Readiness> {
        self.players
            .iter()
            .find(|readiness| readiness.player == *player)
    }
}

/// The co-play records of the pending shared decisions: at most one of each
/// kind, replaced when another decision of that kind appears.
#[derive(Debug, Default)]
pub(super) struct Sharing {
    records: Vec<Record>,
    /// Moves with every readiness or contribution change, for the table's display.
    version: u64,
}

impl Sharing {
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Whether any player is ready on a state at this revision: the cheap test
    /// before looking for a decision to complete.
    pub fn ready_at(&self, revision: u64) -> bool {
        self.records.iter().any(|record| {
            record
                .ready
                .iter()
                .any(|(_, stamp)| stamp.revision == revision)
        })
    }

    /// The pending shared decisions that span players, and their stamps.
    pub fn pending(&self, shared: &[Shared], observation: &Observation) -> Vec<Pending> {
        shared
            .iter()
            .map(|decision| {
                let key = Key::of(decision, observation);
                let changes = self.find(&key).map_or(0, |record| record.changes);
                Pending {
                    decision: decision.clone(),
                    key,
                    stamp: Stamp {
                        revision: observation.revision,
                        changes,
                    },
                }
            })
            .collect()
    }

    fn find(&self, key: &Key) -> Option<&Record> {
        self.records.iter().find(|record| record.key == *key)
    }

    fn record(&mut self, key: &Key) -> &mut Record {
        if let Some(index) = self.records.iter().position(|record| record.key == *key) {
            return &mut self.records[index];
        }
        self.records.retain(|record| !record.key.same_kind(key));
        self.records.push(Record::new(key.clone()));
        self.records.last_mut().expect("Just pushed")
    }

    pub fn is_ready(&self, pending: &Pending, player: &Controller) -> bool {
        self.find(&pending.key)
            .is_some_and(|record| record.is_ready(player, pending.stamp))
    }

    /// Whether the decision waits on its players now.
    pub fn active(&self, pending: &Pending) -> bool {
        match pending.decision.kind {
            SharedKind::Payment {
                at_round_end: false,
            } => self.find(&pending.key).is_some_and(|record| {
                record.clues != [0; 3]
                    || record
                        .ready
                        .iter()
                        .any(|(_, stamp)| *stamp == pending.stamp)
            }),
            _ => true,
        }
    }

    /// The participating players not yet ready, each with its first investigator
    /// taking part, while the decision waits on them.
    pub fn waiting(&self, pending: &Pending, control: &Control) -> Vec<(Controller, Investigator)> {
        if !self.active(pending) {
            return Vec::new();
        }
        let mut waiting: Vec<(Controller, Investigator)> = Vec::new();
        for who in &pending.decision.participants {
            let player = control.controller(*who);
            if !self.is_ready(pending, &player)
                && !waiting.iter().any(|(listed, _)| *listed == player)
            {
                waiting.push((player, *who));
            }
        }
        waiting
    }

    /// Why `player` may not change its part now: the reopen bound.
    pub fn may_change(&self, pending: &Pending, player: &Controller) -> Result<(), String> {
        let reopenings = self
            .find(&pending.key)
            .and_then(|record| record.reopenings(player));
        if !matches!(
            pending.decision.kind,
            SharedKind::Commitment | SharedKind::Payment { .. }
        ) || reopenings.is_none_or(|count| count < REOPENINGS)
        {
            return Ok(());
        }
        Err(format!(
            "You have reopened this shared decision {REOPENINGS} times, the limit: you can no longer change your part, only mark yourself ready on the current contributions."
        ))
    }

    /// Record `player`'s change to its part: a reopening once it has been ready.
    pub fn change(&mut self, pending: &Pending, player: &Controller) -> Result<(), String> {
        self.may_change(pending, player)?;
        let record = self.record(&pending.key);
        if let Some((_, count)) = record
            .reopened
            .iter_mut()
            .find(|(reopener, _)| reopener == player)
        {
            *count += 1;
        }
        record.changed_by = Some(player.clone());
        self.version += 1;
        Ok(())
    }

    /// Mark `player` ready on the current state.
    pub fn ready(&mut self, pending: &Pending, player: &Controller) {
        let record = self.record(&pending.key);
        let stamp = record.stamp(pending.stamp.revision);
        record.ready.retain(|(ready, _)| ready != player);
        record.ready.push((player.clone(), stamp));
        if record.reopenings(player).is_none() {
            record.reopened.push((player.clone(), 0));
        }
        self.version += 1;
    }

    /// Set `player`'s part of a payment from a named allocation and mark it
    /// ready: the other players' parts must be as pending, so nothing is rebased.
    pub fn contribute(
        &mut self,
        pending: &Pending,
        payment: &GroupCluePayment,
        control: &Control,
        player: &Controller,
        id: &str,
    ) -> Result<(), String> {
        let current = self
            .find(&pending.key)
            .map_or([0; 3], |record| record.clues);
        let named = if id == DECLINE {
            let mut named = current;
            for who in Investigator::ALL {
                if control.controller(who) == *player {
                    named[who.index()] = 0;
                }
            }
            named
        } else {
            let named = GroupCluePayment::allocation_of(id).ok_or_else(|| {
                format!("A clue contribution is named act:pay:TONY:PATRICE:JACQUELINE, not {id}.")
            })?;
            Investigator::ALL.map(|who| named[who])
        };
        for who in Investigator::ALL {
            let index = who.index();
            if control.controller(who) == *player {
                let limit = payment.contributors[index].limit(payment.required);
                if named[index] > limit {
                    return Err(format!(
                        "{} can contribute at most {limit} clues to this payment.",
                        who.name()
                    ));
                }
            } else if named[index] != current[index] {
                return Err(format!(
                    "{}'s pending contribution is {}, not {}: another player sets it. Choose again from the current view.",
                    who.name(),
                    current[index],
                    named[index]
                ));
            }
        }
        let total: u16 = named.iter().map(|&clues| u16::from(clues)).sum();
        if total > u16::from(payment.required) {
            return Err(format!(
                "That allocation spends {total} clues; the payment requires exactly {}.",
                payment.required
            ));
        }
        if named != current {
            self.change(pending, player)?;
            let record = self.record(&pending.key);
            record.clues = named;
            record.changes += 1;
        }
        let mut moved = pending.clone();
        moved.stamp = self.record(&pending.key).stamp(pending.stamp.revision);
        self.ready(&moved, player);
        Ok(())
    }

    /// The choice that completes the decision, and the investigator it is
    /// attributed to, once every participating player is ready on the same state.
    pub fn completion(
        &self,
        pending: &Pending,
        control: &Control,
        observation: &Observation,
    ) -> Option<(String, Investigator)> {
        if !self.active(pending) || !self.waiting(pending, control).is_empty() {
            return None;
        }
        let decision = &pending.decision;
        let first = *decision.participants.first()?;
        match decision.kind {
            SharedKind::Commitment => {
                let tester = observation
                    .play
                    .as_ref()
                    .and_then(|play| play.test.as_ref())
                    .map_or(first, |test| test.tester);
                Some((decision.close.clone(), tester))
            }
            SharedKind::Window => Some((
                decision.close.clone(),
                observation.decision_owner.unwrap_or(first),
            )),
            SharedKind::Payment { at_round_end } => {
                let clues = self.find(&pending.key)?.clues;
                let payment = observation.group_clue_payment.as_ref()?;
                if let Some(id) = payment.choice_for(clues) {
                    let payer = Investigator::ALL
                        .into_iter()
                        .find(|who| clues[who.index()] > 0)
                        .unwrap_or(first);
                    return Some((id, payer));
                }
                (at_round_end && clues == [0; 3]).then(|| (DECLINE.to_owned(), first))
            }
        }
    }

    /// Forget the readiness of a decision whose completing choice failed, so the
    /// table does not retry it every frame.
    pub fn unready(&mut self, pending: &Pending) {
        self.record(&pending.key).ready.clear();
        self.version += 1;
    }

    /// The decision as the table shows it.
    pub fn state(&self, pending: &Pending, control: &Control) -> SharedState {
        let record = self.find(&pending.key);
        let players = pending
            .decision
            .players(control)
            .into_iter()
            .map(|player| {
                let investigators: Vec<_> = pending
                    .decision
                    .participants
                    .iter()
                    .copied()
                    .filter(|who| control.controller(*who) == player)
                    .collect();
                let ready = self.is_ready(pending, &player);
                let was_ready = record
                    .is_some_and(|record| record.ready.iter().any(|(ready, _)| *ready == player));
                let cleared_by = record
                    .and_then(|record| record.changed_by.clone())
                    .filter(|changer| !ready && was_ready && *changer != player)
                    .map(|changer| {
                        pending
                            .decision
                            .participants
                            .iter()
                            .copied()
                            .filter(|who| control.controller(*who) == changer)
                            .collect()
                    });
                let reopenings_left = record
                    .and_then(|record| record.reopenings(&player))
                    .map_or(REOPENINGS, |count| REOPENINGS - count.min(REOPENINGS));
                Readiness {
                    player,
                    investigators,
                    ready,
                    cleared_by,
                    reopenings_left,
                }
            })
            .collect();
        SharedState {
            kind: pending.decision.kind,
            close: pending.decision.close.clone(),
            players,
            clues: record.map_or([0; 3], |record| record.clues).into(),
            active: self.active(pending),
        }
    }

    /// The decision as a partner's view gives it: who takes part and is ready, by
    /// investigator, the contributions pending, and how to mark itself ready.
    pub fn view(
        &self,
        pending: &Pending,
        control: &Control,
        me: &Controller,
        observation: &Observation,
    ) -> Value {
        let state = self.state(pending, control);
        let kind = match state.kind {
            SharedKind::Commitment => "commitment",
            SharedKind::Window => "window",
            SharedKind::Payment { .. } => "clue_payment",
        };
        let players: Vec<Value> = state
            .players
            .iter()
            .map(|player| {
                let mut entry = json!({
                    "investigators": player.investigators.iter().map(|who| who.name()).collect::<Vec<_>>(),
                    "ready": player.ready,
                });
                if player.player == *me {
                    entry["yours"] = json!(true);
                }
                entry
            })
            .collect();
        let mut value = json!({
            "kind": kind,
            "ready_with": state.close,
            "players": players,
            "waiting_for": state.outstanding().iter().map(|who| who.name()).collect::<Vec<_>>(),
        });
        if let Some(mine) = state.of(me) {
            value["you_are_ready"] = json!(mine.ready);
            if let Some(changer) = &mine.cleared_by {
                value["cleared_by"] =
                    json!(changer.iter().map(|who| who.name()).collect::<Vec<_>>());
            }
            if matches!(
                state.kind,
                SharedKind::Commitment | SharedKind::Payment { .. }
            ) {
                value["reopenings_left"] = json!(mine.reopenings_left);
            }
        }
        match state.kind {
            SharedKind::Commitment => {
                value["how"] = json!(
                    "Choose your cards' commit options to toggle them, then mark yourself ready (op ready, or choose ready_with). Any change clears the other players' readiness."
                );
            }
            SharedKind::Window => {
                value["how"] = json!(
                    "Use an option, or pass (op ready, or choose ready_with). The window closes once every player with an option has passed in succession; an action restarts it."
                );
            }
            SharedKind::Payment { at_round_end } => {
                if let Some(payment) = &observation.group_clue_payment {
                    value["required"] = json!(payment.required);
                    value["contributions"] = Value::Array(
                        payment
                            .contributors
                            .iter()
                            .map(|contributor| {
                                let who = contributor.investigator;
                                json!({
                                    "investigator": who,
                                    "name": who.name(),
                                    "clues": state.clues[who],
                                    "holds": contributor.clues,
                                    "limit": contributor.limit(payment.required),
                                    "yours": control.controller(who) == *me,
                                })
                            })
                            .collect(),
                    );
                }
                value["how"] = json!(if at_round_end {
                    "Choose act:pay:TONY:PATRICE:JACQUELINE with your investigators' clues (the others' as pending) to set your part and be ready, or choose round-end:decline-act to contribute none. It pays once every player is ready on exactly the requirement, and declines once every player is ready on no clues."
                } else {
                    "Choose act:pay:TONY:PATRICE:JACQUELINE with your investigators' clues (the others' as pending) to set your part and be ready. It pays once every player is ready on exactly the requirement."
                });
            }
        }
        value
    }
}
