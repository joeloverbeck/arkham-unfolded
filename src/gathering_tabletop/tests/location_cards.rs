use super::location_markers::{
    accepted_color, link_triangles, location_card, luminance, project, table_with, texts, tokens,
};
use super::*;
use crate::gathering::{HostileFixture, SelectedWitness};
use crate::tabletop_pieces as pieces;
use bevy::camera::primitives::MeshAabb;

/// Lita Chantler's card code, on her board card and her asset.
const LITA: &str = "01117";

const HALLWAY: &str = "01112";
const ATTIC: &str = "01113";
const PARLOR: &str = "01115";
/// The ticket's accepted olive surface and lichen rim.
const OLIVE: Color = Color::srgb(0x2B as f32 / 255., 0x33 as f32 / 255., 0x18 as f32 / 255.);
const LICHEN: Color = Color::srgb(0x8E as f32 / 255., 0x9A as f32 / 255., 0x4A as f32 / 255.);

/// A named board body: its center and full size on the table, and its color.
fn board(app: &mut App, name: &str) -> (Vec3, Vec2, Color) {
    let world = app.world_mut();
    let (mesh, at, material) = world
        .query::<(
            &Name,
            &Mesh3d,
            &Transform,
            &MeshMaterial3d<StandardMaterial>,
        )>()
        .iter(world)
        .find_map(|(entity, mesh, at, material)| {
            (entity.as_str() == name).then(|| (mesh.0.clone(), *at, material.0.clone()))
        })
        .unwrap_or_else(|| panic!("{name} is on the table"));
    let half = world
        .resource::<Assets<Mesh>>()
        .get(&mesh)
        .unwrap()
        .get_aabb()
        .unwrap()
        .half_extents;
    let color = world
        .resource::<Assets<StandardMaterial>>()
        .get(&material)
        .unwrap()
        .base_color;
    (at.translation, Vec2::new(half.x, half.z) * 2., color)
}

