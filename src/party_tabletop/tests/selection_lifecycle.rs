use super::*;

fn commitment_app(path: PathBuf) -> App {
    let mut app = app(path);
    choose(&mut app, "select:jacqueline");
    choose(&mut app, "investigate");
    app
}

fn select(app: &mut App, source: Source) -> (u64, u64) {
    let revision = app.world().resource::<Table>().view().revision;
    act(app, Action::Select(revision, source, Vec3::ZERO));
    (
        app.world().resource::<Table>().context.menu_epoch(),
        revision,
    )
}

fn toggle(app: &mut App, id: u16) {
    let (epoch, revision) = select(app, Source::Hand(id));
    act(app, Action::Toggle(epoch, revision, id, false));
}

fn committed(app: &App) -> Vec<u16> {
    app.world()
        .resource::<Table>()
        .view()
        .test
        .unwrap()
        .committed
        .iter()
        .map(|card| card.id)
        .collect()
}

#[test]
fn same_revision_reopen_clears_party_selection_but_failed_reopen_and_writes_preserve_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let mut app = commitment_app(path.clone());
    toggle(&mut app, 100);
    let before = app.world().resource::<Table>().session.clone();
    let revision = before.observe().revision;
    let saved = std::fs::read(&path).unwrap();
    app.world_mut().resource_mut::<Table>().save_path = dir.path().into();
    for (action, notice) in [
        (Action::Save, "Save failed"),
        (Action::Reopen, "Reopen failed"),
        (Action::Confirm(revision, false), "Decision not committed"),
    ] {
        act(&mut app, action);
        assert!(app.world().resource::<Table>().notice.starts_with(notice));
        assert_eq!(app.world().resource::<Table>().session, before);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
    }
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    act(&mut app, Action::Confirm(revision, false));
    assert_eq!(committed(&app), [100]);

    // Reopening the same decision succeeds but intentionally drops Party staging.
    let mut app = commitment_app(dir.path().join("reopen.json"));
    toggle(&mut app, 100);
    let before = app.world().resource::<Table>().session.clone();
    act(&mut app, Action::Reopen);
    act(&mut app, Action::Confirm(revision, false));
    assert_eq!(app.world().resource::<Table>().session, before);
    toggle(&mut app, 100);
    act(&mut app, Action::Confirm(revision, false));
    assert_eq!(committed(&app), [100]);
}

#[test]
fn source_less_confirmation_uses_the_changed_party_set_and_keeps_revision_refusal_visible() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let mut app = commitment_app(path.clone());
    toggle(&mut app, 100);
    let revision = app.world().resource::<Table>().view().revision;
    let confirmation = Action::Confirm(revision, false);
    toggle(&mut app, 200);
    toggle(&mut app, 100);
    act(&mut app, Action::Dismiss);
    let before = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::Confirm(revision - 1, false));
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .starts_with("Decision not committed")
    );
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    act(&mut app, confirmation);
    assert_eq!(committed(&app), [200]);
}

#[test]
fn reselected_source_rejects_old_callbacks_and_processes_current_callbacks_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = commitment_app(dir.path().join("party.json"));
    toggle(&mut app, 100);
    let (old, revision) = select(&mut app, Source::Hand(200));
    act(&mut app, Action::Dismiss);
    let (current, _) = select(&mut app, Source::Hand(200));
    act(&mut app, Action::Toggle(old, revision, 200, false));
    let context = &app.world().resource::<Table>().context;
    assert!(
        !context.is_pending(Source::Hand(200)),
        "the old callback must not toggle the card"
    );
    assert!(
        context.is_menu_source(Source::Hand(200)),
        "the current menu remains usable"
    );
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::Toggle(current, revision, 200, false),
        Action::Toggle(current, revision, 200, false),
        Action::Confirm(revision, false),
    ]);
    app.update();
    assert_eq!(committed(&app), [100, 200]);
}

