//! Single-option resolution. A step with exactly one legal outcome is not a
//! genuine decision: the rules session performs it itself, records it in the
//! action history and never makes it an Undo destination. The table, the agent
//! seat and the headless driver therefore meet the same decisions; none of them
//! keeps its own list of automatically chosen steps.

use crate::Choice;

/// The history entry for a step the session performed itself.
pub(super) fn history_entry(label: &str) -> String {
    format!("Automatic: {label}")
}

/// The lone legal choice the session performs itself, if the step has one.
/// A window with a real opportunity always lists it beside its pass, so only a
/// window with nothing to do closes this way.
pub(super) fn forced(choices: &[Choice]) -> Option<&Choice> {
    let [only] = choices else {
        return None;
    };
    (!pause(&only.id)).then_some(only)
}

/// Pauses wait for the player even when they offer one choice. A compound
/// choice, whose parts are selected and then explicitly confirmed, is decided
/// at its confirmation: neither a lone part nor the confirmation is performed
/// for the player. The chaos-token reveal confirms the declared reveal plan and
/// its token selections and cancellations, so the player draws from the bag. A
/// group clue payment is an allocation the player commits once, even when only
/// one split exists. Story and result reading stops are presentation pauses with no rules choice.
fn pause(id: &str) -> bool {
    id == "test:reveal"
        || id.starts_with("tokens:")
        || id.starts_with("commit:")
        || id.starts_with("act:pay:")
}

/// Seam 1: the rules session. Setup edits only place a witness at the named
/// position; every expectation is observed through offered choices, history and
/// play observations. Sources: The Barrier's Objective (01109), Idol of Xanatos
/// (09102), Agenda 1 (01105b), Agenda 2 (01106b) and Rules Reference p.7.
#[cfg(test)]
mod tests {
    use crate::gathering::phases::PhaseDecision;
    use crate::gathering::runtime::{AssetState, Frame};
    use crate::gathering::{HarmFixture, HostileFixture, Investigator, Session};

    fn offered(session: &Session) -> Vec<String> {
        session
            .observe()
            .choices
            .into_iter()
            .map(|choice| choice.id)
            .filter(|id| id != "undo")
            .collect()
    }

    fn choose(session: &mut Session, id: &str) {
        let view = session.observe();
        assert!(
            view.choices.iter().any(|choice| choice.id == id),
            "Expected {id}; offered {:?}",
            offered(session)
        );
        session.choose(view.revision, id).unwrap();
    }

    /// The Barrier's group clue payment, offered as one listed choice, paid with the
    /// Hallway's nine clues as the direct choice naming that allocation.
    fn pay_hallway(session: &mut Session) {
        let view = session.observe();
        assert!(
            view.choices
                .iter()
                .any(|choice| choice.id == "act:pay:<contributions>"),
            "The group clue payment is offered: {:?}",
            view.choices
        );
        session.choose(view.revision, "act:pay:3:3:3").unwrap();
    }

    fn performed(session: &Session, label: &str) -> bool {
        session
            .observe()
            .history
            .contains(&format!("Automatic: {label}"))
    }

    /// Reach the witness's pending step through a genuine decision: a player
    /// window opened above it, which the player passes.
    fn arrive(session: &mut Session) {
        session
            .runtime
            .as_mut()
            .unwrap()
            .frames
            .push(Frame::Phase(PhaseDecision::PlayerWindow { next: None }));
        choose(session, "phase-window:pass");
    }

    const BARRIER_UNPAYABLE: &str =
        "The Barrier does not advance: the investigators in the Hallway cannot pay its clues";

