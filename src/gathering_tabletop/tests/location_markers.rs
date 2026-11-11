use super::*;
use crate::gathering::{HostileFixture, TonyEffectFixture};
use crate::tabletop_pieces as pieces;

const HALLWAY: &str = "01112";
const ATTIC: &str = "01113";
const CELLAR: &str = "01114";
const PARLOR: &str = "01115";

/// The ticket's accepted Okabe–Ito assignment.
fn accepted_color(room: &str) -> Color {
    match room {
        "01111" | HALLWAY => Color::srgb_u8(0xD5, 0x5E, 0x00),
        ATTIC => Color::srgb_u8(0xF0, 0xE4, 0x42),
        CELLAR => Color::srgb_u8(0, 0x72, 0xB2),
        PARLOR => Color::srgb_u8(0xCC, 0x79, 0xA7),
        _ => unreachable!(),
    }
}

/// Unengaged enemies in the requested sideboard groups, at a public moment.
fn table_with_groups(groups: &[(&str, usize)]) -> App {
    let session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::QuickThinking));
    let mut play = session.observe().play.unwrap();
    let template = play.scenario.enemies[0].clone();
    play.scenario.enemies.clear();
    let mut id = 62000;
    for &(room, count) in groups {
        for _ in 0..count {
            let mut enemy = template.clone();
            enemy.card.id = id;
            enemy.engaged = None;
            enemy.exhausted = false;
            enemy.bounties = 0;
            enemy.doom = 0;
            enemy.location = room.into();
            play.scenario.enemies.push(enemy);
            id += 1;
        }
    }
    super::cards::at_public_moment(session, play, PathBuf::new(), false)
}

struct Token {
    at: Vec3,
    radius: f32,
    layers: Vec<(String, Color)>,
}

impl Token {
    fn layer(&self, name: &str) -> Color {
        self.layers
            .iter()
            .find_map(|(layer, color)| (layer == name).then_some(*color))
            .unwrap_or_else(|| panic!("token lacks its {name}"))
    }

    fn rect(&self) -> Rect {
        Rect::from_center_half_size(self.at.xz(), Vec2::splat(self.radius))
    }
}

/// Map tokens and sideboard heading tokens named for a location group.
fn tokens(app: &mut App, name: &str) -> (Vec<Token>, Vec<Token>) {
    use bevy::camera::primitives::MeshAabb;
    let world = app.world_mut();
    let roots: Vec<_> = world
        .query::<(
            Entity,
            &Name,
            &Transform,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        )>()
        .iter(world)
        .filter(|(_, root, ..)| root.as_str() == name)
        .map(|(entity, _, at, mesh, material)| {
            (entity, at.translation, mesh.0.clone(), material.0.clone())
        })
        .collect();
    let color = |world: &World, handle: &Handle<StandardMaterial>| {
        world
            .resource::<Assets<StandardMaterial>>()
            .get(handle)
            .unwrap()
            .base_color
    };
    let (mut map, mut board) = (Vec::new(), Vec::new());
    for (entity, at, mesh, material) in roots {
        let radius = world
            .resource::<Assets<Mesh>>()
            .get(&mesh)
            .unwrap()
            .get_aabb()
            .unwrap()
            .half_extents
            .x;
        let mut layers = vec![("Outline".to_owned(), color(world, &material))];
        for child in world.get::<Children>(entity).unwrap().iter() {
            let name = world.get::<Name>(child).unwrap().to_string();
            let material = world
                .get::<MeshMaterial3d<StandardMaterial>>(child)
                .unwrap();
            layers.push((name, color(world, &material.0)));
        }
        let token = Token { at, radius, layers };
        (if at.x < -9.8 { &mut board } else { &mut map }).push(token);
    }
    (map, board)
}

fn luminance(color: Color) -> f32 {
    color.to_linear().luminance()
}

fn texts(app: &mut App) -> Vec<(Entity, String, String, Vec3)> {
    let world = app.world_mut();
    world
        .query::<(Entity, &Text, &AccessibleLabel, &WorldLabel)>()
        .iter(world)
        .map(|(entity, text, label, at)| (entity, text.0.clone(), label.0.clone(), at.0))
        .collect()
}

