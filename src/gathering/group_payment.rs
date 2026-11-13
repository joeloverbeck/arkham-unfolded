//! Group clue payment (Rules Reference p.6, "Clues"): the investigators spend an
//! act's requisite clues as a group. One listed choice stands for every complete
//! allocation. A player chooses its identity with one contribution per
//! investigator in place of [`GROUP_CLUE_CONTRIBUTIONS`], and the session
//! validates the whole allocation once: eligibility, holdings and the exact
//! requirement. Nothing is spent before that one choice.

use super::runtime::{Frame, Game, Phase, PlayerState};
use super::scenario::{self, HALLWAY, ScenarioDecision};
use super::{Investigator, content};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};
use std::ops::{Index, IndexMut};

/// Stands for the allocation in the one listed group clue payment choice: one
/// whole number per investigator, in the order Tony, Patrice, Jacqueline,
/// separated by colons (`act:pay:2:4:0`).
pub const GROUP_CLUE_CONTRIBUTIONS: &str = "<contributions>";
const PREFIX: &str = "act:pay:";

/// The clues each investigator spends in one group clue payment, in player-card
/// order. It owns the choice identity's numbers: the identity is built, read and
/// summed here, and the observation shows it as one number per investigator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Allocation([u8; 3]);

impl From<[u8; 3]> for Allocation {
    fn from(clues: [u8; 3]) -> Self {
        Self(clues)
    }
}

impl Index<Investigator> for Allocation {
    type Output = u8;

    fn index(&self, who: Investigator) -> &u8 {
        &self.0[who.index()]
    }
}

impl IndexMut<Investigator> for Allocation {
    fn index_mut(&mut self, who: Investigator) -> &mut u8 {
        &mut self.0[who.index()]
    }
}

impl Allocation {
    /// Every clue the allocation spends.
    pub fn total(self) -> u16 {
        self.0.iter().map(|&clues| u16::from(clues)).sum()
    }

    /// The choice identity that commits this allocation.
    fn id(self) -> String {
        let [tony, patrice, jacqueline] = self.0;
        format!("{PREFIX}{tony}:{patrice}:{jacqueline}")
    }

    /// The allocation a choice identity names, in its one canonical spelling.
    pub(super) fn parse(id: &str) -> Option<Self> {
        let mut parts = id.strip_prefix(PREFIX)?.split(':');
        let mut clues = [0; 3];
        for paid in &mut clues {
            *paid = parts.next()?.parse().ok()?;
        }
        let allocation = Self(clues);
        (parts.next().is_none() && allocation.id() == id).then_some(allocation)
    }
}

/// Why an investigator cannot contribute clues to this payment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ineligibility {
    Defeated,
    Resigned,
    /// The Barrier's Objective names the investigators in the Hallway.
    OutsideHallway,
}

impl Ineligibility {
    /// Why this investigator cannot contribute to the current act's payment:
    /// defeated or resigned investigators never can, and The Barrier's Objective
    /// names the investigators in the Hallway.
    fn of(player: &PlayerState, act: u8) -> Option<Self> {
        if player.defeated {
            Some(Self::Defeated)
        } else if player.resigned {
            Some(Self::Resigned)
        } else if act == 2 && player.location != HALLWAY {
            Some(Self::OutsideHallway)
        } else {
            None
        }
    }

    pub fn reason(self) -> &'static str {
        match self {
            Self::Defeated => "Defeated",
            Self::Resigned => "Resigned",
            Self::OutsideHallway => "Not in the Hallway",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contributor {
    pub investigator: Investigator,
    /// Clues this investigator holds.
    pub clues: u8,
    pub location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ineligible: Option<Ineligibility>,
}

impl Contributor {
    /// The most this investigator can contribute: their holdings, never more
    /// than the requirement, and nothing when ineligible.
    pub fn limit(&self, required: u8) -> u8 {
        if self.ineligible.is_some() {
            0
        } else {
            self.clues.min(required)
        }
    }
}

/// The group clue payment the listed choice offers now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupCluePayment {
    /// The listed choice identity, ending in [`GROUP_CLUE_CONTRIBUTIONS`].
    pub choice: String,
    pub act: u8,
    pub act_title: String,
    /// Exactly this many clues are spent: per investigator who started the scenario.
    pub required: u8,
    /// The Barrier's round-end Objective; declining is its other choice.
    pub at_round_end: bool,
    /// Every investigator who started the scenario, in player-card order.
    pub contributors: Vec<Contributor>,
    /// The only complete allocation, when only one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forced: Option<Allocation>,
}

impl GroupCluePayment {
    /// Whether a choice identity is the listed payment, whose allocation is
    /// still to be named.
    pub fn is_listed(id: &str) -> bool {
        id.strip_prefix(PREFIX) == Some(GROUP_CLUE_CONTRIBUTIONS)
    }

