//! Lifetime and motion of the displayed observation, independent of panel refreshes.

use super::{Art, CARDS, GOLD, MUTED, MainCamera, PAPER, Tabletop};
use crate::tabletop_pacing::{PresentationTime, Redraw, RequestRedraw};
use crate::{Observation, TokenStatus, tabletop_harm_text, tabletop_tokens};
use crate::{tabletop_layout, tabletop_standees as standees};
use bevy::prelude::*;

/// Install the same scene lifecycle for the native tabletop and headless regressions.
/// The host supplies Tabletop, Art, meshes/materials, time and the main camera.
pub(super) fn install(app: &mut App) {
    app.init_resource::<super::Context>()
        .init_resource::<Redraw>()
        .add_systems(Last, request_redraw.in_set(RequestRedraw))
        .add_systems(Startup, setup_reminder)
        .add_systems(
            Update,
            (
                // Preserve world-label creation before the equal-z panel roots, so
                // observation replacement keeps labels beneath sidebar/history content.
                synchronize
                    .after(super::handle_actions)
                    .after(super::camera_controls)
                    .before(super::rebuild_panel),
                super::context::highlight
                    .after(synchronize)
                    .before(super::rebuild_panel),
                (
                    label_positions,
                    tabletop_tokens::position_labels,
                    token_label_visibility,
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
#[derive(Component)]
struct StandeeLabel {
    left: bool,
}

const LOCATION_SIZE: Vec2 = Vec2::new(1.80, 2.52);
const STUDY_AT: Vec3 = Vec3::new(-1.8, 0.0, -5.8);
const HALLWAY_AT: Vec3 = Vec3::new(-6.2, 0.0, -4.3);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum LabelDisplay {
    Detail,
    Overview,
    Always,
}

impl LabelDisplay {
    fn visible(self, distance: f32) -> bool {
        match self {
            Self::Detail => distance <= 30.0,
            Self::Overview => distance > 30.0,
            Self::Always => true,
        }
    }
}

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
            Transform::from_xyz(13.5, 0.22, 10.0),
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
) -> Entity {
    commands
        .spawn((
            tabletop_harm_text::ui(value, 14.0, color, font),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                width: px(148),
                padding: UiRect::axes(px(8), px(4)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.055, 0.90)),
            Pickable::IGNORE,
            WorldLabel(point),
            LabelDisplay::Detail,
            TransientScene,
        ))
        .id()
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
        Vec3::new(13.5, 0.15, 10.9),
        if view.jacqueline_used { MUTED } else { GOLD },
        &art.symbol_font,
    );
    for (location, center) in [("Study", STUDY_AT), ("Hallway", HALLWAY_AT)] {
        let occupants: Vec<_> = view
            .investigators
            .iter()
            .enumerate()
            .filter(|(_, investigator)| investigator.location == location)
            .collect();
        if occupants.is_empty() {
            continue;
        }
        // Named footprints describe only locations supplied by this experiment.
        // No location rules, card faces or new selectable actions are introduced.
        commands.spawn((
            Name::new(format!("Location marker: {location}")),
            Mesh3d(meshes.add(Cuboid::new(LOCATION_SIZE.x, 0.035, LOCATION_SIZE.y))),
            MeshMaterial3d(materials.add(Color::srgb(0.26, 0.30, 0.29))),
            Transform::from_translation(center + Vec3::Y * 0.02),
            Pickable::IGNORE,
            TransientScene,
        ));
        let marker_name = label(
            &mut commands,
            location,
            center + Vec3::Y * 0.06,
            PAPER,
            &art.symbol_font,
        );
        commands
            .entity(marker_name)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.width = px(66);
                node.padding = UiRect::all(px(2));
            });
        for (slot, (i, investigator)) in occupants.into_iter().enumerate() {
            let position = tabletop_layout::location_standee_position(center, LOCATION_SIZE, slot)
                .expect("the prepared experiment has at most three investigators per location");
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
                position,
                tabletop_layout::investigator_class(i),
            );
            commands.entity(standee).insert(TransientScene);
            let left = slot < 2;
            let label_at = position
                + Vec3::new(
                    if left { -1.0 } else { 1.0 } * (standees::BASE_RADIUS + 0.10),
                    standees::HEIGHT / 2.0,
                    0.0,
                );
            let name = investigator
                .name
                .split_whitespace()
                .next()
                .unwrap_or(&investigator.name);
            let name_label = label(
                &mut commands,
                format!(
                    "{name}\nHorror {}/{}",
                    investigator.horror, investigator.sanity
                ),
                label_at,
                PAPER,
                &art.symbol_font,
            );
            commands.entity(name_label).insert(StandeeLabel { left });
            commands
                .entity(name_label)
                .entry::<Node>()
                .and_modify(|mut node| {
                    node.width = px(100);
                    node.padding = UiRect::all(px(3));
                });
        }
    }
    label(
        &mut commands,
        format!(
            "Chaos bag · {}\n{} curse in bag",
            view.bag_count, view.curses_in_bag
        ),
        Vec3::new(-6.4, 1.2, 0.5),
        GOLD,
        &art.symbol_font,
    );
    for (index, investigator) in view.investigators.iter().enumerate() {
        let center = crate::tabletop_layout::investigator_center(index, 3);
        let name = label(
            &mut commands,
            &investigator.name,
            center + Vec3::new(0.0, 0.2, -4.05),
            GOLD,
            &art.symbol_font,
        );
        commands.entity(name).insert(LabelDisplay::Always);
        label(
            &mut commands,
            format!(
                "{}\nHorror {}/{}",
                investigator.location, investigator.horror, investigator.sanity
            ),
            center + Vec3::new(-3.5, 0.2, -2.7),
            PAPER,
            &art.symbol_font,
        );
        label(
            &mut commands,
            "Threat area",
            center + Vec3::new(0.0, 0.1, -2.7),
            MUTED,
            &art.symbol_font,
        );
        label(
            &mut commands,
            "Assets",
            center + Vec3::new(0.0, 0.1, -0.7),
            MUTED,
            &art.symbol_font,
        );
        label(
            &mut commands,
            "Hand",
            center + Vec3::new(-3.4, 0.1, 3.8),
            MUTED,
            &art.symbol_font,
        );
        label(
            &mut commands,
            "Deck / discard\nNot represented",
            center + Vec3::new(3.6, 0.1, 2.5),
            MUTED,
            &art.symbol_font,
        );
    }
    let scenario_name = label(
        &mut commands,
        "Scenario / Study",
        Vec3::new(0.0, 0.1, -10.2),
        GOLD,
        &art.symbol_font,
    );
    commands.entity(scenario_name).insert(LabelDisplay::Always);
    label(
        &mut commands,
        "Encounter deck / discard\nReserved / not represented",
        Vec3::new(7.2, 0.1, -7.4),
        MUTED,
        &art.symbol_font,
    );
    label(
        &mut commands,
        "Act / agenda / victory display\nReserved / not represented",
        Vec3::new(-7.2, 0.1, -7.4),
        MUTED,
        &art.symbol_font,
    );
    label(
        &mut commands,
        "Shared test area",
        Vec3::new(0.0, 0.1, 2.2),
        GOLD,
        &art.symbol_font,
    );
    for (i, card) in CARDS.iter().enumerate() {
        let position = super::context::card_position(i, &view);
        let rotation = Quat::from_rotation_y(if card.landscape {
            std::f32::consts::FRAC_PI_2
        } else {
            0.0
        });
        let material = materials.add(StandardMaterial {
            base_color: if art.handles[i].is_some() {
                Color::WHITE
            } else {
                card.color
            },
            base_color_texture: art.handles[i].clone(),
            perceptual_roughness: 0.9,
            unlit: true,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.85, 0.07, 2.60))),
            MeshMaterial3d(materials.add(Color::srgb(0.14, 0.12, 0.10))),
            Transform::from_translation(position - Vec3::Y * 0.045).with_rotation(rotation),
            Pickable::IGNORE,
            TransientScene,
        ));
        commands
            .spawn((
                Mesh3d(meshes.add(Plane3d::default().mesh().size(1.80, 2.52))),
                MeshMaterial3d(material),
                Transform::from_translation(position).with_rotation(rotation),
                TransientScene,
                super::context::Selectable(super::Source::Card(i)),
            ))
            .observe(
                move |event: On<PointerClick>, mut queue: ResMut<super::Actions>| {
                    super::context::select(event, super::Source::Card(i), &mut queue);
                },
            );
        let status = if i == 1 && view.promise_committed {
            "Committed / +4"
        } else if i == 1 {
            "Patrice's hand"
        } else {
            "Select for actions / Inspect"
        };
        let caption_position =
            position + Vec3::new(0.0, 0.07, if card.landscape { 1.25 } else { -1.65 });
        let caption = label(
            &mut commands,
            format!("{}\n{status}", card.title),
            caption_position,
            PAPER,
            &art.symbol_font,
        );
        commands.entity(caption).insert((
            Node {
                position_type: PositionType::Absolute,
                width: px(108),
                padding: UiRect::all(px(4)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
        ));
        if art.handles[i].is_none() {
            let overview_position = if i == 1 && view.promise_committed {
                caption_position + Vec3::Z * 2.9
            } else {
                caption_position
            };
            let title = label(
                &mut commands,
                card.title,
                overview_position,
                PAPER,
                &art.symbol_font,
            );
            commands.entity(title).insert(LabelDisplay::Overview);
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
                    transform: Transform::from_xyz(x, 0.15, 1.2)
                        .with_scale(Vec3::splat(if is_new { 0.5 } else { 1.0 })),
                    label_offset: Vec3::new(0.0, -0.05, -1.0),
                },
            );
        let token_id = token.id;
        commands
            .entity(disk)
            .insert((
                TransientScene,
                Pickable::default(),
                super::context::Selectable(super::Source::Token(token_id)),
            ))
            .observe(
                move |event: On<PointerClick>, mut queue: ResMut<super::Actions>| {
                    super::context::select(event, super::Source::Token(token_id), &mut queue);
                },
            );
        commands.entity(label).insert(TransientScene);
        if is_new {
            commands.entity(disk).insert(Appearing);
        }
    }
    *rendered_view = Some(view.clone());
}

