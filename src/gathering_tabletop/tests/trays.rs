use super::draws::{physical, tick};
use super::*;
use crate::gathering::{HostileFixture, SearchFixture};

fn hornet(fixture: SearchFixture) -> Session {
    let mut session = Session::with_hostile_fixture(HostileFixture::Search(fixture));
    let view = session.observe();
    let event = view.investigators[0]
        .hand
        .iter()
        .find(|card| card.code == "09068")
        .unwrap();
    session
        .choose(
            view.revision,
            &format!("action:ordinary:play:tony:{}", event.id),
        )
        .unwrap();
    session
}

fn card_at(app: &mut App, id: u16) -> Vec3 {
    let world = app.world_mut();
    world
        .query::<(&ContextTarget, &Transform)>()
        .iter(world)
        .find_map(|(target, pose)| {
            (target.source == controls::Source::Card(id)).then_some(pose.translation)
        })
        .expect("each visible search card has its own selectable face")
}

#[test]
fn hornet_search_has_a_separate_readable_home_and_keeps_its_legal_card_menu() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("search.json");
    let session = hornet(SearchFixture::HornetTargets);
    let view = session.observe();
    let searched = view.play.as_ref().unwrap().searched_encounter.clone();
    assert_eq!(searched.len(), 9);
    let mut table = Table::new(session.clone(), path.clone());
    table.reduced_motion = true;
    let mut app = host_with_table(table, false);
    let positions: Vec<_> = searched
        .iter()
        .map(|card| card_at(&mut app, card.id))
        .collect();
    for (index, at) in positions.iter().enumerate() {
        assert!(
            at.z > 30.,
            "search cards belong in front of the investigator, outside Test: {at:?}"
        );
        for other in &positions[..index] {
            assert!(
                (at.x - other.x).abs() > 1.35 || (at.z - other.z).abs() > 1.89,
                "whole card faces must remain separate"
            );
        }
    }
    assert!(button_with_label(&mut app, "Resolve").is_some());
    let resolve = button_with_label(&mut app, "Resolve").unwrap();
    app.world_mut().trigger(Activate { entity: resolve });
    for _ in 0..15 {
        tick(&mut app, 0.1);
    }
    let corners: Vec<_> = positions
        .iter()
        .flat_map(|at| {
            [
                *at + Vec3::new(-0.675, 0., -0.945),
                *at + Vec3::new(0.675, 0., 1.5),
            ]
        })
        .collect();
    for size in [Vec2::new(1600., 1000.), Vec2::new(3200., 2000.)] {
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
        for _ in 0..3 {
            app.update();
        }
        super::review_saves::assert_draw_anchors_unobscured(
            &mut app,
            &corners,
            size,
            "search drawer",
        );
    }
    let choice = &view.choices[0].id;
    let mut expected = session;
    expected.choose(view.revision, choice).unwrap();
    activate_choice(&mut app, choice);
    assert_eq!(app.world().resource::<Table>().session, expected);
    assert_eq!(continuity::load_session(&path).unwrap(), expected);
}

