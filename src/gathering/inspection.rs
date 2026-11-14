//! Readable inspection of the fixed selected card definitions, and of the cards at
//! the table as one player sees them.

use super::{CardView, Investigator, Observation, cards};

/// Watcher from Another Dimension (06017): "Hidden. Revelation - Secretly add this
/// enemy to your hand." It is Patrice's signature weakness, the only hidden card in
/// the selected decks, so only her player may know where it is.
pub const HIDDEN_IN_HAND: (&str, Investigator) = ("06017", Investigator::Patrice);

/// Whose hidden cards an inspection may show.
#[derive(Clone, Copy, Debug)]
pub enum Sight<'a> {
    /// The window's table, which shows every hand and private pile to the human.
    Table,
    /// One player: public cards, and the hidden cards of these investigators only.
    Player(&'a [Investigator]),
}

impl Sight<'_> {
    /// Whether this sight includes `who`'s hidden cards.
    pub fn sees(self, who: Investigator) -> bool {
        match self {
            Self::Table => true,
            Self::Player(mine) => mine.contains(&who),
        }
    }

    /// Whether `card` may be shown: every card but a hidden one whose holder this
    /// sight does not include.
    pub fn shows(self, card: &CardView) -> bool {
        card.code != HIDDEN_IN_HAND.0 || self.sees(HIDDEN_IN_HAND.1)
    }
}

/// Inspect a card at the table by its code, as the table's Inspect shows it: an
/// investigator card with its asset slots, a location by its visible face, any other
/// card with its full printed fields and effective text, plus the Supply and Key
/// counters on it. `None` unless a card with this code is visible with `sight`, so
/// a card in a hand or deck it does not see is answered as a code that names no card.
pub fn table_card(observation: &Observation, code: &str, sight: Sight<'_>) -> Option<String> {
    let Some(play) = observation.play.as_ref() else {
        // Setup: the investigator cards, permanents, the hands this sight includes
        // and the scenario's opening cards.
        let visible = Investigator::ALL.iter().any(|who| who.card_code() == code)
            || ["01104", "01105", "01108", "01111"].contains(&code)
            || observation.investigators.iter().any(|setup| {
                setup.permanents.iter().any(|card| card.code == code)
                    || (sight.sees(setup.investigator)
                        && setup.hand.iter().any(|card| card.code == code))
            });
        return visible
            .then(|| card(code).or_else(|| scenario_card(code, code == "01111")))
            .flatten();
    };
    if let Some(player) = Investigator::ALL
        .iter()
        .find(|who| who.card_code() == code)
        .and_then(|who| {
            play.players
                .iter()
                .find(|player| player.investigator == *who)
        })
    {
        let slots = player
            .slot_usage()
            .iter()
            .map(|slot| slot.details())
            .collect::<Vec<_>>()
            .join("\n");
        let masks = player.masks_controlled();
        let mask_limit = if masks == 0 {
            String::new()
        } else {
            format!(
                "\n\nMask limit: {masks}/1\nYou cannot play or take control of another Mask while you control one. This is a control limit, not an equipment slot; it does not allow slot-style replacement."
            )
        };
        return Some(format!(
            "{}\n\nAsset slots\n{slots}\nCounts show occupied slot units. Slotless assets do not use these limits.{mask_limit}",
            card(code)?
        ));
    }
    if let Some(room) = play
        .scenario
        .locations
        .iter()
        .find(|room| room.code == code)
    {
        return scenario_card(code, room.revealed);
    }
    let here = |card: &CardView| card.code == code && sight.shows(card);
    let agenda = format!("{:05}", 1104 + usize::from(play.agenda));
    let act = format!("{:05}", 1107 + usize::from(play.act));
    let scenario_visible = code == "01104"
        || code == agenda
        || code == act
        || (code == "01117" && play.scenario.lita_in_play)
        || play
            .scenario
            .locations
            .iter()
            .any(|room| room.fog.as_ref().is_some_and(|card| card.code == code))
        || play
            .scenario
            .enemies
            .iter()
            .any(|enemy| enemy.card.code == code)
        || play
            .encounter_discard
            .iter()
            .chain(&play.scenario.victory)
            .chain(&play.scenario.removed)
            .any(|card| card.code == code)
        || play
            .pending_revelation
            .as_ref()
            .is_some_and(|pending| here(&pending.card));
    // A search shows its cards to the searcher, who makes the search's decision.
    let searcher = match sight {
        Sight::Table => true,
        Sight::Player(_) => observation
            .decision_owner
            .is_some_and(|who| sight.sees(who)),
    };
    let player_visible = play.pending.resolving_cards.iter().any(here)
        || play.pending.discard.as_ref().is_some_and(|discard| {
            sight.sees(discard.owner)
                && discard
                    .remaining
                    .iter()
                    .chain(&discard.ordered)
                    .any(|card| card.code == code)
        })
        || (searcher
            && play
                .searched_player
                .iter()
                .chain(&play.searched_encounter)
                .any(|card| card.code == code))
        || play
            .backpacks
            .iter()
            .filter(|attached| sight.sees(attached.owner))
            .flat_map(|attached| &attached.cards)
            .any(|card| card.code == code)
        || play
            .premonition
            .as_ref()
            .is_some_and(|sealed| sealed.card.code == code)
        || play.players.iter().any(|player| {
            (sight.sees(player.investigator) && player.hand.iter().any(|card| card.code == code))
                || player
                    .discard
                    .iter()
                    .chain(&player.threats)
                    .chain(&player.removed)
                    .any(|card| card.code == code)
                || player.assets.iter().any(|asset| asset.card.code == code)
        })
        || play.test.as_ref().is_some_and(|test| {
            test.source_card
                .as_ref()
                .is_some_and(|card| card.code == code)
                || test.committed.iter().any(|entry| entry.card.code == code)
        });
    (scenario_visible || player_visible)
        .then(|| card(code).or_else(|| scenario_card(code, true)))
        .flatten()
        .map(|mut text| {
            for player in &play.players {
                for asset in &player.assets {
                    let counter = match asset.use_kind() {
                        Some(kind @ (cards::UseKind::Supply | cards::UseKind::Key)) => kind.name(),
                        _ => continue,
                    };
                    if asset.card.code == code {
                        text.push_str(&format!(
                            "\n\n{} — {counter}: {}",
                            player.investigator.name(),
                            asset.uses
                        ));
                    }
                }
            }
            text
        })
}

