//! Readable inspection of the fixed selected card definitions.

use super::cards;

/// Inspect a scenario card's currently visible face. The caller supplies the
/// observed location state; this never reveals an act or agenda reverse.
pub fn scenario_card(code: &str, revealed: bool) -> Option<String> {
    super::scenario::inspect(code, revealed).map(|text| readable(&text))
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
