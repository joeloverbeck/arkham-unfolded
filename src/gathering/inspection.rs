//! Readable inspection of the fixed selected card definitions.

use super::cards;

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
