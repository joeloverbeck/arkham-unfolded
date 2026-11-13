//! A partner's view: what a player of its investigators would see. It is built
//! field by field from the rules session's observation, so nothing reaches it by
//! default: public information, and the hidden information of the partner's own
//! investigators only. Other hands, deck order and the encounter deck are absent.
//! Other players are named only by their investigators.
//!
//! A partner session's first view is complete. Every later view carries the fields
//! in [`NOW`] whole, the decision and what it is about, and reports the rest of the
//! table, the board, only as `changes` from the board this partner last received.
//! Changes are computed between two of this partner's own views, so they hold
//! nothing either view did not: no history line, label or caption reaches them.

use super::{Control, Controller, Routing, routing};
use crate::gathering::{
    CardView, Investigator, OptionOwner, Session,
    inspection::{self, INVESTIGATOR_CARDS, Sight},
};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// The fields every view carries whole: where the game is, the pending decision and
/// what it is about. Every other field is the board, sent whole in a complete view
/// and otherwise reported as changes.
const NOW: [&str; 11] = [
    "revision",
    "stage",
    "you_play",
    "round",
    "phase",
    "turn",
    "decision",
    "test",
    "encounter_card",
    "harm",
    "resolving",
];

/// Board fields reported whole when they change: a completed test is a new result,
/// not an edit of the previous one.
const RECORDS: [&str; 1] = ["last_test"];

/// The opening scenario cards every player sees during setup: the scenario
/// reference, Agenda 1a, Act 1a and the Study.
const SETUP_CARDS: [(&str, bool); 4] = [
    ("01104", false),
    ("01105", false),
    ("01108", false),
    ("01111", true),
];

pub(super) struct Viewer<'a> {
    pub partner: &'a str,
    /// Card text already sent to this partner, by code and visible face.
    pub sent: &'a mut BTreeSet<String>,
    /// The board of the last view this partner received; none before its first.
    pub board: &'a mut Option<Map<String, Value>>,
    /// Send the complete view and every visible card's text again.
    pub full: bool,
}

