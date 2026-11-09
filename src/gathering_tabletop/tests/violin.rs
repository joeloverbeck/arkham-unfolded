use super::*;

fn session() -> Session {
    Session::with_hostile_fixture(crate::gathering::HostileFixture::Assets(
        crate::gathering::AssetFixture::RobesOpportunity,
    ))
}

fn isolated_session() -> Session {
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::PatriceCancellationVision);
    let view = session.observe();
    let cancel = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap();
    session.choose(view.revision, &cancel.id).unwrap();
    session
}

#[test]
fn violin_only_offers_recipients_at_patrices_location() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(isolated_session(), directory.path().join("isolated.json")),
        false,
    );
    open(&mut app);
    assert!(choice_button(&mut app, "Patrice Hathaway").is_some());
    assert!(choice_button(&mut app, "Tony Morgan").is_none());
    assert!(choice_button(&mut app, "Jacqueline Fine").is_none());
}

fn open(app: &mut App) {
    finish_presentation(app);
    let view = app.world().resource::<Table>().session.observe();
    let asset = view.play.as_ref().unwrap().players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "06016")
        .unwrap()
        .card
        .id;
    click_source(app, controls::Source::Card(asset));
    press(app, "Use Patrice's Violin…");
}

fn press(app: &mut App, label: &str) {
    let entity = choice_button(app, label).unwrap_or_else(|| panic!("Missing {label}"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

#[test]
fn violin_opens_one_compact_chooser_without_paying_a_cost() {
    let session = session();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("violin.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session.clone(), path.clone()), false);
    open(&mut app);
    for label in ["Draw 1 card", "Gain 1 resource", "Use Violin", "Cancel"] {
        assert!(choice_button(&mut app, label).is_some(), "Missing {label}");
    }
    for choice in session.observe().choices {
        if choice.id.starts_with("violin:") {
            assert!(choice_button(&mut app, &choice.label).is_none());
        }
    }
    assert_eq!(app.world().resource::<Table>().session, session);
    assert_eq!(continuity::load_session(path).unwrap(), session);
}

#[test]
fn escape_dismisses_violin_and_stale_confirmation_cannot_spend() {
    use bevy::input::keyboard::Key;
    let session = session();
    let card = first_payment(&session);
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session.clone(), directory.path().join("escape.json")),
        false,
    );
    open(&mut app);
    press(&mut app, &card.title);
    press(&mut app, "Tony Morgan");
    press(&mut app, "Draw 1 card");
    let table = app.world().resource::<Table>();
    let stale = Action::Violin(
        table.epoch,
        table.session.observe().revision,
        super::super::violin::Input::Confirm,
    );
    keyboard(&mut app, KeyCode::Escape, Key::Escape, true);
    keyboard(&mut app, KeyCode::Escape, Key::Escape, false);
    assert!(choice_button(&mut app, "Use Violin").is_none());
    open(&mut app);
    app.world_mut().resource_mut::<Actions>().0.push(stale);
    app.update();
    assert_eq!(app.world().resource::<Table>().session, session);
}

fn first_payment(session: &Session) -> crate::gathering::CardView {
    let view = session.observe();
    view.play.as_ref().unwrap().players[1]
        .hand
        .iter()
        .find(|card| {
            view.choices
                .iter()
                .any(|choice| choice.id.contains(&format!(":discard:{}:", card.id)))
        })
        .unwrap()
        .clone()
}

#[test]
fn violin_preserves_card_scrolling_across_edits_and_inspection() {
    use super::super::violin::Cards;
    use bevy::input::keyboard::Key;
    let mut session = session();
    for id in [
        "turn:end",
        "turn:patrice",
        "action:ordinary:draw",
        "action:ordinary:draw",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session.clone(), directory.path().join("scroll.json")),
        false,
    );
    // Real images have the same fixed preview height; a shared test image keeps
    // this production layout check independent of private artwork availability.
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    app.insert_resource(art::Art::with_images_for_test(
        [
            "card-60513",
            "card-04201",
            "card-01080",
            "card-01093",
            "card-02192",
        ]
        .map(|role| (role, image.clone())),
    ));
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(960., 640.),
        1.,
    );
    open(&mut app);
    for _ in 0..4 {
        app.update();
    }
    keyboard(&mut app, KeyCode::End, Key::End, true);
    keyboard(&mut app, KeyCode::End, Key::End, false);
    let offset = app
        .world_mut()
        .query_filtered::<&ScrollPosition, With<Cards>>()
        .single(app.world())
        .unwrap()
        .y;
    assert!(offset > 0., "Larger hand must exercise scrolling");
    press(&mut app, "Patrice Hathaway");
    assert_eq!(
        app.world_mut()
            .query_filtered::<&ScrollPosition, With<Cards>>()
            .single(app.world())
            .unwrap()
            .y,
        offset
    );
    let card = first_payment(&session);
    press(&mut app, &format!("Inspect {}", card.title));
    press(&mut app, "Close · Esc");
    assert_eq!(
        app.world_mut()
            .query_filtered::<&ScrollPosition, With<Cards>>()
            .single(app.world())
            .unwrap()
            .y,
        offset
    );
    assert_eq!(app.world().resource::<Table>().session, session);
}

