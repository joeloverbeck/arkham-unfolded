//! Table talk at the table (#209; presentation contract, "Agent partners and table
//! talk"): speech boxes beside the rail's portraits, action lines for partners'
//! decisions, the human's message box and **Menu → Table talk**. What may be said,
//! by whom and when is `crate::partners`; the table keeps a copy of the sitting's
//! messages to show them, never saved with the attempt.
//!
//! Speech boxes and action lines are overlays outside the rebuilt panels, so a
//! rebuild never dismisses one. They take no pointer, focus or keyboard input. One
//! that would cover the decision display, the turn indicator, the Test area during
//! a test, a menu or a reading is hidden, and its reading time waits until it shows
//! again. The message box, while open, holds the keyboard: it runs before Bevy
//! dispatches keys to the focused control, keeps that focus on itself and clears
//! the keys the table's own controls read.

use super::*;
use crate::partners::{LONGEST_MESSAGE, Said};
use bevy::{
    camera::visibility::VisibilitySystems,
    input::{
        ButtonState, InputSystems,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::{FocusedInput, InputFocusSystems},
    picking::hover::Hovered,
    ui::UiSystems,
};
use std::collections::VecDeque;

/// The inspection code under which Menu → Table talk lists the messages.
pub(super) const LOG: &str = "table-talk";
/// The rail's button that opens the message box, and its key.
pub(super) const SPEAK: &str = "Speak · T";

/// Above the scene and the rebuilt panels, below every menu (45) and dialog.
const SPEECH_LAYER: i32 = 40;
/// The message box: above speech boxes, below every menu and dialog.
const BOX_LAYER: i32 = 44;
/// The rail's right edge (`tabletop_ui::navigation`: left 10, width 124).
const RAIL_RIGHT: f32 = 134.;
const GAP: f32 = 8.;
const WIDEST: f32 = 260.;
const NARROWEST: f32 = 120.;
/// A speech box or action line stays readable for a base time plus a time per
/// character, about 900 characters a minute, then fades.
const READ_BASE: f32 = 2.;
const READ_PER_CHARACTER: f32 = 0.065;
const FADE: f32 = 0.6;
/// The most recent action lines shown beside one portrait at once.
const LINES_SHOWN: usize = 3;

const SPEECH: Color = Color::srgb(0.93, 0.91, 0.84);

/// How long `text` stays readable before it fades.
pub(super) fn reading_delay(text: &str) -> f32 {
    READ_BASE + READ_PER_CHARACTER * text.chars().count() as f32
}

/// Table talk as the table keeps it, for this sitting only.
#[derive(Default)]
pub(super) struct Talk {
    /// The sitting's messages, as the partner host holds them.
    pub log: Vec<Said>,
    /// Partners' committed decisions, shown as action lines.
    pub lines: Vec<Line>,
    /// Whether the message box is open.
    pub open: bool,
    /// What the human has typed; kept when the box closes without sending.
    pub draft: String,
    /// A message the human sent, until the partner host has said it.
    pub outgoing: Option<String>,
    /// Why the human's last message was not said.
    pub problem: Option<String>,
    /// Why no one may speak now, such as a peril encounter.
    pub closed: Option<&'static str>,
}

/// One partner decision: the label of the option its player chose.
pub(super) struct Line {
    pub by: Investigator,
    pub label: String,
    pub round: Option<u16>,
}

impl Talk {
    pub(super) fn open(&mut self) {
        self.open = true;
        self.problem = None;
    }

    /// The partner host's answer to the human's message.
    pub(super) fn said(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.draft.clear();
                self.problem = None;
                self.open = false;
            }
            Err(reason) => self.problem = Some(reason),
        }
    }

    /// Take the messages said since the last call; whether there were any.
    pub(super) fn hear(&mut self, said: &[Said]) -> bool {
        let new = said.get(self.log.len()..).unwrap_or_default();
        self.log.extend_from_slice(new);
        !new.is_empty()
    }

    /// Menu → Table talk: each message with its round and speaker.
    pub(super) fn entries(&self) -> Vec<String> {
        if self.log.is_empty() {
            return vec!["No one has spoken yet.".into()];
        }
        self.log
            .iter()
            .map(|said| {
                let when = said
                    .round
                    .map_or_else(|| "Setup".to_owned(), |round| format!("Round {round}"));
                format!("{when} · {}: {}", names(&said.speaker), said.text)
            })
            .collect()
    }
}

