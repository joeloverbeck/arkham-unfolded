//! One reveal affordance for both tables. Adapters supply current offered choices
//! and commit accepted requests through their existing persistence authority.

use crate::{
    Choice, tabletop_feedback as feedback,
    tabletop_pacing::{PresentationTime, Redraw, RequestRedraw},
};
use bevy::{
    input_focus::{
        AutoFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
};

pub(crate) const ART_ROLE: &str = "ui/reveal-chaos-token";
const INK: Color = Color::srgb(0.035, 0.065, 0.075);
const PAPER: Color = Color::srgb(0.92, 0.90, 0.80);
const TEAL: Color = Color::srgb(0.50, 0.82, 0.75);
const FADE_SECONDS: f32 = 0.14;

#[derive(Resource, Default)]
pub(crate) struct Artwork(pub Option<Handle<Image>>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Offer {
    revision: u64,
    reaction: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Route {
    Normal,
    Jacqueline,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Request {
    Press(u64),
    Choose(u64, Route),
    Dismiss(u64),
}

pub(crate) enum Accepted {
    Opened,
    Choice(u64, &'static str),
    Dismissed,
}

#[derive(Resource, Default)]
pub(crate) struct Shortcut {
    offer: Option<Offer>,
    generation: u64,
    chooser: bool,
    reduced_motion: bool,
    dirty: bool,
    opacity: f32,
    requests: Vec<Request>,
}

impl Shortcut {
    /// The offered choices are authoritative; a presentation-stage name is not.
    pub(crate) fn sync(
        &mut self,
        revision: u64,
        choices: &[Choice],
        available: bool,
        reduced_motion: bool,
    ) {
        let offer = (available && choices.iter().any(|c| c.id == "reveal")).then(|| Offer {
            revision,
            reaction: choices.iter().any(|c| c.id == "use-jacqueline"),
        });
        if offer != self.offer {
            self.offer = offer;
            self.dismiss();
        }
        self.reduced_motion = reduced_motion;
    }

    pub(crate) fn dismiss(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.chooser = false;
        self.dirty = true;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.chooser
    }

    pub(crate) fn drain(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    /// Called only after the adapter refreshes the current offer and blockers.
    pub(crate) fn admit(&mut self, request: Request) -> Option<Accepted> {
        let generation = match request {
            Request::Press(g) | Request::Choose(g, _) | Request::Dismiss(g) => g,
        };
        if generation != self.generation {
            return None;
        }
        let offer = self.offer?;
        match request {
            Request::Press(_) if offer.reaction => {
                self.dismiss();
                self.chooser = true;
                Some(Accepted::Opened)
            }
            Request::Press(_) => {
                self.dismiss();
                Some(Accepted::Choice(offer.revision, "reveal"))
            }
            Request::Choose(_, route) if self.chooser && offer.reaction => {
                self.dismiss();
                Some(Accepted::Choice(
                    offer.revision,
                    match route {
                        Route::Normal => "reveal",
                        Route::Jacqueline => "use-jacqueline",
                    },
                ))
            }
            Request::Dismiss(_) => {
                self.dismiss();
                Some(Accepted::Dismissed)
            }
            _ => None,
        }
    }
}

pub(crate) struct RevealPlugin;
impl Plugin for RevealPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Shortcut>()
            .init_resource::<Artwork>()
            .init_resource::<PresentationTime>()
            .init_resource::<Redraw>()
            .add_systems(Last, request_redraw.in_set(RequestRedraw));
    }
}

#[derive(Component)]
pub(crate) struct Root;
#[derive(Component)]
pub(crate) struct FadeColor(Color);
#[derive(Component)]
pub(crate) struct RevealButton;

fn label(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color, opacity: f32) {
    parent.spawn((
        Text::new(crate::tabletop::readable_text(value.into())),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color.with_alpha(opacity)),
        FadeColor(color),
        Pickable::IGNORE,
    ));
}

fn button(
    parent: &mut ChildSpawnerCommands,
    value: &str,
    request: Request,
    image: Option<Handle<Image>>,
    opacity: f32,
) -> Entity {
    parent
        .spawn((
            Button,
            Hovered::default(),
            TabIndex(0),
            RevealButton,
            Node {
                min_height: px(46),
                padding: UiRect::all(px(10)),
                column_gap: px(10),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(INK.with_alpha(opacity)),
            feedback::FeedbackOpacity(opacity),
            feedback::context_button(),
        ))
        .observe(move |_: On<Activate>, mut shortcut: ResMut<Shortcut>| {
            shortcut.requests.push(request)
        })
        .with_children(|parent| {
            if let Some(image) = image {
                parent.spawn((
                    ImageNode {
                        color: TEAL.with_alpha(opacity),
                        ..ImageNode::new(image)
                    },
                    Node {
                        width: px(42),
                        height: px(42),
                        ..default()
                    },
                    FadeColor(TEAL),
                    Pickable::IGNORE,
                ));
            }
            label(parent, value, 16.0, PAPER, opacity);
        })
        .id()
}

/// Called after the adapter refreshes availability. The root survives a short
/// exit transition, but all input expires at the legality boundary immediately.
// Bevy injects each presentation resource and disjoint query independently.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn render(
    mut commands: Commands,
    mut shortcut: ResMut<Shortcut>,
    artwork: Res<Artwork>,
    time: Res<PresentationTime>,
    roots: Query<Entity, With<Root>>,
    mut transforms: Query<&mut UiTransform, With<Root>>,
    mut colors: Query<(
        &FadeColor,
        Option<&mut TextColor>,
        Option<&mut ImageNode>,
        Option<&mut BackgroundColor>,
    )>,
    buttons: Query<Entity, With<RevealButton>>,
    mut feedback_opacity: Query<&mut feedback::FeedbackOpacity, With<RevealButton>>,
) {
    let visible = shortcut.offer.is_some();
    let target = if visible { 1.0 } else { 0.0 };
    if shortcut.reduced_motion {
        shortcut.opacity = target;
    } else {
        let step = time.delta_secs() / FADE_SECONDS;
        shortcut.opacity = if visible {
            (shortcut.opacity + step).min(1.0)
        } else {
            (shortcut.opacity - step).max(0.0)
        };
    }
    if !visible {
        for entity in &buttons {
            commands
                .entity(entity)
                .remove::<(Button, TabIndex)>()
                .insert(Pickable::IGNORE);
        }
    }
    if shortcut.dirty && visible {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        let generation = shortcut.generation;
        let opacity = shortcut.opacity;
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(16),
                    bottom: px(36),
                    max_width: percent(90),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Start,
                    row_gap: px(8),
                    ..default()
                },
                GlobalZIndex(70),
                Root,
                Pickable::IGNORE,
                UiTransform {
                    scale: Vec2::splat(0.94 + 0.06 * opacity),
                    ..default()
                },
            ))
            .with_children(|parent| {
                if shortcut.chooser {
                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: px(6),
                                padding: UiRect::all(px(12)),
                                max_width: px(410),
                                border_radius: BorderRadius::all(px(8)),
                                ..default()
                            },
                            BackgroundColor(INK.with_alpha(opacity)),
                            FadeColor(INK),
                            TabGroup::modal(),
                        ))
                        .with_children(|parent| {
                            label(parent, "Choose how to reveal", 17.0, TEAL, opacity);
                            let first = button(
                                parent,
                                "Reveal 1 · decline Jacqueline for this reveal",
                                Request::Choose(generation, Route::Normal),
                                None,
                                opacity,
                            );
                            parent.commands().entity(first).insert(AutoFocus);
                            button(
                                parent,
                                "Use Jacqueline · reveal 3 tokens",
                                Request::Choose(generation, Route::Jacqueline),
                                None,
                                opacity,
                            );
                            button(
                                parent,
                                "Cancel",
                                Request::Dismiss(generation),
                                None,
                                opacity,
                            );
                            label(parent, feedback::KEYBOARD_HINT, 12.0, PAPER, opacity);
                        });
                }
                let main = button(
                    parent,
                    "Reveal chaos token",
                    Request::Press(generation),
                    artwork.0.clone(),
                    opacity,
                );
                parent.commands().entity(main).insert(Node {
                    width: px(118),
                    min_height: px(72),
                    padding: UiRect::all(px(10)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                });
            });
        shortcut.dirty = false;
    } else if !visible && shortcut.opacity == 0.0 {
        for entity in &roots {
            commands.entity(entity).despawn();
        }
        shortcut.dirty = false;
    }
    for mut transform in &mut transforms {
        transform.scale = Vec2::splat(0.94 + 0.06 * shortcut.opacity);
    }
    for mut opacity in &mut feedback_opacity {
        opacity.0 = shortcut.opacity;
    }
    for (base, text, image, background) in &mut colors {
        let color = base.0.with_alpha(base.0.alpha() * shortcut.opacity);
        if let Some(mut text) = text {
            text.0 = color;
        } else if let Some(mut image) = image {
            image.color = color;
        } else if let Some(mut background) = background {
            background.0 = color;
        }
    }
}

fn request_redraw(shortcut: Res<Shortcut>, mut redraw: ResMut<Redraw>) {
    let target = if shortcut.offer.is_some() { 1.0 } else { 0.0 };
    if shortcut.dirty || shortcut.opacity != target {
        redraw.request();
    }
}
