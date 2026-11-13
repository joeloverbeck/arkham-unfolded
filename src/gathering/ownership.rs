//! Who may answer each option of the pending decision. Ownership is derived
//! from the rules session whenever it is asked for and is never saved.
//!
//! An option belongs to the player of the investigator the rules give it to:
//! their own turn, test, harm, encounter card, or a card they control (Rules
//! Reference p.3 "Triggered Abilities", p.15 "May"). A group choice, which the
//! rules leave to the investigators collectively or give to the lead
//! investigator, has the lead as arbiter (RR p.14, p.17, FAQ §1.5); after the
//! lead is eliminated, the next investigator in player order stands in. In a
//! shared decision each participant's player sets only their own part, and the
//! option that completes it names every participant. The table's setup choice
//! of the lead belongs to no investigator.

use super::runtime::{Frame, Game, Window};
use super::{GroupCluePayment, Investigator, elimination, group_payment, phases, scenario};
use crate::Choice;

/// Whose player may submit one listed option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionOwner {
    /// This investigator's player answers it.
    Investigator(Investigator),
    /// A group choice: the arbiter's player makes it for every investigator.
    Group { arbiter: Investigator },
    /// The option completes a shared decision: a commitment window, a group
    /// clue payment, or a window for fast cards or abilities. Each
    /// participant's player sets only their own part of it.
    Shared { participants: Vec<Investigator> },
    /// The table's setup choice of the lead, owned by no investigator.
    Table,
}

/// One listed option of the pending decision and its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedOption {
    pub id: String,
    pub owner: OptionOwner,
}

/// The owner of each listed option, in listed order.
pub(super) fn owners(game: &Game, choices: &[Choice]) -> Vec<OptionOwner> {
    let payment = group_payment::offered(game, choices).map(|payment| contributors(&payment));
    let rule = Rule::of(game);
    let mut owners: Vec<_> = choices
        .iter()
        .map(|choice| rule.owner(game, &choice.id, payment.as_deref()))
        .collect();
    if let Rule::Window {
        own,
        close: Some(close),
    } = rule
    {
        // A window closes when everyone with an option in it has passed. The
        // commitment window also waits for the tester, whose Confirm it is.
        let mut participants = Vec::new();
        if close == "commit:confirm" {
            participants.push(own);
        }
        for owner in owners.iter().flatten() {
            match owner {
                OptionOwner::Investigator(who) => participants.push(*who),
                OptionOwner::Shared { participants: part } => participants.extend(part),
                OptionOwner::Group { .. } | OptionOwner::Table => {}
            }
        }
        let participants = in_player_card_order(participants);
        for owner in &mut owners {
            if owner.is_none() {
                *owner = Some(OptionOwner::Shared {
                    participants: participants.clone(),
                });
            }
        }
    }
    owners
        .into_iter()
        .map(|owner| owner.expect("Only a window's closing option waits for its participants"))
        .collect()
}

/// The lead, or after the lead's elimination the next investigator in player
/// order who remains (RR p.13 "In Player Order"; the play contract's house
/// practice for choosing a new lead).
fn arbiter(game: &Game) -> Investigator {
    let lead = game.lead;
    let active = |who: Investigator| {
        let player = &game.players[who.index()];
        !player.defeated && !player.resigned
    };
    if active(lead) {
        return lead;
    }
    (1..Investigator::ALL.len())
        .map(|step| Investigator::ALL[(lead.index() + step) % Investigator::ALL.len()])
        .find(|&who| active(who))
        .unwrap_or(lead)
}

/// How the pending frame assigns its listed options. The dispatch follows the
/// order in which the rules session lists them.
enum Rule {
    /// Every option belongs to this investigator.
    Investigator(Investigator),
    /// A group choice: the arbiter answers every option.
    Group(Investigator),
    /// Options from several investigators in one list. An option that uses a
    /// player's card belongs to that card's controller, a group clue payment
    /// is shared by its contributors, and every other option belongs to `own`.
    /// `close` passes the window, once everyone with an option is ready.
    Window {
        own: Investigator,
        close: Option<&'static str>,
    },
    /// A chaos-token reveal: Jacqueline's reaction is hers.
    Tokens(Investigator),
    /// The Barrier's round-end Objective: paying or declining is shared by the
    /// investigators able to contribute.
    RoundEndPayment,
}

