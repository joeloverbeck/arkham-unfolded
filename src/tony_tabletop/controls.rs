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
    EncounterDiscard,
    Bag,
    Token,
}
#[derive(Clone, Copy)]
struct Selection {
    source: Source,
    at: Vec3,
}
pub(super) enum SelectionEffect {
    Menu,
    Inspect(usize),
}
#[derive(Default)]
pub(super) struct Context {
    selected: Option<Selection>,
    open: bool,
    epoch: u64,
    commitments: BTreeSet<u8>,
    utilities: bool,
    stamp: Option<(u64, Option<usize>)>,
}
impl Context {
    pub fn commitments(&self) -> &BTreeSet<u8> {
        &self.commitments
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn menu_epoch(&self) -> u64 {
        self.epoch
    }
    pub fn utilities_open(&self) -> bool {
        self.utilities
    }
    pub fn captures_keyboard(&self) -> bool {
        self.open || self.utilities
    }
    pub fn is_menu_source(&self, source: Source) -> bool {
        self.open
            && self
                .selected
                .is_some_and(|selection| selection.source == source)
    }
    pub fn is_pending(&self, source: Source) -> bool {
        matches!(source, Source::Hand(id) if self.commitments.contains(&id))
    }
    fn can_toggle_commitment(&self, view: &Observation, id: u8) -> bool {
        if self.commitments.contains(&id) {
            return committable(view, id);
        }
        let mut proposed = self.commitments.clone();
        proposed.insert(id);
        commitment_choice(view, &proposed).is_some()
    }
    pub fn contribution(
        &self,
        view: &Observation,
        gameplay: bool,
        source: Source,
    ) -> Option<feedback::Contribution> {
        let Source::Hand(id) = source else {
            return None;
        };
        if !gameplay || !self.can_toggle_commitment(view, id) {
            return None;
        }
        use crate::{tabletop::symbols::Skill, tony::TestKind};
        Some(feedback::Contribution {
            bonus: 1,
            skill: match view.test.as_ref()?.kind {
                TestKind::Fight => Skill::Combat,
                TestKind::Evade => Skill::Agility,
                TestKind::Investigate => Skill::Intellect,
            },
        })
    }
    /// Ordinary menu closure keeps the compound choice; Escape also closes utilities.
    pub fn dismiss(&mut self) {
        self.open = false;
        self.epoch = self.epoch.wrapping_add(1);
    }
    pub fn close_menus(&mut self) {
        self.dismiss();
        self.utilities = false;
    }
    pub fn toggle_utilities(&mut self) {
        self.utilities = !self.utilities;
        self.dismiss();
    }
    pub fn inspect(&mut self, index: usize) -> usize {
        self.dismiss();
        index
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
    pub fn select(
        &mut self,
        view: &Observation,
        gameplay: bool,
        revision: u64,
        source: Source,
        at: Vec3,
    ) -> Option<SelectionEffect> {
        if revision != view.revision || !exists(view, source) {
            return None;
        }
        self.dismiss();
        self.selected = Some(Selection { source, at });
        self.open = true;
        self.utilities = false;
        Some(
            if let Some(index) = self.inspection_only(view, gameplay, source) {
                SelectionEffect::Inspect(self.inspect(index))
            } else {
                SelectionEffect::Menu
            },
        )
    }
    fn accepts(&self, epoch: u64) -> bool {
        self.open && epoch == self.epoch
    }
    /// Admission checks the menu epoch only. Continuity owns visible revision/id refusals.
    pub fn contextual_choice(&self, epoch: u64, revision: u64, id: String) -> Option<Action> {
        self.accepts(epoch).then_some(Action::Choose(revision, id))
    }
    pub fn toggle_commitment(
        &mut self,
        view: &Observation,
        gameplay: bool,
        epoch: u64,
        revision: u64,
        id: u8,
    ) {
        if self.accepts(epoch)
            && gameplay
            && revision == view.revision
            && self.can_toggle_commitment(view, id)
        {
            if !self.commitments.remove(&id) {
                self.commitments.insert(id);
            }
            self.dismiss();
        }
    }
    /// Confirmation carries no epoch or captured set, and also works after dismissal.
    pub fn confirm(&self, view: &Observation, revision: u64) -> Option<Action> {
        commitment_choice(view, &self.commitments).map(|id| Action::Choose(revision, id))
    }
    pub fn confirmation_control(&self, view: &Observation) -> Option<(String, Action)> {
        commitment_choice(view, &self.commitments)?;
        let bonus = commitment_bonus(view, self.commitments.len())?;
        Some((
            format!("Commit selected · {bonus}"),
            Action::ConfirmCommit(view.revision),
        ))
    }
    pub fn pending_summary(&self) -> Option<String> {
        (!self.commitments.is_empty()).then(|| {
            format!(
                "Selected On the Hunt copies: {} · select another card to change this set",
                self.commitments
                    .iter()
                    .map(|id| (id + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
    fn invalidate(&mut self, view: &Observation, replay: Option<usize>) -> bool {
        let mut changed = self.sync(view.revision, replay);
        if self
            .selected
            .is_some_and(|selection| !exists(view, selection.source))
        {
            self.dismiss();
            self.selected = None;
            changed = true;
        }
        changed
    }
}

pub(super) fn invalidate(mut table: ResMut<Table>) {
    let view = table.view();
    let replay = table.replay.as_ref().map(Replay::index);
    if table.context.invalidate(&view, replay) {
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
        Source::EncounterDiscard => !view.encounter_discard().is_empty(),
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
fn committable(view: &Observation, id: u8) -> bool {
    view.hand.iter().any(|card| card.id == id)
        && view
            .choices
            .iter()
            .any(|choice| choice.id == format!("commit:{id}"))
}
fn commitment_choice(view: &Observation, selected: &BTreeSet<u8>) -> Option<String> {
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
        Source::EncounterDiscard => format!(
            "Encounter discard · {} cards",
            view.encounter_discard().len()
        ),
        Source::Bag => format!("Chaos bag · {} tokens", view.bag_count),
        Source::Token => "Revealed chaos token".into(),
    }
}
#[derive(Component)]
pub(super) struct Menu;
struct Entry {
    label: String,
    action: Action,
    warning: Option<String>,
}
fn entries(context: &Context, view: &Observation, gameplay: bool, source: Source) -> Vec<Entry> {
    let mut entries = Vec::new();
    if gameplay {
        for choice in source_choices(view, source) {
            let short = match choice.id.as_str() {
                "fight" => "Fight · bonus/1 action",
                "evade" => "Evade · 1 action",
                "engage" => "Engage · bonus/1 action",
                "resource" => "Gain resource · 1 action",
                "draw" => "Draw · 1 action",
                "investigate" => "Investigate · 1 action",
                "injury" => "Remove Injury · 2 actions",
                "reveal" => "Reveal token",
                "resolve" => "Resolve test",
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
                    context.menu_epoch(),
                    view.revision,
                    choice.id.clone(),
                ),
                warning,
            });
        }
        if let Source::Hand(id) = source
            && context.can_toggle_commitment(view, id)
            && let Some(bonus) = commitment_bonus(view, 1)
        {
            entries.push(Entry {
                label: if context.is_pending(source) {
                    "Unselect commitment".into()
                } else {
                    format!("Select to commit {bonus}")
                },
                action: Action::ToggleCommit(context.menu_epoch(), view.revision, id),
                warning: None,
            });
        }
    }
    if let Some(index) = inspect_index(view, source) {
        entries.push(Entry {
            label: "Inspect".into(),
            action: Action::Inspect(index),
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
                    warning: None,
                });
            }
        }
    }
    if source == Source::EncounterDiscard {
        for card in view.encounter_discard().iter().rev() {
            if let Some(index) = CARDS.iter().position(|face| face.code == card.code) {
                entries.push(Entry {
                    label: format!("Inspect {}", card.title),
                    action: Action::Inspect(index),
                    warning: None,
                });
            }
        }
    }
    entries
}

impl Context {
    fn inspection_only(&self, view: &Observation, gameplay: bool, source: Source) -> Option<usize> {
        crate::tabletop_feedback::sole_inspection(
            entries(self, view, gameplay, source)
                .into_iter()
                .map(|entry| match entry.action {
                    Action::Inspect(index) => Some(index),
                    _ => None,
                }),
        )
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
    let entries = entries(
        &table.context,
        &view,
        table.replay.is_none() && !table.presenting(),
        selection.source,
    );
    let anchor = camera
        .single()
        .ok()
        .and_then(|(camera, transform)| camera.world_to_viewport(transform, selection.at).ok())
        .unwrap_or(size * 0.5);
    let bottom_reserved = (size.y * 0.29).min(250.0) + 20.0;
    commands
        .spawn((
            feedback::menu::panel(anchor, size, 68., bottom_reserved),
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
            if entries.is_empty() {
                p.spawn(art.text("No cards or legal actions here.", 15.0, MUTED));
            }
            for (index, entry) in entries.into_iter().enumerate() {
                let label = feedback::menu::label(&entry.label);
                let label = match entry.warning {
                    Some(warning) => format!("{label}\n{warning}"),
                    None => label,
                };
                let entity = button(&art, p, &label, entry.action, false);
                p.commands()
                    .entity(entity)
                    .insert((feedback::context_button(), feedback::menu::row()));
                if index == 0 {
                    p.commands()
                        .entity(entity)
                        .insert(bevy::input_focus::AutoFocus);
                }
            }
        });
}
