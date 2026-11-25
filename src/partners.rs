//! Agent partners (#202, [ADR 0007]): players who play some investigators inside the
//! human's running game. This module is what a partner may do and see, without any
//! window: who controls each investigator, which options each player may submit, the
//! player-scoped view, and the partner protocol, one JSON request and one JSON answer
//! per line. The table hosts the transport and the presentation
//! (`gathering_tabletop::partners`). Deleting this module, that one, the tools'
//! `partner` command and the integration points that name them removes partners.
//!
//! A partner joins, which claims the one investigator it names, of those the human
//! handed to partners: one session plays one investigator (#295, [ADR 0009]). It
//! waits until that investigator has a decision pending once the table has presented
//! it (a reading or the human's dialog may still be open, #210), or listens to the
//! table without being answered its own decision; observes its view,
//! complete once and then as changes (#206); inspects any card it can see; chooses
//! an option by identity at the current revision; and leaves. Its session ends when
//! it leaves, the human takes its investigator back or turns partners off, the table
//! closes or the attempt restarts. A choice is
//! applied only once the table settles, with nothing of the human's open over it,
//! through the table's own commit path; its answer does not wait for that unless the
//! partner asks to wait too. Who holds
//! each investigator, the held choices and the waits are `unfolded_agent_players::seats`,
//! which no game owns ([ADR 0010]); what follows here is Arkham's. A group
//! choice is its arbiter's player's, and every partner's wait hears it open (#207).
//! In a shared decision that spans players, each player sets its own part and marks
//! itself ready, and the host completes it once every participating player is
//! ready (#208, `shared`). When a decision lists another player's option beside
//! its own investigator's and no pass, such as before each action of a turn, the
//! deciding investigator's player's choice waits until every other player with an
//! option there has passed (F1, S1). Every player may speak, and every other
//! player hears each message (#209, `talk`); the human's ends a partner's wait, and a
//! partner's does not (#296).
//!
//! [ADR 0007]: ../../docs/adr/0007-agent-partners-join-the-running-table.md
//! [ADR 0009]: ../../docs/adr/0009-one-agent-session-plays-one-investigator.md
//! [ADR 0010]: ../../docs/adr/0010-seat-ownership-and-waiting-move-to-the-shared-library.md

mod routing;
mod shared;
mod talk;
mod view;

pub use routing::{PASS_BEFORE, Routing, Shared, SharedKind};
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
use unfolded_agent_players::seats::{
    ClaimRefused, Ended, Event, Game, Holder, Joining, Pending, Refusal, Released, Seats,
    Submitted, Told, Waiting,
};

/// How many of the latest messages heard since joining a complete view gives back,
/// for a partner whose answer was lost to a cut-off command.
pub const RECOVERED_TALK: usize = 20;
/// The partner instructions in this repository, the one statement of the
/// protocol's rules (#211, #218): the pasted line points to them, and every hint
/// in a view names the section of them that holds its rules.
pub const INSTRUCTIONS: &str = "docs/agents/partner-instructions.md";

/// The end of a hint: the section of the instructions that holds its rules.
fn rules_in(section: &str) -> String {
    format!("See {section} in {INSTRUCTIONS}.")
}

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

/// Who controls each investigator. Runtime state of the session at the table; never
/// saved with the attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    controllers: [Controller; 3],
}

/// Without partners: every investigator is the human's.
impl Default for Control {
    fn default() -> Self {
        Self {
            controllers: [const { Controller::Human }; 3],
        }
    }
}

impl Control {
    pub fn controller(&self, who: Investigator) -> Controller {
        self.controllers[who.index()].clone()
    }

    /// Whether the human plays every investigator, as without partners.
    pub fn solo(&self) -> bool {
        *self == Self::default()
    }

    /// Whether the human may submit `id` at the session's pending decision: its
    /// own option, or its part in a shared decision it takes part in. Without
    /// partners every option is the human's. With partners, Undo is the human's only
    /// when the decision it would revert belongs to investigators the human controls.
    pub fn human_may(&self, session: &Session, id: &str) -> bool {
        self.solo() || Routing::of(session, self).human_may(session, self, id)
    }

    /// The options of the session's pending decision that the human may submit,
    /// as the table offers them (#205): every listed one without partners, and
    /// with partners those [`Self::human_may`] allows, as a partner's view lists
    /// only that partner's options, with his pass before another player's
    /// decision he holds an option in (F1).
    pub fn human_choices(&self, session: &Session) -> Vec<Choice> {
        if self.solo() {
            return session.observe().choices;
        }
        self.choices_of(session, &Controller::Human)
    }

    /// The options of the session's pending decision that `player` may submit:
    /// see [`Routing::choices_of`].
    pub fn choices_of(&self, session: &Session, player: &Controller) -> Vec<Choice> {
        Routing::of(session, self).choices_of(session, self, player)
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
    /// Who holds each investigator, with the sessions' waits and held choices.
    seats: Seats<Investigator, Chosen, R>,
    /// What the host keeps of each session that holds an investigator.
    partners: Vec<Partner>,
    /// Readiness, reopenings and pending contributions in shared decisions.
    sharing: shared::Sharing,
    talk: talk::Talk,
    /// The deciding investigator's player's choice while it waits for the other
    /// players with an option in that decision to pass (F1).
    hold: Option<Hold>,
    /// Why the human's held choice was released, at the revision of the
    /// decision he chooses again (review fix G2).
    human_released: Option<(u64, String)>,
}

/// A player's choice, with its label at its revision, to name it once the decision has
/// moved on.
#[derive(Clone)]
struct Chosen {
    id: String,
    label: String,
    /// It waits, or waited, for other players' passes, which the protocol calls held, and
    /// not for the table.
    for_passes: bool,
}

impl Chosen {
    fn new(observation: &crate::gathering::Observation, id: &str) -> Self {
        Self {
            id: id.to_owned(),
            label: label(observation, id),
            for_passes: false,
        }
    }
}

/// The option's label now, or its identity when it is not listed.
fn label(observation: &crate::gathering::Observation, id: &str) -> String {
    observation
        .choices
        .iter()
        .find(|choice| choice.id == id)
        .map_or(id, |choice| choice.label.as_str())
        .to_owned()
}

/// A choice of the deciding investigator's player, held at its revision until the
/// other players with an option in that decision have passed (F1). The holder's
/// next choice replaces it; a choice of the holder's applied at once drops it.
struct Hold {
    player: Controller,
    revision: u64,
    choice: Chosen,
}

impl Hold {
    fn new(player: Controller, observation: &crate::gathering::Observation, id: &str) -> Self {
        Self {
            player,
            revision: observation.revision,
            choice: Chosen {
                for_passes: true,
                ..Chosen::new(observation, id)
            },
        }
    }
}

/// How an accepted submission stands.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Standing {
    /// Its choice was applied.
    Applied,
    /// Its choice waits for the table to settle, and not for another player, as a held
    /// one does.
    Settling,
    /// Its choice is held for the other players' passes.
    Held,
    /// Its choice waited, either way, and was not applied.
    Released,
    /// It is a readiness in this shared decision.
    Ready(shared::Key),
}