fn names(investigators: &[Investigator]) -> String {
    investigators
        .iter()
        .map(|who| who.name())
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Names a rail portrait's investigator, so a speech box can find it.
#[derive(Component)]
pub(super) struct Portrait(pub Investigator);

/// A rebuilt panel no speech box may cover: the decision display, the turn indicator.
#[derive(Component)]
pub(super) struct Avoid;

/// The speech boxes and action lines beside one portrait.
#[derive(Component)]
pub(super) struct SpeechSlot(pub Investigator);

/// What one box beside a portrait shows.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// A table-talk message: a rounded box with a tail and quotation marks.
    Speech,
    /// A partner's decision: a square, edged plate, a record rather than speech.
    Line,
}

#[derive(Component)]
pub(super) struct MessageBox;
#[derive(Component)]
struct Draft;
#[derive(Component)]
struct BoxNote;

/// Speech boxes in progress: the messages and lines not yet read, per portrait.
#[derive(Resource, Default)]
pub(super) struct Speech {
    heard: usize,
    noted: usize,
    slots: [Slot; 3],
}

#[derive(Default)]
struct Slot {
    messages: VecDeque<Timed>,
    lines: Vec<Timed>,
    /// Its boxes are spawned and not yet laid out.
    spawning: bool,
    shown: Vec<Shown>,
    /// Whether its boxes showed on the last frame.
    visible: bool,
}

/// A box's text and how long it has been readable.
struct Timed {
    text: String,
    label: String,
    age: f32,
}

impl Timed {
    fn new(text: String, label: String) -> Self {
        Self {
            text,
            label,
            age: 0.,
        }
    }

    fn done(&self, reduced_motion: bool) -> bool {
        self.age >= reading_delay(&self.text) + if reduced_motion { 0. } else { FADE }
    }

    /// Opacity in twentieths: whole while readable, fading after, unless reduced.
    fn opacity(&self, reduced_motion: bool) -> u8 {
        let delay = reading_delay(&self.text);
        if reduced_motion || self.age <= delay {
            return 20;
        }
        ((1. - (self.age - delay) / FADE).clamp(0., 1.) * 20.).round() as u8
    }
}

#[derive(Clone, PartialEq)]
struct Shown {
    kind: Kind,
    text: String,
    label: String,
    opacity: u8,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Speech>()
        .add_systems(
            PreUpdate,
            keyboard
                .after(InputSystems)
                .before(InputFocusSystems::Dispatch),
        )
        .add_systems(Update, message_box.after(partners::serve))
        .add_systems(
            PostUpdate,
            speech
                .after(UiSystems::Layout)
                .before(VisibilitySystems::VisibilityPropagate),
        );
}