    #[test]
    fn the_barrier_round_end_step_is_offered_only_when_the_hallway_can_pay() {
        // Nine clues in the Hallway: 3 per investigator who started.
        let mut payable = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
        arrive(&mut payable);
        assert_eq!(
            offered(&payable),
            ["round-end:decline-act", "act:pay:<contributions>"],
            "When payable, both advancing and declining remain"
        );
        choose(&mut payable, "round-end:decline-act");

        // The group still holds nine clues, but Jacqueline's three are in the
        // Cellar, so the Hallway holds six.
        let mut unpayable = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
        unpayable.runtime.as_mut().unwrap().players[Investigator::Jacqueline.index()].location =
            super::super::scenario::CELLAR.into();
        arrive(&mut unpayable);
        assert!(performed(&unpayable, BARRIER_UNPAYABLE));
        assert!(
            !unpayable
                .observe()
                .history
                .iter()
                .any(|entry| entry.contains("round-end:")),
            "No Barrier prompt was offered or recorded as a decision"
        );
        // The forced Ghoul doom follows the same way.
        assert!(performed(
            &unpayable,
            "Resolve the agenda's forced Ghoul doom"
        ));

        // The round ends in the same state as declining.
        let declined = payable.observe().play.unwrap();
        let skipped = unpayable.observe().play.unwrap();
        assert_eq!(
            (
                skipped.act,
                skipped.agenda,
                skipped.agenda_doom,
                skipped.round
            ),
            (
                declined.act,
                declined.agenda,
                declined.agenda_doom,
                declined.round
            )
        );
        assert_eq!(
            skipped
                .players
                .iter()
                .map(|player| player.clues)
                .collect::<Vec<_>>(),
            declined
                .players
                .iter()
                .map(|player| player.clues)
                .collect::<Vec<_>>()
        );
        assert_eq!(skipped.scenario.outcome, declined.scenario.outcome);
        assert_eq!(skipped.scenario.enemies, declined.scenario.enemies);
    }

