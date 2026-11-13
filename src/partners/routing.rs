//! Which player may submit each listed option, and whose answer the table waits
//! for. Ownership comes from the rules session ([`Session::ownership`]); this
//! module only maps owners to the players who control them.
//!
//! The human makes every group choice and the table's setup choice of the lead,
//! with all of that decision's options. A shared decision whose participants one
//! player controls is that player's. One whose participants several players
//! control is [`Shared`]: each player answers its own investigators' options in
//! it, and its completing option is every participating player's readiness,
//! which the host collects (#208).

use super::{Control, Controller};
use crate::gathering::{GroupCluePayment, Investigator, OptionOwner, Session};

/// The Barrier's round-end alternative to its group clue payment.
const DECLINE: &str = "round-end:decline-act";

/// The players of the pending decision's options.
#[derive(Clone, Debug)]
pub struct Routing {
    /// Each listed option one player may submit, except Undo: its owner and that
    /// player. A shared decision's completing options are in `shared` instead.
    options: Vec<(String, OptionOwner, Controller)>,
    /// The human answers the whole decision: a group choice or the setup choice
    /// of the lead.
    with_human: bool,
    /// The player the table waits for outside shared decisions, and the
    /// investigator that player answers for.
    awaiting: Option<(Controller, Option<Investigator>)>,
    shared: Vec<Shared>,
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
                    (at_round_end && id == DECLINE)
                        || (id.starts_with("act:pay:") && !GroupCluePayment::is_listed(id))
                }
                _ => false,
            }
    }
}

impl Routing {
    pub fn of(session: &Session, control: &Control) -> Self {
        let decision_owner = session.observe().decision_owner;
        let owned = session.ownership();
        let with_human = owned
            .iter()
            .any(|option| matches!(option.owner, OptionOwner::Group { .. } | OptionOwner::Table));
        let round_end = owned.iter().any(|option| option.id == DECLINE);
        let mut shared: Vec<Shared> = Vec::new();
        let mut options = Vec::new();
        for option in owned {
            if !with_human
                && let OptionOwner::Shared { participants } = &option.owner
                && spans_players(participants, control)
            {
                let kind = if option.id == "commit:confirm" {
                    SharedKind::Commitment
                } else if GroupCluePayment::is_listed(&option.id) || option.id == DECLINE {
                    SharedKind::Payment {
                        at_round_end: round_end,
                    }
                } else {
                    SharedKind::Window
                };
                // The Barrier's decline is the round-end payment's other outcome.
                if option.id != DECLINE {
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
        // A commitment window or a closing window is answered by readiness alone.
        let awaiting = if shared.iter().any(|decision| {
            !matches!(
                decision.kind,
                SharedKind::Payment {
                    at_round_end: false
                }
            )
        }) {
            None
        } else {
            awaiting(&options, decision_owner, control)
        };
        Self {
            options,
            with_human,
            awaiting,
            shared,
        }
    }

    /// The player the table waits for outside shared decisions, if any.
    pub fn awaiting(&self) -> Option<&Controller> {
        self.awaiting.as_ref().map(|(controller, _)| controller)
    }

    /// The investigator the awaited player answers for, when one is named.
    pub fn awaited_investigator(&self) -> Option<Investigator> {
        self.awaiting.as_ref().and_then(|(_, who)| *who)
    }

    /// Whether the human answers the whole decision: a group choice or the setup
    /// choice of the lead.
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

    /// Whether `player` may submit `id` now: one of its own options, or its
    /// part in a shared decision it takes part in.
    pub fn may(&self, session: &Session, control: &Control, player: &Controller, id: &str) -> bool {
        match self.shared_of(id) {
            Some(decision) => decision.players(control).contains(player),
            None => self.controller(session, control, id).as_ref() == Some(player),
        }
    }

    /// The player who may submit `id`: a listed option, or a number or allocation
    /// in place of a listed option's placeholder. `None` when the pending decision
    /// does not accept it from one player alone. Undo is the human's.
    pub fn controller(&self, session: &Session, control: &Control, id: &str) -> Option<Controller> {
        if id == "undo" {
            return session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id == "undo")
                .then_some(Controller::Human);
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

    /// The owner of `id` as the rules session names it.
    pub fn owner(&self, session: &Session, id: &str) -> Option<OptionOwner> {
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

/// Participants more than one player controls.
fn spans_players(participants: &[Investigator], control: &Control) -> bool {
    let mut players = participants.iter().map(|who| control.controller(*who));
    let Some(first) = players.next() else {
        return false;
    };
    players.any(|player| player != first)
}

/// The controller of an owner that is not answered by the human as a whole.
fn single_controller(owner: &OptionOwner, control: &Control) -> Controller {
    match owner {
        OptionOwner::Investigator(who) => control.controller(*who),
        OptionOwner::Shared { participants } => participants
            .first()
            .map_or(Controller::Human, |who| control.controller(*who)),
        OptionOwner::Group { .. } | OptionOwner::Table => Controller::Human,
    }
}

/// The player the table waits for. When several players have options, the
/// player of the investigator whose decision it is (the table's display focus)
/// answers, and the others' options stay available to them.
fn awaiting(
    options: &[(String, OptionOwner, Controller)],
    decision_owner: Option<Investigator>,
    control: &Control,
) -> Option<(Controller, Option<Investigator>)> {
    let (_, first_owner, first) = options.first()?;
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
