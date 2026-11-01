use super::*;
mod act_reveal;
mod clue_motion;
mod counters;
mod location_map;
mod selection_lifecycle;
mod skill_summary;

#[test]
fn native_image_roles_include_all_counter_recipe_assets() {
    crate::assets::assert_counter_recipe_roles_loaded(
        include_str!("../../resources/party-asset-selection.json"),
        image_asset_roles(),
    );
}

#[test]
fn legacy_result_continues_after_render_admission_without_a_finish_click() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let original = include_bytes!("../../tests/fixtures/party-result-ef6daf2.session.json");
    std::fs::write(&path, original).unwrap();
    let (mut app, _) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    app.insert_resource(Table::new(
        continuity::load_session(&path).unwrap(),
        path.clone(),
    ));
    app.insert_resource(RenderHealth::default());
    app.update();
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(
        app.world().resource::<Table>().view().stage,
        crate::party::Stage::Result
    );
    app.insert_resource(RenderHealth::ready_for_test());
    app.update();
    let view = continuity::load_session(&path).unwrap().observe();
    assert_eq!(view.stage, crate::party::Stage::Action);
    assert_eq!(view.bag_count, 16);
    assert!(!view.can_undo);
    assert_eq!(view, app.world().resource::<Table>().view());
}

#[test]
fn legacy_automatic_resume_preserves_failure_and_replay_without_retrying_each_frame() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let original = include_bytes!("../../tests/fixtures/party-result-ef6daf2.session.json");
    std::fs::write(&path, original).unwrap();
    let legacy = continuity::load_session(&path).unwrap();
    let unusable = directory.path().join("directory");
    std::fs::create_dir(&unusable).unwrap();
    let (mut app, _) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    app.insert_resource(Table::new(legacy.clone(), unusable));
    app.update();
    assert_eq!(app.world().resource::<Table>().session, legacy);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .contains("not committed")
    );
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    app.update();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "save failure must not become an unattended retry loop"
    );
    let mut table = Table::new(legacy.clone(), path.clone());
    let mut replay = Replay::new(legacy.recording()).unwrap();
    while replay.next() {}
    table.replay = Some(replay);
    app.insert_resource(table);
    app.update();
    assert_eq!(
        app.world().resource::<Table>().view().stage,
        crate::party::Stage::Result
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(app.world().resource::<Table>().session, legacy);
}

#[test]
fn installed_token_return_clears_physical_tokens_and_sends_resolved_curse_to_pool() {
    let directory = tempfile::tempdir().unwrap();
    for reduced_motion in [false, true] {
        let path = directory
            .path()
            .join(format!("return-{reduced_motion}.json"));
        let (mut app, _) =
            selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
        let mut session = Session::with_draws(vec![
            crate::TokenFace::AutoFail,
            crate::TokenFace::Curse,
            crate::TokenFace::Zero,
            crate::TokenFace::MinusTwo,
        ]);
        for id in [
            "select:tony",
            "investigate",
            "commit:100",
            "use-jacqueline",
            "cancel:0",
            "resolve:1",
            "reveal",
            "resolve:3",
        ] {
            session.choose(session.observe().revision, id).unwrap();
        }
        {
            let mut table = app.world_mut().resource_mut::<Table>();
            table.session = session;
            table.reduced_motion = reduced_motion;
            table.invalidate();
        }
        app.update();
        choose(&mut app, "resolve:2");
        assert_eq!(
            continuity::load_session(&path).unwrap().observe().bag_count,
            16
        );
        assert_eq!(app.world().resource::<Table>().view().curses_in_pool, 10);
        assert!(
            app.world()
                .resource::<Table>()
                .view()
                .held_tokens()
                .is_empty()
        );
        {
            let world = app.world_mut();
            let sources: Vec<_> = world
                .query::<&CardSource>()
                .iter(world)
                .map(|source| source.0)
                .collect();
            assert!(
                !sources
                    .iter()
                    .any(|source| matches!(source, Source::Token(_)))
            );
        }
        if !reduced_motion {
            let world = app.world_mut();
            let destinations: Vec<_> = world
                .query::<&ReturningToken>()
                .iter(world)
                .map(|token| token.destination)
                .collect();
            assert_eq!(
                destinations
                    .iter()
                    .filter(|&&at| at == scene::curse_pool_at())
                    .count(),
                1
            );
            assert_eq!(
                destinations
                    .iter()
                    .filter(|&&at| at == layout::test_bag())
                    .count(),
                3
            );
            let before = world.resource::<Table>().session.clone();
            choose(&mut app, "resource");
            assert_eq!(
                app.world().resource::<Table>().session,
                before,
                "next action waits for token transfer"
            );
            for _ in 0..8 {
                app.world_mut()
                    .resource_mut::<PresentationTime>()
                    .advance_by(std::time::Duration::from_millis(100));
                app.update();
            }
        }
        let world = app.world_mut();
        assert_eq!(world.query::<&ReturningToken>().iter(world).count(), 0);
        choose(&mut app, "resource");
        assert_eq!(
            app.world()
                .resource::<Table>()
                .view()
                .investigator(Investigator::Tony)
                .resources,
            1
        );
        act(&mut app, Action::Reopen);
        let world = app.world_mut();
        assert_eq!(world.query::<&ReturningToken>().iter(world).count(), 0);
        assert_eq!(world.resource::<Table>().view().bag_count, 16);
    }
}