#[test]
fn current_menu_callbacks_retain_continuitys_revision_and_id_refusals() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("party.json"));
    let (epoch, revision) = select(&mut app, Source::Card(cards::index("card/60401").unwrap()));
    let before = app.world().resource::<Table>().session.clone();
    for (revision, id) in [
        (revision + 1, "select:jacqueline"),
        (revision, "not-offered"),
    ] {
        let previous_notice = app.world().resource::<Table>().notice.clone();
        act(&mut app, Action::ContextChoice(epoch, revision, id.into()));
        assert_ne!(
            app.world().resource::<Table>().notice,
            previous_notice,
            "each refusal must produce its own notice, not leave the previous result"
        );
        assert!(
            app.world()
                .resource::<Table>()
                .notice
                .starts_with("Decision not committed")
        );
        assert_eq!(app.world().resource::<Table>().session, before);
    }
    act(
        &mut app,
        Action::ContextChoice(epoch, revision, "select:jacqueline".into()),
    );
    assert_eq!(
        app.world().resource::<Table>().view().actor,
        Some(Investigator::Jacqueline)
    );
}

fn activate(app: &mut App, label: &str) {
    let button = rendered_button(app, label);
    app.world_mut().trigger(Activate { entity: button });
    app.update();
}

fn select_token(app: &mut App, id: u8, label: &str) {
    select(app, Source::Token(id));
    activate(app, label);
}

#[test]
fn rendered_cancellation_preserves_alternatives_and_pending_tokens_through_inspection() {
    for cancelled in [vec![0], vec![1, 2]] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("party.json");
        let mut app = render_app(path.clone());
        app.init_resource::<FeedbackSettings>().add_systems(
            Update,
            (synchronize_feedback, controls::render)
                .chain()
                .after(ui::render),
        );
        app.world_mut()
            .spawn((Camera::default(), GlobalTransform::default(), MainCamera));
        app.world_mut().resource_mut::<Table>().session = Session::with_draws(vec![
            crate::TokenFace::AutoFail,
            crate::TokenFace::MinusOne,
            crate::TokenFace::MinusTwo,
        ]);
        for id in [
            "select:jacqueline",
            "investigate",
            "commit:none",
            "use-jacqueline",
        ] {
            choose(&mut app, id);
        }
        let before = app.world().resource::<Table>().session.clone();
        let revision = before.observe().revision;
        let saved = std::fs::read(&path).unwrap();
        assert!(before.observe().choices.iter().any(|c| c.id == "cancel:0"));
        assert!(
            before
                .observe()
                .choices
                .iter()
                .any(|c| c.id == "cancel:1,2")
        );

        select_token(&mut app, 1, "Select to cancel");
        act(&mut app, Action::Confirm(revision, true));
        assert_eq!(
            app.world().resource::<Table>().session,
            before,
            "incomplete selection cannot commit"
        );
        select_token(&mut app, 0, "Select to cancel");
        act(&mut app, Action::Confirm(revision, true));
        assert_eq!(
            app.world().resource::<Table>().session,
            before,
            "mixed auto-fail selection cannot commit"
        );
        assert!(
            !visible_text(&mut app)
                .iter()
                .any(|text| text == "Confirm selected cancellation")
        );
        select_token(&mut app, 0, "Unselect cancellation");
        select_token(&mut app, 1, "Unselect cancellation");
        for &id in &cancelled {
            select_token(&mut app, id, "Select to cancel");
        }
        select(&mut app, Source::Token(cancelled[0]));
        activate(&mut app, "Inspect scenario reference");
        activate(&mut app, "Close · Esc");
        act(&mut app, Action::Dismiss);
        let world = app.world_mut();
        for (source, feedback) in world.query::<(&CardSource, &CardFeedback)>().iter(world) {
            if let Source::Token(id) = source.0 {
                assert!(!feedback.menu_source);
                assert_eq!(feedback.pending_choice, cancelled.contains(&id));
            }
        }
        assert_eq!(app.world().resource::<Table>().session, before);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        activate(&mut app, "Confirm selected cancellation");
        let after = app.world().resource::<Table>().view();
        let actual: Vec<_> = after
            .tokens
            .iter()
            .filter(|t| t.status == crate::TokenStatus::Canceled)
            .map(|t| t.id)
            .collect();
        assert_eq!(actual, cancelled);
        assert_eq!(continuity::load_session(&path).unwrap().observe(), after);
    }
}