/// Printed types of the encounter cards selected for the original Gathering.
#[cfg(feature = "tabletop")]
pub(crate) fn encounter_card_type(code: &str) -> Option<&'static str> {
    if super::scenario::enemy_definition(code).is_some() {
        Some("Enemy")
    } else if matches!(
        code,
        "01162" | "01163" | "01164" | "01165" | "01166" | "01167" | "01168"
    ) {
        Some("Treachery")
    } else {
        None
    }
}

/// Inspect a scenario card's currently visible face. The caller supplies the
/// observed location state; this never reveals an act or agenda reverse.
pub fn scenario_card(code: &str, revealed: bool) -> Option<String> {
    let face = if revealed {
        super::scenario::InspectionFace::RevealedLocation
    } else {
        super::scenario::InspectionFace::Front
    };
    super::scenario::inspect(code, face).map(|text| readable(&text))
}

/// Readable reverse text for an Act or Agenda whose reverse is already known.
/// The caller must establish public knowledge before offering this inspection.
pub fn scenario_card_reverse(code: &str) -> Option<String> {
    super::scenario::inspect(code, super::scenario::InspectionFace::StoryReverse)
        .map(|text| readable(&text))
}

/// Full printed fields and effective rules for a selected player card.
/// This describes the card, independently of whether its behavior is implemented.
pub fn card(code: &str) -> Option<String> {
    let card = cards::definition(code)?;
    let mut lines = vec![format!(
        "{}{} · {}",
        if card.unique { "Unique · " } else { "" },
        card.title,
        card.code
    )];
    let factions = card
        .factions
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>()
        .join(" / ");
    lines.push(format!(
        "{factions} · {:?}{}",
        card.kind,
        if card.weakness { " · Weakness" } else { "" }
    ));
    if let Some(cost) = card.cost {
        lines.push(format!("Cost {cost}"));
    }
    if let Some(level) = card.level {
        lines.push(format!("Level {level}"));
    }
    if !card.traits.is_empty() {
        lines.push(format!("Traits: {}.", card.traits.join(". ")));
    }
    if let Some(skills) = card.base_skills {
        lines.push(format!(
            "Willpower {} · Intellect {} · Combat {} · Agility {}",
            skills[0], skills[1], skills[2], skills[3]
        ));
    }
    let icons = ["Willpower", "Intellect", "Combat", "Agility", "Wild"]
        .into_iter()
        .zip(card.icons)
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| format!("{name} {count}"))
        .collect::<Vec<_>>();
    if !icons.is_empty() {
        lines.push(format!("Commitment icons: {}", icons.join(" · ")));
    }
    if let Some(health) = card.health {
        lines.push(format!(
            "Health {health}{}",
            if card
                .enemy
                .is_some_and(|enemy| enemy.health_per_investigator)
            {
                " per investigator"
            } else {
                ""
            }
        ));
    }
    if let Some(sanity) = card.sanity {
        lines.push(format!("Sanity {sanity}"));
    }
    if let Some(enemy) = card.enemy {
        lines.push(format!(
            "Fight {} · Evade {} · Attack: {} damage, {} horror",
            enemy.combat, enemy.evade, enemy.attack_damage, enemy.attack_horror
        ));
    }
    if !card.slots.is_empty() {
        lines.push(format!(
            "Slots: {}",
            card.slots
                .iter()
                .map(|slot| format!("{slot:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    lines.push(String::new());
    lines.push(readable(card.effective_text));
    if matches!(code, "02229" | "04032" | "60416") {
        lines.push("\nEffective text includes taboo list 10.".into());
    }
    if code == "11069" {
        lines.push("\nEffective text includes Legacy FAQ 2.5 errata.".into());
    }
    Some(lines.join("\n"))
}

/// The fixed catalogue uses publisher icon names and trait emphasis. Keep the
/// words while translating only that known presentation notation.
fn readable(value: &str) -> String {
    let mut value = value.replace("[[", "").replace("]]", "");
    for (symbol, name) in [
        ("action", "Action"),
        ("fast", "Free triggered ability"),
        ("reaction", "Reaction"),
        ("willpower", "Willpower"),
        ("intellect", "Intellect"),
        ("combat", "Combat"),
        ("agility", "Agility"),
        ("wild", "Wild"),
        ("skull", "Skull"),
        ("cultist", "Cultist"),
        ("tablet", "Tablet"),
        ("elder_thing", "Elder Thing"),
        ("auto_fail", "Auto-fail"),
        ("elder_sign", "Elder Sign"),
        ("curse", "Curse"),
        ("survivor", "Survivor"),
    ] {
        value = value.replace(&format!("[{symbol}]"), &format!("[{name}]"));
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_reverse_inspection_contains_the_six_selected_advancement_effects() {
        for (code, effect) in [
            ("01105", "each investigator discards 1 card at random"),
            (
                "01106",
                "Shuffle the encounter discard pile into the encounter deck",
            ),
            (
                "01107",
                "Each investigator that has not resigned is defeated",
            ),
            ("01108", "Remove the Study from the game"),
            ("01109", "Spawn the set-aside Ghoul Priest in the Hallway"),
            (
                "01110",
                "This hell-pit is my home! No way are we burning it! (Resolution 2)",
            ),
        ] {
            let text =
                scenario_card_reverse(code).unwrap_or_else(|| panic!("Missing reverse for {code}"));
            assert!(text.contains(effect), "{code}: {text}");
            assert!(text.contains("Reverse:"), "{code}: {text}");
            assert!(!text.contains("Front:"), "{code}: {text}");
        }
    }

    #[test]
    fn reverse_inspection_rejects_locations_reference_cards_and_unknown_codes() {
        for code in ["01104", "01111", "01115", "01116", "06003", "unknown"] {
            assert!(scenario_card_reverse(code).is_none(), "{code}");
        }
    }

    #[test]
    fn location_reveal_flag_cannot_reveal_story_reverses() {
        for code in ["01105", "01106", "01107", "01108", "01109", "01110"] {
            let front = scenario_card(code, false).unwrap();
            assert!(front.contains("Front:"), "{code}: {front}");
            assert!(!front.contains("Reverse:"), "{code}: {front}");
            assert_eq!(scenario_card(code, true).unwrap(), front);
        }
        let hidden = scenario_card("01115", false).unwrap();
        assert!(hidden.contains("You cannot move into the Parlor"));
        assert!(!hidden.contains("Parley"));
        let revealed = scenario_card("01115", true).unwrap();
        assert!(revealed.contains("[Action]: Parley. Test [Intellect] (4)"));
        assert!(!revealed.contains("You cannot move into the Parlor"));
    }
}
