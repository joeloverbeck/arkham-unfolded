//! Contextual controls for the finite Jacqueline experiment. Offered choice IDs
//! remain the rules authority; selecting a component or cancellation is local UI state.

use super::*;
use crate::tabletop_layout::{self, Area};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Card(usize),
    Token(u8),
    Bag,
}

#[derive(Component)]
pub(super) struct Selectable(pub Source);

#[derive(Component)]
pub(super) struct SelectionOutline;

/// A separate presentation overlay: changing selection cannot replace the
/// observation scene or restart an in-flight token/reminder animation.
pub(super) fn highlight(
    mut commands: Commands,
    context: Res<Context>,
    objects: Query<(&Selectable, &Transform)>,
    outlines: Query<Entity, With<SelectionOutline>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut previous: Local<Option<(Source, Vec3)>>,
) {
    let selected = context.selected.and_then(|source| {
        objects
            .iter()
            .find(|(object, _)| object.0 == source)
            .map(|(_, transform)| (source, transform.translation))
    });
    if selected == *previous {
        return;
    }
    for entity in &outlines {
        commands.entity(entity).despawn();
    }
    if let Some((source, position)) = selected {
        let (width, depth) = match source {
            Source::Card(index) if CARDS[index].landscape => (2.8, 2.05),
            Source::Card(_) => (2.05, 2.8),
            Source::Token(_) => (1.12, 1.12),
            Source::Bag => (1.5, 1.5),
        };
        let material = materials.add(StandardMaterial {
            base_color: GOLD,
            unlit: true,
            ..default()
        });
        for (size, offset) in [
            (
                Vec3::new(width, 0.035, 0.045),
                Vec3::new(0.0, 0.0, -depth / 2.0),
            ),
            (
                Vec3::new(width, 0.035, 0.045),
                Vec3::new(0.0, 0.0, depth / 2.0),
            ),
            (
                Vec3::new(0.045, 0.035, depth),
                Vec3::new(-width / 2.0, 0.0, 0.0),
            ),
            (
                Vec3::new(0.045, 0.035, depth),
                Vec3::new(width / 2.0, 0.0, 0.0),
            ),
        ] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::new(position.x, 0.19, position.z) + offset),
                Pickable::IGNORE,
                SelectionOutline,
            ));
        }
    }
    *previous = selected;
}

#[derive(Resource, Default)]
pub(super) struct Context {
    pub selected: Option<Source>,
    pub open: bool,
    pub utilities: bool,
    pub anchor: Vec2,
    pub generation: u64,
    pub snapshot: Option<(Observation, Option<usize>)>,
    pub pending: Option<String>,
}

impl Context {
    pub fn invalidate(&mut self) {
        self.open = false;
        self.selected = None;
        self.pending = None;
        self.snapshot = None;
        self.generation += 1;
    }

    pub fn current(&self, table: &Tabletop) -> bool {
        self.snapshot.as_ref() == Some(&(table.view(), table.replay.as_ref().map(Replay::index)))
    }
}

pub(super) fn choices(view: &Observation, source: Source) -> Vec<&crate::Choice> {
    view.choices
        .iter()
        .filter(|choice| match source {
            Source::Card(1) => choice.id == "commit-promise",
            Source::Card(2) => choice.id == "use-jacqueline",
            Source::Bag => choice.id == "reveal",
            Source::Token(_) => choice.id.starts_with("cancel-"),
            _ => false,
        })
        .collect()
}

pub(super) fn source_less(id: &str) -> bool {
    matches!(id, "decline-promise" | "end")
}

pub(super) fn actor(view: &Observation) -> &'static str {
    if view.stage == Stage::Commit {
        "Patrice"
    } else {
        "Jacqueline"
    }
}

pub(super) fn card_position(index: usize, view: &Observation) -> Vec3 {
    match index {
        0 => tabletop_layout::TEST_CENTER + Vec3::new(-2.0, 0.125, -2.6),
        1 if view.promise_committed => tabletop_layout::TEST_CENTER + Vec3::new(0.4, 0.125, -2.6),
        1 => tabletop_layout::investigator_center(1, 3) + Vec3::new(0.0, 0.125, 2.7),
        _ => tabletop_layout::investigator_center(2, 3) + Vec3::new(-3.3, 0.125, 0.0),
    }
}

