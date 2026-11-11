use super::*;
use crate::gathering_tabletop::map_motion::{HOP, REST};

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
            "enemy:skip-empty",
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

const STEP: f32 = 0.02;
/// The beat's first rendered frame may trail the commit by a few updates.
const LAG: f32 = 0.06;

fn location_size() -> Vec2 {
    Vec2::new(
        crate::tabletop_pieces::CARD_WIDTH,
        crate::tabletop_pieces::CARD_LENGTH,
    )
}

fn moving_marker(app: &mut App, title: &str) -> Option<Transform> {
    let name = format!("Moving {title}");
    let world = app.world_mut();
    world
        .query::<(&Name, &Transform)>()
        .iter(world)
        .find(|(marker, _)| marker.as_str() == name)
        .map(|(_, at)| *at)
}

fn marker_radius(app: &mut App, title: &str) -> f32 {
    use bevy::camera::primitives::MeshAabb;
    let name = format!("Moving {title}");
    let world = app.world_mut();
    let handle = world
        .query::<(&Name, &Mesh3d)>()
        .iter(world)
        .find(|(marker, _)| marker.as_str() == name)
        .map(|(_, mesh)| mesh.0.clone())
        .unwrap();
    let aabb = world
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .get_aabb()
        .unwrap();
    aabb.half_extents.x.max(aabb.half_extents.z)
}

/// The point halfway along the brass link the table displays between two rooms,
/// measured along the link itself rather than between its ends.
fn displayed_link_midpoint(app: &mut App, from: &str, to: &str) -> Vec2 {
    use bevy::mesh::VertexAttributeValues;
    let play = app.world().resource::<Table>().view().play.unwrap();
    let map = crate::tabletop_gathering_map::GatheringMap::new(
        play.scenario
            .locations
            .iter()
            .filter(|room| !play.scenario.location_in_victory_display(&room.code))
            .map(|room| (room.code.as_str(), room.revealed)),
        location_size(),
    )
    .unwrap();
    let route: Vec<Vec2> = map
        .connection_route(from, to)
        .unwrap()
        .iter()
        .map(|point| point.xz())
        .collect();
    let half = route
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum::<f32>()
        / 2.;
    let mut walked = 0.;
    let mut midpoint = None;
    for pair in route.windows(2) {
        let length = pair[0].distance(pair[1]);
        if walked + length >= half {
            midpoint = Some(pair[0].lerp(pair[1], (half - walked) / length));
            break;
        }
        walked += length;
    }
    let midpoint = midpoint.unwrap();
    // The rendered brass, not only the routing data, passes under that point.
    let world = app.world_mut();
    let parts: Vec<Entity> = world
        .query_filtered::<&Children, With<crate::tabletop_gathering_map::ConnectionVisual>>()
        .iter(world)
        .flat_map(|children| children.to_vec())
        .collect();
    let meshes = world.resource::<Assets<Mesh>>();
    let on_brass = parts.iter().any(|part| {
        let Some(mesh) = world.get::<Mesh3d>(*part) else {
            return false;
        };
        let Some(VertexAttributeValues::Float32x3(positions)) = meshes
            .get(&mesh.0)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return false;
        };
        positions.chunks(3).any(|triangle| {
            let [a, b, c] = [0, 1, 2].map(|i| Vec2::new(triangle[i][0], triangle[i][2]));
            let sides = [
                (b - a).perp_dot(midpoint - a),
                (c - b).perp_dot(midpoint - b),
                (a - c).perp_dot(midpoint - c),
            ];
            sides.iter().all(|side| *side >= -1e-5) || sides.iter().all(|side| *side <= 1e-5)
        })
    });
    assert!(
        on_brass,
        "The route midpoint must lie on the displayed brass"
    );
    midpoint
}

type Sample = (f32, Transform, Option<Transform>);

