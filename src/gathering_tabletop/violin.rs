//! Local composition of one already-offered Violin choice; costs remain atomic.
use super::*;
use crate::gathering::CardView;
use bevy::{input_focus::tab_navigation::TabIndex, ui::InteractionDisabled};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Benefit {
    Draw,
    Resource,
}

impl Benefit {
    fn label(self) -> &'static str {
        match self {
            Self::Draw => "Draw 1 card",
            Self::Resource => "Gain 1 resource",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Input {
    Discard(u16),
    Recipient(Investigator),
    Benefit(Benefit),
    Inspect(u16),
    Confirm,
    Cancel,
}

struct Offered {
    id: String,
    asset: u16,
    discard: u16,
    recipient: Investigator,
    benefit: Benefit,
}

fn offered(id: &str) -> Option<Offered> {
    let parts: Vec<_> = id.split(':').collect();
    let [
        "violin",
        asset,
        "discard",
        discard,
        "target",
        recipient,
        benefit,
    ] = parts.as_slice()
    else {
        return None;
    };
    Some(Offered {
        id: id.into(),
        asset: asset.parse().ok()?,
        discard: discard.parse().ok()?,
        recipient: Investigator::ALL
            .into_iter()
            .find(|who| who.id() == *recipient)?,
        benefit: match *benefit {
            "draw" => Benefit::Draw,
            "resource" => Benefit::Resource,
            _ => return None,
        },
    })
}

pub(super) fn asset(id: &str) -> Option<u16> {
    offered(id).map(|choice| choice.asset)
}

pub(super) struct Picker {
    revision: u64,
    anchor: Option<Vec3>,
    owner: Investigator,
    cards: Vec<CardView>,
    offered: Vec<Offered>,
    discard: Option<u16>,
    recipient: Option<Investigator>,
    benefit: Option<Benefit>,
    focus: Input,
}

impl Picker {
    fn choice(&self) -> Option<&str> {
        self.offered.iter().find_map(|choice| {
            (self.discard == Some(choice.discard)
                && self.recipient == Some(choice.recipient)
                && self.benefit == Some(choice.benefit))
            .then_some(choice.id.as_str())
        })
    }

    fn card_label(&self, card: &CardView) -> String {
        let copies: Vec<_> = self
            .cards
            .iter()
            .filter(|other| other.code == card.code)
            .collect();
        if copies.len() > 1 {
            let index = copies.iter().position(|other| other.id == card.id).unwrap() + 1;
            format!("{} · {index} of {}", card.title, copies.len())
        } else {
            card.title.clone()
        }
    }

    fn action(&self, table: &Table, input: Input) -> Action {
        Action::Violin(table.epoch, self.revision, input)
    }
}

pub(super) fn open(table: &mut Table, epoch: u64, revision: u64, asset: u16) {
    let view = table.session.observe();
    if epoch != table.epoch || revision != view.revision || table.modal() || table.is_presenting() {
        return;
    }
    let offered: Vec<_> = view
        .choices
        .iter()
        .filter_map(|choice| offered(&choice.id))
        .filter(|choice| choice.asset == asset)
        .collect();
    if offered.is_empty() {
        return;
    }
    let Some(player) = view.play.as_ref().and_then(|play| {
        play.players.iter().find(|player| {
            player
                .assets
                .iter()
                .any(|candidate| candidate.card.id == asset)
        })
    }) else {
        return;
    };
    let cards: Vec<_> = player
        .hand
        .iter()
        .filter(|card| offered.iter().any(|choice| choice.discard == card.id))
        .cloned()
        .collect();
    let selected = table.context.selection();
    let discard = selected.and_then(|selection| match selection.source {
        controls::Source::Card(id) if cards.iter().any(|card| card.id == id) => Some(id),
        _ => None,
    });
    table.violin = Some(Picker {
        revision,
        anchor: selected.map(|selection| selection.at),
        owner: player.investigator,
        focus: cards
            .first()
            .map_or(Input::Cancel, |card| Input::Discard(card.id)),
        cards,
        offered,
        discard,
        recipient: None,
        benefit: None,
    });
    table.context.reset();
    table.epoch += 1;
    table.dirty = true;
}

pub(super) fn sync(table: &mut Table) {
    if table
        .violin
        .as_ref()
        .is_some_and(|picker| picker.revision != table.session.observe().revision)
    {
        table.violin = None;
        table.epoch += 1;
        table.dirty = true;
    }
}

pub(super) fn dispatch(table: &mut Table, action: Action) -> Option<Action> {
    let Action::Violin(epoch, revision, input) = action else {
        return Some(action);
    };
    if epoch != table.epoch
        || revision != table.session.observe().revision
        || table.inspection.is_some()
    {
        return None;
    }
    let picker = table.violin.as_mut()?;
    if picker.revision != revision {
        return None;
    }
    match input {
        Input::Discard(id) if picker.cards.iter().any(|card| card.id == id) => {
            picker.discard = Some(id)
        }
        Input::Recipient(who) if picker.offered.iter().any(|choice| choice.recipient == who) => {
            picker.recipient = Some(who)
        }
        Input::Benefit(benefit)
            if picker
                .offered
                .iter()
                .any(|choice| choice.benefit == benefit) =>
        {
            picker.benefit = Some(benefit)
        }
        Input::Inspect(id) => {
            let card = picker.cards.iter().find(|card| card.id == id)?;
            picker.focus = input;
            return Some(Action::Inspect(card.title.clone(), card.code.clone()));
        }
        Input::Confirm => {
            let id = picker.choice()?.to_owned();
            picker.focus = Input::Confirm;
            return Some(Action::Choose(epoch, revision, id));
        }
        Input::Cancel => {
            table.violin = None;
            table.epoch += 1;
            table.dirty = true;
            return None;
        }
        _ => return None,
    }
    picker.focus = input;
    table.epoch += 1;
    table.dirty = true;
    None
}

#[derive(Component)]
pub(super) struct Root;
#[derive(Component)]
pub(super) struct Cards;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn render(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<art::Art>,
    images: Res<Assets<Image>>,
    health: Res<RenderHealth>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    roots: Query<Entity, With<Root>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut scrolling: Query<(&mut ScrollPosition, &ComputedNode), With<Cards>>,
    mut card_scroll: Local<ScrollPosition>,
    mut previous: Local<Option<(u64, u64, bool, bool, Vec2, String)>>,
) {
    let size = Vec2::new(window.width(), window.height());
    let visible = table.violin.is_some()
        && table.inspection.is_none()
        && health.allows_play()
        && !health.just_became_ready();
    let key = (
        table.epoch,
        table.violin.as_ref().map_or(0, |picker| picker.revision),
        table.violin.is_some(),
        visible,
        size,
        table.notice.clone(),
    );
    if previous.as_ref() == Some(&key) {
        if visible {
            for (mut scroll, node) in &mut scrolling {
                scroll_with_keys(&keys, &mut scroll, node);
            }
        }
        return;
    }
    if let Ok((scroll, _)) = scrolling.single() {
        **card_scroll = **scroll;
    }
    if table.violin.is_none() {
        **card_scroll = Vec2::ZERO;
    }
    *previous = Some(key);
    for root in &roots {
        commands.entity(root).despawn();
    }
    if !visible {
        return;
    }
    let picker = table.violin.as_ref().unwrap();
    let (epoch, revision) = (table.epoch, picker.revision);
    let width = 650_f32.min((size.x - 24.).max(1.));
    let height = (size.y - 32.).clamp(1., 680.);
    let anchor = picker
        .anchor
        .and_then(|at| camera.0.world_to_viewport(camera.1, at).ok())
        .unwrap_or(size * 0.5);
    let right = anchor.x + 18.;
    let left = (if right + width < size.x - 12. {
        right
    } else {
        anchor.x - width - 18.
    })
    .clamp(12., (size.x - width - 12.).max(12.));
    let top = (anchor.y - height * 0.5).clamp(16., (size.y - height - 16.).max(16.));
    commands.spawn((Root, GlobalZIndex(60), Node {
        position_type: PositionType::Absolute, width: percent(100), height: percent(100), ..default()
    })).observe(move |event: On<PointerClick>, mut actions: ResMut<Actions>| {
        if event.button == bevy::picking::pointer::PointerButton::Primary {
            actions.0.push(Action::Violin(epoch, revision, Input::Cancel));
        }
    }).with_children(|root| {
        root.spawn((TabGroup::modal(), BackgroundColor(INK), BorderColor::all(GOLD), Node {
            position_type: PositionType::Absolute, left: px(left), top: px(top), width: px(width), max_height: px(height),
            padding: UiRect::all(px(14)), border: UiRect::all(px(2)), flex_direction: FlexDirection::Column,
            row_gap: px(8), ..default()
        })).observe(|mut event: On<PointerClick>| event.propagate(false)).with_children(|panel| {
            panel.spawn(art.text("Use Patrice's Violin", 22., GOLD));
            panel.spawn(art.text(format!("{} · Exhaust Violin and discard 1 card", picker.owner.name()), 14., PAPER));
            panel.spawn(art.text("Discard", 16., GOLD));
            panel.spawn((Cards, card_scroll.clone(), Node {
                flex_wrap: FlexWrap::Wrap, min_height: px(56), max_height: px((height - 360.).max(56.)),
                overflow: Overflow::scroll_y(), padding: UiRect::all(px(4)), column_gap: px(8), row_gap: px(8), ..default()
            })).observe(scroll_panel).with_children(|cards| {
                for card in &picker.cards {
                    let label = picker.card_label(card);
                    cards.spawn(Node { width: px(112), flex_direction: FlexDirection::Column, row_gap: px(4), flex_shrink: 0., ..default() })
                        .with_children(|column| {
                            let selected = picker.discard == Some(card.id);
                            let entity = option(column, &art, &table, picker, &label, Input::Discard(card.id), selected);
                            if let Some(image) = art.card(&card.code, false) {
                                let preview = column.commands().spawn((ImageNode::new(image), Node {
                                    width: px(72), height: px(100), ..default()
                                }, Pickable::IGNORE)).id();
                                column.commands().entity(entity).insert_children(0, &[preview]);
                            }
                            option(column, &art, &table, picker, &format!("Inspect {label}"), Input::Inspect(card.id), false);
                        });
                }
            });
            panel.spawn(art.text("Recipient", 16., GOLD));
            let portraits = ui::portraits(&art, &images);
            panel.spawn(Node { column_gap: px(8), ..default() }).with_children(|row| {
                for who in Investigator::ALL {
                    if !picker.offered.iter().any(|choice| choice.recipient == who) { continue; }
                    let entity = option(row, &art, &table, picker, who.name(), Input::Recipient(who), picker.recipient == Some(who));
                    row.commands().entity(entity).insert(Node { flex_basis: px(0), flex_grow: 1., min_width: px(0),
                        min_height: px(38), padding: UiRect::all(px(6)), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: px(4), ..default() });
                    if let Some((image, dimensions)) = &portraits[who.index()] {
                        let dimensions = *dimensions * (60. / dimensions.y).min(70. / dimensions.x);
                        let preview = row.commands().spawn((ImageNode::new(image.clone()), Node {
                            width: px(dimensions.x), height: px(dimensions.y), ..default()
                        }, Pickable::IGNORE)).id();
                        row.commands().entity(entity).insert_children(0, &[preview]);
                    }
                }
            });
            panel.spawn(art.text("Benefit", 16., GOLD));
            panel.spawn(Node { column_gap: px(8), ..default() }).with_children(|row| {
                for benefit in [Benefit::Draw, Benefit::Resource] {
                    option(row, &art, &table, picker, benefit.label(), Input::Benefit(benefit), picker.benefit == Some(benefit));
                }
            });
            let summary = if let (Some(id), Some(who), Some(benefit)) = (picker.discard, picker.recipient, picker.benefit) {
                let card = picker.cards.iter().find(|card| card.id == id).unwrap();
                format!("Exhaust Violin; discard {}. {}: {}.", picker.card_label(card), who.name(), benefit.label().to_lowercase())
            } else { "Choose a discard card, recipient and benefit. Nothing is spent until you confirm.".into() };
            panel.spawn(art.text(summary, 14., PAPER));
            if !table.notice.is_empty() { panel.spawn(art.text(&table.notice, 14., GOLD)); }
            panel.spawn(Node { column_gap: px(8), flex_shrink: 0., ..default() }).with_children(|row| {
                option(row, &art, &table, picker, "Cancel", Input::Cancel, false);
                let confirm = option(row, &art, &table, picker, "Use Violin", Input::Confirm, false);
                if picker.choice().is_none() {
                    row.commands().entity(confirm).insert((InteractionDisabled, BackgroundColor(Color::srgb(0.10, 0.12, 0.13))));
                }
            });
        });
    });
}

fn option(
    parent: &mut ChildSpawnerCommands,
    art: &art::Art,
    table: &Table,
    picker: &Picker,
    label: &str,
    input: Input,
    selected: bool,
) -> Entity {
    let entity = ui::button(art, parent, label, picker.action(table, input));
    parent.commands().entity(entity).insert(Node {
        width: percent(100),
        min_width: px(0),
        min_height: px(38),
        padding: UiRect::all(px(6)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(4),
        flex_shrink: 1.,
        ..default()
    });
    if selected {
        parent
            .commands()
            .entity(entity)
            .with_child((art.text("Selected", 12., GOLD), Pickable::IGNORE));
    }
    if picker.focus == input {
        parent
            .commands()
            .entity(entity)
            .insert((AutoFocus, TabIndex(0)));
    }
    entity
}
