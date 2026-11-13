//! The group clue payment chooser: a local compound panel beside the Act card,
//! opened from the session's one listed payment choice. Editing the allocation
//! spends nothing; **Spend N clues and advance** commits the session's choice for
//! the complete allocation once, through the ordinary save-before-install path.
use super::*;
use crate::gathering::{Allocation, GroupCluePayment, counted};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{ModalDialogBarrier, RequestClose};

/// The fill of a control that cannot be used now.
const DISABLED: Color = Color::srgb(0.10, 0.12, 0.13);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Input {
    Add(Investigator),
    Remove(Investigator),
    /// Focus an investigator's row, whose stepper the arrow keys then change.
    Row(Investigator),
    Confirm,
    /// The Barrier's other round-end outcome, a rules choice of its own.
    Decline,
    Cancel,
}

/// Whether a listed choice opens this chooser: the session's payment choice
/// with its allocation still to be named.
pub(super) fn opens(id: &str) -> bool {
    GroupCluePayment::is_listed(id)
}

pub(super) struct Chooser {
    revision: u64,
    payment: GroupCluePayment,
    allocation: Allocation,
    focus: Option<Input>,
    /// With agent partners sharing the payment (#208): the investigators whose
    /// clues the human sets. The others' parts are their players' pending ones.
    mine: Option<[bool; 3]>,
}

impl Chooser {
    fn paid(&self) -> u16 {
        self.allocation.total()
    }

    fn complete(&self) -> bool {
        self.paid() == u16::from(self.payment.required)
    }

    fn limit(&self, who: Investigator) -> u8 {
        self.payment.contributors[who.index()].limit(self.payment.required)
    }

    /// Whether the human sets `who`'s clues here.
    fn sets(&self, who: Investigator) -> bool {
        self.mine.is_none_or(|mine| mine[who.index()])
    }

    fn can_add(&self, who: Investigator) -> bool {
        self.sets(who)
            && self.allocation[who] < self.limit(who)
            && self.paid() < u16::from(self.payment.required)
    }

    fn can_remove(&self, who: Investigator) -> bool {
        self.sets(who) && self.allocation[who] > 0
    }

    /// The session's choice for this allocation, once it is complete and legal.
    /// In a shared payment, the human's part with the others' pending ones,
    /// complete or not: it is his readiness.
    fn choice(&self) -> Option<String> {
        if self.mine.is_none() {
            self.payment.choice_for(self.allocation)
        } else {
            self.payment.part_for(self.allocation)
        }
    }

    fn action(&self, table: &Table, input: Input) -> Action {
        Action::CluePayment(table.epoch, self.revision, input)
    }
}

/// The shared payment the human takes part in, while partners share it.
fn shared(table: &Table) -> Option<&crate::partners::SharedState> {
    table.partners.shared.iter().find(|state| {
        matches!(state.kind, crate::partners::SharedKind::Payment { .. })
            && state.of(&crate::partners::Controller::Human).is_some()
    })
}

pub(super) fn open(table: &mut Table, epoch: u64, revision: u64) {
    let view = table.session.observe();
    if epoch != table.epoch || revision != view.revision || table.modal() || table.is_presenting() {
        return;
    }
    let Some(payment) = view.group_clue_payment else {
        return;
    };
    let shared = shared(table).map(|state| {
        let control = table.presentation.control();
        let mine = Investigator::ALL
            .map(|who| control.controller(who) == crate::partners::Controller::Human);
        (mine, state.clues)
    });
    table.clue_payment = Some(Chooser {
        revision,
        // The program never chooses a strategy: only a forced split is pre-filled,
        // or the parts already pending in a shared payment.
        allocation: shared.map_or_else(|| payment.forced.unwrap_or_default(), |(_, clues)| clues),
        payment,
        focus: None,
        mine: shared.map(|(mine, _)| mine),
    });
    table.context.reset();
    table.epoch += 1;
    table.dirty = true;
}

pub(super) fn sync(table: &mut Table) {
    if table
        .clue_payment
        .as_ref()
        .is_some_and(|chooser| chooser.revision != table.session.observe().revision)
    {
        table.clue_payment = None;
        table.epoch += 1;
        table.dirty = true;
        return;
    }
    // Another player's pending part follows its changes.
    let pending = shared(table).map(|state| state.clues);
    if let Some(chooser) = table.clue_payment.as_mut()
        && let (Some(mine), Some(pending)) = (chooser.mine, pending)
    {
        let mut changed = false;
        for who in Investigator::ALL {
            if !mine[who.index()] && chooser.allocation[who] != pending[who] {
                chooser.allocation[who] = pending[who];
                changed = true;
            }
        }
        if changed {
            table.epoch += 1;
            table.dirty = true;
        }
    }
}

