//! Which player may submit each listed option, and whose answer the table waits
//! for. Ownership comes from the rules session ([`Session::ownership`]); this
//! module only maps owners to the players who control them.
//!
//! A group choice belongs to the player of its arbiter, the lead investigator or
//! the one standing in for an eliminated lead (#207). The human makes the table's
//! setup choice of the lead, with all of that decision's options. A shared
//! decision whose participants one player controls is that player's. One whose
//! participants several players control is [`Shared`]: each player answers its
//! own investigators' options in it, and its completing option is every
//! participating player's readiness, which the host collects (#208).
//!
//! A turn's player window comes before each action (RR p.22, p.24 2.2.1). When
//! another player holds a fast option in it, the window is [`SharedKind::Turn`]:
//! that player passes with readiness, and the active investigator's player's
//! next choice, other than Undo or a part of a shared decision, is applied only
//! once every such player has passed on the current revision (F1, Joel's
//! decision of 2026-11-13: played as the rules have it).

/// A player's pass in a turn's player window. It is co-play's, never the rules
/// session's: the rules session lists no pass before an action.
pub const TURN_PASS: &str = "turn-window:pass";

/// The label of a player's pass in the player window before `actor`'s action.
pub(super) fn turn_pass_label(actor: Investigator) -> String {
    format!("Pass before {}'s action", actor.name())
}

use super::{Control, Controller};
use crate::Choice;
use crate::gathering::{GroupCluePayment, Investigator, OptionOwner, Session};

/// The players of the pending decision's options.
#[derive(Clone, Debug)]
pub struct Routing {
    /// Each listed option one player may submit, except Undo: its owner and that
    /// player. A shared decision's completing options are in `shared` instead.
    options: Vec<(String, OptionOwner, Controller)>,
    /// The human answers the whole decision: the setup choice of the lead.
    with_human: bool,
    /// The player the table waits for outside shared decisions, and the
    /// investigator that player answers for.
    awaiting: Option<(Controller, Option<Investigator>)>,
    shared: Vec<Shared>,
    /// The arbiter of the pending group choice, while one is open.
    group: Option<Investigator>,
    /// Undo, while the session offers it: the owner of the decision it would
    /// revert, and the one player who controls every investigator that owner
    /// names, if one does.
    undo: Option<(OptionOwner, Option<Controller>)>,
}

/// A shared decision whose participants more than one player controls (#208).
/// Each player sets only its own investigators' part; it completes once every
/// participating player is ready on the same contributions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shared {
    pub kind: SharedKind,
    /// The listed option that completes it: Confirm, the window's pass, or the
    /// group clue payment's listed choice.
    pub close: String,
    /// The investigators taking part, in player-card order.
    pub participants: Vec<Investigator>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedKind {
    /// Commitments to a skill test (RR p.26 ST.2): each player toggles its own
    /// investigators' cards, and Confirm is its readiness.
    Commitment,
    /// A window for fast cards and abilities. It closes after every player with
    /// an option has passed in succession; an action restarts the succession.
    Window,
    /// A group clue payment (RR p.7): each player sets its own investigators'
    /// clues. At The Barrier's round end, readiness on no clues declines it.
    Payment { at_round_end: bool },
    /// The player window before an action of `actor`'s turn, while another player
    /// holds a fast option in it. Its participants are those players'
    /// investigators; each passes with [`TURN_PASS`] or readiness, and an action
    /// restarts the succession. The actor's player's choice waits for them.
    Turn { actor: Investigator },
}

impl Shared {
    /// The players taking part, in the order of their first investigator.
    pub fn players(&self, control: &Control) -> Vec<Controller> {
        let mut players: Vec<Controller> = Vec::new();
        for who in &self.participants {
            let player = control.controller(*who);
            if !players.contains(&player) {
                players.push(player);
            }
        }
        players
    }

    /// Whether `id` is this decision's part: its completing option, a payment's
    /// named allocation, or The Barrier's decline.
    pub fn includes(&self, id: &str) -> bool {
        id == self.close
            || match self.kind {
                SharedKind::Payment { at_round_end } => {
                    (at_round_end && id == GroupCluePayment::DECLINE)
                        || GroupCluePayment::allocation_of(id).is_some()
                }
                _ => false,
            }
    }
}

