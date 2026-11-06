use super::*;
use crate::gathering::{HostileFixture, TonyEffectFixture};

fn watch_commitment(fixture: TonyEffectFixture) -> (Session, u16) {
    let mut session = Session::with_hostile_fixture(HostileFixture::Tony(fixture));
    let play = session.observe().play.unwrap();
    let card = play.players[0]
        .hand
        .iter()
        .find(|card| card.code == "03233")
        .unwrap()
        .id;
    let enemy = play.scenario.enemies[0].card.id;
    let view = session.observe();
    session
        .choose(view.revision, &format!("action:ordinary:fight:{enemy}"))
        .unwrap();
    for _ in 0..10 {
        let view = session.observe();
        if view
            .choices
            .iter()
            .any(|c| c.id == format!("commit:{card}"))
        {
            return (session, card);
        }
        let choice = ["window:pass", "test-card:pass"]
            .into_iter()
            .find(|id| view.choices.iter().any(|c| c.id == *id))
            .unwrap();
        session.choose(view.revision, choice).unwrap();
    }
    panic!("Commitment was not reached");
}

#[test]
fn selecting_watch_this_immediately_offers_a_wager_and_requires_an_explicit_choice() {
    let directory = tempfile::tempdir().unwrap();
    let (session, card) = watch_commitment(TonyEffectFixture::WatchSuccess);
    let mut app = host_with_table(
        Table::new(session, directory.path().join("watch.json")),
        true,
    );
    activate_choice(&mut app, &format!("commit:{card}"));
    finish_presentation(&mut app);
    let view = app.world().resource::<Table>().view();
    for amount in 0..=3 {
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id == format!("tony:watch:{card}:{amount}"))
            .unwrap();
        assert!(
            choice_button(&mut app, &choice.label).is_some(),
            "Wager {amount} must appear without reopening the card"
        );
    }
    assert!(!view.choices.iter().any(|c| c.id == "commit:confirm"));
    assert_eq!(view.play.unwrap().players[0].resources, 5);
    activate_choice(&mut app, &format!("tony:watch:{card}:0"));
    assert!(rendered_text(&mut app).contains("Wager: 0 resources"));
    activate_choice(&mut app, "commit:confirm");
    assert_eq!(
        app.world().resource::<Table>().view().play.unwrap().players[0].resources,
        5
    );
}

#[test]
fn two_watch_wagers_share_affordability_survive_reopen_and_clear_on_deselection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("watch.json");
    let (mut session, first) = watch_commitment(TonyEffectFixture::WatchDouble);
    let second = session.observe().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "03233" && card.id != first)
        .unwrap()
        .id;
    for id in [
        format!("commit:{first}"),
        format!("commit:{second}"),
        format!("tony:watch:{first}:3"),
    ] {
        session.choose(session.observe().revision, &id).unwrap();
    }
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(
        Table::new(continuity::load_session(&path).unwrap(), path.clone()),
        true,
    );
    let view = app.world().resource::<Table>().view();
    assert!(
        !view
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
    );
    assert!(
        !view
            .choices
            .iter()
            .any(|choice| choice.id == format!("tony:watch:{second}:3"))
    );
    for size in [Vec2::new(1280., 720.), Vec2::new(1600., 1000.)] {
        for scale in [1., 2.] {
            crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, scale);
            app.world_mut().resource_mut::<Table>().dirty = true;
            for _ in 0..5 {
                app.update();
            }
            for choice in view
                .choices
                .iter()
                .filter(|choice| choice.id.starts_with("tony:watch:"))
            {
                assert_unclipped_button(&mut app, &choice.label, size, scale);
            }
        }
    }
    activate_choice(&mut app, &format!("tony:watch:{second}:2"));
    activate_choice(&mut app, &format!("commit:{first}"));
    activate_choice(&mut app, &format!("commit:{first}"));
    assert!(
        !app.world()
            .resource::<Table>()
            .view()
            .choices
            .iter()
            .any(|choice| choice.id == "commit:confirm")
    );
    activate_choice(&mut app, &format!("tony:watch:{first}:1"));
    assert_eq!(
        app.world().resource::<Table>().view().play.unwrap().players[0].resources,
        5
    );
    activate_choice(&mut app, "commit:confirm");
    let view = app.world().resource::<Table>().view();
    assert_eq!(view.play.as_ref().unwrap().players[0].resources, 2);
    let reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), view);
}

