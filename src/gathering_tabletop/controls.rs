//! Gathering's choice presentation and transient contextual selection. The Session remains the sole
//! authority for legal choices and committed decisions.

use super::*;
use crate::Choice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Investigator(Investigator),
    Card(u16),
    Location(String),
    Deck(Investigator),
    Discard(Investigator),
    Removed(Investigator),
    Act,
    Agenda,
    Bag,
    Token(usize),
    EncounterDeck,
    EncounterDiscard,
    Victory,
    ScenarioRemoved,
    SearchPlayer,
    SearchEncounter,
    Resolving,
    PendingDiscard,
    Backpack(u16),
    Lita,
    ScenarioReference,
}

#[derive(Clone)]
pub(super) struct Selection {
    pub source: Source,
    pub at: Vec3,
    pub title: String,
    pub inspect_code: String,
}

#[derive(Default)]
pub(super) struct Context {
    selected: Option<Selection>,
    pending: bool,
    open: bool,
    generation: u64,
    stamp: Option<(u64, u64)>,
}

pub(super) enum SelectionEffect {
    Menu,
    Inspect(String, String),
}

impl Context {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn captures_keyboard(&self) -> bool {
        self.is_open()
    }

    pub fn is_menu_source(&self, source: &Source) -> bool {
        self.open && self.selected.as_ref().is_some_and(|s| &s.source == source)
    }