/// Advance from a committed move through its whole travel beat, sampling the
/// mover and an accompanying enemy marker against time since the beat began.
fn sample_travel(app: &mut App, who: Investigator, enemy: Option<&str>) -> Vec<Sample> {
    let mut samples = vec![];
    let mut time = 0.;
    while time < 2. * HOP + REST - LAG {
        draws::tick(app, STEP);
        time += STEP;
        let marker = enemy.and_then(|title| moving_marker(app, title));
        samples.push((time, standee(app, who), marker));
    }
    samples
}

fn sample_at(samples: &[Sample], time: f32) -> &Sample {
    samples
        .iter()
        .min_by(|a, b| (a.0 - time).abs().total_cmp(&(b.0 - time).abs()))
        .unwrap()
}

/// Mid-flight moments of the first and second hop.
fn hops() -> [f32; 2] {
    [HOP / 2., HOP + REST + HOP / 2.]
}

fn resting(samples: &[Sample]) -> Vec<&Sample> {
    let rest: Vec<_> = samples
        .iter()
        .filter(|(time, ..)| *time >= HOP + LAG && *time <= HOP + REST - LAG)
        .collect();
    assert!(rest.len() >= 3, "The rest must be perceptible");
    rest
}

/// Both hops carry the standee raised; between them it stands still, upright and
/// at its base height, on the midpoint of the displayed brass arrow.
fn assert_rests_on_the_arrow(samples: &[Sample], start: Transform, midpoint: Vec2) -> Transform {
    for hop in hops() {
        let lifted = sample_at(samples, hop).1;
        assert!(
            lifted.translation.y > start.translation.y + 0.1,
            "Each hop lifts the standee: {lifted:?} at {hop}s"
        );
        assert_eq!(lifted.scale, start.scale, "Lift the whole physical piece");
    }
    let rest = resting(samples);
    let pose = rest[0].1;
    for (time, at, _) in &rest {
        assert_eq!(
            *at, pose,
            "The standee stays still throughout the rest ({time}s)"
        );
    }
    assert!(
        pose.translation.xz().abs_diff_eq(midpoint, 0.001),
        "Rest at the arrow's midpoint {midpoint:?}, not {:?}",
        pose.translation
    );
    assert!((pose.translation.y - start.translation.y).abs() < 0.001);
    assert!(pose.rotation.abs_diff_eq(start.rotation, 0.001));
    assert_eq!(pose.scale, start.scale);
    pose
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
    // The longest travel beat is an investigator's two hops and rest; a few
    // extra frames cover the beat's first render.
    let bound = ((2. * HOP + REST) / 0.05).ceil() as usize + 6;
    for _ in 0..bound {
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

fn continue_stories(app: &mut App) {
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
}

#[test]
fn a_first_entry_hops_to_the_brass_arrow_rests_then_hops_into_the_room() {
    let session = movement_session();
    let play = session.observe().play.unwrap();
    let who = play.actor.unwrap();
    let from = play.players[who.index()].location.clone();
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("hops.json");
    let mut app = host_with_table(Table::new(session, save.clone()), true);
    continue_stories(&mut app);
    finish_presentation(&mut app);
    let start = standee(&mut app, who);
    let midpoint = displayed_link_midpoint(&mut app, &from, "01113");
    choose(&mut app, "action:ordinary:move:01113");
    let committed = app.world().resource::<Table>().session.clone();
    assert_eq!(continuity::load_session(&save).unwrap(), committed);
    let samples = sample_travel(&mut app, who, None);
    assert_rests_on_the_arrow(&samples, start, midpoint);
    assert_eq!(
        room(&app, "01113"),
        (false, 0),
        "Both hops and the rest are one travel beat, before the room turns"
    );
    settle(&mut app);
    assert_eq!(room(&app, "01113"), (true, 6));
    let arrived = standee(&mut app, who);
    assert!(arrived.translation.xz().distance(midpoint) > 0.5);
    assert!((arrived.translation.y - start.translation.y).abs() < 0.001);
    assert_eq!(app.world().resource::<Table>().session, committed);
}

/// Tony, engaged with the Ghoul Priest, moves from the Hallway into the Cellar.
/// Returns the decision before the one that completes the move, and that choice.
fn engaged_move() -> (Session, String) {
    let mut session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringPriestDefeat);
    let mut id = "action:ordinary:move:01114".to_owned();
    for _ in 0..20 {
        let mut next = session.clone();
        next.choose(next.observe().revision, &id).unwrap();
        if next.observe().play.unwrap().players[Investigator::Tony.index()].location == "01114" {
            return (session, id);
        }
        session = next;
        id = session
            .observe()
            .choices
            .iter()
            .find(|choice| choice.id != "undo")
            .unwrap()
            .id
            .clone();
    }
    panic!("Tony did not complete his move within its bound");
}

#[test]
fn an_engaged_enemy_hops_and_rests_in_step_beside_its_investigator() {
    let (session, id) = engaged_move();
    let before = session.observe().play.unwrap();
    assert_eq!(before.players[Investigator::Tony.index()].location, "01112");
    let priest = before
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.engaged == Some(Investigator::Tony))
        .unwrap()
        .clone();
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("engaged.json");
    let mut app = host_with_table(Table::new(session, save.clone()), true);
    continue_stories(&mut app);
    finish_presentation(&mut app);
    let start = standee(&mut app, Investigator::Tony);
    let midpoint = displayed_link_midpoint(&mut app, "01112", "01114");
    choose(&mut app, &id);
    let committed = app.world().resource::<Table>().session.clone();
    assert_eq!(continuity::load_session(&save).unwrap(), committed);
    // Earlier moments of the same commit may precede the travel beat; its
    // marker appears as the beat begins.
    let mut waited = 0.;
    while moving_marker(&mut app, &priest.card.title).is_none() {
        assert!(
            waited < 5.,
            "No travel beat began: {:?}",
            app.world().resource::<Table>().notice
        );
        draws::tick(&mut app, STEP);
        waited += STEP;
    }
    assert_eq!(standee(&mut app, Investigator::Tony), start);
    let radius = marker_radius(&mut app, &priest.card.title);
    let samples = sample_travel(&mut app, Investigator::Tony, Some(&priest.card.title));
    let standee_rest = assert_rests_on_the_arrow(&samples, start, midpoint);
    let rest = resting(&samples);
    let marker_rest = rest[0]
        .2
        .expect("The engaged enemy travels with its investigator");
    for (time, _, marker) in &rest {
        assert_eq!(
            *marker,
            Some(marker_rest),
            "The enemy rests with its investigator ({time}s)"
        );
    }
    for hop in hops() {
        let marker = sample_at(&samples, hop)
            .2
            .expect("The enemy hops with its investigator");
        assert!(
            marker.translation.y > marker_rest.translation.y + 0.1,
            "The enemy is lifted in step: {marker:?} at {hop}s"
        );
    }
    // Set down on the table beside the standee, clear of its base, at the arrow.
    assert!((marker_rest.translation.y - standee_rest.translation.y).abs() < 0.15);
    let apart = marker_rest
        .translation
        .xz()
        .distance(standee_rest.translation.xz());
    assert!(
        apart >= crate::tabletop_standees::radius_for(location_size()) + radius,
        "The resting enemy marker must not overlap the standee ({apart})"
    );
    assert!(marker_rest.translation.xz().distance(midpoint) < location_size().x);
    settle(&mut app);
    let after = app.world().resource::<Table>().view().play.unwrap();
    let moved = after
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == priest.card.id)
        .unwrap();
    assert_eq!(moved.location, "01114");
    assert_eq!(moved.engaged, Some(Investigator::Tony));
    assert_eq!(app.world().resource::<Table>().session, committed);
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
    let session =
        Session::with_hostile_fixture(crate::gathering::HostileFixture::GatheringRoundEnd);
    let dir = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(session, dir.path().join("parlor.json")), true);
    choose(&mut app, "act:pay:3:3:3");
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
