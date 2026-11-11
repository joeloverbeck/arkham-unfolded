use super::*;

#[test]
fn personal_resource_and_clue_counters_do_not_offer_investigator_actions() {
    use crate::tabletop_counters::Counter;

    let dir = tempfile::tempdir().unwrap();
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
    let mut app = host_with_table(Table::new(session, dir.path().join("counters.json")), false);
    let world = app.world_mut();
    let mut seen = 0;
    for (entity, at) in world
        .query_filtered::<(Entity, &Transform), With<Counter>>()
        .iter(world)
    {
        if (0..3).any(|index| {
            at.translation == layout::investigator_resource(index)
                || at.translation == layout::investigator_clue(index)
        }) {
            seen += 1;
            assert!(
                world.get::<ContextTarget>(entity).is_none(),
                "Personal totals are not investigator action targets"
            );
            assert!(
                world.get::<CardFeedback>(entity).is_none(),
                "Personal totals must not inherit investigator availability"
            );
        }
    }
    assert_eq!(
        seen, 6,
        "Exercise each investigator's resource and clue counters"
    );
}

#[test]
fn gameplay_cues_follow_current_windows_on_cards_models_and_the_bag() {
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::JacquelineCancellation);
    let dir = tempfile::tempdir().unwrap();
    for at_reveal in [false, true] {
        if at_reveal {
            for _ in 0..8 {
                let view = session.observe();
                if view
                    .choices
                    .iter()
                    .any(|choice| choice.id == "tokens:jacqueline")
                {
                    break;
                }
                let id = ["window:pass", "test:confirm"]
                    .into_iter()
                    .find(|id| view.choices.iter().any(|choice| choice.id == *id))
                    .expect("route to reveal");
                session.choose(view.revision, id).unwrap();
            }
        }
        let mut app = host_with_table(
            Table::new(session.clone(), dir.path().join("availability.json")),
            false,
        );
        for _ in 0..3 {
            app.update();
        }
        let view = app.world().resource::<Table>().view();
        let world = app.world_mut();
        let mut jacqueline = 0;
        let mut bag = 0;
        for (target, cue) in world.query::<(&ContextTarget, &CardFeedback)>().iter(world) {
            assert_eq!(
                cue.opportunity,
                !controls::source_choices(&view, &target.source).is_empty(),
                "{:?}",
                target.source
            );
            if target.source == controls::Source::Investigator(Investigator::Jacqueline) {
                assert_eq!(cue.opportunity, at_reveal);
                jacqueline += 1;
            }
            if target.source == controls::Source::Bag {
                assert_eq!(cue.opportunity, at_reveal);
                bag += 1;
            }
        }
        assert!(jacqueline >= 2, "card and standee must agree");
        assert_eq!(bag, 1);
        app.world_mut().resource_mut::<Table>().history_open = true;
        app.update();
        assert!(
            app.world_mut()
                .query::<&CardFeedback>()
                .iter(app.world())
                .all(|cue| !cue.opportunity)
        );
        app.world_mut().resource_mut::<Table>().history_open = false;
        app.world_mut()
            .resource_mut::<RenderHealth>()
            .stop("test admission failure".into());
        app.update();
        assert!(
            app.world_mut()
                .query::<&CardFeedback>()
                .iter(app.world())
                .all(|cue| !cue.opportunity)
        );
    }
}

#[test]
fn selection_new_refuses_a_collision_and_open_requires_an_existing_attempt() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("attempt.json");
    let config = AssetConfig::load_at(
        directory.path().join("settings.toml"),
        directory.path().join("art"),
    );
    let options = |intent: &str| {
        Options::parse(
            config.clone(),
            [
                "--experiment",
                "gathering",
                "--attempt",
                intent,
                "--save",
                path.to_str().unwrap(),
            ]
            .map(str::to_owned)
            .into_iter(),
        )
        .unwrap()
        .unwrap()
    };
    assert!(options("open").open_table().is_err());
    assert!(!path.exists());
    let fresh = options("new").open_table().unwrap();
    assert!(!path.exists());
    continuity::save_session(&path, &fresh.session).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(options("new").open_table().is_err());
    assert_eq!(options("open").open_table().unwrap().session, fresh.session);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn opening_a_fresh_native_table_does_not_save_before_artwork_admission() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let table = Table::open(save.clone()).unwrap();
    assert!(
        !save.exists(),
        "checking artwork must not create an attempt"
    );
    assert_eq!(table.session.observe().revision, 0);
}

#[test]
fn registered_table_saves_its_initial_decision_only_after_admission() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let mut app = host(save.clone());
    app.insert_resource(Table::open(save.clone()).unwrap());
    app.insert_resource(RenderHealth::default());
    app.update();
    assert!(!save.exists());
    app.insert_resource(RenderHealth::ready_for_test());
    app.update();
    assert_eq!(
        continuity::load_session(&save).unwrap(),
        app.world().resource::<Table>().session
    );
    let before = std::fs::read(&save).unwrap();
    app.insert_resource(RenderHealth::failed_for_test());
    app.update();
    assert_eq!(std::fs::read(save).unwrap(), before);
}

mod attachment_sideboard;
mod cards;
mod clue_payment;
mod counters_and_enemies;
mod doom;
mod draws;
mod encounter;
mod location_markers;
mod map_motion;
mod navigation;
mod pendulum;
mod restart;
pub(super) mod results;
mod review_saves;
mod round_flow;
mod selection;
mod slots;
mod token_guidance;
mod trays;
mod victory;
mod violin;
mod wagers_keys;

#[test]
fn empty_enemy_phase_advances_without_a_click_and_saves_its_history() {
    let mut session = (0..100)
        .find_map(|seed| {
            let mut session = Session::with_seed(seed);
            for id in [
                "lead:tony",
                "confirm-mulligan",
                "confirm-mulligan",
                "confirm-mulligan",
            ] {
                session.choose(session.observe().revision, id).unwrap();
            }
            (!session.observe().investigators[2]
                .hand
                .iter()
                .any(|card| card.code == "04199"))
            .then_some(Session::with_seed(seed))
        })
        .unwrap();
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
        "turn:tony",
        "turn:end",
        "turn:jacqueline",
        "turn:end",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    // Patrice's is the only remaining turn, so the session began it itself.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("empty-phase.json");
    let mut app = host_with_table(Table::new(session, path.clone()), false);
    activate_choice(&mut app, "turn:end");
    settle_automatic_resolution(&mut app);
    let session = &app.world().resource::<Table>().session;
    let view = session.observe();
    assert!(
        view.choices
            .iter()
            .any(|choice| choice.id.starts_with("upkeep:patrice-discard:")),
        "{:?}",
        view.choices
    );
    assert!(
        view.history
            .iter()
            .any(|entry| entry.contains("Enemy phase — no activity"))
    );
    assert_eq!(&continuity::load_session(path).unwrap(), session);
}

#[test]
fn player_window_explains_and_marks_the_playable_card_without_auto_passing() {
    let mut session = (0..100)
        .find_map(|seed| {
            let mut session = Session::with_seed(seed);
            for id in [
                "lead:tony",
                "confirm-mulligan",
                "confirm-mulligan",
                "confirm-mulligan",
            ] {
                session.choose(session.observe().revision, id).unwrap();
            }
            for id in ["turn:tony", "action:ordinary:investigate"] {
                session.choose(session.observe().revision, id).unwrap();
            }
            session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("premonition:"))
                .then_some(session)
        })
        .expect("A real opening hand with Premonition");
    let before = session.observe();
    let card = before.investigators[2]
        .hand
        .iter()
        .find(|card| card.code == "04199")
        .unwrap()
        .id;
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(
        Table::new(session.clone(), directory.path().join("window.json")),
        false,
    );
    for _ in 0..12 {
        draws::tick(&mut app, 0.1);
    }
    let text = rendered_text(&mut app);
    assert!(text.contains("Jacqueline") && text.contains("Premonition"));
    let world = app.world_mut();
    assert!(
        world
            .query::<(&ContextTarget, &Children)>()
            .iter(world)
            .any(|(target, children)| {
                target.source == controls::Source::Card(card)
                    && children.iter().any(|child| {
                        world
                            .get::<Name>(child)
                            .is_some_and(|name| name.as_str() == "Available gameplay choices")
                            && world.get::<Visibility>(child) == Some(&Visibility::Inherited)
                    })
            })
    );
    assert_eq!(world.resource::<Table>().session, session);
    session.choose(before.revision, "window:pass").unwrap();
}

#[test]
fn violin_opportunity_marks_its_ability_not_the_discard_payment() {
    use crate::gathering::HostileFixture;
    let mut session = Session::with_hostile_fixture(HostileFixture::PatriceCancellationVision);
    let view = session.observe();
    let cancel = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("test-of-will:"))
        .unwrap();
    session.choose(view.revision, &cancel.id).unwrap();
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    let violin = play.players[1]
        .assets
        .iter()
        .find(|asset| asset.card.code == "06016")
        .unwrap()
        .card
        .id;
    let opportunities = controls::window_opportunities(&view);
    assert!(
        opportunities
            .iter()
            .any(|(source, _)| *source == controls::Source::Card(violin))
    );
    for card in &play.players[1].hand {
        assert!(
            !opportunities
                .iter()
                .any(|(source, _)| *source == controls::Source::Card(card.id))
        );
    }
}

