//! Spatial conventions shared by the prepared tabletop experiments.
//! These are presentation coordinates, never game zones or saved state.

use bevy::prelude::*;

#[cfg(test)]
#[path = "tabletop_layout/heading_checks.rs"]
pub(crate) mod heading_checks;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Area {
    Overview,
    Scenario,
    Test,
    Investigator(usize),
}

pub(crate) const SCENARIO_CENTER: Vec3 = Vec3::new(0.0, 0.0, -10.0);
pub(crate) const SCENARIO_SIZE: Vec2 = Vec2::new(18.0, 22.0);
pub(crate) const SCENARIO_FAR_EDGE: f32 = SCENARIO_CENTER.z - SCENARIO_SIZE.y / 2.0;
pub(crate) const SCENARIO_MAP_CENTER: Vec3 = Vec3::new(0.0, 0.05, -8.5);
pub(crate) const SCENARIO_REFERENCE: Vec3 = Vec3::new(-6.0, 0.05, SCENARIO_FAR_EDGE + 5.5);
pub(crate) const ENCOUNTER_DECK: Vec3 = Vec3::new(-6.0, 0.05, SCENARIO_FAR_EDGE + 2.5);
pub(crate) const ENCOUNTER_DISCARD: Vec3 = Vec3::new(6.0, 0.125, SCENARIO_FAR_EDGE + 2.5);
pub(crate) const VICTORY_DISPLAY: Vec3 = Vec3::new(6.0, 0.05, SCENARIO_FAR_EDGE + 5.5);
pub(crate) const TEST_CENTER: Vec3 = Vec3::new(0.0, 0.0, 8.0);
/// Area focus views retain component captions; Overview suppresses this detail.
pub(crate) const DETAIL_LABEL_DISTANCE: f32 = 40.0;
const TEST_SIZE: Vec2 = Vec2::new(14.0, 10.0);
const INVESTIGATOR_SIZE: Vec2 = Vec2::new(18.0, 15.0);
const INVESTIGATOR_SPACING: f32 = 18.6;
const THREAT_REGION_SIZE: Vec2 = Vec2::new(9.9, 3.4);
const ASSET_REGION_SIZE: Vec2 = Vec2::new(17.3, 6.08);
const HAND_REGION_SIZE: Vec2 = Vec2::new(ASSET_REGION_SIZE.x, 3.4);
const UPPER_GROUP_SHIFT: f32 = 0.325;
const THREAT_REGION_Z: f32 = -5.5;
const ASSET_REGION_Z: f32 = 0.16;
const HAND_REGION_Z: f32 = 5.45;

/// Positions beside a location, ordered far-left, near-left, far-right, near-right.
/// The complete rounded bases clear the card; the caller supplies real occupants.
pub(crate) fn location_standee_position(
    center: Vec3,
    card_size: Vec2,
    slot: usize,
) -> Option<Vec3> {
    let (side, row) = match slot {
        0 => (-1.0, -1.0),
        1 => (-1.0, 1.0),
        2 => (1.0, -1.0),
        3 => (1.0, 1.0),
        _ => return None,
    };
    let radius = crate::tabletop_standees::radius_for(card_size);
    let offset = card_size.x / 2.0 + radius + 0.10;
    // A narrow gap follows the card scale while retaining the lateral clearance.
    let pair_offset = radius + card_size.y * 0.025;
    Some(center + Vec3::new(side * offset, 0.0, row * pair_offset))
}

#[derive(Clone, Copy)]
pub(crate) enum ScenarioBookSide {
    Agenda,
    Act,
}

