use super::*;
use crate::tony::HarmSource;

// Exercise the production action/timer schedule without constructing a scene.
fn lifetime_app(path: PathBuf) -> App {
    let mut app = App::new();
    app.insert_resource(Table {
        session: Session::with_draws(vec![TokenFace::Zero]),
        replay: None,
        save_path: path.clone(),
        notice: String::new(),
        inspect: None,
        history: false,
        dirty: true,
        presentation: HarmPresentation::default(),
        reduced_motion: false,
        context: Context::default(),
    })
    .insert_resource(Orbit::new(CameraMode::Tony))
    .insert_resource(RenderHealth::ready_for_test())
    .insert_resource(Actions::default())
    .insert_resource(AssetSettings::new_tony(AssetConfig::load_at(
        path.with_extension("toml"),
        path.with_extension("assets"),
    )))
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<Time>()
    .add_systems(Update, (handle_actions, advance_presentation).chain());
    app
}

fn progress(app: &App) -> f32 {
    app.world()
        .resource::<Table>()
        .presentation
        .animation_progress()
}

#[test]
fn rendering_check_leaves_committed_harm_session_and_save_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = lifetime_app(path.clone());
    choose(&mut app, "investigate");
    assert!(app.world().resource::<Table>().presenting());
    let before = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    app.insert_resource(RenderHealth::default());
    advance(&mut app, 5.0);
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn modals_and_asset_settings_pause_harm_but_context_and_utilities_do_not() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = lifetime_app(directory.path().join("turn.json"));
    choose(&mut app, "investigate");
    advance(&mut app, 0.25);
    assert_eq!(progress(&app), 0.5);

    for modal in [Action::Inspect(0), Action::History] {
        act(&mut app, modal);
        advance(&mut app, 5.0);
        assert_eq!(progress(&app), 0.5);
        assert!(app.world().resource::<Table>().presenting());
        act(&mut app, Action::Close);
    }

    let settings = app.world_mut().remove_resource::<AssetSettings>().unwrap();
    app.insert_resource(settings.with_pending_selection());
    advance(&mut app, 5.0);
    assert_eq!(progress(&app), 0.5);
    // The disconnected external selection is released by the normal polling seam.
    app.world_mut().resource_mut::<AssetSettings>().poll();

    act(&mut app, Action::Utilities);
    assert!(app.world().resource::<Table>().context.utilities_open());
    advance(&mut app, 0.125);
    assert_eq!(progress(&app), 0.75);
    let revision = app.world().resource::<Table>().view().revision;
    act(
        &mut app,
        Action::Select(revision, Source::Card(0), Vec3::ZERO),
    );
    assert_eq!(app.world().resource::<Table>().inspect, Some(0));
    advance(&mut app, 0.125);
    assert_eq!(
        progress(&app),
        0.75,
        "direct inspection pauses harm like the explicit Inspect action"
    );
    act(&mut app, Action::Close);
    // Inspect-only components now pause through the same direct inspector.
    act(&mut app, Action::Select(revision, Source::Bag, Vec3::ZERO));
    assert_eq!(app.world().resource::<Table>().inspect, Some(7));
    assert!(!app.world().resource::<Table>().context.is_open());
    advance(&mut app, 0.125);
    assert_eq!(progress(&app), 0.75);
    act(&mut app, Action::Close);
    // A deck context has no inspection target and does not pause presentation.
    act(&mut app, Action::Select(revision, Source::Deck, Vec3::ZERO));
    assert!(app.world().resource::<Table>().context.is_open());
    assert!(app.world().resource::<Table>().inspect.is_none());
    advance(&mut app, 0.125);
    assert_eq!(progress(&app), 1.0);
    advance(&mut app, 0.71);
    assert!(!app.world().resource::<Table>().presenting());
}

#[test]
fn closing_a_modal_releases_the_timer_in_the_same_update() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = lifetime_app(directory.path().join("turn.json"));
    choose(&mut app, "investigate");
    act(&mut app, Action::History);
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    advance(&mut app, 1.2);
    assert!(!app.world().resource::<Table>().presenting());
}

