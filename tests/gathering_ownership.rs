//! Seam: the rules session's ownership of the pending decision's options.
//! Expectations come from the cited sources, not from the implementation:
//! Rules Reference (AHC60) pp. 3, 7, 12, 13, 14, 15, 17, 24, 25 and 26, FAQ 2.5
//! §1.5, card text, and the play contract's house practice for a new lead.

use arkham_unfolded_experiment::gathering::{
    HarmFixture, HostileFixture, Investigator, LifecycleFixture, OptionOwner, Session, continuity,
};

use Investigator::{Jacqueline, Patrice, Tony};

fn choose(session: &mut Session, id: &str) {
    session
        .choose(session.observe().revision, id)
        .unwrap_or_else(|error| panic!("{id}: {error}"));
}

/// The owner of every listed option, in listed order, without Undo.
fn owners(session: &Session) -> Vec<(String, OptionOwner)> {
    session
        .ownership()
        .into_iter()
        .map(|option| (option.id, option.owner))
        .collect()
}

fn owner(session: &Session, id: &str) -> OptionOwner {
    owners(session)
        .into_iter()
        .find_map(|(listed, owner)| (listed == id).then_some(owner))
        .unwrap_or_else(|| panic!("{id} is not listed: {:?}", owners(session)))
}

#[test]
fn every_listed_option_is_owned_and_undo_is_not_an_option_of_the_pending_decision() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:tony");
    let listed: Vec<_> = session
        .observe()
        .choices
        .into_iter()
        .map(|choice| choice.id)
        .filter(|id| id != "undo")
        .collect();
    let owned: Vec<_> = owners(&session).into_iter().map(|(id, _)| id).collect();
    assert_eq!(owned, listed);
}

/// RR p.14 "Lead Investigator": the investigators choose the lead at the
/// beginning of the scenario. The play contract makes it the table's setup
/// choice in the window, owned by no investigator.
#[test]
fn the_setup_choice_of_the_lead_belongs_to_the_table() {
    let session = Session::with_seed(89);
    let owned = owners(&session);
    assert_eq!(
        owned,
        ["lead:tony", "lead:patrice", "lead:jacqueline"]
            .map(|id| (id.to_owned(), OptionOwner::Table))
            .to_vec()
    );
}

/// RR p.15 "Mulligan": the player who drew a starting hand declares its
/// mulligan. RR p.13 "In Player Order": the lead first, then clockwise in the
/// fixed order Tony, Patrice, Jacqueline.
#[test]
fn each_mulligan_belongs_to_its_investigator_in_player_order_from_the_lead() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:patrice");
    for expected in [Patrice, Jacqueline, Tony] {
        let owned = owners(&session);
        assert!(!owned.is_empty());
        assert!(
            owned
                .iter()
                .all(|(_, owner)| *owner == OptionOwner::Investigator(expected)),
            "{expected:?}: {owned:?}"
        );
        assert_eq!(
            owner(&session, "confirm-mulligan"),
            OptionOwner::Investigator(expected)
        );
        choose(&mut session, "confirm-mulligan");
    }
}

fn reach(session: &mut Session, expected: &str) {
    for _ in 0..12 {
        let choices = session.observe().choices;
        if choices.iter().any(|choice| choice.id == expected) {
            return;
        }
        let id = [
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| choices.iter().any(|choice| choice.id == *id))
        .unwrap_or_else(|| panic!("Expected progression toward {expected}, got {choices:?}"));
        choose(session, id);
    }
    panic!("Did not reach {expected}");
}

fn lead(session: &Session) -> Investigator {
    session.observe().lead.expect("A chosen lead")
}

fn all_owned_by(session: &Session, expected: &OptionOwner) {
    let owned = owners(session);
    assert!(!owned.is_empty());
    for (id, owner) in owned {
        assert_eq!(&owner, expected, "{id}");
    }
}

/// RR p.24 2.2: the investigators choose among themselves who takes the next
/// turn; the play contract makes the lead's player the final arbiter of that
/// group choice (house practice). RR p.24 2.2.1: the active investigator
/// takes their own actions and may end their turn.
#[test]
fn the_lead_arbitrates_the_next_turn_and_the_active_investigator_owns_their_actions() {
    let mut session = Session::with_seed(89);
    choose(&mut session, "lead:jacqueline");
    for _ in 0..3 {
        choose(&mut session, "confirm-mulligan");
    }
    assert_eq!(
        owners(&session),
        ["turn:tony", "turn:patrice", "turn:jacqueline"]
            .map(|id| (
                id.to_owned(),
                OptionOwner::Group {
                    arbiter: Jacqueline
                }
            ))
            .to_vec()
    );
    choose(&mut session, "turn:patrice");
    let owned = owners(&session);
    assert!(owned.iter().any(|(id, _)| id == "turn:end"));
    let actions: Vec<_> = owned
        .iter()
        .filter(|(id, _)| id == "turn:end" || id.starts_with("action:"))
        .collect();
    assert!(actions.len() > 1);
    for (id, owner) in actions {
        assert_eq!(*owner, OptionOwner::Investigator(Patrice), "{id}");
    }
}