fn assert_initial_table(app: &mut App) {
    let view = app.world().resource::<Table>().view();
    assert!(view.play.is_none());
    assert!(inspection_text(&view, "01111").unwrap().contains("Study"));
    for hidden in ["01112", "01113", "01116", "01106", "01109"] {
        assert!(
            inspection_text(&view, hidden).is_none(),
            "{hidden} must remain hidden"
        );
    }
    let world = app.world_mut();
    let sources = world
        .query::<&ContextTarget>()
        .iter(world)
        .map(|target| target.source.clone())
        .collect::<Vec<_>>();
    for source in [
        controls::Source::Bag,
        controls::Source::Act,
        controls::Source::Agenda,
        controls::Source::EncounterDeck,
        controls::Source::Location("01111".into()),
    ] {
        assert!(
            sources.contains(&source),
            "initial table must render {source:?}"
        );
    }
    assert_eq!(
        world
            .query::<&Name>()
            .iter(world)
            .filter(|name| name.as_str() == "Investigator standee")
            .count(),
        3
    );
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
fn initial_table_remains_visible_through_mulligans_and_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("setup.json");
    let mut app = host(path);
    let mut expected = Session::with_seed(19);
    assert_initial_table(&mut app);
    for id in ["lead:jacqueline", "confirm-mulligan", "confirm-mulligan"] {
        activate_choice(&mut app, id);
        expected.choose(expected.observe().revision, id).unwrap();
        assert_initial_table(&mut app);
        assert_eq!(
            app.world().resource::<Table>().session.clone(),
            expected.clone()
        );
    }
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Reopen);
    app.update();
    assert_initial_table(&mut app);
    assert_eq!(
        app.world().resource::<Table>().session.clone(),
        expected.clone()
    );
}

pub(super) fn host(path: PathBuf) -> App {
    host_with_pointer_input(path, false)
}

pub(super) fn host_with_pointer_input(path: PathBuf, raw_window_events: bool) -> App {
    host_with_table(Table::new(Session::with_seed(19), path), raw_window_events)
}

pub(super) fn host_with_table(table: Table, raw_window_events: bool) -> App {
    let mut app = unstarted_host(table, raw_window_events);
    app.update();
    app
}

/// The host with Bevy's own dialog plugins and delayed-command clock, as
/// `DefaultPlugins` registers them natively. Advance `Time` to run the modal
/// plugin's delayed focus.
pub(super) fn host_with_modal_plugins(table: Table) -> App {
    let mut app = unstarted_host(table, false);
    app.add_plugins((
        bevy::ui_widgets::DialogPlugin,
        bevy::ui_widgets::ModalDialogPlugin,
    ))
    .add_systems(PreUpdate, bevy::time::check_delayed_command_queues);
    app.update();
    app
}

fn unstarted_host(table: Table, raw_window_events: bool) -> App {
    let mut app = crate::tabletop_layout::heading_checks::app();
    if raw_window_events {
        // Both are supplied by DefaultPlugins in native startup.
        app.add_message::<bevy::window::WindowEvent>()
            .add_plugins(bevy::picking::input::PointerInputPlugin);
    }
    app.init_resource::<Time>()
        .init_resource::<PresentationTime>()
        .init_resource::<Redraw>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<art::Art>()
        .insert_resource(table)
        .insert_resource(RenderHealth::ready_for_test())
        .insert_resource(Orbit::new().with_home(sideboard::focus(Area::Overview)));
    app.world_mut().spawn((
        Window {
            resolution: (1600, 1000).into(),
            focused: true,
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    install(&mut app);
    // An assertion, not a substitute: every dialog any tabletop test opens is checked.
    app.add_systems(Last, opened_dialogs_offer_focus);
    app
}

/// Bevy's modal plugin moves focus into each newly opened dialog with this exact
/// navigation call. Natively, a dialog with nothing focusable only logs
/// "No focusable entities in modal dialog"; here it fails the test that opened it.
fn opened_dialogs_offer_focus(
    opened: Query<Entity, Added<bevy::ui_widgets::ModalDialog>>,
    navigation: bevy::input_focus::tab_navigation::TabNavigation,
    labels: Query<&AccessibleLabel>,
) {
    for dialog in &opened {
        assert!(
            navigation
                .initialize(dialog, bevy::input_focus::tab_navigation::NavAction::Last)
                .is_ok(),
            "Modal dialog {:?} opened with no focusable descendant",
            labels
                .get(dialog)
                .map_or("without a label", |label| label.0.as_str())
        );
    }
}

/// Every open modal dialog, paired with whether its own tab group can take focus.
pub(super) fn modal_dialogs_and_focus(app: &mut App) -> Vec<(Entity, bool)> {
    use bevy::{
        ecs::system::RunSystemOnce,
        input_focus::tab_navigation::{NavAction, TabNavigation},
        ui_widgets::ModalDialog,
    };
    app.world_mut()
        .run_system_once(
            |dialogs: Query<Entity, With<ModalDialog>>, navigation: TabNavigation| {
                dialogs
                    .iter()
                    .map(|dialog| {
                        (
                            dialog,
                            navigation.initialize(dialog, NavAction::Last).is_ok(),
                        )
                    })
                    .collect::<Vec<_>>()
            },
        )
        .unwrap()
}

pub(super) fn assert_unclipped_button(app: &mut App, label: &str, size: Vec2, scale: f32) {
    let button = button_with_label(app, label)
        .or_else(|| choice_button(app, label))
        .unwrap();
    let world = app.world();
    let node = world.get::<ComputedNode>(button).unwrap();
    assert!(node.size().min_element() > 0.);
    let transform = world.get::<UiGlobalTransform>(button).unwrap();
    let min = transform.transform_point2(-node.size() / 2.) / scale - Vec2::splat(4.);
    let max = transform.transform_point2(node.size() / 2.) / scale + Vec2::splat(4.);
    assert!(
        min.cmpge(Vec2::ZERO).all() && max.cmple(size).all(),
        "{label}'s full focus outline must stay in the viewport: {min:?}..{max:?}"
    );
    let mut ancestor = button;
    while let Some(parent) = world.get::<ChildOf>(ancestor) {
        ancestor = parent.parent();
        if world
            .get::<Node>(ancestor)
            .is_some_and(|node| node.overflow.y == OverflowAxis::Scroll)
        {
            let clip = world.get::<ComputedNode>(ancestor).unwrap();
            let transform = world.get::<UiGlobalTransform>(ancestor).unwrap();
            let top = transform.transform_point2(-clip.size() / 2.).y / scale;
            let bottom = transform.transform_point2(clip.size() / 2.).y / scale;
            assert!(
                min.y >= top - 0.1 && max.y <= bottom + 0.1,
                "{label}'s outline {min:?}..{max:?} is clipped by {top}..{bottom}"
            );
        }
    }
}

#[test]
fn gathering_accepts_static_capture_options_without_writing_settings_or_session() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("gathering.toml");
    let save = directory.path().join("session.json");
    let screenshot = directory.path().join("scenario.png");
    let options = Options::parse(
        AssetConfig::load_at(settings.clone(), directory.path().join("assets")),
        [
            "--experiment",
            "gathering",
            "--save",
            save.to_str().unwrap(),
            "--initial-view",
            "scenario",
            "--screenshot",
            screenshot.to_str().unwrap(),
            "--quit-after-screenshot",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap()
    .unwrap();
    assert_eq!(options.save, save);
    assert!(!save.exists());
    assert!(!settings.exists());
    assert!(!screenshot.exists());
}

#[test]
fn gathering_diagnostic_framing_retains_normal_startup_and_rejects_incomplete_options() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("settings.toml");
    let parse = |args: &[&str]| {
        Options::parse(
            AssetConfig::load_at(settings.clone(), directory.path().join("art")),
            args.iter().copied().map(str::to_owned),
        )
    };
    let mut app = App::new();
    parse(&[])
        .unwrap()
        .unwrap()
        .configure_diagnostics(&mut app)
        .unwrap();
    assert_eq!(
        app.world().resource::<Orbit>().pose(),
        Orbit::new()
            .with_home(sideboard::focus(Area::Overview))
            .pose()
    );
    for (name, area) in [
        ("overview", Area::Overview),
        ("scenario", Area::Scenario),
        ("test", Area::Test),
        ("tony", Area::Investigator(0)),
        ("patrice", Area::Investigator(1)),
        ("jacqueline", Area::Investigator(2)),
    ] {
        parse(&["--initial-view", name])
            .unwrap()
            .unwrap()
            .configure_diagnostics(&mut app)
            .unwrap();
        assert_eq!(
            app.world().resource::<Orbit>().pose().target,
            sideboard::focus(area).0
        );
        assert!(!app.world().resource::<Orbit>().is_following());
    }
    for args in [
        vec!["--initial-view"],
        vec!["--initial-view", "unknown"],
        vec!["--screenshot"],
        vec!["--quit-after-screenshot"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    assert!(!settings.exists());
}

#[test]
fn gathering_static_capture_waits_for_graphics_and_finishes_the_actual_write() {
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("session.json");
    let screenshot = directory.path().join("nested/scenario.png");
    let mut app = host(save.clone());
    let before = app.world().resource::<Table>().session.clone();
    app.add_message::<AppExit>();
    let options = Options::parse(
        AssetConfig::load_at(
            directory.path().join("settings"),
            directory.path().join("art"),
        ),
        [
            "--initial-view",
            "scenario",
            "--screenshot",
            screenshot.to_str().unwrap(),
            "--quit-after-screenshot",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap()
    .unwrap();
    options.configure_diagnostics(&mut app).unwrap();
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        sideboard::focus(Area::Scenario).0
    );
    for health in [
        RenderHealth::default(),
        RenderHealth::failed_for_test(),
        RenderHealth::just_ready_for_test(),
    ] {
        app.insert_resource(health);
        for _ in 0..100 {
            app.update();
        }
        assert_eq!(
            app.world_mut()
                .query::<&Screenshot>()
                .iter(app.world())
                .count(),
            0
        );
        assert!(app.world().resource::<Messages<AppExit>>().is_empty());
    }
    app.insert_resource(RenderHealth::ready_for_test());
    for _ in 0..100 {
        app.update();
    }
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())
        .expect("the registered Gathering path must request a capture");
    assert!(!screenshot.exists());
    assert!(app.world().resource::<Messages<AppExit>>().is_empty());
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: capture_test_image(),
    });
    assert!(
        std::fs::read(&screenshot)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
    assert!(
        app.world()
            .resource::<Messages<AppExit>>()
            .iter_current_update_messages()
            .any(AppExit::is_success)
    );
    assert_eq!(app.world().resource::<Table>().session, before);
    assert!(!save.exists());
}

fn capture_test_image() -> Image {
    use bevy::{
        asset::RenderAssetUsages,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    )
}

#[test]
fn gathering_options_enable_default_and_saved_art_without_changing_preferences() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("gathering.toml");
    let default_root = directory.path().join("default-gathering-assets");
    let parse = || {
        Options::parse(
            AssetConfig::load_at(settings.clone(), default_root.clone()),
            ["--experiment", "gathering"].into_iter().map(str::to_owned),
        )
        .unwrap()
        .unwrap()
    };
    assert_eq!(parse().art_root, default_root);
    assert!(!settings.exists());
    let saved_root = directory.path().join("selected-gathering-assets");
    AssetConfig::load_at(settings.clone(), default_root.clone())
        .save_root(&saved_root)
        .unwrap();
    let before = std::fs::read(&settings).unwrap();
    let options = parse();
    assert_eq!(options.art_root, saved_root);
    assert_eq!(
        options.save,
        PathBuf::from("local/gathering/setup.session.json")
    );
    assert_eq!(std::fs::read(settings).unwrap(), before);
}

#[test]
fn gathering_art_override_is_temporary_and_no_art_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("gathering.toml");
    let saved_root = directory.path().join("saved-assets");
    AssetConfig::load_at(settings.clone(), saved_root.clone())
        .save_root(&saved_root)
        .unwrap();
    let before = std::fs::read(&settings).unwrap();
    for args in [
        vec!["--art-root", "/tmp/gathering-override"],
        vec!["--no-art", "--art-root", "/tmp/gathering-override"],
        vec!["--art-root", "/tmp/gathering-override", "--no-art"],
        vec!["--no-art"],
    ] {
        let no_art = args.contains(&"--no-art");
        let result = Options::parse(
            AssetConfig::load_at(settings.clone(), directory.path().join("unused-default")),
            args.into_iter().map(str::to_owned),
        );
        if no_art {
            assert!(result.is_err());
        } else {
            let options = result.unwrap().unwrap();
            assert_eq!(options.art_root, PathBuf::from("/tmp/gathering-override"));
            assert_eq!(options.config.root(), saved_root);
        }
        assert_eq!(std::fs::read(&settings).unwrap(), before);
    }
}

fn button_with_label(app: &mut App, label: &str) -> Option<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<(Entity, &Children), With<Button>>()
        .iter(world)
        .find_map(|(entity, children)| {
            children
                .iter()
                .any(|child| {
                    world.get::<AccessibleLabel>(child).is_some_and(|text| {
                        crate::tabletop_support::readable_text(text.0.clone()) == label
                    }) || world.get::<Text>(child).is_some_and(|text| text.0 == label)
                })
                .then_some(entity)
        })
}

/// Invoke the production mesh observer with a synthetic intersection. The menu
/// rows below use real UI layout/picking; this helper does not claim GPU picking.
pub(super) fn click_source(app: &mut App, source: controls::Source) {
    let world = app.world_mut();
    let entity = world
        .query::<(Entity, &ContextTarget, Option<&Mesh3d>)>()
        .iter(world)
        .find_map(|(entity, target, mesh)| {
            (target.source == source
                && (!matches!(source, controls::Source::Location(_)) || mesh.is_some()))
            .then_some(entity)
        })
        .unwrap_or_else(|| panic!("The production scene must render {source:?}"));
    click_mesh_observer(app, entity);
}

fn click_mesh_observer(app: &mut App, entity: Entity) {
    queue_mesh_click(app, entity);
    app.update();
}

fn queue_mesh_click(app: &mut App, entity: Entity) {
    use bevy::{
        camera::RenderTarget,
        picking::{
            backend::HitData,
            events::Pointer,
            pointer::{Location, PointerButton, PointerId},
        },
        window::WindowRef,
    };
    let world = app.world_mut();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    let camera = world
        .query_filtered::<Entity, With<MainCamera>>()
        .single(world)
        .unwrap();
    world.trigger(PointerClick {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: RenderTarget::Window(WindowRef::Entity(window))
                    .normalize(Some(window))
                    .unwrap(),
                position: Vec2::new(800., 500.),
            },
        ),
        button: PointerButton::Primary,
        hit: HitData::new(camera, 1., None, None),
        duration: std::time::Duration::from_millis(20),
        count: 1,
    });
}

