//! A timed announcement of an already committed phase. It never advances rules.
use super::*;
use crate::gathering::Phase;

const DURATION: f32 = 2.0;
const EDGE: f32 = 0.25;

pub(super) fn name(phase: Phase) -> &'static str {
    match phase {
        Phase::Investigation => "Investigation",
        Phase::Enemy => "Enemy",
        Phase::Upkeep | Phase::RoundEnd => "Upkeep",
        Phase::Mythos => "Mythos",
    }
}

fn identity(view: &Observation) -> Option<(u16, &'static str)> {
    view.play
        .as_ref()
        .map(|play| (play.round, name(play.phase)))
}

pub(super) struct Announcement {
    pub round: u16,
    pub phase: &'static str,
    pub elapsed: f32,
    pub after_setup_draws: bool,
}

impl Announcement {
    pub fn between(before: &Observation, after: &Observation) -> Option<Self> {
        let (round, phase) = identity(after)?;
        (identity(before) != Some((round, phase))).then_some(Self {
            round,
            phase,
            elapsed: 0.,
            after_setup_draws: before.play.is_none(),
        })
    }

    pub fn advance(&mut self, delta: f32) -> bool {
        self.elapsed += delta;
        self.elapsed >= DURATION
    }

    fn offset(&self, reduced_motion: bool) -> f32 {
        if reduced_motion {
            0.
        } else if self.elapsed < EDGE {
            -64. * (1. - self.elapsed / EDGE)
        } else if self.elapsed > DURATION - EDGE {
            64. * ((self.elapsed - (DURATION - EDGE)) / EDGE).min(1.)
        } else {
            0.
        }
    }
}

#[derive(Component)]
pub(super) struct Banner;
#[derive(Component)]
pub(super) struct BannerFace;

/// Update just the overlay while it moves; do not rebuild the table every frame.
pub(super) fn render(
    mut commands: Commands,
    table: Res<Table>,
    existing: Query<Entity, With<Banner>>,
    mut face: Query<&mut Node, With<BannerFace>>,
    mut shown: Local<Option<(u16, &'static str)>>,
) {
    let announcement = table.presentation.announcement();
    let key = announcement.map(|announcement| (announcement.round, announcement.phase));
    if key != *shown {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        *shown = key;
        if let Some(announcement) = announcement {
            // The full-screen barrier keeps the pointer off the table while the
            // announcement shows, and the presentation state drops other input. The
            // banner offers nothing to focus, so it is not a modal dialog.
            use bevy::ui_widgets::ModalDialogBarrier;
            commands
                .spawn((
                    Banner,
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100),
                        height: percent(100),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    GlobalZIndex(110),
                    ModalDialogBarrier,
                ))
                .with_children(|overlay| {
                    overlay
                        .spawn((
                            BannerFace,
                            Node {
                                left: px(announcement.offset(table.reduced_motion)),
                                width: percent(80),
                                max_width: px(640),
                                padding: UiRect::axes(px(32), px(24)),
                                border: UiRect::vertical(px(2)),
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Column,
                                row_gap: px(8),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.025, 0.09, 0.12, 0.98)),
                            BorderColor::all(GOLD),
                            AccessibleLabel::new(format!(
                                "{} phase, round {}",
                                announcement.phase, announcement.round
                            )),
                        ))
                        .with_children(|panel| {
                            panel.spawn((
                                Text::new(announcement.phase),
                                TextFont {
                                    font_size: 48.0.into(),
                                    ..default()
                                },
                                TextColor(PAPER),
                            ));
                            panel.spawn((
                                Text::new(format!("Round {}", announcement.round)),
                                TextFont {
                                    font_size: 22.0.into(),
                                    ..default()
                                },
                                TextColor(GOLD),
                            ));
                        });
                });
        }
    }
    if let Some(announcement) = announcement {
        let offset = announcement.offset(table.reduced_motion);
        for mut node in &mut face {
            node.left = px(offset);
        }
    }
}
