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

        select_token(&mut app, 1, "Jacqueline: select to cancel");
        act(&mut app, Action::Confirm(revision, true));
        assert_eq!(
            app.world().resource::<Table>().session,
            before,
            "incomplete selection cannot commit"
        );
        select_token(&mut app, 0, "Jacqueline: select to cancel");
        act(&mut app, Action::Confirm(revision, true));
        assert_eq!(
            app.world().resource::<Table>().session,
            before,
            "mixed auto-fail selection cannot commit"
        );
        assert!(
            !visible_text(&mut app)
                .iter()
                .any(|text| text == "Jacqueline: confirm selected cancellation")
        );
        select_token(&mut app, 0, "Jacqueline: unselect cancellation");
        select_token(&mut app, 1, "Jacqueline: unselect cancellation");
        for &id in &cancelled {
            select_token(&mut app, id, "Jacqueline: select to cancel");
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
        activate(&mut app, "Jacqueline: confirm selected cancellation");
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

pub(super) fn production_app(path: PathBuf, health: RenderHealth) -> (App, Entity) {
    use bevy::{
        camera::{ComputedCameraValues, RenderTarget, RenderTargetInfo},
        input_focus::{InputDispatchPlugin, InputFocusPlugin},
        window::{PrimaryWindow, WindowRef},
    };
    let mut app = App::new();
    // Only the headless environment belongs here. Party owns all application
    // registration, including feedback/navigation, reveal, setup and redraw.
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
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
        bevy::ui::UiPlugin,
        bevy::ui_widgets::ButtonPlugin,
    ))
    .init_resource::<Assets<TextureAtlasLayout>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
    .init_resource::<Time<Real>>()
    .init_resource::<Time>()
    .init_resource::<PresentationTime>()
    .init_resource::<Art>()
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>();
    let mut app = with_table(app, path);
    app.insert_resource(health).insert_resource(Capture {
        path: None,
        quit: false,
        frame: 0,
        requested: false,
    });
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (1600, 1000).into(),
                focused: true,
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    install(&mut app);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(100));
    app.update();
    // Production setup creates the sole camera. Supply the metadata normally
    // computed by the renderer; do not replace the camera or its control system.
    let world = app.world_mut();
    let (mut camera, mut projection, mut target) = world
        .query_filtered::<(&mut Camera, &mut Projection, &mut RenderTarget), With<MainCamera>>()
        .single_mut(world)
        .unwrap();
    projection.update(1600., 1000.);
    camera.computed = ComputedCameraValues {
        clip_from_view: projection.get_clip_from_view(),
        target_info: Some(RenderTargetInfo {
            physical_size: UVec2::new(1600, 1000),
            scale_factor: 1.0,
        }),
        ..default()
    };
    *target = RenderTarget::Window(WindowRef::Entity(window));
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

fn right_drag(app: &mut App, window: Entity) {
    use bevy::input::{ButtonState, mouse::MouseButtonInput};
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(Vec2::new(1450., 850.)));
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Right,
        state: ButtonState::Pressed,
        window,
    });
    app.update();
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(Vec2::new(1510., 880.)));
    app.update();
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Right,
        state: ButtonState::Released,
        window,
    });
    app.update();
}

fn assert_hand_marks(app: &mut App, id: u16, source: bool, pending: bool) {
    let world = app.world_mut();
    let children = world
        .query::<(&CardSource, &Children)>()
        .iter(world)
        .find(|(source, _)| source.0 == Source::Hand(id))
        .unwrap_or_else(|| panic!("missing hand card {id}"))
        .1;
    for (name, visible) in [
        ("Card menu source", source),
        ("Card pending choice", pending),
    ] {
        let mark = children
            .iter()
            .find(|child| {
                world
                    .get::<Name>(*child)
                    .is_some_and(|n| n.as_str() == name)
            })
            .unwrap_or_else(|| panic!("missing {name} on hand card {id}"));
        assert_eq!(
            world.get::<InheritedVisibility>(mark).unwrap().get(),
            visible,
            "{name} on hand card {id}"
        );
        if visible {
            let material = world.get::<MeshMaterial3d<StandardMaterial>>(mark).unwrap();
            assert!(
                world
                    .resource::<Assets<StandardMaterial>>()
                    .get(material)
                    .unwrap()
                    .base_color
                    .alpha()
                    > 0.0
            );
        }
    }
}