#[test]
fn imported_bag_descendant_click_selects_the_logical_bag() {
    let dir = tempfile::tempdir().unwrap();
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::JacquelineCancellation);
    let mut app = host_with_table(Table::new(session, dir.path().join("bag.json")), false);
    let world = app.world_mut();
    let bag = world
        .query::<(Entity, &ContextTarget)>()
        .iter(world)
        .find(|(_, target)| target.source == controls::Source::Bag)
        .unwrap()
        .0;
    let visual = world
        .get::<Children>(bag)
        .unwrap()
        .iter()
        .find(|entity| world.get::<WorldAssetRoot>(*entity).is_some())
        .unwrap();
    let node = world.spawn(ChildOf(visual)).id();
    let primitive = world.spawn(ChildOf(node)).id();
    let before = world.resource::<Table>().session.clone();
    queue_mesh_click(&mut app, primitive);
    assert!(
        matches!(app.world().resource::<Actions>().0.last(), Some(Action::Select(_, _, selection)) if selection.source == controls::Source::Bag)
    );
    app.update();
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .inspection
            .as_ref()
            .map(|(_, code)| code.as_str()),
        Some("01104") // Bag inspection retains the scenario's chaos-token reference.
    );
}

fn keyboard(app: &mut App, code: KeyCode, logical_key: bevy::input::keyboard::Key, pressed: bool) {
    let world = app.world_mut();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    world.write_message(bevy::input::keyboard::KeyboardInput {
        key_code: code,
        logical_key,
        state: if pressed {
            bevy::input::ButtonState::Pressed
        } else {
            bevy::input::ButtonState::Released
        },
        text: None,
        repeat: false,
        window,
    });
    app.update();
}

#[test]
fn contextual_keyboard_activation_commits_the_selected_investigator() {
    use bevy::input::keyboard::Key;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("keyboard-choice.json");
    let mut app = host(path.clone());
    click_source(
        &mut app,
        controls::Source::Investigator(Investigator::Patrice),
    );
    app.update();
    let lead = button_with_label(&mut app, "Choose Patrice Hathaway as lead investigator").unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(lead));
    keyboard(&mut app, KeyCode::Enter, Key::Enter, true);
    keyboard(&mut app, KeyCode::Enter, Key::Enter, false);
    let saved = continuity::load_session(path).unwrap().observe();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.lead, Some(Investigator::Patrice));
    finish_presentation(&mut app);
    assert_eq!(saved, app.world().resource::<Table>().view());
}

#[test]
fn context_captures_home_and_escape_and_backdrop_dismiss_without_a_decision() {
    use bevy::input::keyboard::Key;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("dismiss.json");
    let mut app = host(path.clone());
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Focus(Area::Overview));
    app.update();
    let pose = app.world().resource::<Orbit>().pose();
    let view = app.world().resource::<Table>().view();
    click_source(&mut app, controls::Source::Investigator(Investigator::Tony));
    app.update();
    keyboard(&mut app, KeyCode::Home, Key::Home, true);
    assert_eq!(app.world().resource::<Orbit>().pose(), pose);
    keyboard(&mut app, KeyCode::Home, Key::Home, false);
    keyboard(&mut app, KeyCode::Escape, Key::Escape, true);
    keyboard(&mut app, KeyCode::Escape, Key::Escape, false);
    assert!(!app.world().resource::<Table>().context.is_open());
    assert_eq!(app.world().resource::<Table>().view(), view);
    click_source(&mut app, controls::Source::Investigator(Investigator::Tony));
    let world = app.world_mut();
    let backdrop = world
        .query_filtered::<Entity, With<controls::Backdrop>>()
        .single(world)
        .unwrap();
    click_mesh_observer(&mut app, backdrop);
    assert!(!app.world().resource::<Table>().context.is_open());
    assert!(button_with_label(&mut app, "Choose Tony Morgan as lead investigator").is_none());
    assert_eq!(app.world().resource::<Table>().view(), view);
    assert!(!path.exists());
}

