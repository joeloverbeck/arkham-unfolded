use super::*;

fn movement_session() -> Session {
    let mut session = review_saves::expanded_map_transition();
    // Ordinary play from the retained seed reaches the expanded map at end turn.
    for _ in 0..150 {
        let view = session.observe();
        if view
            .choices
            .iter()
            .any(|choice| choice.id == "action:ordinary:move:01113")
        {
            return session;
        }
        let id = [
            "turn:patrice",
            "turn:tony",
            "turn:jacqueline",
            "turn:end",
            "phase-window:pass",
            "enemy:hunters:done",
            "enemy:attacks",
            "enemy:end",
            "round:upkeep",
            "upkeep:resolve",
            "round:mythos",
            "mythos:doom",
            "mythos:encounters",
            "mythos:end",
            "window:pass",
            "commit:confirm",
            "test:reveal",
            "lucky:decline",
            "success:pass",
            "revelation:resolve",
            "harm:horror:investigator",
            "harm:damage:investigator",
        ]
        .into_iter()
        .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        .or_else(|| {
            view.choices
                .iter()
                .find(|choice| choice.id != "undo")
                .map(|choice| choice.id.as_str())
        })
        .unwrap();
        session.choose(view.revision, id).unwrap();
    }
    panic!("Ordinary play did not reach a move within its bound");
}

fn standee(app: &mut App, who: Investigator) -> Transform {
    let world = app.world_mut();
    world
        .query::<(&Name, &ContextTarget, &Transform)>()
        .iter(world)
        .find(|(name, target, _)| {
            name.as_str() == "Investigator standee"
                && target.source == controls::Source::Investigator(who)
        })
        .map(|(_, _, at)| *at)
        .unwrap()
}

fn room(app: &App, code: &str) -> (bool, u8) {
    let view = app.world().resource::<Table>().view();
    let room = view
        .play
        .as_ref()
        .unwrap()
        .scenario
        .locations
        .iter()
        .find(|room| room.code == code)
        .unwrap();
    (room.revealed, room.clues)
}

fn choose(app: &mut App, id: &str) {
    let table = app.world().resource::<Table>();
    let action = Action::Choose(table.epoch, table.view().revision, id.into());
    app.world_mut().resource_mut::<Actions>().0.push(action);
    app.update();
}

fn settle(app: &mut App) {
    for _ in 0..160 {
        if !app.world().resource::<Table>().is_presenting() {
            return;
        }
        draws::tick(app, 0.05);
    }
    panic!("Map presentation did not naturally complete");
}

fn assert_card_lands_with_enemy_arrival(app: &mut App, id: u16) {
    let committed = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    let slot = committed
        .scenario
        .enemies
        .iter()
        .position(|enemy| enemy.card.id == id)
        .unwrap();
    let target = &committed.scenario.enemies[slot];
    for _ in 0..30 {
        let displayed = app
            .world()
            .resource::<Table>()
            .last_rendered
            .as_ref()
            .unwrap()
            .play
            .as_ref()
            .unwrap();
        if displayed
            .scenario
            .enemies
            .iter()
            .find(|enemy| enemy.card.id == id)
            == Some(target)
        {
            assert!(
                draws::physical(app, id)
                    .translation
                    .abs_diff_eq(play::enemy_position(&committed, slot), 0.001),
                "The card must land when the displayed enemy location/engagement changes"
            );
            let world = app.world_mut();
            assert!(
                !world
                    .query::<(&motion::PhysicalCard, &motion::CardMotion)>()
                    .iter(world)
                    .any(|(card, _)| card.0 == id),
                "Arrival must not start another card transfer"
            );
            return;
        }
        draws::tick(app, 0.05);
    }
    panic!("Enemy did not arrive within the travel bound");
}

