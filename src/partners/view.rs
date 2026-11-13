//! A partner's view: what a player of its investigators would see. It is built
//! field by field from the rules session's observation, so nothing reaches it by
//! default: public information, and the hidden information of the partner's own
//! investigators only. Other hands, deck order and the encounter deck are absent.
//! Other players are named only by their investigators.

use super::{Control, Controller, Routing, Shared, SharedKind, routing};
use crate::gathering::{CardView, Investigator, OptionOwner, Session, inspection};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// A shared decision this player takes part in, as its view gives it.
pub(super) struct SharedPart<'a> {
    pub decision: &'a Shared,
    pub view: Value,
    /// Whether it waits on its players now.
    pub waits: bool,
}

/// Watcher from Another Dimension: "Hidden. Revelation - Secretly add this enemy
/// to your hand". Only its holder's player knows where it is.
const HIDDEN_IN_HAND: &str = "06017";
/// The three investigator cards, which every player sees.
const INVESTIGATOR_CARDS: [&str; 3] = ["06003", "06005", "60401"];

pub(super) struct Viewer<'a> {
    pub partner: &'a str,
    /// Card text already sent to this partner, by code and visible face.
    pub sent: &'a mut BTreeSet<String>,
    /// Send every visible card's text again.
    pub all_cards: bool,
}

pub(super) fn build(
    session: &Session,
    control: &Control,
    routing: &Routing,
    viewer: Viewer<'_>,
    awaited: &[(Controller, Option<Investigator>)],
    shared: &[SharedPart<'_>],
) -> Value {
    let me = Controller::Partner(viewer.partner.to_owned());
    let mine: Vec<_> = Investigator::ALL
        .into_iter()
        .filter(|who| control.controller(*who) == me)
        .collect();
    let is_mine = |who: Investigator| mine.contains(&who);
    let observation = session.observe();
    let mut cards = Cards::new(viewer.sent, viewer.all_cards);
    for code in INVESTIGATOR_CARDS {
        cards.see(code, false);
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
        "decision": decision(control, routing, &me, &mine, &observation, awaited, shared),
        "investigators": investigators,
    });
    let fields = view.as_object_mut().expect("An object");
    if !shared.is_empty() {
        fields.insert(
            "shared".into(),
            Value::Array(shared.iter().map(|part| part.view.clone()).collect()),
        );
    }
    // In a commitment it takes part in, every player's pending cards are visible.
    let sees_pending = shared
        .iter()
        .any(|part| part.decision.kind == SharedKind::Commitment);
    if let Some(play) = &observation.play {
        let act = format!("{:05}", 1107 + usize::from(play.act));
        let agenda = format!("{:05}", 1104 + usize::from(play.agenda));
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
        let visible =
            |card: &CardView, holder: Investigator| card.code != HIDDEN_IN_HAND || is_mine(holder);
        if let Some(revelation) = play
            .pending_revelation
            .as_ref()
            .filter(|revelation| visible(&revelation.card, revelation.drawn_by))
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
            .filter(|card| card.code != HIDDEN_IN_HAND || is_mine(Investigator::Patrice))
            .map(|card| {
                cards.see(&card.code, false);
                card_json(card)
            })
            .collect();
        if !resolving.is_empty() {
            fields.insert("resolving".into(), Value::Array(resolving));
        }
        if let Some(harm) = &pending.harm {
            fields.insert("harm".into(), json!(harm));
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
            // Another player's investigators' cards are in their hands: only the
            // ones pending in a commitment this player shares are shown.
            let shown = |id: u64| {
                own.iter().any(|own| u64::from(*own) == id)
                    || (sees_pending && test.selected.iter().any(|card| u64::from(*card) == id))
            };
            if let Some(commitment) = value["commitment"].as_array_mut() {
                commitment.retain(|entry| entry["card_id"].as_u64().is_some_and(shown));
            }
            if let Some(selected) = value["selected"].as_array_mut() {
                selected.retain(|id| id.as_u64().is_some_and(shown));
            }
            if sees_pending {
                let pending: Vec<Value> = play
                    .players
                    .iter()
                    .filter(|player| !is_mine(player.investigator))
                    .flat_map(|player| {
                        player
                            .hand
                            .iter()
                            .filter(|card| test.selected.contains(&card.id))
                            .map(move |card| (player.investigator, card))
                    })
                    .map(|(owner, card)| {
                        cards.see(&card.code, false);
                        json!({"investigator": owner, "id": card.id, "code": card.code, "title": card.title})
                    })
                    .collect();
                value["others_pending"] = Value::Array(pending);
            }
            fields.insert("test".into(), value);
        }
        if let Some(last) = play.completed_tests.last() {
            fields.insert("last_test".into(), json!(last));
        }
    }
    fields.insert("cards".into(), Value::Object(cards.into_text()));
    view
}

/// The pending decision as this player meets it: its own options, and who else
/// decides or may act now, by investigator. A shared decision's completing option
/// is listed for every player taking part: choosing it is readiness.
fn decision(
    control: &Control,
    routing: &Routing,
    me: &Controller,
    mine: &[Investigator],
    observation: &crate::gathering::Observation,
    awaited: &[(Controller, Option<Investigator>)],
    shared: &[SharedPart<'_>],
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
    for part in shared {
        let decision = part.decision;
        let investigator = decision
            .participants
            .iter()
            .find(|who| mine.contains(who))
            .copied();
        let mut ids = vec![decision.close.as_str()];
        if decision.kind == (SharedKind::Payment { at_round_end: true }) {
            ids.push("round-end:decline-act");
        }
        for id in ids {
            let mut label = label(id);
            if decision.kind != (SharedKind::Payment { at_round_end: true }) {
                label = format!("{label} (marks you ready)");
            }
            options.push(json!({"id": id, "label": label, "investigator": investigator}));
        }
    }
    for (player, who) in awaited {
        if let Some(who) = who
            && player != me
            && !mine.contains(who)
        {
            others.insert(who.index());
        }
    }
    let mut decision = json!({
        "yours": awaited.iter().any(|(player, _)| player == me),
        "prompt": observation.prompt,
        "options": options,
        "others_deciding": others.into_iter().map(|index| Investigator::ALL[index].name()).collect::<Vec<_>>(),
    });
    let note = if table {
        Some("The lead investigator is chosen at the table.")
    } else if routing.with_human() && routing.awaiting() != Some(me) {
        Some("Group choices are made at the table in this version; wait for your next decision.")
    } else if shared.iter().any(|part| !part.waits) {
        Some(
            "A group clue payment is possible: choose an allocation to propose your investigators' part.",
        )
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
