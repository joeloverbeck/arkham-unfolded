//! The live decision controls, after ordinary reading stops, expose prevention
//! before Apply (#233). Clicking a control must execute the existing rules choice.
use super::*;
use crate::gathering::{HarmFixture, HostileFixture};

fn choose(session: &mut Session, id: &str) {
    session.choose(session.revision(), id).unwrap();
}

fn reach_live_controls(app: &mut App) {
    for _ in 0..8 {
        settle_automatic_resolution(app);
        if !app.world().resource::<Table>().modal() {
            return;
        }
        let entity = button_with_label(app, "Continue")
            .expect("A prepared consequence may reopen an ordinary reading stop");
        app.world_mut().trigger(Activate { entity });
        app.update();
    }
    panic!("Reading stops did not reach the live decision");
}

fn direct_button(app: &mut App, id: &str) -> Entity {
    assert!(!app.world().resource::<Table>().context.is_open());
    let world = app.world_mut();
    let buttons: Vec<_> = world
        .query_filtered::<(Entity, &ChoiceControl), With<Button>>()
        .iter(world)
        .filter(|(_, choice)| choice.0 == id)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(buttons.len(), 1, "Exactly one direct control for {id}");
    buttons[0]
}

#[test]
fn deny_is_directly_playable_beside_apply_and_keeps_the_other_attack_aspect() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyAttack));
    choose(&mut session, "harm:damage:investigator");
    choose(&mut session, "harm:horror:investigator");
    let before = session.observe();
    let deny = before
        .choices
        .iter()
        .find(|c| c.id.starts_with("deny:") && c.id.ends_with(":damage"))
        .unwrap()
        .id
        .clone();
    let prior_horror = before.play.as_ref().unwrap().players[2].horror;
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("deny.json")),
        false,
    );
    reach_live_controls(&mut app);
    direct_button(&mut app, "harm:confirm");
    let card = before.play.as_ref().unwrap().players[2]
        .hand
        .iter()
        .find(|card| card.code == "05032")
        .unwrap()
        .id;
    click_source(&mut app, controls::Source::Card(card));
    assert!(
        choice_button(&mut app, "Play Deny Existence — ignore 1 damage").is_some(),
        "The contextual action remains usable too"
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    app.update();
    let entity = direct_button(&mut app, &deny);
    assert!(!app.world().resource::<Table>().context.is_open());
    app.world_mut().trigger(Activate { entity });
    app.update();
    let after = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    assert_eq!(after.players[2].damage, 0);
    assert_eq!(after.players[2].horror, prior_horror + 1);
    assert!(
        after.players[2]
            .discard
            .iter()
            .any(|card| card.code == "05032")
    );
}

#[test]
fn idol_is_direct_and_keeps_its_hand_cost_and_aspect_selection() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::IdolAttack));
    let before = session.observe().play.unwrap().players[1].clone();
    let discard = before.hand.iter().find(|card| !card.weakness).unwrap().id;
    choose(&mut session, "harm:damage:investigator");
    let idol = session
        .observe()
        .choices
        .iter()
        .find(|c| c.id.starts_with("idol:"))
        .unwrap()
        .id
        .clone();
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("idol.json")),
        false,
    );
    reach_live_controls(&mut app);
    direct_button(&mut app, "harm:confirm");
    let entity = direct_button(&mut app, &idol);
    app.world_mut().trigger(Activate { entity });
    app.update();
    activate_choice(&mut app, &format!("idol-select:{discard}"));
    activate_choice(&mut app, "idol-pay");
    let choices = app.world().resource::<Table>().session.observe().choices;
    for aspect in ["damage", "horror"] {
        assert!(
            choices
                .iter()
                .any(|c| c.id == format!("idol-prevent:{aspect}:investigator")),
            "One discarded card leaves a genuine choice of which aspect to cancel"
        );
    }
    activate_choice(&mut app, "idol-prevent:damage:investigator");
    let after = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    assert_eq!(after.players[1].damage, before.damage);
    assert_eq!(after.players[1].horror, before.horror + 1);
    assert!(
        after.players[1]
            .discard
            .iter()
            .any(|card| card.id == discard)
    );
    assert!(
        !after.players[1]
            .assets
            .iter()
            .find(|asset| asset.card.code == "09102")
            .unwrap()
            .ready
    );
}

#[test]
fn agenda_discard_exposes_deny_and_proceed_before_any_random_discard() {
    let mut session = Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyAgenda));
    choose(&mut session, "agenda:random-discard");
    let before = session.observe();
    let deny = before
        .choices
        .iter()
        .find(|c| c.id.starts_with("agenda-discard:deny:"))
        .unwrap()
        .id
        .clone();
    let card = before.play.as_ref().unwrap().players[2]
        .hand
        .iter()
        .find(|card| card.code == "05032")
        .unwrap()
        .id;
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("agenda.json")),
        false,
    );
    reach_live_controls(&mut app);
    direct_button(&mut app, "agenda-discard:resolve");
    let entity = direct_button(&mut app, &deny);
    assert_eq!(
        app.world().resource::<Table>().session.observe(),
        before,
        "Reading and showing the options discards nothing"
    );
    app.world_mut().trigger(Activate { entity });
    app.update();
    let after = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    let expected: Vec<_> = before.play.as_ref().unwrap().players[2]
        .hand
        .iter()
        .filter(|c| c.id != card)
        .cloned()
        .collect();
    assert_eq!(after.players[2].hand, expected);
    for index in [0, 1] {
        assert_eq!(
            after.players[index].hand,
            before.play.as_ref().unwrap().players[index].hand
        );
    }
}