pub(super) fn build(
    session: &Session,
    control: &Control,
    routing: &Routing,
    viewer: Viewer<'_>,
) -> Value {
    let me = Controller::Partner(viewer.partner.to_owned());
    let mine: Vec<_> = Investigator::ALL
        .into_iter()
        .filter(|who| control.controller(*who) == me)
        .collect();
    let is_mine = |who: Investigator| mine.contains(&who);
    let observation = session.observe();
    let full = viewer.full || viewer.board.is_none();
    let mut cards = Cards::new(viewer.sent, full);
    for (code, _) in INVESTIGATOR_CARDS {
        cards.see(code, false);
    }
    if observation.play.is_none() {
        for (code, revealed) in SETUP_CARDS {
            cards.see(code, revealed);
        }
    }

    let investigators: Vec<Value> = Investigator::ALL
        .into_iter()
        .map(|who| {
            let setup = &observation.investigators[who.index()];
            let mut entry = json!({
                "investigator": who,
                "name": who.name(),
                "yours": is_mine(who),
            });
            let fields = entry.as_object_mut().expect("An object");
            let hand: &[CardView] = match &observation.play {
                Some(play) => {
                    let player = &play.players[who.index()];
                    let presentation = &player.presentation;
                    fields.extend(object(json!({
                        "location": location(&player.location),
                        "resources": player.resources,
                        "clues": player.clues,
                        "damage": player.damage,
                        "horror": player.horror,
                        "health": presentation.health,
                        "sanity": presentation.sanity,
                        "actions_remaining": play.actions_remaining[who.index()],
                        "turn_finished": presentation.turn_finished,
                        "hand_count": player.hand.len(),
                        "deck_count": player.deck_count,
                        "discard": titles(&player.discard),
                        "in_play": player.assets.iter().map(|asset| {
                            cards.see(&asset.card.code, false);
                            json!({
                                "id": asset.card.id,
                                "code": asset.card.code,
                                "title": asset.card.title,
                                "ready": asset.ready,
                                "uses": asset.uses,
                                "damage": asset.damage,
                                "horror": asset.horror,
                            })
                        }).collect::<Vec<_>>(),
                        "threats": player.threats.iter().map(|card| {
                            cards.see(&card.code, false);
                            card_json(card)
                        }).collect::<Vec<_>>(),
                        "defeated": player.defeated,
                        "resigned": player.resigned,
                    })));
                    &player.hand
                }
                None => {
                    fields.extend(object(json!({
                        "resources": setup.resources,
                        "health": setup.health(),
                        "sanity": setup.sanity(),
                        "hand_count": setup.hand.len(),
                        "deck_count": setup.deck_count,
                        "mulligan_complete": setup.mulligan_complete,
                    })));
                    if is_mine(who) {
                        fields.insert("replacing".into(), json!(setup.selected_mulligan));
                    }
                    &setup.hand
                }
            };
            fields.insert(
                "permanents".into(),
                Value::Array(
                    setup
                        .permanents
                        .iter()
                        .map(|card| {
                            cards.see(&card.code, false);
                            card_json(card)
                        })
                        .collect(),
                ),
            );
            if is_mine(who) {
                for card in hand {
                    cards.see(&card.code, false);
                }
                fields.insert(
                    "hand".into(),
                    Value::Array(hand.iter().map(card_json).collect()),
                );
            }
            entry
        })
        .collect();

    let mut view = json!({
        "revision": observation.revision,
        "stage": observation.stage,
        "you_play": mine,
        "lead": observation.lead,
        "decision": decision(control, routing, &me, &mine, &observation),
        "investigators": investigators,
    });
    let fields = view.as_object_mut().expect("An object");
    if let Some(play) = &observation.play {
        let act = format!("{:05}", 1107 + usize::from(play.act));
        let agenda = format!("{:05}", 1104 + usize::from(play.agenda));
        cards.see("01104", false);
        cards.see(&act, false);
        cards.see(&agenda, false);
        let scenario = &play.scenario;
        fields.extend(object(json!({
            "round": play.round,
            "phase": play.phase,
            "turn": play.actor,
            "act": {"code": act, "title": title(&act)},
            "agenda": {"code": agenda, "title": title(&agenda), "doom": play.agenda_doom},
            "locations": scenario.locations.iter().map(|room| {
                cards.see(&room.code, room.revealed);
                json!({
                    "code": room.code,
                    "name": title(&room.code),
                    "revealed": room.revealed,
                    "clues": room.clues,
                    "attached": room.fog.as_ref().map(|fog| {
                        cards.see(&fog.code, false);
                        fog.title.clone()
                    }),
                })
            }).collect::<Vec<_>>(),
            "enemies": scenario.enemies.iter().map(|enemy| {
                cards.see(&enemy.card.code, false);
                json!({
                    "id": enemy.card.id,
                    "code": enemy.card.code,
                    "title": enemy.card.title,
                    "location": location(&enemy.location),
                    "engaged": enemy.engaged,
                    "exhausted": enemy.exhausted,
                    "damage": enemy.damage,
                    "health": enemy.health(),
                    "doom": enemy.doom,
                })
            }).collect::<Vec<_>>(),
            "encounter_deck_count": play.encounter_deck_count,
            "encounter_discard": titles(&play.encounter_discard),
            "victory": titles(&scenario.victory),
            "chaos_bag_count": play.bag_count,
            "outcome": scenario.outcome,
        })));
        let sight = Sight::Player(&mine);
        if let Some(revelation) = play
            .pending_revelation
            .as_ref()
            .filter(|revelation| sight.shows(&revelation.card))
        {
            cards.see(&revelation.card.code, false);
            fields.insert(
                "encounter_card".into(),
                json!({
                    "card": card_json(&revelation.card),
                    "drawn_by": revelation.drawn_by,
                }),
            );
        }
        let pending = &play.pending;
        let resolving: Vec<_> = pending
            .resolving_cards
            .iter()
            .filter(|card| sight.shows(card))
            .map(|card| {
                cards.see(&card.code, false);
                card_json(card)
            })
            .collect();
        if !resolving.is_empty() {
            fields.insert("resolving".into(), Value::Array(resolving));
        }
        if let Some(harm) = &pending.harm {
            let mut value = json!(harm);
            // A hidden card attacking from its holder's hand names itself only to
            // that player: the others see the harm, not its source.
            let (hidden, holder) = inspection::HIDDEN_IN_HAND;
            if harm.source.contains(&format!("[{hidden}]")) && !sight.sees(holder) {
                value.as_object_mut().expect("An object").remove("source");
            }
            fields.insert("harm".into(), value);
        }
        if let Some(test) = &play.test {
            let own: Vec<u16> = mine
                .iter()
                .flat_map(|who| play.players[who.index()].hand.iter().map(|card| card.id))
                .collect();
            let mut value = json!(test);
            if let Some(source) = &test.source_card {
                cards.see(&source.code, false);
            }
            // Pending commitments of another player's investigators are in their hands.
            if let Some(commitment) = value["commitment"].as_array_mut() {
                commitment.retain(|entry| {
                    entry["owner"]
                        .as_str()
                        .is_some_and(|owner| mine.iter().any(|who| who.id() == owner))
                });
            }
            if let Some(selected) = value["selected"].as_array_mut() {
                selected.retain(|id| {
                    id.as_u64()
                        .is_some_and(|id| own.iter().any(|own| u64::from(*own) == id))
                });
            }
            fields.insert("test".into(), value);
        }
        if let Some(last) = play.completed_tests.last() {
            fields.insert("last_test".into(), json!(last));
        }
    }
    let board: Map<String, Value> = fields
        .iter()
        .filter(|(field, _)| !NOW.contains(&field.as_str()))
        .map(|(field, value)| (field.clone(), value.clone()))
        .collect();
    let last = viewer.board.replace(board.clone());
    if let Some(last) = last.filter(|_| !full) {
        fields.retain(|field, _| NOW.contains(&field.as_str()));
        let mut changes = Vec::new();
        diff(
            "",
            &Value::Object(last),
            &Value::Object(board),
            &mut changes,
        );
        fields.insert("changes".into(), Value::Array(changes));
    }
    fields.insert("cards".into(), Value::Object(cards.into_text()));
    view
}