pub(super) fn select(event: On<PointerClick>, source: Source, queue: &mut Actions) {
    if event.button == PointerButton::Primary {
        queue.0.push(Action::Select(source, event.pointer.position));
    }
}

pub(super) fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    table: Res<Tabletop>,
    mut queue: ResMut<Actions>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        queue.0.push(if table.inspect.is_some() {
            Action::CloseInspect
        } else if table.result_open {
            Action::DismissResult
        } else if table.history {
            Action::History
        } else {
            Action::DismissContext
        });
    }
    // Buttons use Bevy's ordinary Tab/Enter activation. All camera keys are
    // captured for the lifetime of this menu, including its dismissal frame.
}

pub(super) fn synchronize(mut context: ResMut<Context>, mut table: ResMut<Tabletop>) {
    if context.snapshot.is_some() && !context.current(&table) {
        context.invalidate();
        table.dirty = true;
    }
}

fn cancellation_label(choice: &crate::Choice) -> String {
    let ids = choice.id.strip_prefix("cancel-").unwrap_or("");
    let ordinals = ids
        .split('-')
        .filter_map(|id| id.parse::<u8>().ok())
        .map(|id| format!("#{}", id + 1))
        .collect::<Vec<_>>()
        .join(" + ");
    format!("{} ({ordinals})", choice.label)
}

pub(super) fn show(
    commands: &mut Commands,
    context: &Context,
    table: &Tabletop,
    art: &Art,
    viewport: Vec2,
) {
    if !context.open
        || !context.current(table)
        || table.inspect.is_some()
        || table.result_open
        || table.history
    {
        return;
    }
    let Some(source) = context.selected else {
        return;
    };
    let view = table.view();
    let choices = if table.replay.is_none() {
        choices(&view, source)
    } else {
        vec![]
    };
    let title =
        match source {
            Source::Card(index) => CARDS[index].title.to_owned(),
            Source::Bag => format!("Chaos bag / {} tokens", view.bag_count),
            Source::Token(id) => view.tokens.iter().find(|token| token.id == id).map_or(
                "Chaos token".into(),
                |token| {
                    format!(
                        "Token #{} / {} / {:?}",
                        id + 1,
                        token.face.label(),
                        token.status
                    )
                },
            ),
        };
    let radial = !matches!(source, Source::Token(_)) && viewport.x >= 850.0 && viewport.y >= 600.0;
    let width = if radial {
        390.0
    } else {
        350.0_f32.min(viewport.x - 24.0)
    };
    let height = if radial {
        260.0
    } else {
        (180.0 + choices.len() as f32 * 70.0).min(viewport.y - 32.0)
    };
    let anchor = Vec2::new(
        context
            .anchor
            .x
            .clamp(12.0, (viewport.x - width - 12.0).max(12.0)),
        context
            .anchor
            .y
            .clamp(12.0, (viewport.y - height - 12.0).max(12.0)),
    );
    let generation = context.generation;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            GlobalZIndex(65),
            Panel,
        ))
        .observe(|event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == PointerButton::Primary {
                queue.0.push(Action::DismissContext);
            }
        });
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(anchor.x),
                top: px(anchor.y),
                width: px(width),
                height: px(height),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(INK),
            BorderColor::all(GOLD),
            GlobalZIndex(66),
            TabGroup::modal(),
            ScrollPosition::default(),
            Panel,
        ))
        .observe(scroll_panel)
        .with_children(|parent| {
            parent.spawn(art.text(title, 17.0, GOLD));
            if table.replay.is_some() {
                parent.spawn(art.text("Recorded view / inspection only", 14.0, MUTED));
            }
            if radial {
                parent
                    .spawn(Node {
                        height: px(110),
                        width: percent(100),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|ring| {
                        // Stable north action and east inspection positions survive legality changes.
                        ring.spawn(Node {
                            position_type: PositionType::Absolute,
                            left: px(70),
                            top: px(0),
                            width: px(220),
                            ..default()
                        })
                        .with_children(|slot| {
                            if let Some(choice) = choices.first() {
                                let label = match source {
                                    Source::Card(1) => "Commit +4",
                                    Source::Card(2) => "Use reaction",
                                    Source::Bag => "Reveal token",
                                    _ => "Choose",
                                };
                                button(
                                    slot,
                                    art,
                                    label,
                                    Action::ContextChoose(
                                        generation,
                                        view.revision,
                                        choice.id.clone(),
                                    ),
                                    true,
                                );
                            } else {
                                slot.spawn(art.text("No available action", 15.0, MUTED));
                            }
                        });
                        if let Source::Card(index) = source {
                            ring.spawn(Node {
                                position_type: PositionType::Absolute,
                                left: px(225),
                                top: px(60),
                                width: px(130),
                                ..default()
                            })
                            .with_children(|slot| {
                                button(slot, art, "Inspect", Action::Inspect(index), false)
                            });
                        }
                        ring.spawn(Node {
                            position_type: PositionType::Absolute,
                            left: px(5),
                            top: px(60),
                            width: px(125),
                            ..default()
                        })
                        .with_children(|slot| {
                            button(slot, art, "Dismiss", Action::DismissContext, false)
                        });
                    });
                for choice in &choices {
                    parent.spawn(art.text(&choice.label, 14.0, PAPER));
                }
            } else {
                if choices.is_empty() {
                    parent.spawn(art.text("No available game action", 14.0, MUTED));
                }
                for choice in choices {
                    if matches!(source, Source::Token(_)) {
                        let marked = context.pending.as_deref() == Some(choice.id.as_str());
                        button(
                            parent,
                            art,
                            &cancellation_label(choice),
                            Action::PrepareCancellation(generation, choice.id.clone()),
                            marked,
                        );
                    } else {
                        button(
                            parent,
                            art,
                            &choice.label,
                            Action::ContextChoose(generation, view.revision, choice.id.clone()),
                            true,
                        );
                    }
                }
                if let Some(id) = context
                    .pending
                    .as_ref()
                    .filter(|_| table.replay.is_none() && matches!(source, Source::Token(_)))
                {
                    button(
                        parent,
                        art,
                        "Confirm cancellation",
                        Action::ContextChoose(generation, view.revision, id.clone()),
                        true,
                    );
                }
                if let Source::Card(index) = source {
                    button(parent, art, "Inspect", Action::Inspect(index), false);
                }
                button(parent, art, "Dismiss", Action::DismissContext, false);
            }
        });
}