fn location_card(app: &mut App, room: &str) -> Vec3 {
    let world = app.world_mut();
    world
        .query::<(&crate::gathering_tabletop::map_motion::Surface, &Transform)>()
        .iter(world)
        .find_map(|(surface, at)| {
            (*surface == crate::gathering_tabletop::map_motion::Surface::Location(room.into()))
                .then_some(at.translation)
        })
        .unwrap()
}

/// Every brass triangle of the displayed links, in table (x, z) coordinates.
fn link_triangles(app: &mut App) -> Vec<[Vec2; 3]> {
    use bevy::mesh::VertexAttributeValues;
    let world = app.world_mut();
    let links: Vec<_> = world
        .query_filtered::<&Children, With<crate::tabletop_gathering_map::ConnectionVisual>>()
        .iter(world)
        .flat_map(|children| children.iter().collect::<Vec<_>>())
        .collect();
    let mut triangles = Vec::new();
    for layer in links {
        let mesh = world.get::<Mesh3d>(layer).unwrap().0.clone();
        let meshes = world.resource::<Assets<Mesh>>();
        let Some(VertexAttributeValues::Float32x3(points)) = meshes
            .get(&mesh)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("links expose positions");
        };
        for triangle in points.chunks_exact(3) {
            triangles
                .push([0, 1, 2].map(|index| Vec2::new(triangle[index][0], triangle[index][2])));
        }
    }
    triangles
}

#[test]
fn location_colors_mark_the_ring_and_iris_of_map_and_heading_tokens() {
    let mut app = table_with_groups(&[(HALLWAY, 2), (ATTIC, 1), (CELLAR, 1), (PARLOR, 1)]);
    let mut seen = Vec::new();
    for (room, name) in [
        (HALLWAY, "Hallway"),
        (ATTIC, "Attic"),
        (CELLAR, "Cellar"),
        (PARLOR, "Parlor"),
    ] {
        let (map, board) = tokens(&mut app, &format!("Enemy presence {name}"));
        assert_eq!((map.len(), board.len()), (1, 1), "{name}: map and heading");
        for token in map.iter().chain(&board) {
            assert_eq!(token.layer("Location ring"), accepted_color(room), "{name}");
            assert_eq!(token.layer("Iris"), accepted_color(room), "{name}");
            assert!(luminance(token.layer("Outline")) < 0.02, "dark outline");
            assert!(luminance(token.layer("Ivory face")) > 0.7, "ivory face");
            for ink in ["Slit-eye emblem", "Pupil slit"] {
                assert!(luminance(token.layer(ink)) < 0.02, "{name}: ink {ink}");
            }
        }
        seen.push(accepted_color(room));
    }
    for (index, color) in seen.iter().enumerate() {
        assert!(
            !seen[index + 1..].contains(color),
            "Locations in play together never share a color"
        );
    }
    // Identity, not occupancy, decides the color: refilling keeps the group's token.
    let mut emptied = table_with_groups(&[(PARLOR, 1)]);
    assert!(tokens(&mut emptied, "Enemy presence Hallway").0.is_empty());
    let mut refilled = table_with_groups(&[(PARLOR, 1), (HALLWAY, 1)]);
    let (map, _) = tokens(&mut refilled, "Enemy presence Hallway");
    assert_eq!(map[0].layer("Location ring"), accepted_color(HALLWAY));
}

#[test]
fn study_shares_the_hallway_color_because_they_are_never_in_play_together() {
    let session =
        Session::with_hostile_fixture(HostileFixture::Tony(TonyEffectFixture::QuickThinking));
    let mut play = session.observe().play.unwrap();
    let mut study = play.scenario.locations[0].clone();
    study.code = "01111".into();
    play.scenario.locations = vec![study];
    for player in &mut play.players {
        player.location = "01111".into();
    }
    for enemy in &mut play.scenario.enemies {
        enemy.engaged = None;
        enemy.location = "01111".into();
    }
    let mut app = super::cards::at_public_moment(session, play, PathBuf::new(), false);
    let (map, board) = tokens(&mut app, "Enemy presence Study");
    assert_eq!((map.len(), board.len()), (1, 1));
    assert_eq!(map[0].layer("Location ring"), accepted_color(HALLWAY));
}