/// What changed from `old` to `new`, as entries naming where (`at`, a path of field
/// names and identities) and how: `was` and `now` for a changed value, `now` alone
/// for a new or replaced structure (`null` when it is gone), and `added` or
/// `removed` for members of a collection. Collections of cards, enemies, locations
/// and investigators are matched by identity, so a change inside one names it.
fn diff(at: &str, old: &Value, new: &Value, changes: &mut Vec<Value>) {
    if old == new {
        return;
    }
    let field = at.rsplit('.').next().unwrap_or(at);
    match (old, new) {
        (Value::Object(_), Value::Object(_)) if RECORDS.contains(&field) => {
            changes.push(json!({"at": at, "now": new}));
        }
        (Value::Object(previous), Value::Object(current)) if face(previous) => {
            changes.push(json!({"at": at, "was": old, "now": current}));
        }
        (Value::Object(previous), Value::Object(current)) => {
            for (key, value) in current {
                let path = join(at, key);
                match previous.get(key) {
                    Some(before) => diff(&path, before, value, changes),
                    None => changes.push(json!({"at": path, "now": value})),
                }
            }
            for key in previous.keys().filter(|key| !current.contains_key(*key)) {
                changes.push(json!({"at": join(at, key), "now": null}));
            }
        }
        (Value::Array(previous), Value::Array(current)) => {
            match (identified(previous), identified(current)) {
                (Some(before), Some(after)) => {
                    let added: Vec<&Value> = after
                        .iter()
                        .filter(|(key, _)| !before.iter().any(|(old, _)| old == key))
                        .map(|(_, value)| *value)
                        .collect();
                    let removed: Vec<Value> = before
                        .iter()
                        .filter(|(key, _)| !after.iter().any(|(new, _)| new == key))
                        .map(|(_, value)| reference(value))
                        .collect();
                    if !added.is_empty() {
                        changes.push(json!({"at": at, "added": added}));
                    }
                    if !removed.is_empty() {
                        changes.push(json!({"at": at, "removed": removed}));
                    }
                    for (key, value) in &after {
                        if let Some((_, before)) = before.iter().find(|(old, _)| old == key) {
                            diff(&join(at, key), before, value, changes);
                        }
                    }
                }
                _ if previous.iter().chain(current).all(scalar) => {
                    let added = without(current, previous);
                    let removed = without(previous, current);
                    if added.is_empty() && removed.is_empty() {
                        changes.push(json!({"at": at, "now": new}));
                    }
                    if !added.is_empty() {
                        changes.push(json!({"at": at, "added": added}));
                    }
                    if !removed.is_empty() {
                        changes.push(json!({"at": at, "removed": removed}));
                    }
                }
                _ => changes.push(json!({"at": at, "now": new})),
            }
        }
        _ if scalar(old) && scalar(new) => changes.push(json!({"at": at, "was": old, "now": new})),
        _ => changes.push(json!({"at": at, "now": new})),
    }
}