#[test]
fn native_choice_persists_before_showing_real_opening_hands() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gathering.json");
    let mut app = host(path.clone());
    let lead = "Choose Tony Morgan as lead investigator";
    assert!(button_with_label(&mut app, lead).is_none());
    click_source(&mut app, controls::Source::Investigator(Investigator::Tony));
    assert_eq!(app.world().resource::<Table>().view().revision, 0);
    assert!(!path.exists(), "Opening a card menu cannot save a decision");
    let entity = button_with_label(&mut app, lead).expect("The card menu offers its lead choice");
    assert!(button_with_label(&mut app, "Choose Patrice Hathaway as lead investigator").is_none());
    app.world_mut().trigger(Activate { entity });
    app.update();
    let saved = continuity::load_session(path).unwrap().observe();
    assert_eq!(saved, app.world().resource::<Table>().session.observe());
    assert_eq!(saved.stage, crate::gathering::Stage::Mulligan);
    assert!(saved.investigators.iter().all(|p| p.hand.len() == 5));
    assert!(app.world().resource::<Table>().presentation.drawing());
    finish_presentation(&mut app);
    assert_eq!(saved, app.world().resource::<Table>().view());
}

#[test]
fn pointer_lead_choice_through_production_ui_picking_persists() {
    assert_pointer_lead_choice(false, false);
}

#[test]
fn window_mouse_lead_choice_through_production_ui_picking_persists() {
    assert_pointer_lead_choice(true, false);
}

#[test]
fn graphics_health_observations_preserve_pointer_lead_choice() {
    assert_pointer_lead_choice(true, true);
}

#[test]
fn ordinary_setup_tab_navigation_survives_graphics_admission() {
    use bevy::{
        ecs::system::RunSystemOnce,
        input_focus::{
            InputFocus,
            tab_navigation::{NavAction, TabNavigation},
        },
    };

    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("keyboard.json"));
    app.insert_resource(RenderHealth::default());
    app.update();
    app.insert_resource(RenderHealth::just_ready_for_test());
    app.update();
    app.insert_resource(RenderHealth::ready_for_test());
    app.update();
    let result = app
        .world_mut()
        .run_system_once(|navigation: TabNavigation, focus: Res<InputFocus>| {
            navigation.navigate(&focus, NavAction::Next)
        })
        .unwrap();
    assert!(
        result.is_ok(),
        "ordinary setup must admit Tab navigation after graphics readiness: {result:?}"
    );
}

#[test]
fn dynamic_world_labels_follow_projection_without_pinning_to_viewport_edges() {
    for scale in [1., 2.] {
        let directory = tempfile::tempdir().unwrap();
        let mut app = host(directory.path().join("projected-labels.json"));
        app.world_mut()
            .resource_mut::<Orbit>()
            .focus(layout::focus(Area::Investigator(0)));
        let size = Vec2::new(1600., 1000.);
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, scale);
        for _ in 0..3 {
            app.update();
        }
        let world = app.world_mut();
        let label = world
            .query::<(Entity, &WorldLabel, &Text)>()
            .iter(world)
            .find_map(|(entity, label, text)| {
                (!label.1 && text.0.ends_with(" cards")).then_some(entity)
            })
            .expect("The production renderer supplies a changing deck count");
        for (point, visible) in [
            (Vec2::new(-20., 500.), true),
            (Vec2::new(1620., 500.), true),
            (Vec2::new(800., -20.), true),
            (Vec2::new(800., 1020.), true),
            (Vec2::new(1., 1.), true),
            (Vec2::new(1599., 999.), true),
            (Vec2::new(800., 500.), true),
        ] {
            let anchor = {
                let world = app.world_mut();
                let (camera, transform) = world
                    .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
                    .single(world)
                    .unwrap();
                let anchor = camera
                    .viewport_to_world(transform, point)
                    .unwrap()
                    .get_point(10.);
                assert!(
                    camera
                        .world_to_viewport(transform, anchor)
                        .unwrap()
                        .abs_diff_eq(point, 0.01)
                );
                anchor
            };
            app.world_mut().get_mut::<WorldLabel>(label).unwrap().0 = anchor;
            app.update();
            assert_eq!(
                *app.world().get::<Visibility>(label).unwrap() != Visibility::Hidden,
                visible,
                "scale={scale}, projected anchor={point}"
            );
            if visible {
                let node = app.world().get::<Node>(label).unwrap();
                let (Val::Px(left), Val::Px(top), Val::Px(width)) =
                    (node.left, node.top, node.width)
                else {
                    panic!("World label placement uses measured logical pixels")
                };
                assert!((left - (point.x - width / 2.)).abs() < 0.01);
                assert!((top - point.y).abs() < 0.01);
            }
        }
    }
}

#[test]
fn mulligan_prompt_names_each_typed_owner_without_changing_the_camera() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("mulligan-owner.json"));
    assert!(
        app.world()
            .resource::<Table>()
            .view()
            .decision_owner
            .is_none()
    );
    let pose = app.world().resource::<Orbit>().pose();
    activate_choice(&mut app, "lead:patrice");
    for owner in [
        Investigator::Patrice,
        Investigator::Jacqueline,
        Investigator::Tony,
    ] {
        finish_presentation(&mut app);
        let view = app.world().resource::<Table>().view();
        assert_eq!(view.stage, crate::gathering::Stage::Mulligan);
        assert_eq!(view.decision_owner, Some(owner));
        let text = rendered_text(&mut app);
        assert_eq!(
            text.matches(&format!("{}'s decision", owner.name()))
                .count(),
            1,
            "The global mulligan prompt must identify its current owner"
        );
        for other in Investigator::ALL
            .into_iter()
            .filter(|other| *other != owner)
        {
            assert!(!text.contains(&format!("{}'s decision", other.name())));
        }
        assert_eq!(app.world().resource::<Orbit>().pose(), pose);
        activate_choice(&mut app, "confirm-mulligan");
    }
    finish_presentation(&mut app);
    let view = app.world().resource::<Table>().view();
    assert!(view.play.is_some());
    assert!(
        !rendered_text(&mut app).contains("'s decision"),
        "Scenario play already names its decision owner in the phase status"
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), pose);
}

#[test]
fn artwork_card_captions_use_party_positions_without_hover_or_copy_ids() {
    use bevy::ecs::system::RunSystemOnce;
    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("card-caption.json"));
    activate_choice(&mut app, "lead:tony");
    finish_presentation(&mut app);
    let view = app.world().resource::<Table>().view();
    let held = view.investigators[0].hand[0].clone();
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let role: &'static str = Box::leak(format!("card-{}", held.code).into_boxed_str());
    app.insert_resource(art::Art::with_images_for_test([(role, image)]));
    app.world_mut().resource_mut::<Table>().dirty = true;
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Focus(Area::Investigator(0)));
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
        &mut app,
        Vec2::new(1600., 1000.),
        1.,
    );
    for _ in 0..3 {
        app.update();
    }
    let world = app.world_mut();
    let (label, card) = world
        .query::<(Entity, &motion::CardLabel)>()
        .iter(world)
        .find_map(|(entity, label)| {
            world
                .get::<ContextTarget>(label.0)
                .is_some_and(|target| target.source == controls::Source::Card(held.id))
                .then_some((entity, label.0))
        })
        .unwrap();
    world.run_system_once(world_labels).unwrap();
    assert_eq!(
        *world.get::<Visibility>(label).unwrap(),
        Visibility::Inherited
    );
    assert_eq!(world.get::<Text>(label).unwrap().0, held.title);
    let Val::Px(width) = world.get::<Node>(label).unwrap().width else {
        panic!("card caption must have a projected pixel width");
    };
    assert!((1.0..190.0).contains(&width));
    assert!(
        world
            .get::<BackgroundColor>(label)
            .is_none_or(|background| background.0.alpha() == 0.)
    );
    let anchor = world.get::<Transform>(card).unwrap().translation + Vec3::Z * 1.1;
    assert_eq!(world.get::<WorldLabel>(label).unwrap().0, anchor);
    let (camera, transform) = world
        .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
        .single(world)
        .unwrap();
    let point = camera.world_to_viewport(transform, anchor).unwrap();
    assert_eq!(
        world.get::<Node>(label).unwrap().left,
        px(point.x - width / 2.)
    );
    assert_eq!(world.get::<Node>(label).unwrap().top, px(point.y));
}