#[test]
fn a_resident_enemy_waits_for_the_investigator_to_arrive_before_engaging() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringArrival);
    let before = session.observe().play.unwrap();
    let enemy = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.code == "01118")
        .unwrap();
    assert_eq!(enemy.location, "01113");
    assert_eq!(enemy.engaged, None);
    let id = enemy.card.id;
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("arrival.json");
    let mut app = host_with_table(Table::new(session, save.clone()), true);
    let start = draws::physical(&mut app, id);
    choose(&mut app, "action:ordinary:move:01113");
    let committed = app.world().resource::<Table>().session.clone();
    assert_eq!(
        app.world()
            .resource::<Table>()
            .view()
            .play
            .unwrap()
            .scenario
            .enemies,
        before.scenario.enemies
    );
    assert_eq!(draws::physical(&mut app, id), start);
    draws::tick(&mut app, 0.2);
    assert_eq!(
        app.world()
            .resource::<Table>()
            .view()
            .play
            .unwrap()
            .scenario
            .enemies,
        before.scenario.enemies
    );
    assert_card_lands_with_enemy_arrival(&mut app, id);
    settle(&mut app);
    assert_eq!(app.world().resource::<Table>().session, committed);
    assert_eq!(continuity::load_session(&save).unwrap(), committed);
}

#[test]
fn hunter_travel_keeps_other_enemies_in_place_then_transfers_its_card_and_engages() {
    let session = Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringHunters);
    let before = session.observe();
    let original = before.play.as_ref().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("hunter.json");
    let mut app = host_with_table(Table::new(session, save.clone()), true);
    let card_at = draws::physical(&mut app, 400);
    choose(&mut app, "enemy:hunter:400");
    let committed = app.world().resource::<Table>().session.clone();
    let travelling = app.world().resource::<Table>().view().play.unwrap();
    assert_eq!(travelling.scenario.enemies, original.scenario.enemies);
    assert_eq!(travelling.scenario.locations, original.scenario.locations);
    assert_eq!(draws::physical(&mut app, 400), card_at);
    let world = app.world_mut();
    let markers: Vec<_> = world
        .query::<&Name>()
        .iter(world)
        .filter(|name| name.as_str().starts_with("Moving "))
        .map(|name| name.as_str().to_owned())
        .collect();
    assert_eq!(markers, ["Moving Ghoul Priest"]);
    assert_card_lands_with_enemy_arrival(&mut app, 400);
    settle(&mut app);
    let after = app.world().resource::<Table>().view().play.unwrap();
    let priest = after
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == 400)
        .unwrap();
    assert_eq!(priest.location, "01115");
    assert_eq!(priest.engaged, Some(Investigator::Tony));
    assert_ne!(
        draws::physical(&mut app, 400).translation,
        card_at.translation
    );
    for enemy in original
        .scenario
        .enemies
        .iter()
        .filter(|enemy| enemy.card.id != 400)
    {
        assert_eq!(
            after
                .scenario
                .enemies
                .iter()
                .find(|other| other.card.id == enemy.card.id),
            Some(enemy)
        );
    }
    assert_eq!(app.world().resource::<Table>().session, committed);
    assert_eq!(continuity::load_session(&save).unwrap(), committed);
}

#[test]
fn reduced_motion_and_fresh_reopen_show_the_committed_room_without_replaying_placement() {
    let session = movement_session();
    let who = session.observe().play.unwrap().actor.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("reduced.json");
    let mut table = Table::new(session, save.clone());
    table.reduced_motion = true;
    let mut app = host_with_table(table, true);
    while app
        .world()
        .resource::<Table>()
        .presentation
        .story()
        .is_some()
    {
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::ContinueStory);
        app.update();
    }
    choose(&mut app, "action:ordinary:move:01113");
    assert_eq!(room(&app, "01113"), (true, 6));
    let settled = standee(&mut app, who);
    let saved = std::fs::read(&save).unwrap();
    let mut reopened = host_with_table(Table::open_existing(save.clone()).unwrap(), true);
    assert_eq!(room(&reopened, "01113"), (true, 6));
    assert_eq!(standee(&mut reopened, who), settled);
    draws::tick(&mut reopened, 5.);
    assert_eq!(standee(&mut reopened, who), settled);
    assert_eq!(std::fs::read(save).unwrap(), saved);
}

