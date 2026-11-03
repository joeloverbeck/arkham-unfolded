use super::*;
use bevy::input::keyboard::Key;
use std::time::Duration;

fn activate_button(app: &mut App, label: &str) {
    let entity = button_with_label(app, label)
        .unwrap_or_else(|| panic!("The production interface must offer {label}"));
    app.world_mut().trigger(Activate { entity });
    app.update();
}

#[test]
fn menu_restart_preserves_the_old_attempt_and_reopens_the_new_setup() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("restart.json");
    let mut app = host(path.clone());
    activate_choice(&mut app, "lead:tony");
    finish_presentation(&mut app);
    let old = continuity::load_session(&path).unwrap();
    let old_bytes = std::fs::read(&path).unwrap();
    activate_button(&mut app, "Menu");
    activate_button(&mut app, "Motion: animated");
    activate_button(&mut app, "Restart The Gathering…");
    let cancel = button_with_label(&mut app, "Cancel").unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(cancel));
    assert_eq!(std::fs::read(&path).unwrap(), old_bytes);
    activate_button(&mut app, "Restart");

    let new = continuity::load_session(&path).unwrap();
    let view = new.observe_for_play();
    assert_eq!(view.revision, 0);
    assert_eq!(view.lead, None);
    assert!(view.choices.iter().any(|choice| choice.id == "lead:tony"));
    assert_eq!(app.world().resource::<Table>().view(), view);
    assert!(app.world().resource::<Table>().reduced_motion);
    assert!(app.world().resource::<FeedbackSettings>().reduced_motion);
    assert_eq!(view.source, old.observe_for_play().source);
    assert!(!new.same_run(&old));
    let archives: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("restart.json.archived-")
        })
        .collect();
    assert_eq!(archives.len(), 1);
    assert_eq!(std::fs::read(&archives[0]).unwrap(), old_bytes);
    let recovered = continuity::load_session(&archives[0]).unwrap();
    assert_eq!(recovered, old);
    assert_eq!(recovered.recording(), old.recording());
    assert!(rendered_text(&mut app).contains(archives[0].to_str().unwrap()));
    activate_button(&mut app, "Menu");
    activate_button(&mut app, "Reopen saved attempt");
    assert_eq!(app.world().resource::<Table>().session, new);
}

#[test]
fn restart_confirmation_captures_input_and_cancel_or_escape_preserves_the_attempt() {
    for cancel_with_escape in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cancel.json");
        let mut app = host(path.clone());
        activate_choice(&mut app, "lead:tony");
        finish_presentation(&mut app);
        let old = continuity::load_session(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        activate_button(&mut app, "Menu");
        activate_button(&mut app, "Restart The Gathering…");
        app.world_mut()
            .resource_mut::<PresentationTime>()
            .advance_by(Duration::from_millis(100));
        let pose = app.world().resource::<Orbit>().pose();
        keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), true);
        app.update();
        assert_eq!(app.world().resource::<Orbit>().pose(), pose);
        if cancel_with_escape {
            keyboard(&mut app, KeyCode::Escape, Key::Escape, true);
            keyboard(&mut app, KeyCode::Escape, Key::Escape, false);
        } else {
            // Cancel has initial focus: Enter must never confirm a restart.
            keyboard(&mut app, KeyCode::Enter, Key::Enter, true);
            keyboard(&mut app, KeyCode::Enter, Key::Enter, false);
        }
        assert!(button_with_label(&mut app, "Restart").is_none());
        assert!(app.world().resource::<Orbit>().pose().target.z < pose.target.z);
        keyboard(&mut app, KeyCode::KeyW, Key::Character("w".into()), false);
        assert_eq!(app.world().resource::<Table>().session, old);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(!std::fs::read_dir(directory.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".archived-")
        }));
    }
}