impl Rule {
    fn of(game: &Game) -> Self {
        let group = Self::Group(arbiter(game));
        let tester = game.test.as_ref().map(|test| test.tester);
        match game.frames.last() {
            Some(Frame::Backpack(decision)) => return Self::Investigator(decision.owner()),
            Some(Frame::Peter { owner } | Frame::GrantedAction { owner }) => {
                return Self::Investigator(*owner);
            }
            Some(Frame::OutcomeWindow) => {
                return Self::Window {
                    own: tester.expect("Outcome reactions belong to a test"),
                    close: Some("success:pass"),
                };
            }
            Some(Frame::AssetEntry(entry)) => return Self::Investigator(entry.actor()),
            Some(Frame::AfterTest(state)) => {
                return Self::Window {
                    own: state.test.tester,
                    close: Some("after-test:pass"),
                };
            }
            Some(Frame::TestCard(super::test_cards::TestCardDecision::Initiation)) => {
                return Self::Window {
                    own: tester.expect("Test-begin reactions belong to a test"),
                    close: Some("test-card:pass"),
                };
            }
            // The order of simultaneous commitment effects (RR p.17).
            Some(Frame::TestCard(super::test_cards::TestCardDecision::CommitEffects {
                ..
            })) => {
                return group;
            }
            Some(Frame::TestCard(_)) => {
                return Self::Investigator(
                    super::test_cards::decision_owner(game).expect("Test card decision owner"),
                );
            }
            Some(Frame::EncounterSearch(decision))
                if (game.test.is_none() && game.tokens.is_none())
                    || !matches!(
                        decision,
                        super::encounter_search::SearchDecision::FinishEvent { .. }
                    ) =>
            {
                return Self::Investigator(super::encounter_search::decision_owner(decision));
            }
            Some(Frame::Elimination(elimination::EliminationDecision::Trauma(who))) => {
                return Self::Investigator(*who);
            }
            // RR p.14: the remaining players choose a new lead.
            Some(Frame::Elimination(elimination::EliminationDecision::Lead)) => return group,
            Some(Frame::DiscardCards { owner, .. }) => return Self::Investigator(*owner),
            Some(Frame::Draw(draw)) => return Self::Investigator(draw.investigator),
            Some(Frame::Spell(
                super::spells::SpellDecision::MistsDiscard(owner)
                | super::spells::SpellDecision::MistsMove(owner),
            )) => return Self::Investigator(*owner),
            Some(Frame::Combat(decision)) => {
                let owner = super::combat::decision_owner(decision);
                return if matches!(decision, super::combat::CombatDecision::DefeatReactions(_)) {
                    Self::Window {
                        own: owner,
                        close: Some("defeat:done"),
                    }
                } else {
                    Self::Investigator(owner)
                };
            }
            Some(Frame::AssetPlacement(prepared)) => return Self::Investigator(prepared.actor),
            Some(Frame::Tony(super::tony_cards::TonyDecision::MauserReward { owner, .. })) => {
                return Self::Investigator(*owner);
            }
            Some(Frame::Tony(super::tony_cards::TonyDecision::Bounties { .. })) => {
                return Self::Investigator(Investigator::Tony);
            }
            // A forced spawn among several farthest locations (FAQ §1.5).
            Some(Frame::QuarrySpawn { .. }) => return group,
            // Act 3b: the lead investigator must decide.
            Some(Frame::Scenario(scenario::ScenarioDecision::Ending)) => return group,
            // RR p.7: the dealt investigator assigns and prevents their own harm.
            Some(Frame::Harm(harm)) => return Self::Investigator(harm.investigator),
            Some(Frame::Idol(decision)) => return Self::Investigator(decision.owner()),
            Some(Frame::CryptChill(decision)) => return Self::Investigator(decision.owner),
            Some(Frame::HandDiscard(discard)) => {
                return Self::Investigator(discard.current.unwrap_or(game.lead));
            }
            // RR p.26 ST.7: the tester applies several results in their order.
            Some(Frame::TestResults(_)) => {
                return Self::Investigator(tester.expect("Test results belong to a test"));
            }
            _ => {}
        }
        if let Some(tokens) = &game.tokens {
            return if tokens.stage == super::tokens::TokenStage::Declaration {
                Self::Tokens(tokens.revealer)
            } else {
                Self::Investigator(
                    super::tokens::decision_owner(game).expect("Token resolution owner"),
                )
            };
        }
        if let Some(test) = &game.test {
            return match test.window {
                // RR p.26 ST.2: the tester and each other investigator at the
                // same location commit their own cards.
                Window::Commitment => Self::Window {
                    own: test.tester,
                    close: Some("commit:confirm"),
                },
                Window::BeforeCommitment | Window::BeforeReveal => Self::Window {
                    own: game.actor.unwrap_or(test.tester),
                    close: Some("window:pass"),
                },
                _ => Self::Investigator(test.tester),
            };
        }
        match game.frames.last() {
            // RR p.24 1.4: the drawing investigator resolves the card; A Test
            // of Will is played by its holder.
            Some(Frame::Revelation(revelation)) => Self::Window {
                own: revelation.drawn_by,
                close: None,
            },
            Some(Frame::Scenario(scenario::ScenarioDecision::RoundEndWhen)) => {
                Self::RoundEndPayment
            }
            // Agenda 1b, prey and hunter ties (RR pp.12, 17), round-end forced
            // effects in the lead's order (RR p.17), and which investigator
            // earns Lita at the resolution.
            Some(Frame::Scenario(_)) => group,
            Some(Frame::Phase(decision)) => phase(game, decision),
            // Simultaneous forced end-of-turn effects (RR p.17).
            Some(Frame::TurnEnd { .. }) => group,
            // RR p.24: attacks of opportunity in the investigator's order.
            Some(Frame::Opportunity { investigator, .. }) => Self::Investigator(*investigator),
            Some(_) => group,
            // RR p.24 2.2: the investigators choose who takes the next turn.
            None => match game.actor {
                None => group,
                Some(actor) => Self::Window {
                    own: actor,
                    close: None,
                },
            },
        }
    }