fn rim_color(app: &mut App, name: &str) -> Color {
    let world = app.world_mut();
    let material = world
        .query::<(&Name, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .find_map(|(entity, material)| {
            (entity.as_str() == format!("{name} rim")).then(|| material.0.clone())
        })
        .unwrap();
    world
        .resource::<Assets<StandardMaterial>>()
        .get(&material)
        .unwrap()
        .base_color
}

/// The displayed Obscuring Fog copies, by location, in location order.
fn displayed_fog(app: &App) -> Vec<(String, u16)> {
    app.world()
        .resource::<Table>()
        .last_rendered
        .as_ref()
        .and_then(|view| view.play.as_ref())
        .map(|play| {
            play.scenario
                .locations
                .iter()
                .filter_map(|room| room.fog.as_ref().map(|fog| (room.code.clone(), fog.id)))
                .collect()
        })
        .unwrap_or_default()
}

/// A physical card's pose, and its face's full size.
fn card_piece(app: &mut App, id: u16) -> (Vec3, Vec2) {
    let world = app.world_mut();
    let (mesh, at) = world
        .query::<(&motion::PhysicalCard, &Mesh3d, &Transform)>()
        .iter(world)
        .find_map(|(card, mesh, at)| (card.0 == id).then(|| (mesh.0.clone(), *at)))
        .unwrap_or_else(|| panic!("card {id} is on the table"));
    let half = world
        .resource::<Assets<Mesh>>()
        .get(&mesh)
        .unwrap()
        .get_aabb()
        .unwrap()
        .half_extents;
    (
        at.translation,
        Vec2::new(half.x, half.z) * 2. * at.scale.xz(),
    )
}

fn heading(app: &mut App, accessible: &str) -> Vec3 {
    texts(app)
        .into_iter()
        .find_map(|(_, _, label, at)| (label == accessible).then_some(at))
        .unwrap_or_else(|| panic!("{accessible} heads its group"))
}

#[test]
fn attached_cards_leave_the_map_for_the_mirrored_cards_at_locations_board() {
    // The same groups on both boards show that they share one layout.
    let mut app = table_with(&[(HALLWAY, 1), (ATTIC, 1), (PARLOR, 1)], &[HALLWAY, ATTIC]);
    let (enemies, enemy_size, _) = board(&mut app, "Enemies at locations");
    let (location_cards, size, surface) = board(&mut app, "Cards at locations");
    assert_eq!(size, enemy_size, "the boards have the same size");
    assert_eq!(location_cards.z, enemies.z, "the boards share their depth");
    assert_eq!(location_cards.x, -enemies.x, "the boards mirror each other");
    assert!(
        location_cards.x - size.x / 2. > layout::SCENARIO_CENTER.x + layout::SCENARIO_SIZE.x / 2.,
        "the Cards at locations board lies right of Map"
    );
    assert_eq!(surface, OLIVE);
    assert_eq!(rim_color(&mut app, "Cards at locations"), LICHEN);

    let fog = displayed_fog(&app);
    assert_eq!(fog.len(), 2);
    let enemy_ids: Vec<_> = {
        let play = app.world().resource::<Table>().last_rendered.clone();
        let play = play.unwrap().play.unwrap();
        [HALLWAY, ATTIC]
            .map(|room| {
                play.scenario
                    .enemies
                    .iter()
                    .find(|enemy| enemy.location == room)
                    .unwrap()
                    .card
                    .id
            })
            .to_vec()
    };
    let location_size = Vec2::new(pieces::CARD_WIDTH, pieces::CARD_LENGTH);
    let mut previous = f32::MIN;
    for ((room, id), enemy) in fog.iter().zip(enemy_ids) {
        let name = play::location_name(room).to_uppercase();
        let (at, card_size) = card_piece(&mut app, *id);
        assert!(
            (at.x - location_cards.x).abs() + card_size.x / 2. < size.x / 2.,
            "{name}: the attachment lies on its board, not above its location"
        );
        assert!(
            card_size.abs_diff_eq(location_size, 0.001),
            "{name}: full-size card {card_size}"
        );
        let header = heading(&mut app, &format!("Cards at {name} · 1"));
        assert!(
            header.x > location_cards.x - size.x / 2.,
            "{name}: heading on the board"
        );
        assert!(header.z > previous, "groups follow location order");
        previous = header.z;
        assert!(at.z > header.z, "{name}: the card lies beneath its heading");
        // Its place under the heading matches the enemy sideboard's, mirrored.
        let enemy_header = heading(&mut app, &format!("{name} · 1"));
        let (enemy_at, _) = card_piece(&mut app, enemy);
        assert!(
            (at - location_cards).abs_diff_eq(enemy_at - enemies, 0.001)
                && (header - location_cards).abs_diff_eq(enemy_header - enemies, 0.001),
            "{name}: the boards share their group layout"
        );
        // The card keeps its own identity for actions and Inspect.
        let world = app.world_mut();
        let source = world
            .query::<(&motion::PhysicalCard, &ContextTarget)>()
            .iter(world)
            .find_map(|(card, target)| (card.0 == *id).then(|| target.source.clone()));
        assert_eq!(source, Some(controls::Source::Card(*id)));
    }
}

#[test]
fn location_card_markers_are_hexagonal_card_stack_tokens_right_of_enemy_presence() {
    let mut app = table_with(&[(HALLWAY, 2), (ATTIC, 1)], &[HALLWAY, ATTIC, PARLOR]);
    let triangles = link_triangles(&mut app);
    for (room, name, enemies, link_above) in [
        (HALLWAY, "Hallway", true, true),
        (ATTIC, "Attic", true, false),
        (PARLOR, "Parlor", false, false),
    ] {
        let (map, board) = tokens(&mut app, &format!("Cards at locations {name}"));
        assert_eq!((map.len(), board.len()), (1, 1), "{name}: map and heading");
        for token in map.iter().chain(&board) {
            assert!(
                (token.depth / token.radius - 3_f32.sqrt() / 2.).abs() < 0.01,
                "{name}: a hexagonal body"
            );
            assert_eq!(token.layer("Location ring"), accepted_color(room), "{name}");
            assert!(luminance(token.layer("Outline")) < 0.02, "dark outline");
            assert!(luminance(token.layer("Ivory face")) > 0.7, "ivory face");
            assert!(
                luminance(token.layer("Back card")) < 0.02
                    && luminance(token.layer("Front card")) < 0.02,
                "two ink cards"
            );
            assert!(
                luminance(token.layer("Front card edge")) > 0.7,
                "an ivory edge parts the cards"
            );
        }
        let marker = &map[0];
        let card = location_card(&mut app, room);
        let top = card.z - pieces::CARD_LENGTH / 2.;
        let rect =
            Rect::from_center_half_size(marker.at.xz(), Vec2::new(marker.radius, marker.depth));
        assert!(
            rect.max.y < top && rect.min.y > top - 0.75,
            "{name}: in the band"
        );
        let entering: Vec<_> = triangles
            .iter()
            .flatten()
            .filter(|point| {
                point.y < top
                    && point.y > top - 2.
                    && (point.x - card.x).abs() < pieces::CARD_WIDTH / 2.
            })
            .map(|point| point.x)
            .collect();
        assert_eq!(!entering.is_empty(), link_above, "{name}: link above");
        if link_above {
            let right = entering.iter().copied().fold(f32::MIN, f32::max);
            assert!(rect.min.x > right, "{name}: right of the link");
        }
        if enemies {
            let (presence, _) = tokens(&mut app, &format!("Enemy presence {name}"));
            let presence = &presence[0];
            assert!(
                presence.at.x + presence.radius < rect.min.x,
                "{name}: enemy presence left, location cards right"
            );
            assert!(
                (presence.at.z - marker.at.z).abs() < 0.001,
                "{name}: one row"
            );
            if !link_above {
                assert!(
                    ((presence.at.x + marker.at.x) / 2. - card.x).abs() < 0.001,
                    "{name}: the pair is centered"
                );
            }
        } else if !link_above {
            assert!(
                (marker.at.x - card.x).abs() < 0.001,
                "{name}: centered alone"
            );
        }
        let all = texts(&mut app);
        let pill = all
            .iter()
            .find(|(_, _, label, _)| *label == format!("{name} cards: 1"))
            .expect("the location cards marker has a count");
        assert_eq!(pill.1, "1");
        assert!(
            (pill.3.x - marker.at.x).abs() < 0.001,
            "{name}: pill centered"
        );
        assert!(
            pill.3.z >= marker.at.z + marker.depth,
            "{name}: pill beneath"
        );
        let header = all
            .iter()
            .find(|(_, _, label, _)| *label == format!("Cards at {} · 1", name.to_uppercase()))
            .expect("the heading names the location and count");
        assert_eq!(
            header.1,
            crate::tabletop_support::readable_text(format!("{} · 1", name.to_uppercase()))
        );
        assert!(
            header.3.x > board[0].at.x + board[0].radius,
            "token precedes the name"
        );
        assert!((header.3.z - board[0].at.z).abs() < 0.001);
    }
}

/// Step the production systems until the table settles, running `check` after
/// every frame and pressing Continue on a blocking reading.
fn settle_checking(app: &mut App, mut check: impl FnMut(&mut App)) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(50));
    let mut quiet = 0;
    for _ in 0..600 {
        app.update();
        check(app);
        let world = app.world_mut();
        let moving = world
            .query_filtered::<Entity, With<motion::CardMotion>>()
            .iter(world)
            .count();
        let table = world.resource::<Table>();
        let settled =
            !table.is_presenting() && moving == 0 && world.resource::<Actions>().0.is_empty();
        let blocked = table.is_presenting() && table.presentation_blocked();
        if blocked && let Some(next) = button_with_label(app, "Continue") {
            app.world_mut().trigger(Activate { entity: next });
            quiet = 0;
            continue;
        }
        // Automatic steps start within a frame or two of settling.
        quiet = if settled { quiet + 1 } else { 0 };
        if quiet == 5 {
            break;
        }
    }
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::ZERO);
}