#[test]
fn queued_finishes_precede_one_timer_completion_and_discard_excess_time() {
    for (finishes, expected) in [(0, HarmSource::BasicFight), (1, HarmSource::InternalInjury)] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("turn.json");
        let mut app = lifetime_app(path.clone());
        final_tablet_reveal(&mut app);
        choose(&mut app, "resolve");
        let committed = app.world().resource::<Table>().session.clone();
        let saved = std::fs::read(&path).unwrap();
        advance(&mut app, 0.3);
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .extend(std::iter::repeat_n(Action::FinishAnimation, finishes));
        advance(&mut app, 5.0);
        assert_eq!(
            app.world().resource::<Table>().harm().unwrap().source,
            expected
        );
        assert_eq!(progress(&app), 0.0);
        advance(&mut app, 1.0);
        assert_eq!(
            app.world().resource::<Table>().harm().unwrap().source,
            expected
        );
        assert_eq!(app.world().resource::<Table>().session, committed);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
    }

    let directory = tempfile::tempdir().unwrap();
    let mut app = lifetime_app(directory.path().join("turn.json"));
    final_tablet_reveal(&mut app);
    choose(&mut app, "resolve");
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::FinishAnimation, Action::FinishAnimation]);
    advance(&mut app, 0.25);
    assert_eq!(
        app.world().resource::<Table>().harm().unwrap().source,
        HarmSource::InternalInjury
    );
    assert_eq!(progress(&app), 0.5);
}

#[test]
fn a_newly_committed_harm_stage_consumes_the_current_frames_delta() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = lifetime_app(directory.path().join("turn.json"));
    let revision = app.world().resource::<Table>().session.observe().revision;
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Choose(revision, "investigate".into()));
    advance(&mut app, 0.25);
    assert!(app.world().resource::<Table>().presenting());
    assert_eq!(progress(&app), 0.5);
}

#[test]
fn reopen_discards_harm_only_after_loading_succeeds_and_save_does_not_consume_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = lifetime_app(path.clone());
    choose(&mut app, "investigate");
    advance(&mut app, 0.25);
    let committed = app.world().resource::<Table>().session.clone();
    let staged = app.world().resource::<Table>().view();
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::Save);
    assert_eq!(app.world().resource::<Table>().view(), staged);
    assert_eq!(progress(&app), 0.5);
    assert_eq!(std::fs::read(&path).unwrap(), saved);

    std::fs::write(&path, "invalid saved turn").unwrap();
    act(&mut app, Action::Reopen);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .contains("Reopen failed")
    );
    assert_eq!(app.world().resource::<Table>().view(), staged);
    assert_eq!(progress(&app), 0.5);
    assert_eq!(app.world().resource::<Table>().session, committed);
    std::fs::write(&path, &saved).unwrap();
    act(&mut app, Action::Reopen);
    assert!(!app.world().resource::<Table>().presenting());
    assert_eq!(app.world().resource::<Table>().view(), committed.observe());
    assert_eq!(std::fs::read(&path).unwrap(), saved);
}

#[test]
fn replay_animates_without_timed_or_manual_advance_and_resets_at_both_endpoints() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = lifetime_app(path.clone());
    choose(&mut app, "investigate");
    advance(&mut app, 0.25);
    let saved = std::fs::read(&path).unwrap();
    act(&mut app, Action::Replay);
    assert_eq!(progress(&app), 0.0);
    act(&mut app, Action::Back);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .index(),
        0
    );
    assert_eq!(progress(&app), 0.0);
    act(&mut app, Action::Next);
    advance(&mut app, 0.25);
    assert_eq!(progress(&app), 0.5);
    act(&mut app, Action::FinishAnimation);
    assert_eq!(progress(&app), 0.5);
    advance(&mut app, 5.0);
    assert_eq!(progress(&app), 1.0);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .index(),
        1
    );
    act(&mut app, Action::Next);
    assert_eq!(progress(&app), 0.0);
    advance(&mut app, 5.0);
    act(&mut app, Action::Next);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .replay
            .as_ref()
            .unwrap()
            .index(),
        2
    );
    assert_eq!(progress(&app), 0.0);
    act(&mut app, Action::Back);
    assert_eq!(progress(&app), 0.0);
    assert!(app.world().resource::<Table>().harm().is_some());
    act(&mut app, Action::Live);
    assert!(!app.world().resource::<Table>().presenting());
    assert_eq!(app.world().resource::<Table>().view().stage, Stage::Commit);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}

#[test]
fn failed_replay_opening_retains_the_current_live_stage() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = lifetime_app(directory.path().join("turn.json"));
    choose(&mut app, "investigate");
    advance(&mut app, 0.25);
    let staged = app.world().resource::<Table>().view();
    let obstacle = directory.path().join("not-a-directory");
    std::fs::write(&obstacle, "preserve me").unwrap();
    app.world_mut().resource_mut::<Table>().save_path = obstacle.join("turn.json");
    act(&mut app, Action::Replay);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .contains("Could not open recorded replay")
    );
    assert!(app.world().resource::<Table>().replay.is_none());
    assert_eq!(app.world().resource::<Table>().view(), staged);
    assert_eq!(progress(&app), 0.5);
    assert_eq!(std::fs::read_to_string(obstacle).unwrap(), "preserve me");
}