    /// The owner of one option, or `None` for the option that closes a window.
    /// `payment` lists the contributors of a listed group clue payment.
    fn owner(
        &self,
        game: &Game,
        id: &str,
        payment: Option<&[Investigator]>,
    ) -> Option<OptionOwner> {
        let shared = |participants: &[Investigator]| OptionOwner::Shared {
            participants: participants.to_vec(),
        };
        Some(match self {
            Self::Investigator(who) => OptionOwner::Investigator(*who),
            Self::Group(arbiter) => OptionOwner::Group { arbiter: *arbiter },
            Self::Tokens(revealer) => {
                OptionOwner::Investigator(if id.starts_with("tokens:jacqueline") {
                    Investigator::Jacqueline
                } else {
                    *revealer
                })
            }
            Self::RoundEndPayment => payment.map_or_else(
                || OptionOwner::Group {
                    arbiter: arbiter(game),
                },
                shared,
            ),
            Self::Window { own, close } => {
                if Some(id) == *close {
                    return None;
                }
                match payment {
                    Some(participants) if GroupCluePayment::is_listed(id) => shared(participants),
                    _ => OptionOwner::Investigator(card_user(game, id).unwrap_or(*own)),
                }
            }
        })
    }
}

fn phase(game: &Game, decision: &phases::PhaseDecision) -> Rule {
    use phases::PhaseDecision::*;
    match decision {
        PlayerWindow { .. } => Rule::Window {
            own: game.actor.unwrap_or_else(|| arbiter(game)),
            close: Some("phase-window:pass"),
        },
        // RR p.25: in player order; an investigator resolves their own
        // attacks, draws and hand size, attacks in the order they choose.
        EnemyAttacks { order, index }
        | UpkeepDraw { order, index }
        | HandLimit { order, index }
        | MythosDraw { order, index } => order.get(*index).map_or_else(
            || Rule::Group(arbiter(game)),
            |who| Rule::Investigator(*who),
        ),
        AfterAttack(attack) => Rule::Investigator(attack.investigator),
        PatriceDiscard { .. } => Rule::Investigator(Investigator::Patrice),
        // Hunter order (RR p.17) and the framework's own steps.
        _ => Rule::Group(arbiter(game)),
    }
}