/// Settle, checking every frame that the cards-at-locations marker, count and
/// heading match the displayed moment.
fn settle_checking_markers(app: &mut App, fog: u16) {
    settle_checking(app, |app| {
        let displayed = displayed_fog(app)
            .into_iter()
            .find_map(|(room, id)| (id == fog).then_some(room));
        for name in ["Study", "Hallway"] {
            let attached = displayed
                .as_deref()
                .is_some_and(|room| play::location_name(room) == name);
            let (map, board) = tokens(app, &format!("Cards at locations {name}"));
            let labels: Vec<_> = texts(app)
                .into_iter()
                .map(|(_, _, label, _)| label)
                .collect();
            let shown = [
                map.len() == 1,
                board.len() == 1,
                labels.contains(&format!("{name} cards: 1")),
                labels.contains(&format!("Cards at {} · 1", name.to_uppercase())),
            ];
            assert_eq!(
                shown, [attached; 4],
                "{name}: marker, heading token, count and heading follow the displayed moment"
            );
        }
    });
}

fn commit(app: &mut App, id: &str, fog: u16) {
    let (epoch, view, reading) = {
        let table = app.world().resource::<Table>();
        (
            table.epoch,
            table.view(),
            table.presentation.encounter().current().is_some(),
        )
    };
    assert!(
        view.choices.iter().any(|choice| choice.id == id),
        "{id} is offered: {:?}",
        view.choices
    );
    // An open encounter reading answers through its own panel.
    let action = if reading {
        Action::EncounterChoice(epoch, view.revision, id.into())
    } else {
        Action::Choose(epoch, view.revision, id.into())
    };
    app.world_mut().resource_mut::<Actions>().0.push(action);
    settle_checking_markers(app, fog);
}