/// Whether the player of `who` is ready on the shared payment, while partners
/// share it.
fn ready(table: &Table, who: Investigator) -> Option<bool> {
    shared(table)?
        .players
        .iter()
        .find(|player| player.investigators.contains(&who))
        .map(|player| player.ready)
}

/// Local edits are consumed here. Paying and declining rejoin the adapter's
/// ordinary revision-bound, save-before-install choice path.
pub(super) fn dispatch(table: &mut Table, action: Action) -> Option<Action> {
    let Action::CluePayment(epoch, revision, input) = action else {
        return Some(action);
    };
    if epoch != table.epoch || revision != table.session.observe().revision {
        return None;
    }
    let chooser = table.clue_payment.as_mut()?;
    if chooser.revision != revision {
        return None;
    }
    match input {
        Input::Add(who) if chooser.can_add(who) => chooser.allocation[who] += 1,
        Input::Remove(who) if chooser.can_remove(who) => chooser.allocation[who] -= 1,
        Input::Add(_) | Input::Remove(_) => {}
        // Activating a row (a click, or Enter before the keyboard's payment)
        // only remembers it for the next rebuild; it changes nothing shown.
        Input::Row(who) => {
            if chooser.payment.contributors[who.index()]
                .ineligible
                .is_none()
                && chooser.sets(who)
            {
                chooser.focus = Some(input);
            }
            return None;
        }
        Input::Confirm => {
            let id = chooser.choice()?;
            chooser.focus = Some(Input::Confirm);
            return Some(Action::Choose(epoch, revision, id));
        }
        Input::Decline if chooser.payment.at_round_end => {
            chooser.focus = Some(Input::Decline);
            return Some(Action::Choose(
                epoch,
                revision,
                GroupCluePayment::DECLINE.into(),
            ));
        }
        Input::Decline => return None,
        Input::Cancel => {
            table.clue_payment = None;
            table.epoch += 1;
            table.dirty = true;
            return None;
        }
    }
    chooser.focus = Some(match input {
        Input::Add(who) | Input::Remove(who) => Input::Row(who),
        other => other,
    });
    table.epoch += 1;
    table.dirty = true;
    None
}

/// An investigator's row: Tab moves between rows, the arrow keys change its
/// stepper and Enter pays.
#[derive(Component)]
pub(super) struct Row(Investigator);

pub(super) fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    table: Res<Table>,
    focus: Option<Res<InputFocus>>,
    rows: Query<&Row>,
    mut actions: ResMut<Actions>,
) {
    let Some(chooser) = &table.clue_payment else {
        events.clear();
        return;
    };
    let focused = focus
        .and_then(|focus| focus.get())
        .and_then(|entity| rows.get(entity).ok())
        .map(|row| row.0);
    for event in events.read() {
        if event.state != ButtonState::Pressed || table.inspection.is_some() {
            continue;
        }
        let Some(who) = focused else {
            continue;
        };
        let input = match event.key_code {
            KeyCode::ArrowUp | KeyCode::ArrowRight => Input::Add(who),
            KeyCode::ArrowDown | KeyCode::ArrowLeft => Input::Remove(who),
            KeyCode::Enter | KeyCode::NumpadEnter if !event.repeat => Input::Confirm,
            _ => continue,
        };
        actions.0.push(chooser.action(&table, input));
    }
}

#[derive(Component)]
pub(super) struct Root;

