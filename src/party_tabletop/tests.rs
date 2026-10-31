use super::*;
fn app(path: PathBuf) -> App {
    let mut app = App::new();
    app.insert_resource(Table::new(
        Session::with_draws(vec![crate::TokenFace::Zero]),
        path.clone(),
    ))
    .init_resource::<Actions>()
    .insert_resource(RenderHealth::ready_for_test())
    .insert_resource(Orbit::new(CameraMode::Jacqueline))
    .insert_resource(AssetSettings::new_party(AssetConfig::load_at(
        path.with_extension("toml"),
        path.with_extension("assets"),
    )))
    .init_resource::<ButtonInput<KeyCode>>()
    .add_plugins(reveal::RevealPlugin)
    .add_systems(Update, handle_actions);
    app
}
fn act(app: &mut App, action: Action) {
    app.world_mut().resource_mut::<Actions>().0.push(action);
    app.update();
}
fn choose(app: &mut App, id: &str) {
    let revision = app.world().resource::<Table>().view().revision;
    act(app, Action::Choose(revision, id.into()));
}
#[test]
fn rendering_check_blocks_party_decisions_without_creating_a_record() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let mut app = app(path.clone());
    app.insert_resource(RenderHealth::default());
    let before = app.world().resource::<Table>().session.clone();
    choose(&mut app, "select:jacqueline");
    act(&mut app, Action::Save);
    act(&mut app, Action::Replay);
    assert_eq!(app.world().resource::<Table>().session, before);
    assert!(app.world().resource::<Table>().replay.is_none());
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

    app.insert_resource(RenderHealth::ready_for_test());
    choose(&mut app, "select:jacqueline");
    assert_eq!(
        continuity::load_session(&path).unwrap().observe().actor,
        Some(Investigator::Jacqueline)
    );
}

#[test]
fn rendering_health_preserves_party_records_and_drains_blocked_input() {
    for health in [
        RenderHealth::default(),
        RenderHealth::failed_for_test(),
        RenderHealth::just_ready_for_test(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("party.json");
        let mut app = app(path.clone());
        act(&mut app, Action::Save);
        let saved = std::fs::read(&path).unwrap();
        choose(&mut app, "select:jacqueline");
        let before = app.world().resource::<Table>().session.clone();
        // A different valid save makes a blocked Reopen observable too.
        std::fs::write(&path, &saved).unwrap();
        let file_count = std::fs::read_dir(dir.path()).unwrap().count();
        app.insert_resource(health);
        choose(&mut app, "investigate");
        for action in [Action::Save, Action::Reopen, Action::Replay, Action::New] {
            act(&mut app, action);
        }
        let camera_before = app.world().resource::<Orbit>().pose();
        act(&mut app, Action::Focus(Area::Test));
        assert_ne!(app.world().resource::<Orbit>().pose(), camera_before);
        assert_eq!(app.world().resource::<Table>().session, before);
        assert!(app.world().resource::<Table>().replay.is_none());
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), file_count);
        assert!(app.world().resource::<Actions>().0.is_empty());

        app.insert_resource(RenderHealth::ready_for_test());
        app.update();
        assert_eq!(app.world().resource::<Table>().session, before);
        choose(&mut app, "investigate");
        assert_ne!(app.world().resource::<Table>().session, before);
        assert_eq!(
            continuity::load_session(&path).unwrap(),
            app.world().resource::<Table>().session
        );
    }
}