#[test]
fn a_failed_move_save_cannot_start_travel_or_reveal_a_room() {
    let session = movement_session();
    let dir = tempfile::tempdir().unwrap();
    // A directory at the save destination makes the real save operation fail.
    let mut app = host_with_table(Table::new(session.clone(), dir.path().to_owned()), true);
    while app
        .world()
        .resource::<Table>()
        .presentation
        .story()
        .is_some()
    {
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::ContinueStory);
        app.update();
    }
    choose(&mut app, "action:ordinary:move:01113");
    assert_eq!(app.world().resource::<Table>().session, session);
    assert!(
        app.world()
            .resource::<Table>()
            .notice
            .starts_with("Decision not committed:")
    );
    assert!(!app.world().resource::<Table>().is_presenting());
    assert_eq!(room(&app, "01113"), (false, 0));
}

#[test]
fn revisiting_a_revealed_room_carries_patrice_without_turning_or_replenishing_it() {
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringLitaParley);
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("return.json")), true);
    let start = standee(&mut app, Investigator::Patrice);
    choose(&mut app, "action:ordinary:move:01112");
    let mut saw_travel = false;
    for _ in 0..100 {
        draws::tick(&mut app, 0.05);
        saw_travel |=
            standee(&mut app, Investigator::Patrice).translation.y > start.translation.y + 0.1;
        assert_eq!(room(&app, "01112"), (true, 0));
        let world = app.world_mut();
        assert!(!world.query::<&Name>().iter(world).any(|name| matches!(
            name.as_str(),
            "Unrevealed location face" | "New clues glint"
        )));
        if !world.resource::<Table>().is_presenting() {
            break;
        }
    }
    assert!(saw_travel);
    assert!(!app.world().resource::<Table>().is_presenting());
}

#[test]
fn act_two_turns_the_zero_clue_parlor_without_a_mover_or_clue_token() {
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
    for id in [
        "round-end:advance-act",
        "act:contribute:0",
        "act:contribute:0",
        "act:contribute:0",
        "act:contribute:1",
        "act:contribute:1",
        "act:contribute:1",
        "act:contribute:2",
        "act:contribute:2",
    ] {
        session.choose(session.observe().revision, id).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("parlor.json")), true);
    choose(&mut app, "act:contribute:2");
    // Reading the act reverse remains the existing explicit acknowledgment.
    while app
        .world()
        .resource::<Table>()
        .presentation
        .story()
        .is_some()
    {
        app.world_mut()
            .resource_mut::<Actions>()
            .0
            .push(Action::ContinueStory);
        app.update();
    }
    let mut turned = false;
    for _ in 0..100 {
        draws::tick(&mut app, 0.05);
        let world = app.world_mut();
        let names: Vec<_> = world
            .query::<&Name>()
            .iter(world)
            .map(|name| name.as_str().to_owned())
            .collect();
        turned |= names.iter().any(|name| name == "Unrevealed location face");
        assert!(
            !names
                .iter()
                .any(|name| name == "New clues glint" || name.starts_with("Moving "))
        );
        assert_eq!(room(&app, "01115").1, 0);
        if !app.world().resource::<Table>().is_presenting() {
            break;
        }
    }
    assert!(turned);
    assert_eq!(room(&app, "01115"), (true, 0));
    assert!(!app.world().resource::<Table>().is_presenting());
}

