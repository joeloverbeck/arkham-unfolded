//! Which player may submit each listed option, and whose answer the table waits
//! for. Ownership comes from the rules session ([`Session::ownership`]); this
//! module only maps owners to the players who control them.
//!
//! In this slice (#205) the human makes every group choice, the table's setup
//! choice and any shared decision whose participants belong to more than one
//! player, with all of that decision's options. A shared decision whose
//! participants one player controls is that player's.

use super::{Control, Controller};
use crate::gathering::{Investigator, OptionOwner, Session};

/// The players of the pending decision's options.
#[derive(Clone, Debug)]
pub struct Routing {
    /// Each listed option except Undo, its owner and the player who may submit it.
    options: Vec<(String, OptionOwner, Controller)>,
    /// The human answers the whole decision: a group choice, the setup choice of
    /// the lead, or a shared decision that spans players.
    with_human: bool,
    /// The player the table waits for, and the investigator that player answers for.
    awaiting: Option<(Controller, Option<Investigator>)>,
}

impl Routing {
    pub fn of(session: &Session, control: &Control) -> Self {
        let decision_owner = session.observe().decision_owner;
        let owned = session.ownership();
        let with_human = owned
            .iter()
            .any(|option| spans_players(&option.owner, control));
        let options: Vec<_> = owned
            .into_iter()
            .map(|option| {
                let controller = if with_human {
                    Controller::Human
                } else {
                    single_controller(&option.owner, control)
                };
                (option.id, option.owner, controller)
            })
            .collect();
        let awaiting = awaiting(&options, decision_owner, control);
        Self {
            options,
            with_human,
            awaiting,
        }
    }

    /// The player the table waits for, if any option is listed.
    pub fn awaiting(&self) -> Option<&Controller> {
        self.awaiting.as_ref().map(|(controller, _)| controller)
    }

    /// The investigator the awaited player answers for, when one is named.
    pub fn awaited_investigator(&self) -> Option<Investigator> {
        self.awaiting.as_ref().and_then(|(_, who)| *who)
    }

    /// Whether the human answers the whole decision in this slice.
    pub fn with_human(&self) -> bool {
        self.with_human
    }

    /// The listed options, their owners and the players who may submit them.
    pub fn options(&self) -> impl Iterator<Item = (&str, &OptionOwner, &Controller)> {
        self.options
            .iter()
            .map(|(id, owner, controller)| (id.as_str(), owner, controller))
    }

    /// The player who may submit `id`: a listed option, or a number or allocation
    /// in place of a listed option's placeholder. `None` when the pending decision
    /// does not accept it. Undo is the human's.
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

/// A group choice, the table's choice, or a shared decision whose participants
/// more than one player controls.
fn spans_players(owner: &OptionOwner, control: &Control) -> bool {
    match owner {
        OptionOwner::Investigator(_) => false,
        OptionOwner::Group { .. } | OptionOwner::Table => true,
        OptionOwner::Shared { participants } => {
            let mut players = participants.iter().map(|who| control.controller(*who));
            let Some(first) = players.next() else {
                return true;
            };
            players.any(|player| player != first)
        }
    }
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