#[test]
fn rendering_failure_blocks_party_replay_navigation_and_return_to_live() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let mut app = app(path.clone());
    choose(&mut app, "select:jacqueline");
    act(&mut app, Action::Replay);
    act(&mut app, Action::Next);
    let index = app
        .world()
        .resource::<Table>()
        .replay
        .as_ref()
        .unwrap()
        .index();
    let saved = std::fs::read(&path).unwrap();
    app.insert_resource(RenderHealth::failed_for_test());
    for action in [Action::Back, Action::Next, Action::Live] {
        act(&mut app, action);
        assert_eq!(
            app.world()
                .resource::<Table>()
                .replay
                .as_ref()
                .unwrap()
                .index(),
            index
        );
    }
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn rendering_check_cannot_archive_and_restart_a_completed_party_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let mut app = app(path.clone());
    app.world_mut().resource_mut::<Table>().session = completed_attempt(None);
    act(&mut app, Action::Save);
    let before = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    let file_count = std::fs::read_dir(dir.path()).unwrap().count();
    for health in [RenderHealth::default(), RenderHealth::failed_for_test()] {
        app.insert_resource(health);
        act(&mut app, Action::New);
        assert_eq!(app.world().resource::<Table>().session, before);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), file_count);
    }
    app.insert_resource(RenderHealth::ready_for_test());
    act(&mut app, Action::New);
    assert!(app.world().resource::<Table>().view().endpoint.is_none());
    assert_eq!(
        continuity::load_session(&path).unwrap(),
        app.world().resource::<Table>().session
    );
}

#[test]
fn screenshot_capture_and_successful_exit_wait_for_rendering_admission() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.png");
    let mut app = App::new();
    app.insert_resource(RenderHealth::default())
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_resource(Capture {
            path: Some(path.clone()),
            quit: true,
            frame: 90,
            requested: false,
        })
        .add_message::<AppExit>()
        .add_systems(Update, capture_window);
    app.update();
    assert_eq!(app.world().resource::<Capture>().frame, 90);
    assert!(!app.world().resource::<Capture>().requested);
    assert_eq!(
        app.world_mut()
            .query::<&Screenshot>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(app.world().resource::<Messages<AppExit>>().is_empty());

    app.insert_resource(RenderHealth::just_ready_for_test());
    app.update();
    assert!(!app.world().resource::<Capture>().requested);

    app.insert_resource(RenderHealth::ready_for_test());
    app.update();
    assert!(app.world().resource::<Capture>().requested);
    assert_eq!(
        app.world_mut()
            .query::<&Screenshot>()
            .iter(app.world())
            .count(),
        1
    );

    std::fs::write(&path, "completed capture").unwrap();
    app.world_mut().resource_mut::<Capture>().frame = 96;
    app.insert_resource(RenderHealth::failed_for_test());
    app.update();
    assert!(app.world().resource::<Messages<AppExit>>().is_empty());
}

#[test]
fn native_choice_saves_before_install_and_camera_focus_never_selects_actor() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let mut app = app(path.clone());
    let before = app.world().resource::<Table>().view();
    act(&mut app, Action::Focus(Area::Investigator(1)));
    assert_eq!(app.world().resource::<Table>().view(), before);
    choose(&mut app, "select:jacqueline");
    let after = app.world().resource::<Table>().view();
    assert_eq!(after.actor, Some(Investigator::Jacqueline));
    assert_eq!(continuity::load_session(&path).unwrap().observe(), after);
}
#[test]
fn failed_native_choice_and_replay_preserve_live_session_and_camera() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let mut app = app(path.clone());
    choose(&mut app, "select:jacqueline");
    let before = app.world().resource::<Table>().session.clone();
    let pose = app.world().resource::<Orbit>().pose();
    app.world_mut().resource_mut::<Table>().save_path = dir.path().into();
    choose(&mut app, "investigate");
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(app.world().resource::<Orbit>().pose(), pose);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .contains("not committed")
    );
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    act(&mut app, Action::Replay);
    let bytes = std::fs::read(&path).unwrap();
    assert!(app.world().resource::<Table>().replay.is_some());
    choose(&mut app, "investigate");
    act(&mut app, Action::Save);
    act(&mut app, Action::Next);
    act(&mut app, Action::Back);
    act(&mut app, Action::Live);
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
#[test]
fn inspection_preserves_selected_commitments_and_revision_change_invalidates_them() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("party.json"));
    choose(&mut app, "select:jacqueline");
    choose(&mut app, "investigate");
    let view = app.world().resource::<Table>().view();
    let revision = view.revision;
    act(
        &mut app,
        Action::Select(revision, Source::Hand(100), Vec3::ZERO),
    );
    let epoch = app.world().resource::<Table>().context.epoch;
    act(&mut app, Action::Toggle(epoch, revision, 100, false));
    act(
        &mut app,
        Action::Inspect(cards::index("card/07032").unwrap()),
    );
    choose(&mut app, "commit:none");
    assert_eq!(app.world().resource::<Table>().view(), view);
    act(&mut app, Action::Close);
    assert!(
        app.world()
            .resource::<Table>()
            .context
            .commitments
            .contains(&100)
    );
    act(&mut app, Action::Confirm(revision, false));
    assert!(
        app.world()
            .resource::<Table>()
            .context
            .commitments
            .is_empty()
    );
    assert_eq!(
        app.world()
            .resource::<Table>()
            .view()
            .test
            .as_ref()
            .unwrap()
            .skill_value,
        7
    );
}

