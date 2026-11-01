use super::*;

fn activate(app: &mut App, label: &str) {
    let entity = rendered_button(app, label);
    app.world_mut().trigger(Activate { entity });
    app.update();
}

fn key(app: &mut App, window: Entity, key_code: KeyCode, logical_key: bevy::input::keyboard::Key) {
    use bevy::input::{ButtonState, keyboard::KeyboardInput};
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: logical_key.clone(),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

fn closing_session() -> Session {
    let mut session = Session::with_draws(vec![crate::TokenFace::Zero; 6]);
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
}

#[test]
fn production_inspection_hides_unrevealed_act_and_location_faces() {
    let dir = tempfile::tempdir().unwrap();
    let (mut app, _) = selection_lifecycle::production_app(
        dir.path().join("party.json"),
        RenderHealth::ready_for_test(),
    );
    let act_one = cards::index("card/01108").unwrap();
    let revision = app.world().resource::<Table>().view().revision;
    act(
        &mut app,
        Action::Select(revision, Source::Card(act_one), Vec3::ZERO),
    );
    assert_eq!(app.world().resource::<Table>().inspect, Some(act_one));
    assert!(
        !visible_text(&mut app)
            .join("\n")
            .contains("Inspect reverse")
    );
    act(&mut app, Action::Close);

    for role in [
        "card/01108/reverse",
        "card/01109",
        "card/01112",
        "card/01113",
    ] {
        let index = cards::index(role).unwrap();
        act(&mut app, Action::Inspect(index));
        assert_eq!(app.world().resource::<Table>().inspect, None, "{role}");
        act(
            &mut app,
            Action::Select(revision, Source::Card(index), Vec3::ZERO),
        );
        assert_eq!(app.world().resource::<Table>().inspect, None, "{role}");
    }
}

#[test]
fn production_act_reveal_is_required_saved_readable_and_continues_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let (mut app, window) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    app.world_mut()
        .insert_resource(Table::new(closing_session(), path.clone()));
    app.update();
    let revision = app.world().resource::<Table>().view().revision;
    act(
        &mut app,
        Action::Select(
            revision,
            Source::Card(cards::index("card/01108").unwrap()),
            Vec3::ZERO,
        ),
    );
    activate(&mut app, "Spend six group clues and advance Act 1");
    let paid = app.world().resource::<Table>().session.clone();
    assert_eq!(paid.observe().stage, crate::party::Stage::ActReveal);
    assert_eq!(continuity::load_session(&path).unwrap(), paid);
    assert!(app.world().resource::<Table>().modal());
    let text = visible_text(&mut app).join("\n");
    assert!(text.contains("Act advances"));
    assert!(text.contains("The Door on the Floor"));
    assert!(text.contains("You notice that the edges of your newly purchased rug"));
    assert!(text.contains("Remove the Study from the game."));
    assert!(!text.contains("Close  /  Esc"));
    for action in [
        Action::Close,
        Action::Dismiss,
        Action::History,
        Action::New,
        Action::Inspect(0),
        Action::Choose(revision, "advance".into()),
    ] {
        act(&mut app, action);
        assert!(app.world().resource::<Table>().modal());
        assert_eq!(app.world().resource::<Table>().session, paid);
    }
    key(
        &mut app,
        window,
        KeyCode::Escape,
        bevy::input::keyboard::Key::Escape,
    );
    assert!(app.world().resource::<Table>().modal());
    let focused = app.world().resource::<InputFocus>().get().unwrap();
    assert_eq!(
        app.world().get::<AccessibleLabel>(focused).unwrap().0,
        "Continue"
    );
    key(
        &mut app,
        window,
        KeyCode::Tab,
        bevy::input::keyboard::Key::Tab,
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));
    act(&mut app, Action::Reopen);
    assert_eq!(app.world().resource::<Table>().session, paid);
    key(
        &mut app,
        window,
        KeyCode::Enter,
        bevy::input::keyboard::Key::Enter,
    );
    assert_eq!(
        app.world().resource::<Table>().view().endpoint,
        Some(crate::party::Endpoint::Escaped)
    );
    assert!(!app.world().resource::<Table>().modal());
    let finished = app.world().resource::<Table>().session.clone();
    act(
        &mut app,
        Action::Choose(paid.observe().revision, "continue-act".into()),
    );
    assert_eq!(app.world().resource::<Table>().session, finished);
    act(&mut app, Action::History);
    activate(&mut app, "Read Act 1 · The Door on the Floor");
    assert_eq!(
        app.world().resource::<Table>().inspect,
        cards::index("card/01108/reverse")
    );
    assert_eq!(app.world().resource::<Table>().session, finished);
}

