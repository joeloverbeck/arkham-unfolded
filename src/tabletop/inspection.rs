//! A readable view of one original card and its effective text.

use super::{
    Action, Actions, Art, BUTTON, ButtonTone, CARDS, GOLD, MUTED, PAPER, Panel, Run, Symbol,
    scroll_panel, symbol_text,
};
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button, ModalDialog, ModalDialogBarrier, RequestClose},
};

pub(super) fn show(commands: &mut Commands, art: &Art, index: usize, viewport: Vec2) {
    let card = &CARDS[index];
    let width = (viewport.x * 0.94).min(1200.0);
    let height = viewport.y * 0.90;
    let padding = if width < 600.0 { 16.0 } else { 24.0 };
    let content_width = (width - 2.0 * padding).max(1.0);
    let preferred_art_width = if card.landscape { 780.0 } else { 500.0 };
    let has_image = art.handles[index].is_some();
    let side_by_side = has_image && content_width >= preferred_art_width + 24.0 + 280.0;
    let art_width = preferred_art_width.min(content_width);
    let art_height = art_width * if card.landscape { 5.0 / 7.0 } else { 7.0 / 5.0 };
    let text_width = if side_by_side {
        content_width - art_width - 24.0
    } else {
        content_width.min(780.0)
    };

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
            BackgroundColor(Color::srgba(0.015, 0.025, 0.03, 0.85)),
            GlobalZIndex(90),
            ModalDialogBarrier,
            Panel,
        ))
        .observe(|_: On<RequestClose>, mut queue: ResMut<Actions>| {
            queue.0.push(Action::CloseInspect);
        })
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(width),
                        max_height: px(height),
                        padding: UiRect::all(px(padding)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(20),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(8)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.07, 0.10, 0.115)),
                    BorderColor::all(GOLD),
                    ModalDialog,
                    TabGroup::modal(),
                ))
                .with_children(|panel| {
                    // The Close control stays visible while the artwork/text scroll.
                    panel
                        .spawn(Node {
                            width: percent(100),
                            align_items: AlignItems::Center,
                            column_gap: px(20),
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|header| {
                            header.spawn((
                                art.text(card.title, 28.0, GOLD),
                                Node {
                                    flex_grow: 1.0,
                                    min_width: px(0),
                                    ..default()
                                },
                            ));
                            header
                                .spawn((
                                    Button,
                                    Hovered::default(),
                                    TabIndex(0),
                                    Node {
                                        width: px(140),
                                        min_height: px(42),
                                        flex_shrink: 0.0,
                                        padding: UiRect::axes(px(12), px(10)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        border_radius: BorderRadius::all(px(5)),
                                        ..default()
                                    },
                                    BackgroundColor(BUTTON),
                                    ButtonTone(BUTTON),
                                ))
                                .observe(|_: On<Activate>, mut queue: ResMut<Actions>| {
                                    queue.0.push(Action::CloseInspect);
                                })
                                .with_child(art.text("Close card", 16.0, PAPER));
                        });
                    panel
                        .spawn((
                            Node {
                                width: percent(100),
                                min_height: px(0),
                                max_height: px((height - 2.0 * padding - 80.0).max(1.0)),
                                flex_direction: if side_by_side {
                                    FlexDirection::Row
                                } else {
                                    FlexDirection::Column
                                },
                                align_items: if side_by_side {
                                    AlignItems::Start
                                } else {
                                    AlignItems::Center
                                },
                                column_gap: px(24),
                                row_gap: px(24),
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            ScrollPosition::default(),
                        ))
                        .observe(scroll_panel)
                        .with_children(|body| {
                            if let Some(image) = &art.handles[index] {
                                body.spawn(Node {
                                    width: px(art_width),
                                    height: px(art_height),
                                    flex_shrink: 0.0,
                                    ..default()
                                })
                                .with_children(|frame| {
                                    // Rotation is presentation-only. The untouched source
                                    // image retains its original pixel dimensions and hash.
                                    let image_size = if card.landscape {
                                        Vec2::new(art_height, art_width)
                                    } else {
                                        Vec2::new(art_width, art_height)
                                    };
                                    frame.spawn((
                                        Name::new("Inspected card artwork"),
                                        ImageNode::new(image.clone()),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: percent(50),
                                            top: percent(50),
                                            width: px(image_size.x),
                                            height: px(image_size.y),
                                            ..default()
                                        },
                                        UiTransform {
                                            translation: Val2::percent(-50, -50),
                                            rotation: if card.landscape {
                                                Rot2::degrees(-90.0)
                                            } else {
                                                Rot2::IDENTITY
                                            },
                                            ..default()
                                        },
                                        Pickable::IGNORE,
                                    ));
                                });
                            }
                            body.spawn(Node {
                                width: px(text_width),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Column,
                                row_gap: px(18),
                                ..default()
                            })
                            .with_children(|rules| show_text(rules, art, index));
                        });
                });
        });
}

fn show_text(parent: &mut ChildSpawnerCommands, art: &Art, index: usize) {
    let card = &CARDS[index];
    if index == 1 {
        symbol_text(
            parent,
            &art.symbol_font,
            &[
                Run::Words("Skill / 4 "),
                Run::Icon(Symbol::Wild),
                Run::Words(" / Level 0"),
            ],
            card.kind,
            18.0,
            MUTED,
        );
    } else if index == 2 {
        symbol_text(
            parent,
            &art.symbol_font,
            &[
                Run::Words("Investigator / "),
                Run::Icon(Symbol::Willpower),
                Run::Words(" 5"),
            ],
            card.kind,
            18.0,
            MUTED,
        );
    } else {
        parent.spawn(art.text(card.kind, 18.0, MUTED));
    }
    match index {
        0 => symbol_text(
            parent,
            &art.symbol_font,
            &[
                Run::Words("Revelation — Test "),
                Run::Icon(Symbol::Willpower),
                Run::Words(" (3). For each point you fail by, take 1 horror."),
            ],
            card.text,
            20.0,
            PAPER,
        ),
        1 => symbol_text(
            parent,
            &art.symbol_font,
            &[
                Run::Words("After you commit Promise of Power to a skill test, add 1"),
                Run::Icon(Symbol::Curse),
                Run::Words(" token to the chaos bag. If you cannot, take 2 horror instead."),
            ],
            card.text,
            20.0,
            PAPER,
        ),
        _ => {
            parent.spawn(art.text(card.text, 20.0, PAPER));
        }
    }
}