/// The message box's keyboard: T opens it, and while it is open every key types
/// into it. Enter sends, Escape closes it without sending and keeps the draft.
fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    keys: Option<ResMut<ButtonInput<KeyCode>>>,
    table: Option<ResMut<Table>>,
    focus: Option<ResMut<InputFocus>>,
    boxes: Query<Entity, With<MessageBox>>,
) {
    let (Some(mut table), Some(mut keys)) = (table, keys) else {
        events.clear();
        return;
    };
    let table = &mut *table;
    let blocked = !table.partners.enabled
        || table.leaving
        || table.menu_open
        || table.modal()
        || table.presentation.announcement().is_some();
    if blocked {
        // A dialog or the menu takes over; the draft waits for the next opening.
        table.talk.open = false;
        events.clear();
        return;
    }
    let modified = [
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]
    .into_iter()
    .any(|key| keys.pressed(key));
    let was_open = table.talk.open;
    for event in events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let talk = &mut table.talk;
        if !talk.open {
            if !event.repeat
                && !modified
                && matches!(&event.logical_key, Key::Character(key) if key.eq_ignore_ascii_case("t"))
            {
                talk.open();
            }
            continue;
        }
        match &event.logical_key {
            Key::Escape => {
                talk.open = false;
                break;
            }
            Key::Enter if !event.repeat => match talk.closed {
                Some(reason) => talk.problem = Some(reason.into()),
                None if !talk.draft.trim().is_empty() => {
                    talk.outgoing = Some(talk.draft.clone());
                }
                None => {}
            },
            Key::Enter => {}
            Key::Backspace => {
                talk.draft.pop();
                talk.problem = None;
            }
            key => {
                let typed = event.text.as_deref().map(str::to_owned).or(match key {
                    Key::Character(text) => Some(text.to_string()),
                    Key::Space => Some(" ".into()),
                    _ => None,
                });
                for character in typed.iter().flat_map(|text| text.chars()) {
                    if !character.is_control() && talk.draft.chars().count() < LONGEST_MESSAGE {
                        talk.draft.push(character);
                        talk.problem = None;
                    }
                }
            }
        }
    }
    if was_open || table.talk.open {
        // Every key typed or closed the box: no table control sees it this frame.
        keys.reset_all();
    }
    if table.talk.open
        && let (Some(mut focus), Ok(entity)) = (focus, boxes.single())
        && focus.get() != Some(entity)
    {
        focus.set(entity, bevy::input_focus::FocusCause::Navigated);
    }
}

/// The message box beside the rail, while open: what has been typed, and why it
/// cannot be sent when it cannot.
fn message_box(
    mut commands: Commands,
    table: Res<Table>,
    boxes: Query<Entity, With<MessageBox>>,
    mut drafts: Query<&mut Text, (With<Draft>, Without<BoxNote>)>,
    mut notes: Query<&mut Text, (With<BoxNote>, Without<Draft>)>,
) {
    let talk = &table.talk;
    if !talk.open {
        for entity in &boxes {
            commands.entity(entity).despawn();
        }
        return;
    }
    let draft = format!("{}|", talk.draft);
    let note = talk
        .closed
        .map(str::to_owned)
        .or_else(|| talk.problem.clone())
        .unwrap_or_default();
    if boxes.is_empty() {
        commands
            .spawn((
                MessageBox,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(RAIL_RIGHT + 16.),
                    top: px(102),
                    width: px(420),
                    padding: UiRect::all(px(10)),
                    border: UiRect::all(px(2)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6),
                    ..default()
                },
                BackgroundColor(INK),
                BorderColor::all(GOLD),
                GlobalZIndex(BOX_LAYER),
                AccessibleLabel::new("Message to the table"),
            ))
            // Keys reach the box and stop there: Tab cannot move focus away.
            .observe(|mut event: On<FocusedInput<KeyboardInput>>| event.propagate(false))
            .with_children(|p| {
                p.spawn(text(
                    "Table talk · Enter sends · Esc closes",
                    12.,
                    MUTED,
                    Pickable::IGNORE,
                ));
                p.spawn((Draft, text(draft, 16., PAPER, Pickable::IGNORE)));
                p.spawn((BoxNote, text(note, 13., GOLD, Pickable::IGNORE)));
            });
        return;
    }
    for mut text in &mut drafts {
        if text.0 != draft {
            text.0.clone_from(&draft);
        }
    }
    for mut text in &mut notes {
        if text.0 != note {
            text.0.clone_from(&note);
        }
    }
}

fn text(value: impl Into<String>, size: f32, color: Color, pickable: Pickable) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        pickable,
    )
}