    pub fn dismiss(&mut self) {
        self.open = false;
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn reset(&mut self) {
        self.dismiss();
        self.selected = None;
        self.pending = false;
        self.stamp = None;
    }

    pub fn sync(&mut self, epoch: u64, revision: u64) {
        let stamp = Some((epoch, revision));
        if self.stamp != stamp {
            self.reset();
            self.stamp = stamp;
        }
    }

    pub fn select(
        &mut self,
        view: &Observation,
        selection: Selection,
        live: bool,
    ) -> Option<SelectionEffect> {
        if !exists(view, &selection.source) {
            return None;
        }
        self.dismiss();
        self.pending = false;
        let targets = inspections(view, &selection);
        if targets.len() == 1 && (!live || source_choices(view, &selection.source).is_empty()) {
            let target = targets.into_iter().next().expect("One inspection target");
            return Some(SelectionEffect::Inspect(target.title, target.code));
        }
        self.selected = Some(selection);
        self.open = true;
        Some(SelectionEffect::Menu)
    }

    pub fn open_pending(&mut self) {
        self.dismiss();
        self.selected = None;
        self.pending = true;
        self.open = true;
    }

    pub fn accepts(&self, generation: u64) -> bool {
        self.open && self.generation == generation
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

struct Inspection {
    title: String,
    code: String,
    details: Option<String>,
}

/// Public pile members supply inspection faces; no deck contents or private
/// continuation frames participate in this projection.
fn inspections(view: &Observation, selection: &Selection) -> Vec<Inspection> {
    if matches!(selection.source, Source::Victory)
        && let Some(play) = &view.play
    {
        let cards = play.scenario.victory_display();
        if !cards.is_empty() {
            return cards
                .iter()
                .rev()
                .map(|card| Inspection {
                    title: card.title.into(),
                    code: card.code.into(),
                    details: None,
                })
                .collect();
        }
    }
    if matches!(selection.source, Source::Bag | Source::Token(_)) {
        return vec![Inspection {
            title: "The Gathering · Standard reference".into(),
            code: "01104".into(),
            details: None,
        }];
    }
    let mut cards: Vec<&crate::gathering::CardView> = vec![];
    if let Some(play) = &view.play {
        match &selection.source {
            Source::Discard(who) | Source::Removed(who) => {
                if let Some(player) = play
                    .players
                    .iter()
                    .find(|player| player.investigator == *who)
                {
                    cards.extend(if matches!(selection.source, Source::Discard(_)) {
                        player.discard.iter().rev()
                    } else {
                        player.removed.iter().rev()
                    });
                }
            }
            Source::EncounterDiscard => cards.extend(play.encounter_discard.iter().rev()),
            Source::ScenarioRemoved => cards.extend(play.scenario.removed.iter().rev()),
            Source::SearchPlayer => cards.extend(&play.searched_player),
            Source::SearchEncounter => cards.extend(&play.searched_encounter),
            Source::Resolving => {
                cards.extend(&play.pending.resolving_cards);
                if let Some(revelation) = &play.pending_revelation {
                    cards.push(&revelation.card);
                }
            }
            Source::PendingDiscard => {
                if let Some(discard) = &play.pending.discard {
                    cards.extend(&discard.ordered);
                    cards.extend(&discard.remaining);
                }
            }
            Source::Backpack(id) => {
                if let Some(asset) = play
                    .players
                    .iter()
                    .flat_map(|player| &player.assets)
                    .find(|asset| asset.card.id == *id)
                {
                    cards.push(&asset.card);
                }
                if let Some(pack) = play.backpacks.iter().find(|pack| pack.asset == *id) {
                    cards.extend(&pack.cards);
                }
            }
            Source::Card(id) => {
                if let Some(enemy) = play
                    .scenario
                    .enemies
                    .iter()
                    .find(|enemy| enemy.card.id == *id && enemy.engaged.is_some())
                {
                    cards.push(&enemy.card);
                }
            }
            _ => {}
        }
    }
    cards.extend(unengaged_enemies(view, &selection.source));
    let mut identities = std::collections::BTreeSet::new();
    let mut targets = cards
        .into_iter()
        .filter(|card| identities.insert(card.id))
        .map(|card| {
            let key = format!("enemy:{}", card.id);
            let details = play::enemy_inspection(view, &key).map(|(_, details)| details);
            Inspection {
                title: card.title.clone(),
                code: if details.is_some() {
                    key
                } else {
                    card.code.clone()
                },
                details,
            }
        })
        .collect::<Vec<_>>();
    if targets.is_empty() || matches!(selection.source, Source::Location(_)) {
        targets.insert(
            0,
            Inspection {
                title: selection.title.clone(),
                code: selection.inspect_code.clone(),
                details: None,
            },
        );
    }
    targets
}

/// Location markers and headers reach the whole group; each enemy owns its menu.
fn unengaged_enemies<'a>(
    view: &'a Observation,
    source: &Source,
) -> Vec<&'a crate::gathering::CardView> {
    let Some(play) = &view.play else {
        return vec![];
    };
    let room = match source {
        Source::Location(code) => Some(code.as_str()),
        _ => None,
    };
    play.scenario
        .enemies
        .iter()
        .filter(|enemy| enemy.engaged.is_none() && Some(enemy.location.as_str()) == room)
        .map(|enemy| &enemy.card)
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum Route {
    Sources(Vec<Source>),
    Global,
    Pending,
}

fn route(view: &Observation, id: &str) -> Route {
    let id = id
        .strip_prefix("action:ordinary:")
        .or_else(|| id.strip_prefix("action:tony:"))
        .or_else(|| id.strip_prefix("granted:"))
        .unwrap_or(id);
    if global(id) {
        return Route::Global;
    }
    let parts: Vec<_> = id.split(':').collect();
    let owner = view
        .decision_owner
        .or_else(|| view.play.as_ref().and_then(|p| p.actor));
    let owned =
        |constructor: fn(Investigator) -> Source| owner.map(constructor).into_iter().collect();
    let cards = |id: &str| {
        id.parse()
            .map(|id| card_sources(view, id))
            .unwrap_or_default()
    };
    let sources = match parts.as_slice() {
        ["lead" | "turn", who] if *who != "end" => investigator(who)
            .map(|who| vec![Source::Investigator(who)])
            .unwrap_or_default(),
        ["resource"] | ["elimination", "physical" | "mental"] => owned(Source::Investigator),
        ["draw"] | ["upkeep", "draw"] => owned(Source::Deck),
        ["upkeep", "patrice-refill"] => vec![Source::Deck(Investigator::Patrice)],
        ["investigate" | "resign"] => current_location(view, owner),
        ["move" | "quarry" | "spell-move", location] => vec![Source::Location((*location).into())],
        ["parley", "lita"] => vec![Source::Lita],
        ["act", "advance"] | ["round-end", "advance-act"] => vec![Source::Act],
        ["agenda", "search" | "lead-horror" | "random-discard"]
        | ["agenda-discard", "resolve"]
        | ["round-end", "ghoul-doom"] => vec![Source::Agenda],
        ["mythos", "draw"] => vec![Source::EncounterDeck],
        ["test", "reveal"] => vec![Source::Bag],
        [
            "mulligan" | "fight" | "evade" | "engage" | "commit" | "boost" | "premonition"
            | "lucky" | "test-of-will" | "idol" | "idol-select" | "discard-card" | "chill"
            | "spell-discard" | "turn-end" | "opportunity" | "live-learn",
            card,
        ]
        | ["upkeep", "discard" | "patrice-discard", card]
        | ["enemy", "hunter" | "attack", card]
        | ["round-end", "voices", card]
        | ["agenda-discard", "deny", card]
        | ["draw", "reveal", card]
        | ["test-card", "commit-effect" | "pendulum" | "found", card]
        | ["search", "enemy" | "play-hunt", card]
        | ["backpack", "select", card]
        | ["event", "recover", card]
        | ["patrice-shuffle", "keep", card]
        | ["lita", "discard-ally", card]
        | ["defeat", "bleeds", card]
        | ["peter", "heal", card]
        | ["tony", "case" | "chamber", card] => cards(card),
        // The first identity is the actual played copy. Codes, costs, target
        // positions and prediction numbers elsewhere are never card identities.
        [
            "play" | "fast-asset" | "play-colt" | "uncage" | "ward",
            card,
            ..,
        ] if card.parse::<u16>().is_ok() => cards(card),
        ["play" | "uncage", "spell" | "tony" | "event", card, ..] => cards(card),
        ["asset", card, "replace", _] => cards(card),
        ["remove-threat", _, card]
        | ["deny", card, _]
        | ["bounty" | "mauser", card, _]
        | ["tony", "gregory" | "watch", card, _] => cards(card),
        ["fight" | "evade", enemy, "weapon" | "spell", asset]
        | ["fight" | "evade", "cane", asset, enemy, _] => {
            let mut sources = cards(enemy);
            sources.extend(cards(asset));
            sources
        }
        ["investigate", "spell", asset] => {
            let mut sources = cards(asset);
            sources.extend(current_location(view, owner));
            sources
        }
        ["entry", _, "decline"] => return Route::Global,
        ["entry", asset, rest @ ..] => {
            let mut sources = cards(asset);
            if let Route::Sources(inner) = route(view, &rest.join(":")) {
                sources.extend(inner);
            }
            sources
        }
        [
            "violin",
            asset,
            "discard",
            card,
            "target",
            _,
            "draw" | "resource",
        ] => {
            let mut sources = cards(asset);
            sources.extend(cards(card));
            sources
        }
        ["resourceful", source, target] | ["prescient", "return", source, target] => {
            let mut sources = cards(source);
            sources.extend(cards(target));
            sources
        }
        ["harm" | "idol-prevent", "damage" | "horror", "investigator"] => view
            .play
            .as_ref()
            .and_then(|p| p.pending.harm.as_ref())
            .map(|h| vec![Source::Investigator(h.investigator)])
            .unwrap_or_default(),
        ["harm" | "idol-prevent", "damage" | "horror", asset] => cards(asset),
        ["revelation", "resolve"] => view
            .play
            .as_ref()
            .and_then(|p| p.pending_revelation.as_ref())
            .map(|r| card_sources(view, r.card.id))
            .unwrap_or_default(),
        ["backpack", "search"] => owned(Source::Deck),
        ["act", "contribute", index] => {
            let mut sources = indexed_investigator(index);
            sources.push(Source::Act);
            sources
        }
        ["lita", "recipient", index] => indexed_investigator(index),
        ["elimination", "lead", who] | ["tony", "quick-thinking", who] => investigator(who)
            .map(|who| vec![Source::Investigator(who)])
            .unwrap_or_default(),
        ["enemy", "engage", enemy, index] => {
            let mut sources = cards(enemy);
            sources.extend(indexed_investigator(index));
            sources
        }
        ["hunter", "move", enemy, location] => {
            let mut sources = cards(enemy);
            sources.push(Source::Location((*location).into()));
            sources
        }
        ["tokens", "prophecy" | "olive" | "katarina", _, card] => {
            let mut sources = cards(card);
            sources.push(Source::Bag);
            sources
        }
        ["tokens", "jacqueline", rest @ ..] => {
            let mut sources = vec![Source::Investigator(Investigator::Jacqueline), Source::Bag];
            if let [.., "arbiter", asset] = rest {
                sources.extend(cards(asset));
            }
            sources
        }
        ["tokens", "cancel" | "keep" | "effect", index] => {
            let mut sources = vec![Source::Bag];
            if let Ok(index) = index.parse() {
                sources.push(Source::Token(index));
            }
            sources
        }
        // Position indices identify nested reveal groups, not physical tokens.
        // Public observations do not expose group membership; use the bag.
        ["tokens", "position" | "cancel-position" | "eyes", _] => vec![Source::Bag],
        // These decisions name no card identity in the public choice protocol.
        // They belong to the decision maker, never to a guessed physical copy.
        ["test-card", "prescient", "even" | "odd" | "symbol"]
        | ["test-card", "number", "up" | "down" | "confirm" | "cancel"]
        | ["result", _] => owned(Source::Investigator),
        _ => return Route::Pending,
    };
    admitted_sources(view, sources)
}

fn global(id: &str) -> bool {
    matches!(
        id,
        "confirm-mulligan"
            | "undo"
            | "turn:end"
            | "commit:confirm"
            | "tokens:confirm"
            | "harm:confirm"
            | "idol-pay"
            | "idol-cancel"
            | "lucky:decline"
            | "window:pass"
            | "phase-window:pass"
            | "success:pass"
            | "after-test:pass"
            | "test-card:pass"
            | "patrice-shuffle:decline"
            | "backpack:decline"
            | "backpack:attach"
            | "search:decline-hunt"
            | "search:finish-event"
            | "search:no-match"
            | "search:finish"
            | "spell-move:decline"
            | "defeat:done"
            | "peter:decline"
            | "act:cancel"
            | "lita:successor"
            | "resolution:burn"
            | "resolution:refuse"
            | "round-end:finish"
            | "round-end:decline-act"
            | "enemy:attacks"
            | "enemy:attacks-complete"
            | "enemy:next-investigator"
            | "enemy:attack-complete"
            | "enemy:end"
            | "upkeep:ready"
            | "upkeep:draws-complete"
            | "upkeep:resources"
            | "upkeep:hand-limits-complete"
            | "upkeep:next-hand"
            | "round:end"
            | "mythos:begin"
            | "mythos:encounters"
            | "mythos:draws-complete"
            | "investigation:begin"
    )
}

fn current_location(view: &Observation, owner: Option<Investigator>) -> Vec<Source> {
    view.play
        .as_ref()
        .and_then(|p| p.players.iter().find(|p| Some(p.investigator) == owner))
        .map(|p| vec![Source::Location(p.location.clone())])
        .unwrap_or_default()
}

fn indexed_investigator(index: &str) -> Vec<Source> {
    index
        .parse::<usize>()
        .ok()
        .and_then(|index| Investigator::ALL.get(index))
        .map(|who| vec![Source::Investigator(*who)])
        .unwrap_or_default()
}

fn investigator(id: &str) -> Option<Investigator> {
    Investigator::ALL.into_iter().find(|who| who.id() == id)
}

fn admitted_sources(view: &Observation, sources: Vec<Source>) -> Route {
    let mut admitted = vec![];
    for source in sources {
        if exists(view, &source) && !admitted.contains(&source) {
            admitted.push(source);
        }
    }
    if admitted.is_empty() {
        Route::Pending
    } else {
        Route::Sources(admitted)
    }
}

/// Sources are admitted only from the same public observation that supplies the
/// choices. Hidden deck order and private continuation frames are never read.
pub(super) fn exists(view: &Observation, source: &Source) -> bool {
    match source {
        Source::Investigator(who)
        | Source::Deck(who)
        | Source::Discard(who)
        | Source::Removed(who) => view.investigators.iter().any(|p| p.investigator == *who),
        Source::Card(id) => !card_sources(view, *id).is_empty(),
        Source::Location(code) => view.play.as_ref().map_or_else(
            || view.scenario_for_display().locations.contains(code),
            |play| {
                play.scenario.locations.iter().any(|l| &l.code == code)
                    && !play.scenario.location_in_victory_display(code)
            },
        ),
        Source::Act | Source::Agenda | Source::EncounterDeck | Source::ScenarioReference => true,
        Source::Bag => true,
        Source::Token(index) => view.play.as_ref().is_some_and(|play| {
            play.tokens.as_ref().map_or_else(
                || {
                    play.test
                        .as_ref()
                        .is_some_and(|test| *index < test.revealed.len())
                },
                |tokens| *index < tokens.revealed.len(),
            )
        }),
        Source::EncounterDiscard | Source::Victory | Source::ScenarioRemoved => view.play.is_some(),
        Source::SearchPlayer => view
            .play
            .as_ref()
            .is_some_and(|p| !p.searched_player.is_empty()),
        Source::SearchEncounter => view
            .play
            .as_ref()
            .is_some_and(|p| !p.searched_encounter.is_empty()),
        Source::Resolving => view.play.as_ref().is_some_and(|p| {
            !p.pending.resolving_cards.is_empty() || p.pending_revelation.is_some()
        }),
        Source::PendingDiscard => view
            .play
            .as_ref()
            .is_some_and(|p| p.pending.discard.is_some()),
        Source::Backpack(id) => view
            .play
            .as_ref()
            .is_some_and(|p| p.backpacks.iter().any(|b| b.asset == *id)),
        Source::Lita => view.play.as_ref().is_some_and(|p| p.scenario.lita_in_play),
    }
}

fn card_sources(view: &Observation, id: u16) -> Vec<Source> {
    let mut sources = vec![];
    let mut add = |cards: &[crate::gathering::CardView], container: Option<Source>| {
        if cards.iter().any(|card| card.id == id) {
            if !sources.contains(&Source::Card(id)) {
                sources.push(Source::Card(id));
            }
            if let Some(container) = container
                && !sources.contains(&container)
            {
                sources.push(container);
            }
        }
    };
    for player in &view.investigators {
        add(&player.hand, None);
        add(&player.permanents, None);
    }
    if let Some(play) = &view.play {
        for player in &play.players {
            add(&player.hand, None);
            add(&player.threats, None);
            add(&player.discard, Some(Source::Discard(player.investigator)));
            add(&player.removed, Some(Source::Removed(player.investigator)));
            for asset in &player.assets {
                add(
                    std::slice::from_ref(&asset.card),
                    play.backpacks
                        .iter()
                        .any(|pack| pack.asset == asset.card.id)
                        .then_some(Source::Backpack(asset.card.id)),
                );
            }
        }
        for enemy in &play.scenario.enemies {
            add(std::slice::from_ref(&enemy.card), None);
        }
        for location in &play.scenario.locations {
            if let Some(fog) = &location.fog {
                add(std::slice::from_ref(fog), None);
            }
        }
        if let Some(sealed) = &play.premonition {
            add(std::slice::from_ref(&sealed.card), None);
        }
        add(&play.encounter_discard, Some(Source::EncounterDiscard));
        add(&play.scenario.victory, Some(Source::Victory));
        add(&play.scenario.removed, Some(Source::ScenarioRemoved));
        add(&play.searched_player, Some(Source::SearchPlayer));
        add(&play.searched_encounter, Some(Source::SearchEncounter));
        add(&play.pending.resolving_cards, Some(Source::Resolving));
        if let Some(discard) = &play.pending.discard {
            add(&discard.remaining, Some(Source::PendingDiscard));
            add(&discard.ordered, Some(Source::PendingDiscard));
        }
        if let Some(revelation) = &play.pending_revelation {
            add(
                std::slice::from_ref(&revelation.card),
                Some(Source::Resolving),
            );
        }
        if let Some(test) = &play.test {
            if let Some(card) = &test.source_card {
                add(std::slice::from_ref(card), None);
            }
            for card in &test.committed {
                add(std::slice::from_ref(&card.card), None);
            }
        }
        for backpack in &play.backpacks {
            add(&backpack.cards, Some(Source::Backpack(backpack.asset)));
        }
    }
    sources
}

pub(super) fn source_choices(view: &Observation, source: &Source) -> Vec<Choice> {
    let enemies = unengaged_enemies(view, source);
    view.choices
        .iter()
        .filter_map(|choice| {
            let Route::Sources(sources) = route(view, &choice.id) else {
                return None;
            };
            let enemy = enemies
                .iter()
                .find(|enemy| sources.contains(&Source::Card(enemy.id)));
            if !sources.contains(source) && enemy.is_none() {
                return None;
            }
            let mut displayed = display_choice(view, choice);
            if let Some(enemy) = enemy {
                let copies = enemies
                    .iter()
                    .filter(|other| other.title == enemy.title)
                    .collect::<Vec<_>>();
                if copies.len() > 1 {
                    let ordinal = copies
                        .iter()
                        .position(|other| other.id == enemy.id)
                        .expect("The target is in its public group")
                        + 1;
                    displayed.label = format!(
                        "{} · {ordinal} of {}: {}",
                        enemy.title,
                        copies.len(),
                        displayed.label
                    );
                }
            }
            Some(displayed)
        })
        .collect()
}

pub(super) fn global_choices(view: &Observation) -> Vec<Choice> {
    view.choices
        .iter()
        .filter(|choice| route(view, &choice.id) == Route::Global)
        .map(|choice| display_choice(view, choice))
        .collect()
}

/// Administrative steps have no strategic alternative. The UI still commits
/// their existing revision-bound choices, preserving saves and the audit trail.
pub(super) fn automatic_choice(view: &Observation) -> Option<&str> {
    if view
        .choices
        .iter()
        .any(|choice| choice.id == "enemy:skip-empty")
    {
        return Some("enemy:skip-empty");
    }
    let mut choices = view.choices.iter().filter(|choice| choice.id != "undo");
    let choice = choices.next()?;
    if choices.next().is_some() {
        return None;
    }
    matches!(
        choice.id.as_str(),
        "enemy:attacks"
            | "enemy:next-investigator"
            | "enemy:attacks-complete"
            | "enemy:attack-complete"
            | "enemy:end"
            | "phase-window:pass"
    )
    .then_some(choice.id.as_str())
}

pub(super) fn window_opportunities(view: &Observation) -> Vec<(Source, String)> {
    if !view
        .choices
        .iter()
        .any(|choice| matches!(choice.id.as_str(), "window:pass" | "phase-window:pass"))
    {
        return vec![];
    }
    let mut result = Vec::new();
    for choice in &view.choices {
        let Route::Sources(mut sources) = route(view, &choice.id) else {
            continue;
        };
        // The other Violin route is its discard payment, not a playable card.
        if choice.id.starts_with("violin:") {
            sources.truncate(1);
        }
        for source in sources {
            if result.iter().any(|(prior, _)| prior == &source) {
                continue;
            }
            let label = match &source {
                Source::Card(id) | Source::Backpack(id) => view
                    .play
                    .as_ref()
                    .and_then(|play| {
                        play.players.iter().find_map(|player| {
                            player
                                .hand
                                .iter()
                                .chain(player.assets.iter().map(|asset| &asset.card))
                                .find(|card| card.id == *id)
                                .map(|card| {
                                    format!("{} — {}", player.investigator.name(), card.title)
                                })
                        })
                    })
                    .unwrap_or_else(|| choice.label.clone()),
                Source::Investigator(owner) => format!("{} — {}", owner.name(), choice.label),
                _ => continue,
            };
            result.push((source, label));
        }
    }
    result
}

pub(super) fn pending_choices(view: &Observation) -> Vec<Choice> {
    view.choices
        .iter()
        .filter(|choice| route(view, &choice.id) == Route::Pending)
        .map(|choice| display_choice(view, choice))
        .collect()
}

/// An admitted result window. Recognition is cheap for modal/input checks;
/// presentation choices are derived only when the renderer requests them.
pub(super) struct ResultOpportunities<'a> {
    view: &'a Observation,
    decline: &'a Choice,
}

pub(super) fn result_opportunities(view: &Observation) -> Option<ResultOpportunities<'_>> {
    // Small Favor also defeats enemies without a skill test. That window keeps
    // its ordinary reaction route instead of reopening an earlier result.
    if !view.choices.iter().any(|choice| {
        result_decline(choice)
            && (choice.id != "defeat:done"
                || view.play.as_ref().is_some_and(|play| play.test.is_some()))
    }) {
        return None;
    }
    Some(ResultOpportunities {
        view,
        // Admission and footer selection are distinct: preserve the first
        // recognized decline even when a later one admitted this window.
        decline: view.choices.iter().find(|choice| result_decline(choice))?,
    })
}

