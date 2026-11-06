//! Content-sized reading stops; callers own their content and acknowledgment.
use super::*;
use bevy::ui_widgets::{ModalDialog, ModalDialogBarrier};

pub(super) fn panel(
    commands: &mut Commands,
    size: Vec2,
    width: f32,
    title: &str,
    heading: impl FnOnce(&mut ChildSpawnerCommands),
    details: impl FnOnce(&mut ChildSpawnerCommands),
    footer: impl FnOnce(&mut ChildSpawnerCommands),
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.025, 0.03, 0.88)),
            GlobalZIndex(90),
            ModalDialogBarrier,
            Panel,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(width),
                        max_height: px(size.y * 0.9),
                        padding: UiRect::all(px(22)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        ..default()
                    },
                    BackgroundColor(INK),
                    ModalDialog,
                    TabGroup::modal(),
                    AccessibleLabel::new(title),
                ))
                .with_children(|panel| {
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.,
                            row_gap: px(10),
                            ..default()
                        })
                        .with_children(heading);
                    panel
                        .spawn((
                            Node {
                                min_height: px(0),
                                overflow: Overflow::scroll_y(),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(px(6)),
                                row_gap: px(8),
                                ..default()
                            },
                            ScrollPosition::default(),
                            ScrollablePanel { inspection: true },
                        ))
                        .observe(crate::tabletop_ui::scroll_panel)
                        .with_children(details);
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_shrink: 0.,
                            row_gap: px(10),
                            ..default()
                        })
                        .with_children(footer);
                });
        });
}
