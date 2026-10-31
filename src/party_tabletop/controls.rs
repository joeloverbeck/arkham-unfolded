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
    pub selected: Option<(Source, Vec3)>,
    pub open: bool,
    pub epoch: u64,
    pub utilities: bool,
    pub commitments: BTreeSet<u16>,
    pub cancellations: BTreeSet<u16>,
    stamp: Option<(u64, Option<usize>)>,
}
impl Context {
    pub fn dismiss(&mut self) {
        self.open = false;
        self.epoch = self.epoch.wrapping_add(1);
    }
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
    pub fn select(&mut self, source: Source, at: Vec3) {
        self.dismiss();
        self.selected = Some((source, at));
        self.open = true;
        self.utilities = false;
    }
    pub fn accepts(&self, epoch: u64) -> bool {
        self.open && self.epoch == epoch
    }
}
pub(super) fn exists(view: &Observation, source: Source) -> bool {
    match source {
        Source::Card(index) => index < CARDS.len(),
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
        Source::Token(id) => view.tokens.iter().any(|t| t.id == id),
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
pub(super) fn compound_choice(
    view: &Observation,
    set: &BTreeSet<u16>,
    cancel: bool,
) -> Option<String> {
    if set.is_empty() {
        return None;
    }
    view.choices
        .iter()
        .find(|c| ids(&c.id, cancel).as_ref() == Some(set))
        .map(|c| c.id.clone())
}
pub(super) fn selectable(view: &Observation, id: u16, cancel: bool) -> bool {
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
fn entries(table: &Table, source: Source) -> Vec<Entry> {
    let view = table.view();
    let mut entries = Vec::new();
    if table.replay.is_none() {
        for choice in source_choices(&view, source) {
            entries.push(Entry {
                label: choice.label.clone(),
                action: Action::ContextChoice(
                    table.context.epoch,
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
            && selectable(&view, id, cancel)
        {
            let selected = if cancel {
                table.context.cancellations.contains(&id)
            } else {
                table.context.commitments.contains(&id)
            };
            let label = if cancel {
                if selected {
                    "Unselect cancellation".into()
                } else {
                    "Select to cancel".into()
                }
            } else {
                let card = view
                    .investigators
                    .iter()
                    .flat_map(|i| &i.hand)
                    .find(|c| c.id == id)
                    .unwrap();
                if selected {
                    "Unselect commitment".into()
                } else {
                    format!(
                        "Select to commit +{} intellect{}",
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
                action: Action::Toggle(table.context.epoch, view.revision, id, cancel),
            });
        }
    }
    for (label, index) in inspect_targets(&view, source) {
        entries.push(Entry {
            label,
            action: Action::Inspect(index),
        });
    }
    if source == Source::Card(cards::index("card/01108").unwrap()) {
        entries.push(Entry {
            label: "Inspect reverse".into(),
            action: Action::Inspect(cards::index("card/01108/reverse").unwrap()),
        });
    }
    entries
}
pub(super) fn inspection_only(table: &Table, source: Source) -> Option<usize> {
    feedback::sole_inspection(entries(table, source).into_iter().map(|e| {
        if let Action::Inspect(i) = e.action {
            Some(i)
        } else {
            None
        }
    }))
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
    let entries = entries(&table, source);
    let anchor = camera
        .0
        .world_to_viewport(camera.1, at)
        .unwrap_or(size * 0.5);
    let radial = entries.len() <= 3
        && !matches!(source, Source::Discard(_))
        && size.x >= 950.
        && size.y >= 650.;
    let width = if radial {
        450.
    } else {
        340_f32.min(size.x - 36.)
    };
    let height = if radial {
        255.
    } else {
        (110. + entries.len() as f32 * 75.).min((size.y - 340.).max(140.))
    };
    let at = Vec2::new(
        (anchor.x - width / 2.).clamp(145., (size.x - width - 12.).max(145.)),
        (anchor.y - height / 2.).clamp(85., (size.y - height - 260.).max(85.)),
    );
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(at.x),
                top: px(at.y),
                width: px(width),
                height: px(height),
                padding: UiRect::all(px(8)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::scroll_y(),
                ..default()
            },
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
                        table
                            .view()
                            .actor
                            .map_or("Choose investigator", Investigator::name),
                        feedback::KEYBOARD_HINT
                    ),
                14.,
                GOLD,
            ));
            if entries.is_empty() {
                p.spawn(art.text("No legal action here at this decision.", 14., MUTED));
            }
            for (index, entry) in entries.into_iter().enumerate() {
                if radial {
                    let (x, y) = [(145., 90.), (290., 155.), (0., 155.)][index];
                    p.spawn(Node {
                        position_type: PositionType::Absolute,
                        left: px(x),
                        top: px(y),
                        width: px(145),
                        ..default()
                    })
                    .with_children(|p| {
                        let entity = ui::button(&art, p, &entry.label, entry.action);
                        if index == 0 {
                            p.commands()
                                .entity(entity)
                                .insert(bevy::input_focus::AutoFocus);
                        }
                    });
                } else {
                    let entity = ui::button(&art, p, &entry.label, entry.action);
                    if index == 0 {
                        p.commands()
                            .entity(entity)
                            .insert(bevy::input_focus::AutoFocus);
                    }
                }
            }
        });
}