fn keyboard_app(path: PathBuf) -> (App, Entity) {
    use bevy::{
        camera::{ComputedCameraValues, RenderTarget, RenderTargetInfo},
        input_focus::{InputDispatchPlugin, InputFocusPlugin},
        window::{PrimaryWindow, WindowRef},
    };
    let mut app = App::new();
    // The same input, UI and feedback plugins as native startup. FeedbackPlugin
    // owns tab navigation; the harness must not install a substitute for it.
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
        feedback::FeedbackPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ))
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<Time<Real>>()
    .init_resource::<Time>()
    .init_resource::<PresentationTime>();
    let mut app = render_app_in(app, path);
    app.add_systems(
        Update,
        camera_controls
            .after(handle_actions)
            .before(scene::synchronize),
    )
    .add_systems(
        Update,
        synchronize_feedback
            .after(scene::synchronize)
            .before(ui::render),
    )
    .add_systems(Update, controls::render.after(ui::render));
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    {
        let mut window = app.world_mut().get_mut::<Window>(window).unwrap();
        window.resolution = (1600, 1000).into();
        window.focused = true;
    }
    app.world_mut().entity_mut(window).insert(PrimaryWindow);
    app.world_mut().spawn((
        MainCamera,
        Transform::default(),
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1600, 1000),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        RenderTarget::Window(WindowRef::Entity(window)),
    ));
    app.update();
    (app, window)
}

fn key(app: &mut App, window: Entity, key_code: KeyCode, logical_key: bevy::input::keyboard::Key) {
    use bevy::input::{ButtonState, keyboard::KeyboardInput};
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: logical_key.clone(),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
    }
}

#[test]
fn party_keyboard_selection_captures_camera_and_preserves_pending_cards_through_inspection() {
    use bevy::input::keyboard::Key;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keyboard.json");
    let (mut app, window) = keyboard_app(path.clone());
    choose(&mut app, "select:jacqueline");
    choose(&mut app, "investigate");
    let before = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    let pose = app.world().resource::<Orbit>().pose();
    key(&mut app, window, KeyCode::KeyW, Key::Character("w".into()));
    let panned = app.world().resource::<Orbit>().pose();
    assert_ne!(
        panned.target, pose.target,
        "camera is active outside a menu"
    );
    for id in [100, 200] {
        select(&mut app, Source::Hand(id));
        let focused = app.world().resource::<InputFocus>().get().unwrap();
        assert_eq!(
            focused,
            rendered_button(
                &mut app,
                if id == 100 {
                    "Select to commit +4 intellect · add one curse"
                } else {
                    "Select to commit +1 intellect"
                }
            )
        );
        key(&mut app, window, KeyCode::KeyW, Key::Character("w".into()));
        key(&mut app, window, KeyCode::ArrowLeft, Key::ArrowLeft);
        assert_eq!(app.world().resource::<Orbit>().pose(), panned);
        key(&mut app, window, KeyCode::Enter, Key::Enter);
    }
    select(&mut app, Source::Hand(100));
    key(&mut app, window, KeyCode::Tab, Key::Tab);
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        Some(rendered_button(&mut app, "Inspect"))
    );
    key(&mut app, window, KeyCode::Space, Key::Space);
    assert_eq!(
        app.world().resource::<Table>().inspect,
        cards::index("card/07032")
    );
    key(&mut app, window, KeyCode::KeyW, Key::Character("w".into()));
    assert_eq!(app.world().resource::<Orbit>().pose(), panned);
    key(&mut app, window, KeyCode::Escape, Key::Escape);
    assert!(app.world().resource::<Table>().inspect.is_none());
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    activate(&mut app, "Commit selected · +5 intellect");
    assert_eq!(committed(&app), [100, 200]);
    assert_eq!(app.world().resource::<Orbit>().pose(), panned);
}