fn investigate(app: &mut App, fog: u16) {
    commit(app, "action:ordinary:investigate", fog);
    for _ in 0..8 {
        let view = app.world().resource::<Table>().view();
        let Some(next) = ["window:pass", "commit:confirm"]
            .into_iter()
            .find(|id| view.choices.iter().any(|choice| choice.id == *id))
        else {
            break;
        };
        commit(app, next, fog);
    }
    commit(app, "test:reveal", fog);
}

#[test]
fn attaching_and_discarding_move_the_card_marker_and_count_together() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::with_hostile_fixture(HostileFixture::SelectedWitness(
        SelectedWitness::ObscuringFog,
    ));
    let fog = session
        .observe()
        .play
        .unwrap()
        .pending_revelation
        .unwrap()
        .card
        .id;
    let mut app = host_with_table(
        Table::new(session, directory.path().join("fog.json")),
        false,
    );
    settle_checking_markers(&mut app, fog);
    assert!(displayed_fog(&app).is_empty());

    commit(&mut app, "revelation:resolve", fog);
    let attached = displayed_fog(&app);
    assert_eq!(attached.len(), 1, "the Fog is attached");
    assert_eq!(attached[0].1, fog);
    let name = play::location_name(&attached[0].0).to_uppercase();
    let (at, _) = card_piece(&mut app, fog);
    let header = heading(&mut app, &format!("Cards at {name} · 1"));
    assert!(
        at.x > 9.8 && at.z > header.z,
        "the card lands under its heading"
    );

    // The fixture's first investigation fails and the second succeeds.
    investigate(&mut app, fog);
    assert_eq!(displayed_fog(&app).len(), 1, "a failed test keeps the Fog");
    investigate(&mut app, fog);
    assert!(displayed_fog(&app).is_empty(), "success discards the Fog");
    let (at, _) = card_piece(&mut app, fog);
    assert!(
        at.xz().abs_diff_eq(layout::ENCOUNTER_DISCARD.xz(), 0.001),
        "the card reaches the encounter discard: {at}"
    );
    let (board, _, _) = board(&mut app, "Cards at locations");
    assert!(board.x > 9.8, "the empty board remains");
}