impl ResultOpportunities<'_> {
    pub(super) fn uses(&self) -> Vec<Choice> {
        self.view
            .choices
            .iter()
            .filter(|choice| choice.id != "undo" && !result_decline(choice))
            .map(|choice| self.displayed(choice))
            .collect()
    }

    pub(super) fn decline(&self) -> Choice {
        self.displayed(self.decline)
    }

    fn displayed(&self, choice: &Choice) -> Choice {
        let mut shown = choice.clone();
        shown.label = self.label(choice);
        shown
    }

    fn label(&self, choice: &Choice) -> String {
        if choice.id == "defeat:done" {
            return if self
                .view
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("defeat:bleeds:"))
            {
                "Decline If it bleeds...".into()
            } else {
                "Finish enemy-defeat reactions".into()
            };
        }
        if let Some(location) = choice
            .id
            .strip_prefix("spell-move:")
            .filter(|value| *value != "decline")
        {
            return format!(
                "Move to {} (Mists of R'lyeh)",
                play::location_name(location)
            );
        }
        if let Some(owner) = choice.id.strip_prefix("tony:quick-thinking:") {
            let name = match owner {
                "tony" => "Tony",
                "patrice" => "Patrice",
                "jacqueline" => "Jacqueline",
                _ => owner,
            };
            return format!("Take {name}'s extra action (Quick Thinking)");
        }
        if matches!(choice.id.as_str(), "after-test:pass" | "success:pass") {
            // Other declines count here too; this is wording context, not the
            // use group. A second offered decline retains the plural wording.
            let others: Vec<_> = self
                .view
                .choices
                .iter()
                .filter(|other| other.id != choice.id && other.id != "undo")
                .collect();
            if others.len() == 1 && others[0].id.starts_with("tony:quick-thinking:") {
                return "Decline Quick Thinking".into();
            }
            return "Decline all listed effects".into();
        }
        display_history(self.view, &choice.label)
    }
}