#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<art::Art>,
    images: Res<Assets<Image>>,
    health: Res<RenderHealth>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    roots: Query<Entity, With<Root>>,
    mut previous: Local<Option<(u64, bool, Vec2, String)>>,
) {
    let size = Vec2::new(window.width(), window.height());
    let visible = table.clue_payment.is_some()
        && table.inspection.is_none()
        && health.allows_play()
        && !health.just_became_ready();
    // A partner's readiness changes what the chooser shows without a new epoch.
    let key = (
        table.epoch,
        visible,
        size,
        format!("{}|{:?}", table.notice, shared(&table)),
    );
    if previous.as_ref() == Some(&key) {
        return;
    }
    *previous = Some(key);
    crate::tabletop_ui::release_dialogs_in::<Root>(&mut commands);
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !visible {
        return;
    }
    let chooser = table.clue_payment.as_ref().unwrap();
    let payment = &chooser.payment;
    let width = 560_f32.min((size.x - 24.).max(1.));
    let height = (size.y - 32.).clamp(1., 560.);
    let act =
        layout::scenario_book_card_transform(layout::ScenarioBookSide::Act, Vec2::new(1.35, 1.89))
            .translation;
    let anchor = camera
        .0
        .world_to_viewport(camera.1, act)
        .unwrap_or(size * 0.5);
    let right = anchor.x + 60.;
    let left = (if right + width < size.x - 12. {
        right
    } else {
        anchor.x - width - 60.
    })
    .clamp(12., (size.x - width - 12.).max(12.));
    let top = (anchor.y - height * 0.5).clamp(16., (size.y - height - 16.).max(16.));
    let portraits = ui::portraits(&art, &images);
    let first_row = Investigator::ALL
        .into_iter()
        .find(|who| payment.contributors[who.index()].ineligible.is_none() && chooser.sets(*who));
    let focus = chooser.focus.or(first_row.map(Input::Row));
    let barrier = commands
        .spawn((
            // Only this renderer rebuilds the chooser: the table's rebuild
            // leaves it open.
            Root,
            GlobalZIndex(60),
            ModalDialogBarrier,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .observe(|_: On<RequestClose>, mut actions: ResMut<Actions>| actions.0.push(Action::Close))
        .id();
    let cancel = chooser.action(&table, Input::Cancel);
    commands.entity(barrier).observe(
        move |event: On<PointerClick>, mut actions: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                actions.0.push(cancel.clone());
            }
        },
    );
    commands.entity(barrier).with_children(|root| {
        root.spawn((
            crate::tabletop_ui::DialogFrame,
            TabGroup::modal(),
            AccessibleLabel::new("Group clue payment"),
            BackgroundColor(INK),
            BorderColor::all(GOLD),
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                top: px(top),
                width: px(width),
                max_height: px(height),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .with_children(|panel| {
            panel.spawn(art.text(format!("Advance {}", payment.act_title), 22., GOLD));
            panel.spawn(art.text(
                if payment.at_round_end {
                    "Investigators in the Hallway spend clues as a group."
                } else {
                    "The investigators spend clues as a group."
                },
                14.,
                PAPER,
            ));
            for who in Investigator::ALL {
                row(panel, &art, &table, chooser, who, portraits[who.index()].as_ref(), focus);
            }
            panel.spawn(art.text(
                format!("Paid {} / {} clues", chooser.paid(), payment.required),
                18.,
                if chooser.complete() { GOLD } else { PAPER },
            ));
            if chooser.mine.is_some() {
                panel.spawn(art.text(
                    "Agent partners set their own investigators' clues. Your Spend is your readiness: the clues are spent once every player is ready on exactly the requirement.",
                    13.,
                    PAPER,
                ));
            }
            panel.spawn(art.text(
                "Tab: next investigator · Arrows: change · Enter: spend · Esc: cancel. Nothing is spent until you confirm.",
                12.,
                MUTED,
            ));
            if !table.notice.is_empty() {
                panel.spawn(art.text(&table.notice, 14., GOLD));
            }
            panel
                .spawn(Node { column_gap: px(8), flex_shrink: 0., ..default() })
                .with_children(|buttons| {
                    let (label, input) = if payment.at_round_end {
                        ("Do not advance", Input::Decline)
                    } else {
                        ("Cancel", Input::Cancel)
                    };
                    let other = control(buttons, &art, &table, chooser, label, input, focus);
                    if payment.at_round_end {
                        buttons
                            .commands()
                            .entity(other)
                            .insert(ChoiceControl(GroupCluePayment::DECLINE.into()));
                    }
                    let label = if chooser.mine.is_some() && !chooser.complete() {
                        format!(
                            "Ready with my part ({} / {} clues)",
                            chooser.paid(),
                            payment.required
                        )
                    } else {
                        format!("Spend {} clues and advance", payment.required)
                    };
                    let spend = control(
                        buttons,
                        &art,
                        &table,
                        chooser,
                        &label,
                        Input::Confirm,
                        focus,
                    );
                    match chooser.choice() {
                        Some(id) => {
                            buttons.commands().entity(spend).insert(ChoiceControl(id));
                        }
                        None => {
                            buttons.commands().entity(spend).insert((
                                InteractionDisabled,
                                BackgroundColor(DISABLED),
                            ));
                        }
                    }
                });
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn row(
    panel: &mut ChildSpawnerCommands,
    art: &art::Art,
    table: &Table,
    chooser: &Chooser,
    who: Investigator,
    portrait: Option<&(Handle<Image>, Vec2)>,
    focus: Option<Input>,
) {
    let contributor = &chooser.payment.contributors[who.index()];
    let location = play::location_name(&contributor.location);
    let held = format!(
        "{} held · {location}",
        counted(contributor.clues, "clue", "clues")
    );
    let paid = chooser.allocation[who];
    panel
        .spawn(Node {
            column_gap: px(8),
            align_items: AlignItems::Center,
            flex_shrink: 0.,
            ..default()
        })
        .with_children(|line| {
            // In a shared payment each contribution carries its player's readiness.
            let readiness = match ready(table, who) {
                Some(true) => " · ready",
                Some(false) if !chooser.sets(who) => " · its partner is deciding",
                Some(false) => " · not ready",
                None => "",
            };
            let details = match contributor.ineligible {
                None => format!("{} · {held} · pays {paid}{readiness}", who.name()),
                Some(reason) => format!("{} · {held} · {}", who.name(), reason.reason()),
            };
            let entity = ui::button(art, line, &details, chooser.action(table, Input::Row(who)));
            line.commands().entity(entity).insert((
                Row(who),
                Node {
                    flex_grow: 1.,
                    min_width: px(0),
                    min_height: px(48),
                    padding: UiRect::all(px(6)),
                    column_gap: px(8),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    flex_shrink: 1.,
                    ..default()
                },
            ));
            if let Some((image, dimensions)) = portrait {
                let dimensions = *dimensions * (40. / dimensions.y).min(48. / dimensions.x);
                let preview = line
                    .commands()
                    .spawn((
                        ImageNode::new(image.clone()),
                        Node {
                            width: px(dimensions.x),
                            height: px(dimensions.y),
                            flex_shrink: 0.,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                line.commands()
                    .entity(entity)
                    .insert_children(0, &[preview]);
            }
            // Another player's investigator: its player sets its part.
            if contributor.ineligible.is_some() || !chooser.sets(who) {
                line.commands()
                    .entity(entity)
                    .remove::<TabIndex>()
                    .insert((InteractionDisabled, BackgroundColor(DISABLED)));
                return;
            }
            if focus == Some(Input::Row(who)) {
                line.commands().entity(entity).insert(AutoFocus);
            }
            for (label, input, enabled) in [
                (
                    format!("Remove 1 clue from {}", who.name()),
                    Input::Remove(who),
                    chooser.can_remove(who),
                ),
                (
                    format!("Add 1 clue from {}", who.name()),
                    Input::Add(who),
                    chooser.can_add(who),
                ),
            ] {
                let glyph = if matches!(input, Input::Remove(_)) {
                    "−"
                } else {
                    "+"
                };
                let step = ui::button(art, line, glyph, chooser.action(table, input));
                // The row is the stepper's one Tab stop; the arrows change it.
                line.commands().entity(step).remove::<TabIndex>().insert((
                    AccessibleLabel::new(label),
                    Node {
                        width: px(38),
                        min_height: px(38),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_shrink: 0.,
                        ..default()
                    },
                ));
                if !enabled {
                    line.commands()
                        .entity(step)
                        .insert((InteractionDisabled, BackgroundColor(DISABLED)));
                }
            }
        });
}

fn control(
    parent: &mut ChildSpawnerCommands,
    art: &art::Art,
    table: &Table,
    chooser: &Chooser,
    label: &str,
    input: Input,
    focus: Option<Input>,
) -> Entity {
    let entity = ui::button(art, parent, label, chooser.action(table, input));
    parent.commands().entity(entity).insert(Node {
        flex_basis: px(0),
        flex_grow: 1.,
        min_width: px(0),
        min_height: px(38),
        padding: UiRect::all(px(6)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        flex_shrink: 1.,
        ..default()
    });
    if focus == Some(input) {
        parent.commands().entity(entity).insert(AutoFocus);
    }
    entity
}