#[test]
fn production_hand_captions_do_not_overlap_after_text_layout() {
    use crate::tabletop_layout::heading_checks;
    let directory = tempfile::tempdir().unwrap();
    for size in [Vec2::new(1280., 800.), Vec2::new(1600., 1000.)] {
        for scale in [1., 2.] {
            let path = directory
                .path()
                .join(format!("hand-captions-{}-{scale}.json", size.x));
            let mut app = host(path.clone());
            app.insert_resource(Table::new(Session::with_seed(14376244771021323084), path));
            app.world_mut().resource_mut::<Table>().reduced_motion = true;
            app.update();
            for id in [
                "lead:tony",
                "confirm-mulligan",
                "confirm-mulligan",
                "confirm-mulligan",
                "turn:patrice",
                "action:ordinary:resource",
            ] {
                activate_choice(&mut app, id);
            }
            finish_presentation(&mut app);
            heading_checks::viewport::<MainCamera>(&mut app, size, scale);
            app.world_mut()
                .resource_mut::<Actions>()
                .0
                .push(Action::Focus(Area::Investigator(1)));
            for _ in 0..5 {
                app.update();
            }
            let view = app.world().resource::<Table>().view();
            let hand = &view.investigators[1].hand;
            assert_eq!(hand.len(), 5);
            let world = app.world_mut();
            let captions = hand
                .iter()
                .map(|held| {
                    world
                        .query::<(Entity, &motion::CardLabel)>()
                        .iter(world)
                        .find_map(|(entity, caption)| {
                            world
                                .get::<ContextTarget>(caption.0)
                                .is_some_and(|target| {
                                    target.source == controls::Source::Card(held.id)
                                })
                                .then_some(entity)
                        })
                        .expect("every hand card has its production caption")
                })
                .collect::<Vec<_>>();
            let panel = world
                .query_filtered::<(Entity, &Node), With<Panel>>()
                .iter(world)
                .find_map(|(entity, node)| (node.bottom == px(10.)).then_some(entity))
                .unwrap();
            heading_checks::assert_hand_captions(&mut app, &captions, panel, size, scale);
            heading_checks::assert_sideways_caption_wrapping(&mut app, &captions);
        }
    }
}

#[test]
fn production_chaos_captions_do_not_overlap_after_text_layout() {
    use crate::tabletop_layout::heading_checks;
    let directory = tempfile::tempdir().unwrap();
    for size in [Vec2::new(1280., 800.), Vec2::new(1600., 1000.)] {
        for scale in [1., 2.] {
            let path = directory
                .path()
                .join(format!("chaos-captions-{}-{scale}.json", size.x));
            let mut app = host(path.clone());
            app.insert_resource(Table::new(Session::with_seed(14376244771021323084), path));
            app.world_mut().resource_mut::<Table>().reduced_motion = true;
            app.update();
            for id in [
                "lead:tony",
                "confirm-mulligan",
                "confirm-mulligan",
                "confirm-mulligan",
                "turn:patrice",
                "action:ordinary:resource",
            ] {
                activate_choice(&mut app, id);
            }
            finish_presentation(&mut app);
            heading_checks::viewport::<MainCamera>(&mut app, size, scale);
            for area in [Area::Scenario, Area::Test] {
                app.world_mut()
                    .resource_mut::<Actions>()
                    .0
                    .push(Action::Focus(area));
                for _ in 0..5 {
                    app.update();
                }
                let world = app.world_mut();
                let panel = world
                    .query_filtered::<(Entity, &Node), With<Panel>>()
                    .iter(world)
                    .find_map(|(entity, node)| (node.bottom == px(10.)).then_some(entity))
                    .unwrap();
                heading_checks::assert_chaos_captions(&mut app, panel, size, scale, area);
            }
        }
    }
}

fn assert_pointer_lead_choice(raw_window_events: bool, continuing_health_observations: bool) {
    use bevy::{
        camera::RenderTarget,
        picking::{
            hover::HoverMap,
            pointer::{
                Location, PointerAction, PointerButton, PointerId, PointerInput, PointerLocation,
            },
        },
        window::WindowRef,
    };

    let update = |app: &mut App| {
        if continuing_health_observations {
            // Native monitor_rendering mutates health sampling bookkeeping every
            // frame even when the admission state remains Ready.
            app.world_mut().resource_mut::<RenderHealth>().set_changed();
        }
        app.update();
    };

    for scale in [1.0, 2.0] {
        for same_frame in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("pointer.json");
            let mut app = host_with_pointer_input(path.clone(), raw_window_events);
            // Supply only renderer metadata. Layout, picking, button observers,
            // and Gathering actions use the plugins and installer used natively.
            crate::tabletop_layout::heading_checks::viewport::<MainCamera>(
                &mut app,
                Vec2::new(1600.0, 1000.0),
                scale,
            );
            click_source(&mut app, controls::Source::Investigator(Investigator::Tony));
            for _ in 0..5 {
                update(&mut app);
            }
            let world = app.world_mut();
            let window = world
                .query_filtered::<Entity, With<Window>>()
                .single(world)
                .unwrap();
            let button = world
                .query_filtered::<(Entity, &Children), With<Button>>()
                .iter(world)
                .find_map(|(entity, children)| {
                    children
                        .iter()
                        .any(|child| {
                            world.get::<Text>(child).is_some_and(|text| {
                                text.0 == "Choose Tony Morgan as lead investigator"
                            })
                        })
                        .then_some(entity)
                })
                .expect("production lead choice is rendered");
            let position = world
                .get::<bevy::ui::UiGlobalTransform>(button)
                .unwrap()
                .translation
                / scale;
            let location = Location {
                target: RenderTarget::Window(WindowRef::Entity(window))
                    .normalize(Some(window))
                    .unwrap(),
                position,
            };
            if raw_window_events {
                world.write_message(bevy::window::WindowEvent::CursorMoved(
                    bevy::window::CursorMoved {
                        window,
                        position,
                        delta: None,
                    },
                ));
            } else {
                world.spawn((PointerId::Mouse, PointerLocation::new(location.clone())));
            }
            update(&mut app);
            let hits = &app.world().resource::<HoverMap>()[&PointerId::Mouse];
            let hit_button = hits.keys().any(|entity| {
                *entity == button
                    || app
                        .world()
                        .get::<ChildOf>(*entity)
                        .is_some_and(|parent| parent.parent() == button)
            });
            let button_survived_hover = app.world().get_entity(button).is_ok();
            assert_eq!(app.world().resource::<Table>().view().revision, 0);
            for state in [
                bevy::input::ButtonState::Pressed,
                bevy::input::ButtonState::Released,
            ] {
                if raw_window_events {
                    app.world_mut()
                        .write_message(bevy::window::WindowEvent::MouseButtonInput(
                            bevy::input::mouse::MouseButtonInput {
                                button: MouseButton::Left,
                                state,
                                window,
                            },
                        ));
                } else {
                    let action = match state {
                        bevy::input::ButtonState::Pressed => {
                            PointerAction::Press(PointerButton::Primary)
                        }
                        bevy::input::ButtonState::Released => {
                            PointerAction::Release(PointerButton::Primary)
                        }
                    };
                    app.world_mut().write_message(PointerInput::new(
                        PointerId::Mouse,
                        location.clone(),
                        action,
                    ));
                }
                if !same_frame {
                    update(&mut app);
                }
            }
            update(&mut app);
            let view = app.world().resource::<Table>().view();
            assert_eq!(
                view.revision, 1,
                "scale={scale}, same_frame={same_frame}, hit_button={hit_button}, button_survived_hover={button_survived_hover}"
            );
            assert!(hit_button, "production picking must hit the lead button");
            assert_eq!(view.stage, crate::gathering::Stage::Mulligan);
            let saved = continuity::load_session(&path).unwrap().observe();
            assert_eq!(saved, app.world().resource::<Table>().session.observe());
            finish_presentation(&mut app);
            assert_eq!(saved, app.world().resource::<Table>().view());
        }
    }
}

#[test]
fn reopening_invalidates_a_choice_queued_from_the_previous_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gathering.json");
    let mut app = host(path.clone());
    continuity::save_session(&path, &app.world().resource::<Table>().session).unwrap();
    let before = std::fs::read(&path).unwrap();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::Reopen, Action::Choose(0, 0, "lead:tony".into())]);
    app.update();
    assert_eq!(app.world().resource::<Table>().view().revision, 0);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn failed_native_save_preserves_the_live_decision_and_existing_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let obstruction = dir.path().join("obstruction");
    std::fs::write(&obstruction, "existing file").unwrap();
    let mut app = host(obstruction.join("gathering.json"));
    let before = app.world().resource::<Table>().view();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Choose(0, 0, "lead:tony".into()));
    app.update();
    let table = app.world().resource::<Table>();
    assert_eq!(table.view(), before);
    assert!(table.notice.contains("not committed"));
    assert_eq!(
        std::fs::read_to_string(obstruction).unwrap(),
        "existing file"
    );
}

#[test]
fn party_inspection_contract_blocks_queued_background_actions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("modal.json");
    let mut app = host(path.clone());
    let before = app.world().resource::<Table>().session.clone();
    let camera = app.world().resource::<Orbit>().pose();
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::Inspect("Tony Morgan".into(), "06003".into()),
        Action::Save,
        Action::Focus(Area::Scenario),
    ]);
    app.update();
    let table = app.world().resource::<Table>();
    assert!(
        table.inspection.is_some(),
        "Inspection remains the blocking dialog"
    );
    assert!(table.inspection.is_some());
    assert_eq!(table.session, before);
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
    assert!(!path.exists(), "Background Save is blocked by inspection");
}

#[test]
fn party_utilities_stay_in_menu_instead_of_the_live_decision_panel() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = host(dir.path().join("utilities.json"));
    assert!(button_with_label(&mut app, "Menu").is_some());
    for utility in ["History", "Save", "Reopen", "Replay", "Choose asset folder"] {
        assert!(
            button_with_label(&mut app, utility).is_none(),
            "{utility} belongs in Menu"
        );
    }
}