/// A node's rectangle in UI units.
fn rect(node: &ComputedNode, transform: &UiGlobalTransform) -> Rect {
    let center = transform.translation * node.inverse_scale_factor;
    Rect::from_center_size(center, node.size() * node.inverse_scale_factor)
}

/// Speech boxes and action lines: take what was said and done, place each
/// portrait's boxes beside it clear of what they must not cover, and count their
/// reading time only while they show.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn speech(
    mut commands: Commands,
    table: Option<Res<Table>>,
    time: Res<PresentationTime>,
    mut speech: ResMut<Speech>,
    mut redraw: Option<ResMut<Redraw>>,
    focus: Option<Res<InputFocus>>,
    window: Option<Single<&Window>>,
    portraits: Query<(
        Entity,
        &Portrait,
        &Hovered,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    avoided: Query<(&ComputedNode, &UiGlobalTransform, &InheritedVisibility), With<Avoid>>,
    layers: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            &GlobalZIndex,
        ),
        Without<SpeechSlot>,
    >,
    mut slots: Query<(
        Entity,
        &SpeechSlot,
        &mut Node,
        &ComputedNode,
        &UiGlobalTransform,
        &mut Visibility,
    )>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let Some(table) = table else {
        return;
    };
    let speech = &mut *speech;
    let reduced = table.reduced_motion;
    let talk = &table.talk;
    speech.heard = speech.heard.min(talk.log.len());
    for said in &talk.log[speech.heard..] {
        if let Some(first) = said.speaker.first() {
            speech.slots[first.index()].messages.push_back(Timed::new(
                format!("“{}”", said.text),
                format!("{} says: {}", names(&said.speaker), said.text),
            ));
        }
    }
    speech.heard = talk.log.len();
    speech.noted = speech.noted.min(talk.lines.len());
    for line in &talk.lines[speech.noted..] {
        speech.slots[line.by.index()].lines.push(Timed::new(
            line.label.clone(),
            format!("{}: {}", line.by.name(), line.label),
        ));
    }
    speech.noted = talk.lines.len();

    let play = table
        .last_rendered
        .as_ref()
        .and_then(|view| view.play.as_ref());
    let round = play.map(|play| play.round);
    // What a speech box must not cover, in UI units: the panels it keeps between,
    // and what floats above it or shows the test.
    let panels: Vec<Rect> = avoided
        .iter()
        .filter(|(_, _, visible)| visible.get())
        .map(|(node, transform, _)| rect(node, transform))
        .collect();
    let mut covering: Vec<Rect> = layers
        .iter()
        .filter(|(_, _, visible, layer)| visible.get() && layer.0 > SPEECH_LAYER)
        .map(|(node, transform, ..)| rect(node, transform))
        .collect();
    if play.is_some_and(|play| play.test.is_some() || play.tokens.is_some())
        && let Some(area) = camera
            .single()
            .ok()
            .and_then(|(camera, at)| test_area(camera, at))
    {
        covering.push(area);
    }
    let protected: Vec<Rect> = panels.iter().chain(&covering).copied().collect();
    let height = window.map_or(1000., |window| window.height());

    let mut floor_above = f32::NEG_INFINITY;
    for who in Investigator::ALL {
        let slot = &mut speech.slots[who.index()];
        let portrait = portraits
            .iter()
            .find(|(_, portrait, ..)| portrait.0 == who)
            .map(|(entity, _, hovered, node, transform)| {
                let recalled =
                    hovered.0 || focus.as_ref().and_then(|focus| focus.get()) == Some(entity);
                (rect(node, transform), recalled)
            });
        let recalled = portrait.is_some_and(|(_, recalled)| recalled);
        let content: Vec<Shown> = if recalled {
            recall(talk, who, round)
        } else {
            live(slot, reduced)
        };
        let existing = slots
            .iter()
            .find(|(_, owner, ..)| owner.0 == who)
            .map(|(entity, ..)| entity);
        if content.is_empty() {
            if let Some(entity) = existing {
                commands.entity(entity).despawn();
            }
            slot.shown.clear();
            slot.visible = false;
            slot.spawning = false;
            continue;
        }
        let Some(entity) = existing else {
            if !slot.spawning {
                commands
                    .spawn((
                        SpeechSlot(who),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(RAIL_RIGHT + GAP),
                            top: px(0),
                            width: px(WIDEST),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            row_gap: px(4),
                            ..default()
                        },
                        GlobalZIndex(SPEECH_LAYER),
                        Pickable::IGNORE,
                        Visibility::Hidden,
                    ))
                    .with_children(|p| {
                        for shown in &content {
                            spawn_box(p, shown);
                        }
                    });
                slot.shown = content;
                slot.spawning = true;
            }
            if let Some(redraw) = redraw.as_mut() {
                // The next frame lays the new boxes out.
                redraw.request();
            }
            continue;
        };
        slot.spawning = false;
        if slot.shown != content {
            commands
                .entity(entity)
                .despawn_children()
                .with_children(|p| {
                    for shown in &content {
                        spawn_box(p, shown);
                    }
                });
            slot.shown = content;
        }
        let Ok((_, _, mut node, computed, transform, mut visibility)) = slots.get_mut(entity)
        else {
            continue;
        };
        let actual = rect(computed, transform);
        let mut visible = false;
        if let Some((anchor, _)) = portrait
            && actual.height() > 0.
        {
            let left = RAIL_RIGHT + GAP;
            let size = actual.size();
            // Between the turn indicator and the decision display, and below the
            // boxes of the portraits above.
            let column = Rect::new(left, 0., left + WIDEST, height);
            let ceiling = panel_edge(&panels, column, anchor.center().y, true)
                .max(floor_above + GAP)
                .max(GAP);
            let floor = panel_edge(&panels, column, anchor.center().y, false).min(height - GAP);
            let top = (anchor.center().y - size.y / 2.)
                .min(floor - size.y)
                .max(ceiling);
            let mut widest = WIDEST;
            for area in &covering {
                if area.min.y < top + size.y && area.max.y > top && area.min.x > left {
                    widest = widest.min(area.min.x - GAP - left);
                }
            }
            node.left = px(left);
            node.top = px(top);
            node.width = px(widest.max(NARROWEST));
            let target = Rect::new(left, top, left + size.x, top + size.y);
            let placed = (actual.min - target.min).length() < 1.;
            // A new box waits until it is laid out where it belongs; a shown box
            // stays shown while it moves there, wherever it is clear.
            visible = (placed || slot.visible)
                && widest >= NARROWEST
                && top + size.y <= floor
                && actual.max.y <= height
                && !protected
                    .iter()
                    .any(|area| !area.intersect(actual).is_empty());
            floor_above = floor_above.max(target.max.y);
            if !placed && let Some(redraw) = redraw.as_mut() {
                // The next frame lays the box out where it belongs.
                redraw.request();
            }
        }
        let wanted = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
        slot.visible = visible;
        if visible && !recalled {
            let delta = time.delta_secs();
            if let Some(message) = slot.messages.front_mut() {
                message.age += delta;
            }
            for line in &mut slot.lines {
                line.age += delta;
            }
            if slot
                .messages
                .front()
                .is_some_and(|message| message.done(reduced))
            {
                slot.messages.pop_front();
            }
            slot.lines.retain(|line| !line.done(reduced));
            if let Some(redraw) = redraw.as_mut() {
                // Reading time runs only on the frames that show it.
                redraw.request();
            }
        }
    }
}