#[test]
fn a_failed_restart_preserves_live_state_and_reports_the_refusal_inside_confirmation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("stale.json");
    let mut app = host(path.clone());
    activate_choice(&mut app, "lead:tony");
    finish_presentation(&mut app);
    let old = app.world().resource::<Table>().session.clone();
    activate_button(&mut app, "Menu");
    activate_button(&mut app, "Restart The Gathering…");
    let mut newer = old.clone();
    let revision = newer.observe_for_play().revision;
    continuity::commit_choice(&path, &mut newer, revision, "confirm-mulligan").unwrap();
    let bytes = std::fs::read(&path).unwrap();
    activate_button(&mut app, "Restart");
    assert_eq!(app.world().resource::<Table>().session, old);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(button_with_label(&mut app, "Cancel").is_some());
    let world = app.world_mut();
    let dialog = world
        .query_filtered::<Entity, With<bevy::ui_widgets::ModalDialog>>()
        .single(world)
        .unwrap();
    let error_visible_in_dialog =
        world
            .query::<(Entity, &Text)>()
            .iter(world)
            .any(|(mut entity, text)| {
                if !text.0.starts_with("Restart failed:") {
                    return false;
                }
                while let Some(parent) = world.get::<ChildOf>(entity) {
                    entity = parent.parent();
                    if entity == dialog {
                        return true;
                    }
                }
                false
            });
    assert!(error_visible_in_dialog);
}

#[test]
fn restart_rejects_old_confirmations_and_choices_and_commits_only_once() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("queued.json");
    let mut app = host(path.clone());
    let old = app.world().resource::<Table>().session.clone();
    continuity::save_session(&path, &old).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let choice_epoch = app.world().resource::<Table>().epoch;
    activate_button(&mut app, "Menu");
    activate_button(&mut app, "Restart The Gathering…");
    let old_confirmation = app.world().resource::<Table>().epoch;
    activate_button(&mut app, "Cancel");
    activate_button(&mut app, "Menu");
    activate_button(&mut app, "Restart The Gathering…");
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::ConfirmRestart(old_confirmation),
        Action::Choose(choice_epoch, 0, "lead:tony".into()),
        Action::Replay,
    ]);
    app.update();
    // Observe refusal before a later valid confirmation can hide a stale action.
    assert_eq!(app.world().resource::<Table>().session, old);
    assert!(app.world().resource::<Table>().restart_confirmation);
    assert!(app.world().resource::<Table>().replay.is_none());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let epoch = app.world().resource::<Table>().epoch;
    app.world_mut().resource_mut::<Actions>().0.extend([
        Action::ConfirmRestart(epoch),
        Action::ConfirmRestart(epoch),
        Action::Choose(choice_epoch, 0, "lead:tony".into()),
    ]);
    app.update();
    let fresh = continuity::load_session(&path).unwrap();
    assert_eq!(fresh, app.world().resource::<Table>().session);
    assert!(fresh.is_fresh());
    assert!(!fresh.same_run(&old));
    let archives: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".archived-")
        })
        .collect();
    assert_eq!(archives.len(), 1);
    assert_eq!(std::fs::read(&archives[0]).unwrap(), bytes);
}

#[test]
fn restart_is_unavailable_in_replay_and_refuses_unready_graphics() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("admission.json");
    let mut app = host(path.clone());
    let old = app.world().resource::<Table>().session.clone();
    continuity::save_session(&path, &old).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    activate_button(&mut app, "Menu");
    activate_button(&mut app, "Open recorded replay");
    assert!(button_with_label(&mut app, "Restart The Gathering…").is_none());
    let epoch = app.world().resource::<Table>().epoch;
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::Restart(epoch), Action::ConfirmRestart(epoch)]);
    app.update();
    assert!(app.world().resource::<Table>().replay.is_some());
    assert!(!app.world().resource::<Table>().restart_confirmation);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Live);
    app.update();
    activate_button(&mut app, "Restart The Gathering…");
    for health in [
        RenderHealth::default(),
        RenderHealth::failed_for_test(),
        RenderHealth::just_ready_for_test(),
    ] {
        app.insert_resource(health);
        let epoch = app.world().resource::<Table>().epoch;
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::ConfirmRestart(epoch));
        app.update();
        assert_eq!(app.world().resource::<Table>().session, old);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}