#[test]
fn searched_cards_travel_from_and_back_to_the_deck_before_the_drawer_closes() {
    let directory = tempfile::tempdir().unwrap();
    let session =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HornetNoMatch));
    let event = session.observe().investigators[0]
        .hand
        .iter()
        .find(|card| card.code == "09068")
        .unwrap()
        .id;
    let mut app = host_with_table(
        Table::new(session, directory.path().join("travel.json")),
        false,
    );
    let camera = app.world().resource::<Orbit>().pose();
    activate_choice(&mut app, &format!("action:ordinary:play:tony:{event}"));
    let target = app.world().resource::<Table>().session.observe();
    let searched = target.play.as_ref().unwrap().searched_encounter.clone();
    for _ in 0..30 {
        if !app
            .world()
            .resource::<Table>()
            .view()
            .play
            .unwrap()
            .searched_encounter
            .is_empty()
        {
            break;
        }
        tick(&mut app, 0.1);
    }
    let start = physical(&mut app, searched[0].id);
    assert!(
        start.translation.xz().distance(layout::ENCOUNTER_DECK.xz()) < 0.01,
        "A disclosed search card starts at its deck: {start:?}"
    );
    assert!((start.rotation * Vec3::Y).y < -0.99);
    finish_presentation(&mut app);
    assert!(physical(&mut app, searched[0].id).translation.z > 30.);
    activate_choice(&mut app, "search:no-match");
    let returning = physical(&mut app, searched[0].id);
    assert!(returning.translation.z > 30., "Return begins at the drawer");
    tick(&mut app, 0.35);
    assert_ne!(
        physical(&mut app, searched[0].id).translation,
        returning.translation
    );
    let world = app.world_mut();
    assert!(
        world
            .query::<(&Name, &Visibility)>()
            .iter(world)
            .any(|(name, visible)| name.as_str() == "Resolution tray"
                && *visible != Visibility::Hidden),
        "The drawer stays out during returns"
    );
    finish_presentation(&mut app);
    for _ in 0..10 {
        tick(&mut app, 0.1);
    }
    let world = app.world_mut();
    assert!(
        !world
            .query::<(&motion::PhysicalCard, &Transform)>()
            .iter(world)
            .any(|(card, _)| searched.iter().any(|searched| searched.id == card.0)),
        "Shuffled cards no longer have inspectable public faces"
    );
    assert!(
        world
            .query::<(&Name, &Visibility)>()
            .iter(world)
            .filter(|(name, _)| name.as_str() == "Resolution tray")
            .all(|(_, visible)| *visible == Visibility::Hidden)
    );
    assert_eq!(
        app.world().resource::<Orbit>().pose(),
        camera,
        "Manual camera stays put"
    );
}

#[test]
fn backpack_cards_use_the_tray_and_finish_with_the_same_saved_attachments() {
    use crate::gathering::BackpackFixture;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("backpack.json");
    let session = Session::with_hostile_fixture(HostileFixture::Backpack(BackpackFixture::Search));
    let pack = session.observe().investigators[0]
        .hand
        .iter()
        .find(|card| card.code == "04037")
        .unwrap()
        .id;
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    activate_choice(&mut app, &format!("action:ordinary:play:{pack}"));
    activate_choice(&mut app, "backpack:search");
    finish_presentation(&mut app);
    let searched = app
        .world()
        .resource::<Table>()
        .view()
        .play
        .unwrap()
        .searched_player;
    assert_eq!(searched.len(), 6);
    for card in &searched {
        assert!(card_at(&mut app, card.id).z > 30.);
    }
    for card in &searched[..3] {
        activate_choice(&mut app, &format!("backpack:select:{}", card.id));
    }
    let pending = continuity::load_session(&path).unwrap();
    let mut resumed = host_with_table(Table::new(pending.clone(), path), false);
    for card in &searched {
        assert!(card_at(&mut resumed, card.id).z > 30.);
    }
    let mut expected = pending;
    expected
        .choose(expected.observe().revision, "backpack:attach")
        .unwrap();
    activate_choice(&mut resumed, "backpack:attach");
    finish_presentation(&mut resumed);
    assert_eq!(resumed.world().resource::<Table>().session, expected);
    let play = expected.observe().play.unwrap();
    assert_eq!(play.backpacks[0].cards, searched[..3]);
    assert!(
        !play.players[0]
            .hand
            .iter()
            .any(|card| searched[..3].contains(card))
    );
}

#[test]
fn a_revelation_keeps_its_tray_while_its_cancellation_test_uses_test() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested.json");
    let session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    let view = session.observe();
    let revelation = view
        .play
        .as_ref()
        .unwrap()
        .pending_revelation
        .as_ref()
        .unwrap()
        .card
        .id;
    let choice = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap()
        .id
        .clone();
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    let at = card_at(&mut app, revelation);
    assert!(at.z > 30. && at.x.abs() < 7.);
    activate_choice(&mut app, &choice);
    finish_presentation(&mut app);
    tick(&mut app, 0.5);
    assert_eq!(card_at(&mut app, revelation), at);
    let during = app.world().resource::<Table>().session.observe();
    let source = during
        .play
        .as_ref()
        .unwrap()
        .test
        .as_ref()
        .unwrap()
        .source_card
        .as_ref()
        .unwrap()
        .id;
    assert!(
        physical(&mut app, source)
            .translation
            .abs_diff_eq(layout::test_source(), 0.001)
    );
    let resumed = continuity::load_session(&path).unwrap();
    assert_eq!(resumed.observe(), during);
    let mut reopened = host_with_table(Table::new(resumed, path), false);
    assert_eq!(card_at(&mut reopened, revelation), at);
    assert!(
        physical(&mut reopened, source)
            .translation
            .abs_diff_eq(layout::test_source(), 0.001)
    );
}

