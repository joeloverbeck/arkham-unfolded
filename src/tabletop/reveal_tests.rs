use super::*;

fn app(path: PathBuf, used: bool) -> App {
    app_in(App::new(), path, used)
}

fn app_in(mut app: App, path: PathBuf, used: bool) -> App {
    let fixture = if used {
        Fixture::curse().with_jacqueline_used()
    } else {
        Fixture::curse()
    };
    let mut session = Session::new(fixture.clone());
    session.choose(0, "decline-promise").unwrap();
    app.insert_resource(Tabletop {
        session,
        fixture,
        replay: None,
        save_path: path,
        notice: String::new(),
        inspect: None,
        history: false,
        result_open: false,
        reduced_motion: false,
        dirty: true,
    })
    .insert_resource(Orbit::new(CameraMode::Jacqueline))
    .init_resource::<Actions>()
    .init_resource::<Context>()
    .add_plugins(reveal::RevealPlugin)
    .add_systems(
        Update,
        (handle_actions, sync_reveal, reveal::render).chain(),
    );
    app.update();
    app
}

#[test]
fn reveal_shortcut_keyboard_tabs_within_chooser_and_escape_cancels_without_a_decision() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput},
        },
        input_focus::{InputDispatchPlugin, InputFocus, InputFocusPlugin},
        window::PrimaryWindow,
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keyboard.session.json");
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::text::TextPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::visibility::VisibilityPlugin,
        bevy::input::InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        crate::tabletop_feedback::FeedbackPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ))
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<bevy::prelude::Time<Real>>()
    .add_systems(Update, context::keyboard.before(handle_actions));
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    let mut app = app_in(app, path.clone(), false);
    let before = app.world().resource::<Tabletop>().session.observe();
    let camera = app.world().resource::<Orbit>().pose();
    click(&mut app, "Reveal chaos token");
    app.update();
    let normal = button(&mut app, "Reveal 1 · decline Jacqueline for this reveal");
    let reaction = button(&mut app, "Use Jacqueline · reveal 3 tokens");
    let cancel = button(&mut app, "Cancel");
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(normal));

    let key = |app: &mut App, key_code, logical_key: Key, state| {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    };
    for expected in [reaction, cancel, normal] {
        key(&mut app, KeyCode::Tab, Key::Tab, ButtonState::Pressed);
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(expected));
        key(&mut app, KeyCode::Tab, Key::Tab, ButtonState::Released);
    }
    key(
        &mut app,
        KeyCode::ShiftLeft,
        Key::Shift,
        ButtonState::Pressed,
    );
    key(&mut app, KeyCode::Tab, Key::Tab, ButtonState::Pressed);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(cancel));
    key(&mut app, KeyCode::Tab, Key::Tab, ButtonState::Released);
    key(
        &mut app,
        KeyCode::ShiftLeft,
        Key::Shift,
        ButtonState::Released,
    );
    key(&mut app, KeyCode::Escape, Key::Escape, ButtonState::Pressed);
    key(
        &mut app,
        KeyCode::Escape,
        Key::Escape,
        ButtonState::Released,
    );
    assert!(!app.world().resource::<reveal::Shortcut>().is_open());
    assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
    assert!(!path.exists());
}

fn button(app: &mut App, label: &str) -> Entity {
    let expected = readable_text(label.into());
    let world = app.world_mut();
    let mut query = world.query_filtered::<(Entity, &Children), With<Button>>();
    query
        .iter(world)
        .find_map(|(entity, children)| {
            children
                .iter()
                .any(|child| {
                    world
                        .get::<Text>(child)
                        .is_some_and(|text| text.0 == expected)
                })
                .then_some(entity)
        })
        .unwrap_or_else(|| panic!("button missing: {label}"))
}

fn click(app: &mut App, label: &str) {
    let entity = button(app, label);
    app.world_mut().trigger(Activate { entity });
    app.update();
}

