use super::*;
use crate::party_tabletop::consequences::MovingClue;
use crate::tabletop_counters::{Counter, CounterKind, CounterLabel};
use std::time::Duration;

pub(super) fn motion_app(session: Session, path: PathBuf) -> App {
    let (mut app, _) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    app.insert_resource(Table::new(session, path));
    tick(&mut app, 0);
    app
}

fn tick(app: &mut App, milliseconds: u64) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(Duration::from_millis(milliseconds));
    app.update();
}

pub(super) fn clues_at(app: &mut App, at: Vec3) -> u8 {
    let world = app.world_mut();
    world
        .query_filtered::<(&Counter, &Transform), Without<MovingClue>>()
        .iter(world)
        .filter(|(counter, transform)| {
            counter.kind == CounterKind::Clue && transform.translation.abs_diff_eq(at, 0.001)
        })
        .map(|(counter, _)| counter.amount)
        .sum()
}

fn moving(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<MovingClue>>()
        .iter(world)
        .collect()
}

fn returning(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&ReturningToken>().iter(world).count()
}

fn labels(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<CounterLabel>>()
        .iter(world)
        .collect()
}

#[test]
fn overlap_blocks_until_clue_arrival_then_admits_without_an_extra_save() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("overlap.json");
    let mut app = motion_app(clue_motion::before_discovery(0), path.clone());
    choose(&mut app, "resolve:0");
    let saved = std::fs::read(&path).unwrap();
    let committed = app.world().resource::<Table>().session.clone();
    let moving_labels = labels(&mut app);
    assert_eq!(moving(&mut app).len(), 1);
    assert_eq!(returning(&mut app), 1);
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 0);
    assert_eq!(
        clues_at(&mut app, layout::location_clue(layout::SCENARIO_MAP_CENTER)),
        5
    );

    tick(&mut app, 650);
    assert_eq!(returning(&mut app), 0);
    assert_eq!(moving(&mut app).len(), 1);
    // Action draining precedes animation, even on the update that reaches arrival.
    let revision = committed.observe().revision;
    for milliseconds in [140, 20] {
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::Choose(revision, "resource".into()));
        tick(&mut app, milliseconds);
        assert_eq!(app.world().resource::<Table>().session, committed);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
    }
    assert!(moving(&mut app).is_empty());
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 1);
    assert!(
        moving_labels
            .iter()
            .all(|&entity| app.world().get_entity(entity).is_err())
    );
    tick(&mut app, 0);
    choose(&mut app, "resource");
    assert_eq!(
        app.world().resource::<Table>().view().investigators[0].resources,
        1
    );
}

#[test]
fn reduced_motion_switch_cleans_overlap_but_does_not_admit_queued_gameplay_early() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("reduced.json");
    let mut app = motion_app(clue_motion::before_discovery(0), path.clone());
    choose(&mut app, "resolve:0");
    let saved = std::fs::read(&path).unwrap();
    let old_labels = labels(&mut app);
    let revision = app.world().resource::<Table>().view().revision;
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::Focus(Area::Investigator(0)),
        Action::Dismiss,
        Action::Close,
        Action::Motion,
        Action::Choose(revision, "resource".into()),
        Action::History,
    ]);
    tick(&mut app, 0);
    assert!(app.world().resource::<Table>().reduced_motion);
    assert!(!app.world().resource::<Table>().history);
    assert!(moving(&mut app).is_empty());
    assert_eq!(returning(&mut app), 0);
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 1);
    assert!(
        old_labels
            .iter()
            .all(|&entity| app.world().get_entity(entity).is_err())
    );
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    choose(&mut app, "resource");
    assert_eq!(
        app.world().resource::<Table>().view().investigators[0].resources,
        1
    );
}

#[test]
fn queued_choices_see_clue_commit_immediately_but_token_return_only_after_scene_sync() {
    let directory = tempfile::tempdir().unwrap();
    for success in [false, true] {
        let mut session = Session::with_draws(vec![if success {
            crate::TokenFace::Zero
        } else {
            crate::TokenFace::AutoFail
        }]);
        for id in ["select:tony", "investigate", "commit:none", "reveal"] {
            session.choose(session.observe().revision, id).unwrap();
        }
        let revision = session.observe().revision;
        let path = directory.path().join(format!("queued-{success}.json"));
        let mut app = motion_app(session, path.clone());
        app.world_mut().resource_mut::<Actions>().0.extend([
            Action::Choose(revision, "resolve:0".into()),
            Action::Choose(revision + 1, "resource".into()),
        ]);
        tick(&mut app, 0);
        let saved = continuity::load_session(&path).unwrap().observe();
        assert_eq!(saved.investigators[0].resources, u8::from(!success));
        assert_eq!(saved.investigators[0].clues, u8::from(success));
        assert_eq!(saved, app.world().resource::<Table>().view());
        assert_eq!(returning(&mut app), 1);
        assert_eq!(moving(&mut app).len(), usize::from(success));
    }
}

