//! Contextual controls for the finite Jacqueline experiment. Offered choice IDs
//! remain the rules authority; selecting a component or cancellation is local UI state.

use super::*;
use crate::tabletop_layout::{self, Area};

#[cfg(test)]
mod tests;

/// Receives empty-space dismissal clicks without capturing camera gestures.
#[derive(Component)]
pub(super) struct DismissalBackdrop;

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

/// Keep feedback separate from the observation scene: pointer movement and
/// menu dismissal must not replace cards or restart token/reminder animations.
pub(super) fn feedback(
    table: Res<Tabletop>,
    context: Res<Context>,
    settings: Option<Res<AssetSettings>>,
    mut feedback: ResMut<crate::tabletop_feedback::FeedbackSettings>,
    mut objects: Query<(&Selectable, &mut crate::tabletop_feedback::CardFeedback)>,
) {
    feedback.blocked = table.inspect.is_some()
        || table.result_open
        || table.history
        || settings.is_some_and(|settings| settings.is_busy());
    feedback.reduced_motion = table.reduced_motion;
    for (source, mut mark) in &mut objects {
        mark.menu_source =
            context.open && !feedback.blocked && context.selected_source() == Some(source.0);
        mark.pending_choice = match source.0 {
            Source::Token(id) => context.pending_cancellation().is_some_and(|choice| {
                choice
                    .strip_prefix("cancel-")
                    .into_iter()
                    .flat_map(|ids| ids.split('-'))
                    .any(|selected| selected.parse::<u8>() == Ok(id))
            }),
            _ => false,
        };
    }
}

