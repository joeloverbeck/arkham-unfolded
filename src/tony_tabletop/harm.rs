//! A short, local presentation of already committed harm. No rules run here.

use super::*;
use crate::tony::{HarmRecipient, HarmSource};

const STAGE_SECONDS: f32 = 1.2;
const DAMAGE: Color = Color::srgb(1.0, 0.48, 0.38);
const HORROR: Color = Color::srgb(0.48, 0.72, 1.0);

#[derive(Component)]
pub(super) struct HarmVisual {
    rest: Vec3,
    falling: bool,
}

pub(super) fn advance_presentation(
    time: Res<Time>,
    mut table: ResMut<Table>,
    settings: Res<AssetSettings>,
) {
    if table.modal() || settings.is_busy() || table.harm().is_none() {
        return;
    }
    table.stage_elapsed += time.delta_secs();
    if table.presenting() && table.stage_elapsed >= STAGE_SECONDS {
        table.presentation.pop_front();
        table.stage_elapsed = 0.0;
        table.dirty = true;
    }
}

pub(super) fn recipient_position(recipient: HarmRecipient) -> Vec3 {
    match recipient {
        HarmRecipient::Tony => Vec3::new(-4.7, 0.05, 1.2),
        HarmRecipient::Ghoul => Vec3::new(-2.1, 0.05, -1.9),
    }
}

pub(super) fn harm_amounts(parent: &mut ChildSpawnerCommands, art: &Art, hit: Harm) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            ..default()
        })
        .with_children(|p| {
            for (amount, label, glyph, color) in [
                (
                    hit.damage,
                    if hit.is_direct() {
                        "Direct damage"
                    } else {
                        "Damage"
                    },
                    "Q",
                    DAMAGE,
                ),
                (hit.horror, "Horror", "R", HORROR),
            ] {
                if amount == 0 {
                    continue;
                }
                p.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|row| {
                    if let Some(font) = &art.symbol_font {
                        row.spawn((
                            Text::new(glyph),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::Px(26.0),
                                ..default()
                            },
                            TextColor(color),
                        ));
                    }
                    row.spawn(text(format!("+{amount} {label}"), 22.0, color));
                });
            }
        });
}

fn counter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    at: Vec3,
    horror: bool,
    arriving: bool,
) {
    let role = if horror {
        "token/horror"
    } else {
        "token/damage"
    };
    let image = art.image(role);
    let color = if horror { HORROR } else { DAMAGE };
    let size = if horror {
        Vec2::new(0.43, 0.35)
    } else {
        Vec2::new(0.35, 0.43)
    };
    let mut entity = commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size.x, size.y))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: if image.is_some() { Color::WHITE } else { color },
            base_color_texture: image,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 2.0,
            ..default()
        })),
        Transform::from_translation(at),
        Scene,
        Pickable::IGNORE,
    ));
    if arriving {
        entity.insert(HarmVisual {
            rest: at,
            falling: true,
        });
    }
}

fn outline(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    size: Vec2,
    color: Color,
) {
    let material = materials.add(StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    });
    for (offset, dimensions) in [
        (
            Vec3::new(-size.x / 2.0, 0.02, 0.0),
            Vec3::new(0.035, 0.025, size.y),
        ),
        (
            Vec3::new(size.x / 2.0, 0.02, 0.0),
            Vec3::new(0.035, 0.025, size.y),
        ),
        (
            Vec3::new(0.0, 0.02, -size.y / 2.0),
            Vec3::new(size.x, 0.025, 0.035),
        ),
        (
            Vec3::new(0.0, 0.02, size.y / 2.0),
            Vec3::new(size.x, 0.025, 0.035),
        ),
    ] {
        let at = center + offset;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(dimensions))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(at),
            HarmVisual {
                rest: at,
                falling: false,
            },
            Scene,
            Pickable::IGNORE,
        ));
    }
}

pub(super) fn harm_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    art: &Art,
    view: &Observation,
    hit: Option<Harm>,
) {
    for (recipient, damage, horror, visible) in [
        (HarmRecipient::Tony, view.damage, view.horror, true),
        (HarmRecipient::Ghoul, view.ghoul.damage, 0, view.ghoul.alive),
    ] {
        if !visible {
            continue;
        }
        let at = recipient_position(recipient);
        for (amount, is_horror, x) in [(damage, false, -0.32), (horror, true, 0.32)] {
            if amount == 0 {
                continue;
            }
            let position = at + Vec3::new(x, 0.06, 0.35);
            let arriving = hit.is_some_and(|h| {
                h.recipient == recipient
                    && if is_horror {
                        h.horror > 0
                    } else {
                        h.damage > 0
                    }
            });
            counter(
                commands, meshes, materials, art, position, is_horror, arriving,
            );
            commands.spawn((
                text(amount.to_string(), 16.0, PAPER),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(30),
                    ..default()
                },
                BackgroundColor(INK),
                WorldLabel(position + Vec3::new(0.0, 0.08, 0.23), 30.0),
                Scene,
                Pickable::IGNORE,
            ));
        }
    }
    let Some(hit) = hit else {
        return;
    };
    let target = recipient_position(hit.recipient);
    let target_size = if hit.recipient == HarmRecipient::Tony {
        Vec2::new(CARD_LENGTH + 0.1, CARD_WIDTH + 0.1)
    } else {
        Vec2::new(CARD_WIDTH + 0.1, CARD_LENGTH + 0.1)
    };
    outline(commands, meshes, materials, target, target_size, PAPER);
    let (source, size) = match hit.source {
        HarmSource::OpportunityAttack => (
            recipient_position(HarmRecipient::Ghoul),
            Vec2::new(CARD_WIDTH + 0.1, CARD_LENGTH + 0.1),
        ),
        HarmSource::BasicFight => (
            recipient_position(HarmRecipient::Tony),
            Vec2::new(CARD_LENGTH + 0.1, CARD_WIDTH + 0.1),
        ),
        HarmSource::InternalInjury => (
            Vec3::new(3.0, 0.05, -1.9),
            Vec2::new(CARD_WIDTH + 0.1, CARD_LENGTH + 0.1),
        ),
        HarmSource::Tablet | HarmSource::Cultist => (Vec3::new(0.5, 0.2, 0.7), Vec2::splat(1.4)),
    };
    outline(commands, meshes, materials, source, size, GOLD);
    let mut amounts = Vec::new();
    if hit.damage > 0 {
        amounts.push(format!(
            "+{} {}",
            hit.damage,
            if hit.is_direct() {
                "Direct damage"
            } else {
                "Damage"
            }
        ));
    }
    if hit.horror > 0 {
        amounts.push(format!("+{} Horror", hit.horror));
    }
    world_label(
        commands,
        amounts.join("\n"),
        target + Vec3::new(0.0, 0.45, -0.9),
        GOLD,
    );
}

pub(super) fn animate_harm(table: Res<Table>, mut visuals: Query<(&HarmVisual, &mut Transform)>) {
    let progress = (table.stage_elapsed / 0.5).clamp(0.0, 1.0);
    for (visual, mut transform) in &mut visuals {
        transform.translation = visual.rest;
        transform.scale = Vec3::ONE;
        if table.reduced_motion {
            continue;
        }
        if visual.falling {
            transform.translation.y += 0.4 * (1.0 - progress).powi(2);
        } else {
            let pulse = (progress * std::f32::consts::PI).sin() * 0.06;
            transform.scale = Vec3::splat(1.0 + pulse);
        }
    }
}