    /// The choice identity that pays this allocation, when it is complete and legal.
    pub fn choice_for(&self, allocation: impl Into<Allocation>) -> Option<String> {
        let allocation = allocation.into();
        self.accepts(allocation).then(|| allocation.id())
    }

    /// The identity naming this allocation as the players' parts of a payment
    /// they share (#208), when each investigator is within their limit and no
    /// more than the requirement is named in all. A part need not be complete:
    /// only [`Self::choice_for`]'s identity pays.
    pub fn part_for(&self, allocation: impl Into<Allocation>) -> Option<String> {
        let allocation = allocation.into();
        self.within(allocation).then(|| allocation.id())
    }

    /// The allocation a payment's or a part's identity names, in its one canonical
    /// spelling.
    pub fn allocation_of(id: &str) -> Option<Allocation> {
        Allocation::parse(id)
    }

    /// Complete and legal: each investigator within their limit, and exactly the
    /// requirement in all.
    fn accepts(&self, allocation: Allocation) -> bool {
        self.within(allocation) && allocation.total() == u16::from(self.required)
    }

    /// Each investigator within their limit, and no more than the requirement.
    fn within(&self, allocation: Allocation) -> bool {
        self.contributors.len() == allocation.0.len()
            && self
                .contributors
                .iter()
                .zip(allocation.0)
                .all(|(contributor, clues)| clues <= contributor.limit(self.required))
            && allocation.total() <= u16::from(self.required)
    }