pub(super) fn rendered_text(app: &mut App) -> String {
    let world = app.world_mut();
    // Semantic text assertions read the English label of rich-text glyph runs.
    // Dedicated symbol tests separately check the actual spans and font handles.
    let mut query = world.query::<(&Text, Option<&AccessibleLabel>)>();
    query
        .iter(world)
        .map(|(text, label)| {
            if text.0.is_empty() {
                crate::tabletop_support::readable_text(
                    label.map_or("", |label| label.0.as_str()).to_owned(),
                )
            } else {
                text.0.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn production_renderer_consumes_live_locations_enemies_assets_and_discard_inspection() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = host(dir.path().join("runtime.json"));
    // A fixture exercises the production adapter without weakening native entry guards.
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringPriestDefeat);
    let view = session.observe();
    let play = view.play.as_ref().unwrap();
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = session;
        table.dirty = true;
    }
    app.update();
    let text = rendered_text(&mut app);
    assert!(text.contains(&format!("Agenda {}", play.agenda)));
    assert!(text.contains("Total doom"));
    assert!(text.contains(&format!("Act {}", play.act)));
    for enemy in &play.scenario.enemies {
        assert!(text.contains(&enemy.card.title));
        assert!(!text.contains(&format!("copy {}", enemy.card.id)));
    }
    for player in &play.players {
        for asset in &player.assets {
            assert!(text.contains(&asset.card.title));
            assert!(!text.contains(&format!("copy {}", asset.card.id)));
        }
    }
    assert!(inspection_text(&view, "01112").unwrap().contains("Hallway"));
    assert!(
        inspection_text(&view, "01116")
            .unwrap()
            .contains("Ghoul Priest")
    );
    assert!(
        inspection_text(&view, "discard:tony")
            .unwrap()
            .contains("discard")
    );
    let world = app.world_mut();
    let mut labels = world.query::<(&Text, &motion::CardLabel)>();
    let mut room_position = |name: &str| {
        let (_, label) = labels
            .iter(world)
            .find(|(text, _)| text.0.starts_with(name))
            .unwrap();
        world.get::<Transform>(label.0).unwrap().translation
    };
    let hallway = room_position("Hallway");
    let attic = room_position("Attic");
    let cellar = room_position("Cellar");
    let parlor = room_position("Parlor");
    assert!(attic.z < hallway.z && cellar.z > hallway.z && parlor.x > hallway.x);
    assert_eq!(
        (attic.x, cellar.x, parlor.z),
        (hallway.x, hallway.x, hallway.z)
    );
    assert_eq!(
        world
            .query_filtered::<Entity, With<crate::tabletop_gathering_map::ConnectionVisual>>()
            .iter(world)
            .count(),
        3
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::History);
    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, With<crate::tabletop_gathering_map::ConnectionVisual>>()
            .iter(world)
            .count(),
        3,
        "Rerender replaces the links with the scene"
    );
}

#[test]
fn card_counters_keep_typed_accessible_uses_and_undamaged_enemy_capacity() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Tony(
        crate::gathering::TonyEffectFixture::GregoryCigarette,
    ));
    let mut app = host_with_table(Table::new(session, PathBuf::new()), false);
    let text = rendered_text(&mut app);
    for expected in ["Ammo 3", "Bounties 5", "Bounties 1", "Damage 0/2"] {
        assert!(
            app.world_mut()
                .query::<&AccessibleLabel>()
                .iter(app.world())
                .any(|label| label.0 == expected),
            "Missing {expected:?}"
        );
    }
    assert!(text.contains("0/2"));
    assert!(!text.contains("Uses 3"));
    assert!(!text.contains("Damage 0  /  Doom"));
}

#[test]
fn unrevealed_room_inspection_preserves_the_observed_face() {
    let view = Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd)
        .observe();
    let room = view
        .play
        .as_ref()
        .unwrap()
        .scenario
        .locations
        .iter()
        .find(|room| !room.revealed)
        .unwrap();
    let text =
        inspection_text(&view, &room.code).expect("An in-play unrevealed room can be inspected");
    assert_eq!(
        text,
        crate::gathering::inspection::scenario_card(&room.code, false).unwrap()
    );
    assert_ne!(
        text,
        crate::gathering::inspection::scenario_card(&room.code, true).unwrap()
    );
    assert!(
        inspection_text(&view, "01118").is_none(),
        "Set-aside hidden enemies are not visible"
    );
}

#[test]
fn unrevealed_room_mesh_and_inspection_never_use_revealed_art_as_a_fallback() {
    for include_unrevealed_art in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let mut app = host(directory.path().join("hidden-room.json"));
        let (revealed, unrevealed) = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            (images.add(Image::default()), images.add(Image::default()))
        };
        let mut handles = vec![("card-01115", revealed.clone())];
        if include_unrevealed_art {
            handles.push(("card-01115-back", unrevealed.clone()));
        }
        app.insert_resource(art::Art::with_images_for_test(handles));
        {
            let mut table = app.world_mut().resource_mut::<Table>();
            table.session =
                Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
            assert!(
                table
                    .view()
                    .play
                    .unwrap()
                    .scenario
                    .locations
                    .iter()
                    .any(|room| { room.code == "01115" && !room.revealed })
            );
            table.dirty = true;
        }
        app.update();
        let world = app.world_mut();
        let room = world
            .query::<(Entity, &ContextTarget)>()
            .iter(world)
            .find_map(|(entity, target)| {
                (target.source == controls::Source::Location("01115".into())).then_some(entity)
            })
            .unwrap();
        let textures: Vec<_> = world
            .get::<Children>(room)
            .into_iter()
            .flat_map(|children| children.iter())
            .chain(std::iter::once(room))
            .filter_map(|child| world.get::<MeshMaterial3d<StandardMaterial>>(child))
            .filter_map(|material| {
                world
                    .resource::<Assets<StandardMaterial>>()
                    .get(&material.0)
            })
            .filter_map(|material| material.base_color_texture.clone())
            .collect();
        assert!(
            !textures.contains(&revealed),
            "The hidden room cannot expose its revealed face"
        );
        assert_eq!(textures.contains(&unrevealed), include_unrevealed_art);
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::Inspect("Parlor".into(), "01115".into()));
        app.update();
        let world = app.world_mut();
        let inspection_images: Vec<_> = world
            .query::<&ImageNode>()
            .iter(world)
            .map(|image| image.image.clone())
            .collect();
        assert_eq!(
            inspection_images,
            if include_unrevealed_art {
                vec![unrevealed]
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn ordinary_act_and_agenda_inspect_landscape_without_rotating_the_book_face_twice() {
    use bevy::camera::primitives::MeshAabb;
    for (source, role) in [
        (controls::Source::Act, "card-01108"),
        (controls::Source::Agenda, "card-01105"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut app = host(directory.path().join("scenario-inspection.json"));
        let image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        app.insert_resource(art::Art::with_images_for_test([(role, image.clone())]));
        activate_choice(&mut app, "lead:tony");
        for _ in 0..3 {
            activate_choice(&mut app, "confirm-mulligan");
        }
        finish_presentation(&mut app);
        let world = app.world_mut();
        let root = world
            .query::<(Entity, &ContextTarget)>()
            .iter(world)
            .find_map(|(entity, target)| (target.source == source).then_some(entity))
            .unwrap();
        let root_transform = world.get::<Transform>(root).unwrap();
        assert!(
            root_transform.rotation.abs_diff_eq(
                Quat::from_rotation_x(75_f32.to_radians())
                    * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                0.0001
            ),
            "The book root already rotates the original portrait image into a landscape page"
        );
        let face = root;
        let material = world.get::<MeshMaterial3d<StandardMaterial>>(face).unwrap();
        assert_eq!(
            world
                .resource::<Assets<StandardMaterial>>()
                .get(&material.0)
                .unwrap()
                .base_color_texture
                .as_ref(),
            Some(&image),
            "The Party plane carries the original face on the book root"
        );
        let mesh = world.get::<Mesh3d>(face).unwrap();
        let bounds = world
            .resource::<Assets<Mesh>>()
            .get(&mesh.0)
            .unwrap()
            .get_aabb()
            .unwrap();
        assert!((bounds.half_extents.x * 2. - 1.35).abs() < 0.0001);
        assert!((bounds.half_extents.z * 2. - 1.89).abs() < 0.0001);

        click_source(&mut app, source);
        if app.world().resource::<Table>().context.is_open() {
            let entity = button_with_label(&mut app, "Inspect").unwrap();
            app.world_mut().trigger(Activate { entity });
            app.update();
        }
        let world = app.world_mut();
        let (image_node, node, transform, parent) = world
            .query::<(&ImageNode, &Node, &UiTransform, &ChildOf)>()
            .single(world)
            .unwrap();
        assert_eq!(image_node.image, image);
        assert_eq!(
            transform.rotation,
            Rot2::degrees(-90.),
            "The inspector must rotate sideways Act/Agenda artwork into readable landscape"
        );
        let frame = world.get::<Node>(parent.parent()).unwrap();
        let (Val::Px(width), Val::Px(height)) = (frame.width, frame.height) else {
            panic!("The image frame must have measured dimensions")
        };
        assert!(width > height);
        assert!((width / height - 7. / 5.).abs() < 0.0001);
        assert_eq!((node.width, node.height), (Val::Px(height), Val::Px(width)));
    }
}

#[test]
fn a_pickable_standee_mesh_bubbles_to_its_investigator_menu() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("standee.json"));
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session =
            Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringPriestDefeat);
        table.dirty = true;
    }
    app.update();
    let world = app.world_mut();
    let root = world
        .query_filtered::<(Entity, &ContextTarget), Without<Mesh3d>>()
        .iter(world)
        .find_map(|(entity, target)| {
            (target.source == controls::Source::Investigator(Investigator::Tony)).then_some(entity)
        })
        .expect("Tony has a rendered standee root");
    let mesh = world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .find(|child| world.get::<Mesh3d>(*child).is_some())
        .expect("The standee has a pickable mesh child");
    assert!(
        world
            .get::<Pickable>(mesh)
            .is_none_or(|pickable| pickable.is_hoverable)
    );
    let before = world.resource::<Table>().view();
    click_mesh_observer(&mut app, mesh);
    assert!(
        app.world()
            .resource::<Table>()
            .context
            .is_menu_source(&controls::Source::Investigator(Investigator::Tony))
    );
    assert_eq!(app.world().resource::<Table>().view(), before);
}

