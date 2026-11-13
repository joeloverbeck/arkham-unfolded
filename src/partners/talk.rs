//! Table talk (#209): every player may speak, and every player hears every message,
//! named only by the speaker's investigators. A partner speaks with a `say` request;
//! the human through the table's message box. Talk can persuade but never decides:
//! it is kept for this sitting only, never saved with the attempt and never rules
//! evidence.
//!
//! No one speaks while a peril encounter resolves (play contract, after RR p.16).
//! Exchanges are limited until play moves on or another player speaks. Between
//! committed decisions, a partner may send at most [`CHATTER_LIMIT`] messages in a
//! row unless another player has spoken since its last one. And once
//! [`EXCHANGE_LIMIT`] messages have been said since the latest message of a player
//! new to that decision's talk, partners wait until play moves on or a player who
//! has not spoken at it speaks. Every player's messages count alike, so the limits
//! reveal nothing of who is human; only the human is never refused.

use super::Table;
use crate::gathering::{Investigator, Peril, Session};
use serde_json::{Value, json};

/// The longest message, in characters: one short message.
pub const LONGEST_MESSAGE: usize = 280;
/// How many messages a partner may send in a row between committed decisions,
/// unless another player has spoken since its last one.
pub const CHATTER_LIMIT: usize = 3;
/// How many messages an exchange holds: at one decision, counted from the latest
/// message of a player who had not spoken at it before.
pub const EXCHANGE_LIMIT: usize = 6;
/// Why no one may speak; the table's message box says the same.
pub const PERIL: &str = "Table talk is closed: no one may speak while a peril encounter resolves (Rules Reference p. 16). It opens again once the encounter has resolved.";

/// One message, as every player hears it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    /// The speaker's investigators in player order; a speech box shows beside the first.
    pub speaker: Vec<Investigator>,
    pub text: String,
    /// The round it was said in; none during setup.
    pub round: Option<u16>,
    /// The partner who said it; none for the human.
    from: Option<String>,
    /// The decision it was said at.
    revision: u64,
}

impl Said {
    /// The message for a partner: its words and its speaker's investigators.
    fn heard(&self) -> Value {
        let mut said = json!({
            "speaker": self.speaker.iter().map(|who| who.name()).collect::<Vec<_>>(),
            "text": self.text,
        });
        if let Some(round) = self.round {
            said["round"] = json!(round);
        }
        said
    }
}

/// The sitting's table talk.
#[derive(Default)]
pub(super) struct Talk {
    said: Vec<Said>,
    /// The decision whose presentation the table was last seen to have finished: a
    /// peril encounter that decision drew has been shown and is over.
    presented_at: Option<u64>,
}

impl Talk {
    pub fn said(&self) -> &[Said] {
        &self.said
    }

    /// Why no one may speak now, if no one may.
    pub fn closed(&mut self, table: &impl Table) -> Option<&'static str> {
        self.watch(table);
        let session = table.session();
        match session.peril() {
            Some(Peril::Resolving) => Some(PERIL),
            Some(Peril::Resolved) if self.presented_at != Some(session.revision()) => Some(PERIL),
            _ => None,
        }
    }

    /// Note whether the table has finished presenting the latest decision. Talk
    /// itself never waits for the table: it decides nothing.
    pub fn watch(&mut self, table: &impl Table) {
        if table.presented() {
            self.presented_at = Some(table.session().revision());
        }
    }

    /// Add one message by `from` (a partner, or the human when none) for `speaker`,
    /// or say why it may not be said.
    pub fn say(
        &mut self,
        from: Option<&str>,
        speaker: Vec<Investigator>,
        text: &str,
        table: &impl Table,
    ) -> Result<(), String> {
        if speaker.is_empty() {
            return Err(match from {
                Some(_) => "You play no investigator now, so you have no seat at the table to speak from.".into(),
                None => "Every investigator is played by a partner: take one over to speak at the table.".into(),
            });
        }
        if let Some(reason) = self.closed(table) {
            return Err(reason.into());
        }
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            return Err("Say something: the message is empty.".into());
        }
        let length = text.chars().count();
        if length > LONGEST_MESSAGE {
            return Err(format!(
                "A message holds at most {LONGEST_MESSAGE} characters; this one has {length}. Say it more briefly."
            ));
        }
        let session = table.session();
        let revision = session.revision();
        if from.is_some() {
            let run = self
                .said
                .iter()
                .rev()
                .take_while(|said| said.from.as_deref() == from && said.revision == revision)
                .count();
            if run >= CHATTER_LIMIT {
                return Err(format!(
                    "You have sent {run} messages in a row since the last decision, the chatter limit of {CHATTER_LIMIT}. Speak again once play moves on or another player speaks."
                ));
            }
            let (speakers, heard) = self.exchange(revision);
            if speakers.contains(&from) && heard >= EXCHANGE_LIMIT {
                return Err(format!(
                    "The table has heard {heard} messages since a player new to this exchange spoke, the exchange limit of {EXCHANGE_LIMIT}. Speak again once play moves on or a player who has not spoken since the last decision speaks."
                ));
            }
        }
        self.said.push(Said {
            speaker: in_player_order(session, speaker),
            text,
            round: session.observe().play.map(|play| play.round),
            from: from.map(str::to_owned),
            revision,
        });
        Ok(())
    }

    /// The players who have spoken since the decision at `revision` was reached,
    /// and how many messages have been said since the latest first message of one
    /// of them.
    fn exchange(&self, revision: u64) -> (Vec<Option<&str>>, usize) {
        let at_decision = self
            .said
            .iter()
            .rev()
            .take_while(|said| said.revision == revision)
            .count();
        let mut speakers = Vec::new();
        let mut heard = 0;
        for said in &self.said[self.said.len() - at_decision..] {
            let from = said.from.as_deref();
            if !speakers.contains(&from) {
                speakers.push(from);
                heard = 0;
            }
            heard += 1;
        }
        (speakers, heard)
    }

    /// The messages from `told` on that `listener` did not say itself.
    pub fn since(&self, told: usize, listener: &str) -> Vec<Value> {
        self.said
            .get(told..)
            .unwrap_or_default()
            .iter()
            .filter(|said| said.from.as_deref() != Some(listener))
            .map(Said::heard)
            .collect()
    }

    /// Whether `listener` has a message from `told` on to hear.
    pub fn unheard(&self, told: usize, listener: &str) -> bool {
        self.said
            .get(told..)
            .unwrap_or_default()
            .iter()
            .any(|said| said.from.as_deref() != Some(listener))
    }
}

/// `who` in player order: from the current lead, then in seating order (RR p.13).
/// Before the lead is chosen, in seating order.
fn in_player_order(session: &Session, mut who: Vec<Investigator>) -> Vec<Investigator> {
    let observation = session.observe();
    let lead = observation
        .play
        .as_ref()
        .map(|play| play.lead)
        .or(observation.lead)
        .map_or(0, Investigator::index);
    who.sort_by_key(|who| (who.index() + Investigator::ALL.len() - lead) % Investigator::ALL.len());
    who
}
