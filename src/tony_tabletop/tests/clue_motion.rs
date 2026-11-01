use super::*;
use crate::tony::HarmSource;

fn app_with_draws(path: PathBuf, draws: Vec<TokenFace>) -> App {
    let mut app = presentation_app(path);
    app.world_mut().resource_mut::<Table>().session = Session::with_draws(draws);
    app
}

fn finish_all(app: &mut App) {
    while app.world().resource::<Table>().presenting() {
        act(app, Action::FinishAnimation);
    }
}

fn investigate_to_resolve(app: &mut App, commitment: &str) {
    choose(app, "investigate");
    finish_all(app);
    choose(app, commitment);
    choose(app, "reveal");
}

fn counters(app: &App) -> (u8, u8) {
    let view = app.world().resource::<Table>().view();
    (view.study_clues, view.clues)
}

#[test]
fn production_registration_positions_the_clue_label_and_follows_its_flight() {
    use super::super::clue_motion::TravelingClueLabel;
    use bevy::camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo};

    let directory = tempfile::tempdir().unwrap();
    let mut app = app_with_draws(directory.path().join("turn.json"), vec![TokenFace::Zero]);
    // Substitute only renderer-owned viewport metadata. The actual application
    // registration installs the animation and CounterPlugin label projection.
    let mut projection = PerspectiveProjection::default();
    projection.update(1600.0, 1000.0);
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            computed: ComputedCameraValues {
                clip_from_view: projection.get_clip_from_view(),
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(1600, 1000),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            ..default()
        },
        GlobalTransform::from(Transform::from_xyz(0.0, 80.0, 45.0).looking_at(Vec3::ZERO, Vec3::Y)),
    ));
    investigate_to_resolve(&mut app, "commit:none");
    choose(&mut app, "resolve");
    let label_position = |app: &mut App| {
        let world = app.world_mut();
        let (node, visibility) = world
            .query_filtered::<(&Node, &Visibility), With<TravelingClueLabel>>()
            .single(world)
            .unwrap();
        assert_eq!(*visibility, Visibility::Visible);
        assert!(matches!(node.left, Val::Px(_)));
        assert!(matches!(node.top, Val::Px(_)));
        (node.left, node.top)
    };
    let departure = label_position(&mut app);
    advance(&mut app, 0.4);
    assert_ne!(label_position(&mut app), departure);
    assert_eq!(counters(&app), (5, 0));
    advance(&mut app, 0.41);
    assert_eq!(counters(&app), (5, 1));
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<(), With<TravelingClueLabel>>()
            .iter(world)
            .count(),
        0,
    );
}

#[test]
fn committed_discovery_departs_then_lands_without_repeating_after_scene_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = app_with_draws(path.clone(), vec![TokenFace::Zero, TokenFace::Zero]);
    for (old, remaining) in [(0, 5), (1, 4)] {
        investigate_to_resolve(&mut app, "commit:none");
        choose(&mut app, "resolve");
        assert_eq!(counters(&app), (remaining, old));
        assert_eq!(
            app.world().resource::<Table>().session.observe().clues,
            old + 1
        );
        assert_eq!(
            continuity::load_session(&path).unwrap().observe().clues,
            old + 1
        );
        assert_eq!(
            super::super::clue_motion::traveling_count(app.world_mut()),
            1
        );

        advance(&mut app, 0.3);
        let progress = app.world().resource::<Table>().presentation.clue_progress();
        act(&mut app, Action::Focus(Area::Investigator(0)));
        act(&mut app, Action::Utilities);
        assert_eq!(
            app.world().resource::<Table>().presentation.clue_progress(),
            progress
        );
        assert_eq!(
            super::super::clue_motion::traveling_count(app.world_mut()),
            1
        );
        assert_eq!(counters(&app), (remaining, old));

        advance(&mut app, 0.51);
        assert_eq!(counters(&app), (remaining, old + 1));
        assert_eq!(
            super::super::clue_motion::traveling_count(app.world_mut()),
            0
        );
        advance(&mut app, 2.0);
        assert_eq!(counters(&app), (remaining, old + 1));
    }
}

