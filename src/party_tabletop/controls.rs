//! Owns Party's transient selection lifecycle; continuity owns committed choices.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Card(usize),
    Hand(u16),
    Committed(u16),
    Deck(Investigator),
    Discard(Investigator),
    Bag,
    Token(u8),
}
#[derive(Default)]
pub(super) struct Context {
    selected: Option<(Source, Vec3)>,
    open: bool,
    epoch: u64,
    utilities: bool,
    commitments: BTreeSet<u16>,
    cancellations: BTreeSet<u16>,
    stamp: Option<(u64, Option<usize>)>,
}
pub(super) enum SelectionEffect {
    Menu,
    Inspect(usize),
}
impl Context {
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
                .is_some_and(|(selected, _)| selected == source)
    }
    pub fn is_pending(&self, source: Source) -> bool {
        match source {
            Source::Hand(id) => self.commitments.contains(&id),
            Source::Token(id) => self.cancellations.contains(&u16::from(id)),
            _ => false,
        }
    }
    fn can_toggle_commitment(&self, view: &Observation, id: u16) -> bool {
        if self.commitments.contains(&id) {
            return selectable(view, id, false);
        }
        let mut proposed = self.commitments.clone();
        proposed.insert(id);
        compound_choice(view, &proposed, false).is_some()
    }
    pub fn contribution(
        &self,
        view: &Observation,
        live: bool,
        source: Source,
    ) -> Option<feedback::Contribution> {
        let Source::Hand(id) = source else {
            return None;
        };
        if !live || !self.can_toggle_commitment(view, id) {
            return None;
        }
        let card = view
            .investigators
            .iter()
            .flat_map(|i| &i.hand)
            .find(|card| card.id == id)?;
        Some(feedback::Contribution {
            bonus: card.intellect_icons,
            skill: crate::tabletop::symbols::Skill::Intellect,
        })
    }
    /// Ordinary menu closure keeps both staged sets and the utilities panel.
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
    pub fn inspect(&mut self, view: &Observation, index: usize) -> Option<usize> {
        if !cards::inspectable(view, index) {
            return None;
        }
        self.dismiss();
        Some(index)
    }
    /// Party also resets after successful reopen, return to live, and a new attempt.
    pub fn reset(&mut self) {
        self.dismiss();
        self.selected = None;
        self.commitments.clear();
        self.cancellations.clear();
        self.stamp = None;
    }
    pub fn sync(&mut self, revision: u64, replay: Option<usize>) {
        let stamp = Some((revision, replay));
        if self.stamp != stamp {
            self.reset();
            self.stamp = stamp;
        }
    }
    pub fn select(
        &mut self,
        view: &Observation,
        live: bool,
        revision: u64,
        source: Source,
        at: Vec3,
    ) -> Option<SelectionEffect> {
        if revision != view.revision || !exists(view, source) {
            return None;
        }
        self.dismiss();
        self.selected = Some((source, at));
        self.open = true;
        self.utilities = false;
        Some(
            if let Some(index) = self.inspection_only(view, live, source) {
                self.dismiss();
                SelectionEffect::Inspect(index)
            } else {
                SelectionEffect::Menu
            },
        )
    }
    fn accepts(&self, epoch: u64) -> bool {
        self.open && self.epoch == epoch
    }
    /// Do not pre-check revision/id: continuity must report those refusals visibly.
    pub fn contextual_choice(&self, epoch: u64, revision: u64, id: String) -> Option<Action> {
        self.accepts(epoch).then_some(Action::Choose(revision, id))
    }
    pub fn toggle(
        &mut self,
        view: &Observation,
        live: bool,
        epoch: u64,
        revision: u64,
        id: u16,
        cancel: bool,
    ) {
        if live
            && self.accepts(epoch)
            && revision == view.revision
            && if cancel {
                selectable(view, id, true)
            } else {
                self.can_toggle_commitment(view, id)
            }
        {
            let set = if cancel {
                &mut self.cancellations
            } else {
                &mut self.commitments
            };
            if !set.remove(&id) {
                set.insert(id);
            }
            self.dismiss();
        }
    }
    fn pending_set(&self, cancel: bool) -> &BTreeSet<u16> {
        if cancel {
            &self.cancellations
        } else {
            &self.commitments
        }
    }
    /// Resolve the current set, including when its menu has since closed.
    pub fn confirm(&self, view: &Observation, revision: u64, cancel: bool) -> Option<Action> {
        compound_choice(view, self.pending_set(cancel), cancel)
            .map(|id| Action::Choose(revision, id))
    }
    pub fn confirmation_control(
        &self,
        view: &Observation,
        cancel: bool,
    ) -> Option<(String, Action)> {
        compound_choice(view, self.pending_set(cancel), cancel)?;
        let label = if cancel {
            "Jacqueline: confirm selected cancellation".into()
        } else {
            let bonus: u16 = view
                .investigators
                .iter()
                .flat_map(|i| &i.hand)
                .filter(|c| self.is_pending(Source::Hand(c.id)))
                .map(|c| u16::from(c.intellect_icons))
                .sum();
            format!("Commit selected · +{bonus} intellect")
        };
        Some((label, Action::Confirm(view.revision, cancel)))
    }
    pub fn pending_summary(&self, view: &Observation) -> Option<String> {
        if self.commitments.is_empty() && self.cancellations.is_empty() {
            return None;
        }
        let selected = view
            .investigators
            .iter()
            .flat_map(|i| &i.hand)
            .filter(|c| self.is_pending(Source::Hand(c.id)))
            .map(|c| format!("{}'s {}", c.owner.name(), c.title))
            .collect::<Vec<_>>()
            .join(", ");
        Some(if !selected.is_empty() {
            format!(
                "Selected: {selected}. Supporters may each commit one appropriate card. Select again to remove."
            )
        } else {
            format!(
                "Selected token cancellations: {}. Choose one auto-fail or two non-auto-fail tokens, then confirm.",
                self.cancellations
                    .iter()
                    .map(|id| (id + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}
pub(super) fn exists(view: &Observation, source: Source) -> bool {
    match source {
        Source::Card(index) => cards::inspectable(view, index),
        Source::Hand(id) => view
            .investigators
            .iter()
            .any(|i| i.hand.iter().any(|c| c.id == id)),
        Source::Committed(id) => view
            .test
            .as_ref()
            .is_some_and(|t| t.committed.iter().any(|c| c.id == id)),
        Source::Deck(who) => view.investigator(who).deck_count > 0,
        Source::Discard(who) => !view.investigator(who).discard.is_empty(),
        Source::Bag => true,
        Source::Token(id) => view.held_tokens().iter().any(|t| t.id == id),
    }
}
fn ids(choice: &str, cancel: bool) -> Option<BTreeSet<u16>> {
    choice
        .strip_prefix(if cancel { "cancel:" } else { "commit:" })?
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()
}
fn compound_choice(view: &Observation, set: &BTreeSet<u16>, cancel: bool) -> Option<String> {
    if set.is_empty() {
        return None;
    }
    view.choices
        .iter()
        .find(|c| ids(&c.id, cancel).as_ref() == Some(set))
        .map(|c| c.id.clone())
}
fn selectable(view: &Observation, id: u16, cancel: bool) -> bool {
    view.choices
        .iter()
        .any(|c| ids(&c.id, cancel).is_some_and(|ids| ids.contains(&id)))
}
pub(super) fn has_source(id: &str) -> bool {
    matches!(
        id,
        "resource" | "draw" | "investigate" | "reveal" | "use-jacqueline" | "advance"
    ) || [
        "select:",
        "commit:",
        "cancel:",
        "resolve:",
        "lucky:",
        "shuffle:retain:",
    ]
    .iter()
    .any(|p| id.starts_with(p))
}
fn source_choices(view: &Observation, source: Source) -> Vec<&crate::Choice> {
    view.choices
        .iter()
        .filter(|choice| match source {
            Source::Card(index) => match CARDS[index].role {
                "card/06003" => {
                    choice.id == "select:tony"
                        || (view.actor == Some(Investigator::Tony) && choice.id == "resource")
                }
                "card/06005" => {
                    choice.id == "select:patrice"
                        || (view.actor == Some(Investigator::Patrice) && choice.id == "resource")
                }
                "card/60401" => {
                    choice.id == "select:jacqueline"
                        || choice.id == "use-jacqueline"
                        || (view.actor == Some(Investigator::Jacqueline) && choice.id == "resource")
                }
                "card/01111" => choice.id == "investigate",
                "card/01108" => choice.id == "advance",
                _ => false,
            },
            Source::Deck(who) => view.actor == Some(who) && choice.id == "draw",
            Source::Hand(id) => choice.id == format!("lucky:{id}"),
            Source::Discard(who) => choice
                .id
                .strip_prefix("shuffle:retain:")
                .and_then(|id| id.parse::<u16>().ok())
                .is_some_and(|id| view.investigator(who).discard.iter().any(|c| c.id == id)),
            Source::Bag => matches!(choice.id.as_str(), "reveal" | "use-jacqueline"),
            Source::Token(id) => choice.id == format!("resolve:{id}"),
            _ => false,
        })
        .collect()
}
struct Entry {
    label: String,
    action: Action,
}
fn inspect_targets(view: &Observation, source: Source) -> Vec<(String, usize)> {
    let card = |code: &str| cards::index(&format!("card/{code}"));
    match source {
        Source::Card(index) => vec![("Inspect".into(), index)],
        Source::Hand(id) => view
            .investigators
            .iter()
            .flat_map(|i| &i.hand)
            .find(|c| c.id == id)
            .and_then(|c| card(&c.code))
            .map(|i| vec![("Inspect".into(), i)])
            .unwrap_or_default(),
        Source::Committed(id) => view
            .test
            .as_ref()
            .and_then(|t| t.committed.iter().find(|c| c.id == id))
            .and_then(|c| card(&c.code))
            .map(|i| vec![("Inspect".into(), i)])
            .unwrap_or_default(),
        Source::Discard(who) => view
            .investigator(who)
            .discard
            .iter()
            .rev()
            .filter_map(|c| {
                card(&c.code).map(|i| (format!("Inspect {} · copy {}", c.title, c.id), i))
            })
            .collect(),
        Source::Bag | Source::Token(_) => vec![(
            "Inspect scenario reference".into(),
            cards::index("card/01104").unwrap(),
        )],
        Source::Deck(_) => vec![],
    }
}
fn entries(context: &Context, view: &Observation, live: bool, source: Source) -> Vec<Entry> {
    let mut entries = Vec::new();
    if live {
        for choice in source_choices(view, source) {
            entries.push(Entry {
                label: ui::choice_label(view, choice),
                action: Action::ContextChoice(
                    context.menu_epoch(),
                    view.revision,
                    choice.id.clone(),
                ),
            });
        }
        let selection = match source {
            Source::Hand(id) => Some((id, false)),
            Source::Token(id) => Some((u16::from(id), true)),
            _ => None,
        };
        if let Some((id, cancel)) = selection
            && if cancel {
                selectable(view, id, true)
            } else {
                context.can_toggle_commitment(view, id)
            }
        {
            let selected = context.is_pending(source);
            let label = if cancel {
                if selected {
                    "Jacqueline: unselect cancellation".into()
                } else {
                    "Jacqueline: select to cancel".into()
                }
            } else {
                let card = view
                    .investigators
                    .iter()
                    .flat_map(|i| &i.hand)
                    .find(|c| c.id == id)
                    .unwrap();
                if selected {
                    format!("{}: unselect commitment", ui::short_name(card.owner))
                } else {
                    format!(
                        "{}: select to commit +{} intellect{}",
                        ui::short_name(card.owner),
                        card.intellect_icons,
                        if card.code == "07032" {
                            " · add one curse"
                        } else {
                            ""
                        }
                    )
                }
            };
            entries.push(Entry {
                label,
                action: Action::Toggle(context.menu_epoch(), view.revision, id, cancel),
            });
        }
    }
    for (label, index) in inspect_targets(view, source) {
        entries.push(Entry {
            label,
            action: Action::Inspect(index),
        });
    }
    entries
}
impl Context {
    fn inspection_only(&self, view: &Observation, live: bool, source: Source) -> Option<usize> {
        feedback::sole_inspection(entries(self, view, live, source).into_iter().map(|e| {
            if let Action::Inspect(i) = e.action {
                Some(i)
            } else {
                None
            }
        }))
    }
}
#[derive(Component)]
pub(super) struct Menu;
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn render(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<Art>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    menus: Query<Entity, With<Menu>>,
    mut previous: Local<Option<(u64, u64, bool, Vec2, BTreeSet<u16>, BTreeSet<u16>)>>,
) {
    let size = Vec2::new(window.width(), window.height());
    let key = (
        table.context.epoch,
        table.view().revision,
        table.modal(),
        size,
        table.context.commitments.clone(),
        table.context.cancellations.clone(),
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
    let Some((source, at)) = table.context.selected else {
        return;
    };
    let entries = entries(
        &table.context,
        &table.view(),
        table.replay.is_none(),
        source,
    );
    let anchor = camera
        .0
        .world_to_viewport(camera.1, at)
        .unwrap_or(size * 0.5);
    commands
        .spawn((
            feedback::menu::panel(anchor, size, 110., (size.y * 0.29).min(250.) + 20.),
            BackgroundColor(INK),
            GlobalZIndex(50),
            Menu,
            TabGroup::modal(),
        ))
        .observe(scroll_panel)
        .with_children(|p| {
            p.spawn(art.text(
                format!(
                    "{}\n{}",
                    ui::turn_heading(&table.view()),
                    feedback::KEYBOARD_HINT
                ),
                14.,
                GOLD,
            ));
            if entries.is_empty() {
                p.spawn(art.text("No legal action here at this decision.", 14., MUTED));
            }
            for (index, entry) in entries.into_iter().enumerate() {
                let label = feedback::menu::label(&entry.label);
                let entity = ui::button(&art, p, &label, entry.action);
                p.commands().entity(entity).insert(feedback::menu::row());
                if index == 0 {
                    p.commands()
                        .entity(entity)
                        .insert(bevy::input_focus::AutoFocus);
                }
            }
        });
}
