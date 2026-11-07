//! The established Party screen layout. Adapters own content and action authority.
use crate::tabletop_feedback as feedback;
use bevy::{
    input::mouse::MouseScrollUnit,
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Button, ModalDialog, ModalDialogBarrier},
};

const INK: Color = Color::srgb(0.035, 0.055, 0.065);
const PAPER: Color = Color::srgb(0.91, 0.89, 0.81);
const GOLD: Color = Color::srgb(0.86, 0.69, 0.37);

pub(crate) fn header(
    commands: &mut Commands,
    marker: impl Bundle,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                top: px(10),
                right: px(130),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            Pickable::IGNORE,
            marker,
        ))
        .with_children(contents);
}

pub(crate) fn menu_button(
    commands: &mut Commands,
    marker: impl Bundle,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                top: px(12),
                width: px(108),
                ..default()
            },
            marker,
        ))
        .with_children(contents);
}

pub(crate) fn navigation(
    commands: &mut Commands,
    marker: impl Bundle,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(10),
                top: px(102),
                bottom: px(220),
                width: px(124),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Pickable::IGNORE,
            marker,
        ))
        .observe(scroll_panel)
        .with_children(contents);
}

pub(crate) fn decision_panel(
    commands: &mut Commands,
    marker: impl Bundle,
    size: Vec2,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(150),
                right: px(12),
                bottom: px(10),
                max_height: px((size.y * 0.29).min(270.)),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(INK),
            marker,
        ))
        .observe(scroll_panel)
        .with_children(contents)
        .id()
}

pub(crate) fn utilities(
    commands: &mut Commands,
    marker: impl Bundle,
    size: Vec2,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                top: px(62),
                width: px(305),
                max_height: px((size.y - 240.).max(150.)),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(INK),
            GlobalZIndex(45),
            marker,
        ))
        .observe(scroll_panel)
        .with_children(contents);
}