#[test]
fn production_test_heading_clears_surfaces_after_text_layout() {
    use crate::tabletop_layout::heading_checks;

    let directory = tempfile::tempdir().unwrap();
    for scale in [1.0, 2.0] {
        let mut app = with_table(
            heading_checks::app(),
            directory.path().join(format!("heading-{scale}.json")),
        );
        app.init_resource::<Art>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, (scene::synchronize, world_labels).chain());
        heading_checks::assert_heading(&mut app, MainCamera, scale);
    }
}

#[test]
fn production_harm_captions_clear_cards_and_each_other_after_text_layout() {
    use crate::tabletop_layout::heading_checks;

    let directory = tempfile::tempdir().unwrap();
    for size in [Vec2::new(1280.0, 720.0), Vec2::new(1600.0, 1000.0)] {
        for scale in [1.0, 2.0] {
            for artwork in [false, true] {
                let (mut app, _) = selection_lifecycle::production_app(
                    directory
                        .path()
                        .join(format!("captions-{}-{scale}-{artwork}.json", size.x)),
                    RenderHealth::ready_for_test(),
                );
                if artwork {
                    // Only artwork presence is needed to select numeric captions.
                    // Caption font/layout and production registration remain real.
                    for role in ["token/damage", "token/horror"] {
                        app.world_mut()
                            .resource_mut::<Art>()
                            .handles
                            .insert(role.into(), Handle::default());
                    }
                }
                {
                    let mut table = app.world_mut().resource_mut::<Table>();
                    table.reduced_motion = true;
                }
                // A normal decision rebuilds the scene with the selected artwork branch.
                choose(&mut app, "select:tony");
                heading_checks::viewport::<MainCamera>(&mut app, size, scale);
                for who in [
                    Investigator::Tony,
                    Investigator::Patrice,
                    Investigator::Jacqueline,
                ] {
                    act(&mut app, Action::Focus(Area::Investigator(who.index())));
                    // Camera/label projection, real text layout, then UI transforms.
                    for _ in 0..5 {
                        app.update();
                    }
                    let view = app.world().resource::<Table>().view();
                    let investigator = view.investigator(who);
                    let world = app.world_mut();
                    let card = world
                        .query::<(Entity, &Name)>()
                        .iter(world)
                        .find(|(_, name)| name.as_str() == who.name())
                        .unwrap()
                        .0;
                    let card_x = world.get::<GlobalTransform>(card).unwrap().translation().x;
                    let captions = [
                        ("Damage", investigator.damage, investigator.health),
                        ("Horror", investigator.horror, investigator.sanity),
                    ]
                    .map(|(title, amount, capacity)| {
                        let expected = if artwork {
                            format!("{amount}/{capacity}")
                        } else {
                            format!("{title} {amount}/{capacity}")
                        };
                        let matches: Vec<_> = world
                            .query::<(Entity, &WorldLabel, &Text)>()
                            .iter(world)
                            .filter(|(_, anchor, text)| {
                                text.0 == expected
                                    && (anchor.0.x - card_x).abs() < 2.0
                                    && (anchor.0.x < card_x) == (title == "Damage")
                            })
                            .map(|(entity, _, _)| entity)
                            .collect();
                        assert_eq!(
                            matches.len(),
                            1,
                            "exactly one {expected} caption for {}",
                            who.name()
                        );
                        matches[0]
                    });
                    heading_checks::assert_harm_captions::<MainCamera>(
                        &mut app, captions, card, size, scale,
                    );
                }
            }
        }
    }
}