pub(super) fn focus_strip(
    parent: &mut ChildSpawnerCommands,
    art: &Art,
    images: Option<&Assets<Image>>,
) {
    for (label, area) in [
        ("Overview", Area::Overview),
        ("Scenario", Area::Scenario),
        ("Test", Area::Test),
    ] {
        button(parent, art, label, Action::Focus(area), false);
    }
    for (index, name) in ["Tony Morgan", "Patrice Hathaway", "Jacqueline Fine"]
        .iter()
        .enumerate()
    {
        let tone = tabletop_layout::investigator_color(index);
        let ready = art
            .investigators
            .iter()
            .find(|(found, _)| found == name)
            .and_then(|(_, handle)| {
                images.and_then(|images| images.get(handle)).map(|image| {
                    let size = image.size_f32();
                    (handle, size * (108.0 / size.x).min(82.0 / size.y))
                })
            });
        parent
            .spawn((
                Button,
                Hovered::default(),
                AccessibleLabel::new(format!("Focus {name}")),
                BackgroundColor(tone),
                ButtonTone(tone),
                Node {
                    width: ready.map_or(percent(100), |(_, size)| px(size.x + 12.0)),
                    min_height: px(ready.map_or(94.0, |(_, size)| size.y + 12.0)),
                    align_self: AlignSelf::Center,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::all(px(6)),
                    ..default()
                },
            ))
            .observe(move |_: On<Activate>, mut queue: ResMut<Actions>| {
                queue.0.push(Action::Focus(Area::Investigator(index)))
            })
            .with_children(|portrait| {
                if let Some((handle, size)) = ready {
                    portrait.spawn((
                        ImageNode::new(handle.clone()),
                        Node {
                            width: px(size.x),
                            height: px(size.y),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                } else {
                    portrait.spawn(art.text(*name, 15.0, PAPER));
                }
            });
    }
}