fn choice_source(app: &mut App, id: &str) -> Option<controls::Source> {
    let view = app.world().resource::<Table>().view();
    let world = app.world_mut();
    world
        .query::<&ContextTarget>()
        .iter(world)
        .find_map(|target| {
            controls::source_choices(&view, &target.source)
                .iter()
                .any(|choice| choice.id == id)
                .then(|| target.source.clone())
        })
}

/// Pay the offered group clue payment through its chooser: open it from the
/// listed choice and spend its pre-filled split, which must be the only one.
pub(super) fn pay_group_clues(app: &mut App) {
    activate_choice(app, "act:pay:<contributions>");
    let required = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .group_clue_payment
        .expect("An offered group clue payment")
        .required;
    let spend = choice_button(app, &format!("Spend {required} clues and advance"))
        .expect("The chooser offers its spend button");
    app.world_mut().trigger(Activate { entity: spend });
    app.update();
}

fn activate_choice(app: &mut App, id: &str) {
    finish_presentation(app);
    let view = app.world().resource::<Table>().view();
    let choice = view.choices.iter().find(|choice| choice.id == id).unwrap();
    let source = choice_source(app, id);
    let displayed = source.as_ref().and_then(|source| {
        controls::source_choices(&view, source)
            .into_iter()
            .find(|choice| choice.id == id)
    });
    let label = ui::choice_label(displayed.as_ref().unwrap_or(choice)).to_owned();
    if choice_button(app, &label).is_none() {
        if let Some(source) = source {
            click_source(app, source);
        } else {
            let pending = button_with_label(app, "Pending decision").expect(
                "A legal choice must have a visible card source or pending-decision control",
            );
            app.world_mut().trigger(Activate { entity: pending });
            app.update();
        }
    }
    let entity = choice_button(app, &label).unwrap_or_else(|| {
        panic!("The selected context or global controls must offer {id}: {label}")
    });
    app.world_mut().trigger(Activate { entity });
    app.update();
}

/// Prefer the active modal's action over an identically captioned shortcut.
/// Use the full accessible label so symbol spans do not erase action identity.
pub(super) fn choice_button(app: &mut App, label: &str) -> Option<Entity> {
    use bevy::input_focus::tab_navigation::TabGroup;
    let world = app.world_mut();
    let menu_label = feedback::menu::label(label);
    let mut candidates: Vec<_> = world
        .query_filtered::<(Entity, &Children, Option<&AccessibleLabel>), With<Button>>()
        .iter(world)
        .filter(|(_, children, accessible)| {
            accessible.is_some_and(|text| text.0 == label || text.0 == menu_label)
                || children.iter().any(|child| {
                    world
                        .get::<Text>(child)
                        .is_some_and(|text| text.0 == label || text.0 == menu_label)
                })
        })
        .map(|(entity, _, _)| {
            let mut parent = world.get::<ChildOf>(entity).map(ChildOf::parent);
            let mut modal = false;
            while let Some(ancestor) = parent {
                modal |= world
                    .get::<TabGroup>(ancestor)
                    .is_some_and(|group| group.modal);
                parent = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
            }
            (entity, modal)
        })
        .collect();
    if candidates.iter().any(|(_, modal)| *modal) {
        candidates.retain(|(_, modal)| *modal);
    }
    assert!(
        candidates.len() <= 1,
        "Ambiguous action {label:?}: {} matching controls",
        candidates.len()
    );
    candidates.first().map(|(entity, _)| *entity)
}

#[test]
#[should_panic(expected = "Ambiguous action")]
fn choice_selector_rejects_duplicate_actions_in_the_active_menu() {
    let mut app = App::new();
    app.world_mut()
        .spawn(bevy::input_focus::tab_navigation::TabGroup::modal())
        .with_children(|parent| {
            for _ in 0..2 {
                parent
                    .spawn((Button, AccessibleLabel::new("Reveal")))
                    .with_child(Text::new("Reveal"));
            }
        });
    choice_button(&mut app, "Reveal");
}

/// Wait for the current presentation to finish or reach a player-controlled pause.
/// This does not settle later automatic choices, card motion or closing trays.
pub(super) fn finish_presentation(app: &mut App) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(100));
    for _ in 0..200 {
        let table = app.world().resource::<Table>();
        if !table.is_presenting() || table.presentation_blocked() {
            break;
        }
        app.update();
    }
    let table = app.world().resource::<Table>();
    assert!(
        !table.is_presenting() || table.presentation_blocked(),
        "live presentation must finish automatically"
    );
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::ZERO);
}

/// Tick production systems until automatic work settles or an encounter awaits
/// reading. Never choose a player option or dismiss a reading stop.
pub(super) fn settle_automatic_resolution(app: &mut App) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(100));
    let mut remaining = String::new();
    for _ in 0..200 {
        app.update();
        let world = app.world_mut();
        let moving = world
            .query_filtered::<Entity, With<motion::CardMotion>>()
            .iter(world)
            .count();
        let table = world.resource::<Table>();
        let view = table.view();
        let consequences = world.resource::<consequences::State>().active();
        let queued = world.resource::<Actions>().0.len();
        if table.presentation.encounter().current().is_some()
            || (!table.is_presenting()
                && !table.trays.active()
                && moving == 0
                && !consequences
                && queued == 0)
        {
            world
                .resource_mut::<PresentationTime>()
                .advance_by(std::time::Duration::ZERO);
            return;
        }
        remaining = format!(
            "pending={:?}, presenting={}, moving={moving}, trays_active={}, consequences={consequences}, queued={queued}, blocked={}, notice={:?}",
            view.play.as_ref().map(|play| &play.pending),
            table.is_presenting(),
            table.trays.active(),
            table.presentation_blocked(),
            table.notice,
        );
    }
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::ZERO);
    panic!("automatic resolution did not settle after 200 updates: {remaining}");
}

#[test]
fn live_runtime_choice_keeps_manual_camera() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("runtime.json");
    let mut app = host(path.clone());
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session =
            Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringPriestDefeat);
        table.dirty = true;
    }
    app.update();
    let pose = app.world().resource::<Orbit>().pose();
    assert!(!app.world().resource::<Orbit>().is_following());
    activate_choice(&mut app, "action:ordinary:fight:400");
    finish_presentation(&mut app);
    assert_eq!(app.world().resource::<Orbit>().pose(), pose);
    let view = app.world().resource::<Table>().view();
    let test = view.play.as_ref().unwrap().test.as_ref().unwrap();
    let text = rendered_text(&mut app);
    assert!(text.contains(&format!("base Combat {}", test.base_skill)));
    assert!(text.contains(&format!("difficulty {}", test.difficulty)));
    assert_eq!(continuity::load_session(&path).unwrap().observe(), view);
}

#[test]
fn actual_search_pile_menu_takes_focus_and_scrolls_by_keyboard_without_choosing() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("long-list.json"));
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Backpack(
            crate::gathering::BackpackFixture::Search,
        ));
        table.dirty = true;
    }
    {
        let world = app.world_mut();
        let mut windows = world.query::<&mut Window>();
        windows
            .single_mut(world)
            .unwrap()
            .resolution
            .set(800., 500.);
    }
    for _ in 0..5 {
        app.update();
    }
    let pack = app.world().resource::<Table>().view().play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "04037")
        .unwrap()
        .id;
    activate_choice(&mut app, &format!("action:ordinary:play:{pack}"));
    activate_choice(&mut app, "backpack:search");
    finish_presentation(&mut app);
    click_source(&mut app, controls::Source::SearchPlayer);
    for _ in 0..3 {
        app.update();
    }
    let before = app.world().resource::<Table>().view();
    assert_eq!(before.play.as_ref().unwrap().searched_player.len(), 6);
    let focus = app
        .world()
        .resource::<InputFocus>()
        .get()
        .expect("The shared contextual menu receives focus");
    assert!(app.world().get::<Button>(focus).is_some());
    let window = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<Window>>()
            .single(world)
            .unwrap()
    };
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::End,
            logical_key: bevy::input::keyboard::Key::End,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
    app.update();
    let world = app.world_mut();
    let (scroll, node) = world
        .query_filtered::<(&ScrollPosition, &ComputedNode), With<controls::Menu>>()
        .single(world)
        .unwrap();
    assert!(
        node.content_size().y > node.size().y,
        "A real long list exceeds the available panel"
    );
    assert!(scroll.y > 0.);
    assert_eq!(
        app.world().resource::<Table>().view(),
        before,
        "Focus and scrolling cannot select an action"
    );
}

