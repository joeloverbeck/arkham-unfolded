use super::*;

fn host(path: PathBuf) -> App {
    let mut app = crate::tabletop_layout::heading_checks::app();
    app.init_resource::<Time>()
        .init_resource::<PresentationTime>()
        .init_resource::<Redraw>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(Table::new(Session::with_seed(19), path))
        .insert_resource(RenderHealth::ready_for_test())
        .insert_resource(Orbit::new(CameraMode::Jacqueline));
    app.world_mut().spawn((
        Window {
            resolution: (1600, 1000).into(),
            focused: true,
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    install(&mut app);
    app.update();
    app
}

#[test]
fn native_choice_persists_before_showing_real_opening_hands() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gathering.json");
    let mut app = host(path.clone());
    let entity = {
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
                            .is_some_and(|text| text.0 == "Choose Tony Morgan as lead investigator")
                    })
                    .then_some(entity)
            })
            .expect("production lead button is rendered")
    };
    app.world_mut().trigger(Activate { entity });
    app.update();
    let saved = continuity::load_session(path).unwrap().observe_for_play();
    assert_eq!(saved, app.world().resource::<Table>().view());
    assert_eq!(saved.stage, crate::gathering::Stage::Mulligan);
    assert!(saved.investigators.iter().all(|p| p.hand.len() == 5));
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
fn replay_cannot_choose_or_overwrite_the_live_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("gathering.json");
    let mut app = host(path.clone());
    continuity::save_session(&path, &app.world().resource::<Table>().session).unwrap();
    let before = std::fs::read(&path).unwrap();
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::Replay,
        Action::Choose(0, 0, "lead:tony".into()),
        Action::Save,
    ]);
    app.update();
    let table = app.world().resource::<Table>();
    assert!(table.replay.is_some());
    assert_eq!(table.session.observe_for_play().revision, 0);
    assert_eq!(std::fs::read(path).unwrap(), before);
}