/// Existing sideways artwork becomes two landscape pages on one backward-leaning plane.
pub(crate) fn scenario_book_card_transform(
    side: ScenarioBookSide,
    portrait_size: Vec2,
) -> Transform {
    let tilt = 75.0_f32.to_radians();
    let side = match side {
        ScenarioBookSide::Agenda => -1.0,
        ScenarioBookSide::Act => 1.0,
    };
    Transform::from_xyz(
        side * portrait_size.y / 2.0,
        0.05 + portrait_size.x / 2.0 * tilt.sin(),
        SCENARIO_FAR_EDGE + 2.5,
    )
    .with_rotation(Quat::from_rotation_x(tilt) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
}

pub(crate) fn investigator_center(index: usize, count: usize) -> Vec3 {
    Vec3::new(
        (index as f32 - count.saturating_sub(1) as f32 / 2.0) * INVESTIGATOR_SPACING,
        0.0,
        22.5,
    )
}

fn personal(index: usize, x: f32, z: f32) -> Vec3 {
    investigator_center(index, 3) + Vec3::new(x, 0.05, z)
}

pub(crate) fn investigator_heading(index: usize) -> Vec3 {
    personal(index, 0.0, -8.15)
}

pub(crate) fn scenario_heading() -> Vec3 {
    Vec3::new(0.0, 0.1, SCENARIO_FAR_EDGE - 0.65)
}

pub(crate) fn test_heading() -> Vec3 {
    Vec3::new(0.0, 0.0, 2.0)
}

#[derive(Component)]
pub(crate) struct TestAreaHeading;

/// Center this screen-space heading inside its projected gap, including UI padding.
/// At an extreme angle or zoom there may be no readable space for a heading.
pub(crate) fn test_heading_screen_position(
    camera: &Camera,
    transform: &GlobalTransform,
    node: &ComputedNode,
) -> Option<Vec2> {
    let height = node.size().y * node.inverse_scale_factor;
    let scenario_edge = camera
        .world_to_viewport(
            transform,
            Vec3::new(0.0, 0.0, SCENARIO_CENTER.z + SCENARIO_SIZE.y / 2.0),
        )
        .ok()?;
    let test_edge = camera
        .world_to_viewport(
            transform,
            Vec3::new(0.0, 0.0, TEST_CENTER.z - TEST_SIZE.y / 2.0),
        )
        .ok()?;
    if height <= 0.0 || (test_edge.y - scenario_edge.y).abs() < height + 2.0 {
        return None;
    }
    Some((scenario_edge + test_edge) / 2.0 - Vec2::Y * height / 2.0)
}

pub(crate) fn investigator_card(index: usize) -> Vec3 {
    personal(index, -7.15, -5.37)
}

pub(crate) fn investigator_status(index: usize) -> Vec3 {
    personal(index, -7.15, -3.75)
}

/// Stable homes remain fixed when either personal total becomes zero.
pub(crate) fn investigator_resource(index: usize) -> Vec3 {
    personal(index, -7.95, -4.35).with_y(0.14)
}

pub(crate) fn investigator_clue(index: usize) -> Vec3 {
    personal(index, -6.35, -4.35).with_y(0.14)
}

pub(crate) fn location_clue(card_at: Vec3) -> Vec3 {
    card_at + Vec3::Y * 0.09
}

/// Center the complete row; a gap distinguishes a fourth restricted allowance.
pub(crate) fn investigator_action(index: usize, slot: usize, additional: bool) -> Vec3 {
    let gap = if additional { 0.15 } else { 0.0 };
    let span = if additional {
        3.0 * 0.48 + gap
    } else {
        2.0 * 0.48
    };
    personal(
        index,
        -7.15 - span / 2.0 + slot as f32 * 0.48 + if slot == 3 { gap } else { 0.0 },
        -3.25,
    )
    .with_y(0.14)
}

/// Center of a permanent counter; its caption sits between this row and the card.
pub(crate) fn investigator_harm(index: usize, horror: bool) -> Vec3 {
    investigator_center(index, 3) + Vec3::new(-7.15 + if horror { 0.75 } else { -0.75 }, 0.14, -7.0)
}

/// Named substitutes need wider captions, without moving the physical counters.
pub(crate) fn investigator_harm_caption(index: usize, horror: bool, named: bool) -> Vec3 {
    let outward = if named {
        if horror { 0.45 } else { -0.45 }
    } else {
        0.0
    };
    investigator_harm(index, horror) + Vec3::new(outward, 0.0, 0.4)
}

pub(crate) fn investigator_deck(index: usize) -> Vec3 {
    personal(index, 6.0 - UPPER_GROUP_SHIFT, -5.5)
}

pub(crate) fn investigator_discard(index: usize) -> Vec3 {
    personal(index, 8.0 - UPPER_GROUP_SHIFT, -5.5)
}

pub(crate) fn investigator_threat(index: usize, slot: usize) -> Vec3 {
    personal(index, -3.9 - UPPER_GROUP_SHIFT + slot as f32 * 1.95, -5.4)
}

pub(crate) fn investigator_asset(index: usize, slot: usize) -> Vec3 {
    personal(
        index,
        -6.5 + (slot % 6) as f32 * 2.6,
        -1.5 + (slot / 6) as f32 * 3.0,
    )
}

/// A centered horizontal hand retains full-sized faces; navigation handles overflow.
pub(crate) fn hand_position(index: usize, count: usize, slot: usize) -> Vec3 {
    personal(
        index,
        slot as f32 * 1.93 - count.saturating_sub(1) as f32 * 0.965,
        5.5,
    )
}

pub(crate) fn test_bag() -> Vec3 {
    TEST_CENTER + Vec3::new(5.3, 0.35, -3.1)
}

pub(crate) fn test_source() -> Vec3 {
    TEST_CENTER + Vec3::new(-4.9, 0.05, -3.2)
}

pub(crate) fn test_commitment(count: usize, slot: usize) -> Vec3 {
    let row_count = count.saturating_sub(slot / 6 * 6).min(6);
    TEST_CENTER
        + Vec3::new(
            (slot % 6) as f32 * 2.15 - row_count.saturating_sub(1) as f32 * 1.075,
            0.05,
            (slot / 6) as f32 * 3.3,
        )
}

pub(crate) fn test_token(canceled: bool, slot: usize) -> Vec3 {
    TEST_CENTER
        + Vec3::new(
            if canceled { 1.2 } else { -5.4 } + (slot % 4) as f32 * 1.3,
            0.15,
            3.0 + (slot / 4) as f32 * 1.3,
        )
}

pub(crate) fn zone_labels(index: usize) -> [(&'static str, Vec3); 5] {
    [
        ("Threats", personal(index, -UPPER_GROUP_SHIFT, -7.15)),
        ("Assets", personal(index, 0.0, -3.03)),
        ("Hand", personal(index, 0.0, 7.15)),
        ("Deck", investigator_deck(index) - Vec3::Z * 1.7),
        ("Discard", investigator_discard(index) - Vec3::Z * 1.7),
    ]
}

pub(crate) fn test_zone_labels() -> [(&'static str, Vec3); 3] {
    [
        ("Committed cards", TEST_CENTER + Vec3::new(0.0, 0.05, -1.8)),
        ("Revealed", TEST_CENTER + Vec3::new(-3.4, 0.05, 1.8)),
        ("Canceled", TEST_CENTER + Vec3::new(3.3, 0.05, 1.8)),
    ]
}

/// Presentation identity, independent of secondary deckbuilding access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InvestigatorClass {
    Guardian,
    Seeker,
    Rogue,
    Mystic,
    Survivor,
    Neutral,
}

impl InvestigatorClass {
    pub(crate) fn asset_name(self) -> &'static str {
        match self {
            Self::Guardian => "guardian",
            Self::Seeker => "seeker",
            Self::Rogue => "rogue",
            Self::Mystic => "mystic",
            Self::Survivor => "survivor",
            Self::Neutral => "neutral",
        }
    }
    /// Muted class colors shared by table borders, portrait accents and standee holders.
    pub(crate) fn color(self) -> Color {
        match self {
            Self::Guardian => Color::srgb(0.16, 0.29, 0.47),
            Self::Seeker => Color::srgb(0.56, 0.39, 0.12),
            Self::Rogue => Color::srgb(0.12, 0.32, 0.22),
            Self::Mystic => Color::srgb(0.27, 0.19, 0.40),
            Self::Survivor => Color::srgb(0.37, 0.14, 0.19),
            Self::Neutral => Color::srgb(0.65, 0.65, 0.62),
        }
    }
}

pub(crate) fn investigator_class(index: usize) -> InvestigatorClass {
    match index {
        0 => InvestigatorClass::Rogue,    // Tony.
        1 => InvestigatorClass::Survivor, // Patrice.
        2 => InvestigatorClass::Mystic,   // Jacqueline.
        _ => InvestigatorClass::Neutral,
    }
}

pub(crate) fn investigator_color(index: usize) -> Color {
    investigator_class(index).color()
}

/// Target and orbit distance at the normal playing angle. Focus never chooses an actor.
pub(crate) fn focus(area: Area) -> (Vec3, f32) {
    match area {
        Area::Overview => (Vec3::new(0.0, 0.0, 7.0), 72.0),
        // The deeper surface occupies the scene above the fixed decision strip.
        // Aim nearer its front edge to leave space for the far-edge heading too.
        Area::Scenario => (SCENARIO_CENTER + Vec3::Z * 4.5, DETAIL_LABEL_DISTANCE),
        Area::Test => (TEST_CENTER + Vec3::Z * 0.5, 20.0),
        // Aim toward the near edge so the physical hand clears the decision strip.
        Area::Investigator(index) => (investigator_center(index, 3) + Vec3::Z * 0.7, 26.0),
    }
}

pub(crate) fn spawn_surfaces(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    surface(
        commands,
        meshes,
        materials,
        SCENARIO_CENTER,
        SCENARIO_SIZE,
        Color::srgb(0.11, 0.22, 0.24),
    );
    surface(
        commands,
        meshes,
        materials,
        TEST_CENTER,
        TEST_SIZE,
        Color::srgb(0.28, 0.23, 0.17),
    );
    for (center, size) in [
        (TEST_CENTER, Vec2::new(13.4, 3.2)),
        (
            TEST_CENTER + Vec3::new(-3.3, 0.0, 3.15),
            Vec2::new(6.2, 3.1),
        ),
        (TEST_CENTER + Vec3::new(3.3, 0.0, 3.15), Vec2::new(6.2, 3.1)),
    ] {
        region(
            commands,
            meshes,
            materials,
            center,
            size,
            Color::srgb(0.34, 0.29, 0.23),
        );
    }
    for index in 0..3 {
        surface(
            commands,
            meshes,
            materials,
            investigator_center(index, 3),
            INVESTIGATOR_SIZE,
            investigator_color(index),
        );
        let color = investigator_color(index).mix(&Color::WHITE, 0.12);
        for (center, size) in [
            (
                personal(index, -UPPER_GROUP_SHIFT, THREAT_REGION_Z),
                THREAT_REGION_SIZE,
            ),
            (personal(index, 0.0, ASSET_REGION_Z), ASSET_REGION_SIZE),
            (personal(index, 0.0, HAND_REGION_Z), HAND_REGION_SIZE),
            (investigator_deck(index), Vec2::new(1.95, 3.4)),
            (investigator_discard(index), Vec2::new(1.95, 3.4)),
        ] {
            region(commands, meshes, materials, center, size, color);
        }
    }
}

fn region(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    size: Vec2,
    color: Color,
) {
    let fill = materials.add(StandardMaterial {
        base_color: color.mix(&Color::BLACK, 0.67),
        perceptual_roughness: 0.95,
        ..default()
    });
    let border = materials.add(StandardMaterial {
        base_color: color.mix(&Color::BLACK, 0.35),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, 0.012, size.y))),
        MeshMaterial3d(fill),
        Transform::from_xyz(center.x, -0.048, center.z),
        Pickable::IGNORE,
    ));
    for (offset, dimensions) in [
        (
            Vec3::new(0.0, 0.0, -size.y / 2.0),
            Vec3::new(size.x, 0.015, 0.025),
        ),
        (
            Vec3::new(0.0, 0.0, size.y / 2.0),
            Vec3::new(size.x, 0.015, 0.025),
        ),
        (
            Vec3::new(-size.x / 2.0, 0.0, 0.0),
            Vec3::new(0.025, 0.015, size.y),
        ),
        (
            Vec3::new(size.x / 2.0, 0.0, 0.0),
            Vec3::new(0.025, 0.015, size.y),
        ),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(dimensions))),
            MeshMaterial3d(border.clone()),
            Transform::from_translation(Vec3::new(center.x, -0.037, center.z) + offset),
            Pickable::IGNORE,
        ));
    }
}