#[test]
fn markers_and_headings_show_counts_without_letters() {
    let mut app = table_with_groups(&[(HALLWAY, 3), (PARLOR, 1)]);
    let (map, board) = tokens(&mut app, "Enemy presence Hallway");
    let texts = texts(&mut app);
    let pill = texts
        .iter()
        .find(|(_, _, label, _)| label == "Hallway enemies: 3")
        .expect("the Hallway marker has a count");
    assert_eq!(pill.1, "3");
    assert!(
        app.world()
            .get::<BackgroundColor>(pill.0)
            .unwrap()
            .0
            .alpha()
            > 0.9,
        "the count sits on a dark pill"
    );
    assert!(luminance(app.world().get::<BackgroundColor>(pill.0).unwrap().0) < 0.02);
    assert!((pill.3.x - map[0].at.x).abs() < 0.001, "pill is centered");
    assert!(
        pill.3.z >= map[0].at.z + map[0].radius - 0.001,
        "pill is beneath the token"
    );
    let heading = texts
        .iter()
        .find(|(_, _, label, _)| label == "HALLWAY · 3")
        .expect("the heading names the location and count");
    assert!(
        heading.3.x > board[0].at.x + board[0].radius,
        "the heading token precedes the name"
    );
    assert!((heading.3.z - board[0].at.z).abs() < 0.001);
    assert!(texts.iter().any(|(_, _, label, _)| label == "PARLOR · 1"));
    for (_, text, label, _) in &texts {
        for value in [text, label] {
            let mut words = value.split_whitespace();
            assert!(
                !(words.next().is_some_and(
                    |word| word.len() == 1 && word.chars().all(|c| c.is_ascii_uppercase())
                ) && matches!(words.next(), Some("·" | "/"))),
                "no marker letter in {value:?}"
            );
        }
    }
}

#[test]
fn markers_sit_in_a_band_above_the_card_and_flank_a_link_entering_from_above() {
    let mut app = table_with_groups(&[(HALLWAY, 2), (ATTIC, 1), (CELLAR, 1), (PARLOR, 1)]);
    let triangles = link_triangles(&mut app);
    assert!(!triangles.is_empty());
    let standees: Vec<_> = {
        let world = app.world_mut();
        world
            .query::<(&crate::gathering_tabletop::map_motion::Surface, &Transform)>()
            .iter(world)
            .filter(|(surface, _)| {
                matches!(
                    surface,
                    crate::gathering_tabletop::map_motion::Surface::Investigator(_)
                )
            })
            .map(|(_, at)| at.translation.xz())
            .collect()
    };
    assert!(!standees.is_empty());
    let standee_radius =
        crate::tabletop_standees::radius_for(Vec2::new(pieces::CARD_WIDTH, pieces::CARD_LENGTH));
    // In The Gathering's arrangement Attic's link reaches the Hallway's top edge
    // and the Hallway's link reaches the Cellar's; Attic and Parlor have none above.
    for (room, name, link_above) in [
        (HALLWAY, "Hallway", true),
        (ATTIC, "Attic", false),
        (CELLAR, "Cellar", true),
        (PARLOR, "Parlor", false),
    ] {
        let card = location_card(&mut app, room);
        let top = card.z - pieces::CARD_LENGTH / 2.;
        let (map, _) = tokens(&mut app, &format!("Enemy presence {name}"));
        let token = &map[0];
        let rect = token.rect();
        assert!(rect.max.y < top, "{name}: the token stays off the card");
        assert!(rect.min.y > top - 1., "{name}: the token stays in the band");
        let entering: Vec<_> = triangles
            .iter()
            .flatten()
            .filter(|point| {
                point.y < top
                    && point.y > top - 2.
                    && (point.x - card.x).abs() < pieces::CARD_WIDTH / 2.
            })
            .collect();
        assert_eq!(!entering.is_empty(), link_above, "{name}: link above");
        if link_above {
            let left = entering
                .iter()
                .map(|point| point.x)
                .fold(f32::MAX, f32::min);
            assert!(
                rect.max.x < left,
                "{name}: a lone enemy marker keeps the left side of the link"
            );
        } else {
            assert!((token.at.x - card.x).abs() < 0.001, "{name}: centered");
        }
        for triangle in &triangles {
            let bounds = Rect::from_corners(
                triangle[0].min(triangle[1]).min(triangle[2]),
                triangle[0].max(triangle[1]).max(triangle[2]),
            );
            assert!(
                bounds.intersect(rect).is_empty(),
                "{name}: no brass link under the token"
            );
        }
        for standee in &standees {
            assert!(
                Rect::from_center_half_size(*standee, Vec2::splat(standee_radius))
                    .intersect(rect)
                    .is_empty(),
                "{name}: standee slots stay clear of the marker"
            );
        }
        let pill = texts(&mut app)
            .into_iter()
            .find(|(_, _, label, _)| label.starts_with(&format!("{name} enemies")))
            .unwrap();
        assert!(
            pill.3.z >= rect.max.y - 0.001 && pill.3.z < top,
            "{name}: the count hangs beneath the token inside the band"
        );
    }
}

