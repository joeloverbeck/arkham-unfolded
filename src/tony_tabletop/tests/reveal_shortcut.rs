use super::*;

fn reveal_button(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut buttons = world.query_filtered::<(Entity, &Children), With<Button>>();
    buttons
        .iter(world)
        .find_map(|(entity, children)| {
            children
                .iter()
                .any(|child| {
                    world
                        .get::<Text>(child)
                        .is_some_and(|text| text.0 == "Reveal chaos token")
                })
                .then_some(entity)
        })
        .expect("the live reveal button")
}

#[test]
fn shortcut_uses_the_saved_choice_once_and_focuses_test_only_on_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("reveal.session.json");
    let mut app = presentation_app(path.clone());
    choose(&mut app, "evade");
    let before = app.world().resource::<Table>().session.observe();
    let entity = reveal_button(&mut app);
    app.world_mut().trigger(Activate { entity });
    app.world_mut().trigger(Activate { entity });
    advance(&mut app, 0.0);
    let table = app.world().resource::<Table>();
    let after = table.session.observe();
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.stage, Stage::Resolve);
    assert!(after.held_token.is_some());
    assert_eq!(continuity::load_session(&path).unwrap().observe(), after);
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        layout::focus(Area::Test).0
    );
    assert!(!app.world().resource::<Orbit>().is_following());
    // The fading old entity cannot draw a second token, even if a queued activation survives.
    app.world_mut().trigger(Activate { entity });
    advance(&mut app, 0.0);
    assert_eq!(app.world().resource::<Table>().session.observe(), after);
}

#[test]
fn failed_shortcut_save_retains_state_randomness_and_camera_then_can_retry() {
    let dir = tempfile::tempdir().unwrap();
    let good_path = dir.path().join("reveal.session.json");
    let mut app = presentation_app(good_path.clone());
    choose(&mut app, "evade");
    let before = serde_json::to_value(&app.world().resource::<Table>().session).unwrap();
    let pose = app.world().resource::<Orbit>().pose();
    app.world_mut().resource_mut::<Table>().save_path = dir.path().to_path_buf();
    let entity = reveal_button(&mut app);
    app.world_mut().trigger(Activate { entity });
    advance(&mut app, 0.0);
    assert_eq!(
        serde_json::to_value(&app.world().resource::<Table>().session).unwrap(),
        before
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), pose);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .contains("not committed")
    );
    app.world_mut().resource_mut::<Table>().save_path = good_path;
    let entity = reveal_button(&mut app);
    app.world_mut().trigger(Activate { entity });
    advance(&mut app, 0.0);
    assert_eq!(
        app.world().resource::<Table>().session.observe().stage,
        Stage::Resolve
    );
}

#[test]
fn queued_shortcut_does_not_cross_inspection_replay_or_a_changed_decision() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = presentation_app(dir.path().join("reveal.session.json"));
    choose(&mut app, "evade");
    let before = app.world().resource::<Table>().session.observe();
    let entity = reveal_button(&mut app);
    app.world_mut().trigger(Activate { entity });
    act(&mut app, Action::Inspect(0));
    assert_eq!(app.world().resource::<Table>().session.observe(), before);
    act(&mut app, Action::Close);
    let entity = reveal_button(&mut app);
    app.world_mut().trigger(Activate { entity });
    act(&mut app, Action::Replay);
    assert_eq!(app.world().resource::<Table>().session.observe(), before);
    act(&mut app, Action::Live);
    let entity = reveal_button(&mut app);
    app.world_mut().trigger(Activate { entity });
    choose(&mut app, "undo");
    assert_eq!(
        app.world().resource::<Table>().session.observe().stage,
        Stage::Action
    );
}

#[test]
fn reveal_shortcut_fade_keeps_text_and_image_backgrounds_transparent() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = keyboard_menu_app(dir.path().join("reveal.session.json"));
    app.world_mut().resource_mut::<reveal::Artwork>().0 = Some(Handle::default());
    choose(&mut app, "evade");
    advance(&mut app, 0.2);
    let entity = reveal_button(&mut app);
    let children = app.world().get::<Children>(entity).unwrap();
    assert_eq!(children.len(), 2, "prepared icon plus readable label");
    for child in children {
        assert_eq!(
            app.world()
                .get::<BackgroundColor>(*child)
                .unwrap()
                .0
                .alpha(),
            0.0,
            "foreground fade must preserve transparent node backgrounds"
        );
    }
}

#[test]
fn reveal_shortcut_has_no_first_frame_flash_and_reduced_motion_settles_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = keyboard_menu_app(dir.path().join("reveal.session.json"));
    choose(&mut app, "evade");
    let entity = reveal_button(&mut app);
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(entity)
            .unwrap()
            .0
            .alpha(),
        0.0
    );
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<reveal::Root>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world().get::<UiTransform>(root).unwrap().scale,
        Vec2::splat(0.94)
    );
    advance(&mut app, 0.07);
    let alpha = app
        .world()
        .get::<BackgroundColor>(entity)
        .unwrap()
        .0
        .alpha();
    assert!((alpha - 0.5).abs() < 0.001);
    app.world_mut().resource_mut::<Table>().reduced_motion = true;
    advance(&mut app, 0.0);
    assert_eq!(
        app.world()
            .get::<BackgroundColor>(entity)
            .unwrap()
            .0
            .alpha(),
        1.0
    );
    assert_eq!(
        app.world().get::<UiTransform>(root).unwrap().scale,
        Vec2::ONE
    );
    // Becoming illegal ends keyboard and pointer input before any exit decoration.
    app.world_mut().resource_mut::<Table>().reduced_motion = false;
    app.world_mut().trigger(Activate { entity });
    advance(&mut app, 0.0);
    assert!(app.world().get::<Button>(entity).is_none());
    assert!(app.world().get::<TabIndex>(entity).is_none());
}

#[test]
fn reveal_shortcut_uses_production_keyboard_activation() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, window) = keyboard_menu_app(dir.path().join("reveal.session.json"));
    choose(&mut app, "evade");
    let entity = reveal_button(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, bevy::input_focus::FocusCause::Navigated);
    keyboard_press(&mut app, window, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<Table>().session.observe().stage,
        Stage::Resolve
    );
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        layout::focus(Area::Test).0
    );
}

#[test]
fn reveal_shortcut_cannot_act_while_an_external_asset_dialog_is_pending() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = presentation_app(dir.path().join("reveal.session.json"));
    choose(&mut app, "evade");
    let before = app.world().resource::<Table>().session.observe();
    let entity = reveal_button(&mut app);
    app.world_mut().trigger(Activate { entity });
    let (settings, pending_selection) = AssetSettings::new_tony(AssetConfig::load_at(
        dir.path().join("settings.toml"),
        dir.path().join("art"),
    ))
    .with_pending_selection();
    app.insert_resource(settings);
    advance(&mut app, 0.0);
    assert!(app.world().resource::<AssetSettings>().is_busy());
    assert_eq!(app.world().resource::<Table>().session.observe(), before);
    assert!(app.world().get::<Button>(entity).is_none());
    drop(pending_selection);
    advance(&mut app, 0.0);
    assert!(!app.world().resource::<AssetSettings>().is_busy());
    // Releasing the dialog must not dispatch the activation that was drained.
    assert_eq!(app.world().resource::<Table>().session.observe(), before);
}