/// RR p.7 "Dealing Damage/Horror": the investigator dealt harm assigns it to
/// assets they control, and prevention is resolved for that dealing. Only the
/// dealt investigator's own Deny Existence is offered, and it is theirs.
#[test]
fn harm_assignment_and_its_prevention_belong_to_the_dealt_investigator() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyAttack));
    assert_ne!(
        lead(&session),
        Jacqueline,
        "The witness separates lead and dealt investigator"
    );
    all_owned_by(&session, &OptionOwner::Investigator(Jacqueline));
    let play = session.observe().play.unwrap();
    let olive = play.players[2]
        .assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .id;
    choose(&mut session, &format!("harm:damage:{olive}"));
    choose(&mut session, "harm:horror:investigator");
    assert!(
        owners(&session)
            .iter()
            .any(|(id, _)| id.starts_with("deny:"))
    );
    all_owned_by(&session, &OptionOwner::Investigator(Jacqueline));
}

/// RR p.3 "Triggered Abilities" and p.15 "May": Lita Chantler's controller
/// triggers her reaction, though another investigator is attacking.
#[test]
fn lita_chantlers_reaction_belongs_to_her_controller_not_the_attacker() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringLitaDamage);
    choose(&mut session, "action:ordinary:fight:400");
    reach(&mut session, "lita:add-damage");
    let play = session.observe().play.unwrap();
    assert_eq!(play.test.as_ref().unwrap().tester, Tony);
    let controller = play
        .players
        .iter()
        .find(|player| player.assets.iter().any(|asset| asset.card.code == "01117"))
        .expect("Lita is in play under a player's control")
        .investigator;
    assert_eq!(controller, Patrice);
    for id in ["lita:add-damage", "lita:decline-damage"] {
        assert_eq!(owner(&session, id), OptionOwner::Investigator(Patrice));
    }
}

/// Act 3b (01110b): the lead investigator must decide whether to burn the
/// house; RR p.14 makes the lead the final arbiter of such choices. It is a
/// group choice, not the decision of the investigator who defeated the Priest.
#[test]
fn the_lead_decides_act_three_b() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    choose(&mut session, "action:ordinary:fight:400");
    reach(&mut session, "resolution:burn");
    all_owned_by(
        &session,
        &OptionOwner::Group {
            arbiter: lead(&session),
        },
    );
}

/// RR p.17 "Priority of Simultaneous Resolution": the lead determines the
/// order of forced abilities that resolve at the same time (the agenda's Ghoul
/// doom and Dissonant Voices at round end).
#[test]
fn the_lead_orders_simultaneous_round_end_effects() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEndVoices);
    choose(&mut session, "round-end:decline-act");
    let owned = owners(&session);
    assert!(
        owned
            .iter()
            .any(|(id, _)| id.starts_with("round-end:voices:"))
    );
    all_owned_by(
        &session,
        &OptionOwner::Group {
            arbiter: lead(&session),
        },
    );
}

/// RR p.17 (simultaneous forced effects) orders the Hunters; RR p.12
/// "Hunter": among equally near investigators, the lead chooses where the
/// enemy moves.
#[test]
fn the_lead_orders_hunters_and_breaks_their_ties() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringHunters);
    let arbiter = OptionOwner::Group {
        arbiter: lead(&session),
    };
    assert_eq!(owner(&session, "enemy:hunter:400"), arbiter);
    choose(&mut session, "enemy:hunter:400");
    let owned = owners(&session);
    assert!(owned.iter().any(|(id, _)| id.starts_with("hunter:move:")));
    all_owned_by(&session, &arbiter);
}

/// RR p.14: when the lead is eliminated the remaining players choose a new
/// lead. The play contract's house practice: the next investigator in player
/// order (RR p.13, clockwise Tony, Patrice, Jacqueline) arbitrates it.
#[test]
fn the_next_investigator_in_player_order_chooses_the_new_lead() {
    // Jacqueline leads and is defeated: Tony follows her.
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::EndTurnDefeat));
    assert_eq!(lead(&session), Jacqueline);
    choose(&mut session, "action:ordinary:resource");
    choose(&mut session, "harm:damage:investigator");
    choose(&mut session, "harm:damage:investigator");
    reach(&mut session, "elimination:lead:tony");
    all_owned_by(&session, &OptionOwner::Group { arbiter: Tony });

    // Patrice leads and is defeated: Jacqueline follows her.
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::PromiseDefeat));
    assert_eq!(lead(&session), Patrice);
    let promise = session.observe().play.unwrap().players[1]
        .hand
        .iter()
        .find(|card| card.code == "07032")
        .unwrap()
        .id;
    if owners(&session).iter().any(|(id, _)| id == "window:pass") {
        choose(&mut session, "window:pass");
    }
    choose(&mut session, &format!("commit:{promise}"));
    choose(&mut session, "commit:confirm");
    reach(&mut session, "elimination:lead:jacqueline");
    all_owned_by(
        &session,
        &OptionOwner::Group {
            arbiter: Jacqueline,
        },
    );
}

