//! Option labels state each option's effective cost once, as currently
//! modified, followed by its effect (spec #163, "Option labels"). Expected
//! labels come from the accepted wording, not from the label composition.

use arkham_unfolded_experiment::gathering::{
    AssetFixture, HostileFixture, LifecycleFixture, PlayerEventFixture, SearchFixture,
    SelectedWitness, Session, SpellEventFixture, TonyEffectFixture,
};

fn choose(session: &mut Session, id: &str) {
    session.choose(session.observe().revision, id).unwrap();
}

fn continue_test(session: &mut Session) {
    let choices = session.observe().choices;
    let id = [
        "test-card:pass",
        "window:pass",
        "commit:confirm",
        "test:reveal",
        "lucky:decline",
        "success:pass",
        "after-test:pass",
    ]
    .into_iter()
    .find(|id| choices.iter().any(|choice| choice.id == *id))
    .map(str::to_owned)
    .or_else(|| {
        choices
            .iter()
            .find(|choice| choice.id.starts_with("result:"))
            .map(|choice| choice.id.clone())
    })
    .unwrap_or_else(|| panic!("Unhandled test continuation {choices:?}"));
    choose(session, &id);
}

fn finish_current_test(session: &mut Session) {
    for _ in 0..25 {
        if session.observe().play.unwrap().test.is_none() {
            return;
        }
        continue_test(session);
    }
    panic!("Skill test did not finish");
}

fn reach(session: &mut Session, expected: &str) {
    for _ in 0..25 {
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == expected)
        {
            return;
        }
        continue_test(session);
    }
    panic!("Did not reach {expected}");
}

fn label(session: &Session, id: &str) -> String {
    let view = session.observe();
    view.choices
        .iter()
        .find(|choice| choice.id == id)
        .unwrap_or_else(|| {
            panic!(
                "{id} is not offered; offered: {:?}",
                view.choices
                    .iter()
                    .map(|choice| &choice.id)
                    .collect::<Vec<_>>()
            )
        })
        .label
        .clone()
}

fn threat(session: &Session, player: usize, code: &str) -> u16 {
    session.observe().play.unwrap().players[player]
        .threats
        .iter()
        .find(|card| card.code == code)
        .unwrap()
        .id
}

fn hand_card(session: &Session, player: usize, code: &str) -> u16 {
    session.observe().play.unwrap().players[player]
        .hand
        .iter()
        .find(|card| card.code == code)
        .unwrap()
        .id
}

fn enemy(session: &Session, code: &str) -> u16 {
    session
        .observe()
        .play
        .unwrap()
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == code)
        .unwrap()
        .card
        .id
}

fn removal_label(session: &Session, card: u16) -> String {
    let view = session.observe();
    view.choices
        .iter()
        .find(|choice| {
            choice.id.starts_with("action:ordinary:remove-threat:")
                && choice.id.ends_with(&format!(":{card}"))
        })
        .unwrap_or_else(|| panic!("No removal of {card} is offered"))
        .label
        .clone()
}

#[test]
fn two_action_weaknesses_state_their_two_action_cost_once() {
    let injury_game = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    let injury = threat(&injury_game, 0, "02038");
    assert_eq!(
        removal_label(&injury_game, injury),
        "Spend two actions: Discard Internal Injury"
    );

    // Tony shares the Hallway with Patrice, so he may discard her Vision.
    let vision_game = Session::with_hostile_fixture(HostileFixture::GatheringRetaliate);
    let vision = threat(&vision_game, 1, "05041");
    assert_eq!(
        removal_label(&vision_game, vision),
        "Spend two actions: Discard The 13th Vision"
    );

    let chronophobia_game =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::LeadDefeat));
    let chronophobia = threat(&chronophobia_game, 2, "02039");
    assert_eq!(
        removal_label(&chronophobia_game, chronophobia),
        "Spend two actions: Discard Chronophobia"
    );
}

#[test]
fn frozen_in_fear_states_the_first_move_fight_and_evade_at_two_actions() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::GetOverHereFrozen));
    let swarm = enemy(&session, "01159");
    assert_eq!(
        label(&session, "action:ordinary:move:01112"),
        "Spend two actions: Move to Hallway"
    );
    assert_eq!(
        label(&session, &format!("action:ordinary:fight:{swarm}")),
        "Spend two actions: Fight Swarm of Rats"
    );
    assert_eq!(
        label(&session, &format!("action:ordinary:evade:{swarm}")),
        "Spend two actions: Evade Swarm of Rats"
    );
    assert_eq!(
        label(&session, "action:ordinary:resource"),
        "Spend an action: Gain 1 resource",
        "Frozen in Fear taxes only move, fight and evade"
    );

    choose(&mut session, &format!("action:ordinary:evade:{swarm}"));
    finish_current_test(&mut session);
    assert_eq!(session.observe().play.unwrap().actions_remaining[0], 1);
    assert_eq!(
        label(&session, "action:ordinary:move:01112"),
        "Spend an action: Move to Hallway",
        "The surcharge applies only to the first such action each round"
    );
}