    fn label(&self, allocation: Allocation) -> String {
        let paid: Vec<_> = self
            .contributors
            .iter()
            .zip(allocation.0)
            .filter(|(_, clues)| *clues > 0)
            .map(|(contributor, clues)| format!("{} {clues}", contributor.investigator.name()))
            .collect();
        format!(
            "Spend {} clues to advance {}: {}",
            self.required,
            self.act_title,
            paid.join(", ")
        )
    }
}

pub(super) fn listed_id() -> String {
    format!("{PREFIX}{GROUP_CLUE_CONTRIBUTIONS}")
}

/// The payment when the investigators can afford it at this timing point.
fn payable(game: &Game, at_round_end: bool) -> Option<GroupCluePayment> {
    let act = game.act;
    let required = scenario::act_requirement(game, act, at_round_end)?;
    let contributors: Vec<_> = game
        .players
        .iter()
        .map(|player| Contributor {
            investigator: player.investigator,
            clues: player.clues,
            location: player.location.clone(),
            ineligible: Ineligibility::of(player, act),
        })
        .collect();
    let limits: Vec<_> = contributors
        .iter()
        .map(|contributor| contributor.limit(required))
        .collect();
    if limits.len() != 3
        || limits.iter().map(|&limit| u16::from(limit)).sum::<u16>() < u16::from(required)
    {
        return None;
    }
    let mut complete = (0..=limits[0])
        .flat_map(|first| (0..=limits[1]).map(move |second| (first, second)))
        .filter_map(|(first, second)| {
            let third = required.checked_sub(first)?.checked_sub(second)?;
            (third <= limits[2]).then_some(Allocation([first, second, third]))
        });
    let only = complete.next();
    let forced = only.filter(|_| complete.next().is_none());
    Some(GroupCluePayment {
        choice: listed_id(),
        act,
        act_title: content::title(if act == 1 { "01108" } else { "01109" })
            .expect("Selected act identity")
            .into(),
        required,
        at_round_end,
        contributors,
        forced,
    })
}

fn listed(payment: &GroupCluePayment) -> Choice {
    Choice {
        id: payment.choice.clone(),
        label: format!(
            "Spend {} clues as a group: advance {}",
            payment.required, payment.act_title
        ),
        cancel_faces: vec![],
    }
}

/// During an investigator's turn, including an action they were granted then.
fn during_turn(game: &Game) -> bool {
    let (actor, phase) = game
        .frames
        .iter()
        .find_map(|frame| match frame {
            Frame::RestoreTurn {
                previous_actor,
                previous_phase,
            } => Some((*previous_actor, *previous_phase)),
            _ => None,
        })
        .unwrap_or((game.actor, game.phase));
    actor.is_some() && phase == Phase::Investigation && game.scenario.outcome.is_none()
}

/// Trapped's payment, a fast player ability in every player window during an
/// investigator's turn (Rules Reference p.6, "Clues").
pub(super) fn player_window_choice(game: &Game) -> Option<Choice> {
    (game.act == 1 && during_turn(game))
        .then(|| payable(game, false))
        .flatten()
        .map(|payment| listed(&payment))
}

/// The Barrier's Objective at round end, when the Hallway can pay.
pub(super) fn round_end_choice(game: &Game) -> Option<Choice> {
    payable(game, true).map(|payment| listed(&payment))
}

fn at_round_end(game: &Game) -> bool {
    game.test.is_none()
        && matches!(
            game.frames.last(),
            Some(Frame::Scenario(ScenarioDecision::RoundEndWhen))
        )
}

/// The payment the current choices offer, for the observation.
pub(super) fn offered(game: &Game, choices: &[Choice]) -> Option<GroupCluePayment> {
    choices
        .iter()
        .any(|choice| GroupCluePayment::is_listed(&choice.id))
        .then(|| payable(game, at_round_end(game)))
        .flatten()
}

/// A complete, legal allocation of the listed payment, as the choice it commits.
pub(super) fn direct_choice(game: &Game, id: &str) -> Option<Choice> {
    let allocation = Allocation::parse(id)?;
    let payment = offered(game, &game.choices())?;
    payment.accepts(allocation).then(|| Choice {
        id: id.into(),
        label: payment.label(allocation),
        cancel_faces: vec![],
    })
}

/// Spend the validated allocation and advance the act. At round end this is The
/// Barrier's Objective; otherwise Trapped's payment inside the open window.
pub(super) fn pay(game: &mut Game, id: &str) -> Result<scenario::ScenarioStep, RuleError> {
    let allocation =
        Allocation::parse(id).ok_or_else(|| RuleError("Invalid group clue payment.".into()))?;
    if at_round_end(game) {
        scenario::advance_at_round_end(game, allocation)
    } else {
        spend(game, allocation, false)?;
        scenario::advance_act_one(game);
        Ok(scenario::ScenarioStep::Continue)
    }
}

/// Validate the entire allocation against the payment due at this timing point
/// before charging anyone. The player chooses the allocation; this never chooses
/// who pays.
pub(super) fn spend(
    game: &mut Game,
    allocation: Allocation,
    at_round_end: bool,
) -> Result<(), RuleError> {
    if !payable(game, at_round_end).is_some_and(|payment| payment.accepts(allocation)) {
        return Err(RuleError(
            "The current act requires the exact group clue payment.".into(),
        ));
    }
    for (player, clues) in game.players.iter_mut().zip(allocation.0) {
        player.clues -= clues;
    }
    Ok(())
}

/// Seam 1: the rules session. Positions are reached through genuine decisions;
/// setup edits only place clues, enemies or investigators where a witness needs
/// them. Sources: Rules Reference p.6 "Clues", Trapped (01108) and The Barrier
/// (01109).
#[cfg(test)]
mod tests {
    use super::{Allocation, GROUP_CLUE_CONTRIBUTIONS, Ineligibility};
    use crate::gathering::scenario::{CELLAR, EnemyState, HALLWAY, STUDY};
    use crate::gathering::{CardView, HostileFixture, Investigator, Session};

    const LISTED: &str = "act:pay:<contributions>";

    fn ids(session: &Session) -> Vec<String> {
        session
            .observe()
            .choices
            .into_iter()
            .map(|choice| choice.id)
            .collect()
    }

    fn choose(session: &mut Session, id: &str) {
        let view = session.observe();
        session
            .choose(view.revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error:?}; offered {:?}", ids(session)));
    }

    /// Tony's first action choice of real play: Trapped, with the Study's clues
    /// still on the Study.
    fn tonys_turn() -> Session {
        let mut session = Session::with_seed(89);
        for id in [
            "lead:tony",
            "confirm-mulligan",
            "confirm-mulligan",
            "confirm-mulligan",
            "turn:tony",
        ] {
            choose(&mut session, id);
        }
        session
    }