#[test]
fn physical_scene_represents_all_three_hands_and_uses_shared_standees() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("party.json"));
    app.init_resource::<Art>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_systems(Update, scene::synchronize.after(handle_actions));
    app.update();
    let world = app.world_mut();
    let hands: Vec<_> = world
        .query::<&CardSource>()
        .iter(world)
        .filter_map(|source| {
            if let Source::Hand(id) = source.0 {
                Some(id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(hands, vec![0, 100, 200]);
    crate::tabletop_standee_checks::assert_standees(
        world,
        &[
            (layout::InvestigatorClass::Rogue, None),
            (layout::InvestigatorClass::Survivor, None),
            (layout::InvestigatorClass::Mystic, None),
        ],
    );
}

#[test]
fn rendered_reveal_shortcut_reaches_the_saved_reaction_choice() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("party.json"));
    app.add_systems(
        Update,
        (sync_reveal, reveal::render).chain().after(handle_actions),
    );
    choose(&mut app, "select:jacqueline");
    choose(&mut app, "investigate");
    choose(&mut app, "commit:none");
    let before = app.world().resource::<Table>().session.clone();
    let button = rendered_button(&mut app, "Reveal chaos token");
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(app.world().resource::<reveal::Shortcut>().is_open());
    assert_eq!(app.world().resource::<Table>().session, before);
    let normal = rendered_button(&mut app, "Reveal 1 · decline Jacqueline for this reveal");
    app.world_mut().trigger(Activate { entity: normal });
    app.world_mut().trigger(Activate { entity: normal });
    app.update();
    let table = app.world().resource::<Table>();
    assert_eq!(table.view().revision, before.observe().revision + 1);
    assert_eq!(table.view().tokens.len(), 1);
    assert!(!table.view().jacqueline_used);
    assert_eq!(
        continuity::load_session(&table.save_path)
            .unwrap()
            .observe(),
        table.view()
    );
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        layout::TEST_CENTER
    );
}

fn rendered_button(app: &mut App, label: &str) -> Entity {
    let rendered = crate::tabletop::readable_text(label.to_owned());
    let world = app.world_mut();
    world
        .query_filtered::<(Entity, &Children), With<Button>>()
        .iter(world)
        .find_map(|(entity, children)| {
            children
                .iter()
                .any(|child| {
                    world
                        .get::<Text>(child)
                        .is_some_and(|text| text.0 == rendered)
                })
                .then_some(entity)
        })
        .unwrap_or_else(|| panic!("missing rendered button: {label}"))
}

fn render_app(path: PathBuf) -> App {
    let mut app = app(path);
    app.init_resource::<Art>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_systems(
            Update,
            (scene::synchronize, ui::render)
                .chain()
                .after(handle_actions),
        );
    app.world_mut().spawn(Window::default());
    app
}

fn visible_text(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect()
}

fn completed_attempt(advance: Option<bool>) -> Session {
    let mut session = Session::with_draws(vec![crate::TokenFace::Zero; 6]);
    if let Some(advance) = advance {
        for actor in ["tony", "jacqueline"] {
            session
                .choose(session.observe().revision, &format!("select:{actor}"))
                .unwrap();
            for _ in 0..3 {
                for id in [
                    "investigate",
                    "commit:none",
                    "reveal",
                    "resolve:0",
                    "finish-test",
                ] {
                    session.choose(session.observe().revision, id).unwrap();
                }
            }
            if actor == "tony" {
                session
                    .choose(session.observe().revision, "end-turn")
                    .unwrap();
            }
        }
        session
            .choose(
                session.observe().revision,
                if advance {
                    "advance"
                } else {
                    "decline-advance"
                },
            )
            .unwrap();
    } else {
        for actor in ["tony", "patrice", "jacqueline"] {
            session
                .choose(session.observe().revision, &format!("select:{actor}"))
                .unwrap();
            session
                .choose(session.observe().revision, "end-turn")
                .unwrap();
        }
    }
    session
}