/// The bag consists of two independent clickable meshes; retain its small
/// source cue only while its menu is open. Cards and tokens use shared feedback.
#[allow(clippy::too_many_arguments)]
pub(super) fn highlight(
    mut commands: Commands,
    context: Res<Context>,
    feedback: Res<crate::tabletop_feedback::FeedbackSettings>,
    objects: Query<(&Selectable, &Transform)>,
    outlines: Query<Entity, With<SelectionOutline>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut previous: Local<Option<(Source, Vec3)>>,
) {
    let selected = context
        .selected_source()
        .filter(|source| !feedback.blocked && context.menu_open() && *source == Source::Bag)
        .and_then(|source| {
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
    if let Some((_, position)) = selected {
        let (width, depth) = (1.5, 1.5);
        let material = materials.add(StandardMaterial {
            base_color: MUTED,
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
    selected: Option<Source>,
    open: bool,
    utilities: bool,
    anchor: Vec2,
    generation: u64,
    snapshot: Option<(Observation, Option<usize>)>,
    pending: Option<String>,
}

impl Context {
    pub fn selected_source(&self) -> Option<Source> {
        self.selected
    }

    pub fn menu_open(&self) -> bool {
        self.open
    }

    pub fn utilities_open(&self) -> bool {
        self.utilities
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn pending_cancellation(&self) -> Option<&str> {
        self.pending.as_deref()
    }

    pub fn select(
        &mut self,
        view: &Observation,
        replay_index: Option<usize>,
        source: Source,
        anchor: Vec2,
    ) {
        if !self.current(view, replay_index) {
            self.pending = None;
        }
        self.generation += 1;
        self.selected = Some(source);
        self.anchor = anchor;
        self.open = true;
        self.snapshot = Some((view.clone(), replay_index));
    }

    /// Inspection and focus close the menu but retain selection and utilities.
    pub fn close_menu(&mut self) {
        self.open = false;
        self.generation += 1;
    }

    pub fn dismiss(&mut self) {
        self.close_menu();
        self.utilities = false;
    }

    pub fn toggle_utilities(&mut self) {
        self.close_menu();
        self.utilities = !self.utilities;
    }

    /// Camera movement invalidates callbacks only while closing an open menu.
    pub fn camera_moved(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.close_menu();
        true
    }

    /// Mode transitions invalidate even when their attempted operation fails or
    /// leaves an identical observation. Anchor and utilities retain their state.
    pub fn invalidate(&mut self) {
        self.close_menu();
        self.selected = None;
        self.pending = None;
        self.snapshot = None;
    }

    /// Call at the existing post-action/synchronization points, not before
    /// admission: source-less callbacks deliberately do not require a snapshot.
    pub fn synchronize(&mut self, view: &Observation, replay_index: Option<usize>) -> bool {
        if self.snapshot.is_none() || self.current(view, replay_index) {
            return false;
        }
        self.invalidate();
        true
    }

    fn current(&self, view: &Observation, replay_index: Option<usize>) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|(snapshot, index)| snapshot == view && *index == replay_index)
    }

    pub fn prepare_cancellation(
        &mut self,
        view: &Observation,
        replay_index: Option<usize>,
        generation: u64,
        id: &str,
    ) {
        // Preserve preparation's existing lack of a menu-open check. Each token
        // offers every complete cancellation, including sets without that token.
        if replay_index.is_none()
            && self.generation == generation
            && self.current(view, replay_index)
            && self.selected.is_some_and(|source| {
                matches!(source, Source::Token(_))
                    && choices(view, source).iter().any(|choice| choice.id == id)
            })
        {
            self.pending = Some(id.to_owned());
        }
    }

    /// Admit an exact offered choice without committing it. The continuity
    /// boundary still owns revision checking, save-before-install and failure.
    pub fn offered_choice<'a>(
        &self,
        view: &'a Observation,
        replay_index: Option<usize>,
        generation: u64,
        id: &str,
    ) -> Option<&'a crate::Choice> {
        if replay_index.is_some()
            || !self.open
            || self.generation != generation
            || !self.current(view, replay_index)
            || (id.starts_with("cancel-") && self.pending_cancellation() != Some(id))
        {
            return None;
        }
        choices(view, self.selected?)
            .into_iter()
            .find(|choice| choice.id == id)
    }

    pub fn offered_source_less<'a>(
        &self,
        view: &'a Observation,
        replay_index: Option<usize>,
        generation: u64,
        id: &str,
    ) -> Option<&'a crate::Choice> {
        if replay_index.is_some() || self.generation != generation || !source_less(id) {
            return None;
        }
        view.choices.iter().find(|choice| choice.id == id)
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
    if context.synchronize(&table.view(), table.replay.as_ref().map(Replay::index)) {
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

fn choice_label(choice: &crate::Choice) -> &str {
    if choice.id == "commit-promise" {
        // The printed icons stay wild; the offered bonus applies to this willpower test.
        "Commit Promise of Power (+4 willpower; add a curse)"
    } else {
        &choice.label
    }
}

pub(super) fn show(
    commands: &mut Commands,
    context: &Context,
    table: &Tabletop,
    art: &Art,
    viewport: Vec2,
) {
    if !context.open
        || !context.current(&table.view(), table.replay.as_ref().map(Replay::index))
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
            DismissalBackdrop,
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
            // A modal tab group must acquire focus before Tab can navigate it.
            let mut first_button = true;
            let mut menu_button = |parent: &mut ChildSpawnerCommands,
                                   art: &Art,
                                   label: &str,
                                   action: Action,
                                   _primary: bool| {
                let entity = button(parent, art, label, action, false);
                parent
                    .commands()
                    .entity(entity)
                    .insert(crate::tabletop_feedback::context_button());
                if first_button {
                    parent
                        .commands()
                        .entity(entity)
                        .insert(bevy::input_focus::AutoFocus);
                    first_button = false;
                }
            };
            parent.spawn(art.text(title, 17.0, GOLD));
            parent.spawn(art.text(crate::tabletop_feedback::KEYBOARD_HINT, 14.0, MUTED));
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
                                    Source::Card(1) => "Commit +4 willpower",
                                    Source::Card(2) => "Use reaction",
                                    Source::Bag => "Reveal token",
                                    _ => "Choose",
                                };
                                menu_button(
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
                                menu_button(slot, art, "Inspect", Action::Inspect(index), false)
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
                            menu_button(slot, art, "Dismiss", Action::DismissContext, false)
                        });
                    });
                for choice in &choices {
                    parent.spawn(art.text(choice_label(choice), 14.0, PAPER));
                }
            } else {
                if choices.is_empty() {
                    parent.spawn(art.text("No available game action", 14.0, MUTED));
                }
                for choice in choices {
                    if matches!(source, Source::Token(_)) {
                        let marked = context.pending.as_deref() == Some(choice.id.as_str());
                        menu_button(
                            parent,
                            art,
                            &format!(
                                "{}{}",
                                if marked { "Selected: " } else { "" },
                                cancellation_label(choice)
                            ),
                            Action::PrepareCancellation(generation, choice.id.clone()),
                            marked,
                        );
                    } else {
                        menu_button(
                            parent,
                            art,
                            choice_label(choice),
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
                    menu_button(
                        parent,
                        art,
                        "Confirm cancellation",
                        Action::ContextChoose(generation, view.revision, id.clone()),
                        true,
                    );
                }
                if let Source::Card(index) = source {
                    menu_button(parent, art, "Inspect", Action::Inspect(index), false);
                }
                menu_button(parent, art, "Dismiss", Action::DismissContext, false);
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