#[test]
fn frozen_in_fear_states_event_and_asset_entry_surcharges_in_the_cost() {
    let session = Session::with_hostile_fixture(HostileFixture::SpellEvents(
        SpellEventFixture::UncageRazorFrozen,
    ));
    let razor = hand_card(&session, 2, "06201");
    assert_eq!(
        label(
            &session,
            &format!("action:ordinary:play:tony:{razor}:06201:400:stay")
        ),
        "Spend two actions: Play Spectral Razor for 2 resources; attack Ghoul Priest (copy 400)"
    );

    for (case, cost) in [
        (SpellEventFixture::UncageRazorFrozen, "Spend an action"),
        (SpellEventFixture::UncageRazorStacked, "Spend two actions"),
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::SpellEvents(case));
        let uncage = hand_card(&session, 2, "03033");
        let razor = hand_card(&session, 2, "06201");
        choose(
            &mut session,
            &format!("action:ordinary:play:tony:{uncage}:03033"),
        );
        assert_eq!(
            label(&session, &format!("uncage:event:{razor}:06201:400:engage")),
            format!(
                "{cost}: Play Spectral Razor for 0 resources; attack Ghoul Priest (copy 400), \
                 engage it before the attack"
            ),
            "Uncage waives resources, never Frozen in Fear's additional action per copy"
        );
    }

    let mut cane_game =
        Session::with_hostile_fixture(HostileFixture::Assets(AssetFixture::SwordCaneFrozen));
    let cane = hand_card(&cane_game, 2, "07029");
    choose(&mut cane_game, &format!("action:ordinary:play:{cane}"));
    assert_eq!(
        label(
            &cane_game,
            &format!("entry:{cane}:evade:cane:{cane}:400:willpower")
        ),
        "Spend two actions: Evade Ghoul Priest with Sword Cane using willpower, without \
         exhausting it",
        "Two Frozen copies tax the free entry trigger; Sword Cane's own cost stays waived"
    );
}

#[test]
fn tonys_bounty_action_states_any_additional_action_it_consumes() {
    let priest_game = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    assert_eq!(
        label(&priest_game, "action:tony:fight:400"),
        "Spend Tony's additional bounty action: Fight Ghoul Priest"
    );

    for (fixture, cost) in [
        (
            SearchFixture::TonyEvents,
            "Spend Tony's additional bounty action",
        ),
        (
            SearchFixture::GetOverHereFrozen,
            "Spend Tony's additional bounty action and an action",
        ),
    ] {
        let session = Session::with_hostile_fixture(HostileFixture::Search(fixture));
        let event = hand_card(&session, 0, "60114");
        let minion = enemy(&session, "01160");
        assert_eq!(
            label(
                &session,
                &format!("action:tony:play:tony:{event}:60114:{minion}")
            ),
            format!(
                "{cost}: Play Get over here! for 2 resources to pull, engage and fight Ghoul \
                 Minion"
            )
        );
    }
}

#[test]
fn a_granted_action_states_its_cost_once() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::QuickThinking));
    let view = session.observe().play.unwrap();
    let quick = hand_card(&session, 0, "02229");
    let first = view.scenario.enemies[0].card.id;
    choose(&mut session, &format!("action:ordinary:fight:{first}"));
    reach(&mut session, &format!("commit:{quick}"));
    choose(&mut session, &format!("commit:{quick}"));
    reach(&mut session, "tony:quick-thinking:tony");
    choose(&mut session, "tony:quick-thinking:tony");
    assert_eq!(
        label(&session, "granted:resource"),
        "Spend the granted action: Gain 1 resource"
    );
}

#[test]
fn action_labels_carry_one_cost_prefix_and_no_doubled_colons() {
    let mut views = vec![];
    for fixture in [
        HostileFixture::GatheringPriestDefeat,
        HostileFixture::GatheringRetaliate,
        HostileFixture::GatheringLitaParley,
        HostileFixture::Lifecycle(LifecycleFixture::Resignation),
        HostileFixture::Search(SearchFixture::HornetTargets),
        HostileFixture::Search(SearchFixture::TonyEvents),
        HostileFixture::Search(SearchFixture::GetOverHereFrozen),
        HostileFixture::SpellEvents(SpellEventFixture::UncageRazorFrozen),
    ] {
        views.push(Session::with_hostile_fixture(fixture).observe());
    }
    let mut checked = 0;
    for view in &views {
        for choice in view
            .choices
            .iter()
            .filter(|choice| choice.id.starts_with("action:"))
        {
            checked += 1;
            assert_eq!(
                choice.label.matches(": ").count(),
                1,
                "{} has more than one cost/effect separator",
                choice.label
            );
            let (_, effect) = choice.label.split_once(": ").unwrap();
            for phrase in ["Spend ", "additional action", " actions"] {
                assert!(
                    !effect.contains(phrase),
                    "{}: the effect restates a cost",
                    choice.label
                );
            }
        }
    }
    assert!(checked > 40, "Only {checked} action labels were checked");
}

