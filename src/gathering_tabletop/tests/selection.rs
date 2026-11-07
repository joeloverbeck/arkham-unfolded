use super::*;

#[test]
fn outgoing_return_command_preserves_recovered_overrides_and_refreshable_preferences() {
    use std::ffi::OsString;
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("gathering.toml");
    let recovered = directory.path().join("recovered assets");
    let changed_preference = directory.path().join("new preference");
    let mut config = AssetConfig::load_at(settings.clone(), recovered.clone());
    config.save_root(&recovered).unwrap();
    let before = std::fs::read(&settings).unwrap();
    let original =
        ["app", "--experiment", "gathering", "--art-root", "/missing"].map(str::to_owned);
    let command = return_selection_command(&original, &recovered).unwrap();
    assert_eq!(command.get_program(), std::env::current_exe().unwrap());
    assert_eq!(
        command.get_args().map(OsString::from).collect::<Vec<_>>(),
        vec![
            "--entry".into(),
            "selection".into(),
            "--art-root".into(),
            recovered.clone().into_os_string()
        ]
    );
    assert_eq!(std::fs::read(&settings).unwrap(), before);

    let ordinary = ["app", "--experiment", "gathering"].map(str::to_owned);
    let command = return_selection_command(&ordinary, &recovered).unwrap();
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        ["--entry", "selection"]
    );
    config.save_root(&changed_preference).unwrap();
    let next = Options::parse(
        AssetConfig::load_at(settings, recovered),
        ["--experiment", "gathering"].map(str::to_owned).into_iter(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(next.art_root, changed_preference);
}

#[test]
fn pending_artwork_picker_blocks_navigation_and_game_actions_until_it_finishes() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let mut app = host(save.clone());
    let before = app.world().resource::<Table>().session.clone();
    let (settings, pending) = AssetSettings::new_gathering(AssetConfig::load_at(
        directory.path().join("gathering.toml"),
        directory.path().join("art"),
    ))
    .with_pending_selection();
    app.insert_resource(settings);
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::ReturnToSelection,
        Action::Choose(0, 0, "lead:tony".into()),
    ]);
    app.update();
    assert!(!app.world().resource::<Table>().leaving);
    assert_eq!(app.world().resource::<Table>().session, before);
    assert!(!save.exists());
    drop(pending);
    app.update();
    assert_eq!(app.world().resource::<Table>().session, before);
    assert!(!app.world().resource::<Table>().leaving);
}

#[test]
fn return_saves_the_pending_choice_and_discards_queued_game_actions() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let mut app = host(save.clone());
    let before = app.world().resource::<Table>().session.clone();
    let navigation = app.world().resource::<Navigation>().returning.clone();
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::ReturnToSelection,
        Action::Choose(0, 0, "lead:tony".into()),
    ]);
    app.update();
    assert!(app.world().resource::<Table>().leaving);
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(Table::open_existing(save).unwrap().session, before);
    assert!(!app.world().resource::<Messages<AppExit>>().is_empty());
    app.set_runner(|mut app| {
        app.update();
        AppExit::Success
    });
    assert_eq!(app.run(), AppExit::Success);
    assert!(navigation.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn failed_return_preserves_the_live_attempt_and_recovery_controls() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let mut app = host(save.clone());
    let before = app.world().resource::<Table>().session.clone();
    std::fs::write(&save, b"competing invalid record").unwrap();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::ReturnToSelection);
    app.update();
    let table = app.world().resource::<Table>();
    assert!(!table.leaving);
    assert_eq!(table.session, before);
    assert!(table.notice.starts_with("Could not return to selection:"));
    assert!(app.world().resource::<Messages<AppExit>>().is_empty());
    assert_eq!(std::fs::read(&save).unwrap(), b"competing invalid record");
}

#[test]
fn admission_collision_never_resumes_even_an_identical_record() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let mut app = host(save.clone());
    let table = Table::fresh(save.clone()).unwrap();
    continuity::save_session(&save, &table.session).unwrap();
    let bytes = std::fs::read(&save).unwrap();
    app.insert_resource(table);
    app.update();
    assert!(app.world().resource::<Table>().initial_save_pending);
    assert!(
        !app.world()
            .resource::<Navigation>()
            .created
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    assert!(!app.world().resource::<RenderHealth>().allows_play());
    assert_eq!(std::fs::read(&save).unwrap(), bytes);
}

#[test]
fn remembered_attempt_changes_only_after_successful_native_admission() {
    let directory = tempfile::tempdir().unwrap();
    let reference = directory.path().join("last.json");
    let save = directory.path().join("attempt.json");
    let mut app = host(save.clone());
    app.insert_resource(Table::open(save.clone()).unwrap());
    app.insert_resource(RenderHealth::default());
    app.insert_resource(RememberAttempt {
        reference: reference.clone(),
        pending: true,
    });
    app.update();
    assert!(!reference.exists());
    assert!(
        !app.world()
            .resource::<Navigation>()
            .created
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    app.insert_resource(RenderHealth::ready_for_test());
    app.update();
    assert!(
        app.world()
            .resource::<Navigation>()
            .created
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    assert_eq!(
        crate::attempts::recalled(&reference).unwrap(),
        Some(save.clone())
    );
    assert_eq!(
        Table::open_existing(save).unwrap().session,
        app.world().resource::<Table>().session
    );
}