#[test]
fn native_paid_ward_and_unapplied_horror_remain_visible_and_inspectable() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("ward.json"));
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Harm(
            crate::gathering::HarmFixture::WardCancellation,
        ));
        table.dirty = true;
    }
    app.update();
    let view = app.world().resource::<Table>().view();
    let id = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("ward:"))
        .unwrap()
        .id
        .clone();
    activate_choice(&mut app, &id);
    finish_presentation(&mut app);
    let view = app.world().resource::<Table>().view();
    assert!(
        inspection_text(&view, "resolving-cards")
            .unwrap()
            .contains("Ward of Protection")
    );
    assert!(
        inspection_text(&view, "01065")
            .unwrap()
            .contains("Ward of Protection")
    );
    let text = rendered_text(&mut app);
    assert!(text.contains("1 horror on Jacqueline Fine"));
    assert!(text.contains("Jacqueline Fine  /  Resolving"));
}

#[test]
fn simultaneous_weaknesses_remain_inspectable_at_their_order_decision() {
    use crate::gathering::{DrawFixture, HostileFixture};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("weakness-order.json");
    let mut app = host(path.clone());
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = Session::with_hostile_fixture(HostileFixture::Draws(
            DrawFixture::JacquelineWeaknessOrder,
        ));
        table.dirty = true;
    }
    app.update();
    activate_choice(&mut app, "result:0");
    finish_presentation(&mut app);
    let view = app.world().resource::<Table>().view();
    let held = &view.play.as_ref().unwrap().pending.resolving_cards;
    assert_eq!(
        held.len(),
        2,
        "Both drawn revelations remain public while their order is chosen"
    );
    for (code, title) in [("02039", "Chronophobia"), ("60403", "Dark Future")] {
        assert!(inspection_text(&view, code).unwrap().contains(title));
        assert!(
            inspection_text(&view, "resolving-cards")
                .unwrap()
                .contains(title)
        );
    }
    assert_eq!(continuity::load_session(path).unwrap().observe(), view);
}

#[test]
fn native_automatic_reveal_is_presented_after_commit_and_cannot_accept_rules_input() {
    use crate::gathering::{HostileFixture, SelectedWitness};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reveal.json");
    let mut app = host(path.clone());
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = Session::with_hostile_fixture(HostileFixture::SelectedWitness(
            SelectedWitness::GraspingHands,
        ));
        table.dirty = true;
    }
    app.update();
    settle_automatic_resolution(&mut app);
    loop {
        finish_presentation(&mut app);
        let view = app.world().resource::<Table>().view();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        // The witness poses its revelation step; the table never commits it.
        let id = ["revelation:resolve", "window:pass", "commit:confirm"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .unwrap();
        activate_choice(&mut app, id);
    }
    activate_choice(&mut app, "test:reveal");
    let final_view = continuity::load_session(&path).unwrap().observe();
    assert_eq!(final_view.play.as_ref().unwrap().players[0].damage, 2);
    assert_eq!(final_view.play.as_ref().unwrap().bag_count, 16);
    let saved = std::fs::read(&path).unwrap();
    let table = app.world().resource::<Table>();
    assert!(table.is_presenting());
    assert!(table.view().choices.is_empty());
    assert_eq!(table.view().play.as_ref().unwrap().bag_count, 15);
    let blocked = Action::Choose(
        table.epoch,
        final_view.revision,
        final_view.choices[0].id.clone(),
    );
    let first_view = table.view();
    app.world_mut().resource_mut::<Actions>().0.push(blocked);
    app.update();
    assert_eq!(app.world().resource::<Table>().view(), first_view);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    assert!(!rendered_text(&mut app).contains("Next event"));
    // Inspection pauses the real presentation clock, without hiding its token face.
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::History);
    app.update();
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_secs(10));
    app.update();
    assert_eq!(app.world().resource::<Table>().view(), first_view);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::ZERO);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    app.update();
    assert_eq!(
        app.world().resource::<Table>().session.observe(),
        final_view
    );
    finish_presentation(&mut app);
    let result = button_with_label(&mut app, "Continue")
        .expect("Encounter tests also wait for acknowledgment");
    app.world_mut().trigger(Activate { entity: result });
    app.update();
    finish_presentation(&mut app);
    assert_eq!(app.world().resource::<Table>().view(), final_view);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

fn physical_position(app: &mut App, id: u16) -> Vec3 {
    let world = app.world_mut();
    let mut cards = world.query::<(&motion::PhysicalCard, &Transform)>();
    let positions: Vec<_> = cards
        .iter(world)
        .filter(|(card, _)| card.0 == id)
        .map(|(_, transform)| transform.translation)
        .collect();
    assert_eq!(positions.len(), 1, "Exactly one physical card is rendered");
    positions[0]
}

#[test]
fn committed_physical_card_moves_from_hand_and_inspection_preserves_its_position() {
    use crate::gathering::{HostileFixture, TestCardFixture};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("motion.json");
    let mut app = host(path.clone());
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session =
            Session::with_hostile_fixture(HostileFixture::TestCards(TestCardFixture::PromiseCurse));
        table.dirty = true;
    }
    app.update();
    let promise = app.world().resource::<Table>().view().play.unwrap().players[1]
        .hand
        .iter()
        .find(|card| card.code == "07032")
        .unwrap()
        .id;
    let origin = physical_position(&mut app, promise);
    activate_choice(&mut app, "window:pass");
    activate_choice(&mut app, &format!("commit:{promise}"));
    activate_choice(&mut app, "commit:confirm");
    let summary = rendered_text(&mut app);
    assert!(
        summary.contains("cards +4") && summary.contains("chaos pending"),
        "{summary}"
    );
    assert_eq!(physical_position(&mut app, promise), origin);
    let end = layout::test_commitment(1, 0);
    assert_ne!(origin, end);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(150));
    app.update();
    let halfway = physical_position(&mut app, promise);
    assert!(halfway.distance(end) < origin.distance(end));
    assert!(halfway.distance(origin) > 0.);
    let before = std::fs::read(&path).unwrap();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Inspect("Promise of Power".into(), "07032".into()));
    app.update();
    let held = physical_position(&mut app, promise);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(200));
    app.update();
    assert_eq!(physical_position(&mut app, promise), held);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    app.update();
    app.update();
    assert!(physical_position(&mut app, promise).abs_diff_eq(end, 0.001));
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn story_reading_reuses_party_barrier_and_continue_never_reapplies_saved_effects() {
    use crate::gathering::HostileFixture;
    use bevy::ui_widgets::{ModalDialog, ModalDialogBarrier};
    let session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("story-reading.json");
    let mut app = host(path.clone());
    app.insert_resource(Table::new(session, path.clone()));
    app.update();
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .story()
            .is_none()
    );
    pay_group_clues(&mut app);
    for _ in 0..30 {
        if app
            .world()
            .resource::<Table>()
            .presentation
            .story()
            .is_some()
        {
            break;
        }
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
    }
    assert_eq!(
        app.world().resource::<Table>().presentation.story(),
        Some(story::StoryCard::ActTwo)
    );
    let size = Vec2::new(1600., 1000.);
    crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
    app.world_mut().resource_mut::<Table>().dirty = true;
    for _ in 0..4 {
        app.update();
    }
    let button = button_with_label(&mut app, "Continue").unwrap();
    let world = app.world_mut();
    let panel = world
        .query_filtered::<Entity, With<ModalDialog>>()
        .single(world)
        .unwrap();
    let height = world.get::<ComputedNode>(panel).unwrap().size().y;
    assert!(height < 650., "Short Act story grew to {height}");
    let center = world.get::<UiGlobalTransform>(panel).unwrap().translation;
    let button_center = world.get::<UiGlobalTransform>(button).unwrap().translation;
    assert!((center.x - button_center.x).abs() < 1., "Center Continue");
    assert_unclipped_button(&mut app, "Continue", size, 1.);
    let saved = std::fs::read(&path).unwrap();
    let record = app.world().resource::<Table>().session.clone();
    assert_eq!(
        app.world_mut()
            .query::<&ModalDialog>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query::<&ModalDialogBarrier>()
            .iter(app.world())
            .count(),
        1
    );
    let pose = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::Close, Action::Focus(Area::Investigator(2))]);
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_secs(20));
    app.update();
    assert_eq!(
        app.world().resource::<Table>().presentation.story(),
        Some(story::StoryCard::ActTwo)
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), pose);
    let continue_button = button_with_label(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate {
        entity: continue_button,
    });
    app.update();
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .story()
            .is_none()
    );
    assert_eq!(app.world().resource::<Table>().session.clone(), record);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    let resumed = continuity::load_session(&path).unwrap();
    app.insert_resource(Table::new(resumed, path.clone()));
    app.update();
    assert_eq!(
        app.world().resource::<Table>().presentation.story(),
        Some(story::StoryCard::ActTwo)
    );
    assert_eq!(std::fs::read(path).unwrap(), saved);
}
