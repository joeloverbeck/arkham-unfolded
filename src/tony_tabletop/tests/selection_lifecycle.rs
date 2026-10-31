use super::*;

fn commitment_app(path: PathBuf) -> App {
    let mut app = presentation_app(path);
    for _ in 0..3 {
        choose(&mut app, "draw");
        act(&mut app, Action::FinishAnimation);
    }
    choose(&mut app, "fight");
    app
}

fn select(app: &mut App, id: u8) -> (u64, u64) {
    let revision = app.world().resource::<Table>().view().revision;
    act(app, Action::Select(revision, Source::Hand(id), Vec3::ZERO));
    (
        app.world().resource::<Table>().context.menu_epoch(),
        revision,
    )
}

fn toggle(app: &mut App, id: u8) {
    let (epoch, revision) = select(app, id);
    act(app, Action::ToggleCommit(epoch, revision, id));
}

fn committed(app: &App) -> Vec<u8> {
    app.world()
        .resource::<Table>()
        .view()
        .committed
        .iter()
        .map(|card| card.id)
        .collect()
}

#[test]
fn same_revision_reopen_and_failed_writes_preserve_tonys_pending_choice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("turn.json");
    let mut app = commitment_app(path.clone());
    toggle(&mut app, 0);
    let before = app.world().resource::<Table>().session.clone();
    let saved = std::fs::read(&path).unwrap();
    let revision = before.observe().revision;
    app.world_mut().resource_mut::<Table>().save_path = dir.path().into();
    for (action, notice) in [
        (Action::Save, "Save failed"),
        (Action::Reopen, "Reopen failed"),
        (Action::ConfirmCommit(revision), "Decision not committed"),
    ] {
        act(&mut app, action);
        assert!(app.world().resource::<Table>().notice.starts_with(notice));
        assert_eq!(app.world().resource::<Table>().session, before);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
    }
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    act(&mut app, Action::Reopen);
    assert_eq!(app.world().resource::<Table>().session, before);
    act(&mut app, Action::ConfirmCommit(revision));
    assert_eq!(committed(&app), [0]);
    assert_eq!(
        continuity::load_session(&path).unwrap(),
        app.world().resource::<Table>().session
    );
}

#[test]
fn confirmation_uses_the_changed_set_with_the_menu_closed_and_keeps_revision_refusals_visible() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("turn.json");
    let mut app = commitment_app(path.clone());
    toggle(&mut app, 0);
    let revision = app.world().resource::<Table>().view().revision;
    let confirmation = Action::ConfirmCommit(revision);
    let saved = std::fs::read(&path).unwrap();
    toggle(&mut app, 3);
    toggle(&mut app, 0);
    act(&mut app, Action::DismissMenu);
    let before = app.world().resource::<Table>().session.clone();
    act(&mut app, Action::ConfirmCommit(revision - 1));
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .starts_with("Decision not committed")
    );
    assert_eq!(app.world().resource::<Table>().session, before);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    act(&mut app, confirmation);
    assert_eq!(committed(&app), [3]);
}

#[test]
fn reselected_source_rejects_old_callbacks_and_processes_current_callbacks_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = commitment_app(dir.path().join("turn.json"));
    toggle(&mut app, 0);
    let (old, revision) = select(&mut app, 3);
    act(&mut app, Action::DismissMenu);
    let (current, _) = select(&mut app, 3);
    act(&mut app, Action::ToggleCommit(old, revision, 3));
    let context = &app.world().resource::<Table>().context;
    assert!(
        !context.is_pending(Source::Hand(3)),
        "the old callback must not toggle the card"
    );
    assert!(
        context.is_menu_source(Source::Hand(3)),
        "the current menu remains usable"
    );
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::ToggleCommit(current, revision, 3),
        Action::ToggleCommit(current, revision, 3),
        Action::ConfirmCommit(revision),
    ]);
    app.update();
    assert_eq!(committed(&app), [0, 3]);
}

#[test]
fn current_menu_callbacks_retain_continuitys_revision_and_id_refusals() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = presentation_app(dir.path().join("turn.json"));
    act(&mut app, Action::Select(0, Source::Card(2), Vec3::ZERO));
    let epoch = app.world().resource::<Table>().context.menu_epoch();
    let before = app.world().resource::<Table>().session.clone();
    for (revision, id) in [(1, "fight"), (0, "not-offered")] {
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
    act(&mut app, Action::ContextChoice(epoch, 0, "fight".into()));
    assert_eq!(app.world().resource::<Table>().view().stage, Stage::Commit);
}
