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
    /// Masks have a control limit, independent of equipment slots and readiness.
    pub fn masks_controlled(&self) -> usize {
        self.assets
            .iter()
            .filter(|asset| {
                asset_definition(&asset.card.code)
                    .is_some_and(|definition| definition.has_trait("Mask"))
            })
            .count()
    }

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

#[cfg(test)]
mod tests {
    use crate::gathering::{BoostFixture, HostileFixture, Session};

    #[test]
    fn mask_control_limit_rejects_a_second_without_costs_and_is_personal() {
        let mut session =
            Session::with_hostile_fixture(HostileFixture::Boosts(BoostFixture::MaskRetry));
        // Arrange real selected card copies, then exercise only public choices.
        let game = session.runtime.as_mut().unwrap();
        for enemy in &mut game.scenario.enemies {
            enemy.exhausted = true;
        }
        let patrice = &mut game.players[1];
        patrice.resources = 5;
        let second = patrice
            .deck
            .iter()
            .position(|card| card.code == "10111")
            .unwrap();
        let card = patrice.deck.remove(second);
        let id = card.id;
        patrice.hand.push(card);
        let first = patrice
            .assets
            .iter_mut()
            .find(|asset| asset.card.code == "10111")
            .unwrap();
        first.ready = false;
        first.uses = 0;
        let before = session.observe();
        let action = format!("action:ordinary:play:{id}");
        assert!(!before.choices.iter().any(|choice| choice.id == action));
        assert!(session.choose(before.revision, &action).is_err());
        assert_eq!(
            session.observe(),
            before,
            "An illegal Mask does not pay costs"
        );

        // A Mask controlled by another investigator does not occupy Patrice's limit.
        let game = session.runtime.as_mut().unwrap();
        let index = game.players[1]
            .assets
            .iter()
            .position(|asset| asset.card.code == "10111")
            .unwrap();
        let mask = game.players[1].assets.remove(index);
        game.players[0].assets.push(mask);
        let free = session.observe();
        assert!(
            free.choices.iter().any(|choice| choice.id == action),
            "{:?}",
            free.choices
        );
        session.choose(free.revision, &action).unwrap();
        assert!(
            session.observe().play.unwrap().players[1]
                .assets
                .iter()
                .any(|asset| asset.card.id == id)
        );
    }
}
