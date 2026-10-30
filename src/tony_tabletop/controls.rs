//! Selection is transient presentation state; only a complete offered choice reaches rules.
use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Card(usize),
    Hand(u8),
    Committed(u8),
    Deck,
    Discard,
    Bag,
    Token,
}
#[derive(Clone, Copy)]
pub(super) struct Selection {
    pub source: Source,
    pub at: Vec3,
}
#[derive(Default)]
pub(super) struct Context {
    pub selected: Option<Selection>,
    pub open: bool,
    pub epoch: u64,
    pub commitments: BTreeSet<u8>,
    pub utilities: bool,
    stamp: Option<(u64, Option<usize>)>,
}
impl Context {
    pub fn dismiss(&mut self) {
        self.open = false;
        self.epoch = self.epoch.wrapping_add(1);
    }
    pub fn sync(&mut self, revision: u64, replay: Option<usize>) -> bool {
        let stamp = Some((revision, replay));
        if self.stamp == stamp {
            return false;
        }
        self.stamp = stamp;
        self.dismiss();
        self.selected = None;
        self.commitments.clear();
        true
    }
    pub fn select(&mut self, source: Source, at: Vec3) {
        self.dismiss();
        self.selected = Some(Selection { source, at });
        self.open = true;
        self.utilities = false;
    }
    pub fn accepts(&self, epoch: u64) -> bool {
        self.open && epoch == self.epoch
    }
}

pub(super) fn invalidate(mut table: ResMut<Table>) {
    let view = table.view();
    let replay = table.replay.as_ref().map(Replay::index);
    if table.context.sync(view.revision, replay) {
        table.dirty = true;
    }
    if table
        .context
        .selected
        .is_some_and(|selection| !exists(&view, selection.source))
    {
        table.context.dismiss();
        table.context.selected = None;
        table.dirty = true;
    }
}
pub(super) fn exists(view: &Observation, source: Source) -> bool {
    match source {
        Source::Card(2) => view.ghoul.alive,
        Source::Card(4) => view.injury,
        Source::Card(index) => index < CARDS.len(),
        Source::Hand(id) => view.hand.iter().any(|card| card.id == id),
        Source::Committed(id) => view.committed.iter().any(|card| card.id == id),
        Source::Token => view.held_token.is_some(),
        Source::Discard => !view.discard.is_empty(),
        Source::Deck => view.deck_count > 0,
        Source::Bag => true,
    }
}
pub(super) fn source_choices(view: &Observation, source: Source) -> Vec<&crate::Choice> {
    view.choices
        .iter()
        .filter(|choice| match source {
            Source::Card(0) => choice.id == "resource",
            Source::Card(1) => choice.id == "investigate",
            Source::Card(2) => matches!(choice.id.as_str(), "fight" | "evade" | "engage"),
            Source::Card(4) => choice.id == "injury",
            Source::Deck => choice.id == "draw",
            Source::Bag => choice.id == "reveal",
            Source::Token => choice.id == "resolve",
            _ => false,
        })
        .collect()
}
pub(super) fn committable(view: &Observation, id: u8) -> bool {
    view.hand.iter().any(|card| card.id == id)
        && view
            .choices
            .iter()
            .any(|choice| choice.id == format!("commit:{id}"))
}
pub(super) fn commitment_choice(view: &Observation, selected: &BTreeSet<u8>) -> Option<String> {
    if selected.is_empty() {
        return None;
    }
    // Match sets against the offered choice IDs; do not assume an engine ordering.
    view.choices.iter().find_map(|choice| {
        let ids = choice.id.strip_prefix("commit:")?;
        let ids = ids
            .split(',')
            .map(str::parse::<u8>)
            .collect::<Result<BTreeSet<_>, _>>()
            .ok()?;
        (ids == *selected).then(|| choice.id.clone())
    })
}

