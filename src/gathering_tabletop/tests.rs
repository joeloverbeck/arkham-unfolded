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

fn rendered_text(app: &mut App) -> String {
    let world = app.world_mut();
    let mut query = world.query::<&Text>();
    query
        .iter(world)
        .map(|text| text.0.as_str())
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
    let view = session.observe_for_play();
    let play = view.play.as_ref().unwrap();
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = session;
        table.dirty = true;
    }
    app.update();
    let text = rendered_text(&mut app);
    assert!(text.contains(&format!(
        "Agenda {} · {} doom",
        play.agenda, play.agenda_doom
    )));
    assert!(text.contains(&format!("Act {}", play.act)));
    for enemy in &play.scenario.enemies {
        assert!(text.contains(&format!("{} · copy {}", enemy.card.title, enemy.card.id)));
    }
    for player in &play.players {
        for asset in &player.assets {
            assert!(text.contains(&format!("{} · copy {}", asset.card.title, asset.card.id)));
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
            .find(|(text, _)| text.0.starts_with(&format!("{name} ·")))
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
fn unrevealed_room_inspection_preserves_the_observed_face() {
    let view = Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd)
        .observe_for_play();
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

fn activate_choice(app: &mut App, id: &str) {
    finish_presentation(app);
    let label = app
        .world()
        .resource::<Table>()
        .view()
        .choices
        .into_iter()
        .find(|choice| choice.id == id)
        .unwrap()
        .label;
    let world = app.world_mut();
    let mut query = world.query_filtered::<(Entity, &Children), With<Button>>();
    let entity = query
        .iter(world)
        .find_map(|(entity, children)| {
            children
                .iter()
                .any(|child| world.get::<Text>(child).is_some_and(|text| text.0 == label))
                .then_some(entity)
        })
        .expect("The legal choice has a production button");
    world.trigger(Activate { entity });
    app.update();
}

fn finish_presentation(app: &mut App) {
    if app.world().resource::<Table>().is_presenting() {
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::PresentationFinish);
        app.update();
    }
}

#[test]
fn live_runtime_choice_keeps_manual_camera_and_replay_follows_with_manual_override() {
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
    assert!(rendered_text(&mut app).contains(&format!(
        "Printed base {} · Difficulty {}",
        test.base_skill, test.difficulty
    )));
    assert_eq!(
        continuity::load_session(&path).unwrap().observe_for_play(),
        view
    );
    let bytes = std::fs::read(&path).unwrap();
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .extend([Action::Replay, Action::Next]);
    app.update();
    assert!(app.world().resource::<Orbit>().is_following());
    assert_eq!(
        play::focus(&app.world().resource::<Table>().view()),
        Area::Test
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Focus(Area::Overview));
    app.update();
    assert!(!app.world().resource::<Orbit>().is_following());
    assert_eq!(
        app.world().resource::<Orbit>().pose().target,
        layout::focus(Area::Overview).0
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Back);
    app.update();
    assert!(app.world().resource::<Orbit>().is_following());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "Camera/replay navigation cannot rewrite the match"
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Live);
    app.update();
    assert!(
        !app.world().resource::<Orbit>().is_following(),
        "Return preserves the live manual setting"
    );
}

#[test]
fn actual_long_choice_panel_takes_focus_and_scrolls_by_keyboard_without_choosing() {
    let directory = tempfile::tempdir().unwrap();
    let mut app = host(directory.path().join("long-list.json"));
    {
        let mut table = app.world_mut().resource_mut::<Table>();
        table.session = Session::with_hostile_fixture(crate::gathering::HostileFixture::Search(
            crate::gathering::SearchFixture::TonyEvents,
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
    let before = app.world().resource::<Table>().view();
    assert!(before.choices.len() > 10);
    let focus = app
        .world()
        .resource::<InputFocus>()
        .get()
        .expect("First legal choice receives focus");
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
    let mut panels = world.query::<(&ScrollablePanel, &ScrollPosition, &ComputedNode)>();
    let (_, scroll, node) = panels
        .iter(world)
        .find(|(panel, _, _)| !panel.inspection)
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
    assert!(text.contains("Resolving cards · 1"));
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
    assert_eq!(
        continuity::load_session(path).unwrap().observe_for_play(),
        view
    );
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
    activate_choice(&mut app, "revelation:resolve");
    loop {
        finish_presentation(&mut app);
        let view = app.world().resource::<Table>().view();
        if view.choices.iter().any(|choice| choice.id == "test:reveal") {
            break;
        }
        let id = ["window:pass", "commit:confirm"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
            .unwrap();
        activate_choice(&mut app, id);
    }
    activate_choice(&mut app, "test:reveal");
    let final_view = continuity::load_session(&path).unwrap().observe_for_play();
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
    assert!(rendered_text(&mut app).contains("Next event"));
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
        .extend([Action::Close, Action::PresentationNext]);
    app.update();
    assert_eq!(
        app.world().resource::<Table>().session.observe_for_play(),
        final_view
    );
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
    finish_presentation(&mut app);
    assert!(rendered_text(&mut app).contains("Cards +4 · Other bonuses +0 · Chaos pending"));
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
