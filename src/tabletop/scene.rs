//! Lifetime and motion of the displayed observation, independent of panel refreshes.

use super::{Art, CARDS, GOLD, MUTED, MainCamera, PAPER, Tabletop, standees};
use crate::{Observation, TokenStatus, tabletop_harm_text, tabletop_tokens};
use bevy::prelude::*;

/// Install the same scene lifecycle for the native tabletop and headless regressions.
/// The host supplies Tabletop, Art, meshes/materials, time and the main camera.
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup_reminder).add_systems(
        Update,
        (
            // Preserve world-label creation before the equal-z panel roots, so
            // observation replacement keeps labels beneath sidebar/history content.
            synchronize
                .after(super::handle_actions)
                .after(super::camera_controls)
                .before(super::rebuild_panel),
            (
                label_positions,
                tabletop_tokens::position_labels,
                animate_tokens,
                animate_reminder,
            )
                .chain()
                .after(super::rebuild_panel)
                .before(super::button_colors)
                .before(super::capture_window),
        ),
    );
}

#[derive(Component)]
struct TransientScene;
#[derive(Component)]
struct Appearing;
#[derive(Component)]
struct ReminderPose {
    angle: f32,
    target: f32,
}
#[derive(Component)]
struct WorldLabel(Vec3);

fn setup_reminder(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    art: Res<Art>,
) {
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.20, 0.06))),
            MeshMaterial3d(materials.add(Color::srgb(0.35, 0.35, 0.35))),
            Transform::from_xyz(4.55, 0.22, 1.9),
            Pickable::IGNORE,
            ReminderPose {
                angle: 0.0,
                target: 0.0,
            },
        ))
        .with_children(|parent| {
            for (y, rotation, color) in [
                (
                    0.035,
                    -std::f32::consts::FRAC_PI_2,
                    Color::srgb(0.35, 0.95, 0.65),
                ),
                (
                    -0.035,
                    std::f32::consts::FRAC_PI_2,
                    Color::srgb(0.44, 0.35, 0.47),
                ),
            ] {
                parent.spawn((
                    Mesh3d(meshes.add(Circle::new(0.19))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        base_color_texture: art.reminder.clone(),
                        alpha_mode: AlphaMode::Mask(0.5),
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, y, 0.0).with_rotation(Quat::from_rotation_x(rotation)),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn label(
    commands: &mut Commands,
    value: impl Into<String>,
    point: Vec3,
    color: Color,
    font: &Option<Handle<Font>>,
) {
    commands.spawn((
        tabletop_harm_text::ui(value, 16.0, color, font),
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(8), px(4)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.045, 0.055, 0.90)),
        Pickable::IGNORE,
        WorldLabel(point),
        TransientScene,
    ));
}

// Each dependency is supplied explicitly by Bevy; panel refresh state is not one of them.
#[allow(clippy::too_many_arguments)]
fn synchronize(
    mut commands: Commands,
    table: Res<Tabletop>,
    art: Res<Art>,
    transient: Query<Entity, With<TransientScene>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut reminder: Single<&mut ReminderPose>,
    mut rendered_view: Local<Option<Observation>>,
) {
    let view = table.view();
    // Live/replay selections may share a revision while displaying different observations.
    if rendered_view.as_ref() == Some(&view) {
        return;
    }
    for entity in &transient {
        commands.entity(entity).despawn();
    }
    reminder.target = if view.jacqueline_used {
        std::f32::consts::PI
    } else {
        0.0
    };
    label(
        &mut commands,
        format!(
            "Reaction reminder\n{}",
            if view.jacqueline_used {
                "USED THIS ROUND"
            } else {
                "READY"
            }
        ),
        Vec3::new(4.55, 0.15, 2.65),
        if view.jacqueline_used { MUTED } else { GOLD },
        &art.symbol_font,
    );
    let colors = [
        Color::srgb(0.66, 0.43, 0.20),
        Color::srgb(0.62, 0.23, 0.26),
        Color::srgb(0.37, 0.28, 0.64),
    ];
    for (i, investigator) in view.investigators.iter().enumerate() {
        let here = investigator.location == "Study";
        let (x, z) = if here {
            (-3.4 + i as f32 * 3.3, -2.3)
        } else {
            (-4.6, -0.35)
        };
        if !here {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.2, 0.035, 1.6))),
                MeshMaterial3d(materials.add(Color::srgb(0.12, 0.20, 0.22))),
                Transform::from_xyz(x, -0.02, z),
                Pickable::IGNORE,
                TransientScene,
            ));
        }
        let image = art
            .investigators
            .iter()
            .find(|(name, _)| name == &investigator.name)
            .map(|(_, image)| image);
        let standee = standees::spawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            image,
            Vec3::new(x, 0.0, z),
            colors[i % colors.len()],
        );
        commands.entity(standee).insert(TransientScene);
        label(
            &mut commands,
            format!(
                "{}\n{} · Horror {}/{}",
                investigator.name, investigator.location, investigator.horror, investigator.sanity
            ),
            Vec3::new(x, 2.05, z),
            PAPER,
            &art.symbol_font,
        );
    }
    label(
        &mut commands,
        format!(
            "Chaos bag · {}\n{} curse in bag",
            view.bag_count, view.curses_in_bag
        ),
        Vec3::new(-4.5, 1.2, 1.75),
        GOLD,
        &art.symbol_font,
    );
    for (i, card) in CARDS.iter().enumerate() {
        let status = if i == 1 && view.promise_committed {
            "Committed · +4"
        } else {
            "Click to inspect"
        };
        label(
            &mut commands,
            format!("{}\n{status}", card.title),
            Vec3::new(
                -2.0 + i as f32 * 2.4,
                0.2,
                if card.landscape { 2.96 } else { 3.36 },
            ),
            PAPER,
            &art.symbol_font,
        );
    }
    if view.promise_committed {
        let frame = materials.add(StandardMaterial {
            base_color: GOLD,
            unlit: true,
            ..default()
        });
        for (size, at) in [
            (Vec3::new(2.0, 0.055, 0.09), Vec3::new(0.4, 0.15, 0.57)),
            (Vec3::new(2.0, 0.055, 0.09), Vec3::new(0.4, 0.15, 3.23)),
            (Vec3::new(0.09, 0.055, 2.7), Vec3::new(-0.57, 0.15, 1.9)),
            (Vec3::new(0.09, 0.055, 2.7), Vec3::new(1.37, 0.15, 1.9)),
        ] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(frame.clone()),
                Transform::from_translation(at),
                Pickable::IGNORE,
                TransientScene,
            ));
        }
    }
    let mut canceled_index = 0;
    let mut active_index = 0;
    let has_canceled = view
        .tokens
        .iter()
        .any(|token| token.status == TokenStatus::Canceled);
    for (i, token) in view.tokens.iter().enumerate() {
        let canceled = token.status == TokenStatus::Canceled;
        let x = if canceled {
            let x = -4.5 + canceled_index as f32 * 1.3;
            canceled_index += 1;
            x
        } else if has_canceled {
            let x = -0.6 + active_index as f32 * 1.3;
            active_index += 1;
            x
        } else {
            -1.8 + i as f32 * 1.30
        };
        let is_new = rendered_view.as_ref().is_none_or(|previous| {
            !previous
                .tokens
                .iter()
                .any(|held| held.id == token.id && held.face == token.face)
        });
        let image = art
            .token_handles
            .iter()
            .find(|(face, _)| *face == token.face)
            .map(|(_, image)| image.clone());
        let (disk, label) =
            tabletop_tokens::spawn(
                &mut commands,
                &mut meshes,
                &mut materials,
                tabletop_tokens::TokenVisual {
                    face: token.face,
                    status: token.status,
                    ordinal: Some(usize::from(token.id) + 1),
                    image,
                    transform: Transform::from_xyz(x, 0.15, -0.20)
                        .with_scale(Vec3::splat(if is_new { 0.5 } else { 1.0 })),
                    label_offset: Vec3::new(0.0, -0.05, -1.0),
                },
            );
        commands.entity(disk).insert(TransientScene);
        commands.entity(label).insert(TransientScene);
        if is_new {
            commands.entity(disk).insert(Appearing);
        }
    }
    *rendered_view = Some(view.clone());
}

fn label_positions(
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut labels: Query<(&WorldLabel, &mut Node, &mut Visibility)>,
) {
    for (anchor, mut node, mut visibility) in &mut labels {
        if let Ok(point) = camera.0.world_to_viewport(camera.1, anchor.0) {
            node.left = px(point.x - 65.0);
            node.top = px(point.y - 16.0);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn animate_tokens(
    mut commands: Commands,
    time: Res<Time>,
    mut tokens: Query<(Entity, &mut Transform), With<Appearing>>,
) {
    for (entity, mut transform) in &mut tokens {
        transform.scale = transform
            .scale
            .lerp(Vec3::ONE, (time.delta_secs() * 10.0).min(1.0));
        if transform.scale.x > 0.995 {
            transform.scale = Vec3::ONE;
            commands.entity(entity).remove::<Appearing>();
        }
    }
}

fn animate_reminder(time: Res<Time>, mut reminder: Single<(&mut Transform, &mut ReminderPose)>) {
    let (transform, pose) = &mut *reminder;
    pose.angle += (pose.target - pose.angle) * (time.delta_secs() * 8.0).min(1.0);
    transform.rotation = Quat::from_rotation_z(pose.angle);
}

#[cfg(test)]
mod tests;