fn surface(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    size: Vec2,
    color: Color,
) {
    for (width, depth, y, height, tint) in [
        (size.x, size.y, -0.24, 0.30, color),
        (
            size.x - 0.16,
            size.y - 0.16,
            -0.075,
            0.03,
            color.mix(&Color::BLACK, 0.48),
        ),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(width, height, depth))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: tint,
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_translation(center + Vec3::Y * y),
            Pickable::IGNORE,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_focus_clears_navigation_and_decision_controls() {
        use bevy::camera::{CameraProjection, RenderTargetInfo};

        for size in [
            Vec2::new(1280.0, 720.0),
            Vec2::new(1280.0, 800.0),
            Vec2::new(1600.0, 1000.0),
            Vec2::new(1920.0, 1080.0),
        ] {
            let mut projection = PerspectiveProjection::default();
            projection.update(size.x, size.y);
            let mut camera = Camera::default();
            camera.computed.clip_from_view = projection.get_clip_from_view();
            camera.computed.target_info = Some(RenderTargetInfo {
                physical_size: size.as_uvec2(),
                scale_factor: 1.0,
            });
            let (target, distance) = focus(Area::Scenario);
            let transform = GlobalTransform::from(
                Transform::from_translation(
                    target + Vec3::new(0.0, 1.0_f32.sin(), 1.0_f32.cos()) * distance,
                )
                .looking_at(target, Vec3::Y),
            );
            // The accepted 18-by-22 area, its far-edge heading, and the raised book.
            let points = [
                Vec3::new(-9.0, 0.0, -21.0),
                Vec3::new(9.0, 0.0, -21.0),
                Vec3::new(-9.0, 0.0, 1.0),
                Vec3::new(9.0, 0.0, 1.0),
                Vec3::new(0.0, 0.1, -21.65),
                Vec3::new(-1.89, 1.4, -18.5),
                Vec3::new(1.89, 1.4, -18.5),
            ];
            let decision_top = size.y - (size.y * 0.29).min(270.0) - 10.0;
            for point in points {
                let screen = camera.world_to_viewport(&transform, point).unwrap();
                assert!(
                    screen.x >= 150.0
                        && screen.x <= size.x - 12.0
                        && screen.y >= 104.0
                        && screen.y <= decision_top - 8.0,
                    "Scenario point {point:?} projects to {screen:?} at {size:?}, under fixed controls",
                );
            }
        }
    }

    #[test]
    fn shared_surfaces_leave_matching_gaps_and_room_for_the_test_heading() {
        let scenario_near = SCENARIO_CENTER.z + SCENARIO_SIZE.y / 2.0;
        let test_far = TEST_CENTER.z - TEST_SIZE.y / 2.0;
        let test_near = TEST_CENTER.z + TEST_SIZE.y / 2.0;
        let investigator_far = investigator_center(1, 3).z - INVESTIGATOR_SIZE.y / 2.0;
        assert_eq!(test_far - scenario_near, 2.0);
        assert_eq!(investigator_far - test_near, 2.0);
        assert!(test_heading().z > scenario_near + 0.5);
        assert!(test_heading().z < test_far - 0.5);
        assert_eq!(SCENARIO_SIZE, Vec2::new(18.0, 22.0));
        assert_eq!(TEST_CENTER, Vec3::new(0.0, 0.0, 8.0));
        assert_eq!(investigator_center(1, 3), Vec3::new(0.0, 0.0, 22.5));
    }

    #[test]
    fn test_heading_centers_its_full_pixel_height_between_projected_edges() {
        use bevy::camera::{CameraProjection, RenderTargetInfo};

        for scale in [1.0, 2.0] {
            let mut projection = PerspectiveProjection::default();
            projection.update(1600.0, 1000.0);
            let mut camera = Camera::default();
            camera.computed.clip_from_view = projection.get_clip_from_view();
            camera.computed.target_info = Some(RenderTargetInfo {
                physical_size: UVec2::new((1600.0 * scale) as u32, (1000.0 * scale) as u32),
                scale_factor: scale,
            });
            for area in [Area::Overview, Area::Scenario, Area::Test] {
                let (target, distance) = focus(area);
                let transform = GlobalTransform::from(
                    Transform::from_translation(
                        target + Vec3::new(0.0, 1.0_f32.sin(), 1.0_f32.cos()) * distance,
                    )
                    .looking_at(target, Vec3::Y),
                );
                // A full single-line 18px heading, or 13/14px text with reduced padding.
                let node = ComputedNode {
                    size: Vec2::new(190.0, 22.0) * scale,
                    inverse_scale_factor: 1.0 / scale,
                    ..default()
                };
                let heading = test_heading_screen_position(&camera, &transform, &node)
                    .expect("normal views must retain the heading");
                let far = camera
                    .world_to_viewport(&transform, Vec3::new(0.0, 0.0, 1.0))
                    .unwrap();
                let near = camera
                    .world_to_viewport(&transform, Vec3::new(0.0, 0.0, 3.0))
                    .unwrap();
                assert!(heading.y >= far.y.min(near.y) + 1.0);
                assert!(heading.y + 22.0 <= far.y.max(near.y) - 1.0);
                let crowded = ComputedNode {
                    size: Vec2::new(190.0, 1000.0) * scale,
                    ..node
                };
                assert!(test_heading_screen_position(&camera, &transform, &crowded).is_none());
            }
        }
    }

    #[test]
    fn four_location_standees_clear_cards_and_each_other() {
        for size in [Vec2::new(1.35, 1.89), Vec2::new(1.80, 2.52)] {
            let radius = crate::tabletop_standees::radius_for(size);
            let center = Vec3::new(-2.5, 0.0, -3.8);
            for count in 1..=4 {
                let points: Vec<_> = (0..count)
                    .map(|slot| location_standee_position(center, size, slot).unwrap())
                    .collect();
                for (index, point) in points.iter().enumerate() {
                    assert!(
                        ((point.x - center.x).abs() - radius - size.x / 2.0 - 0.10).abs() < 0.00001
                    );
                    assert!((point.z - center.z).abs() + radius < size.y / 2.0);
                    assert_eq!(point.y, center.y);
                    assert_eq!(point.x < center.x, index < 2);
                    for other in &points[..index] {
                        assert!(point.distance(*other) > radius * 2.0);
                    }
                }
                if count >= 2 {
                    assert!(points[0].z < points[1].z);
                    let gap = points[1].z - points[0].z - radius * 2.0;
                    assert!(
                        gap > 0.08 && gap < 0.14,
                        "visible, narrow same-side gap: {gap}"
                    );
                }
            }
            assert!(location_standee_position(center, size, 4).is_none());
        }
    }

    #[test]
    fn scenario_book_joins_landscape_pages_above_the_table() {
        let size = Vec2::new(1.35, 1.89);
        let agenda = scenario_book_card_transform(ScenarioBookSide::Agenda, size);
        let act = scenario_book_card_transform(ScenarioBookSide::Act, size);
        assert!(agenda.translation.x < act.translation.x);
        for x in [-size.x / 2.0, size.x / 2.0] {
            let agenda_edge = agenda.transform_point(Vec3::new(x, 0.0, size.y / 2.0));
            let act_edge = act.transform_point(Vec3::new(x, 0.0, -size.y / 2.0));
            assert!(agenda_edge.abs_diff_eq(act_edge, 0.0001));
            assert!(agenda_edge.x.abs() < 0.0001);
        }
        for page in [agenda, act] {
            let bottom = page.transform_point(Vec3::new(-size.x / 2.0, 0.0, 0.0));
            let top = page.transform_point(Vec3::new(size.x / 2.0, 0.0, 0.0));
            assert!((bottom.y - 0.05).abs() < 0.0001);
            assert!(top.y - bottom.y > size.x * 0.9);
            assert!(top.z < bottom.z, "the top leans away from the viewing side");
            assert!((page.rotation * Vec3::Y).z > 0.9);
        }
    }

    #[test]
    fn participating_areas_are_centered_without_changing_spacing() {
        for count in 1..=4 {
            let centers: Vec<_> = (0..count)
                .map(|index| investigator_center(index, count))
                .collect();
            assert!(centers.iter().map(|center| center.x).sum::<f32>().abs() < 0.00001);
            for pair in centers.windows(2) {
                assert!((pair[1].x - pair[0].x - 18.6).abs() < 0.00001);
            }
        }
    }

    #[test]
    fn centered_hands_keep_one_row_of_full_faces_separate_from_assets() {
        let center = investigator_center(1, 3);
        for count in 1..=9 {
            let points: Vec<_> = (0..count)
                .map(|slot| hand_position(1, count, slot))
                .collect();
            assert!(
                (points.iter().map(|point| point.x).sum::<f32>() / points.len() as f32 - center.x)
                    .abs()
                    < 0.00001
            );
            for pair in points.windows(2) {
                assert!(
                    pair[1].x - pair[0].x > 1.85,
                    "largest current portrait stays unobscured"
                );
                assert_eq!(pair[0].z, pair[1].z, "hands never acquire a second row");
            }
            for point in &points {
                assert!((point.x - center.x).abs() + 1.85 / 2.0 < HAND_REGION_SIZE.x / 2.0);
                assert!((point.z - center.z).abs() + 2.6 / 2.0 < INVESTIGATOR_SIZE.y / 2.0);
            }
            // The accepted twelve-face presentation challenge has two asset rows.
            let last_asset = investigator_asset(1, 11);
            assert!(points[0].z - 1.3 > last_asset.z + 1.3);
        }
    }

    #[test]
    fn five_full_size_threats_clear_each_other_and_the_personal_piles() {
        let center = investigator_center(0, 3);
        let threats: Vec<_> = (0..5).map(|slot| investigator_threat(0, slot)).collect();
        for threat in &threats {
            assert!(
                (threat.x - center.x + UPPER_GROUP_SHIFT).abs() + 1.85 / 2.0
                    < THREAT_REGION_SIZE.x / 2.0
            );
            assert!(
                (threat.z - center.z - THREAT_REGION_Z).abs() + 1.3 < THREAT_REGION_SIZE.y / 2.0
            );
        }
        for pair in threats.windows(2) {
            assert!(pair[1].x - pair[0].x - 1.85 > 0.09);
        }
        assert!(threats[0].x - 1.85 / 2.0 > investigator_card(0).x + 2.6 / 2.0);
        assert!(threats[4].x + 1.85 / 2.0 < investigator_deck(0).x - 1.85 / 2.0);
        assert!(investigator_deck(0).x + 1.85 / 2.0 < investigator_discard(0).x - 1.85 / 2.0);
        const {
            assert!(
                ASSET_REGION_Z - ASSET_REGION_SIZE.y / 2.0
                    > THREAT_REGION_Z + THREAT_REGION_SIZE.y / 2.0 + 0.3
            );
        }
    }

    #[test]
    fn permanent_harm_counters_and_captions_clear_the_investigator_card() {
        let card = investigator_card(0);
        for horror in [false, true] {
            let token = investigator_harm(0, horror);
            assert!(token.z + 0.35 < card.z - 1.35 / 2.0);
            // Caption offset plus a small world-space clearance below its anchor.
            assert!(investigator_harm_caption(0, horror, false).z + 0.3 < card.z - 1.35 / 2.0);
            assert!(token.y - 0.14 / 2.0 > 0.0);
        }
        assert_eq!(
            investigator_harm(0, true).x - investigator_harm(0, false).x,
            1.5
        );
    }

    #[test]
    fn personal_rows_and_upper_group_preserve_the_requested_alignment() {
        assert_eq!(HAND_REGION_SIZE.x, ASSET_REGION_SIZE.x);
        assert!((ASSET_REGION_Z + ASSET_REGION_SIZE.y / 2.0 - 3.2).abs() < 0.00001);
        for index in 0..3 {
            let center = investigator_center(index, 3);
            let discard_right = investigator_discard(index).x + 1.95 / 2.0;
            assert!((discard_right - center.x - ASSET_REGION_SIZE.x / 2.0).abs() < 0.00001);
            let card = investigator_card(index);
            let resource = investigator_resource(index);
            let clue = investigator_clue(index);
            assert!((resource.x + clue.x - 2.0 * card.x).abs() < 0.00001);
            assert!(resource.x < clue.x);
            assert!(resource.z - 0.327 * 0.85 > card.z + 1.35 / 2.0);
            for additional in [false, true] {
                let last = if additional { 3 } else { 2 };
                let left = investigator_action(index, 0, additional);
                let right = investigator_action(index, last, additional);
                assert!((left.x + right.x - 2.0 * card.x).abs() < 0.00001);
                assert!(left.z - 0.21 > resource.z + 0.327 * 0.85);
                assert!(
                    left.z + 0.21 + 0.1 < center.z + ASSET_REGION_Z - ASSET_REGION_SIZE.y / 2.0
                );
            }
            // Preserve the accepted twelve-face fixture, including exhausted
            // orientations, while reserving the counter band above its border.
            for slot in 0..12 {
                let at = investigator_asset(index, slot);
                for size in [Vec2::new(1.85, 2.6), Vec2::new(2.6, 1.85)] {
                    assert!((at.x - center.x).abs() + size.x / 2.0 < ASSET_REGION_SIZE.x / 2.0);
                    assert!(
                        (at.z - center.z - ASSET_REGION_Z).abs() + size.y / 2.0
                            < ASSET_REGION_SIZE.y / 2.0
                    );
                }
            }
            assert_eq!(
                investigator_harm(index, false),
                center + Vec3::new(-7.90, 0.14, -7.0)
            );
            assert_eq!(
                investigator_harm(index, true),
                center + Vec3::new(-6.40, 0.14, -7.0)
            );
        }
    }

    #[test]
    fn investigator_card_leaves_a_small_visible_gap_below_harm_captions() {
        use bevy::camera::{CameraProjection, RenderTargetInfo};

        for size in [Vec2::new(1280.0, 720.0), Vec2::new(1600.0, 1000.0)] {
            let mut projection = PerspectiveProjection::default();
            projection.update(size.x, size.y);
            let mut camera = Camera::default();
            camera.computed.clip_from_view = projection.get_clip_from_view();
            camera.computed.target_info = Some(RenderTargetInfo {
                physical_size: size.as_uvec2(),
                scale_factor: 1.0,
            });
            let (target, distance) = focus(Area::Investigator(0));
            let transform = GlobalTransform::from(
                Transform::from_translation(
                    target + Vec3::new(0.0, 1.0_f32.sin(), 1.0_f32.cos()) * distance,
                )
                .looking_at(target, Vec3::Y),
            );
            let caption = camera
                .world_to_viewport(&transform, investigator_harm_caption(0, false, false))
                .unwrap();
            // Landscape investigator depth is the ordinary card width (1.35).
            let edge = camera
                .world_to_viewport(&transform, investigator_card(0) - Vec3::Z * 0.675)
                .unwrap();
            // A single 11px caption has a 1.2-em line box. Keep a small positive gap.
            let gap = edge.y - caption.y - 13.2;
            assert!(
                (0.5..=7.0).contains(&gap),
                "Expected a small readable harm-caption gap at {size:?}, got {gap}px",
            );
        }
    }
}