#[test]
fn failed_save_and_failed_investigation_do_not_start_a_clue_transfer() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = app_with_draws(path.clone(), vec![TokenFace::Zero]);
    investigate_to_resolve(&mut app, "commit:none");
    let original = std::fs::read(&path).unwrap();
    app.world_mut().resource_mut::<Table>().save_path = directory.path().to_owned();
    choose(&mut app, "resolve");
    assert_eq!(counters(&app), (6, 0));
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .clue()
            .is_none()
    );
    assert_eq!(
        super::super::clue_motion::traveling_count(app.world_mut()),
        0
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);

    let mut app = app_with_draws(
        directory.path().join("failure.json"),
        vec![TokenFace::AutoFail],
    );
    investigate_to_resolve(&mut app, "commit:none");
    choose(&mut app, "resolve");
    assert_eq!(counters(&app), (6, 0));
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .clue()
            .is_none()
    );
    assert_eq!(
        super::super::clue_motion::traveling_count(app.world_mut()),
        0
    );
}

#[test]
fn tablet_clue_and_end_turn_injury_keep_their_consequence_order() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = app_with_draws(
        directory.path().join("turn.json"),
        vec![TokenFace::Zero, TokenFace::Tablet],
    );
    // Use the bonus on Fight, then two ordinary resources. The third ordinary
    // action investigates with On the Hunt so Tablet's -2 still succeeds.
    for id in [
        "fight",
        "commit:none",
        "reveal",
        "resolve",
        "resource",
        "resource",
    ] {
        choose(&mut app, id);
        finish_all(&mut app);
    }
    investigate_to_resolve(&mut app, "commit:0");
    choose(&mut app, "resolve");
    assert_eq!(
        app.world().resource::<Table>().harm().unwrap().source,
        HarmSource::Tablet
    );
    assert_eq!(counters(&app), (6, 0));
    act(&mut app, Action::FinishAnimation);
    assert!(app.world().resource::<Table>().harm().is_none());
    assert!(
        app.world()
            .resource::<Table>()
            .presentation
            .clue()
            .is_some()
    );
    assert_eq!(counters(&app), (5, 0));
    let before_injury = app.world().resource::<Table>().view().damage;
    advance(&mut app, 0.81);
    assert_eq!(
        app.world().resource::<Table>().harm().unwrap().source,
        HarmSource::InternalInjury
    );
    assert_eq!(counters(&app), (5, 1));
    assert_eq!(
        app.world().resource::<Table>().view().damage,
        before_injury + 1
    );
}

#[test]
fn reduced_motion_places_clues_immediately_and_reopen_replay_do_not_reanimate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut app = app_with_draws(path.clone(), vec![TokenFace::Zero]);
    investigate_to_resolve(&mut app, "commit:none");
    act(&mut app, Action::Motion);
    choose(&mut app, "resolve");
    assert_eq!(counters(&app), (5, 1));
    assert!(!app.world().resource::<Table>().presenting());
    assert_eq!(
        super::super::clue_motion::traveling_count(app.world_mut()),
        0
    );

    act(&mut app, Action::Motion);
    for action in [
        Action::Reopen,
        Action::Replay,
        Action::Next,
        Action::Back,
        Action::Live,
    ] {
        act(&mut app, action);
        assert!(
            app.world()
                .resource::<Table>()
                .presentation
                .clue()
                .is_none()
        );
        assert_eq!(
            super::super::clue_motion::traveling_count(app.world_mut()),
            0
        );
    }
    assert_eq!(counters(&app), (5, 1));
    let mut reopened = presentation_app(path.clone());
    reopened.world_mut().resource_mut::<Table>().session = continuity::load_session(&path).unwrap();
    advance(&mut reopened, 0.0);
    assert_eq!(counters(&reopened), (5, 1));
    assert_eq!(
        super::super::clue_motion::traveling_count(reopened.world_mut()),
        0
    );
}