/// The nearest edge, less a gap, of a panel across `column`: above `y` when
/// `above`, else below it.
fn panel_edge(panels: &[Rect], column: Rect, y: f32, above: bool) -> f32 {
    let mut edge = if above {
        f32::NEG_INFINITY
    } else {
        f32::INFINITY
    };
    for area in panels {
        let across = area.min.x < column.max.x && area.max.x > column.min.x;
        if !across {
            continue;
        }
        if above && area.max.y <= y {
            edge = edge.max(area.max.y + GAP);
        } else if !above && area.min.y >= y {
            edge = edge.min(area.min.y - GAP);
        }
    }
    edge
}

/// The Test area on screen, while a test shows there.
fn test_area(camera: &Camera, at: &GlobalTransform) -> Option<Rect> {
    let half = layout::TEST_SIZE / 2.;
    let corners = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
        .map(|(x, z)| layout::TEST_CENTER + Vec3::new(x * half.x, 0., z * half.y));
    let points: Vec<Vec2> = corners
        .into_iter()
        .filter_map(|corner| camera.world_to_viewport(at, corner).ok())
        .collect();
    let first = *points.first()?;
    Some(
        points
            .iter()
            .fold(Rect::from_corners(first, first), |area, point| {
                area.union_point(*point)
            }),
    )
}