fn label_positions(
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    orbit: Res<super::Orbit>,
    mut labels: Query<(
        &WorldLabel,
        &LabelDisplay,
        Option<&StandeeLabel>,
        &mut Node,
        &mut Visibility,
    )>,
) {
    for (anchor, display, standee, mut node, mut visibility) in &mut labels {
        if let Ok(point) = camera.0.world_to_viewport(camera.1, anchor.0) {
            let Val::Px(width) = node.width else { continue };
            node.left = px(match standee {
                Some(label) if label.left => point.x - width,
                Some(_) => point.x,
                None => point.x - width / 2.0,
            });
            node.top = px(point.y - 16.0);
            *visibility = if display.visible(orbit.pose().distance) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn token_label_visibility(
    orbit: Res<super::Orbit>,
    mut labels: Query<&mut Visibility, With<tabletop_tokens::TokenLabel>>,
) {
    if orbit.pose().distance > 30.0 {
        for mut visibility in &mut labels {
            *visibility = Visibility::Hidden;
        }
    }
}

fn animate_tokens(
    mut commands: Commands,
    time: Res<PresentationTime>,
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

fn animate_reminder(
    time: Res<PresentationTime>,
    mut reminder: Single<(&mut Transform, &mut ReminderPose)>,
) {
    let (transform, pose) = &mut *reminder;
    pose.angle += (pose.target - pose.angle) * (time.delta_secs() * 8.0).min(1.0);
    if (pose.target - pose.angle).abs() < 0.001 {
        pose.angle = pose.target;
    }
    transform.rotation = Quat::from_rotation_z(pose.angle);
}

fn request_redraw(
    mut redraw: ResMut<Redraw>,
    appearing: Query<(), With<Appearing>>,
    reminders: Query<&ReminderPose>,
) {
    if !appearing.is_empty() || reminders.iter().any(|pose| pose.angle != pose.target) {
        redraw.request();
    }
}

#[cfg(test)]
mod tests;