fn result_decline(choice: &Choice) -> bool {
    matches!(
        choice.id.as_str(),
        "after-test:pass"
            | "success:pass"
            | "lucky:decline"
            | "patrice-shuffle:decline"
            | "spell-move:decline"
            | "lita:decline-damage"
            | "defeat:done"
    )
}

// Spell-event choices retain canonical physical identities in their IDs and
// observations. Their existing "(copy N)" annotations are unnecessary on screen.
fn display_choice(view: &Observation, choice: &Choice) -> Choice {
    let mut shown = choice.clone();
    if choice.id == "test:reveal"
        && let Some(label) = reveal::announcement(view)
    {
        shown.label = label;
        return shown;
    }
    if let Some(id) = choice
        .id
        .strip_prefix("opportunity:")
        .and_then(|id| id.parse::<u16>().ok())
    {
        shown.label = public_title(view, id).map_or_else(
            || "Resolve this enemy's attack of opportunity".into(),
            |title| format!("Resolve {title}'s attack of opportunity"),
        );
        return shown;
    }
    if let Some(id) = choice
        .id
        .strip_prefix("turn-end:")
        .and_then(|id| id.parse::<u16>().ok())
    {
        shown.label = public_title(view, id).map_or_else(
            || "Resolve this end-of-turn effect".into(),
            |title| format!("Resolve {title}'s end-of-turn effect"),
        );
        return shown;
    }
    shown.label = display_history(view, &choice.label);
    shown
}

