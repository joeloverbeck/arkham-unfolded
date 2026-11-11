use super::location_markers::{
    accepted_color, link_triangles, location_card, luminance, project, table_with, texts, tokens,
};
use super::*;
use crate::gathering::{HostileFixture, SelectedWitness};
use crate::tabletop_pieces as pieces;
use bevy::camera::primitives::MeshAabb;

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
fn attached_cards_leave_the_map_for_a_mirrored_attachment_sideboard() {
    // The same groups on both boards show that they share one layout.
    let mut app = table_with(&[(HALLWAY, 1), (ATTIC, 1), (PARLOR, 1)], &[HALLWAY, ATTIC]);
    let (enemies, enemy_size, _) = board(&mut app, "Enemies at locations");
    let (attachments, size, surface) = board(&mut app, "Attachments at locations");
    assert_eq!(size, enemy_size, "the boards have the same size");
    assert_eq!(attachments.z, enemies.z, "the boards share their depth");
    assert_eq!(attachments.x, -enemies.x, "the boards mirror each other");
    assert!(
        attachments.x - size.x / 2. > layout::SCENARIO_CENTER.x + layout::SCENARIO_SIZE.x / 2.,
        "the attachment sideboard lies right of Map"
    );
    assert_eq!(surface, OLIVE);
    assert_eq!(rim_color(&mut app, "Attachments at locations"), LICHEN);

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
            (at.x - attachments.x).abs() + card_size.x / 2. < size.x / 2.,
            "{name}: the attachment lies on its board, not above its location"
        );
        assert!(
            card_size.abs_diff_eq(location_size, 0.001),
            "{name}: full-size card {card_size}"
        );
        let header = heading(&mut app, &format!("Attached to {name} · 1"));
        assert!(
            header.x > attachments.x - size.x / 2.,
            "{name}: heading on the board"
        );
        assert!(header.z > previous, "groups follow location order");
        previous = header.z;
        assert!(at.z > header.z, "{name}: the card lies beneath its heading");
        // Its place under the heading matches the enemy sideboard's, mirrored.
        let enemy_header = heading(&mut app, &format!("{name} · 1"));
        let (enemy_at, _) = card_piece(&mut app, enemy);
        assert!(
            (at - attachments).abs_diff_eq(enemy_at - enemies, 0.001)
                && (header - attachments).abs_diff_eq(enemy_header - enemies, 0.001),
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
fn attachment_markers_are_hexagonal_chain_link_tokens_right_of_enemy_presence() {
    let mut app = table_with(&[(HALLWAY, 2), (ATTIC, 1)], &[HALLWAY, ATTIC, PARLOR]);
    let triangles = link_triangles(&mut app);
    for (room, name, enemies, link_above) in [
        (HALLWAY, "Hallway", true, true),
        (ATTIC, "Attic", true, false),
        (PARLOR, "Parlor", false, false),
    ] {
        let (map, board) = tokens(&mut app, &format!("Attachment {name}"));
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
                luminance(token.layer("Chain link")) < 0.02,
                "ink chain link"
            );
            assert!(luminance(token.layer("Link opening")) > 0.7, "open links");
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
                "{name}: enemy presence left, attachment right"
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
            .find(|(_, _, label, _)| *label == format!("{name} attachments: 1"))
            .expect("the attachment marker has a count");
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
            .find(|(_, _, label, _)| *label == format!("Attached to {} · 1", name.to_uppercase()))
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

/// Step the production systems until the table settles, checking every frame
/// that the attachment marker, count and heading match the displayed moment.
fn settle_checking_markers(app: &mut App, fog: u16) {
    app.world_mut()
        .resource_mut::<PresentationTime>()
        .advance_by(std::time::Duration::from_millis(50));
    let mut quiet = 0;
    for _ in 0..600 {
        app.update();
        let displayed = displayed_fog(app)
            .into_iter()
            .find_map(|(room, id)| (id == fog).then_some(room));
        for name in ["Study", "Hallway"] {
            let attached = displayed
                .as_deref()
                .is_some_and(|room| play::location_name(room) == name);
            let (map, board) = tokens(app, &format!("Attachment {name}"));
            let labels: Vec<_> = texts(app)
                .into_iter()
                .map(|(_, _, label, _)| label)
                .collect();
            let shown = [
                map.len() == 1,
                board.len() == 1,
                labels.contains(&format!("{name} attachments: 1")),
                labels.contains(&format!("Attached to {} · 1", name.to_uppercase())),
            ];
            assert_eq!(
                shown, [attached; 4],
                "{name}: marker, heading token, count and heading follow the displayed moment"
            );
        }
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
    let header = heading(&mut app, &format!("Attached to {name} · 1"));
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
    let (board, _, _) = board(&mut app, "Attachments at locations");
    assert!(board.x > 9.8, "the empty board remains");
}

#[test]
fn an_attachment_heading_lights_only_its_own_markers() {
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
        rings(&mut app, "Attachment Hallway")
            .iter()
            .all(|color| *color == accepted_color(HALLWAY))
    );
    let camera = app.world().resource::<Orbit>().pose();
    let header = app
        .world_mut()
        .query::<(Entity, &AccessibleLabel)>()
        .iter(app.world())
        .find_map(|(entity, label)| (label.0 == "Attached to HALLWAY · 1").then_some(entity))
        .unwrap();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(header, bevy::input_focus::FocusCause::Navigated);
    app.update();
    assert_eq!(rings(&mut app, "Attachment Hallway"), [Color::WHITE; 2]);
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
            for name in ["Enemies at locations", "Attachments at locations"] {
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