pub(crate) fn button(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
) -> Entity {
    parent
        .spawn((
            Button,
            Hovered::default(),
            TabIndex(0),
            AccessibleLabel::new(label),
            Node {
                width: percent(100),
                min_height: px(38),
                padding: UiRect::all(px(8)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_shrink: 0.,
                ..default()
            },
            feedback::context_button(),
        ))
        .with_child((
            crate::tabletop_harm_text::ui(label, 14., PAPER, font),
            Pickable::IGNORE,
        ))
        .id()
}

pub(crate) fn portrait(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    name: &str,
    image: Option<&(Handle<Image>, Vec2)>,
    current_turn: bool,
) -> Entity {
    let dimensions = image.map(|(_, size)| *size * (108. / size.x).min(82. / size.y));
    let label = format!(
        "Focus {name}{}",
        if current_turn { " · current turn" } else { "" }
    );
    parent
        .spawn((
            Button,
            Hovered::default(),
            TabIndex(0),
            AccessibleLabel::new(label),
            Node {
                width: px(dimensions.map_or(108., |size| size.x) + 10.),
                min_height: px(38),
                padding: UiRect::all(px(5)),
                align_self: AlignSelf::Center,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                flex_shrink: 0.,
                ..default()
            },
            feedback::context_button(),
        ))
        .with_children(|p| {
            if let Some(((image, _), size)) = image.zip(dimensions) {
                p.spawn((
                    ImageNode::new(image.clone()),
                    Node {
                        width: px(size.x),
                        height: px(size.y),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            if current_turn {
                p.spawn((
                    Node {
                        padding: UiRect::axes(px(8), px(2)),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    BackgroundColor(GOLD),
                    Pickable::IGNORE,
                ))
                .with_child((
                    crate::tabletop_harm_text::ui("TURN", 12., INK, font),
                    Pickable::IGNORE,
                ));
            }
        })
        .id()
}

pub(crate) fn controls(
    parent: &mut ChildSpawnerCommands,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(8),
            row_gap: px(5),
            ..default()
        })
        .with_children(contents);
}

pub(crate) fn control_slot(
    parent: &mut ChildSpawnerCommands,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            width: px(240),
            ..default()
        })
        .with_children(contents);
}

pub(crate) fn card_image(
    parent: &mut ChildSpawnerCommands,
    image: Handle<Image>,
    landscape: bool,
    available_width: f32,
) {
    let width = available_width.min(if landscape { 760. } else { 430. });
    let height = width * if landscape { 5. / 7. } else { 7. / 5. };
    parent
        .spawn(Node {
            width: px(width),
            height: px(height),
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|frame| {
            frame.spawn((
                ImageNode::new(image),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: percent(50),
                    width: px(if landscape { height } else { width }),
                    height: px(if landscape { width } else { height }),
                    border_radius: BorderRadius::all(px(
                        width.min(height) * crate::tabletop_pieces::card_shape::CORNER_FRACTION
                    )),
                    ..default()
                },
                UiTransform {
                    translation: Val2::percent(-50, -50),
                    rotation: if landscape {
                        Rot2::degrees(-90.)
                    } else {
                        Rot2::IDENTITY
                    },
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

/// The title and Close control remain outside the scrolling reading body.
/// Returns the barrier so the adapter can bind its own dismissal action.
#[allow(clippy::too_many_arguments)]
pub(crate) fn inspection(
    commands: &mut Commands,
    marker: impl Bundle,
    body_marker: impl Bundle,
    font: &Handle<Font>,
    size: Vec2,
    title: &str,
    close: impl FnOnce(&mut ChildSpawnerCommands),
    body: impl FnOnce(&mut ChildSpawnerCommands, f32),
) -> Entity {
    let width = (size.x * 0.94).min(1200.);
    let height = size.y * 0.9;
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
            marker,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(width),
                        max_height: px(height),
                        padding: UiRect::all(px(22)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(16),
                        ..default()
                    },
                    BackgroundColor(INK),
                    ModalDialog,
                    TabGroup::modal(),
                ))
                .with_children(|panel| {
                    panel.spawn(crate::tabletop_harm_text::ui(title, 27., GOLD, font));
                    close(panel);
                    panel
                        .spawn((
                            Node {
                                max_height: px((height - 150.).max(100.)),
                                overflow: Overflow::scroll_y(),
                                padding: UiRect::all(px(6)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: px(16),
                                ..default()
                            },
                            ScrollPosition::default(),
                            body_marker,
                        ))
                        .observe(scroll_panel)
                        .with_children(|p| body(p, width - 55.));
                });
        })
        .id()
}

pub(crate) fn scroll_panel(
    event: On<PointerScroll>,
    mut panels: Query<(&mut ScrollPosition, &ComputedNode)>,
) {
    if let Ok((mut scroll, node)) = panels.get_mut(event.entity) {
        let delta = event.y
            * if event.unit == MouseScrollUnit::Line {
                32.
            } else {
                1.
            };
        scroll.y = (scroll.y - delta).clamp(0., (node.content_size().y - node.size().y).max(0.));
    }
}

/// Party's blocking story-reading surface. Adapters own the revealed text,
/// artwork and Continue semantics; the geometry and modal capture are shared.
#[allow(clippy::too_many_arguments)]
pub(crate) fn story_reading(
    commands: &mut Commands,
    marker: impl Bundle,
    body_marker: impl Bundle,
    size: Vec2,
    accessible_title: &str,
    heading: impl FnOnce(&mut ChildSpawnerCommands),
    picture: impl FnOnce(&mut ChildSpawnerCommands, f32),
    description: impl FnOnce(&mut ChildSpawnerCommands),
    footer: impl FnOnce(&mut ChildSpawnerCommands),
) {
    let width = (size.x * 0.94).min(1200.);
    let columns = width >= 900.;
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
            BackgroundColor(Color::srgba(0.015, 0.025, 0.03, 0.92)),
            GlobalZIndex(90),
            ModalDialogBarrier,
            marker,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(width),
                        height: px(size.y * 0.9),
                        padding: UiRect::all(px(22)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        ..default()
                    },
                    BackgroundColor(INK),
                    ModalDialog,
                    TabGroup::modal(),
                    AccessibleLabel::new(accessible_title),
                ))
                .with_children(|panel| {
                    heading(panel);
                    panel
                        .spawn((
                            Node {
                                flex_grow: 1.,
                                min_height: px(0),
                                overflow: Overflow::scroll_y(),
                                padding: UiRect::all(px(6)),
                                flex_direction: if columns {
                                    FlexDirection::Row
                                } else {
                                    FlexDirection::Column
                                },
                                align_items: if columns {
                                    AlignItems::Start
                                } else {
                                    AlignItems::Center
                                },
                                column_gap: px(24),
                                row_gap: px(16),
                                ..default()
                            },
                            ScrollPosition::default(),
                            body_marker,
                        ))
                        .observe(scroll_panel)
                        .with_children(|body| {
                            picture(
                                body,
                                if columns {
                                    (width - 68.) * 0.45
                                } else {
                                    width - 44.
                                },
                            );
                            body.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: px(12),
                                width: if columns {
                                    px((width - 68.) * 0.55)
                                } else {
                                    percent(100)
                                },
                                flex_shrink: 0.,
                                ..default()
                            })
                            .with_children(description);
                        });
                    footer(panel);
                });
        });
}