/// What a portrait shows without the pointer on it: the message being read, and
/// the newest action lines.
fn live(slot: &Slot, reduced: bool) -> Vec<Shown> {
    let lines = slot.lines.len().saturating_sub(LINES_SHOWN);
    slot.messages
        .front()
        .map(|message| (Kind::Speech, message))
        .into_iter()
        .chain(slot.lines[lines..].iter().map(|line| (Kind::Line, line)))
        .map(|(kind, timed)| Shown {
            kind,
            text: timed.text.clone(),
            label: timed.label.clone(),
            opacity: timed.opacity(reduced),
        })
        .collect()
}

/// What hovering or focusing a portrait recalls: its latest message, and its
/// investigator's action lines this round.
fn recall(talk: &Talk, who: Investigator, round: Option<u16>) -> Vec<Shown> {
    let message = talk
        .log
        .iter()
        .rev()
        .find(|said| said.speaker.first() == Some(&who))
        .map(|said| Shown {
            kind: Kind::Speech,
            text: format!("“{}”", said.text),
            label: format!("{} says: {}", names(&said.speaker), said.text),
            opacity: 20,
        });
    message
        .into_iter()
        .chain(
            talk.lines
                .iter()
                .filter(|line| line.by == who && line.round == round)
                .map(|line| Shown {
                    kind: Kind::Line,
                    text: line.label.clone(),
                    label: format!("{}: {}", who.name(), line.label),
                    opacity: 20,
                }),
        )
        .collect()
}

fn spawn_box(p: &mut ChildSpawnerCommands, shown: &Shown) {
    let alpha = f32::from(shown.opacity) / 20.;
    let (background, border, ink, radius) = match shown.kind {
        Kind::Speech => (SPEECH, SPEECH, INK, 10.),
        Kind::Line => (INK, GOLD, PAPER, 0.),
    };
    p.spawn((
        shown.kind,
        Node {
            // Wrap within the column rather than run past it.
            max_width: percent(100),
            padding: UiRect::axes(px(10), px(5)),
            border: UiRect::all(px(if shown.kind == Kind::Line { 2 } else { 1 })),
            border_radius: BorderRadius::all(px(radius)),
            ..default()
        },
        BackgroundColor(background.with_alpha(alpha)),
        BorderColor::all(border.with_alpha(alpha)),
        AccessibleLabel::new(shown.label.clone()),
        Pickable::IGNORE,
    ))
    .with_children(|p| {
        if shown.kind == Kind::Speech {
            // The tail points left, at the portrait.
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-6),
                    top: px(9),
                    width: px(11),
                    height: px(11),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(45.)),
                BackgroundColor(background.with_alpha(alpha)),
                Pickable::IGNORE,
            ));
        }
        p.spawn(text(
            shown.text.clone(),
            14.,
            ink.with_alpha(alpha),
            Pickable::IGNORE,
        ));
    });
}

#[cfg(test)]
mod tests;