fn app(path: PathBuf) -> App {
    app_in(App::new(), path)
}
fn app_in(app: App, path: PathBuf) -> App {
    let mut app = with_table(app, path);
    // These choice-only tests do not install presentation clocks or animation.
    // Native motion is exercised through production_app in the motion tests.
    app.world_mut().resource_mut::<Table>().reduced_motion = true;
    app.init_resource::<Actions>()
        .add_plugins(reveal::RevealPlugin)
        .add_systems(Update, handle_actions);
    app
}
fn with_table(mut app: App, path: PathBuf) -> App {
    app.insert_resource(Table::new(
        Session::with_draws(vec![crate::TokenFace::Zero]),
        path.clone(),
    ))
    .insert_resource(RenderHealth::ready_for_test())
    .insert_resource(Orbit::new(CameraMode::Jacqueline))
    .insert_resource(AssetSettings::new_party(AssetConfig::load_at(
        path.with_extension("toml"),
        path.with_extension("assets"),
    )))
    .init_resource::<ButtonInput<KeyCode>>();
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
    let epoch = app.world().resource::<Table>().context.menu_epoch();
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
            .is_pending(Source::Hand(100))
    );
    act(&mut app, Action::Confirm(revision, false));
    assert!(
        !app.world()
            .resource::<Table>()
            .context
            .is_pending(Source::Hand(100))
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
        layout::focus(Area::Test).0
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
    render_app_in(App::new(), path)
}
fn render_app_in(app: App, path: PathBuf) -> App {
    let mut app = app_in(app, path);
    app.world_mut().resource_mut::<Table>().reduced_motion = true;
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

#[test]
fn physical_harm_totals_include_zeroes_and_survive_scene_replacement_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = render_app(dir.path().join("harm-totals.json"));
    app.update();
    for step in 0..3 {
        let world = app.world_mut();
        let positions: Vec<_> = world
            .query_filtered::<&Transform, With<crate::tabletop_harm_tokens::HarmToken>>()
            .iter(world)
            .map(|transform| transform.translation)
            .collect();
        assert_eq!(
            positions.len(),
            6,
            "each investigator keeps both physical tokens"
        );
        for index in 0..3 {
            for horror in [false, true] {
                assert!(positions.contains(&layout::investigator_harm(index, horror)));
            }
        }
        let captions: Vec<_> = world
            .query::<(&Text, &WorldLabel)>()
            .iter(world)
            .map(|(text, at)| (text.0.clone(), at.0))
            .collect();
        for (index, damage, horror) in [(0, "0/9", "0/5"), (1, "0/7", "0/7"), (2, "0/6", "2/9")] {
            for (is_horror, kind, total) in [(false, "Damage", damage), (true, "Horror", horror)] {
                assert!(captions.contains(&(
                    format!("{kind} {total}"),
                    layout::investigator_harm_caption(index, is_horror, true),
                )));
            }
        }
        match step {
            0 => {
                choose(&mut app, "select:tony");
                choose(&mut app, "draw");
            }
            1 => act(&mut app, Action::Reopen),
            _ => {}
        }
    }
}