/// Agenda 1b (01105b): the lead investigator must decide between taking 2
/// horror and each investigator discarding a random card.
#[test]
fn the_lead_decides_agenda_one_b() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringDoomThreshold);
    choose(&mut session, "mythos:begin");
    assert!(
        owners(&session)
            .iter()
            .any(|(id, _)| id == "agenda:lead-horror")
    );
    all_owned_by(
        &session,
        &OptionOwner::Group {
            arbiter: lead(&session),
        },
    );
}

/// Ownership is derived each time and never saved: a saved and reloaded
/// session reports the same owners, and the save holds no ownership.
#[test]
fn ownership_survives_a_save_without_being_saved() {
    let mut session = Session::with_seed(89);
    for id in [
        "lead:patrice",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
    ] {
        choose(&mut session, id);
    }
    let before = session.ownership();
    assert_eq!(before[0].owner, OptionOwner::Group { arbiter: Patrice });
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ownership.json");
    continuity::save_session(&path, &session).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    for key in ["\"arbiter\"", "\"participants\"", "\"ownership\""] {
        assert!(!saved.contains(key), "{key}");
    }
    assert_eq!(continuity::load_session(&path).unwrap().ownership(), before);
}

/// RR p.7: a defeated asset goes to its owner's discard pile, so the owner
/// orders the assets that leave play together.
#[test]
fn the_owner_orders_their_simultaneously_defeated_assets() {
    let mut session = Session::with_hostile_fixture(HostileFixture::SimultaneousAssetDefeat);
    assert_ne!(
        lead(&session),
        Jacqueline,
        "The witness separates lead and owner"
    );
    let play = session.observe().play.unwrap();
    let asset = |code: &str| {
        play.players[2]
            .assets
            .iter()
            .find(|asset| asset.card.code == code)
            .unwrap()
            .card
            .id
    };
    for id in [
        "action:ordinary:resource".to_owned(),
        format!("harm:damage:{}", asset("04197")),
        format!("harm:damage:{}", asset("60412")),
        "harm:horror:investigator".to_owned(),
        "harm:horror:investigator".to_owned(),
    ] {
        choose(&mut session, &id);
    }
    assert!(
        owners(&session)
            .iter()
            .any(|(id, _)| id.starts_with("discard-card:"))
    );
    all_owned_by(&session, &OptionOwner::Investigator(Jacqueline));
}

/// Tony's Quarry (06012) spawns at the location farthest from Tony. FAQ 2.5
/// §1.5: when investigators must make a forced choice with several valid
/// options, the lead investigator decides.
#[test]
fn the_lead_breaks_a_tie_for_a_forced_spawn() {
    let mut session = Session::with_hostile_fixture(HostileFixture::GatheringQuarryPlacement);
    choose(&mut session, "revelation:resolve");
    let owned = owners(&session);
    assert!(
        owned
            .iter()
            .filter(|(id, _)| id.starts_with("quarry:"))
            .count()
            > 1
    );
    all_owned_by(
        &session,
        &OptionOwner::Group {
            arbiter: lead(&session),
        },
    );
}

/// Along played paths through ordinary preparation and a range of witnesses,
/// every listed option reports an owner. Options are taken in rotation; a
/// listed placeholder is skipped because it needs a named number or split.
#[test]
fn every_option_along_played_paths_is_owned() {
    let starts = [
        Session::with_seed(7),
        Session::with_seed(89),
        Session::with_hostile_fixture(HostileFixture::GatheringFromSetup),
        Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat),
        Session::with_hostile_fixture(HostileFixture::GatheringHunters),
        Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision),
        Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::IdolAttack)),
    ];
    for (path, mut session) in starts.into_iter().enumerate() {
        for step in 0..300 {
            let listed: Vec<_> = session
                .observe()
                .choices
                .into_iter()
                .map(|choice| choice.id)
                .filter(|id| id != "undo")
                .collect();
            let owned: Vec<_> = session
                .ownership()
                .into_iter()
                .map(|option| option.id)
                .collect();
            assert_eq!(owned, listed, "path {path}, step {step}");
            let playable: Vec<_> = listed.iter().filter(|id| !id.ends_with('>')).collect();
            if playable.is_empty() {
                break;
            }
            let id = playable[(step * 7 + path) % playable.len()].clone();
            choose(&mut session, &id);
        }
    }
}