    fn hold(session: &mut Session, clues: [u8; 3]) {
        for (player, clues) in session
            .runtime
            .as_mut()
            .unwrap()
            .players
            .iter_mut()
            .zip(clues)
        {
            player.clues = clues;
        }
    }

    fn clues(session: &Session) -> Vec<u8> {
        session
            .observe()
            .play
            .unwrap()
            .players
            .iter()
            .map(|player| player.clues)
            .collect()
    }

    #[test]
    fn trapped_is_one_listed_payment_whose_observation_carries_the_decision() {
        let mut session = tonys_turn();
        hold(&mut session, [3, 1, 1]);
        assert!(!ids(&session).iter().any(|id| id.starts_with("act:")));
        assert_eq!(session.observe().group_clue_payment, None);

        hold(&mut session, [4, 2, 1]);
        let view = session.observe();
        let listed: Vec<_> = view
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("act:"))
            .collect();
        assert_eq!(listed.len(), 1, "No single-clue contribution choices");
        assert_eq!(listed[0].id, LISTED);
        assert!(listed[0].id.ends_with(GROUP_CLUE_CONTRIBUTIONS));
        assert_eq!(listed[0].label, "Spend 6 clues as a group: advance Trapped");
        let payment = view.group_clue_payment.unwrap();
        assert_eq!(payment.choice, LISTED);
        assert_eq!((payment.act, payment.act_title.as_str()), (1, "Trapped"));
        // 2 per investigator who started the scenario.
        assert_eq!(payment.required, 6);
        assert!(!payment.at_round_end);
        let held: Vec<_> = payment
            .contributors
            .iter()
            .map(|who| (who.investigator, who.clues, who.ineligible))
            .collect();
        assert_eq!(
            held,
            [
                (Investigator::Tony, 4, None),
                (Investigator::Patrice, 2, None),
                (Investigator::Jacqueline, 1, None),
            ]
        );
        assert!(payment.contributors.iter().all(|who| who.location == STUDY));
        assert_eq!(payment.forced, None, "4+2+0, 4+1+1 and 3+2+1 all pay");
        assert_eq!(
            payment.choice_for([4, 1, 1]).as_deref(),
            Some("act:pay:4:1:1")
        );
        assert_eq!(
            payment.choice_for([4, 1, 0]),
            None,
            "Short of the requirement"
        );
        assert_eq!(
            payment.choice_for([3, 2, 2]),
            None,
            "Beyond Jacqueline's clue"
        );
    }

    #[test]
    fn the_only_split_is_named_and_ineligible_investigators_contribute_nothing() {
        let mut session = tonys_turn();
        hold(&mut session, [4, 2, 0]);
        assert_eq!(
            session.observe().group_clue_payment.unwrap().forced,
            Some(Allocation::from([4, 2, 0]))
        );

        hold(&mut session, [5, 1, 3]);
        session.runtime.as_mut().unwrap().players[Investigator::Jacqueline.index()].resigned = true;
        let payment = session.observe().group_clue_payment.unwrap();
        assert_eq!(
            payment.contributors[Investigator::Jacqueline.index()].ineligible,
            Some(Ineligibility::Resigned)
        );
        assert_eq!(payment.forced, Some(Allocation::from([5, 1, 0])));
        let revision = session.observe().revision;
        assert!(session.choose(revision, "act:pay:4:0:2").is_err());
        assert_eq!(clues(&session), [5, 1, 3]);
    }

    #[test]
    fn a_defeated_investigator_contributes_nothing_and_the_observation_names_the_split() {
        let mut session = tonys_turn();
        hold(&mut session, [4, 3, 2]);
        session.runtime.as_mut().unwrap().players[Investigator::Patrice.index()].defeated = true;
        let view = session.observe();
        let payment = view.group_clue_payment.as_ref().unwrap();
        assert_eq!(
            payment.contributors[Investigator::Patrice.index()].ineligible,
            Some(Ineligibility::Defeated)
        );
        // Tony's 4 and Jacqueline's 2 are the only way to spend 6.
        let observed = serde_json::to_value(&view).unwrap();
        assert_eq!(
            observed["group_clue_payment"]["forced"],
            serde_json::json!([4, 0, 2]),
            "The observation names the split as one number per investigator"
        );
        assert_eq!(
            payment.choice_for([4, 0, 2]).as_deref(),
            Some("act:pay:4:0:2")
        );
        assert_eq!(payment.choice_for([3, 1, 2]), None, "Patrice is defeated");
        let revision = view.revision;
        assert!(session.choose(revision, "act:pay:3:1:2").is_err());
        assert_eq!(
            clues(&session),
            [4, 3, 2],
            "A refused allocation spends nothing"
        );
        choose(&mut session, "act:pay:4:0:2");
        assert_eq!(session.observe().play.unwrap().act, 2);
        assert_eq!(clues(&session), [0, 3, 0]);
    }