#[test]
#[ignore = "Exports ordinary native-review saves only when explicitly requested"]
fn export_map_motion_review_saves() {
    let output = PathBuf::from(
        std::env::var_os("ARKHAM_MAP_REVIEW_DIR").expect("Set a fresh export directory"),
    );
    let before = movement_session();
    let (_directory, mut app) =
        review_saves::open_review_save_in_view(&before, Some("scenario")).unwrap();
    review_saves::assert_visible_button(&mut app, "Continue");
    let acknowledge = choice_button(&mut app, "Continue").unwrap();
    app.world_mut().trigger(Activate {
        entity: acknowledge,
    });
    app.update();
    click_source(&mut app, controls::Source::Location("01113".into()));
    for _ in 0..3 {
        app.update();
    }
    review_saves::assert_visible_button(&mut app, "Spend an action: Move to Attic");
    let mut after = before.clone();
    after
        .choose(after.observe().revision, "action:ordinary:move:01113")
        .unwrap();
    review_saves::export_review_saves(
        &output,
        &[
            ("before-move", &before, "scenario"),
            ("after-move", &after, "scenario"),
        ],
    );
}

#[test]
fn first_visit_carries_the_whole_standee_before_flipping_and_placing_clues() {
    let session = movement_session();
    let who = session.observe().play.unwrap().actor.unwrap();
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("movement.json");
    let mut app = host_with_table(Table::new(session, save.clone()), true);
    while app
        .world()
        .resource::<Table>()
        .presentation
        .story()
        .is_some()
    {
        let continue_button = choice_button(&mut app, "Continue").unwrap();
        app.world_mut().trigger(Activate {
            entity: continue_button,
        });
        app.update();
    }
    let start = standee(&mut app, who);
    let others: Vec<_> = Investigator::ALL
        .into_iter()
        .filter(|other| *other != who)
        .map(|other| (other, standee(&mut app, other)))
        .collect();
    finish_presentation(&mut app);
    let table = app.world().resource::<Table>();
    let action = Action::Choose(
        table.epoch,
        table.view().revision,
        "action:ordinary:move:01113".into(),
    );
    app.world_mut().resource_mut::<Actions>().0.push(action);
    app.update();
    let committed = app.world().resource::<Table>().session.clone();
    assert_eq!(continuity::load_session(&save).unwrap(), committed);
    assert_eq!(
        room(&app, "01113"),
        (false, 0),
        "Arrival must precede revelation"
    );
    draws::tick(&mut app, 0.12);
    let lifted = standee(&mut app, who);
    assert!(lifted.translation.y > start.translation.y + 0.1);
    assert_eq!(lifted.scale, start.scale, "Lift the whole physical piece");
    for (other, at) in &others {
        assert_eq!(standee(&mut app, *other), *at);
    }
    // A scene rebuild and an untimed menu preserve the exact pose and save.
    app.world_mut().resource_mut::<Table>().menu_open = true;
    app.world_mut().resource_mut::<Table>().dirty = true;
    draws::tick(&mut app, 5.);
    assert_eq!(standee(&mut app, who), lifted);
    app.world_mut().resource_mut::<Table>().menu_open = false;
    app.world_mut().resource_mut::<Table>().dirty = true;
    let mut saw_flip_without_clues = false;
    let mut saw_clues = false;
    let mut saw_named_entry_harm = false;
    for _ in 0..120 {
        draws::tick(&mut app, 0.05);
        let text = rendered_text(&mut app);
        assert!(
            !text.contains("from 01113"),
            "Entry harm must name its location"
        );
        saw_named_entry_harm |= text.contains("from Attic");
        let (revealed, clues) = room(&app, "01113");
        saw_flip_without_clues |= revealed && clues == 0;
        saw_clues |= revealed && clues == 6;
        if !app.world().resource::<Table>().is_presenting() {
            break;
        }
    }
    assert!(saw_flip_without_clues && saw_clues);
    assert!(saw_named_entry_harm);
    assert!(
        !app.world().resource::<Table>().is_presenting(),
        "Remaining view: {:?}",
        app.world().resource::<Table>().view()
    );
    assert_ne!(standee(&mut app, who).translation, start.translation);
    for (other, at) in others {
        assert_eq!(standee(&mut app, other), at);
    }
    assert_eq!(app.world().resource::<Table>().session, committed);
    assert_eq!(continuity::load_session(&save).unwrap(), committed);
}
