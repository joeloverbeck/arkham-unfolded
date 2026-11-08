//! Crystal Pendulum's local picker. Only confirming a number submits a rule choice.
use super::*;
use crate::gathering::PendulumPrompt;
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput},
};

#[derive(Clone)]
pub(super) enum Input {
    Open,
    Other,
    Predict(u16),
}

pub(super) struct Picker {
    prompt: PendulumPrompt,
    revision: u64,
    other: Option<String>,
}

impl Picker {
    fn new(prompt: PendulumPrompt, revision: u64) -> Self {
        Self {
            prompt,
            revision,
            other: prompt.saved_number.map(|number| number.to_string()),
        }
    }

    pub(super) fn saved(session: &Session) -> Option<Self> {
        session
            .pendulum_prompts()
            .into_iter()
            .find(|prompt| prompt.saved_number.is_some())
            .map(|prompt| Self::new(prompt, session.observe().revision))
    }

    fn action(&self, epoch: u64, input: Input) -> Action {
        Action::Prediction(epoch, self.revision, self.prompt.asset, input)
    }
}

pub(super) fn open(table: &mut Table, epoch: u64, revision: u64, asset: u16) {
    if epoch != table.epoch
        || revision != table.session.observe().revision
        || table.is_presenting()
        || table.modal()
    {
        return;
    }
    if let Some(prompt) = table
        .session
        .pendulum_prompts()
        .into_iter()
        .find(|prompt| prompt.asset == asset)
    {
        table.prediction = Some(Picker::new(prompt, revision));
        table.context.reset();
        table.dirty = true;
    }
}

/// Local controls are consumed here. A valid confirmation rejoins the adapter's
/// ordinary revision-bound, save-before-install choice path.
pub(super) fn dispatch(table: &mut Table, action: Action) -> Option<Action> {
    let Action::Prediction(epoch, revision, asset, input) = action else {
        return Some(action);
    };
    if matches!(input, Input::Open) {
        open(table, epoch, revision, asset);
        return None;
    }
    let picker = table.prediction.as_mut()?;
    if epoch != table.epoch || revision != picker.revision || asset != picker.prompt.asset {
        return None;
    }
    match input {
        Input::Other => {
            picker.other = Some(String::new());
            table.dirty = true;
            None
        }
        Input::Predict(number) => {
            table.prediction = None;
            Some(Action::Choose(
                epoch,
                revision,
                format!("test-card:predict:{asset}:{number}"),
            ))
        }
        Input::Open => unreachable!(),
    }
}

pub(super) fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    mut table: ResMut<Table>,
    mut actions: ResMut<Actions>,
) {
    for event in events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let epoch = table.epoch;
        let Some(picker) = table.prediction.as_mut() else {
            continue;
        };
        let Some(text) = picker.other.as_mut() else {
            continue;
        };
        match &event.logical_key {
            Key::Character(digits) if digits.chars().all(|c| c.is_ascii_digit()) => {
                // Keep invalid/overflow input visible; never clamp it into a
                // different prediction. Six digits suffice to show overflow.
                if text.len() + digits.len() <= 6 {
                    text.push_str(digits);
                }
            }
            Key::Backspace => {
                text.pop();
            }
            Key::Delete => text.clear(),
            Key::Enter if !event.repeat => {
                if let Ok(number) = text.parse::<u16>() {
                    actions.0.push(picker.action(epoch, Input::Predict(number)));
                }
            }
            _ => continue,
        }
        table.dirty = true;
    }
}

pub(super) fn render(commands: &mut Commands, table: &Table, art: &art::Art) {
    use bevy::ui_widgets::{ModalDialog, ModalDialogBarrier};
    let Some(picker) = &table.prediction else {
        return;
    };
    commands
        .spawn((
            Panel,
            GlobalZIndex(100),
            ModalDialogBarrier,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    ModalDialog,
                    TabGroup::modal(),
                    BackgroundColor(INK),
                    BorderColor::all(GOLD),
                    Node {
                        width: percent(90),
                        max_width: px(460),
                        padding: UiRect::all(px(24)),
                        border: UiRect::all(px(2)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        ..default()
                    },
                ))
                .with_children(|panel| {
                    panel.spawn(art.text("Exhaust Crystal Pendulum", 24., GOLD));
                    panel.spawn(art.text(picker.prompt.owner.name(), 16., PAPER));
                    if let Some(number) = picker.prompt.saved_number {
                        panel.spawn(art.text(format!("Saved prediction: {number}"), 16., PAPER));
                    }
                    panel
                        .spawn(Node {
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: px(8),
                            row_gap: px(8),
                            ..default()
                        })
                        .with_children(|grid| {
                            for number in 0..=10 {
                                let button = ui::button(
                                    art,
                                    grid,
                                    &format!("Predict {number}"),
                                    picker.action(table.epoch, Input::Predict(number)),
                                );
                                grid.commands().entity(button).entry::<Node>().and_modify(
                                    |mut node| {
                                        node.width = percent(30);
                                        node.flex_grow = 1.;
                                    },
                                );
                                if number == 0 && picker.other.is_none() {
                                    grid.commands().entity(button).insert(AutoFocus);
                                }
                            }
                        });
                    ui::button(
                        art,
                        panel,
                        "Other margin…",
                        picker.action(table.epoch, Input::Other),
                    );
                    if let Some(text) = &picker.other {
                        panel
                            .spawn(art.text(
                                format!("Margin: {}", if text.is_empty() { "▏" } else { text }),
                                22.,
                                PAPER,
                            ))
                            .insert((
                                AccessibleLabel::new("Prediction number input"),
                                bevy::input_focus::tab_navigation::TabIndex(0),
                                AutoFocus,
                            ));
                        match text.parse::<u16>() {
                            Ok(number) if number > 10 => {
                                ui::button(
                                    art,
                                    panel,
                                    &format!("Predict {number}"),
                                    picker.action(table.epoch, Input::Predict(number)),
                                );
                            }
                            Ok(_) => {}
                            Err(_) => {
                                panel.spawn(art.text(
                                    "Type a whole number from 0 to 65535.",
                                    15.,
                                    MUTED,
                                ));
                            }
                        }
                        panel.spawn(art.text(
                            "Enter confirms · Backspace edits · Delete clears",
                            13.,
                            MUTED,
                        ));
                    }
                    ui::button(art, panel, "Dismiss", Action::Close);
                });
        });
}