#[test]
fn enemy_rows_start_just_below_their_heading_and_clear_damage_tokens() {
    for size in [Vec2::new(1600., 1000.), Vec2::new(1280., 800.)] {
        let mut app = table_with_groups(&[(HALLWAY, 3), (PARLOR, 1)]);
        crate::tabletop_layout::heading_checks::viewport::<MainCamera>(&mut app, size, 1.);
        app.world_mut()
            .resource_mut::<Orbit>()
            .focus(enemy_board::focus(Area::Scenario));
        for _ in 0..4 {
            app.update();
        }
        let (_, board) = tokens(&mut app, "Enemy presence Hallway");
        let heading = &board[0];
        let world = app.world_mut();
        let first: Vec<_> = world
            .query::<(Entity, &motion::PhysicalCard, &Transform)>()
            .iter(world)
            .filter(|(_, card, _)| (62000..62003).contains(&card.0))
            .map(|(entity, _, at)| (entity, at.translation))
            .collect();
        assert_eq!(first.len(), 3);
        let attached: Vec<_> = world
            .query::<(&card_counters::Attached, &Transform)>()
            .iter(world)
            .map(|(attached, at)| (attached.0, attached.1, at.translation))
            .collect();
        let damage: Vec<_> = first
            .iter()
            .map(|(card, _)| {
                let above: Vec<_> = attached
                    .iter()
                    .filter(|(owner, offset, _)| owner == card && offset.z < 0.)
                    .map(|(_, _, at)| *at)
                    .collect();
                assert_eq!(above.len(), 1, "each enemy shows one damage token");
                above[0]
            })
            .collect();
        let token_radius = 0.248;
        for ((_, card), damage) in first.iter().zip(&damage) {
            let card_top = card.z - pieces::CARD_LENGTH / 2.;
            let heading_bottom = heading.at.z + heading.radius;
            assert!(
                damage.z - token_radius > heading_bottom + 0.05,
                "the damage token clears the heading token"
            );
            assert!(damage.z < card_top, "the damage token sits above the card");
            assert!(
                card_top - heading_bottom < 2. * token_radius + 0.5,
                "the card starts just below its heading"
            );
        }
        // At the normal Scenario focus the heading's text clears the damage tokens too.
        let world = app.world_mut();
        let (camera, transform) = world
            .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>()
            .single(world)
            .map(|(camera, transform)| (camera.clone(), *transform))
            .unwrap();
        let (camera, transform) = (&camera, &transform);
        let label = world
            .query::<(&AccessibleLabel, &Node, &ComputedNode)>()
            .iter(world)
            .find(|(label, ..)| label.0 == "HALLWAY · 3")
            .map(|(_, node, computed)| {
                (
                    node.left,
                    node.top,
                    computed.size() * computed.inverse_scale_factor,
                )
            })
            .unwrap();
        let (Val::Px(left), Val::Px(top), size) = label else {
            panic!("the heading is positioned on screen");
        };
        assert!(size.y > 0., "the heading text has a measured height");
        let text = Rect::from_corners(Vec2::new(left, top), Vec2::new(left, top) + size);
        for damage in &damage {
            let corners = [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z].map(|edge| {
                camera
                    .world_to_viewport(transform, damage + edge * token_radius)
                    .unwrap()
            });
            let disk = Rect::from_corners(
                corners.iter().copied().fold(Vec2::MAX, Vec2::min),
                corners.iter().copied().fold(Vec2::MIN, Vec2::max),
            );
            assert!(
                text.intersect(disk).is_empty(),
                "{size:?}: heading text {text:?} covers a damage token {disk:?}"
            );
        }
    }
}
