use super::consequences::{clues_at, motion_app};
use super::*;
use crate::party_tabletop::consequences as motion;

pub(super) fn before_discovery(number: usize) -> Session {
    let mut session = Session::with_draws(vec![crate::TokenFace::Zero; 6]);
    let mut ordinal = 0;
    for actor in ["tony", "patrice"] {
        session
            .choose(session.observe().revision, &format!("select:{actor}"))
            .unwrap();
        for _ in 0..3 {
            for id in ["investigate", "commit:none", "reveal"] {
                session.choose(session.observe().revision, id).unwrap();
            }
            if ordinal == number {
                return session;
            }
            session
                .choose(session.observe().revision, "resolve:0")
                .unwrap();
            ordinal += 1;
        }
        session
            .choose(session.observe().revision, "end-turn")
            .unwrap();
    }
    unreachable!("six real clues in this attempt")
}

fn settle(app: &mut App) {
    for _ in 0..10 {
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(std::time::Duration::from_millis(100));
        app.update();
    }
}

#[test]
fn committed_first_subsequent_and_last_clues_move_once_through_the_installed_scene() {
    let directory = tempfile::tempdir().unwrap();
    for ordinal in [0, 1, 5] {
        let path = directory.path().join(format!("discovery-{ordinal}.json"));
        let (mut app, _) =
            selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
        let session = before_discovery(ordinal);
        let investigator = session.observe().test.unwrap().investigator;
        let previous = session.observe().investigator(investigator).clues;
        app.insert_resource(Table::new(session, path.clone()));
        app.update();
        choose(&mut app, "resolve:0");
        let committed = continuity::load_session(&path).unwrap();
        let table = app.world().resource::<Table>();
        assert_eq!(
            committed.observe().investigator(investigator).clues,
            previous + 1
        );
        assert_eq!(table.view().study_clues, 5 - ordinal as u8);
        assert_eq!(
            clues_at(&mut app, layout::investigator_clue(investigator.index())),
            previous
        );
        {
            let world = app.world_mut();
            let transforms: Vec<_> = world
                .query_filtered::<&Transform, With<motion::MovingClue>>()
                .iter(world)
                .cloned()
                .collect();
            assert_eq!(transforms.len(), 1);
            assert!(transforms[0].translation.y > layout::SCENARIO_MAP_CENTER.y);
        }
        // Camera focus preserves the moving token and saved consequence. A real
        // scene replacement is characterized separately in consequences tests.
        act(
            &mut app,
            Action::Focus(Area::Investigator(investigator.index())),
        );
        choose(&mut app, "resource");
        assert_eq!(app.world().resource::<Table>().session, committed);
        settle(&mut app);
        assert_eq!(
            clues_at(&mut app, layout::investigator_clue(investigator.index())),
            previous + 1
        );
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<motion::MovingClue>>()
                .iter(world)
                .count(),
            0
        );
        act(&mut app, Action::Reopen);
        assert_eq!(
            clues_at(&mut app, layout::investigator_clue(investigator.index())),
            previous + 1
        );
        assert_eq!(app.world().resource::<Table>().session, committed);
    }
}

#[test]
fn reduced_motion_failure_and_interrupted_reopen_never_duplicate_discovery() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("saved.json");
    let (mut app, _) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    let session = before_discovery(0);
    app.insert_resource(Table::new(session.clone(), directory.path().into()));
    app.update();
    choose(&mut app, "resolve:0");
    assert_eq!(app.world().resource::<Table>().session, session);
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 0);
    assert_eq!(
        app.world_mut()
            .query::<&motion::MovingClue>()
            .iter(app.world())
            .count(),
        0
    );
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    choose(&mut app, "resolve:0");
    assert_eq!(
        app.world_mut()
            .query::<&motion::MovingClue>()
            .iter(app.world())
            .count(),
        1
    );
    // A fresh process sees the committed state, not an unsaved animation stage.
    let mut reopened = motion_app(continuity::load_session(&path).unwrap(), path.clone());
    assert_eq!(clues_at(&mut reopened, layout::investigator_clue(0)), 1);
    assert_eq!(
        reopened
            .world_mut()
            .query::<&motion::MovingClue>()
            .iter(reopened.world())
            .count(),
        0
    );
    act(&mut app, Action::Motion);
    assert_eq!(
        app.world_mut()
            .query::<&motion::MovingClue>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 1);

    let mut table = Table::new(before_discovery(0), path);
    table.reduced_motion = true;
    app.insert_resource(table);
    app.update();
    choose(&mut app, "resolve:0");
    assert_eq!(
        app.world_mut()
            .query::<&motion::MovingClue>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(clues_at(&mut app, layout::investigator_clue(0)), 1);
}

#[test]
fn source_identity_and_test_success_are_required_for_discovery_motion() {
    let mut session = before_discovery(0);
    let before = session.observe();
    session.choose(before.revision, "resolve:0").unwrap();
    let after = session.observe();
    assert!(motion::Transfer::from_committed_discovery(&before, &after).is_some());
    let mut changed = before.clone();
    changed.test = None;
    assert!(motion::Transfer::from_committed_discovery(&changed, &after).is_none());
    let mut changed = before.clone();
    changed.investigators[0].location = "Hallway".into();
    assert!(motion::Transfer::from_committed_discovery(&changed, &after).is_none());
    let mut changed = after.clone();
    changed.test.as_mut().unwrap().result = Some(false);
    assert!(motion::Transfer::from_committed_discovery(&before, &changed).is_none());
    assert!(motion::Transfer::from_committed_discovery(&after, &after).is_none());
}