#[test]
fn non_test_tokens_resolve_in_the_tray_and_sealed_premonition_stays_with_its_owner() {
    use crate::gathering::TokenFixture;
    for case in [TokenFixture::VoiceRaCurse, TokenFixture::Premonition] {
        let directory = tempfile::tempdir().unwrap();
        let session = Session::with_hostile_fixture(HostileFixture::Tokens(case));
        let mut app = host_with_table(
            Table::new(session, directory.path().join("tokens.json")),
            false,
        );
        activate_choice(&mut app, "test:reveal");
        let mut witnessed = false;
        for _ in 0..60 {
            let world = app.world_mut();
            for (target, at) in world.query::<(&ContextTarget, &Transform)>().iter(world) {
                if matches!(target.source, controls::Source::Token(_)) {
                    assert!(at.translation.z > 30. && at.translation.x > 10.);
                    witnessed = true;
                }
            }
            tick(&mut app, 0.1);
        }
        assert!(
            witnessed,
            "The non-test reveal must have a visible token beat"
        );
        let final_view = app.world().resource::<Table>().session.observe();
        if let Some(sealed) = &final_view.play.as_ref().unwrap().premonition {
            let at = physical(&mut app, sealed.card.id).translation;
            assert!(
                at.z > 30. && at.x > 10.,
                "Sealed Premonition remains with Jacqueline: {at:?}"
            );
        }
        assert!(final_view.play.unwrap().completed_tests.is_empty());
    }
}

#[test]
fn pending_sealed_reveal_keeps_card_and_tokens_visible_then_moves_them_before_discard() {
    use crate::gathering::TokenFixture;
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::PremonitionVoice));
    session
        .choose(session.observe().revision, "test:reveal")
        .unwrap();
    let voice = session.observe().play.unwrap().players[2]
        .hand
        .iter()
        .find(|card| card.code == "60416")
        .unwrap()
        .id;
    session
        .choose(
            session.observe().revision,
            &format!("action:ordinary:play:spell:{voice}"),
        )
        .unwrap();
    let view = session.observe();
    let sealed = view
        .play
        .as_ref()
        .unwrap()
        .premonition
        .clone()
        .expect("Pending replacement still owns visible sealed tokens");
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session, directory.path().join("sealed.json")),
        false,
    );
    assert!(physical(&mut app, sealed.card.id).translation.z > 30.);
    assert!(rendered_text(&mut app).contains("Resolve sealed"));
    activate_choice(&mut app, "test:reveal");
    // Voice of Ra still has legal modifiers for its fresh positions: the first
    // press opens their chooser, and this press selects the announced reveal.
    assert!(
        app.world()
            .resource::<crate::tabletop_reveal::Shortcut>()
            .is_open()
    );
    let world = app.world_mut();
    let reveal = world
        .query_filtered::<Entity, (With<Button>, With<bevy::input_focus::AutoFocus>)>()
        .single(world)
        .unwrap();
    world.trigger(Activate { entity: reveal });
    app.update();
    assert!(
        app.world()
            .resource::<Table>()
            .session
            .observe()
            .play
            .unwrap()
            .premonition
            .is_none()
    );
    let start = physical(&mut app, sealed.card.id).translation;
    assert!(start.z > 30., "Card must stay while tokens depart");
    let token_start = super::super::trays::sealed_token(
        view.play.as_ref().unwrap(),
        sealed.controller,
        0,
        sealed.faces.len(),
    );
    let token_at = |app: &mut App| {
        let world = app.world_mut();
        world
            .query::<(&sealed::Piece, &Transform)>()
            .iter(world)
            .find(|(piece, _)| piece.face == sealed.faces[0])
            .map(|(_, at)| at.translation)
            .unwrap()
    };
    assert!(token_at(&mut app).abs_diff_eq(token_start, 0.01));
    tick(&mut app, 0.2);
    let traveling = token_at(&mut app);
    assert!(!traveling.abs_diff_eq(token_start, 0.01));
    assert!(
        physical(&mut app, sealed.card.id)
            .translation
            .abs_diff_eq(start, 0.01)
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::History);
    app.update();
    tick(&mut app, 0.4);
    assert!(
        token_at(&mut app).abs_diff_eq(traveling, 0.01),
        "History pauses token travel across a redraw"
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    app.update();
    for _ in 0..80 {
        tick(&mut app, 0.1);
    }
    let final_view = app.world().resource::<Table>().session.observe();
    let play = final_view.play.as_ref().unwrap();
    assert!(
        play.premonition.is_none(),
        "notice={} choices={:?}",
        app.world().resource::<Table>().notice,
        final_view.choices
    );
    assert!(
        play.players[sealed.controller.index()]
            .discard
            .iter()
            .any(|card| card.id == sealed.card.id)
    );
    assert!(!app.world().resource::<Table>().trays.active());
    let world = app.world_mut();
    assert!(
        world
            .query_filtered::<&Visibility, With<super::super::trays::Surface>>()
            .iter(world)
            .all(|visibility| *visibility == Visibility::Hidden)
    );
}