    fn idol_witness(damage: u8, horror: u8) -> (Session, u16) {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::IdolAttack));
        let game = session.runtime.as_mut().unwrap();
        let Some(Frame::Harm(harm)) = game.frames.last_mut() else {
            panic!("The Idol witness awaits harm assignment");
        };
        (harm.damage, harm.horror) = (damage, horror);
        let idol = game.players[Investigator::Patrice.index()]
            .assets
            .iter()
            .find(|asset| asset.card.code == "09102")
            .unwrap()
            .card
            .id;
        (session, idol)
    }

    /// Use the Idol, discarding `count` non-weakness cards.
    fn pay_idol(session: &mut Session, idol: u16, count: usize) {
        choose(session, &format!("idol:{idol}"));
        let cards: Vec<_> = session.observe().play.unwrap().players[Investigator::Patrice.index()]
            .hand
            .iter()
            .filter(|card| !card.weakness && card.code != "06017")
            .take(count)
            .map(|card| card.id)
            .collect();
        for card in cards {
            choose(session, &format!("idol-select:{card}"));
        }
        choose(session, "idol-pay");
    }

    #[test]
    fn idol_cancels_by_itself_only_for_one_target_and_aspect_or_full_coverage() {
        // A single target and aspect: two damage on Patrice, one card paid.
        let (mut single, idol) = idol_witness(2, 0);
        let before = single.observe().play.unwrap().players[Investigator::Patrice.index()].damage;
        choose(&mut single, "harm:damage:investigator");
        pay_idol(&mut single, idol, 1);
        assert!(performed(
            &single,
            "Cancel 1 assigned damage on Patrice Hathaway"
        ));
        assert!(
            !offered(&single)
                .iter()
                .any(|id| id.starts_with("idol-prevent:"))
        );
        assert_eq!(
            single.observe().play.unwrap().players[Investigator::Patrice.index()].damage,
            before + 1
        );

        // Several targets: Peter Sylvestre and Patrice each hold one assigned
        // horror, one card paid. The allocation remains Patrice's choice.
        let (mut several, idol) = idol_witness(0, 2);
        let game = several.runtime.as_mut().unwrap();
        let patrice = &mut game.players[Investigator::Patrice.index()];
        let at = patrice
            .deck
            .iter()
            .position(|card| card.code == "02033")
            .expect("Peter Sylvestre is in Patrice's deck");
        let peter = patrice.deck.remove(at);
        let peter_id = peter.id;
        patrice.assets.push(AssetState {
            card: peter,
            ready: true,
            uses: 0,
            damage: 0,
            horror: 0,
        });
        choose(&mut several, &format!("harm:horror:{peter_id}"));
        choose(&mut several, "harm:horror:investigator");
        pay_idol(&mut several, idol, 1);
        let mut targets = offered(&several);
        targets.sort();
        assert_eq!(
            targets,
            [
                format!("idol-prevent:horror:{peter_id}"),
                "idol-prevent:horror:investigator".into()
            ]
        );

        // Several aspects on one target stay a choice too.
        let (mut aspects, idol) = idol_witness(1, 1);
        choose(&mut aspects, "harm:damage:investigator");
        pay_idol(&mut aspects, idol, 1);
        let mut targets = offered(&aspects);
        targets.sort();
        assert_eq!(
            targets,
            [
                "idol-prevent:damage:investigator",
                "idol-prevent:horror:investigator"
            ]
        );

        // Full coverage: two cards paid for one damage and one horror.
        let (mut covered, idol) = idol_witness(1, 1);
        let before = covered.observe().play.unwrap().players[Investigator::Patrice.index()].clone();
        choose(&mut covered, "harm:damage:investigator");
        pay_idol(&mut covered, idol, 2);
        assert!(performed(
            &covered,
            "Cancel all assigned harm (1 damage and 1 horror)"
        ));
        let after = covered.observe().play.unwrap().players[Investigator::Patrice.index()].clone();
        assert_eq!((after.damage, after.horror), (before.damage, before.horror));
        covered.validate().unwrap();
    }

    #[test]
    fn agenda_one_forced_horror_resolves_by_itself_only_when_nobody_can_discard() {
        let mut choosing = Session::with_hostile_fixture(HostileFixture::GatheringDoomThreshold);
        choose(&mut choosing, "mythos:begin");
        let mut options = offered(&choosing);
        options.sort();
        assert_eq!(options, ["agenda:lead-horror", "agenda:random-discard"]);

        let mut forced = Session::with_hostile_fixture(HostileFixture::GatheringDoomThreshold);
        for player in &mut forced.runtime.as_mut().unwrap().players {
            let hand = std::mem::take(&mut player.hand);
            player.discard.extend(hand);
        }
        choose(&mut forced, "mythos:begin");
        assert!(performed(&forced, "The lead investigator takes 2 horror"));
        assert!(!offered(&forced).iter().any(|id| id.starts_with("agenda:")));
        let history = forced.observe().history;
        let horror = history
            .iter()
            .position(|entry| entry == "Automatic: The lead investigator takes 2 horror")
            .unwrap();
        assert!(
            history[horror..]
                .iter()
                .any(|entry| entry.contains("2 horror")),
            "The lead's horror follows the agenda's forced outcome"
        );
    }

    #[test]
    fn agenda_search_and_round_end_finish_resolve_by_themselves() {
        let mut search = Session::with_hostile_fixture(HostileFixture::GatheringAgendaTwoNoGhoul);
        arrive(&mut search);
        assert!(performed(
            &search,
            "Resolve Rise of the Ghouls: search the encounter deck"
        ));
        assert_eq!(search.observe().play.unwrap().agenda, 3);

        // With Agenda 2 and no Dissonant Voices, The Barrier's advancement
        // leaves no AT ability, so finishing the round is its only outcome.
        let mut finish = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
        let game = finish.runtime.as_mut().unwrap();
        (game.agenda, game.agenda_doom) = (2, 0);
        let round = game.round;
        pay_hallway(&mut finish);
        let bounty = offered(&finish)
            .into_iter()
            .find(|id| id.starts_with("bounty:") && id.ends_with(":0"))
            .expect("Tony may place a bounty on the entering Priest");
        choose(&mut finish, &bounty);
        assert!(performed(
            &finish,
            "Finish the round after the act and its entry reactions"
        ));
        assert_eq!(finish.observe().play.unwrap().round, round + 1);
    }
}