fn public_title(view: &Observation, id: u16) -> Option<&str> {
    for player in &view.investigators {
        if let Some(card) = player
            .hand
            .iter()
            .chain(&player.permanents)
            .find(|card| card.id == id)
        {
            return Some(&card.title);
        }
    }
    let play = view.play.as_ref()?;
    for player in &play.players {
        if let Some(card) = player
            .hand
            .iter()
            .chain(&player.threats)
            .chain(&player.discard)
            .chain(&player.removed)
            .chain(player.assets.iter().map(|asset| &asset.card))
            .find(|card| card.id == id)
        {
            return Some(&card.title);
        }
    }
    play.scenario
        .enemies
        .iter()
        .map(|enemy| &enemy.card)
        .chain(&play.encounter_discard)
        .chain(&play.scenario.victory)
        .chain(&play.scenario.removed)
        .chain(&play.pending.resolving_cards)
        .find(|card| card.id == id)
        .map(|card| card.title.as_str())
}

/// Historical records keep their original audit text. The player-facing view
/// replaces physical identities only in the known generated annotations.
pub(super) fn display_history(view: &Observation, original: &str) -> String {
    let mut remaining = original;
    let mut label = String::new();
    while let Some(start) = remaining.find(" (copy ") {
        label.push_str(&remaining[..start]);
        let annotation = &remaining[start + 7..];
        let Some(end) = annotation.find(')') else {
            label.push_str(&remaining[start..]);
            remaining = "";
            break;
        };
        if annotation[..end].is_empty()
            || !annotation[..end].bytes().all(|byte| byte.is_ascii_digit())
        {
            label.push_str(&remaining[start..start + 8 + end]);
        }
        remaining = &annotation[end + 1..];
    }
    label.push_str(remaining);
    for (marker, fallback, suffix) in [
        ("enemy ", "the enemy", ""),
        ("end-of-turn effect ", "the card", "'s end-of-turn effect"),
    ] {
        let mut remaining = label.as_str();
        let mut result = String::new();
        while let Some(start) = remaining.find(marker) {
            result.push_str(&remaining[..start]);
            let tail = &remaining[start + marker.len()..];
            let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
            if digits == 0 {
                result.push_str(marker);
                remaining = tail;
                continue;
            }
            let title = tail[..digits]
                .parse::<u16>()
                .ok()
                .and_then(|id| public_title(view, id))
                .unwrap_or(fallback);
            result.push_str(title);
            result.push_str(suffix);
            remaining = &tail[digits..];
        }
        result.push_str(remaining);
        label = result;
    }
    label
}