#[test]
fn production_act_save_failures_never_disclose_or_skip_the_reading_point() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let blocked = dir.path().join("blocked.json");
    std::fs::create_dir(&blocked).unwrap();
    let (mut app, _) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    let closing = closing_session();
    app.world_mut()
        .insert_resource(Table::new(closing.clone(), blocked.clone()));
    app.update();
    choose(&mut app, "advance");
    assert_eq!(app.world().resource::<Table>().session, closing);
    assert!(!app.world().resource::<Table>().modal());
    assert!(
        !visible_text(&mut app)
            .join("\n")
            .contains("The Door on the Floor")
    );
    app.world_mut().resource_mut::<Table>().save_path = path.clone();
    choose(&mut app, "advance");
    let paid = app.world().resource::<Table>().session.clone();
    app.world_mut().resource_mut::<Table>().save_path = blocked;
    activate(&mut app, "Continue");
    assert_eq!(app.world().resource::<Table>().session, paid);
    assert!(app.world().resource::<Table>().modal());
    let text = visible_text(&mut app).join("\n");
    assert!(text.contains("Decision not committed:"));
    assert!(text.contains("The Door on the Floor"));
    app.world_mut().resource_mut::<Table>().save_path = path;
    activate(&mut app, "Continue");
    assert_eq!(
        app.world().resource::<Table>().view().endpoint,
        Some(crate::party::Endpoint::Escaped)
    );
}

#[test]
fn production_replay_discloses_act_only_at_its_recorded_moment_and_keeps_navigation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("party.json");
    let (mut app, _) =
        selection_lifecycle::production_app(path.clone(), RenderHealth::ready_for_test());
    let live = completed_attempt(Some(true));
    app.world_mut()
        .insert_resource(Table::new(live.clone(), path));
    app.update();
    act(&mut app, Action::Replay);
    act(
        &mut app,
        Action::Inspect(cards::index("card/01108/reverse").unwrap()),
    );
    assert_eq!(app.world().resource::<Table>().inspect, None);
    act(&mut app, Action::History);
    assert!(
        !visible_text(&mut app)
            .join("\n")
            .contains("The Door on the Floor")
    );
    act(&mut app, Action::Close);
    while app.world().resource::<Table>().view().stage != crate::party::Stage::ActReveal {
        let table = app.world().resource::<Table>();
        let replay = table.replay.as_ref().unwrap();
        assert!(replay.index() + 1 < replay.len());
        act(&mut app, Action::Next);
    }
    assert!(app.world().resource::<Table>().modal());
    let text = visible_text(&mut app).join("\n");
    assert!(text.contains("Act advances  /  Recorded moment"), "{text}");
    assert!(!visible_text(&mut app).iter().any(|text| text == "Continue"));
    activate(&mut app, "Previous moment");
    assert!(!app.world().resource::<Table>().modal());
    activate(&mut app, "Next moment");
    assert!(app.world().resource::<Table>().modal());
    activate(&mut app, "Next moment");
    assert_eq!(
        app.world().resource::<Table>().view().endpoint,
        Some(crate::party::Endpoint::Escaped)
    );
    activate(&mut app, "Previous moment");
    assert!(app.world().resource::<Table>().modal());
    activate(&mut app, "Return to live attempt");
    assert!(app.world().resource::<Table>().replay.is_none());
    assert_eq!(app.world().resource::<Table>().session, live);
}