    #[test]
    fn one_choice_spends_a_complete_allocation_and_advances_trapped() {
        let mut session = tonys_turn();
        hold(&mut session, [4, 2, 1]);
        let before = session.clone();
        // Overpaying, underpaying, spending clues nobody holds, the placeholder
        // and non-canonical spellings are refused without any change.
        for refused in [
            "act:pay:4:2:1",
            "act:pay:3:1:1",
            "act:pay:2:2:2",
            LISTED,
            "act:pay:04:1:1",
            "act:pay:4:1:1:0",
            "act:pay:4:1",
        ] {
            let revision = session.observe().revision;
            assert!(session.choose(revision, refused).is_err(), "{refused}");
            assert_eq!(session, before, "{refused}");
        }

        choose(&mut session, "act:pay:4:1:1");
        let view = session.observe();
        assert_eq!(view.revision, before.observe().revision + 1, "One decision");
        let play = view.play.as_ref().unwrap();
        assert_eq!(play.act, 2);
        assert_eq!(clues(&session), [0, 1, 0]);
        assert!(play.players.iter().all(|player| player.location == HALLWAY));
        assert!(
            view.history.contains(
                &"Spend 6 clues to advance Trapped: Tony Morgan 4, Patrice Hathaway 1, Jacqueline Fine 1"
                    .to_owned()
            )
        );
        // The act's revealed reverse remains the Undo floor.
        assert!(!ids(&session).contains(&"undo".to_owned()));
        assert_eq!(view.group_clue_payment, None);
    }

    #[test]
    fn trapped_is_offered_in_a_skill_test_window_and_the_test_still_resolves() {
        let mut session = tonys_turn();
        hold(&mut session, [4, 2, 1]);
        choose(&mut session, "action:ordinary:investigate");
        let window = ids(&session);
        assert!(
            window.contains(&"window:pass".to_owned()) && window.contains(&LISTED.to_owned()),
            "The player window before commitment offers the payment: {window:?}"
        );
        choose(&mut session, "act:pay:4:2:0");
        assert_eq!(session.observe().play.unwrap().act, 2);
        finish_test(&mut session);
        let play = session.observe().play.unwrap();
        assert_eq!(play.actor, Some(Investigator::Tony));
        assert!(play.test.is_none());
        assert_eq!(clues(&session)[2], 1);
    }

    /// Fight and evade a Study enemy that Trapped discards mid-test: the test
    /// still completes, and its effect on the departed enemy is lost.
    #[test]
    fn trapped_paid_mid_test_discards_the_tested_enemy_without_stalling_the_test() {
        for action in ["fight", "evade"] {
            let mut session = tonys_turn();
            hold(&mut session, [4, 2, 1]);
            let game = session.runtime.as_mut().unwrap();
            let index = game
                .encounter_deck
                .iter()
                .position(|card| card.code == "01160")
                .expect("A Ghoul Minion in the encounter deck");
            let card: CardView = game.encounter_deck.remove(index);
            let id = card.id;
            game.scenario.enemies.push(EnemyState {
                card,
                location: STUDY.into(),
                engaged: Some(Investigator::Tony),
                exhausted: false,
                damage: 0,
                doom: 0,
                bounties: 0,
                owner: None,
            });
            choose(&mut session, &format!("action:ordinary:{action}:{id}"));
            // Make the test succeed unless the automatic failure is drawn.
            session
                .runtime
                .as_mut()
                .unwrap()
                .test
                .as_mut()
                .unwrap()
                .difficulty = 0;
            choose(&mut session, "act:pay:4:2:0");
            assert!(
                !session
                    .observe()
                    .play
                    .unwrap()
                    .scenario
                    .enemies
                    .iter()
                    .any(|enemy| enemy.card.id == id),
                "{action}"
            );
            finish_test(&mut session);
            let play = session.observe().play.unwrap();
            assert!(play.test.is_none(), "{action}");
            assert_eq!(play.actor, Some(Investigator::Tony), "{action}");
        }
    }