#[test]
fn a_location_cards_heading_lights_only_its_own_markers() {
    let mut app = table_with(&[(HALLWAY, 1)], &[HALLWAY]);
    let rings = |app: &mut App, name: &str| -> Vec<Color> {
        let (map, board) = tokens(app, name);
        map.iter()
            .chain(&board)
            .map(|token| token.layer("Location ring"))
            .collect()
    };
    app.update();
    assert!(
        rings(&mut app, "Cards at locations Hallway")
            .iter()
            .all(|color| *color == accepted_color(HALLWAY))
    );
    let camera = app.world().resource::<Orbit>().pose();
    let header = app
        .world_mut()
        .query::<(Entity, &AccessibleLabel)>()
        .iter(app.world())
        .find_map(|(entity, label)| (label.0 == "Cards at HALLWAY · 1").then_some(entity))
        .unwrap();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(header, bevy::input_focus::FocusCause::Navigated);
    app.update();
    assert_eq!(
        rings(&mut app, "Cards at locations Hallway"),
        [Color::WHITE; 2]
    );
    assert_eq!(
        rings(&mut app, "Enemy presence Hallway"),
        [accepted_color(HALLWAY); 2],
        "the enemy group at the same location stays unlit"
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
}

#[test]
fn scenario_and_overview_frame_both_sideboards_without_moving_the_camera_back() {
    for size in [Vec2::new(1600., 1000.), Vec2::new(1280., 800.)] {
        for (area, distance) in [(Area::Scenario, 40.), (Area::Overview, 72.)] {
            let mut app = table_with(&[(HALLWAY, 1)], &[HALLWAY]);
            crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
            let (target, focus_distance) = sideboard::focus(area);
            assert_eq!(
                focus_distance, distance,
                "{area:?}: the distance is unchanged"
            );
            app.world_mut()
                .resource_mut::<Orbit>()
                .focus((target, focus_distance));
            for _ in 0..4 {
                app.update();
            }
            let mut edges = Vec::new();
            for name in ["Enemies at locations", "Cards at locations"] {
                let (center, extent, _) = board(&mut app, name);
                let decision_top = size.y - (size.y * 0.29).min(270.) - 10.;
                for corner in [
                    Vec2::new(-1., -1.),
                    Vec2::new(1., -1.),
                    Vec2::new(-1., 1.),
                    Vec2::new(1., 1.),
                ] {
                    let point = center + (corner * extent / 2.).extend(0.).xzy();
                    let screen = project(&mut app, point);
                    assert!(
                        screen.x >= 150. && screen.x <= size.x - 12.,
                        "{size} {area:?}: {name} corner {point} at {screen} leaves the view"
                    );
                    if area == Area::Scenario {
                        assert!(
                            screen.y >= 104. && screen.y <= decision_top - 8.,
                            "{size} {area:?}: {name} corner {point} at {screen} is under controls"
                        );
                    }
                    edges.push(screen.x);
                }
            }
            // Re-centred: the outer edges sit equally far from the viewport's center.
            let left = edges.iter().copied().fold(f32::MAX, f32::min);
            let right = edges.iter().copied().fold(f32::MIN, f32::max);
            assert!(
                ((left + right) / 2. - size.x / 2.).abs() < 1.,
                "{size} {area:?}: {left}..{right} is centered"
            );
        }
    }
}

/// The Parley witness's public moment, edited only in what the board shows:
/// uncontrolled Lita at the Parlor, Obscuring Fog attached there, and all three
/// investigators in the Parlor. The witness already has Patrice there with the
/// Parlor revealed.
fn parlor_moment(lita: bool, fog: bool, crowd: bool) -> crate::gathering::PlayObservation {
    let session = Session::with_hostile_fixture(HostileFixture::GatheringLitaParley);
    let mut play = session.observe().play.unwrap();
    assert!(play.scenario.lita_at_parlor.is_some());
    if !lita {
        play.scenario.lita_at_parlor = None;
    }
    for location in &mut play.scenario.locations {
        location.fog = (fog && location.code == PARLOR).then(|| crate::gathering::CardView {
            id: 62100,
            code: "01168".into(),
            title: "Obscuring Fog".into(),
            weakness: false,
        });
    }
    if crowd {
        for player in &mut play.players {
            player.location = PARLOR.into();
        }
    }
    play
}

fn parlor_table(play: crate::gathering::PlayObservation) -> App {
    let session = Session::with_hostile_fixture(HostileFixture::GatheringLitaParley);
    super::cards::at_public_moment(session, play, PathBuf::new(), false)
}

/// The card face showing a source, and where it lies.
fn face(app: &mut App, source: &controls::Source) -> Option<(Entity, Vec3)> {
    let world = app.world_mut();
    world
        .query_filtered::<(Entity, &ContextTarget, &Transform), With<CardFeedback>>()
        .iter(world)
        .find_map(|(entity, target, at)| {
            (target.source == *source).then_some((entity, at.translation))
        })
}

fn caption_label(app: &mut App, card: Entity) -> String {
    let world = app.world_mut();
    world
        .query::<(&motion::CardLabel, &AccessibleLabel)>()
        .iter(world)
        .find_map(|(label, accessible)| (label.0 == card).then(|| accessible.0.clone()))
        .expect("a board card keeps its accessible caption")
}

/// The Parlor's group as shown: its heading's accessible name and its map count,
/// checking that the map marker and heading token appear exactly with them.
fn parlor_group(app: &mut App) -> Option<(String, String)> {
    let (map, board) = tokens(app, "Cards at locations Parlor");
    let all = texts(app);
    let heading = all
        .iter()
        .find(|(_, _, label, _)| label.starts_with("Cards at PARLOR · "))
        .map(|(_, _, label, _)| label.clone());
    let pill = all
        .iter()
        .find(|(_, _, label, _)| label.starts_with("Parlor cards: "))
        .map(|(_, text, _, _)| text.clone());
    assert_eq!(
        [map.len(), board.len()],
        [heading.is_some() as usize; 2],
        "the map marker and heading token follow the heading"
    );
    assert_eq!(
        heading.is_some(),
        pill.is_some(),
        "the count follows the heading"
    );
    heading.zip(pill)
}

#[test]
fn a_crowded_parlor_shows_lita_and_fog_under_one_heading_off_the_map() {
    let mut app = parlor_table(parlor_moment(true, true, true));
    assert_eq!(
        parlor_group(&mut app),
        Some(("Cards at PARLOR · 2".into(), "2".into()))
    );
    let header = heading(&mut app, "Cards at PARLOR · 2");
    let (board, size, _) = board(&mut app, "Cards at locations");
    let (lita, lita_at) = face(&mut app, &controls::Source::Lita).expect("Lita is shown");
    let (fog, fog_at) = face(&mut app, &controls::Source::Card(62100)).expect("Fog is shown");
    for (name, at) in [("Lita", lita_at), ("Fog", fog_at)] {
        assert!(
            (at.x - board.x).abs() + pieces::CARD_WIDTH / 2. < size.x / 2.,
            "{name} lies on the Cards at locations board, not on Map: {at}"
        );
        assert!(at.z > header.z, "{name} lies under the Parlor heading");
    }
    // The character comes first; each card keeps its own place and relationship.
    assert!(lita_at.x < fog_at.x && (lita_at.z - fog_at.z).abs() < 0.001);
    assert_eq!(caption_label(&mut app, lita), "Lita Chantler\nAt Parlor");
    assert_eq!(
        caption_label(&mut app, fog),
        "Obscuring Fog\nAttached to Parlor"
    );
    // Every investigator stands at the Parlor, clear of its card and the board.
    let parlor = location_card(&mut app, PARLOR);
    let standees: Vec<_> = {
        let world = app.world_mut();
        world
            .query::<(&crate::gathering_tabletop::map_motion::Surface, &Transform)>()
            .iter(world)
            .filter_map(|(surface, at)| {
                matches!(
                    surface,
                    crate::gathering_tabletop::map_motion::Surface::Investigator(_)
                )
                .then_some(at.translation)
            })
            .collect()
    };
    assert_eq!(standees.len(), 3);
    for at in standees {
        assert!(
            (at.x - parlor.x).abs() > pieces::CARD_WIDTH / 2.
                && (at.x - parlor.x).abs() < pieces::CARD_WIDTH * 2.,
            "a Parlor standee beside its card: {at}"
        );
        assert!(at.x < board.x - size.x / 2., "standees stay on Map");
    }
}

#[test]
fn lita_alone_or_fog_alone_forms_the_parlor_group_and_none_leaves_no_marker() {
    for (lita, fog, expected) in [
        (true, false, Some("Lita Chantler\nAt Parlor")),
        (false, true, Some("Obscuring Fog\nAttached to Parlor")),
        (false, false, None),
    ] {
        // Lita's card uses the board even when the Parlor is empty.
        let mut app = parlor_table(parlor_moment(lita, fog, false));
        let shown = parlor_group(&mut app);
        assert_eq!(
            shown,
            expected.map(|_| ("Cards at PARLOR · 1".to_owned(), "1".to_owned())),
            "lita {lita}, fog {fog}"
        );
        let source = if lita {
            controls::Source::Lita
        } else {
            controls::Source::Card(62100)
        };
        match expected {
            Some(caption) => {
                let (card, _) = face(&mut app, &source).unwrap();
                assert_eq!(caption_label(&mut app, card), caption);
            }
            None => assert!(face(&mut app, &controls::Source::Lita).is_none()),
        }
    }
}

#[test]
fn lita_lights_her_parlor_group_without_moving_the_camera() {
    let mut app = parlor_table(parlor_moment(true, true, false));
    app.update();
    let (lita, _) = face(&mut app, &controls::Source::Lita).unwrap();
    let camera = app.world().resource::<Orbit>().pose();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(lita, bevy::input_focus::FocusCause::Navigated);
    app.update();
    let (map, board) = tokens(&mut app, "Cards at locations Parlor");
    assert!(
        map.iter()
            .chain(&board)
            .all(|token| token.layer("Location ring") == Color::WHITE),
        "her group's marker and heading light"
    );
    assert_eq!(app.world().resource::<Orbit>().pose(), camera);
}

#[test]
fn taking_control_of_lita_leaves_the_fog_alone_under_the_parlor() {
    let before = parlor_moment(true, true, true);
    let mut after = before.clone();
    // Parley moves her own card, as the rules do.
    let lita = after.scenario.lita_at_parlor.take().unwrap();
    let id = lita.id;
    let patrice = &mut after.players[Investigator::Patrice.index()];
    let mut asset = patrice.assets[0].clone();
    asset.card = lita;
    patrice.assets.push(asset);
    let mut app = parlor_table(before);
    assert_eq!(parlor_group(&mut app).unwrap().1, "2");
    let mut app = parlor_table(after);
    assert_eq!(
        parlor_group(&mut app),
        Some(("Cards at PARLOR · 1".into(), "1".into()))
    );
    assert!(face(&mut app, &controls::Source::Lita).is_none());
    let (fog, _) = face(&mut app, &controls::Source::Card(62100)).unwrap();
    assert_eq!(
        caption_label(&mut app, fog),
        "Obscuring Fog\nAttached to Parlor"
    );
    let world = app.world_mut();
    let shown = world
        .query::<&motion::PhysicalCard>()
        .iter(world)
        .filter(|card| card.0 == id)
        .count();
    assert_eq!(shown, 1, "Lita appears once, among Patrice's assets");
}

/// Settle, checking every frame that Lita's board card, the Parlor heading,
/// marker and count, and her asset follow the same displayed moment. Returns
/// where her one physical card stood at each frame.
fn settle_checking_lita(app: &mut App) -> Vec<Vec3> {
    let mut trail = Vec::new();
    settle_checking(app, |app| {
        let displayed = app
            .world()
            .resource::<Table>()
            .last_rendered
            .as_ref()
            .and_then(|view| view.play.clone())
            .unwrap();
        let at_parlor = displayed.scenario.lita_at_parlor.is_some();
        let controlled = displayed
            .players
            .iter()
            .flat_map(|player| &player.assets)
            .filter(|asset| asset.card.code == LITA)
            .count();
        assert_eq!(
            (
                parlor_group(app).is_some(),
                face(app, &controls::Source::Lita).is_some()
            ),
            (at_parlor, at_parlor),
            "Lita's card and the Parlor group follow the displayed moment"
        );
        assert_eq!(controlled, usize::from(!at_parlor), "Lita is shown once");
        let id = displayed
            .scenario
            .lita_at_parlor
            .iter()
            .chain(
                displayed
                    .players
                    .iter()
                    .flat_map(|player| player.assets.iter().map(|asset| &asset.card)),
            )
            .find(|card| card.code == LITA)
            .map(|card| card.id);
        let world = app.world_mut();
        let shown: Vec<_> = world
            .query::<(&motion::PhysicalCard, &Transform)>()
            .iter(world)
            .filter(|(card, _)| Some(card.0) == id)
            .map(|(_, at)| at.translation)
            .collect();
        assert!(shown.len() <= 1, "one physical Lita card");
        trail.extend(shown);
    });
    trail
}

/// Answer the current Parley test through its visible controls: commit the
/// given card, pass windows, reveal and decline Lucky!. Returns Lita's card
/// positions frame by frame.
fn finish_parley(app: &mut App, mut commit: Option<u16>) -> Vec<Vec3> {
    let mut started = false;
    let mut trail = Vec::new();
    for _ in 0..12 {
        trail.extend(settle_checking_lita(app));
        let view = app.world().resource::<Table>().view();
        let testing = view.play.as_ref().is_some_and(|play| play.test.is_some());
        if started && !testing {
            return trail;
        }
        started |= testing;
        let has = |id: &str| view.choices.iter().any(|choice| choice.id == id);
        // Committing toggles, so each card is committed once.
        let next = if let Some(card) = commit.take_if(|card| has(&format!("commit:{card}"))) {
            format!("commit:{card}")
        } else if let Some(id) = [
            "commit:confirm",
            "window:pass",
            "test:reveal",
            "lucky:decline",
        ]
        .into_iter()
        .find(|id| has(id))
        {
            id.to_owned()
        } else {
            return trail;
        };
        activate_choice(app, &next);
    }
    panic!("the Parley test must finish");
}

fn parley_row(app: &mut App) -> Option<Entity> {
    let view = app.world().resource::<Table>().view();
    let parley = view
        .choices
        .iter()
        .find(|choice| choice.id == "action:ordinary:parley:lita")?;
    choice_button(app, ui::choice_label(parley))
}

#[test]
fn lita_offers_parley_from_her_board_card_and_slides_to_assets_once_on_success() {
    let session = Session::with_hostile_fixture(HostileFixture::GatheringLitaParley);
    let patrice = session.observe().play.unwrap().players[Investigator::Patrice.index()].clone();
    let rise = patrice
        .hand
        .iter()
        .find(|card| card.code == "02192")
        .unwrap()
        .id;
    let peter = patrice
        .assets
        .iter()
        .find(|asset| asset.card.code == "02033")
        .unwrap()
        .card
        .id;

    // Outside the Parlor her card has nothing to choose, so it opens her inspection.
    let mut away = session.clone();
    let revision = away.observe().revision;
    away.choose(revision, "action:ordinary:move:01112").unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut app = host_with_table(Table::new(away, directory.path().join("away.json")), false);
    settle_checking_lita(&mut app);
    assert!(
        !app.world()
            .resource::<Table>()
            .view()
            .choices
            .iter()
            .any(|choice| choice.id.contains("parley")),
        "the rules offer no Parley outside the Parlor"
    );
    click_source(&mut app, controls::Source::Lita);
    assert!(parley_row(&mut app).is_none());
    assert!(
        app.world().resource::<Table>().modal(),
        "her inspection opens"
    );

    // In the Parlor her card offers Parley and Inspect. A failed test keeps her.
    let mut app = host_with_table(
        Table::new(session.clone(), directory.path().join("fail.json")),
        false,
    );
    settle_checking_lita(&mut app);
    click_source(&mut app, controls::Source::Lita);
    let inspect = choice_button(&mut app, "Inspect").expect("Lita can be inspected");
    app.world_mut().trigger(Activate { entity: inspect });
    app.update();
    assert!(
        app.world().resource::<Table>().modal(),
        "her inspection opens"
    );
    app.world_mut()
        .resource_mut::<Actions>()
        .0
        .push(Action::Close);
    app.update();
    click_source(&mut app, controls::Source::Lita);
    let row = parley_row(&mut app).expect("the rules offer Parley here");
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    finish_parley(&mut app, None);
    settle_checking_lita(&mut app);
    let play = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    assert!(
        play.scenario.lita_at_parlor.is_some(),
        "a failed Parley keeps her at the Parlor"
    );
    assert!(parlor_group(&mut app).is_some());

    // Success: she joins Patrice's assets and the Parlor group leaves with her.
    let mut app = host_with_table(
        Table::new(session, directory.path().join("success.json")),
        false,
    );
    settle_checking_lita(&mut app);
    click_source(&mut app, controls::Source::Lita);
    let row = parley_row(&mut app).unwrap();
    app.world_mut().trigger(Activate { entity: row });
    app.update();
    let mut trail = finish_parley(&mut app, Some(rise));
    trail.extend(settle_checking_lita(&mut app));
    // Her one card slides from the board to the Assets row, not a cut.
    let (start, end) = (trail[0], *trail.last().unwrap());
    let board = sideboard::LOCATION_CARDS.center.x - sideboard::LOCATION_CARDS.size.x / 2.;
    assert!(
        start.x > board && end.x < board,
        "from the board to Assets: {start} -> {end}"
    );
    assert!(
        trail
            .iter()
            .any(|at| at.distance(start) > 1. && at.distance(end) > 1.),
        "the card passes between them: {trail:?}"
    );
    let play = app
        .world()
        .resource::<Table>()
        .session
        .observe()
        .play
        .unwrap();
    let assets = &play.players[Investigator::Patrice.index()].assets;
    assert!(assets.iter().any(|asset| asset.card.code == LITA));
    assert!(
        !assets.iter().any(|asset| asset.card.id == peter),
        "the ordinary Ally-slot resolution makes room"
    );
    assert!(parlor_group(&mut app).is_none());
    assert!(face(&mut app, &controls::Source::Lita).is_none());
}