#[test]
fn substitutes_name_the_book_assets_and_each_honest_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = render_app(dir.path().join("party.json"));
    app.update();
    let text = visible_text(&mut app).join("\n");
    for expected in [
        "Trapped",
        "Act 1",
        "What's Going On?!",
        "Agenda 1",
        "Bounty Contracts",
        "Arcane Research",
    ] {
        assert!(
            text.contains(expected),
            "missing readable substitute: {expected}"
        );
    }
    for (advance, expected) in [
        (Some(true), "Escaped the Study"),
        (Some(false), "Finished without advancing"),
        (None, "All three turns ended"),
    ] {
        let mut app = render_app(dir.path().join(format!("result-{advance:?}.json")));
        app.world_mut().resource_mut::<Table>().session = completed_attempt(advance);
        app.update();
        let text = visible_text(&mut app).join("\n");
        assert!(
            text.contains(expected),
            "missing explicit outcome: {expected}"
        );
        if advance == Some(true) {
            assert!(text.contains("The Barrier") && text.contains("Act 2"));
        }
    }
}

fn settle_camera(app: &mut App) -> Vec3 {
    let keys = ButtonInput::default();
    let buttons = ButtonInput::default();
    let window = Window::default();
    let mut orbit = app.world_mut().resource_mut::<Orbit>();
    orbit.step(FrameInput {
        keys: &keys,
        buttons: &buttons,
        window: &window,
        scroll_y: 0.,
        dt: 1.,
        capture: CameraCapture::default(),
        reduced_motion: true,
    });
    orbit.pose().target
}

#[test]
fn replay_follows_current_actions_after_tests_and_preserves_manual_override() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("party.json"));
    for id in [
        "select:tony",
        "investigate",
        "commit:none",
        "reveal",
        "resolve:0",
        "finish-test",
        "draw",
        "resource",
        "end-turn",
        "select:patrice",
    ] {
        choose(&mut app, id);
    }
    act(&mut app, Action::Replay);
    for _ in 0..4 {
        act(&mut app, Action::Next);
    }
    assert_eq!(
        settle_camera(&mut app),
        layout::TEST_CENTER,
        "actual revealed test focuses Test"
    );
    for _ in 0..3 {
        act(&mut app, Action::Next);
    }
    assert_eq!(
        app.world()
            .resource::<Table>()
            .view()
            .investigator(Investigator::Tony)
            .hand
            .len(),
        2
    );
    assert_eq!(
        settle_camera(&mut app),
        layout::focus(Area::Investigator(0)).0,
        "draw after test focuses Tony"
    );
    act(&mut app, Action::Next);
    assert_eq!(
        settle_camera(&mut app),
        layout::focus(Area::Investigator(0)).0,
        "resource after test focuses Tony"
    );
    act(&mut app, Action::Next);
    act(&mut app, Action::Next);
    assert_eq!(
        settle_camera(&mut app),
        layout::focus(Area::Investigator(1)).0,
        "new actor focuses Patrice"
    );
    act(&mut app, Action::Focus(Area::Scenario));
    act(&mut app, Action::Back);
    assert_eq!(
        settle_camera(&mut app),
        layout::focus(Area::Scenario).0,
        "manual override survives navigation"
    );
}