#[test]
fn last_key_and_resourceful_offer_order_directly_and_preserve_both_outcomes() {
    use crate::gathering::PlayerEventFixture;
    for key_first in [false, true] {
        let mut session = Session::with_hostile_fixture(HostileFixture::PlayerEvents(
            PlayerEventFixture::KeyringLast,
        ));
        let player = &session.observe().play.unwrap().players[1];
        let key = player
            .assets
            .iter()
            .find(|asset| asset.card.code == "60507")
            .unwrap()
            .card
            .id;
        let resourceful = player
            .hand
            .iter()
            .find(|card| card.code == "03039")
            .unwrap()
            .id;
        for id in [
            format!("action:ordinary:investigate:spell:{key}"),
            "window:pass".into(),
            format!("commit:{resourceful}"),
            "commit:confirm".into(),
        ] {
            session.choose(session.observe().revision, &id).unwrap();
        }
        if session
            .observe()
            .choices
            .iter()
            .any(|choice| choice.id == "window:pass")
        {
            session
                .choose(session.observe().revision, "window:pass")
                .unwrap();
        }
        session
            .choose(session.observe().revision, "test:reveal")
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keyring.json");
        continuity::save_session(&path, &session).unwrap();
        let mut app = host_with_table(
            Table::new(continuity::load_session(&path).unwrap(), path.clone()),
            true,
        );
        finish_presentation(&mut app);
        if let Some(entity) = button_with_label(&mut app, "Continue") {
            app.world_mut().trigger(Activate { entity });
            app.update();
        }
        let view = app.world().resource::<Table>().view();
        let key_choice = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with("result:") && choice.label.contains("Old Keyring"))
            .unwrap();
        let resourceful_choice = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with("result:") && choice.label.contains("Resourceful"))
            .unwrap();
        for choice in [key_choice, resourceful_choice] {
            assert!(
                choice_button(&mut app, &choice.label).is_some(),
                "The ordering choice must be directly visible"
            );
        }
        assert!(rendered_text(&mut app).contains("All remaining results still apply"));
        let first = if key_first {
            key_choice
        } else {
            resourceful_choice
        };
        activate_choice(&mut app, &first.id);
        if key_first {
            let view = app.world().resource::<Table>().view();
            if let Some(choice) = view.choices.iter().find(|choice| {
                choice.id.starts_with("result:") && choice.label.contains("Resourceful")
            }) {
                activate_choice(&mut app, &choice.id);
            }
        }
        let recover_key = format!("resourceful:{resourceful}:{key}");
        let view = app.world().resource::<Table>().view();
        assert_eq!(
            view.choices.iter().any(|choice| choice.id == recover_key),
            key_first
        );
        if key_first {
            activate_choice(&mut app, &recover_key);
        } else {
            assert!(
                !view
                    .choices
                    .iter()
                    .any(|choice| choice.id.starts_with("resourceful:")),
                "With no eligible discard, Resourceful cannot recover a card that is still in play"
            );
        }
        let view = app.world().resource::<Table>().view();
        assert!(
            !view.play.as_ref().unwrap().players[1]
                .assets
                .iter()
                .any(|asset| asset.card.id == key)
        );
        assert_eq!(
            view.play.as_ref().unwrap().players[1]
                .hand
                .iter()
                .any(|card| card.id == key),
            key_first
        );
        assert_eq!(
            continuity::load_session(&path).unwrap(),
            app.world().resource::<Table>().session
        );
    }
}