fn join(at: &str, key: &str) -> String {
    if at.is_empty() {
        key.to_owned()
    } else {
        format!("{at}.{key}")
    }
}

fn scalar(value: &Value) -> bool {
    !value.is_object() && !value.is_array()
}

/// A printed face rather than a physical card: a code with plain fields, such as a
/// location, act or agenda named where something is. It changes as a whole.
fn face(fields: &Map<String, Value>) -> bool {
    fields.contains_key("code") && !fields.contains_key("id") && fields.values().all(scalar)
}

/// The members of a collection by identity, when every member has one and no two
/// share it: an investigator, a physical card or enemy by id, a location by code.
fn identified(values: &[Value]) -> Option<Vec<(String, &Value)>> {
    let members: Vec<(String, &Value)> = values
        .iter()
        .map(|value| {
            let fields = value.as_object()?;
            let identity = ["investigator", "id", "code"]
                .into_iter()
                .find_map(|key| fields.get(key))?;
            Some((
                identity
                    .as_str()
                    .map_or_else(|| identity.to_string(), str::to_owned),
                value,
            ))
        })
        .collect::<Option<_>>()?;
    let mut keys: Vec<&String> = members.iter().map(|(key, _)| key).collect();
    keys.sort();
    keys.dedup();
    (keys.len() == members.len() && !members.is_empty()).then_some(members)
}