impl Routing {
    pub fn of(session: &Session, control: &Control) -> Self {
        let decision_owner = session.observe().decision_owner;
        let owned = session.ownership();
        let group = owned.iter().find_map(|option| match option.owner {
            OptionOwner::Group { arbiter } => Some(arbiter),
            _ => None,
        });
        let with_human = owned
            .iter()
            .any(|option| option.owner == OptionOwner::Table);
        let round_end = owned
            .iter()
            .any(|option| option.id == GroupCluePayment::DECLINE);
        let mut shared: Vec<Shared> = Vec::new();
        let mut options = Vec::new();
        for option in owned {
            if !with_human
                && let OptionOwner::Shared { participants } = &option.owner
                && spans_players(participants, control)
            {
                let kind = if option.id == "commit:confirm" {
                    SharedKind::Commitment
                } else if GroupCluePayment::is_listed(&option.id)
                    || option.id == GroupCluePayment::DECLINE
                {
                    SharedKind::Payment {
                        at_round_end: round_end,
                    }
                } else {
                    SharedKind::Window
                };
                // The Barrier's decline is the round-end payment's other outcome.
                if option.id != GroupCluePayment::DECLINE {
                    shared.push(Shared {
                        kind,
                        close: option.id,
                        participants: participants.clone(),
                    });
                }
                continue;
            }
            let controller = if with_human {
                Controller::Human
            } else {
                single_controller(&option.owner, control)
            };
            options.push((option.id, option.owner, controller));
        }
        if let Some(actor) = session.turn_window().filter(|_| !with_human)
            && let Some(window) = turn_window(actor, &options, control)
        {
            shared.push(window);
        }
        // A commitment window or a closing window is answered by readiness alone.
        let awaiting = if shared.iter().any(|decision| {
            !matches!(
                decision.kind,
                SharedKind::Payment {
                    at_round_end: false
                } | SharedKind::Turn { .. }
            )
        }) {
            None
        } else {
            awaiting(&options, decision_owner, control)
        };
        let undo = session.undo_owner().map(|owner| {
            let undoer = sole_controller(&owner, control);
            (owner, undoer)
        });
        Self {
            options,
            with_human,
            awaiting,
            shared,
            group,
            undo,
        }
    }

    /// The player who may undo the latest undoable decision: the one who now
    /// controls every investigator that decision belonged to. `None` while Undo
    /// is not offered, or when no single player controls them all.
    pub fn undoer(&self) -> Option<&Controller> {
        self.undo.as_ref().and_then(|(_, undoer)| undoer.as_ref())
    }

    /// The owner of the decision Undo would revert, while Undo is offered.
    pub fn undo_owner(&self) -> Option<&OptionOwner> {
        self.undo.as_ref().map(|(owner, _)| owner)
    }

    /// The player the table waits for outside shared decisions, if any.
    pub fn awaiting(&self) -> Option<&Controller> {
        self.awaiting.as_ref().map(|(controller, _)| controller)
    }

    /// The arbiter of the pending group choice, while one is open.
    pub fn group(&self) -> Option<Investigator> {
        self.group
    }

    /// The investigator the awaited player answers for, when one is named.
    pub fn awaited_investigator(&self) -> Option<Investigator> {
        self.awaiting.as_ref().and_then(|(_, who)| *who)
    }

    /// Whether the human answers the whole decision: the setup choice of the lead.
    pub fn with_human(&self) -> bool {
        self.with_human
    }

    /// The listed options one player may submit, their owners and that player.
    pub fn options(&self) -> impl Iterator<Item = (&str, &OptionOwner, &Controller)> {
        self.options
            .iter()
            .map(|(id, owner, controller)| (id.as_str(), owner, controller))
    }

    /// The pending shared decisions whose participants several players control.
    pub fn shared(&self) -> &[Shared] {
        &self.shared
    }

    /// The shared decision `id` is a part of, when it spans players.
    pub fn shared_of(&self, id: &str) -> Option<&Shared> {
        self.shared.iter().find(|decision| decision.includes(id))
    }

    /// Whether the human may submit `id`: see [`Self::may`].
    pub fn human_may(&self, session: &Session, control: &Control, id: &str) -> bool {
        self.may(session, control, &Controller::Human, id)
    }

    /// Whether `player` may submit `id`: its own option, its part in a shared
    /// decision it takes part in, and Undo only when the decision it would revert
    /// is its own (#207). An identity no player may submit alone and no shared
    /// decision includes is the session's to refuse, as without partners.
    pub fn may(&self, session: &Session, control: &Control, player: &Controller, id: &str) -> bool {
        if id == "undo" {
            return self.undoer() == Some(player);
        }
        match self.shared_of(id) {
            Some(decision) => decision.players(control).contains(player),
            None => self
                .controller(session, control, id)
                .is_none_or(|controller| controller == *player),
        }
    }

    /// The pending decision's listed options `player` may submit, and its pass
    /// when it takes part in a turn's player window, which the rules session does
    /// not list (F1).
    pub fn choices_of(
        &self,
        session: &Session,
        control: &Control,
        player: &Controller,
    ) -> Vec<Choice> {
        let mut choices: Vec<Choice> = session
            .observe()
            .choices
            .into_iter()
            .filter(|choice| self.may(session, control, player, &choice.id))
            .collect();
        let window = self.shared.iter().find_map(|decision| match decision.kind {
            SharedKind::Turn { actor } if decision.players(control).contains(player) => Some(actor),
            _ => None,
        });
        if let Some(actor) = window {
            choices.push(Choice {
                id: TURN_PASS.to_owned(),
                label: turn_pass_label(actor),
                cancel_faces: Vec::new(),
            });
        }
        choices
    }