#[derive(Component)]
pub(super) struct Menu;

#[derive(Component)]
pub(super) struct Backdrop;

pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, backdrop)
        .add_systems(Update, keyboard_scroll.after(handle_actions));
}

fn backdrop(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn((
            Backdrop,
            Mesh3d(meshes.add(Plane3d::default().mesh().size(200., 200.))),
            MeshMaterial3d(materials.add(INK)),
            Transform::from_xyz(0., -0.5, 0.),
        ))
        .observe(|event: On<PointerClick>, mut queue: ResMut<Actions>| {
            if event.button == bevy::picking::pointer::PointerButton::Primary {
                queue.0.push(Action::DismissContext);
            }
        });
}

fn keyboard_scroll(
    keys: Res<ButtonInput<KeyCode>>,
    table: Res<Table>,
    mut menus: Query<(&mut ScrollPosition, &ComputedNode), With<Menu>>,
) {
    if !table.context.is_open() || table.modal() {
        return;
    }
    for (mut scroll, node) in &mut menus {
        scroll_with_keys(&keys, &mut scroll, node);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn render(
    mut commands: Commands,
    table: Res<Table>,
    art: Res<art::Art>,
    health: Res<RenderHealth>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    menus: Query<Entity, With<Menu>>,
    mut previous: Local<Option<(u64, u64, u64, bool, Vec2)>>,
) {
    let size = Vec2::new(window.width(), window.height());
    let Some(view) = table.last_rendered.as_ref() else {
        return;
    };
    let blocked = table.modal() || !health.allows_play() || health.just_became_ready();
    let key = (
        table.context.generation(),
        table.epoch,
        view.revision,
        blocked,
        size,
    );
    if previous.as_ref() == Some(&key) {
        return;
    }
    *previous = Some(key);
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    if !table.context.open || blocked {
        return;
    }
    let (title, anchor, choices) = if table.context.pending {
        ("Current decision", size * 0.5, pending_choices(view))
    } else if let Some(selection) = &table.context.selected {
        (
            selection.title.as_str(),
            camera
                .0
                .world_to_viewport(camera.1, selection.at)
                .unwrap_or(size * 0.5),
            source_choices(view, &selection.source),
        )
    } else {
        return;
    };
    use feedback::menu::{Item, MenuText};
    let mut content = vec![
        Item::Text(title.to_owned(), 18., GOLD),
        Item::Text(feedback::KEYBOARD_HINT.to_owned(), 14., MUTED),
    ];
    if let Some(owner) = view.decision_owner {
        content.push(Item::Text(
            format!("{}'s decision", owner.name()),
            14.,
            PAPER,
        ));
    }
    for choice in choices.into_iter().filter(|_| !table.is_presenting()) {
        content.push(Item::Action(
            feedback::menu::label(&choice.label),
            Action::ContextChoice(
                table.context.generation(),
                table.epoch,
                view.revision,
                choice.id.clone(),
            ),
        ));
    }
    if let Some(selection) = &table.context.selected {
        let targets = inspections(view, selection);
        for (index, target) in targets.iter().enumerate() {
            let label = if matches!(selection.source, Source::Bag | Source::Token(_)) {
                "Inspect scenario reference".into()
            } else if targets.len() == 1 {
                "Inspect".into()
            } else {
                let count = targets
                    .iter()
                    .filter(|other| other.title == target.title)
                    .count();
                if count > 1 {
                    let ordinal = targets[..=index]
                        .iter()
                        .filter(|other| other.title == target.title)
                        .count();
                    format!("Inspect {} · {ordinal} of {count}", target.title)
                } else {
                    format!("Inspect {}", target.title)
                }
            };
            let label = target
                .details
                .as_ref()
                .map_or(label.clone(), |details| format!("{label}\n{details}"));
            content.push(Item::Action(
                label,
                Action::ContextInspect(
                    table.context.generation(),
                    table.epoch,
                    view.revision,
                    target.title.clone(),
                    target.code.clone(),
                ),
            ));
        }
    }
    content.push(Item::Action("Dismiss".into(), Action::DismissContext));
    let menu = feedback::menu::spawn(
        &mut commands,
        feedback::menu::panel(anchor, size, 110., (size.y * 0.29).min(250.) + 20.),
        (BackgroundColor(INK), GlobalZIndex(50), Menu),
        MenuText {
            symbol_font: art.symbol_font(),
            action_size: 14.,
            action_pickable: Pickable::IGNORE,
        },
        content,
        |row, action| {
            row.observe(move |_: On<Activate>, mut queue: ResMut<Actions>| {
                queue.0.push(action.clone());
            });
        },
    );
    commands.entity(menu).observe(scroll_panel);
}

#[cfg(test)]
mod tests;
