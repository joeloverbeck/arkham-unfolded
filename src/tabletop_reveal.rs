//! One reveal affordance for the tables. Adapters supply current offered choices
//! and commit accepted requests through their existing persistence authority.

use crate::{
    Choice, tabletop_feedback as feedback,
    tabletop_pacing::{PresentationTime, Redraw, RequestRedraw},
};
use bevy::{
    input::mouse::MouseScrollUnit,
    input_focus::{
        AutoFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
};
use std::borrow::Cow;

pub(crate) const ART_ROLE: &str = "ui/reveal-chaos-token";
const INK: Color = Color::srgb(0.035, 0.065, 0.075);
const PAPER: Color = Color::srgb(0.92, 0.90, 0.80);
const TEAL: Color = Color::srgb(0.50, 0.82, 0.75);
const FADE_SECONDS: f32 = 0.14;

#[derive(Resource, Default)]
pub(crate) struct Artwork(pub Option<Handle<Image>>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Offer {
    epoch: u64,
    revision: u64,
    routes: Routes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Routes {
    Legacy { reaction: bool },
    Offered(Vec<Choice>),
}

impl Offer {
    fn needs_chooser(&self) -> bool {
        match &self.routes {
            Routes::Legacy { reaction } => *reaction,
            Routes::Offered(choices) => choices.len() > 1,
        }
    }

    fn choice(&self, route: Route) -> Option<Cow<'static, str>> {
        match (&self.routes, route) {
            (Routes::Legacy { .. }, Route::Normal) => Some(Cow::Borrowed("reveal")),
            (Routes::Legacy { reaction: true }, Route::Jacqueline) => {
                Some(Cow::Borrowed("use-jacqueline"))
            }
            (Routes::Offered(choices), Route::Offered(index)) => choices
                .get(index)
                .map(|choice| Cow::Owned(choice.id.clone())),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Route {
    Normal,
    Jacqueline,
    Offered(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Request {
    Press(u64),
    Choose(u64, Route),
    Dismiss(u64),
}

pub(crate) enum Accepted {
    Opened,
    Choice(u64, Cow<'static, str>),
    Dismissed,
}

#[derive(Resource, Default)]
pub(crate) struct Shortcut {
    caption: Option<String>,
    symbol_font: Handle<Font>,
    offer: Option<Offer>,
    generation: u64,
    chooser: bool,
    attention_cue: bool,
    attention_phase: f32,
    attention_moving: bool,
    rendered_artwork: Option<Handle<Image>>,
    reduced_motion: bool,
    dirty: bool,
    opacity: f32,
    viewport: Option<Vec2>,
    requests: Vec<Request>,
}

impl Shortcut {
    pub(crate) fn attention_cue(&mut self, enabled: bool) {
        if self.attention_cue != enabled {
            self.attention_cue = enabled;
            self.dirty = true;
        }
    }
    pub(crate) fn caption(&mut self, caption: Option<String>, font: Handle<Font>) {
        if self.caption != caption || self.symbol_font != font {
            self.caption = caption;
            self.symbol_font = font;
            self.dirty = true;
        }
    }
    /// Explicit routes supplied by a consumer's current rules observation.
    pub(crate) fn sync_offered(
        &mut self,
        epoch: u64,
        revision: u64,
        choices: &[Choice],
        available: bool,
        reduced_motion: bool,
    ) {
        self.set_offer(
            (available && !choices.is_empty()).then(|| Offer {
                epoch,
                revision,
                routes: Routes::Offered(choices.to_vec()),
            }),
            reduced_motion,
        );
    }
    /// The offered choices are authoritative; a presentation-stage name is not.
    pub(crate) fn sync(
        &mut self,
        revision: u64,
        choices: &[Choice],
        available: bool,
        reduced_motion: bool,
    ) {
        let offer = (available && choices.iter().any(|c| c.id == "reveal")).then(|| Offer {
            epoch: 0,
            revision,
            routes: Routes::Legacy {
                reaction: choices.iter().any(|c| c.id == "use-jacqueline"),
            },
        });
        self.set_offer(offer, reduced_motion);
    }

    fn set_offer(&mut self, offer: Option<Offer>, reduced_motion: bool) {
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
        let offer = self.offer.clone()?;
        match request {
            Request::Press(_) if offer.needs_chooser() => {
                self.dismiss();
                self.chooser = true;
                Some(Accepted::Opened)
            }
            Request::Press(_) => {
                let route = match &offer.routes {
                    Routes::Legacy { .. } => Route::Normal,
                    Routes::Offered(_) => Route::Offered(0),
                };
                let id = offer.choice(route)?;
                self.dismiss();
                Some(Accepted::Choice(offer.revision, id))
            }
            Request::Choose(_, route) if self.chooser && offer.needs_chooser() => {
                let id = offer.choice(route)?;
                self.dismiss();
                Some(Accepted::Choice(offer.revision, id))
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
            .add_systems(Update, offered_keyboard_scroll)
            .add_systems(Last, request_redraw.in_set(RequestRedraw));
    }
}

#[derive(Component)]
pub(crate) struct Root;
#[derive(Component)]
pub(crate) struct FadeColor(Color);
#[derive(Component)]
pub(crate) struct RevealButton;
#[derive(Component)]
pub(crate) struct AttentionIcon;
#[derive(Component)]
struct OfferedMenu;

fn offered_scroll(
    event: On<PointerScroll>,
    mut menus: Query<(&mut ScrollPosition, &ComputedNode), With<OfferedMenu>>,
) {
    if let Ok((mut position, node)) = menus.get_mut(event.entity) {
        let delta = event.y
            * if event.unit == MouseScrollUnit::Line {
                32.
            } else {
                1.
            };
        position.y =
            (position.y - delta).clamp(0., (node.content_size().y - node.size().y).max(0.));
    }
}

fn offered_keyboard_scroll(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    shortcut: Res<Shortcut>,
    mut menus: Query<(&mut ScrollPosition, &ComputedNode), With<OfferedMenu>>,
) {
    let Some(keys) = keys.filter(|_| shortcut.is_open()) else {
        return;
    };
    for (mut position, node) in &mut menus {
        let maximum = (node.content_size().y - node.size().y).max(0.);
        if keys.just_pressed(KeyCode::PageDown) {
            position.y = (position.y + node.size().y * 0.8).min(maximum);
        }
        if keys.just_pressed(KeyCode::PageUp) {
            position.y = (position.y - node.size().y * 0.8).max(0.);
        }
        if keys.just_pressed(KeyCode::End) {
            position.y = maximum;
        }
        if keys.just_pressed(KeyCode::Home) {
            position.y = 0.;
        }
    }
}

fn offered_menu(
    commands: &mut Commands,
    choices: &[Choice],
    generation: u64,
    size: Vec2,
    opacity: f32,
    font: &Handle<Font>,
) {
    use feedback::menu::{Item, MenuText};
    let mut items = vec![
        Item::Text("Choose how to reveal".into(), 17., TEAL),
        Item::Text(feedback::KEYBOARD_HINT.into(), 12., PAPER),
    ];
    items.extend(choices.iter().enumerate().map(|(index, choice)| {
        Item::Action(
            choice.label.clone(),
            Request::Choose(generation, Route::Offered(index)),
        )
    }));
    items.push(Item::Action("Cancel".into(), Request::Dismiss(generation)));
    let menu = feedback::menu::spawn(
        commands,
        feedback::menu::panel(Vec2::new(16., size.y - 150.), size, 80., 130.),
        (
            BackgroundColor(INK.with_alpha(opacity)),
            FadeColor(INK),
            GlobalZIndex(70),
            Root,
            OfferedMenu,
            UiTransform::default(),
        ),
        MenuText {
            symbol_font: font,
            action_size: 16.,
            action_pickable: Pickable::IGNORE,
        },
        items,
        |row, request| {
            row.insert((RevealButton, feedback::FeedbackOpacity(opacity)))
                .observe(move |_: On<Activate>, mut shortcut: ResMut<Shortcut>| {
                    shortcut.requests.push(request)
                });
        },
    );
    commands.entity(menu).observe(offered_scroll);
    fade_text(commands, menu, opacity);
}

fn fade_text(commands: &mut Commands, root: Entity, opacity: f32) {
    commands.queue(move |world: &mut World| {
        let mut descendants = vec![root];
        while let Some(entity) = descendants.pop() {
            if let Some(children) = world.get::<Children>(entity) {
                descendants.extend(children.iter());
            }
            if let Some(color) = world.get::<TextColor>(entity).map(|color| color.0) {
                world.entity_mut(entity).insert((
                    FadeColor(color),
                    TextColor(color.with_alpha(color.alpha() * opacity)),
                ));
            }
        }
    });
}

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
    font: &Handle<Font>,
    centered: bool,
) -> Entity {
    parent
        .spawn((
            Button,
            AccessibleLabel::new(value),
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
            let text = parent
                .spawn((
                    crate::tabletop_harm_text::ui(value, 16., PAPER, font),
                    Pickable::IGNORE,
                ))
                .id();
            if centered {
                parent.commands().entity(text).insert((
                    TextLayout::justify(Justify::Center),
                    Node {
                        max_width: percent(100),
                        ..default()
                    },
                ));
            }
            fade_text(&mut parent.commands(), text, opacity);
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
    mut transforms: Query<&mut UiTransform, (With<Root>, Without<AttentionIcon>)>,
    mut icons: Query<&mut UiTransform, (With<AttentionIcon>, Without<Root>)>,
    mut colors: Query<(
        &FadeColor,
        Option<&mut TextColor>,
        Option<&mut ImageNode>,
        Option<&mut BackgroundColor>,
    )>,
    buttons: Query<Entity, With<RevealButton>>,
    mut feedback_opacity: Query<&mut feedback::FeedbackOpacity, With<RevealButton>>,
    windows: Query<&Window>,
) {
    let viewport = windows
        .iter()
        .next()
        .map_or(Vec2::new(1280., 720.), |window| {
            Vec2::new(window.width(), window.height())
        });
    if shortcut.chooser
        && matches!(
            shortcut.offer.as_ref().map(|offer| &offer.routes),
            Some(Routes::Offered(_))
        )
        && shortcut.viewport != Some(viewport)
    {
        shortcut.viewport = Some(viewport);
        shortcut.dirty = true;
    }
    let visible = shortcut.offer.is_some();
    if shortcut.rendered_artwork != artwork.0 {
        shortcut.rendered_artwork = artwork.0.clone();
        shortcut.dirty = true;
    }
    shortcut.attention_moving = shortcut.attention_cue
        && visible
        && artwork.0.is_some()
        && !shortcut.reduced_motion
        && !shortcut.chooser
        && windows.iter().any(|window| window.focused);
    if shortcut.attention_moving {
        shortcut.attention_phase = (shortcut.attention_phase + time.delta_secs() / 2.4) % 1.0;
    } else {
        shortcut.attention_phase = 0.;
    }
    let bob = -1.5 * (1.0 - (shortcut.attention_phase * std::f32::consts::TAU).cos());
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
        let offered = shortcut
            .offer
            .as_ref()
            .and_then(|offer| match &offer.routes {
                Routes::Offered(choices) => Some(choices),
                Routes::Legacy { .. } => None,
            });
        if shortcut.chooser
            && let Some(choices) = offered
        {
            offered_menu(
                &mut commands,
                choices,
                generation,
                viewport,
                opacity,
                &shortcut.symbol_font,
            );
        }
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
                    scale: Vec2::splat(if shortcut.attention_cue {
                        1.0
                    } else {
                        0.94 + 0.06 * opacity
                    }),
                    ..default()
                },
            ))
            .with_children(|parent| {
                if shortcut.chooser && offered.is_none() {
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
                                &shortcut.symbol_font,
                                false,
                            );
                            parent.commands().entity(first).insert(AutoFocus);
                            button(
                                parent,
                                "Use Jacqueline · reveal 3 tokens",
                                Request::Choose(generation, Route::Jacqueline),
                                None,
                                opacity,
                                &shortcut.symbol_font,
                                false,
                            );
                            button(
                                parent,
                                "Cancel",
                                Request::Dismiss(generation),
                                None,
                                opacity,
                                &shortcut.symbol_font,
                                false,
                            );
                            label(parent, feedback::KEYBOARD_HINT, 12.0, PAPER, opacity);
                        });
                }
                if shortcut.attention_cue
                    && let Some(image) = artwork.0.clone()
                {
                    parent.spawn((
                        ImageNode {
                            color: TEAL.with_alpha(opacity),
                            ..ImageNode::new(image)
                        },
                        Node {
                            width: px(42),
                            height: px(42),
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                        AttentionIcon,
                        UiTransform {
                            translation: Val2::px(0., bob),
                            ..default()
                        },
                        FadeColor(TEAL),
                        Pickable::IGNORE,
                    ));
                }
                let main = button(
                    parent,
                    shortcut.caption.as_deref().unwrap_or("Reveal chaos token"),
                    Request::Press(generation),
                    if shortcut.attention_cue {
                        None
                    } else {
                        artwork.0.clone()
                    },
                    opacity,
                    &shortcut.symbol_font,
                    shortcut.attention_cue,
                );
                parent.commands().entity(main).insert(Node {
                    width: px(if shortcut.caption.is_some() && !shortcut.attention_cue {
                        240
                    } else {
                        118
                    }),
                    min_height: px(72),
                    padding: UiRect::all(px(10)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    align_items: AlignItems::Center,
                    justify_content: if shortcut.attention_cue {
                        JustifyContent::Center
                    } else {
                        default()
                    },
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
        transform.scale = Vec2::splat(if shortcut.attention_cue {
            1.0
        } else {
            0.94 + 0.06 * shortcut.opacity
        });
    }
    for mut transform in &mut icons {
        transform.translation = Val2::px(0., bob);
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
    if shortcut.dirty || shortcut.opacity != target || shortcut.attention_moving {
        redraw.request();
    }
}

#[cfg(test)]
mod offered_tests {
    use super::*;

    fn choice(id: &str, label: &str) -> Choice {
        Choice {
            id: id.into(),
            label: label.into(),
            cancel_faces: vec![],
        }
    }

    fn app(choices: &[Choice]) -> App {
        let mut app = App::new();
        app.add_plugins(RevealPlugin).add_systems(Update, render);
        app.world_mut()
            .resource_mut::<Shortcut>()
            .sync_offered(3, 7, choices, true, true);
        app.update();
        app
    }

    #[test]
    fn known_symbol_caption_spans_follow_the_button_fade() {
        let mut app = app(&[choice("test:reveal", "Reveal")]);
        {
            let mut shortcut = app.world_mut().resource_mut::<Shortcut>();
            shortcut.caption(
                Some("Resolve sealed [tablet], [auto_fail] from Premonition".into()),
                Handle::default(),
            );
            shortcut.reduced_motion = false;
            shortcut.opacity = 0.2;
        }
        app.update();
        let world = app.world_mut();
        let spans = world
            .query_filtered::<(&TextSpan, &TextColor, &FadeColor), With<TextSpan>>()
            .iter(world)
            .collect::<Vec<_>>();
        assert!(spans.iter().any(|(span, _, _)| span.0.contains('o')));
        assert!(spans.iter().any(|(span, _, _)| span.0.contains('q')));
        assert!(
            spans
                .iter()
                .all(|(_, color, _)| (color.0.alpha() - 0.2).abs() < 0.001)
        );
        app.world_mut().resource_mut::<Shortcut>().opacity = 0.8;
        app.update();
        let world = app.world_mut();
        assert!(
            world
                .query_filtered::<&TextColor, With<TextSpan>>()
                .iter(world)
                .all(|color| (color.0.alpha() - 0.8).abs() < 0.001)
        );
    }

    fn click(app: &mut App, label: &str) -> Request {
        let world = app.world_mut();
        let entity = world
            .query_filtered::<(Entity, &Children), With<Button>>()
            .iter(world)
            .find_map(|(entity, children)| {
                children
                    .iter()
                    .any(|child| world.get::<Text>(child).is_some_and(|text| text.0 == label))
                    .then_some(entity)
            })
            .unwrap_or_else(|| panic!("No offered button: {label}"));
        world.trigger(Activate { entity });
        world
            .resource_mut::<Shortcut>()
            .drain()
            .pop()
            .expect("A production button request")
    }

    #[test]
    fn arbitrary_single_offer_keeps_its_canonical_choice_id() {
        let mut app = app(&[choice("test:reveal", "Resolve the declared token reveal")]);
        let request = click(&mut app, "Reveal chaos token");
        let accepted = app.world_mut().resource_mut::<Shortcut>().admit(request);
        assert!(matches!(accepted, Some(Accepted::Choice(7, id)) if id == "test:reveal"));
    }

    #[test]
    fn explicit_requests_expire_on_epoch_changes_and_input_blocking() {
        let choices = [choice("test:reveal", "Reveal")];
        let mut app = app(&choices);
        let stale = click(&mut app, "Reveal chaos token");
        app.world_mut()
            .resource_mut::<Shortcut>()
            .sync_offered(4, 7, &choices, true, true);
        assert!(
            app.world_mut()
                .resource_mut::<Shortcut>()
                .admit(stale)
                .is_none()
        );
        app.update();
        let blocked = click(&mut app, "Reveal chaos token");
        app.world_mut()
            .resource_mut::<Shortcut>()
            .sync_offered(4, 7, &choices, false, true);
        assert!(
            app.world_mut()
                .resource_mut::<Shortcut>()
                .admit(blocked)
                .is_none()
        );
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<Entity, With<RevealButton>>()
                .iter(world)
                .count(),
            0
        );
    }

    #[test]
    fn several_explicit_routes_render_the_authoritys_labels_and_submit_only_the_selected_id() {
        let mut app = app(&[
            choice("test:reveal", "Resolve this declared reveal"),
            choice(
                "tokens:olive:2:47",
                "Exhaust Olive at reveal 2: reveal 3 and resolve 2",
            ),
        ]);
        let press = click(&mut app, "Reveal chaos token");
        assert!(matches!(
            app.world_mut().resource_mut::<Shortcut>().admit(press),
            Some(Accepted::Opened)
        ));
        app.update();
        let request = click(
            &mut app,
            "Exhaust Olive at reveal 2: reveal 3 and resolve 2",
        );
        assert!(
            matches!(app.world_mut().resource_mut::<Shortcut>().admit(request), Some(Accepted::Choice(7, id)) if id == "tokens:olive:2:47")
        );
    }

    #[test]
    fn long_offered_chooser_fits_and_scrolls_to_every_action() {
        use bevy::{
            camera::{ComputedCameraValues, RenderTarget, RenderTargetInfo},
            input::keyboard::{Key, KeyboardInput},
            window::{PrimaryWindow, WindowRef},
        };
        let mut app = crate::tabletop_layout::heading_checks::app();
        app.init_resource::<Assets<StandardMaterial>>()
            .add_plugins((feedback::FeedbackPlugin, RevealPlugin))
            .add_systems(Update, render);
        let size = UVec2::new(420, 320);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: (size.x, size.y).into(),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.world_mut().spawn((
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: size,
                        scale_factor: 1.,
                    }),
                    ..default()
                },
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(window)),
        ));
        let choices: Vec<_> = (0..20)
            .map(|index| {
                choice(
                    &format!("offered:{index}"),
                    &format!("Reveal option {index}: retain this complete descriptive label"),
                )
            })
            .collect();
        app.world_mut()
            .resource_mut::<Shortcut>()
            .sync_offered(3, 7, &choices, true, true);
        app.update();
        let press = click(&mut app, "Reveal chaos token");
        app.world_mut().resource_mut::<Shortcut>().admit(press);
        for _ in 0..3 {
            app.update();
        }
        let world = app.world_mut();
        let (menu, node, transform) = world
            .query_filtered::<(Entity, &ComputedNode, &UiGlobalTransform), With<OfferedMenu>>()
            .single(world)
            .unwrap();
        let bounds = Rect {
            min: transform.transform_point2(-node.size() * 0.5),
            max: transform.transform_point2(node.size() * 0.5),
        };
        assert!(bounds.min.cmpge(Vec2::ZERO).all());
        assert!(bounds.max.cmple(size.as_vec2()).all());
        assert!(node.content_size().y > node.size().y);
        assert_eq!(world.get::<ScrollPosition>(menu).unwrap().y, 0.);
        world.write_message(KeyboardInput {
            key_code: KeyCode::End,
            logical_key: Key::End,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert!(app.world().get::<ScrollPosition>(menu).unwrap().y > 0.);
        let resized = UVec2::new(520, 400);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set_physical_resolution(resized.x, resized.y);
        let world = app.world_mut();
        world
            .query::<&mut Camera>()
            .single_mut(world)
            .unwrap()
            .computed
            .target_info
            .as_mut()
            .unwrap()
            .physical_size = resized;
        for _ in 0..3 {
            app.update();
        }
        let world = app.world_mut();
        let (resized_menu, node, transform) = world
            .query_filtered::<(Entity, &ComputedNode, &UiGlobalTransform), With<OfferedMenu>>()
            .single(world)
            .unwrap();
        assert_ne!(resized_menu, menu, "Resize refreshes the fitted panel");
        let bounds = Rect {
            min: transform.transform_point2(-node.size() * 0.5),
            max: transform.transform_point2(node.size() * 0.5),
        };
        assert!(bounds.min.cmpge(Vec2::ZERO).all());
        assert!(bounds.max.cmple(resized.as_vec2()).all());
        for state in [
            bevy::input::ButtonState::Released,
            bevy::input::ButtonState::Pressed,
        ] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::End,
                logical_key: Key::End,
                state,
                text: None,
                repeat: false,
                window,
            });
            app.update();
        }
        let last = choices.last().unwrap();
        let world = app.world_mut();
        let (node, transform) = world
            .query_filtered::<(&AccessibleLabel, &ComputedNode, &UiGlobalTransform), With<Button>>()
            .iter(world)
            .find_map(|(label, node, transform)| {
                (label.0 == last.label).then_some((node, transform))
            })
            .unwrap();
        let last_bounds = Rect {
            min: transform.transform_point2(-node.size() * 0.5),
            max: transform.transform_point2(node.size() * 0.5),
        };
        assert!(last_bounds.min.y >= bounds.min.y);
        assert!(last_bounds.max.y <= bounds.max.y);
        let request = click(&mut app, &last.label);
        assert!(
            matches!(app.world_mut().resource_mut::<Shortcut>().admit(request), Some(Accepted::Choice(7, id)) if id == last.id)
        );
    }
}
