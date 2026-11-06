//! One capacity calculation for placement legality and the public asset display.
use super::{
    cards::{CardDefinition, Slot},
    player_cards::asset_definition,
    runtime::PlayerView,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotUsage {
    pub name: &'static str,
    pub used: usize,
    pub capacity: usize,
    /// Additional capacity reserved for Spell or Ritual assets.
    pub spell_ritual: usize,
}

impl SlotUsage {
    pub fn details(&self) -> String {
        if self.spell_ritual == 0 {
            self.label()
        } else {
            format!(
                "{} · extra capacity from Sign Magick ({} in play)",
                self.label(),
                self.spell_ritual
            )
        }
    }

    pub fn label(&self) -> String {
        let restriction = if self.spell_ritual == 0 {
            String::new()
        } else {
            format!(" (+{} Spell/Ritual)", self.spell_ritual)
        };
        format!(
            "{}: {}/{}{restriction}",
            self.name, self.used, self.capacity
        )
    }
}

impl PlayerView {
    pub fn slot_usage(&self) -> Vec<SlotUsage> {
        usage(
            &self
                .assets
                .iter()
                .filter_map(|asset| asset_definition(&asset.card.code))
                .collect::<Vec<_>>(),
        )
    }
}

pub(super) fn usage(definitions: &[&CardDefinition]) -> Vec<SlotUsage> {
    let extra_arcane = definitions
        .iter()
        .filter(|definition| definition.code == "05112")
        .count();
    [
        (Slot::Ally, "Ally", 1),
        (Slot::Arcane, "Arcane", 2),
        (Slot::Hand, "Hand", 2),
        (Slot::Body, "Body", 1),
        (Slot::Accessory, "Accessory", 1),
    ]
    .into_iter()
    .map(|(slot, name, base)| {
        let spell_ritual = if slot == Slot::Arcane {
            extra_arcane
        } else {
            0
        };
        SlotUsage {
            name,
            used: definitions
                .iter()
                .flat_map(|definition| definition.slots)
                .filter(|occupied| **occupied == slot)
                .count(),
            capacity: base + spell_ritual,
            spell_ritual,
        }
    })
    .collect()
}

pub(super) fn fit(definitions: &[&CardDefinition]) -> bool {
    usage(definitions)
        .iter()
        .all(|slot| slot.used <= slot.capacity)
        && definitions
            .iter()
            .filter(|definition| !definition.has_trait("Spell") && !definition.has_trait("Ritual"))
            .flat_map(|definition| definition.slots)
            .filter(|slot| **slot == Slot::Arcane)
            .count()
            <= 2
}