#[test]
fn production_hand_recenters_after_draw_and_restores_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = render_app(dir.path().join("layout.json"));
    choose(&mut app, "select:tony");
    for count in 1..=4 {
        let view = app.world().resource::<Table>().view();
        let hand = &view.investigator(Investigator::Tony).hand;
        assert_eq!(hand.len(), count);
        let world = app.world_mut();
        let mut query = world.query::<(&CardSource, &Transform)>();
        let positions: Vec<_> = hand
            .iter()
            .map(|card| {
                query
                    .iter(world)
                    .find(|(source, _)| source.0 == Source::Hand(card.id))
                    .expect("every held card remains an interactive physical card")
                    .1
                    .translation
            })
            .collect();
        let center = layout::investigator_center(0, 3);
        assert!(
            (positions.iter().map(|p| p.x).sum::<f32>() / count as f32 - center.x).abs() < 0.001
        );
        assert!(positions.iter().all(|p| p.z > center.z));
        for pair in positions.windows(2) {
            assert!(pair[1].x - pair[0].x > CARD_WIDTH);
        }
        if count < 4 {
            choose(&mut app, "draw");
        }
    }
    let before = app.world().resource::<Table>().view();
    act(&mut app, Action::Reopen);
    assert_eq!(app.world().resource::<Table>().view(), before);
}

#[test]
fn production_tokens_move_sections_only_after_cancellation_and_replay_preserves_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = render_app(dir.path().join("token-layout.json"));
    app.world_mut().resource_mut::<Table>().session = Session::with_draws(vec![
        crate::TokenFace::Zero,
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
    let positions = |app: &mut App| {
        let world = app.world_mut();
        world
            .query::<(&CardSource, &Transform)>()
            .iter(world)
            .filter_map(|(source, transform)| {
                if let Source::Token(id) = source.0 {
                    Some((id, transform.translation))
                } else {
                    None
                }
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before = positions(&mut app);
    assert_eq!(before.len(), 3);
    assert!(before.values().all(|p| p.x < layout::TEST_CENTER.x));
    choose(&mut app, "cancel:0,1");
    let after = positions(&mut app);
    assert!(after[&0].x > layout::TEST_CENTER.x && after[&1].x > layout::TEST_CENTER.x);
    assert!(after[&2].x < layout::TEST_CENTER.x);
    let live = app.world().resource::<Table>().session.clone();
    act(&mut app, Action::Replay);
    while app
        .world()
        .resource::<Table>()
        .replay
        .as_ref()
        .unwrap()
        .index()
        + 1
        < app
            .world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .len()
    {
        act(&mut app, Action::Next);
    }
    assert_eq!(positions(&mut app), after);
    assert_eq!(app.world().resource::<Table>().session, live);
}

fn completed_attempt(advance: Option<bool>) -> Session {
    let mut session = Session::with_draws(vec![crate::TokenFace::Zero; 6]);
    if let Some(advance) = advance {
        for actor in ["tony", "jacqueline"] {
            session
                .choose(session.observe().revision, &format!("select:{actor}"))
                .unwrap();
            for _ in 0..3 {
                for id in ["investigate", "commit:none", "reveal", "resolve:0"] {
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
        if advance {
            session
                .choose(session.observe().revision, "continue-act")
                .unwrap();
        }
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
        layout::focus(Area::Test).0,
        "actual revealed test focuses Test"
    );
    for _ in 0..2 {
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
    let mut discard = layout::investigator_discard(Investigator::Patrice.index());
    discard.y = 0.;
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
    let labels = visible_text(&mut app);
    assert!(labels.iter().any(|text| text == "Discard"));
    assert!(labels.iter().any(|text| text.contains("Promise of Power")));
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
        while app
            .world()
            .resource::<Table>()
            .view()
            .test
            .as_ref()
            .is_none_or(|test| test.result.is_none())
        {
            let replay = app.world().resource::<Table>().replay.as_ref().unwrap();
            assert!(replay.index() + 1 < replay.len());
            act(&mut app, Action::Next);
        }
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
            for id in ["investigate", "commit:none", "reveal", "resolve:0"] {
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
            .contains("The Door on the Floor")
    );
    choose(&mut app, "continue-act");
    assert!(
        visible_text(&mut app)
            .join("\n")
            .contains("Escaped the Study")
    );
}