    fn finish_test(session: &mut Session) {
        for _ in 0..30 {
            let offered = ids(session);
            if session.observe().play.unwrap().test.is_none()
                && offered.contains(&"turn:end".to_owned())
            {
                return;
            }
            let next = [
                "window:pass",
                "commit:confirm",
                "test:reveal",
                "tokens:confirm",
                "lucky:decline",
                "success:pass",
                "after-test:pass",
            ]
            .into_iter()
            .find(|id| offered.iter().any(|offered| offered == id))
            .map(str::to_owned)
            .or_else(|| offered.iter().find(|id| id.starts_with("result:")).cloned())
            .unwrap_or_else(|| panic!("No test step among {offered:?}"));
            choose(session, &next);
        }
        panic!("The test did not finish");
    }

    #[test]
    fn trapped_is_not_offered_outside_an_investigators_turn() {
        let mut session = Session::with_seed(89);
        for id in [
            "lead:tony",
            "confirm-mulligan",
            "confirm-mulligan",
            "confirm-mulligan",
        ] {
            choose(&mut session, id);
        }
        hold(&mut session, [2, 2, 2]);
        assert_eq!(session.observe().play.unwrap().actor, None);
        assert!(!ids(&session).contains(&LISTED.to_owned()));
        assert_eq!(session.observe().group_clue_payment, None);
    }

    /// Review fix B1 (#208): a part of a shared payment is named and judged on
    /// the rules side. Players set their own investigators' clues one part at a
    /// time, so a part may be incomplete; RR p.7 still bounds each investigator by
    /// the clues they hold, and The Barrier requires exactly 9 here.
    #[test]
    fn a_shared_payments_part_is_named_within_holdings_and_the_requirement() {
        let session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
        let payment = session.observe().group_clue_payment.unwrap();
        assert_eq!(payment.part_for([3, 0, 0]), Some("act:pay:3:0:0".into()));
        assert_eq!(payment.part_for([3, 3, 3]), Some("act:pay:3:3:3".into()));
        assert_eq!(payment.part_for([4, 0, 0]), None, "Tony holds 3 clues");
        assert_eq!(
            payment.choice_for([3, 0, 0]),
            None,
            "Only a complete one pays"
        );
        assert_eq!(
            super::GroupCluePayment::allocation_of("act:pay:0:2:1"),
            Some(Allocation::from([0, 2, 1]))
        );
        assert_eq!(super::GroupCluePayment::allocation_of(LISTED), None);
    }

    #[test]
    fn the_barrier_pays_from_the_hallway_at_round_end_or_declines() {
        let session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
        assert_eq!(ids(&session), ["round-end:decline-act", LISTED]);
        let payment = session.observe().group_clue_payment.unwrap();
        assert!(payment.at_round_end);
        // 3 per investigator who started the scenario.
        assert_eq!(
            (payment.act_title.as_str(), payment.required),
            ("The Barrier", 9)
        );
        assert_eq!(payment.forced, Some(Allocation::from([3, 3, 3])));

        let mut declined = session.clone();
        choose(&mut declined, "round-end:decline-act");
        assert_eq!(clues(&declined), [3, 3, 3], "Declining spends nothing");
        assert_eq!(declined.observe().play.unwrap().act, 2);

        // Jacqueline's clues in the Cellar cannot be spent; the Hallway's can.
        let mut split = session.clone();
        let game = split.runtime.as_mut().unwrap();
        game.players[Investigator::Jacqueline.index()].location = CELLAR.into();
        game.players[Investigator::Tony.index()].clues = 6;
        let payment = split.observe().group_clue_payment.unwrap();
        assert_eq!(
            payment.contributors[Investigator::Jacqueline.index()].ineligible,
            Some(Ineligibility::OutsideHallway)
        );
        assert_eq!(payment.forced, Some(Allocation::from([6, 3, 0])));
        let revision = split.observe().revision;
        assert!(split.choose(revision, "act:pay:3:3:3").is_err());
        choose(&mut split, "act:pay:6:3:0");
        let play = split.observe().play.unwrap();
        assert_eq!(play.act, 3, "The Barrier advanced");
        assert!(play.scenario.priest_spawned);
        assert_eq!(clues(&split), [0, 0, 3]);
    }
}