#[test]
fn replay_focuses_patrice_discard_for_the_after_test_shuffle() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path().join("shuffle.json"));
    app.world_mut().resource_mut::<Table>().session = Session::with_draws(vec![
        crate::TokenFace::Zero,
        crate::TokenFace::ElderSign,
        crate::TokenFace::MinusFour,
        crate::TokenFace::AutoFail,
    ]);
    for id in [
        "select:tony",
        "investigate",
        "commit:100",
        "reveal",
        "resolve:0",
        "finish-test",
        "end-turn",
        "select:patrice",
        "draw",
        "resource",
        "investigate",
        "commit:none",
        "use-jacqueline",
        "cancel:2",
        "resolve:0",
        "resolve:1",
        "lucky:101",
        "finish-test",
        "shuffle:retain:100",
    ] {
        choose(&mut app, id);
    }
    act(&mut app, Action::Replay);
    while app.world().resource::<Table>().view().stage != crate::party::Stage::AfterTest {
        let replay = app.world().resource::<Table>().replay.as_ref().unwrap();
        assert!(replay.index() + 1 < replay.len());
        act(&mut app, Action::Next);
    }
    let discard = Vec3::new(3.9, 0., 11.1);
    assert_eq!(settle_camera(&mut app), discard);
    act(&mut app, Action::Next);
    assert_eq!(
        settle_camera(&mut app),
        discard,
        "the recorded shuffle result stays at its affected discard"
    );
}

#[test]
fn discard_substitute_names_the_top_card() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = render_app(dir.path().join("cards.json"));
    for id in [
        "select:jacqueline",
        "investigate",
        "commit:100",
        "reveal",
        "resolve:0",
        "finish-test",
    ] {
        choose(&mut app, id);
    }
    let world = app.world_mut();
    assert!(
        world
            .query::<&CardSource>()
            .iter(world)
            .any(|source| source.0 == Source::Discard(Investigator::Patrice))
    );
    assert!(
        visible_text(&mut app)
            .iter()
            .any(|text| text.contains("Discard") && text.contains("Promise of Power"))
    );
}

#[test]
fn settled_result_text_survives_reopen_and_replay_until_the_next_action() {
    for (token, total, outcome) in [
        (crate::TokenFace::Zero, 3, "Success"),
        (crate::TokenFace::MinusTwo, 1, "Failure"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut app = render_app(dir.path().join("result.json"));
        app.world_mut().resource_mut::<Table>().session = Session::with_draws(vec![token]);
        for id in [
            "select:tony",
            "investigate",
            "commit:none",
            "reveal",
            "resolve:0",
            "finish-test",
        ] {
            choose(&mut app, id);
        }
        let expected = format!("final value {total}");
        let assert_result = |app: &mut App| {
            let text = visible_text(app).join("\n");
            assert!(
                text.contains(&expected) && text.contains(outcome),
                "settled result not visible: {text}"
            );
        };
        assert_result(&mut app);
        act(&mut app, Action::Reopen);
        assert_result(&mut app);
        choose(&mut app, "resource");
        assert!(!visible_text(&mut app).join("\n").contains(&expected));
        act(&mut app, Action::Replay);
        while app.world().resource::<Table>().view().stage != crate::party::Stage::Result {
            let replay = app.world().resource::<Table>().replay.as_ref().unwrap();
            assert!(replay.index() + 1 < replay.len());
            act(&mut app, Action::Next);
        }
        act(&mut app, Action::Next);
        assert_result(&mut app);
        act(&mut app, Action::Next);
        assert!(!visible_text(&mut app).join("\n").contains(&expected));
    }
}

#[test]
fn sixth_clue_result_is_visible_beside_the_closing_decision() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = render_app(dir.path().join("closing.json"));
    app.world_mut().resource_mut::<Table>().session =
        Session::with_draws(vec![crate::TokenFace::Zero; 6]);
    for actor in ["tony", "jacqueline"] {
        choose(&mut app, &format!("select:{actor}"));
        for _ in 0..3 {
            for id in [
                "investigate",
                "commit:none",
                "reveal",
                "resolve:0",
                "finish-test",
            ] {
                choose(&mut app, id);
            }
        }
        if actor == "tony" {
            choose(&mut app, "end-turn");
        }
    }
    assert_eq!(
        app.world().resource::<Table>().view().stage,
        crate::party::Stage::Closing
    );
    let text = visible_text(&mut app).join("\n");
    assert!(text.contains("final value 3") && text.contains("Success"));
    choose(&mut app, "advance");
    assert!(
        visible_text(&mut app)
            .join("\n")
            .contains("Escaped the Study")
    );
}
