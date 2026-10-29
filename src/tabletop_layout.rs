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

/// Unmodified WASD pans in the camera's horizontal plane at a zoom-scaled rate.
/// Callers own input capture; this function also rejects modified shortcuts.
pub(crate) fn pan_delta(keys: &ButtonInput<KeyCode>, yaw: f32, dt: f32, distance: f32) -> Vec3 {
    if keys.any_pressed([
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]) {
        return Vec3::ZERO;
    }
    let horizontal =
        f32::from(keys.pressed(KeyCode::KeyD)) - f32::from(keys.pressed(KeyCode::KeyA));
    let forward = f32::from(keys.pressed(KeyCode::KeyW)) - f32::from(keys.pressed(KeyCode::KeyS));
    let direction = Vec2::new(horizontal, forward).normalize_or_zero();
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    let ahead = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    (right * direction.x + ahead * direction.y) * dt * distance * 0.55
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

    #[test]
    fn pan_is_camera_relative_normalized_and_stops_on_release() {
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyW);
        let forward = pan_delta(&keys, 0.0, 0.1, 16.0);
        assert!(forward.z < 0.0 && forward.y == 0.0);
        let turned = pan_delta(&keys, std::f32::consts::FRAC_PI_2, 0.1, 16.0);
        assert!(turned.x < 0.0 && turned.z.abs() < 0.00001);
        keys.press(KeyCode::KeyD);
        assert!((forward.length() - pan_delta(&keys, 0.0, 0.1, 16.0).length()).abs() < 0.00001);
        keys.press(KeyCode::ControlLeft);
        assert_eq!(pan_delta(&keys, 0.0, 0.1, 16.0), Vec3::ZERO);
        keys.reset_all();
        assert_eq!(pan_delta(&keys, 0.0, 0.1, 16.0), Vec3::ZERO);
    }
}