#[test]
fn player_card_horror_never_offers_deny_or_an_empty_prevention_pause() {
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenySpellHorror));
    let prior_horror = session.observe().play.unwrap().players[2].horror;
    choose(&mut session, "harm:horror:investigator");
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("player-horror.json")),
        false,
    );
    reach_live_controls(&mut app);
    let view = app.world().resource::<Table>().session.observe();
    assert!(view.play.as_ref().unwrap().pending.harm.is_none());
    assert_eq!(
        view.play.as_ref().unwrap().players[2].horror,
        prior_horror + 1
    );
    assert!(
        view.play.as_ref().unwrap().players[2]
            .hand
            .iter()
            .any(|c| c.code == "05032")
    );
    let world = app.world_mut();
    assert!(
        world
            .query::<&ChoiceControl>()
            .iter(world)
            .all(|c| !c.0.starts_with("deny:") && c.0 != "harm:confirm")
    );
}

fn press(app: &mut App, label: &str) {
    let entity = button_with_label(app, label).unwrap_or_else(|| panic!("Missing {label}"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

#[test]
fn prevention_controls_belong_to_the_human_only_when_they_control_the_investigator() {
    use Investigator::{Jacqueline, Patrice, Tony};
    for assigned in [vec![Tony, Patrice], vec![Jacqueline]] {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Harm(HarmFixture::DenyTablet));
        choose(&mut session, "harm:damage:investigator");
        let deny = session
            .observe()
            .choices
            .iter()
            .find(|c| c.id.starts_with("deny:"))
            .unwrap()
            .id
            .clone();
        let directory = tempfile::tempdir().unwrap();
        let mut app = host_with_table(
            Table::new(session, directory.path().join("partners.json")),
            false,
        );
        app.insert_resource(partners::PartnerSocket(
            directory.path().join("partners.sock"),
        ));
        reach_live_controls(&mut app);
        press(&mut app, "Menu");
        press(&mut app, "Agent partners");
        for who in &assigned {
            press(&mut app, &format!("{}: you play", who.name()));
        }
        press(&mut app, "Enable agent partners");
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::Close);
        app.update();
        if assigned.contains(&Jacqueline) {
            let world = app.world_mut();
            assert!(
                world
                    .query::<&ChoiceControl>()
                    .iter(world)
                    .all(|c| c.0 != deny && c.0 != "harm:confirm")
            );
            let revision = app.world().resource::<Table>().session.revision();
            let epoch = app.world().resource::<Table>().epoch;
            app.world_mut()
                .resource_mut::<Actions>()
                .0
                .push(Action::Choose(epoch, revision, deny.clone()));
            app.update();
            assert_eq!(
                app.world().resource::<Table>().session.revision(),
                revision,
                "Human cannot submit the partner's prevention"
            );
            press(&mut app, "Take over Jacqueline Fine");
        }
        direct_button(&mut app, "harm:confirm");
        let entity = direct_button(&mut app, &deny);
        app.world_mut().trigger(Activate { entity });
        app.update();
        assert_eq!(
            app.world()
                .resource::<Table>()
                .session
                .observe()
                .play
                .unwrap()
                .players[2]
                .damage,
            0
        );
    }
}

#[test]
fn harm_labels_only_name_the_present_types_and_apply_the_displayed_amounts() {
    for (case, damage, horror, amounts, apply) in [
        (HarmFixture::DenyTablet, 1, 0, "1 damage", "Apply 1 damage"),
        (HarmFixture::DenyCultist, 0, 1, "1 horror", "Apply 1 horror"),
        (
            HarmFixture::DenyAttack,
            1,
            1,
            "1 damage and 1 horror",
            "Apply 1 damage and 1 horror simultaneously",
        ),
    ] {
        let mut session = Session::with_hostile_fixture(HostileFixture::Harm(case));
        let before = session.observe().play.unwrap().players[2].clone();
        if damage > 0 {
            choose(&mut session, "harm:damage:investigator");
        }
        if horror > 0 {
            choose(&mut session, "harm:horror:investigator");
        }
        let directory = tempfile::tempdir().unwrap();
        let mut app = host_with_table(
            Table::new(session, directory.path().join("labels.json")),
            false,
        );
        reach_live_controls(&mut app);
        let shown = rendered_text(&mut app);
        assert!(
            shown.contains(&format!("Pending {amounts} on Jacqueline Fine")),
            "{shown}"
        );
        for (amount, aspect) in [(damage, "damage"), (horror, "horror")] {
            if amount > 0 {
                assert!(
                    shown.contains(&format!(
                        "Assigned to Jacqueline Fine: {amount} {aspect} (not yet applied)"
                    )),
                    "{shown}"
                );
            }
        }
        assert!(
            !shown.contains("0 damage") && !shown.contains("0 horror"),
            "{shown}"
        );
        let entity =
            choice_button(&mut app, apply).expect("Apply names precisely the remaining harm");
        app.world_mut().trigger(Activate { entity });
        app.update();
        let after = app
            .world()
            .resource::<Table>()
            .session
            .observe()
            .play
            .unwrap();
        assert_eq!(after.players[2].damage, before.damage + damage);
        assert_eq!(after.players[2].horror, before.horror + horror);
    }
}