#[test]
fn reveal_shortcut_preserves_the_reaction_choice_and_can_be_dismissed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jacqueline.session.json");
    let mut app = app(path.clone(), false);
    let before = app.world().resource::<Tabletop>().session.observe();
    let camera = app.world().resource::<Orbit>().pose();
    click(&mut app, "Reveal chaos token");
    assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
    assert!(!path.exists(), "opening the chooser is not a game decision");
    click(&mut app, "Cancel");
    assert_eq!(app.world().resource::<Tabletop>().session.observe(), before);
    click(&mut app, "Reveal chaos token");
    click(&mut app, "Use Jacqueline · reveal 3 tokens");
    let after = app.world().resource::<Tabletop>().session.observe();
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.stage, Stage::Cancellation);
    assert_eq!(after.tokens.len(), 3);
    assert!(
        after
            .choices
            .iter()
            .any(|choice| choice.id.starts_with("cancel-"))
    );
    assert_eq!(continuity::load_session(&path).unwrap().observe(), after);
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        tabletop_layout::focus(Area::Test).0
    );
}

#[test]
fn reveal_shortcut_decline_retains_jacqueline_for_the_next_curse_reveal() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("jacqueline.session.json"), false);
    let mut session = Session::new(Fixture::curse());
    session.choose(0, "commit-promise").unwrap();
    app.world_mut().resource_mut::<Tabletop>().session = session;
    app.update();
    click(&mut app, "Reveal chaos token");
    click(&mut app, "Reveal 1 · decline Jacqueline for this reveal");
    let after = app.world().resource::<Tabletop>().session.observe();
    assert_eq!(after.stage, Stage::BeforeReveal);
    assert_eq!(after.tokens.len(), 1);
    assert!(
        after
            .choices
            .iter()
            .any(|choice| choice.id == "use-jacqueline")
    );
    click(&mut app, "Reveal chaos token");
    click(&mut app, "Use Jacqueline · reveal 3 tokens");
    assert_eq!(
        app.world().resource::<Tabletop>().session.observe().stage,
        Stage::Cancellation
    );
}

#[test]
fn reveal_shortcut_without_a_reaction_draws_directly_and_rejects_double_activation() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("jacqueline.session.json"), true);
    let mut session = Session::new(Fixture::curse().with_jacqueline_used());
    session.choose(0, "commit-promise").unwrap();
    app.world_mut().resource_mut::<Tabletop>().session = session;
    app.update();
    let before = app.world().resource::<Tabletop>().session.observe();
    let entity = button(&mut app, "Reveal chaos token");
    app.world_mut().trigger(Activate { entity });
    app.world_mut().trigger(Activate { entity });
    app.update();
    let after = app.world().resource::<Tabletop>().session.observe();
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(
        after.stage,
        Stage::Finished,
        "the existing mandatory curse continuation is automatic"
    );
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        tabletop_layout::focus(Area::Test).0
    );
}

#[test]
fn reveal_shortcut_failed_save_and_modal_preserve_decision_and_camera() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().to_path_buf(), false);
    let before = serde_json::to_value(&app.world().resource::<Tabletop>().session).unwrap();
    let camera = app.world().resource::<Orbit>().pose();
    click(&mut app, "Reveal chaos token");
    click(&mut app, "Use Jacqueline · reveal 3 tokens");
    assert_eq!(
        serde_json::to_value(&app.world().resource::<Tabletop>().session).unwrap(),
        before
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
    assert!(
        app.world()
            .resource::<Tabletop>()
            .notice
            .contains("not committed")
    );
    click(&mut app, "Reveal chaos token");
    let entity = button(&mut app, "Use Jacqueline · reveal 3 tokens");
    app.world_mut().trigger(Activate { entity });
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Inspect(0));
    app.update();
    assert_eq!(
        serde_json::to_value(&app.world().resource::<Tabletop>().session).unwrap(),
        before
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
}