/// The investigators able to contribute to a group clue payment (RR p.7).
fn contributors(payment: &GroupCluePayment) -> Vec<Investigator> {
    payment
        .contributors
        .iter()
        .filter(|contributor| contributor.limit(payment.required) > 0)
        .map(|contributor| contributor.investigator)
        .collect()
}

/// The controller of the player card an option uses, by the option's identity.
fn card_user(game: &Game, id: &str) -> Option<Investigator> {
    const FIRST: [&str; 7] = [
        "premonition:",
        "violin:",
        "boost:",
        "commit:",
        "test-of-will:",
        "test-card:predict:",
        "tony:gregory:",
    ];
    if let Some(who) = id.strip_prefix("tony:quick-thinking:") {
        return Investigator::ALL
            .into_iter()
            .find(|investigator| investigator.id() == who);
    }
    let card = if let Some(rest) = id.strip_prefix("prescient:return:") {
        rest.split(':').nth(1)
    } else {
        FIRST
            .iter()
            .find_map(|prefix| id.strip_prefix(prefix))
            .and_then(|rest| rest.split(':').next())
    }?
    .parse::<u16>()
    .ok()?;
    let holder = game.players.iter().find(|player| {
        player.hand.iter().any(|held| held.id == card)
            || player.assets.iter().any(|asset| asset.card.id == card)
            || player.discard.iter().any(|held| held.id == card)
    });
    debug_assert!(holder.is_some(), "{id} names no player's card");
    holder.map(|player| player.investigator)
}

fn in_player_card_order(mut investigators: Vec<Investigator>) -> Vec<Investigator> {
    investigators.sort_by_key(|who| who.index());
    investigators.dedup();
    investigators
}

/// Seam: the rules session's ownership of its listed options. Setup edits only
/// place selected cards, clues and investigators where a witness needs them;
/// every expectation is read from the Rules Reference (AHC60), card text or
/// the observation, never from this module. Sources: RR pp.3, 7, 15, 17, 24,
/// 26; A Test of Will (60513), Premonition (04199), Patrice's Violin (06016),
/// Unexpected Courage (01093), Trapped (01108).
#[cfg(test)]
mod tests {
    use super::OptionOwner;
    use crate::gathering::runtime::{AssetState, Game};
    use crate::gathering::{HostileFixture, Investigator, Session};
    use Investigator::{Jacqueline, Patrice, Tony};

    fn choose(session: &mut Session, id: &str) {
        session
            .choose(session.observe().revision, id)
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }

    fn owner(session: &Session, id: &str) -> OptionOwner {
        session
            .option_owner(id)
            .unwrap_or_else(|| panic!("{id} is not accepted: {:?}", session.ownership()))
    }

    fn game(session: &mut Session) -> &mut Game {
        session.runtime.as_mut().expect("Scenario play")
    }