pub(super) fn commitment_bonus(view: &Observation, count: usize) -> Option<String> {
    use crate::{tabletop::symbols::Skill, tony::TestKind};
    let skill = match view.test.as_ref()?.kind {
        TestKind::Fight => Skill::Combat,
        TestKind::Evade => Skill::Agility,
        TestKind::Investigate => Skill::Intellect,
    };
    // This bounded turn's only committable cards are On the Hunt: one icon each.
    Some(format!("+{count} {}", skill.name()))
}
fn inspect_index(view: &Observation, source: Source) -> Option<usize> {
    match source {
        Source::Card(index) => Some(index),
        Source::Hand(id) | Source::Committed(id) => view
            .hand
            .iter()
            .chain(&view.committed)
            .find(|card| card.id == id)
            .and_then(|card| CARDS.iter().position(|face| face.code == card.code)),
        Source::Token | Source::Bag => Some(7),
        _ => None,
    }
}
fn source_name(view: &Observation, source: Source) -> String {
    match source {
        Source::Card(index) => CARDS[index].title.into(),
        Source::Hand(id) | Source::Committed(id) => view
            .hand
            .iter()
            .chain(&view.committed)
            .find(|card| card.id == id)
            .map_or_else(
                || "Card".into(),
                |card| format!("{} · copy {}", card.title, id + 1),
            ),
        Source::Deck => format!("Tony's deck · {} cards", view.deck_count),
        Source::Discard => format!("Tony's discard · {} cards", view.discard.len()),
        Source::Bag => format!("Chaos bag · {} tokens", view.bag_count),
        Source::Token => "Revealed chaos token".into(),
    }
}
#[derive(Component)]
pub(super) struct Menu;
struct Entry {
    label: String,
    action: Action,
    slot: usize,
    warning: Option<String>,
}
fn entries(table: &Table, view: &Observation, source: Source) -> Vec<Entry> {
    let mut entries = Vec::new();
    if table.replay.is_none() && !table.presenting() {
        for choice in source_choices(view, source) {
            let (short, slot) = match choice.id.as_str() {
                "fight" => ("Fight · bonus/1 action", 0),
                "evade" => ("Evade · 1 action", 1),
                "engage" => ("Engage · bonus/1 action", 2),
                "resource" => ("Gain resource · 1 action", 0),
                "draw" => ("Draw · 1 action", 0),
                "investigate" => ("Investigate · 1 action", 0),
                "injury" => ("Remove Injury · 2 actions", 0),
                "reveal" => ("Reveal token", 0),
                "resolve" => ("Resolve test", 0),
                _ => continue,
            };
            let warning = view.opportunity_attack(&choice.id).map(|hit| {
                format!(
                    "Attack of opportunity: Ghoul Minion\n{} damage + {} horror",
                    hit.damage, hit.horror
                )
            });
            let label = if matches!(choice.id.as_str(), "fight" | "engage") {
                format!(
                    "{} · {}",
                    if choice.id == "fight" {
                        "Fight"
                    } else {
                        "Engage"
                    },
                    if view.bonus_action && view.ghoul.bounties > 0 {
                        "bonus action"
                    } else {
                        "1 action"
                    }
                )
            } else {
                short.into()
            };
            entries.push(Entry {
                label,
                action: Action::ContextChoice(
                    table.context.epoch,
                    view.revision,
                    choice.id.clone(),
                ),
                slot,
                warning,
            });
        }
        if let Source::Hand(id) = source
            && committable(view, id)
            && let Some(bonus) = commitment_bonus(view, 1)
        {
            entries.push(Entry {
                label: if table.context.commitments.contains(&id) {
                    "Unselect commitment".into()
                } else {
                    format!("Select to commit {bonus}")
                },
                action: Action::ToggleCommit(table.context.epoch, view.revision, id),
                slot: 0,
                warning: None,
            });
        }
    }
    if let Some(index) = inspect_index(view, source) {
        entries.push(Entry {
            label: "Inspect".into(),
            action: Action::Inspect(index),
            slot: 3,
            warning: None,
        });
    }
    if source == Source::Discard {
        // The physical pile preserves order; this browser exposes each known instance.
        for card in view.discard.iter().rev() {
            if let Some(index) = CARDS.iter().position(|face| face.code == card.code) {
                entries.push(Entry {
                    label: format!("Inspect {} · copy {}", card.title, card.id + 1),
                    action: Action::Inspect(index),
                    slot: 3,
                    warning: None,
                });
            }
        }
    }
    entries
}