/// A submission of a partner's that was accepted.
struct Submission {
    /// The choice it named, if it named one: its id, and its label at its revision.
    named: Option<(String, String)>,
    standing: Standing,
}

impl Submission {
    fn of(choice: &Chosen, standing: Standing) -> Self {
        Self {
            named: Some((choice.id.clone(), choice.label.clone())),
            standing,
        }
    }
}

impl Standing {
    /// Its word in the protocol, as an answer's status and where a wait that followed the
    /// submission says how it stands.
    fn word(&self) -> &'static str {
        match self {
            Self::Applied => "chosen",
            Self::Settling => "settling",
            Self::Held => "held",
            Self::Released => "released",
            Self::Ready(_) => "ready",
        }
    }
}

/// What a submission does once it is accepted.
#[derive(Clone, Copy)]
enum Then {
    /// It is answered.
    Answer,
    /// It waits, for this many seconds or without a period.
    Wait(Option<u64>),
}

impl Then {
    fn of(wait: bool, seconds: Option<u64>) -> Self {
        if wait {
            Self::Wait(seconds)
        } else {
            Self::Answer
        }
    }
}

/// What a held wait is for.
enum Purpose {
    /// The first thing that concerns its partner.
    Anything,
    /// The same, but for its partner's own decision that was pending as it began.
    Listening,
    /// What follows its partner's submission, the latest of them, of whose standing its
    /// answer tells.
    Following(Submission),
}

/// Something other than its own decision that ends a partner's wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum News {
    /// Its choice held for the other players' passes was applied.
    HeldApplied,
    GroupChoice,
    Shared,
    /// The human spoke.
    TableTalk,
}