    /// A card with this code in `to`'s hand: one already there, or else the
    /// first from a deck, or else from another hand.
    fn give(session: &mut Session, code: &str, to: Investigator) -> u16 {
        let game = game(session);
        if let Some(card) = game.players[to.index()]
            .hand
            .iter()
            .find(|card| card.code == code)
        {
            return card.id;
        }
        let take = |cards: &mut Vec<crate::gathering::CardView>| {
            let at = cards.iter().position(|card| card.code == code)?;
            Some(cards.remove(at))
        };
        let card = game
            .players
            .iter_mut()
            .find_map(|player| take(&mut player.deck))
            .or_else(|| {
                game.players
                    .iter_mut()
                    .find_map(|player| take(&mut player.hand))
            })
            .unwrap_or_else(|| panic!("{code} is in a selected deck"));
        let id = card.id;
        game.players[to.index()].hand.push(card);
        id
    }

    fn put_into_play(session: &mut Session, code: &str, to: Investigator) -> u16 {
        let id = give(session, code, to);
        let player = &mut game(session).players[to.index()];
        let at = player.hand.iter().position(|card| card.id == id).unwrap();
        let card = player.hand.remove(at);
        player.assets.push(AssetState {
            card,
            ready: true,
            uses: 0,
            damage: 0,
            horror: 0,
        });
        id
    }

    fn turn(of: Investigator) -> Session {
        let mut session = Session::with_seed(89);
        for id in [
            "lead:jacqueline",
            "confirm-mulligan",
            "confirm-mulligan",
            "confirm-mulligan",
        ] {
            choose(&mut session, id);
        }
        choose(&mut session, &format!("turn:{}", of.id()));
        session
    }

    /// Premonition is played by the player holding it during any fast player
    /// window; Patrice's Violin is a fast ability its controller uses, though
    /// it may benefit another investigator (RR p.3, p.15). Tony's turn is his
    /// own (RR p.24 2.2.1). A fast window closes once everyone with an
    /// option in it has passed.
    #[test]
    fn fast_cards_belong_to_their_holders_inside_another_investigators_turn() {
        let mut session = turn(Tony);
        let premonition = give(&mut session, "04199", Jacqueline);
        let violin = put_into_play(&mut session, "06016", Patrice);
        let tony = &mut game(&mut session).players[Tony.index()];
        tony.deck.append(&mut tony.hand);
        let owned = session.ownership();
        assert_eq!(
            owner(&session, &format!("premonition:{premonition}")),
            OptionOwner::Investigator(Jacqueline)
        );
        let violins: Vec<_> = owned
            .iter()
            .filter(|option| option.id.starts_with(&format!("violin:{violin}:")))
            .collect();
        assert!(
            violins
                .iter()
                .any(|option| option.id.contains(":target:tony:"))
        );
        for option in violins {
            assert_eq!(
                option.owner,
                OptionOwner::Investigator(Patrice),
                "{}",
                option.id
            );
        }
        for option in owned
            .iter()
            .filter(|option| option.id == "turn:end" || option.id.starts_with("action:"))
        {
            assert_eq!(
                option.owner,
                OptionOwner::Investigator(Tony),
                "{}",
                option.id
            );
        }

        choose(&mut session, "action:ordinary:investigate");
        assert_eq!(
            owner(&session, "window:pass"),
            OptionOwner::Shared {
                participants: vec![Patrice, Jacqueline]
            }
        );
    }

    /// RR p.26 ST.2: the tester commits any number of their own cards and each
    /// other investigator at the same location may commit one of theirs.
    /// Unexpected Courage's two wild icons fit any test. The commitment window
    /// is shared by the tester and each investigator there able to commit.
    #[test]
    fn each_investigator_commits_their_own_cards_and_shares_the_window_with_the_tester() {
        let mut session = turn(Tony);
        let tonys = give(&mut session, "01093", Tony);
        let patrices = give(&mut session, "01093", Patrice);
        let game = game(&mut session);
        game.players[Jacqueline.index()].location = "01112".into();
        let jacquelines = game.players[Jacqueline.index()].hand.clone();
        choose(&mut session, "action:ordinary:investigate");
        if session
            .ownership()
            .iter()
            .any(|option| option.id == "window:pass")
        {
            choose(&mut session, "window:pass");
        }
        assert_eq!(
            owner(&session, &format!("commit:{tonys}")),
            OptionOwner::Investigator(Tony)
        );
        assert_eq!(
            owner(&session, &format!("commit:{patrices}")),
            OptionOwner::Investigator(Patrice)
        );
        assert!(
            jacquelines.iter().all(|card| session
                .option_owner(&format!("commit:{}", card.id))
                .is_none()),
            "Jacqueline is elsewhere"
        );
        assert_eq!(
            owner(&session, "commit:confirm"),
            OptionOwner::Shared {
                participants: vec![Tony, Patrice]
            }
        );
    }