/// A removed member, named but not repeated.
fn reference(value: &Value) -> Value {
    value.as_object().map_or_else(
        || value.clone(),
        |fields| {
            Value::Object(
                fields
                    .iter()
                    .filter(|(key, _)| {
                        ["investigator", "id", "code", "title", "name"].contains(&key.as_str())
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )
        },
    )
}

/// The members of `from` left after removing one occurrence of each of `taken`.
fn without(from: &[Value], taken: &[Value]) -> Vec<Value> {
    let mut left = taken.to_vec();
    from.iter()
        .filter(
            |value| match left.iter().position(|taken| taken == *value) {
                Some(index) => {
                    left.remove(index);
                    false
                }
                None => true,
            },
        )
        .cloned()
        .collect()
}

/// The pending decision as this player meets it: its own options, and who else
/// decides or may act now, by investigator.
fn decision(
    control: &Control,
    routing: &Routing,
    me: &Controller,
    mine: &[Investigator],
    observation: &crate::gathering::Observation,
) -> Value {
    let label = |id: &str| {
        observation
            .choices
            .iter()
            .find(|choice| choice.id == id)
            .map_or(id, |choice| choice.label.as_str())
            .to_owned()
    };
    let mut options = Vec::new();
    let mut others = BTreeSet::new();
    let mut table = false;
    for (id, owner, controller) in routing.options() {
        if controller == me {
            let investigator = match owner {
                OptionOwner::Investigator(who) => Some(*who),
                _ => routing.awaited_investigator(),
            };
            options.push(json!({"id": id, "label": label(id), "investigator": investigator}));
            continue;
        }
        table |= matches!(owner, OptionOwner::Table);
        for who in routing::investigators(owner) {
            if control.controller(who) != *me {
                others.insert(who.index());
            }
        }
    }
    if let Some(who) = routing.awaited_investigator()
        && routing.awaiting() != Some(me)
        && !mine.contains(&who)
    {
        others.insert(who.index());
    }
    let mut decision = json!({
        "yours": routing.awaiting() == Some(me),
        "prompt": observation.prompt,
        "options": options,
        "others_deciding": others.into_iter().map(|index| Investigator::ALL[index].name()).collect::<Vec<_>>(),
    });
    // Every player hears a group choice open and what it is about; only its
    // arbiter's player has its options.
    if let Some(arbiter) = routing.group() {
        decision["group_choice"] = json!({
            "arbiter": arbiter,
            "yours": control.controller(arbiter) == *me,
            "choosing_among": observation
                .choices
                .iter()
                .filter(|choice| choice.id != "undo")
                .map(|choice| choice.label.as_str())
                .collect::<Vec<_>>(),
        });
    }
    // Undo is not an option of the pending decision: it reverts this player's own
    // latest decision, and is offered only while nothing has followed it.
    if routing.undoer() == Some(me) {
        decision["undo"] = json!({
            "id": "undo",
            "label": label("undo"),
            "investigator": routing
                .undo_owner()
                .and_then(|owner| routing::investigators(owner).first().copied()),
        });
    }
    let note = if table {
        Some("The lead investigator is chosen at the table.")
    } else if routing.with_human() && routing.awaiting() != Some(me) {
        Some("Shared decisions are made at the table in this version; wait for your next decision.")
    } else {
        None
    };
    if let Some(note) = note {
        decision["note"] = json!(note);
    }
    decision
}

/// Card text on first sight: each visible card's printed fields and effective
/// text, keyed by code, sent once per partner unless asked for again.
struct Cards<'a> {
    sent: &'a mut BTreeSet<String>,
    all: bool,
    text: Map<String, Value>,
}

impl<'a> Cards<'a> {
    fn new(sent: &'a mut BTreeSet<String>, all: bool) -> Self {
        Self {
            sent,
            all,
            text: Map::new(),
        }
    }

    fn see(&mut self, code: &str, revealed: bool) {
        let key = if revealed {
            format!("{code} revealed")
        } else {
            code.to_owned()
        };
        if self.text.contains_key(&key) || (!self.all && self.sent.contains(&key)) {
            return;
        }
        let text = inspection::card(code).or_else(|| inspection::scenario_card(code, revealed));
        if let Some(text) = text {
            self.sent.insert(key.clone());
            self.text.insert(key, Value::String(text));
        }
    }

    fn into_text(self) -> Map<String, Value> {
        self.text
    }
}

fn card_json(card: &CardView) -> Value {
    json!({"id": card.id, "code": card.code, "title": card.title})
}

fn titles(cards: &[CardView]) -> Vec<&str> {
    cards.iter().map(|card| card.title.as_str()).collect()
}

fn title(code: &str) -> &str {
    crate::gathering::card_title(code).unwrap_or(code)
}

fn location(code: &str) -> Value {
    json!({"code": code, "name": title(code)})
}

fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("An object literal"),
    }
}
