//! Spatial conventions shared by the two prepared tabletop experiments.
//! These are presentation coordinates, never game zones or saved state.

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Area {
    Overview,
    Scenario,
    Test,
    Investigator(usize),
}

pub(crate) const TEST_CENTER: Vec3 = Vec3::new(0.0, 0.0, 0.8);

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
    let offset = card_size.x / 2.0 + crate::tabletop_standees::BASE_RADIUS + 0.10;
    Some(center + Vec3::new(side * offset, 0.0, row * card_size.y / 4.0))
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
        -7.2,
    )
    .with_rotation(Quat::from_rotation_x(tilt) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
}

pub(crate) fn investigator_center(index: usize, count: usize) -> Vec3 {
    Vec3::new(
        (index as f32 - count.saturating_sub(1) as f32 / 2.0) * 12.0,
        0.0,
        10.0,
    )
}

/// Presentation identity, independent of secondary deckbuilding access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InvestigatorClass {
    #[expect(
        dead_code,
        reason = "Part of the six-class palette; not in the prepared party."
    )]
    Guardian,
    #[expect(
        dead_code,
        reason = "Part of the six-class palette; not in the prepared party."
    )]
    Seeker,
    Rogue,
    Mystic,
    Survivor,
    Neutral,
}

impl InvestigatorClass {
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
        Area::Overview => (Vec3::new(0.0, 0.0, 4.0), 44.0),
        Area::Scenario => (Vec3::new(0.0, 0.0, -3.0), 23.0),
        Area::Test => (TEST_CENTER, 12.0),
        // Aim toward the near edge so the physical hand clears the decision strip.
        Area::Investigator(index) => (investigator_center(index, 3) + Vec3::Z * 1.4, 16.0),
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
        Vec3::new(0.0, 0.0, -3.0),
        Vec2::new(18.0, 11.0),
        Color::srgb(0.11, 0.22, 0.24),
    );
    for index in 0..3 {
        surface(
            commands,
            meshes,
            materials,
            investigator_center(index, 3),
            Vec2::new(11.4, 9.0),
            investigator_color(index),
        );
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
    fn four_location_standees_clear_cards_and_each_other() {
        use crate::tabletop_standees::BASE_RADIUS;
        for size in [Vec2::new(1.35, 1.89), Vec2::new(1.80, 2.52)] {
            let center = Vec3::new(-2.5, 0.0, -3.8);
            for count in 1..=4 {
                let points: Vec<_> = (0..count)
                    .map(|slot| location_standee_position(center, size, slot).unwrap())
                    .collect();
                for (index, point) in points.iter().enumerate() {
                    assert!((point.x - center.x).abs() - BASE_RADIUS > size.x / 2.0);
                    assert!((point.z - center.z).abs() + BASE_RADIUS < size.y / 2.0);
                    assert_eq!(point.y, center.y);
                    assert_eq!(point.x < center.x, index < 2);
                    for other in &points[..index] {
                        assert!(point.distance(*other) > BASE_RADIUS * 2.0);
                    }
                }
                if count >= 2 {
                    assert!(points[0].z < points[1].z);
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
            assert!(centers.iter().map(|center| center.x).sum::<f32>().abs() < f32::EPSILON);
            for pair in centers.windows(2) {
                assert_eq!(pair[1].x - pair[0].x, 12.0);
            }
        }
    }
}