#[test]
fn defeated_assets_wait_in_the_drawer_and_both_travel_to_the_ordered_discard() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::SimultaneousAssetDefeat);
    let view = session.observe();
    let assets = &view.play.as_ref().unwrap().players[2].assets;
    let olive = assets
        .iter()
        .find(|asset| asset.card.code == "04197")
        .unwrap()
        .card
        .id;
    let robes = assets
        .iter()
        .find(|asset| asset.card.code == "60412")
        .unwrap()
        .card
        .id;
    let mut app = host_with_table(
        Table::new(session, directory.path().join("discard.json")),
        false,
    );
    for id in [
        "action:ordinary:resource".into(),
        "opportunity:400".into(),
        format!("harm:damage:{olive}"),
        format!("harm:damage:{robes}"),
        "harm:horror:investigator".into(),
        "harm:horror:investigator".into(),
    ] {
        activate_choice(&mut app, &id);
    }
    finish_presentation(&mut app);
    tick(&mut app, 0.5);
    let view = app.world().resource::<Table>().view();
    assert_eq!(
        view.play
            .as_ref()
            .unwrap()
            .pending
            .discard
            .as_ref()
            .unwrap()
            .remaining
            .len(),
        2
    );
    for id in [olive, robes] {
        assert!(physical(&mut app, id).translation.z > 30.);
    }
    {
        let view = app.world().resource::<Table>().session.observe();
        let choice = view
            .choices
            .iter()
            .find(|choice| choice.id.starts_with("discard-card:"))
            .unwrap()
            .id
            .clone();
        activate_choice(&mut app, &choice);
    }
    for id in [olive, robes] {
        assert!(
            physical(&mut app, id).translation.z > 30.,
            "Every ordered card starts its departure in the tray"
        );
    }
    finish_presentation(&mut app);
    let play = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    assert!(play.pending.discard.is_none());
    assert!(play.players[2].discard.iter().any(|card| card.id == olive));
    assert!(play.players[2].discard.iter().any(|card| card.id == robes));
}