#[test]
fn party_controls_preserve_staging_then_save_commitment_reveal_and_replay() {
    use bevy::input::keyboard::Key;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keyboard.json");
    let (mut app, window) = production_app(path.clone(), RenderHealth::ready_for_test());
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
        assert_hand_marks(&mut app, id, true, false);
        let focused = app.world().resource::<InputFocus>().get().unwrap();
        assert_eq!(
            focused,
            rendered_button(
                &mut app,
                if id == 100 {
                    "Patrice: select to commit +4 intellect · add one curse"
                } else {
                    "Jacqueline: select to commit +1 intellect"
                }
            )
        );
        key(&mut app, window, KeyCode::KeyW, Key::Character("w".into()));
        key(&mut app, window, KeyCode::ArrowLeft, Key::ArrowLeft);
        assert_eq!(app.world().resource::<Orbit>().pose(), panned);
        key(&mut app, window, KeyCode::Enter, Key::Enter);
        assert_hand_marks(&mut app, id, false, true);
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
    right_drag(&mut app, window);
    assert_eq!(app.world().resource::<Orbit>().pose(), panned);
    key(&mut app, window, KeyCode::Escape, Key::Escape);
    assert!(app.world().resource::<Table>().inspect.is_none());
    assert_hand_marks(&mut app, 100, false, true);
    select(&mut app, Source::Hand(100));
    assert_hand_marks(&mut app, 100, true, true);
    right_drag(&mut app, window);
    let orbited = app.world().resource::<Orbit>().pose();
    assert_ne!(
        orbited.yaw, panned.yaw,
        "the production camera adapter orbits"
    );
    assert_hand_marks(&mut app, 100, false, true);
    assert_hand_marks(&mut app, 200, false, true);
    assert!(
        app.world_mut()
            .query_filtered::<Entity, With<controls::Menu>>()
            .iter(app.world())
            .next()
            .is_none(),
        "camera navigation dismisses the contextual controls"
    );
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    activate(&mut app, "Commit selected · +5 intellect");
    assert_eq!(committed(&app), [100, 200]);
    let committed_view = app.world().resource::<Table>().view();
    assert_eq!(committed_view.revision, before.observe().revision + 1);
    assert_eq!(
        continuity::load_session(&path).unwrap().observe(),
        committed_view
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), orbited);

    let saved_commitment = std::fs::read(&path).unwrap();
    activate(&mut app, "Reveal chaos token");
    let reveal_choice = rendered_button(&mut app, "Reveal 1 · decline Jacqueline for this reveal");
    key(&mut app, window, KeyCode::ArrowLeft, Key::ArrowLeft);
    assert_eq!(app.world().resource::<Orbit>().pose(), orbited);
    assert_eq!(app.world().resource::<Table>().view(), committed_view);
    assert_eq!(std::fs::read(&path).unwrap(), saved_commitment);
    app.world_mut().trigger(Activate {
        entity: reveal_choice,
    });
    app.world_mut().trigger(Activate {
        entity: reveal_choice,
    });
    app.update();
    let revealed = app.world().resource::<Table>().view();
    assert_eq!(revealed.revision, committed_view.revision + 1);
    assert_eq!(revealed.tokens.len(), 1);
    assert!(!revealed.jacqueline_used);
    assert_eq!(continuity::load_session(&path).unwrap().observe(), revealed);

    let live = app.world().resource::<Table>().session.clone();
    let saved_reveal = std::fs::read(&path).unwrap();
    activate(&mut app, "Menu");
    activate(&mut app, "Open recorded replay");
    let first = app.world().resource::<Table>().view();
    activate(&mut app, "Next moment");
    assert_ne!(app.world().resource::<Table>().view(), first);
    activate(&mut app, "Previous moment");
    assert_eq!(app.world().resource::<Table>().view(), first);
    assert_eq!(app.world().resource::<Table>().session, live);
    assert_eq!(std::fs::read(&path).unwrap(), saved_reveal);
    activate(&mut app, "Return to live attempt");
    assert_eq!(app.world().resource::<Table>().view(), revealed);
    assert_eq!(app.world().resource::<Table>().session, live);
    assert_eq!(std::fs::read(&path).unwrap(), saved_reveal);
}

#[test]
fn party_installed_controls_discard_gameplay_input_until_rendering_is_ready() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blocked.json");
    let (mut app, _) = production_app(path.clone(), RenderHealth::ready_for_test());
    choose(&mut app, "select:jacqueline");
    choose(&mut app, "investigate");
    let before = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    let label = before
        .observe()
        .choices
        .iter()
        .find(|c| c.id == "commit:none")
        .unwrap()
        .label
        .clone();
    for health in [
        RenderHealth::default(),
        RenderHealth::just_ready_for_test(),
        RenderHealth::failed_for_test(),
    ] {
        app.insert_resource(health);
        activate(&mut app, &label);
        assert_eq!(app.world().resource::<Table>().session, before);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        app.insert_resource(RenderHealth::ready_for_test());
        app.update();
        assert_eq!(
            app.world().resource::<Table>().session,
            before,
            "blocked input must be drained"
        );
        assert_eq!(std::fs::read(&path).unwrap(), saved);
    }
    activate(&mut app, &label);
    assert_eq!(
        app.world().resource::<Table>().view().revision,
        before.observe().revision + 1
    );
    assert_eq!(
        continuity::load_session(&path).unwrap().observe(),
        app.world().resource::<Table>().view()
    );
}