pub(super) fn inspection_only(table: &Table, view: &Observation, source: Source) -> Option<usize> {
    if !matches!(
        source,
        Source::Card(_) | Source::Hand(_) | Source::Committed(_)
    ) {
        return None;
    }
    match entries(table, view, source).as_slice() {
        [
            Entry {
                action: Action::Inspect(index),
                ..
            },
        ] => Some(*index),
        _ => None,
    }
}

type MenuSnapshot = (u64, u64, bool, Vec2, bool, bool, BTreeSet<u8>);

pub(super) fn draw_menu(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<Art>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    window: Single<&Window>,
    menus: Query<Entity, With<Menu>>,
    mut previous: Local<Option<MenuSnapshot>>,
) {
    let size = Vec2::new(window.width(), window.height());
    let key = (
        table.context.epoch,
        table.view().revision,
        table.modal(),
        size,
        table.context.open,
        table.presenting(),
        table.context.commitments.clone(),
    );
    if previous.as_ref() == Some(&key) {
        return;
    }
    *previous = Some(key);
    for entity in &menus {
        commands.entity(entity).despawn();
    }
    if !table.context.open || table.modal() {
        return;
    }
    let Some(selection) = table.context.selected else {
        return;
    };
    let view = table.view();
    let entries = entries(&table, &view, selection.source);
    let anchor = camera
        .single()
        .ok()
        .and_then(|(camera, transform)| camera.world_to_viewport(transform, selection.at).ok())
        .unwrap_or(size * 0.5);
    let radial = entries.len() <= 3
        && selection.source != Source::Discard
        && entries.iter().all(|e| e.warning.is_none())
        && size.x >= 950.0
        && size.y >= 650.0;
    let width = if radial {
        450.0
    } else {
        320.0_f32.min(size.x - 36.0)
    };
    let bottom_reserved = (size.y * 0.29).min(250.0) + 20.0;
    let height = if radial {
        260.0
    } else {
        (136.0 + entries.len() as f32 * 90.0).min((size.y - bottom_reserved - 80.0).max(120.0))
    };
    let left_edge = if size.x >= 650.0 { 145.0 } else { 12.0 };
    let at = Vec2::new(
        (anchor.x - width * 0.5).clamp(left_edge, (size.x - width - 12.0).max(left_edge)),
        (anchor.y - height * 0.5).clamp(68.0, (size.y - height - bottom_reserved).max(68.0)),
    );
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(at.x),
                top: px(at.y),
                width: px(width),
                height: px(height),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                padding: UiRect::all(px(8)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.055, 0.97)),
            GlobalZIndex(50),
            Menu,
            TabGroup::modal(),
        ))
        .observe(scroll_panel)
        .with_children(|p| {
            p.spawn(art.text(
                format!(
                    "{} · Tony's decision\n{}",
                    source_name(&view, selection.source),
                    feedback::KEYBOARD_HINT,
                ),
                14.0,
                GOLD,
            ));
            if radial {
                for (index, entry) in entries.into_iter().enumerate() {
                    let (x, y) =
                        [(145.0, 96.0), (290.0, 148.0), (145.0, 200.0), (0.0, 148.0)][entry.slot];
                    p.spawn(Node {
                        position_type: PositionType::Absolute,
                        left: px(x + 6.0),
                        top: px(y),
                        width: px(140),
                        ..default()
                    })
                    .with_children(|p| {
                        let entity = button(&art, p, &entry.label, entry.action, false);
                        p.commands()
                            .entity(entity)
                            .insert(feedback::context_button());
                        if index == 0 {
                            p.commands()
                                .entity(entity)
                                .insert(bevy::input_focus::AutoFocus);
                        }
                    });
                }
            } else {
                if entries.is_empty() {
                    p.spawn(art.text("No cards or legal actions here.", 15.0, MUTED));
                }
                for (index, entry) in entries.into_iter().enumerate() {
                    let entity = button(&art, p, &entry.label, entry.action, false);
                    p.commands()
                        .entity(entity)
                        .insert(feedback::context_button());
                    if index == 0 {
                        p.commands()
                            .entity(entity)
                            .insert(bevy::input_focus::AutoFocus);
                    }
                    if let Some(warning) = entry.warning {
                        p.spawn(art.text(warning, 14.0, GOLD));
                    }
                }
            }
        });
}