    /// A Test of Will: "Play when an investigator at your location draws a
    /// non-weakness treachery card." Its holder plays it; the drawing
    /// investigator resolves the encounter card (RR p.24 1.4), whoever leads.
    #[test]
    fn a_test_of_will_belongs_to_its_holder_and_the_encounter_to_its_drawer() {
        let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
        let tonys = give(&mut session, "60513", Tony);
        let game = game(&mut session);
        game.lead = Jacqueline;
        let drawer_location = game.players[Patrice.index()].location.clone();
        let tony = &mut game.players[Tony.index()];
        tony.location = drawer_location;
        tony.resources = tony.resources.max(1);
        let play = session.observe().play.unwrap();
        assert_eq!(play.pending_revelation.as_ref().unwrap().drawn_by, Patrice);
        assert_eq!(
            owner(&session, &format!("test-of-will:{tonys}")),
            OptionOwner::Investigator(Tony)
        );
        for card in &play.players[Patrice.index()].hand {
            if card.code == "60513" {
                assert_eq!(
                    owner(&session, &format!("test-of-will:{}", card.id)),
                    OptionOwner::Investigator(Patrice)
                );
            }
        }
        assert_eq!(
            owner(&session, "revelation:resolve"),
            OptionOwner::Investigator(Patrice)
        );
    }

    /// RR p.7 "Clues" and Trapped (01108): any or all investigators may
    /// contribute clues to the group payment; one holding none has no part.
    #[test]
    fn a_group_clue_payment_is_shared_by_the_investigators_able_to_contribute() {
        for (held, participants) in [
            ([4, 2, 0], vec![Tony, Patrice]),
            ([4, 2, 1], vec![Tony, Patrice, Jacqueline]),
        ] {
            let mut session = turn(Tony);
            for (player, clues) in game(&mut session).players.iter_mut().zip(held) {
                player.clues = clues;
            }
            let shared = OptionOwner::Shared { participants };
            assert_eq!(owner(&session, "act:pay:<contributions>"), shared);
            assert_eq!(
                owner(&session, "act:pay:4:2:0"),
                shared,
                "A named allocation"
            );
            assert_eq!(
                owner(&session, "action:ordinary:resource"),
                OptionOwner::Investigator(Tony)
            );
        }
    }

    /// RR p.17 "Prey": when several investigators tie, the lead investigator
    /// decides which one the enemy engages.
    #[test]
    fn the_lead_breaks_a_tie_for_an_enemys_engagement() {
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringHunters);
        let game = game(&mut session);
        game.lead = Tony;
        let rats = game
            .scenario
            .enemies
            .iter_mut()
            .find(|enemy| enemy.card.code == "01159")
            .expect("Swarm of Rats, which has no prey instruction");
        rats.engaged = None;
        rats.exhausted = false;
        let (id, location) = (rats.card.id, rats.location.clone());
        for (player, here) in game.players.iter_mut().zip([false, true, true]) {
            if here {
                player.location = location.clone();
            } else if player.location == location {
                player.location = "01111".into();
            }
        }
        game.check_engagement(id);
        let owned = session.ownership();
        assert_eq!(
            owned
                .iter()
                .map(|option| option.id.as_str())
                .collect::<Vec<_>>(),
            [
                format!("enemy:engage:{id}:1"),
                format!("enemy:engage:{id}:2")
            ]
        );
        for option in owned {
            assert_eq!(option.owner, OptionOwner::Group { arbiter: Tony });
        }
    }
}