struct Partner {
    id: String,
    /// Card text already sent, by code and face.
    sent: BTreeSet<String>,
    /// The board of the last view sent, which the next view's changes start from.
    board: Option<serde_json::Map<String, Value>>,
    /// The revision of the last group choice a wait reported to this partner.
    told_group: Option<u64>,
    /// The shared decisions this partner's last view showed, and their state then.
    seen: Vec<(shared::Key, shared::Stamp)>,
    /// The table talk this partner has been told, if only by a wait nobody may have read:
    /// messages before this index.
    told_talk: usize,
    /// The table talk said before this partner sat down: messages before this index.
    joined_talk: usize,
    /// A choice of its that other players' options held was applied, and no wait has
    /// told it.
    held_applied: bool,
    /// What its held wait is for, while one is held.
    purpose: Purpose,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Join {
        /// The one investigator to play. Any other number is refused by the host
        /// rather than as no request, so the refusal names those open to partners.
        #[serde(default)]
        investigators: Vec<Investigator>,
        /// The joining session's own identity, when it has one: an identity that
        /// plays an investigator may not join again.
        #[serde(default)]
        partner: Option<String>,
        /// The joining session's identifiers, when its tool gives any: a session that
        /// plays an investigator may not join again.
        #[serde(default)]
        session: Option<String>,
    },
    Wait {
        partner: String,
        #[serde(default)]
        timeout_seconds: Option<u64>,
        /// Hear the table while a decision of this partner's waits: answer anything else
        /// that ends a wait, a later decision of its own included, or the period's end
        /// (stories 28-29).
        #[serde(default)]
        listen: bool,
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
        /// Then wait, on the same answer, for the first thing that concerns the partner.
        #[serde(default)]
        wait: bool,
        /// The period of that wait.
        #[serde(default)]
        timeout_seconds: Option<u64>,
    },
    /// Readiness in the pending shared decision, on the state of the last view.
    Ready {
        partner: String,
        revision: u64,
        /// Then wait, on the same answer, for the first thing that concerns the partner.
        #[serde(default)]
        wait: bool,
        /// The period of that wait.
        #[serde(default)]
        timeout_seconds: Option<u64>,
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
            seats: Seats::new(Investigator::ALL),
            partners: Vec::new(),
            sharing: shared::Sharing::default(),
            talk: talk::Talk::default(),
            hold: None,
            human_released: None,
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
            controllers: Investigator::ALL.map(|who| match self.seats.holder(&who) {
                Holder::Human => Controller::Human,
                Holder::Unclaimed => Controller::Unclaimed,
                Holder::Session(id) => Controller::Partner(id),
            }),
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
            for who in Investigator::ALL {
                if self.selected[who.index()] {
                    self.seats.hand_over(&who);
                }
            }
            return Vec::new();
        }
        let reason = "Agent partners were turned off.";
        let answers = self.close(reason);
        for who in Investigator::ALL {
            // Closing ended the session that held it.
            let _ = self.seats.take_back(&who, reason);
        }
        answers
    }

    /// Hand `who` to partners, or return it to the human (Take over). The session
    /// that plays it ends at once: its held requests, and any later partner request
    /// by its identity except a join, are answered that it was taken back, and its
    /// held choice is not applied.
    #[must_use = "The answers to the ended session's held requests"]
    pub fn set_selected(&mut self, who: Investigator, selected: bool) -> Vec<Answer<R>> {
        self.selected[who.index()] = selected;
        if selected {
            if self.enabled {
                self.seats.hand_over(&who);
            }
            return Vec::new();
        }
        let Holder::Session(id) = self.seats.holder(&who) else {
            let _ = self.seats.take_back(&who, "");
            return Vec::new();
        };
        let me = Controller::Partner(id.clone());
        let held = self
            .hold
            .as_ref()
            .filter(|hold| hold.player == me)
            .map(|hold| format!(" Your held choice, {}, was not applied.", hold.choice.label));
        let settling = self
            .seats
            .held(&id)
            .map(|(_, choice)| format!(" Your choice, {}, was not applied.", choice.label));
        let reason = format!(
            "The table took {} over, so your session ended: stop.{}{}",
            who.name(),
            held.unwrap_or_default(),
            settling.unwrap_or_default()
        );
        let ended = self.seats.take_back(&who, &reason);
        self.ended(ended, &reason)
    }

    /// Forget sessions that ended for `reason`, dropping their held choices. The answers
    /// to their held waits.
    fn ended(
        &mut self,
        ended: impl IntoIterator<Item = Ended<Chosen, R>>,
        reason: &str,
    ) -> Vec<Answer<R>> {
        let mut answers = Vec::new();
        for ended in ended {
            self.partners.retain(|partner| partner.id != ended.identity);
            self.drop_hold(&Controller::Partner(ended.identity));
            answers.extend(ended.wait.map(|reply| (reply, closed(reason))));
        }
        answers
    }

    /// End every partner session: the table closed or partners were turned off.
    pub fn close(&mut self, reason: &str) -> Vec<Answer<R>> {
        let ended = self.seats.close(reason);
        self.ended(ended, reason)
    }

    /// Whether the client of a held wait has died, as `hung_up` says of its reply.
    pub fn hung_up(&self, hung_up: impl Fn(&R) -> bool) -> bool {
        self.seats.hung_up(hung_up)
    }

    /// End each held wait whose client has died, so that it tells nobody anything and its
    /// investigator's silence counts from now. The partner's session goes on.
    pub fn end_hung_up(&mut self, hung_up: impl Fn(&R) -> bool, now: Instant) {
        for id in self.seats.end_hung_up(hung_up, now) {
            if let Some(partner) = self.partner_mut(&id) {
                partner.purpose = Purpose::Anything;
            }
        }
    }

    /// How long since the partner holding `who` was last heard from; zero while its choice
    /// waits for the table, and while it waits, unless its wait cannot tell it the decision
    /// that the table waits for it to make, at the revision `decision`. `None` when no
    /// partner holds `who`.
    pub fn silence(
        &self,
        who: Investigator,
        decision: Option<u64>,
        now: Instant,
    ) -> Option<Duration> {
        self.seats.silence(&who, decision, now)
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

    /// Whether [`Self::poll`] would answer a held wait, settle a choice, or complete a
    /// shared decision, now, at a table that has `presented` its decision and is `settled`
    /// or not, as [`Table`] names them.
    pub fn due(&self, session: &Session, presented: bool, settled: bool, now: Instant) -> bool {
        if self.hold_due(session, settled) || (settled && self.completion(session).is_some()) {
            return true;
        }
        if self.seats.settle_due(session.revision(), settled) {
            return true;
        }
        let mut moment: Option<Moment> = None;
        self.partners
            .iter()
            .filter(|partner| self.seats.waits(&partner.id))
            .any(|partner| {
                let moment =
                    presented.then(|| &*moment.get_or_insert_with(|| self.moment(session)));
                self.seats
                    .would_tell(&partner.id, self.events(partner, moment), now)
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
        if moment.waits_for_window(&self.sharing, &control, &human, id) {
            // His click in his own decision is his part: it is applied once
            // every other player with an option there has passed on this
            // revision. A later click replaces it.
            self.hold = Some(Hold::new(human, &moment.observation, id));
            self.human_released = None;
            return Ok(HumanChoice::Ready);
        }
        // His own choice applied at once drops the one he held.
        self.drop_hold(&human);
        Ok(HumanChoice::Commit)
    }

    /// The human's choice waiting for the other players' passes in his own
    /// decision, at the session's revision.
    pub fn human_held(&self, session: &Session) -> Option<&str> {
        self.hold
            .as_ref()
            .filter(|hold| hold.player == Controller::Human && hold.revision == session.revision())
            .map(|hold| hold.choice.id.as_str())
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
            Request::Join {
                investigators,
                partner,
                session,
            } => {
                let joining = Joining {
                    identity: partner.as_deref(),
                    session: session.as_deref(),
                };
                let answer = self.join(investigators, joining, table, now);
                return vec![(reply, answer)];
            }
            Request::Wait { partner, .. }
            | Request::Observe { partner, .. }
            | Request::Inspect { partner, .. }
            | Request::Choose { partner, .. }
            | Request::Ready { partner, .. }
            | Request::Say { partner, .. }
            | Request::Leave { partner } => partner.clone(),
        };
        let Some(plays) = self.plays(&id) else {
            if let Some(reason) = self.seats.ended(&id) {
                return vec![(reply, closed(reason))];
            }
            return vec![(
                reply,
                refused(format!(
                    "No partner {id} plays at this table. Join first; a partner identity ends when its session does."
                )),
            )];
        };
        match request {
            Request::Join { .. } => unreachable!("Answered above"),
            Request::Observe { full, .. } => {
                vec![(reply, self.answer(&id, "observed", None, table, full, now))]
            }
            Request::Inspect { card, .. } => {
                vec![(reply, self.inspect(&id, plays, &card, table, now))]
            }
            Request::Say { text, .. } => {
                self.seats.heard(&id, now);
                let answer = match self.talk.say(Some(&id), vec![plays], &text, &*table) {
                    Ok(()) => self.answer(&id, "said", None, table, false, now),
                    Err(reason) => self.answer(&id, "refused", Some(reason), table, false, now),
                };
                vec![(reply, answer)]
            }
            Request::Leave { .. } => {
                let left = self.seats.leave(&id);
                let mut answers = self.ended(left, "You left the table.");
                answers.push((
                    reply,
                    json!({"ok": true, "mode": "partner", "status": "left", "partner": id}),
                ));
                answers
            }
            Request::Wait {
                timeout_seconds,
                listen,
                ..
            } => {
                self.advance(table);
                let moment = self.moment(table.session());
                let decision = self.own_decision(&moment, &id);
                let partner = self.partner(&id).expect("A known partner");
                let news = !self.news(partner, Some(&moment)).is_empty();
                let pending = match decision {
                    Some(revision) if listen => Pending::Before(revision),
                    // Other news ends it at once.
                    _ if news => Pending::Other,
                    Some(revision) => Pending::Decision(revision),
                    None => Pending::Nothing,
                };
                let period = timeout_seconds.map(Duration::from_secs);
                match self.seats.wait(&id, reply, period, pending, now) {
                    Waiting::Refused(reply) => {
                        let reason = format!(
                            "You have been told of your decision, and it still waits for you: decide it, or listen. {}",
                            rules_in("Wait")
                        );
                        vec![(
                            reply,
                            self.answer(&id, "refused", Some(reason), table, false, now),
                        )]
                    }
                    Waiting::Held { replaced } => {
                        let partner = self.partner_mut(&id).expect("A known partner");
                        partner.purpose = if listen {
                            Purpose::Listening
                        } else {
                            Purpose::Anything
                        };
                        let mut answers: Vec<_> = replaced
                            .map(|older| (older, replaced_wait(&id, BY_WAIT)))
                            .into_iter()
                            .collect();
                        answers.extend(self.poll(table, now));
                        answers
                    }
                }
            }
            Request::Ready {
                revision,
                wait,
                timeout_seconds,
                ..
            } => {
                let submission = self.ready(&id, revision, None, table);
                let answers = self.poll(table, now);
                self.submitted(
                    (&id, reply),
                    submission,
                    Then::of(wait, timeout_seconds),
                    answers,
                    table,
                    now,
                )
            }
            Request::Choose {
                revision,
                id: choice,
                wait,
                timeout_seconds,
                ..
            } => {
                let shared = self
                    .moment(table.session())
                    .routing
                    .shared_of(&choice)
                    .is_some();
                let submission = if shared {
                    self.ready(&id, revision, Some(&choice), table)
                } else {
                    self.choose(&id, revision, &choice, table)
                };
                // A shared decision's readiness may complete it before its answer.
                let answers = if shared {
                    self.poll(table, now)
                } else {
                    Vec::new()
                };
                self.submitted(
                    (&id, reply),
                    submission,
                    Then::of(wait, timeout_seconds),
                    answers,
                    table,
                    now,
                )
            }
        }
    }

    /// Answer a submission of the session `id` at once with how it stands, or, when it
    /// asked to wait too, for a period or without one, and its own next
    /// decision is not there to tell, hold its wait. An accepted one ends the session's
    /// listening wait, which was for what came before it. Then whatever else its
    /// submission let be answered, after `answers`.
    fn submitted(
        &mut self,
        (id, mut reply): (&str, R),
        submission: Result<Submission, String>,
        then: Then,
        mut answers: Vec<Answer<R>>,
        table: &impl Table,
        now: Instant,
    ) -> Vec<Answer<R>> {
        let moment = self.moment(table.session());
        let decision = self.own_decision(&moment, id);
        if let Some(revision) = decision {
            self.seats.told_in_answer(id, revision);
        }
        let mut accepted = None;
        let (status, error) = match submission {
            Err(reason) => ("refused", Some(reason)),
            Ok(submission) => {
                let status = submission.standing.word();
                let partner = self.partner_mut(id).expect("A known partner");
                match partner.purpose {
                    Purpose::Following(_) => partner.purpose = Purpose::Following(submission),
                    _ => accepted = Some(submission),
                }
                if matches!(partner.purpose, Purpose::Listening) {
                    partner.purpose = Purpose::Anything;
                    let older = self.seats.end_wait(id);
                    answers.extend(older.map(|older| (older, replaced_wait(id, BY_CHOICE))));
                }
                (status, None)
            }
        };
        if error.is_none()
            && let Then::Wait(seconds) = then
        {
            let period = seconds.map(Duration::from_secs);
            let pending = decision.map_or(Pending::Nothing, Pending::Decision);
            match self.seats.wait(id, reply, period, pending, now) {
                Waiting::Held { replaced } => {
                    if let Some(submission) = accepted {
                        let partner = self.partner_mut(id).expect("A known partner");
                        partner.purpose = Purpose::Following(submission);
                    }
                    answers.extend(replaced.map(|older| (older, replaced_wait(id, BY_WAIT))));
                    answers.extend(self.answer_waits(table, now));
                    return answers;
                }
                Waiting::Refused(told) => reply = told,
            }
        }
        let answer = self.answer(id, status, error, table, false, now);
        answers.insert(0, (reply, answer));
        answers.extend(self.answer_waits(table, now));
        answers
    }

    /// A partner's choice: applied at once at a settled table, held until it settles
    /// otherwise, or held for the other players' passes (F1).
    fn choose(
        &mut self,
        id: &str,
        revision: u64,
        choice: &str,
        table: &mut impl Table,
    ) -> Result<Submission, String> {
        let me = Controller::Partner(id.to_owned());
        self.rules(table).check(&me, revision, choice)?;
        self.advance(table);
        // A new choice supersedes the news that an earlier held one was applied.
        let superseded = self
            .partner_mut(id)
            .is_some_and(|known| std::mem::take(&mut known.held_applied));
        let moment = self.moment(table.session());
        // The deciding investigator's player's choice waits for the other players with
        // an option there to pass: it is held, replacing one held before.
        if self.window_holds(&moment, id, revision, choice) {
            let _ = self.seats.withdraw(id);
            self.seats.decided(id);
            let hold = Hold::new(me, &moment.observation, choice);
            let submission = Submission::of(&hold.choice, Standing::Held);
            self.hold = Some(hold);
            return Ok(submission);
        }
        let chosen = Chosen::new(&moment.observation, choice);
        let accepted = |standing| Ok(Submission::of(&chosen, standing));
        let mut rules = Rules {
            control: self.control(),
            sharing: &mut self.sharing,
            hold: &mut self.hold,
            table,
        };
        match self.seats.submit(id, revision, chosen.clone(), &mut rules) {
            Submitted::Applied => accepted(Standing::Applied),
            Submitted::Held => accepted(Standing::Settling),
            Submitted::Refused(refusal) => {
                if let Some(known) = self.partner_mut(id) {
                    known.held_applied = superseded;
                }
                Err(why(&refusal, revision))
            }
        }
    }

    /// What the table does on its own before anyone is told of it: the held choice and
    /// the shared decision whose moment has come, then the partners' choices that waited
    /// for it to settle.
    fn advance(&mut self, table: &mut impl Table) {
        self.talk.watch(&*table);
        if !self.settle_hold(table)
            && table.settled()
            && let Some((pending, id, by)) = self.completion(table.session())
        {
            let revision = table.session().revision();
            if let Err(error) = table.commit(revision, &id, by) {
                eprintln!("Agent partners: the shared decision did not complete: {error}");
                self.sharing.unready(&pending);
            }
        }
        if !self
            .seats
            .settle_due(table.session().revision(), table.settled())
        {
            return;
        }
        // A choice that now meets another player's option is held for its pass instead,
        // replacing one held before.
        let moment = self.moment(table.session());
        for id in self.identities() {
            let meets = self.seats.held(&id).is_some_and(|(revision, choice)| {
                self.window_holds(&moment, &id, revision, &choice.id)
            });
            if meets && let Some(choice) = self.seats.withdraw(&id) {
                let me = Controller::Partner(id.clone());
                self.hold = Some(Hold::new(me, &moment.observation, &choice.id));
                self.stands(&id, &Standing::Settling, Standing::Held);
            }
        }
        let mut rules = Rules {
            control: self.control(),
            sharing: &mut self.sharing,
            hold: &mut self.hold,
            table,
        };
        for settled in self.seats.settle(&mut rules) {
            let now = if settled.applied {
                Standing::Applied
            } else {
                Standing::Released
            };
            self.stands(&settled.identity, &Standing::Settling, now);
        }
    }

    /// The submission the held wait of the session `id` follows stood as `was` and stands
    /// as `now`. Only one choice of a session's waits for the table, and only one is held,
    /// each its latest, so `was` says which.
    fn stands(&mut self, id: &str, was: &Standing, now: Standing) {
        if let Some(partner) = self.partner_mut(id)
            && let Purpose::Following(submission) = &mut partner.purpose
            && submission.standing == *was
        {
            submission.standing = now;
        }
    }

    /// A submission as the answer to the wait that follows it tells it: the choice it
    /// named, and how it stands.
    fn standing(&self, submission: &Submission, session: &Session, id: &str) -> Value {
        let status = match &submission.standing {
            Standing::Ready(key) => {
                let moment = self.moment(session);
                let me = Controller::Partner(id.to_owned());
                match moment.pending.iter().find(|pending| pending.key == *key) {
                    Some(pending) if self.sharing.is_ready(pending, &me) => "ready",
                    // Another player's change cleared it.
                    Some(_) => "cleared",
                    None => "over",
                }
            }
            standing => standing.word(),
        };
        let mut choice = json!({"status": status});
        if let Some((id, label)) = &submission.named {
            choice["id"] = json!(id);
            choice["label"] = json!(label);
        }
        choice
    }

    /// Complete a shared decision whose players are all ready and settle the choices that
    /// waited for the table, then answer every wait whose moment has come: its partner's
    /// choice was released, or it hears a group choice open, has news of a shared decision
    /// or has a decision it has not been told of, once the table has presented it, has
    /// a message of the human's to hear, or the period it asked for ran out.
    pub fn poll(&mut self, table: &mut impl Table, now: Instant) -> Vec<Answer<R>> {
        self.advance(table);
        self.answer_waits(table, now)
    }

    /// Answer every wait whose moment has come, at the table as it is.
    fn answer_waits(&mut self, table: &impl Table, now: Instant) -> Vec<Answer<R>> {
        let mut answers = Vec::new();
        // Routed only once the table has presented its decision.
        let mut moment: Option<Moment> = None;
        for id in self.identities() {
            if !self.seats.waits(&id) {
                continue;
            }
            if moment.is_none() && table.presented() {
                moment = Some(self.moment(table.session()));
            }
            let partner = self.partner(&id).expect("A known partner");
            let events = self.events(partner, moment.as_ref());
            let Some((reply, told)) = self.seats.tell(&id, events, now) else {
                continue;
            };
            let partner = self.partner_mut(&id).expect("A known partner");
            let purpose = std::mem::replace(&mut partner.purpose, Purpose::Anything);
            // What is told again comes with a complete view: the changes of the last went
            // to a wait nobody may have read.
            let (status, reason, full) = match told {
                Told::Elapsed => {
                    let mut answer = nothing_yet(&id);
                    answer["investigator"] = json!(self.plays(&id));
                    self.said_with(&mut answer, &purpose, table, &id);
                    answers.push((reply, answer));
                    continue;
                }
                Told::Released { released, again } => {
                    ("released", Some(unapplied(&released)), again)
                }
                Told::Decision { again } => ("decision", None, again),
                Told::News(News::HeldApplied) => {
                    partner.held_applied = false;
                    (Standing::Applied.word(), None, false)
                }
                Told::News(News::GroupChoice) => {
                    partner.told_group = Some(table.session().revision());
                    ("group_choice", None, false)
                }
                Told::News(News::Shared) => ("shared", None, false),
                Told::News(News::TableTalk) => ("table_talk", None, false),
            };
            // A released choice's reason is news, not a refusal.
            let mut answer = self.view_answer(&id, status, None, table, full);
            if let Some(reason) = reason {
                answer["reason"] = json!(reason);
            }
            self.said_with(&mut answer, &purpose, table, &id);
            answers.push((reply, answer));
        }
        answers
    }

    /// Say in the answer to a wait that follows a submission how that stands.
    fn said_with(&self, answer: &mut Value, purpose: &Purpose, table: &impl Table, id: &str) {
        if let Purpose::Following(submission) = purpose {
            answer["choice"] = self.standing(submission, table.session(), id);
        }
    }

    /// Every session's identity, in the order they joined.
    fn identities(&self) -> Vec<String> {
        self.seats.sessions().map(str::to_owned).collect()
    }

    fn partner(&self, id: &str) -> Option<&Partner> {
        self.partners.iter().find(|partner| partner.id == id)
    }

    fn partner_mut(&mut self, id: &str) -> Option<&mut Partner> {
        self.partners.iter_mut().find(|partner| partner.id == id)
    }

    fn known(&self, id: &str) -> bool {
        self.plays(id).is_some()
    }

    /// The one investigator the session `id` plays.
    fn plays(&self, id: &str) -> Option<Investigator> {
        self.seats.seat(id).copied()
    }

    /// The revision of the decision the table waits for the session `id` to make, unless
    /// its choice there already waits for the table.
    fn own_decision(&self, moment: &Moment, id: &str) -> Option<u64> {
        let me = Controller::Partner(id.to_owned());
        (moment.awaits(&me) && self.seats.held(id).is_none()).then_some(moment.observation.revision)
    }

    /// What a held wait of `partner` may answer now, in this order: that its choice held
    /// for the other players' passes was applied and, at a table that has presented its
    /// decision, `moment`, a group choice it has not been told of, news of a shared
    /// decision it takes part in that changed since its last view, and a decision of its
    /// own; then the human's talk, which the table's own events carry as well, as each
    /// does other partners'. A choice released comes before them all.
    fn events(&self, partner: &Partner, moment: Option<&Moment>) -> Vec<Event<News>> {
        let mut events: Vec<_> = self
            .news(partner, moment)
            .into_iter()
            .map(Event::News)
            .collect();
        if let Some(moment) = moment
            && moment.awaits(&Controller::Partner(partner.id.clone()))
        {
            let talk = events
                .iter()
                .position(|event| *event == Event::News(News::TableTalk));
            let decision = Event::Decision(moment.observation.revision);
            events.insert(talk.unwrap_or(events.len()), decision);
        }
        events
    }

    /// [`Self::events`] but for a decision of `partner`'s own.
    fn news(&self, partner: &Partner, moment: Option<&Moment>) -> Vec<News> {
        let mut news = Vec::new();
        if partner.held_applied {
            news.push(News::HeldApplied);
        }
        if let Some(moment) = moment {
            if moment.routing.group().is_some()
                && partner.told_group != Some(moment.observation.revision)
            {
                news.push(News::GroupChoice);
            }
            if moment.shared_news(&self.control(), partner) {
                news.push(News::Shared);
            }
        }
        if self.talk.wakes(partner.told_talk) {
            news.push(News::TableTalk);
        }
        news
    }

    /// Seat one session with the one investigator it names, at once or not at all.
    fn join(
        &mut self,
        named: &[Investigator],
        joining: Joining,
        table: &impl Table,
        now: Instant,
    ) -> Value {
        match (self.seats.recognise(joining), joining.identity) {
            (Err(ClaimRefused::Holds(plays)), Some(id)) => {
                let reason = format!(
                    "You already play {}, and one agent session plays one investigator. Nothing changed: go on with your identity.",
                    plays.name()
                );
                return self.answer(id, "refused", Some(reason), table, false, now);
            }
            (Err(ClaimRefused::SameSession(plays)), _) => {
                return refused(format!(
                    "This session already plays {}, and one agent session plays one investigator. Nothing changed: go on with the identity your join gave you.",
                    plays.name()
                ));
            }
            _ => {}
        }
        let control = self.control();
        let open: Vec<String> = Investigator::ALL
            .into_iter()
            .filter(|who| control.controller(*who) == Controller::Unclaimed)
            .map(|who| format!("{} ({})", who.name(), who.id()))
            .collect();
        if open.is_empty() {
            return refused(
                "No investigator is open to partners right now: the table has handed none to partners, or other partner sessions play every one it handed. There is nothing for you to play, so stop."
                    .into(),
            );
        }
        let refuse =
            |why: String| refused(format!("{why} Open to partners now: {}.", open.join(", ")));
        let &[plays] = named else {
            return refuse("One agent session plays one investigator: name exactly one.".into());
        };
        let id = match self.seats.claim(&plays, joining, now) {
            Ok(id) => id,
            Err(ClaimRefused::Held) => {
                return refuse(format!(
                    "{} is already played by another partner.",
                    plays.name()
                ));
            }
            Err(_) => return refuse(format!("{} is not open to partners.", plays.name())),
        };
        self.partners.push(Partner {
            id: id.clone(),
            sent: BTreeSet::new(),
            board: None,
            told_group: None,
            seen: Vec::new(),
            // A player hears what is said from when it sits down.
            told_talk: self.talk.said().len(),
            joined_talk: self.talk.said().len(),
            held_applied: false,
            purpose: Purpose::Anything,
        });
        self.answer(&id, "joined", None, table, false, now)
    }

    /// The pending decision as routed now, with its shared decisions and the
    /// players it waits for. The deciding investigator's player has decided while
    /// its choice is held for the other players' passes.
    fn moment(&self, session: &Session) -> Moment {
        Moment::of(session, &self.control(), &self.sharing, self.hold.as_ref())
    }

    /// What a partner's choice is checked against and committed through.
    fn rules<'a, T: Table>(&'a mut self, table: &'a mut T) -> Rules<'a, T> {
        Rules {
            control: self.control(),
            sharing: &mut self.sharing,
            hold: &mut self.hold,
            table,
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

    /// Whether [`Self::settle_hold`] has work now: the held choice is to be
    /// released, or, at a `settled` table, applied.
    fn hold_due(&self, session: &Session, settled: bool) -> bool {
        let Some(hold) = &self.hold else {
            return false;
        };
        if hold.revision != session.revision() {
            return true;
        }
        let control = self.control();
        let moment = self.moment(session);
        moment
            .routing
            .controller(session, &control, &hold.choice.id)
            .as_ref()
            != Some(&hold.player)
            || (settled
                && !moment.waits_for_window(&self.sharing, &control, &hold.player, &hold.choice.id))
    }

    /// Release the held choice once the decision has moved on or its player no
    /// longer answers for it; apply it once the table has settled and every other
    /// player with an option there has passed on its revision, or none holds one
    /// now (F1). Whether a choice was committed.
    fn settle_hold(&mut self, table: &mut impl Table) -> bool {
        let Some(hold) = &self.hold else {
            return false;
        };
        let session = table.session();
        let revision = session.revision();
        if hold.revision != revision {
            let hold = self.hold.take().expect("Held");
            self.release(
                &hold,
                "the decision moved on before it could be applied",
                revision,
                true,
            );
            return false;
        }
        let control = self.control();
        let moment = self.moment(session);
        if moment
            .routing
            .controller(session, &control, &hold.choice.id)
            .as_ref()
            != Some(&hold.player)
        {
            let hold = self.hold.take().expect("Held");
            self.release(
                &hold,
                "the investigator it was for is no longer yours",
                revision,
                false,
            );
            return false;
        }
        if !table.settled()
            || moment.waits_for_window(&self.sharing, &control, &hold.player, &hold.choice.id)
        {
            return false;
        }
        let hold = self.hold.take().expect("Held");
        let by = match &hold.player {
            player @ Controller::Partner(_) => check(
                player,
                revision,
                &hold.choice.id,
                &moment.routing,
                &control,
                session,
            ),
            _ => match moment.routing.owner(session, &hold.choice.id) {
                Some(OptionOwner::Investigator(who)) => Ok(who),
                _ => session
                    .opportunity_window()
                    .ok_or_else(|| "No decision is pending.".to_owned()),
            },
        };
        let result = by.and_then(|by| table.commit(revision, &hold.choice.id, by));
        match (&hold.player, &result) {
            (Controller::Partner(partner), Ok(())) => {
                if let Some(known) = self.partners.iter_mut().find(|known| known.id == *partner) {
                    known.held_applied = true;
                }
                let partner = partner.clone();
                self.stands(&partner, &Standing::Held, Standing::Applied);
            }
            (Controller::Partner(_), Err(reason)) => self.release(&hold, reason, revision, true),
            (_, Err(error)) => {
                eprintln!("Agent partners: the human's held choice was not applied: {error}");
                self.release(&hold, error, revision, true);
            }
            (_, Ok(())) => {}
        }
        result.is_ok()
    }

    /// Tell the holder its held choice was released, not applied, and why: a
    /// partner through its next answer, the human in his decision display while the
    /// decision stays at `revision`. The human is asked to choose again only while
    /// the investigator it was for is still his (`still_his`): once he has handed it
    /// to partners, the decision may be theirs.
    fn release(&mut self, hold: &Hold, why: &str, revision: u64, still_his: bool) {
        let released = format!(
            "Your held choice, {}, was not applied: {why}.",
            hold.choice.label
        );
        match &hold.player {
            Controller::Partner(partner) => {
                // Its decision is asked for again.
                let choice = hold.choice.clone();
                let why = Refusal::Game(why.to_owned());
                self.seats.release(partner, Released { choice, why });
                self.stands(partner, &Standing::Held, Standing::Released);
            }
            _ if still_his => {
                self.human_released = Some((revision, format!("{released} Choose again.")));
            }
            _ => self.human_released = Some((revision, released)),
        }
    }

    /// Why the human's held choice in his own decision was not applied, while
    /// the decision that released it is pending.
    pub fn human_released(&self, session: &Session) -> Option<&str> {
        self.human_released
            .as_ref()
            .filter(|(revision, _)| *revision == session.revision())
            .map(|(_, notice)| notice.as_str())
    }

    /// Drop `player`'s held choice: a choice of its own was applied at once.
    fn drop_hold(&mut self, player: &Controller) {
        drop_hold(&mut self.hold, player);
    }

    /// Whether `partner`'s held choice of `id` at `revision` waits for the other
    /// players' passes: a current choice of the deciding investigator's player
    /// while another player with an option there has not passed. A stale one is
    /// refused.
    fn window_holds(&self, moment: &Moment, partner: &str, revision: u64, id: &str) -> bool {
        revision == moment.observation.revision
            && moment.waits_for_window(
                &self.sharing,
                &self.control(),
                &Controller::Partner(partner.to_owned()),
                id,
            )
    }

    /// A partner's readiness in a shared decision: through `ready`, or by choosing
    /// the decision's completing option, a named clue allocation or The Barrier's
    /// decline.
    fn ready(
        &mut self,
        partner: &str,
        revision: u64,
        id: Option<&str>,
        table: &impl Table,
    ) -> Result<Submission, String> {
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
                            .map(|()| pending.key.clone())
                    } else {
                        Err("The shared decision changed since your last view; nothing was rebased. Observe it and decide again.".into())
                    }
                }
            }
        };
        let key = result?;
        self.seats.decided(partner);
        Ok(Submission {
            named: id.map(|id| (id.to_owned(), label(&moment.observation, id))),
            standing: Standing::Ready(key),
        })
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
    fn inspect(
        &mut self,
        id: &str,
        plays: Investigator,
        card: &str,
        table: &impl Table,
        now: Instant,
    ) -> Value {
        self.seats.heard(id, now);
        let observation = table.session().observe();
        match inspection::table_card(&observation, card, Sight::Player(plays)) {
            Some(text) => json!({
                "ok": true, "mode": "partner", "status": "inspected", "partner": id,
                "investigator": plays, "card": card, "text": text,
            }),
            None => {
                let reason = "No card with that code is in your sight. Inspect a card you can see by its code: in your investigator's hand, in play, being resolved or in a public pile.".to_owned();
                self.answer(id, "refused", Some(reason), table, false, now)
            }
        }
    }

    /// An answer to a request of a known partner's other than a wait, with its
    /// investigator, its view and the choices of its released that no such answer has told
    /// it of.
    fn answer(
        &mut self,
        id: &str,
        status: &str,
        error: Option<String>,
        table: &impl Table,
        full: bool,
        now: Instant,
    ) -> Value {
        self.seats.heard(id, now);
        let released: Vec<String> = self.seats.read(id).iter().map(unapplied).collect();
        let mut answer = self.view_answer(id, status, error, table, full);
        if !released.is_empty() {
            answer["released"] = json!(released.join(" "));
        }
        answer
    }

    /// An answer for a known partner, with its investigator and view.
    fn view_answer(
        &mut self,
        id: &str,
        status: &str,
        error: Option<String>,
        table: &impl Table,
        full: bool,
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
        let plays = self.plays(id).expect("A known partner");
        let partner = self
            .partners
            .iter_mut()
            .find(|partner| partner.id == id)
            .expect("A known partner");
        partner.seen = moment
            .pending
            .iter()
            .map(|pending| (pending.key.clone(), pending.stamp))
            .collect();
        let mut view = view::build(
            table.session(),
            &control,
            &moment.routing,
            view::Viewer {
                partner: id,
                plays,
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
        // In its own investigator's decision, its next choice waits for the
        // players with an option there to pass (F1): said in a field of its own,
        // beside any note.
        if let Some(passes) = self.passes_view(&moment, &control, &me) {
            view["decision"]["waits_for_passes"] = passes;
        }
        // Its choice that waits for the table, which a later one replaces.
        if let Some((_, choice)) = self.seats.held(id) {
            view["decision"]["settling"] = json!({"id": choice.id, "label": choice.label});
        }
        let mut answer = json!({
            "ok": error.is_none(),
            "mode": "partner",
            "status": status,
            "partner": id,
            "investigator": plays,
            "view": view,
        });
        if let Some(error) = error {
            answer["error"] = json!(error);
        }
        answer
    }

    /// The other players' passes as the deciding investigator's player meets them
    /// (F1): whose passes its next choice waits for, while any is outstanding.
    fn passes_view(&self, moment: &Moment, control: &Control, me: &Controller) -> Option<Value> {
        let pending = moment.window_before(control, me)?;
        let SharedKind::Before { decider, action } = pending.decision.kind else {
            unreachable!("The passes before a decision")
        };
        let waiting: Vec<&str> = self
            .sharing
            .waiting(pending, control)
            .into_iter()
            .map(|(_, who)| who.name())
            .collect();
        if waiting.is_empty() {
            return None;
        }
        let names = waiting.join(" and ");
        let have = if waiting.len() == 1 { "has" } else { "have" };
        let before = if action {
            "before_action_of"
        } else {
            "before_decision_of"
        };
        let mut passes = json!({
            before: decider.name(),
            "waiting_for": waiting,
            // The next step in brief; the partner instructions state the rules.
            "how": format!(
                "Your next choice, other than Undo or your part of a shared decision, is held until {names} {have} passed here: choose answers held. {}",
                rules_in("Held choices")
            ),
        });
        if let Some(hold) = self
            .hold
            .as_ref()
            .filter(|hold| hold.player == *me && hold.revision == moment.observation.revision)
        {
            passes["held"] = json!({"id": hold.choice.id, "label": hold.choice.label});
        }
        Some(passes)
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
    fn of(
        session: &Session,
        control: &Control,
        sharing: &shared::Sharing,
        hold: Option<&Hold>,
    ) -> Self {
        let observation = session.observe();
        let routing = Routing::of(session, control);
        let pending = sharing.pending(routing.shared(), &observation);
        let holding = hold
            .filter(|hold| hold.revision == observation.revision)
            .map(|hold| &hold.player);
        let mut awaited: Vec<(Controller, Option<Investigator>)> = routing
            .awaiting()
            .filter(|player| Some(*player) != holding)
            .map(|player| (player.clone(), routing.awaited_investigator()))
            .into_iter()
            .collect();
        for decision in &pending {
            for (player, who) in sharing.waiting(decision, control) {
                if !awaited.iter().any(|(listed, _)| *listed == player) {
                    awaited.push((player, Some(who)));
                }
            }
        }
        Self {
            observation,
            routing,
            pending,
            awaited,
        }
    }

    /// Whether the table waits for `player`.
    fn awaits(&self, player: &Controller) -> bool {
        self.awaited.iter().any(|(listed, _)| listed == player)
    }

    /// Whether a shared decision `partner` takes part in changed since its last view.
    fn shared_news(&self, control: &Control, partner: &Partner) -> bool {
        let me = Controller::Partner(partner.id.clone());
        self.pending.iter().any(|pending| {
            pending.decision.players(control).contains(&me)
                && partner
                    .seen
                    .iter()
                    .any(|(key, stamp)| *key == pending.key && *stamp != pending.stamp)
        })
    }

    /// Whether `player`'s choice of `id` waits for the other players' passes (F1):
    /// it is the deciding investigator's player's choice, other than Undo or a part
    /// of a shared decision (an act clue payment always is one), and a player with
    /// an option there has not yet passed.
    fn waits_for_window(
        &self,
        sharing: &shared::Sharing,
        control: &Control,
        player: &Controller,
        id: &str,
    ) -> bool {
        id != "undo"
            && self.routing.shared_of(id).is_none()
            && !routing::pays_act(id)
            && self
                .window_before(control, player)
                .is_some_and(|pending| !sharing.waiting(pending, control).is_empty())
    }

    /// The decision of an investigator `player` controls, while another player
    /// with an option in it takes part (F1).
    fn window_before(&self, control: &Control, player: &Controller) -> Option<&shared::Pending> {
        self.pending.iter().find(|pending| {
            matches!(pending.decision.kind, SharedKind::Before { decider, .. } if control.controller(decider) == *player)
        })
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

/// The session and the host's own state, as a partner's choice is checked and committed.
struct Rules<'a, T> {
    control: Control,
    sharing: &'a mut shared::Sharing,
    hold: &'a mut Option<Hold>,
    table: &'a mut T,
}

impl<T: Table> Rules<'_, T> {
    fn moment(&self) -> Moment {
        Moment::of(
            self.table.session(),
            &self.control,
            self.sharing,
            self.hold.as_ref(),
        )
    }

    /// Whether `me` may submit `id` at `revision` now, and as which investigator.
    fn check(&self, me: &Controller, revision: u64, id: &str) -> Result<Investigator, String> {
        let moment = self.moment();
        let by = check(
            me,
            revision,
            id,
            &moment.routing,
            &self.control,
            self.table.session(),
        )?;
        // A partner's option in a commitment window changes its part.
        if let Some(pending) = moment.changes(&self.control, me) {
            self.sharing.may_change(pending, me)?;
        }
        Ok(by)
    }
}

impl<T: Table> Game for Rules<'_, T> {
    type Seat = Investigator;
    type Choice = Chosen;

    fn revision(&self) -> u64 {
        self.table.session().revision()
    }

    fn settled(&self) -> bool {
        self.table.settled()
    }

    fn check(&self, seat: &Investigator, choice: &Chosen) -> Result<(), String> {
        let me = self.control.controller(*seat);
        self.check(&me, self.revision(), &choice.id).map(|_| ())
    }

    fn commit(&mut self, seat: &Investigator, choice: &Chosen) -> Result<(), String> {
        let me = self.control.controller(*seat);
        let revision = self.revision();
        let by = self.check(&me, revision, &choice.id)?;
        let change = self.moment().changes(&self.control, &me).cloned();
        self.table.commit(revision, &choice.id, by)?;
        if let Some(pending) = &change {
            self.sharing
                .change(pending, &me)
                .expect("Checked before committing");
        }
        // Its own choice applied at once drops the one it held.
        drop_hold(self.hold, &me);
        Ok(())
    }
}

/// Whether the partner `me` may submit `id` at `revision` now, and as which investigator.
fn check(
    me: &Controller,
    revision: u64,
    id: &str,
    routing: &Routing,
    control: &Control,
    session: &Session,
) -> Result<Investigator, String> {
    let current = session.observe().revision;
    if revision != current {
        return Err(stale(revision, current));
    }
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
    if routing.controller(session, control, id).as_ref() != Some(me) {
        if id == "undo" {
            return Err(match owner {
                OptionOwner::Table => {
                    "Undo reverts only your own latest decision; the latest was made at the table."
                        .into()
                }
                _ => format!(
                    "Undo reverts only your own latest decision; the latest belongs to {names}."
                ),
            });
        }
        return Err(match owner {
            OptionOwner::Table => {
                "That choice is made at the table: the lead investigator is chosen there.".into()
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
    Ok(Investigator::ALL
        .into_iter()
        .find(|who| control.controller(*who) == *me)
        .expect("Only a session at the table controls an option"))
}

fn drop_hold(hold: &mut Option<Hold>, player: &Controller) {
    if hold.as_ref().is_some_and(|held| held.player == *player) {
        *hold = None;
    }
}

fn stale(revision: u64, current: u64) -> String {
    format!(
        "This choice is stale: it names revision {revision}, and the decision is now at revision {current}. Observe the current decision."
    )
}

/// Why a choice made at `revision` was refused.
fn why(refusal: &Refusal, revision: u64) -> String {
    match refusal {
        Refusal::Stale { current } => stale(revision, *current),
        Refusal::Game(reason) => reason.clone(),
        Refusal::Unseated => "Your session plays no investigator.".into(),
    }
}

/// Why a choice that waited, for the table or for other players' passes, was not applied.
fn unapplied(released: &Released<Chosen>) -> String {
    let why = match &released.why {
        Refusal::Game(reason) => reason.trim_end_matches('.').to_owned(),
        _ => "the decision moved on before the table settled".to_owned(),
    };
    let held = if released.choice.for_passes {
        "held "
    } else {
        ""
    };
    format!(
        "Your {held}choice, {}, was not applied: {why}. Read the current view.",
        released.choice.label
    )
}

fn refused(reason: String) -> Value {
    json!({"ok": false, "mode": "partner", "status": "refused", "error": reason})
}

fn closed(reason: &str) -> Value {
    json!({"ok": true, "mode": "partner", "status": "closed", "reason": reason})
}

/// A wait whose period ended with nothing for its partner.
pub(crate) fn nothing_yet(partner: &str) -> Value {
    json!({"ok": true, "mode": "partner", "status": "nothing_yet", "partner": partner})
}

/// Why a wait was [replaced](replaced_wait).
const BY_WAIT: &str = "A newer wait of yours took this one's place: let it be.";
const BY_CHOICE: &str = "A choice of yours, or your readiness, ended this listening wait: go on from its answer, and wait again when you have nothing to decide.";

/// A wait that its partner's newer wait, or the choice it listened before, took the place
/// of.
fn replaced_wait(partner: &str, reason: &str) -> Value {
    json!({
        "ok": true, "mode": "partner", "status": "replaced", "partner": partner,
        "reason": reason,
    })
}

#[cfg(test)]
mod tests;
