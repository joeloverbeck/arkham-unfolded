use super::*;
use bevy::mesh::VertexAttributeValues;

fn location(code: &str, revealed: bool) -> LocationView {
    LocationView {
        name: "Display name is not topology".into(),
        code: code.into(),
        revealed,
        clues: 0,
    }
}

fn map(revealed: bool) -> GatheringMap {
    GatheringMap::new(
        &[
            location("01112", true),
            location("01113", revealed),
            location("01114", revealed),
            location("01115", revealed),
        ],
        Vec2::new(super::super::CARD_WIDTH, super::super::CARD_LENGTH),
    )
    .unwrap()
}

fn connection() -> Connection {
    Connection {
        from: Room::Hallway,
        to: Room::Attic,
        forward: true,
        backward: true,
    }
}

fn vertices(mesh: &Mesh) -> &[[f32; 3]] {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("link must expose positions")
    };
    positions
}

// Independent separating-axis oracle for filled triangles, including their edges.
fn triangle_overlaps_rect(triangle: &[Vec2], rectangle: Rect) -> bool {
    let corners = [
        rectangle.min,
        Vec2::new(rectangle.min.x, rectangle.max.y),
        rectangle.max,
        Vec2::new(rectangle.max.x, rectangle.min.y),
    ];
    let axes = [
        Vec2::X,
        Vec2::Y,
        (triangle[1] - triangle[0]).perp(),
        (triangle[2] - triangle[1]).perp(),
        (triangle[0] - triangle[2]).perp(),
    ];
    axes.into_iter()
        .filter(|axis| axis.length_squared() > 0.000001)
        .all(|axis| {
            let (a_min, a_max) = triangle
                .iter()
                .map(|point| point.dot(axis))
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                    (min.min(value), max.max(value))
                });
            let (b_min, b_max) = corners
                .iter()
                .map(|point| point.dot(axis))
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                    (min.min(value), max.max(value))
                });
            a_max >= b_min && b_max >= a_min
        })
}

fn assert_mesh_clears(points: &[Vec2], obstacles: &[Rect]) {
    let mesh = link_mesh(points, connection(), LINK_HALF_WIDTH, LINK_HEIGHT);
    let positions = vertices(&mesh);
    assert!(positions.iter().flatten().all(|value| value.is_finite()));
    for triangle in positions.chunks_exact(3) {
        let flat = triangle
            .iter()
            .map(|point| Vec2::new(point[0], point[2]))
            .collect::<Vec<_>>();
        assert!(
            obstacles
                .iter()
                .all(|&obstacle| !triangle_overlaps_rect(&flat, obstacle)),
            "link triangle {flat:?} intersects a reserved footprint"
        );
    }
}

#[test]
fn inspected_original_faces_produce_only_the_three_reciprocal_hallway_pairs() {
    for revealed in [false, true] {
        let graph = map(revealed);
        assert_eq!(
            graph
                .links
                .iter()
                .map(|(link, _)| *link)
                .collect::<Vec<_>>(),
            [
                Connection {
                    from: Room::Hallway,
                    to: Room::Attic,
                    forward: true,
                    backward: true
                },
                Connection {
                    from: Room::Hallway,
                    to: Room::Cellar,
                    forward: true,
                    backward: true
                },
                Connection {
                    from: Room::Hallway,
                    to: Room::Parlor,
                    forward: true,
                    backward: true
                },
            ]
        );
    }
    let study = GatheringMap::new(&[location("01111", true)], Vec2::new(1.35, 1.89)).unwrap();
    assert!(study.links.is_empty());
    let partial = GatheringMap::new(
        &[location("01112", true), location("01113", false)],
        Vec2::new(1.35, 1.89),
    )
    .unwrap();
    assert_eq!(partial.links.len(), 1);
    assert_eq!(partial.links[0].0.to, Room::Attic);
    assert!(GatheringMap::entry_blocked(&location("01115", false)));
    assert!(!GatheringMap::entry_blocked(&location("01115", true)));
    assert!(!GatheringMap::entry_blocked(&location("01113", false)));
}

#[test]
fn relative_layout_reserves_every_standee_and_captions_within_the_real_scenario_area() {
    let graph = map(false);
    let center = graph.position("01112");
    assert!(graph.position("01113").z < center.z);
    assert!(graph.position("01114").z > center.z);
    assert!(graph.position("01115").x > center.x);
    let card_size = Vec2::new(super::super::CARD_WIDTH, super::super::CARD_LENGTH);
    let radius = crate::tabletop_standees::radius_for(card_size);
    for placement in &graph.placements {
        let bounds = placement.footprint.bounds;
        assert!(bounds.min.x > -9. && bounds.max.x < 9.);
        assert!(bounds.min.y > -16.5 && bounds.max.y < 1.);
        for slot in 0..4 {
            let standee = layout::location_standee_position(placement.at, card_size, slot)
                .unwrap()
                .xz();
            assert!(bounds.contains(standee - Vec2::splat(radius)));
            assert!(bounds.contains(standee + Vec2::splat(radius)));
        }
    }
    let obstacles = graph
        .placements
        .iter()
        .map(|p| p.footprint.bounds)
        .collect::<Vec<_>>();
    for (_, points) in &graph.links {
        assert_mesh_clears(points, &obstacles);
    }
}