#[test]
fn replay_return_follows_rendered_forward_transition_and_back_waits_for_return() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.json");
    let mut session = clue_motion::before_discovery(0);
    session
        .choose(session.observe().revision, "resolve:0")
        .unwrap();
    continuity::save_session(&path, &session).unwrap();
    let saved = std::fs::read(&path).unwrap();
    let recording = session.recording();
    let mut app = motion_app(session, path.clone());
    act(&mut app, Action::Replay);
    for _ in 0..recording.moments.len() - 2 {
        act(&mut app, Action::Next);
    }
    let index = app
        .world()
        .resource::<Table>()
        .replay
        .as_ref()
        .unwrap()
        .index();
    assert_eq!(
        app.world().resource::<Table>().view().held_tokens().len(),
        1
    );
    // Neither intermediate observation was rendered, so this pair starts no return.
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::Next, Action::Back]);
    tick(&mut app, 0);
    assert_eq!(returning(&mut app), 0);
    for _ in 0..2 {
        act(&mut app, Action::Next);
        assert_eq!(returning(&mut app), 1);
        assert!(moving(&mut app).is_empty());
        assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 1);
        act(&mut app, Action::Back);
        assert_eq!(
            app.world()
                .resource::<Table>()
                .replay
                .as_ref()
                .unwrap()
                .index(),
            index + 1
        );
        tick(&mut app, 650);
        tick(&mut app, 0);
        act(&mut app, Action::Back);
        assert_eq!(
            app.world()
                .resource::<Table>()
                .replay
                .as_ref()
                .unwrap()
                .index(),
            index
        );
        assert_eq!(returning(&mut app), 0);
        assert_eq!(
            app.world().resource::<Table>().view().held_tokens().len(),
            1
        );
    }
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    assert_eq!(
        app.world().resource::<Table>().session.recording(),
        recording
    );
}

#[test]
fn full_scene_replacement_removes_returning_tokens_but_preserves_the_moving_clue() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = motion_app(
        clue_motion::before_discovery(0),
        directory.path().join("scene.json"),
    );
    choose(&mut app, "resolve:0");
    let clue = moving(&mut app)[0];
    let scene_entity = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<CardSource>>()
            .iter(world)
            .next()
            .unwrap()
    };
    // A diagnostic observation change forces a real replacement; normal gameplay
    // is blocked during motion. Focus alone leaves the rendered observation equal.
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        let revision = table.view().revision;
        table.session.choose(revision, "resource").unwrap();
        table.invalidate();
    }
    tick(&mut app, 100);
    assert!(app.world().get_entity(scene_entity).is_err());
    assert_eq!(returning(&mut app), 0);
    assert_eq!(moving(&mut app), vec![clue]);
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 0);
    tick(&mut app, 710);
    assert!(moving(&mut app).is_empty());
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 1);
}

#[test]
fn pending_clue_keeps_production_redraw_active_after_token_return_then_releases_it() {
    use bevy::winit::{UpdateMode, WinitSettings};
    let directory = tempfile::tempdir().unwrap();
    let mut app = motion_app(
        clue_motion::before_discovery(0),
        directory.path().join("redraw.json"),
    );
    choose(&mut app, "resolve:0");
    tick(&mut app, 650);
    assert_eq!(returning(&mut app), 0);
    assert_eq!(moving(&mut app).len(), 1);
    app.add_message::<bevy::window::WindowEvent>()
        .add_plugins(TabletopPacingPlugin);
    // Keep this motion frozen while real cadence time expires. Only its ongoing
    // redraw demand, not camera/appearance movement, can keep this fixture active.
    app.add_systems(PreUpdate, |mut time: ResMut<PresentationTime>| {
        time.advance_by(Duration::ZERO)
    });
    app.insert_resource(Orbit::new(CameraMode::Jacqueline));
    app.update();
    std::thread::sleep(Duration::from_millis(2100));
    app.update();
    assert_eq!(
        app.world().resource::<WinitSettings>().focused_mode,
        UpdateMode::Continuous
    );
    act(&mut app, Action::Motion);
    assert!(moving(&mut app).is_empty());
    std::thread::sleep(Duration::from_millis(200));
    app.update();
    assert_eq!(
        app.world().resource::<WinitSettings>().focused_mode,
        UpdateMode::reactive_low_power(Duration::from_millis(250))
    );
    app.world_mut().spawn(Screenshot::primary_window());
    app.update();
    assert_eq!(
        app.world().resource::<WinitSettings>().focused_mode,
        UpdateMode::Continuous
    );
}