#[test]
fn premonition_with_ignored_tokens_retains_its_drawer_through_the_completion_snapshot() {
    use crate::gathering::TokenFixture;
    let directory = tempfile::tempdir().unwrap();
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tokens(TokenFixture::PremonitionOlive));
    let olive = session
        .observe()
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("tokens:olive:"))
        .unwrap()
        .id
        .clone();
    for id in [
        olive.as_str(),
        "test:reveal",
        "tokens:keep:0",
        "tokens:keep:1",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let event = session
        .observe()
        .play
        .unwrap()
        .pending
        .resolving_cards
        .iter()
        .find(|card| card.code == "04199")
        .unwrap()
        .id;
    let mut app = host_with_table(
        Table::new(session, directory.path().join("ignored.json")),
        false,
    );
    activate_choice(&mut app, "tokens:confirm");
    let view = app.world().resource::<Table>().view();
    assert!(
        !view
            .play
            .as_ref()
            .unwrap()
            .tokens
            .as_ref()
            .unwrap()
            .ignored
            .is_empty()
    );
    assert!(physical(&mut app, event).translation.z > 30.);
    finish_presentation(&mut app);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .view()
            .play
            .unwrap()
            .premonition
            .unwrap()
            .faces
            .len(),
        2
    );
}

#[test]
fn a_player_drawn_weakness_has_a_physical_deck_to_drawer_beat() {
    use crate::gathering::DrawFixture;
    let directory = tempfile::tempdir().unwrap();
    let session =
        Session::with_hostile_fixture(HostileFixture::Draws(DrawFixture::JacquelineWeaknessOrder));
    let mut app = host_with_table(
        Table::new(session, directory.path().join("weakness-draw.json")),
        false,
    );
    if let Some(entity) = button_with_label(&mut app, "Continue") {
        app.world_mut().trigger(Activate { entity });
        app.update();
    }
    activate_choice(&mut app, "result:0");
    let view = app.world().resource::<Table>().view();
    let card = view.play.unwrap().pending.resolving_cards[0].clone();
    let at = physical(&mut app, card.id);
    assert!(
        at.translation
            .xz()
            .distance(layout::investigator_deck(2).xz())
            < 0.01
    );
    assert!((at.rotation * Vec3::Y).y < -0.99);
    finish_presentation(&mut app);
    assert!(physical(&mut app, card.id).translation.z > 30.);
}

#[test]
fn queued_cleanup_for_another_investigator_does_not_claim_the_active_discard_drawer() {
    use crate::gathering::TonyEffectFixture;
    let directory = tempfile::tempdir().unwrap();
    let mut session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::QuickThinking));
    let before = session.observe();
    let play = before.play.unwrap();
    let cards: Vec<_> = [(0, "02229"), (0, "60114"), (1, "01093")]
        .into_iter()
        .map(|(owner, code)| {
            play.players[owner]
                .hand
                .iter()
                .find(|card| card.code == code)
                .unwrap()
                .id
        })
        .collect();
    session
        .choose(
            before.revision,
            &format!("action:ordinary:fight:{}", play.scenario.enemies[0].card.id),
        )
        .unwrap();
    for _ in 0..8 {
        let view = session.observe();
        if view
            .choices
            .iter()
            .any(|choice| choice.id == format!("commit:{}", cards[0]))
        {
            break;
        }
        let id = ["test-card:pass", "window:pass"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .unwrap();
        session.choose(view.revision, id).unwrap();
    }
    for card in cards {
        session
            .choose(session.observe().revision, &format!("commit:{card}"))
            .unwrap();
    }
    for _ in 0..24 {
        let view = session.observe();
        if view.play.as_ref().unwrap().pending.discard.is_some() {
            break;
        }
        let id = [
            "test-card:pass",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
            "success:pass",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .map(str::to_string)
        .or_else(|| {
            view.choices
                .iter()
                .find(|choice| choice.id.starts_with("result:"))
                .map(|choice| choice.id.clone())
        })
        .unwrap_or_else(|| panic!("Unexpected cleanup preparation {:?}", view.choices));
        session.choose(view.revision, &id).unwrap();
    }
    let pending = session.observe().play.unwrap().pending.discard.unwrap();
    assert_eq!(pending.owner, Investigator::Tony);
    assert_eq!(pending.remaining.len(), 2);
    let mut app = host_with_table(
        Table::new(session, directory.path().join("owners.json")),
        false,
    );
    for card in pending.remaining {
        let at = physical(&mut app, card.id).translation;
        assert!(
            at.x < -10. && at.z > 30.,
            "Tony's active order uses Tony's drawer: {at:?}"
        );
    }
}