#[test]
fn editing_inspecting_and_canceling_violin_choices_preserves_the_live_and_saved_game() {
    let session = session();
    let card = first_payment(&session);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("cancel.json");
    continuity::save_session(&path, &session).unwrap();
    let mut app = host_with_table(Table::new(session.clone(), path.clone()), false);
    open(&mut app);
    press(&mut app, "Use Violin");
    assert_eq!(
        app.world().resource::<Table>().session,
        session,
        "Incomplete confirmation does nothing"
    );
    press(&mut app, &card.title);
    press(&mut app, "Tony Morgan");
    press(&mut app, "Gain 1 resource");
    press(&mut app, "Patrice Hathaway");
    press(&mut app, "Draw 1 card");
    press(&mut app, &format!("Inspect {}", card.title));
    assert_eq!(
        app.world()
            .resource::<Table>()
            .inspection
            .as_ref()
            .unwrap()
            .1,
        card.code
    );
    press(&mut app, "Close · Esc");
    assert!(choice_button(&mut app, "Use Violin").is_some());
    assert!(rendered_text(&mut app).contains(&format!("discard {}.", card.title)));
    press(&mut app, "Cancel");
    assert!(choice_button(&mut app, "Use Violin").is_none());
    assert_eq!(app.world().resource::<Table>().session, session);
    assert_eq!(continuity::load_session(path).unwrap(), session);
}

#[test]
fn violin_confirmation_pays_the_selected_cost_and_benefit_once_and_reopens_exactly() {
    let session = session();
    let card = first_payment(&session);
    let before = session.observe();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("confirm.json");
    let mut app = host_with_table(Table::new(session.clone(), path.clone()), false);
    open(&mut app);
    press(&mut app, &card.title);
    press(&mut app, "Tony Morgan");
    press(&mut app, "Gain 1 resource");
    let confirm = choice_button(&mut app, "Use Violin").unwrap();
    app.world_mut().trigger(Activate { entity: confirm });
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    let committed = &app.world().resource::<Table>().session;
    let after = committed.observe();
    let players = &after.play.as_ref().unwrap().players;
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(
        players[0].resources,
        before.play.as_ref().unwrap().players[0].resources + 1
    );
    assert!(!players[1].hand.iter().any(|held| held.id == card.id));
    assert!(
        players[1]
            .discard
            .iter()
            .any(|discarded| discarded.id == card.id)
    );
    assert!(
        !players[1]
            .assets
            .iter()
            .find(|asset| asset.card.code == "06016")
            .unwrap()
            .ready
    );
    assert_eq!(&continuity::load_session(path).unwrap(), committed);
}

#[test]
fn failed_violin_save_preserves_the_selection_for_retry_without_paying() {
    let session = session();
    let card = first_payment(&session);
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session.clone(), directory.path().to_path_buf()),
        false,
    );
    open(&mut app);
    press(&mut app, &card.title);
    press(&mut app, "Tony Morgan");
    press(&mut app, "Gain 1 resource");
    press(&mut app, "Use Violin");
    assert_eq!(app.world().resource::<Table>().session, session);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .starts_with("Decision not committed")
    );
    assert!(choice_button(&mut app, "Use Violin").is_some());
    let path = directory.path().join("retry.json");
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    press(&mut app, "Use Violin");
    let saved = continuity::load_session(path).unwrap();
    assert_eq!(saved.observe().revision, session.observe().revision + 1);
    assert_eq!(app.world().resource::<Table>().session, saved);
}

#[test]
fn violin_choices_with_artwork_fit_the_viewport_and_keep_completion_controls_visible() {
    for size in [Vec2::new(1280., 800.), Vec2::new(960., 640.)] {
        let directory = tempfile::tempdir().unwrap();
        let mut app = host_with_table(
            Table::new(session(), directory.path().join("layout.json")),
            false,
        );
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.insert_resource(art::Art::with_images_for_test(
            [
                "card-60513",
                "card-04201",
                "card-01080",
                "card-01093",
                "card-02192",
                "standee/06003",
                "standee/06005",
                "standee/60401",
            ]
            .map(|role| (role, image.clone())),
        ));
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
        open(&mut app);
        for _ in 0..5 {
            app.update();
        }
        for label in [
            "Draw 1 card",
            "Gain 1 resource",
            "Tony Morgan",
            "Patrice Hathaway",
            "Jacqueline Fine",
            "Use Violin",
            "Cancel",
        ] {
            assert_unclipped_button(&mut app, label, size, 1.);
        }
    }
}