    /// The player who may submit `id`: a listed option, or a number or allocation
    /// in place of a listed option's placeholder. `None` when the pending decision
    /// does not accept it from one player alone. Undo is its
    /// [undoer](Self::undoer)'s, if it has one.
    pub fn controller(&self, session: &Session, control: &Control, id: &str) -> Option<Controller> {
        if id == "undo" {
            return self.undoer().cloned();
        }
        if let Some((_, _, controller)) = self.options.iter().find(|(listed, ..)| listed == id) {
            return Some(controller.clone());
        }
        if self.shared_of(id).is_some() {
            return None;
        }
        let owner = session.option_owner(id)?;
        Some(if self.with_human {
            Controller::Human
        } else {
            single_controller(&owner, control)
        })
    }

    /// The owner of `id` as the rules session names it; for Undo, the owner of
    /// the decision it would revert.
    pub fn owner(&self, session: &Session, id: &str) -> Option<OptionOwner> {
        if id == "undo" {
            return self.undo_owner().cloned();
        }
        self.options
            .iter()
            .find(|(listed, ..)| listed == id)
            .map(|(_, owner, _)| owner.clone())
            .or_else(|| session.option_owner(id))
    }
}

/// The investigators an owner names.
pub(super) fn investigators(owner: &OptionOwner) -> Vec<Investigator> {
    match owner {
        OptionOwner::Investigator(who) => vec![*who],
        OptionOwner::Group { arbiter } => vec![*arbiter],
        OptionOwner::Shared { participants } => participants.clone(),
        OptionOwner::Table => vec![],
    }
}

/// Whether `id` pays an act's clues: the listed group clue payment or a named
/// allocation. The investigators spend them as a group, normally as a [free]
/// player ability, and any or all may contribute (RR p.3, "Act Deck and Agenda
/// Deck"). Each player able to contribute may propose its part, as #208's shared
/// payment, and a proposal wakes the others; so it is every contributing player's
/// part of one group payment, even when one player alone can contribute (review
/// fix G4).
pub(super) fn pays_act(id: &str) -> bool {
    GroupCluePayment::is_listed(id) || GroupCluePayment::allocation_of(id).is_some()
}

/// The player window before `actor`'s action, when a player other than the
/// actor's holds one of its options: every such player's investigators with an
/// option take part. None without such a player. A possible act clue payment
/// makes no one a participant: it waits until a player proposes it.
fn turn_window(
    actor: Investigator,
    options: &[(String, OptionOwner, Controller)],
    control: &Control,
) -> Option<Shared> {
    let actors = control.controller(actor);
    let mut participants: Vec<Investigator> = options
        .iter()
        .filter(|(id, _, controller)| *controller != actors && !pays_act(id))
        .flat_map(|(_, owner, _)| investigators(owner))
        .filter(|who| control.controller(*who) != actors)
        .collect();
    participants.sort_by_key(|who| who.index());
    participants.dedup();
    (!participants.is_empty()).then(|| Shared {
        kind: SharedKind::Turn { actor },
        close: TURN_PASS.to_owned(),
        participants,
    })
}

/// Participants more than one player controls.
fn spans_players(participants: &[Investigator], control: &Control) -> bool {
    let mut players = participants.iter().map(|who| control.controller(*who));
    let Some(first) = players.next() else {
        return false;
    };
    players.any(|player| player != first)
}

/// The one player who controls every investigator `owner` names; the table's
/// own choice is the human's. No one does for a shared decision that spans
/// players, so no one may undo one.
fn sole_controller(owner: &OptionOwner, control: &Control) -> Option<Controller> {
    if *owner == OptionOwner::Table {
        return Some(Controller::Human);
    }
    let mut players = investigators(owner)
        .into_iter()
        .map(|who| control.controller(who));
    let first = players.next()?;
    players.all(|player| player == first).then_some(first)
}

/// The controller of an option one player answers.
fn single_controller(owner: &OptionOwner, control: &Control) -> Controller {
    match owner {
        OptionOwner::Investigator(who) | OptionOwner::Group { arbiter: who } => {
            control.controller(*who)
        }
        OptionOwner::Shared { participants } => participants
            .first()
            .map_or(Controller::Human, |who| control.controller(*who)),
        OptionOwner::Table => Controller::Human,
    }
}

/// The player the table waits for. A group choice waits for its arbiter's
/// player. When several players have options, the player of the investigator
/// whose decision it is (the table's display focus) answers, and the others'
/// options stay available to them.
fn awaiting(
    options: &[(String, OptionOwner, Controller)],
    decision_owner: Option<Investigator>,
    control: &Control,
) -> Option<(Controller, Option<Investigator>)> {
    let (_, first_owner, first) = options.first()?;
    if let OptionOwner::Group { arbiter } = first_owner {
        return Some((first.clone(), Some(*arbiter)));
    }
    let has_options = |controller: &Controller| {
        options
            .iter()
            .any(|(_, _, candidate)| candidate == controller)
    };
    let principal = decision_owner
        .map(|who| (control.controller(who), who))
        .filter(|(controller, _)| has_options(controller));
    if let Some((controller, who)) = principal {
        return Some((controller, Some(who)));
    }
    let who = investigators(first_owner).first().copied();
    Some((first.clone(), who))
}