#[test]
fn visible_input_order_and_occupancy_independent_fields_do_not_relayout_the_map() {
    let first = map(false);
    let second = GatheringMap::new(
        &[
            location("01115", true),
            location("01114", false),
            location("01113", true),
            location("01112", false),
        ],
        Vec2::new(super::super::CARD_WIDTH, super::super::CARD_LENGTH),
    )
    .unwrap();
    for code in ["01112", "01113", "01114", "01115"] {
        assert_eq!(first.position(code), second.position(code));
    }
    assert_eq!(first.links, second.links);
}

#[test]
fn detours_clear_rotated_unequal_footprints_and_an_intervening_obstacle() {
    let from = Footprint::location(
        Transform::from_xyz(-6., 0., -3.).with_rotation(Quat::from_rotation_y(0.35)),
        Vec2::new(1.2, 1.7),
    );
    let to = Footprint::location(
        Transform::from_xyz(5., 0., 1.).with_rotation(Quat::from_rotation_y(-0.23)),
        Vec2::new(1.7, 2.2),
    );
    let obstacles = [
        from.bounds,
        to.bounds,
        Rect::from_corners(Vec2::new(-1., -3.), Vec2::new(1., 3.)),
    ];
    let points = route(from, to, &obstacles).unwrap();
    assert!(points.len() > 3, "the obstructed route has rounded bends");
    assert_mesh_clears(&points, &obstacles);
}

#[test]
fn arrowheads_clear_an_obstacle_that_misses_the_shaft() {
    let from = Footprint {
        center: Vec2::ZERO,
        bounds: Rect::from_corners(Vec2::splat(-1.), Vec2::splat(1.)),
    };
    let to = Footprint {
        center: Vec2::new(6., 0.),
        bounds: Rect::from_corners(Vec2::new(5., -1.), Vec2::new(7., 1.)),
    };
    let obstacles = [
        from.bounds,
        to.bounds,
        Rect::from_corners(Vec2::new(1.4, 0.15), Vec2::new(2., 0.5)),
    ];
    let points = route(from, to, &obstacles).unwrap();
    assert_mesh_clears(&points, &obstacles);
}

#[test]
fn coincident_or_enclosed_locations_have_no_fake_route() {
    let small = Footprint {
        center: Vec2::ZERO,
        bounds: Rect::from_corners(Vec2::splat(-1.), Vec2::splat(1.)),
    };
    assert!(route(small, small, &[small.bounds]).is_none());
    let enclosing = Footprint {
        center: Vec2::new(0.1, 0.1),
        bounds: Rect::from_corners(Vec2::splat(-4.), Vec2::splat(4.)),
    };
    assert!(route(small, enclosing, &[small.bounds, enclosing.bounds]).is_none());
}

#[test]
fn short_endpoint_segments_keep_arrow_bases_inside_their_own_segment() {
    let points = [Vec2::ZERO, Vec2::new(0.1, 0.), Vec2::new(0.1, 5.)];
    let mesh = link_mesh(&points, connection(), LINK_HALF_WIDTH, LINK_HEIGHT);
    let start_head = &vertices(&mesh)[..3];
    assert!(
        start_head.iter().all(|point| point[0] <= 0.1),
        "start head must not extend beyond its short endpoint segment"
    );
}

#[test]
fn one_way_and_reciprocal_links_have_one_and_two_arrowheads() {
    let points = [Vec2::ZERO, Vec2::new(2., 0.)];
    let mut direction = connection();
    direction.backward = false;
    let one = link_mesh(&points, direction, LINK_HALF_WIDTH, LINK_HEIGHT);
    let two = link_mesh(&points, connection(), LINK_HALF_WIDTH, LINK_HEIGHT);
    assert_eq!(vertices(&one).len(), 9);
    assert_eq!(vertices(&two).len(), 12);
    assert_eq!(&vertices(&one)[0], &[2., LINK_HEIGHT, 0.]);
    assert_eq!(&vertices(&two)[0], &[0., LINK_HEIGHT, 0.]);
    assert_eq!(&vertices(&two)[3], &[2., LINK_HEIGHT, 0.]);
}