#[test]
fn effects_use_no_second_colon_and_counts_agree_in_number() {
    let resign =
        Session::with_hostile_fixture(HostileFixture::Lifecycle(LifecycleFixture::Resignation));
    assert_eq!(
        label(&resign, "action:ordinary:resign"),
        "Spend an action: Resign and leave the house"
    );
    let parley = Session::with_hostile_fixture(HostileFixture::GatheringLitaParley);
    assert_eq!(
        label(&parley, "action:ordinary:parley:lita"),
        "Spend an action: Parley with Lita Chantler; test intellect (4) to take control"
    );
    let hornet =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HornetTargets));
    let nest = hand_card(&hornet, 0, "09068");
    assert_eq!(
        label(&hornet, &format!("action:ordinary:play:tony:{nest}")),
        "Spend an action: Play Kicking the Hornet's Nest to search for a non-Elite enemy"
    );

    let events = Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::TonyEvents));
    let intel = hand_card(&events, 0, "05111");
    assert_eq!(
        label(
            &events,
            &format!("action:ordinary:play:tony:{intel}:05111:01113:0:0")
        ),
        "Spend an action: Play Intel Report for 2 resources to discover 1 clue at Attic"
    );
    assert_eq!(
        label(
            &events,
            &format!("action:ordinary:play:tony:{intel}:05111:01113:1:0")
        ),
        "Spend an action: Play Intel Report for 4 resources to discover 2 clues at Attic"
    );

    let mut gregory_game =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::GregoryCigarette));
    let before = gregory_game.observe().play.unwrap();
    let asset = |code: &str| {
        before.players[0]
            .assets
            .iter()
            .find(|asset| asset.card.code == code)
            .unwrap()
            .card
            .id
    };
    let (gregory, gun) = (asset("06162"), asset("06011"));
    let target = before.scenario.enemies[0].card.id;
    choose(
        &mut gregory_game,
        &format!("action:ordinary:fight:{target}:weapon:{gun}"),
    );
    assert_eq!(
        label(&gregory_game, &format!("tony:gregory:{gregory}:1")),
        "Spend 1 resource from Gregory Gry; gain 1 if you succeed by at least 1"
    );
    assert_eq!(
        label(&gregory_game, &format!("tony:gregory:{gregory}:2")),
        "Spend 2 resources from Gregory Gry; gain 2 if you succeed by at least 2"
    );
}

// FAQ v2.5 p.15: "You can investigate a location even if there are no clues on
// it... Investigating a location with no clues might still be useful to
// trigger card abilities." Obscuring Fog (01168): "Attached location gets +2
// shroud. Forced - After attached location is successfully investigated:
// Discard Obscuring Fog." Old Keyring (60507): "If you succeed, remove 1 key".
#[test]
fn investigating_a_clueless_location_stays_offered_and_says_why() {
    let hallway = Session::with_hostile_fixture(HostileFixture::GatheringPriestDefeat);
    assert_eq!(
        label(&hallway, "action:ordinary:investigate"),
        "Spend an action: Investigate this location (shroud 1 · no clues to discover)"
    );

    let study = Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HornetTargets));
    assert_eq!(
        label(&study, "action:ordinary:investigate"),
        "Spend an action: Investigate this location",
        "A location with clues needs no annotation"
    );

    for (case, expected) in [
        (
            SelectedWitness::ObscuringFog,
            "Spend an action: Investigate this location (shroud 4 including Obscuring Fog's +2)",
        ),
        (
            SelectedWitness::ObscuringFogNoClues,
            "Spend an action: Investigate this location (shroud 4 including Obscuring Fog's +2 · \
             no clues to discover · a success still discards Obscuring Fog)",
        ),
    ] {
        let mut fogged = Session::with_hostile_fixture(HostileFixture::SelectedWitness(case));
        choose(&mut fogged, "revelation:resolve");
        assert_eq!(label(&fogged, "action:ordinary:investigate"), expected);
    }

    let mut keyring_game = Session::with_hostile_fixture(HostileFixture::PlayerEvents(
        PlayerEventFixture::KeyringSuccess,
    ));
    let keyring = keyring_game.observe().play.unwrap().players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "60507")
        .unwrap()
        .card
        .id;
    let id = format!("action:ordinary:investigate:spell:{keyring}");
    assert_eq!(
        label(&keyring_game, &id),
        "Spend an action: Investigate with Old Keyring",
        "The Cellar still has clues"
    );
    choose(&mut keyring_game, "action:ordinary:move:01112");
    assert_eq!(
        label(&keyring_game, &id),
        "Spend an action: Investigate with Old Keyring (shroud 1 · no clues to discover · a \
         success still uses a key)"
    );
}