#[test]
fn turn_indicator_stays_with_tony_while_jacqueline_helps_after_her_turn_ends() {
    for with_portrait in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("party.json");
        let (mut app, _) = production_app(path.clone(), RenderHealth::ready_for_test());
        // This path uses Jacqueline's three-token reveal; the ordinary UI
        // fixture supplies only one deterministic draw.
        app.world_mut().resource_mut::<Table>().session = Session::with_draws(vec![
            crate::TokenFace::Zero,
            crate::TokenFace::MinusOne,
            crate::TokenFace::MinusTwo,
        ]);
        if with_portrait {
            let image = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .add(Image::default());
            for code in ["06003", "06005", "60401"] {
                app.world_mut()
                    .resource_mut::<Art>()
                    .handles
                    .insert(format!("standee/{code}"), image.clone());
            }
            app.update();
        }
        let assert_turn = |app: &mut App, who: Option<Investigator>| {
            let labels: Vec<_> = app
                .world_mut()
                .query::<&AccessibleLabel>()
                .iter(app.world())
                .map(|label| label.0.clone())
                .filter(|label| label.ends_with(" · current turn"))
                .collect();
            assert_eq!(
                labels,
                who.map(|who| format!("Focus {} · current turn", who.name()))
                    .into_iter()
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                visible_text(app)
                    .iter()
                    .filter(|text| text.as_str() == "TURN")
                    .count(),
                usize::from(who.is_some())
            );
        };
        assert_turn(&mut app, None);
        choose(&mut app, "select:jacqueline");
        assert_turn(&mut app, Some(Investigator::Jacqueline));
        activate(&mut app, "End Jacqueline's turn");
        assert_turn(&mut app, None);
        choose(&mut app, "select:tony");
        assert_turn(&mut app, Some(Investigator::Tony));
        assert!(
            visible_text(&mut app)
                .iter()
                .any(|text| text == "Tony's turn")
        );

        let before = app.world().resource::<Table>().session.clone();
        let focus = app
            .world_mut()
            .query::<(Entity, &AccessibleLabel)>()
            .iter(app.world())
            .find(|(_, label)| label.0 == "Focus Jacqueline Fine")
            .unwrap()
            .0;
        app.world_mut().trigger(Activate { entity: focus });
        app.update();
        assert_eq!(app.world().resource::<Table>().session, before);
        assert_turn(&mut app, Some(Investigator::Tony));
        choose(&mut app, "investigate");
        select(&mut app, Source::Hand(200));
        activate(&mut app, "Jacqueline: select to commit +1 intellect");
        activate(&mut app, "Commit selected · +1 intellect");
        assert!(
            app.world()
                .resource::<Table>()
                .view()
                .investigator(Investigator::Jacqueline)
                .turn_finished
        );
        assert_eq!(
            app.world()
                .resource::<Table>()
                .view()
                .test
                .unwrap()
                .committed[0]
                .owner,
            Investigator::Jacqueline
        );
        assert_turn(&mut app, Some(Investigator::Tony));

        select(&mut app, Source::Card(cards::index("card/60401").unwrap()));
        activate(
            &mut app,
            "Jacqueline: use ability · reveal three, then choose cancellations",
        );
        let live = app.world().resource::<Table>().view();
        assert_eq!(
            live.stage,
            crate::party::Stage::Cancellation,
            "{}",
            app.world().resource::<Table>().notice
        );
        assert_eq!(live.actor, Some(Investigator::Tony));
        assert!(live.jacqueline_used);
        assert_eq!(continuity::load_session(&path).unwrap().observe(), live);
        assert_turn(&mut app, Some(Investigator::Tony));
        act(&mut app, Action::Reopen);
        assert_turn(&mut app, Some(Investigator::Tony));

        act(&mut app, Action::Replay);
        assert_turn(&mut app, None);
        act(&mut app, Action::Next);
        assert_turn(&mut app, Some(Investigator::Jacqueline));
        assert!(
            visible_text(&mut app).iter().any(|text| text
                == &crate::tabletop::readable_text("Recorded · Jacqueline's turn".into()))
        );
        let moments = app
            .world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .len();
        for _ in 0..moments {
            if app.world().resource::<Table>().view().stage == crate::party::Stage::Cancellation {
                break;
            }
            act(&mut app, Action::Next);
        }
        assert_eq!(
            app.world().resource::<Table>().view().stage,
            crate::party::Stage::Cancellation
        );
        assert_turn(&mut app, Some(Investigator::Tony));
        assert!(
            visible_text(&mut app).iter().any(
                |text| text == &crate::tabletop::readable_text("Recorded · Tony's turn".into())
            )
        );
        assert_eq!(app.world().resource::<Table>().session.observe(), live);
    }
}
