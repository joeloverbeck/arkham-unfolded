//! Asset play through the shared Investigation action transaction.

pub(super) use super::asset_entry::{
    AssetEntry, activate_asset_action, asset_action_choices, choose_entry, entry_choices,
};
use super::{
    CardView, Investigator,
    cards::{self, CardDefinition, CardKind, Slot},
    runtime::{AssetState, Frame, Game, Phase, PlayerState},
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

/// Named source witnesses, unavailable through ordinary player entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetFixture {
    RobesOpportunity,
    SignMagickSlots,
    SwordCaneOpportunity,
    SwordCaneFrozen,
    PairedLongColts,
    UncageRobes,
}

fn take_fixture_card(player: &mut PlayerState, code: &str) -> CardView {
    if let Some(index) = player.hand.iter().position(|card| card.code == code) {
        player.hand.remove(index)
    } else {
        let index = player
            .deck
            .iter()
            .position(|card| card.code == code)
            .expect("Selected card");
        player.deck.remove(index)
    }
}

pub(super) fn configure_fixture(game: &mut Game, case: AssetFixture) {
    let actor = if case == AssetFixture::PairedLongColts {
        Investigator::Tony
    } else {
        Investigator::Jacqueline
    };
    for frame in game.frames.drain(..) {
        if let super::runtime::Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    for player in &mut game.players {
        player.location = "01112".into();
    }
    let priest = super::scenario::advance_act_two(game).expect("Original Priest");
    game.scenario
        .enemies
        .iter_mut()
        .find(|enemy| enemy.card.id == priest)
        .expect("Priest")
        .engaged = Some(actor);
    let player = &mut game.players[actor.index()];
    let (assets, hand): (&[&str], &[&str]) = match case {
        AssetFixture::RobesOpportunity => (&["60412"], &["60408", "11067"]),
        AssetFixture::SignMagickSlots => (&["60408", "11067"], &["05112", "11067"]),
        AssetFixture::SwordCaneOpportunity | AssetFixture::SwordCaneFrozen => (&[], &["07029"]),
        AssetFixture::PairedLongColts => (&[], &["06011", "06011"]),
        AssetFixture::UncageRobes => (&["60412"], &["03033", "60408", "05112", "07029"]),
    };
    for code in assets {
        let card = take_fixture_card(player, code);
        player.assets.push(AssetState {
            card,
            ready: true,
            uses: cards::definition(code)
                .unwrap()
                .uses
                .map_or(0, |(_, uses)| uses),
            damage: 0,
            horror: 0,
        });
    }
    let hand: Vec<_> = hand
        .iter()
        .map(|code| take_fixture_card(player, code))
        .collect();
    player.hand.extend(hand);
    if case == AssetFixture::SignMagickSlots {
        player.resources = 6;
    }
    if case == AssetFixture::UncageRobes {
        player.resources = 0;
    }
    if case == AssetFixture::SwordCaneFrozen {
        for _ in 0..2 {
            let index = game
                .encounter_deck
                .iter()
                .position(|card| card.code == "01164")
                .expect("Two original Frozen copies");
            player.threats.push(game.encounter_deck.remove(index));
        }
    }
    game.enable_fixture_turn(actor);
    if case == AssetFixture::UncageRobes {
        game.set_fixture_tokens(vec![
            TokenFace::Skull,
            TokenFace::PlusOne,
            TokenFace::ElderSign,
        ]);
    }
    game.history = vec![match case {
        AssetFixture::RobesOpportunity => "Privileged Robes witness: Jacqueline controls ready original Robes, holds Clairvoyance and is engaged with the Priest. This tests the optional Spell discount before opportunity damage, not ordinary preparation. [60412; 60408; RR Appendix I]",
        AssetFixture::SignMagickSlots => "Privileged Sign Magick witness: Jacqueline has two arcane Spells, Sign Magick and Eyes in hand, six resources and the engaged Priest. This tests Fast play and the restricted third slot, not ordinary preparation. [05112; RR Fast]",
        AssetFixture::SwordCaneOpportunity => "Privileged Sword Cane witness: Jacqueline holds her real Sword Cane while engaged with the Priest. This verifies opportunity-free play and the optional free entry ability, not ordinary preparation. [07029]",
        AssetFixture::SwordCaneFrozen => "Privileged Sword Cane/Frozen witness: Jacqueline holds her real Sword Cane, is engaged with the Priest and has both original Frozen in Fear copies. This tests external additional costs despite waived activation costs. [07029; 01164 official-response FAQ]",
        AssetFixture::PairedLongColts => "Privileged paired Long Colt witness: Tony holds both actual signature weapons, with five resources and the engaged Priest. This verifies ordinary first play followed by an optional second play at no resource cost. [06011]",
        AssetFixture::UncageRobes => "Privileged Uncage/Robes witness: Jacqueline has zero resources, ready Robes, Clairvoyance and Uncage in hand, and the engaged Priest. This verifies nested play costs after the outer event's opportunity attack. [03033; 60412; 60408]",
    }.into()];
}

pub(super) fn fast_choices(game: &Game, actor: Investigator) -> Vec<Choice> {
    if game.phase != Phase::Investigation || game.actor != Some(actor) {
        return vec![];
    }
    asset_choices(game, actor, PlayMode::Fast)
}

pub(super) fn uncage_targets(game: &Game, actor: Investigator) -> Vec<Choice> {
    uncage_targets_inner(game, actor, false)
}

pub(super) fn can_play_uncage(game: &Game, actor: Investigator) -> bool {
    !uncage_targets_inner(game, actor, true).is_empty()
}

fn uncage_targets_inner(game: &Game, actor: Investigator, before_outer_cost: bool) -> Vec<Choice> {
    let mut choices = asset_choices(game, actor, PlayMode::Uncage);
    choices.extend(if before_outer_cost {
        super::spell_events::uncage_choices_after_play_cost(game, actor)
    } else {
        super::spell_events::uncage_choices(game, actor)
    });
    // Voice of Ra is the currently consumed Spell event. Other selected Spell
    // events need their own effects before this verification runtime offers them.
    let player = &game.players[actor.index()];
    for action in super::spells::action_choices(game, actor) {
        let Some(id) = action
            .id
            .strip_prefix("play:spell:")
            .and_then(|id| id.parse::<u16>().ok())
        else {
            continue;
        };
        if player
            .hand
            .iter()
            .any(|card| card.id == id && card.code == "60416")
        {
            choices.push(Choice {
                id: format!("uncage:spell:{id}"),
                label: "Play Voice of Ra through Uncage the Soul (0 resources)".into(),
                cancel_faces: vec![],
            });
        }
    }
    choices
}

pub(super) fn begin_uncage(game: &mut Game, actor: Investigator) -> Result<(), RuleError> {
    if !uncage_targets(game, actor).is_empty() {
        game.frames
            .push(Frame::AssetEntry(AssetEntry::Uncage { actor }));
    } else {
        game.history.push("Uncage the Soul has no playable supported Spell or Ritual after its opportunity attack. [03033]".into());
    }
    Ok(())
}

pub(super) fn play_uncaged(
    game: &mut Game,
    actor: Investigator,
    id: &str,
) -> Result<(), RuleError> {
    if id.starts_with("uncage:event:") {
        let paid = super::spell_events::pay_action(game, actor, id, true)?;
        return super::encounter_search::resolve_action(game, paid);
    }
    if let Some(card) = id.strip_prefix("uncage:spell:") {
        if !uncage_targets(game, actor)
            .iter()
            .any(|choice| choice.id == id)
        {
            return Err(RuleError(
                "That Uncage event target is not currently legal.".into(),
            ));
        }
        let paid = super::spells::pay_action(game, actor, &format!("play:spell:{card}"))?;
        return super::spells::resolve_action(game, paid);
    }
    let prepared = pay_asset_inner(game, actor, id, PlayMode::Uncage)?;
    if let Some(prepared) = resolve_asset(game, prepared)? {
        game.frames.push(Frame::AssetPlacement(prepared));
    }
    Ok(())
}

pub(super) fn play_fast(game: &mut Game, actor: Investigator, id: &str) -> Result<(), RuleError> {
    let prepared = pay_asset_inner(game, actor, id, PlayMode::Fast)?;
    if let Some(prepared) = resolve_asset(game, prepared)? {
        game.frames.push(Frame::AssetPlacement(prepared));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayMode {
    Ordinary,
    Fast,
    LongColtFree,
    Uncage,
}

impl PlayMode {
    fn prefix(self) -> &'static str {
        match self {
            Self::Ordinary => "play",
            Self::Fast => "fast-asset",
            Self::LongColtFree => "play-colt",
            Self::Uncage => "uncage",
        }
    }

    fn permits(self, definition: &CardDefinition) -> bool {
        match self {
            Self::Ordinary => definition.code != "05112",
            Self::Fast => definition.code == "05112",
            Self::LongColtFree => definition.code == "06011",
            Self::Uncage => definition.has_trait("Spell") || definition.has_trait("Ritual"),
        }
    }

    fn reduction(self) -> u8 {
        if self == Self::Uncage { 3 } else { 0 }
    }
}

pub(super) fn free_colt_choices(game: &Game, actor: Investigator) -> Vec<Choice> {
    asset_choices(game, actor, PlayMode::LongColtFree)
}

pub(super) fn play_free_colt(
    game: &mut Game,
    actor: Investigator,
    id: &str,
) -> Result<(), RuleError> {
    let prepared = pay_asset_inner(game, actor, id, PlayMode::LongColtFree)?;
    if let Some(prepared) = resolve_asset(game, prepared)? {
        game.frames.push(Frame::AssetPlacement(prepared));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PreparedAsset {
    pub actor: Investigator,
    pub card: CardView,
    pub resource_cost: u8,
    replacements: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SpellPayment {
    pub robes: Option<u16>,
    pub cost: u8,
}

/// The original Robes' reaction is optional and only reduces Spell costs.
/// Callers retain responsibility for the card's initiation window and target.
pub(super) fn spell_cost_choices(
    game: &Game,
    actor: Investigator,
    code: &str,
) -> Vec<SpellPayment> {
    reduced_cost_choices(game, actor, code, 0)
}

fn reduced_cost_choices(
    game: &Game,
    actor: Investigator,
    code: &str,
    reduction: u8,
) -> Vec<SpellPayment> {
    let player = &game.players[actor.index()];
    let Some(definition) = cards::definition(code) else {
        return vec![];
    };
    let Some(cost) = definition.cost else {
        return vec![];
    };
    let cost = cost.saturating_sub(reduction);
    let mut payments = vec![];
    if cost <= player.resources {
        payments.push(SpellPayment { robes: None, cost });
    }
    if cost > 0 && definition.has_trait("Spell") && cost - 1 <= player.resources {
        payments.extend(
            player
                .assets
                .iter()
                .filter(|asset| asset.card.code == "60412" && asset.ready)
                .map(|asset| SpellPayment {
                    robes: Some(asset.card.id),
                    cost: cost - 1,
                }),
        );
    }
    payments
}

pub(super) fn pay_spell_cost(
    game: &mut Game,
    actor: Investigator,
    code: &str,
    robes: Option<u16>,
) -> Result<u8, RuleError> {
    pay_reduced_cost(game, actor, code, robes, 0)
}

fn pay_reduced_cost(
    game: &mut Game,
    actor: Investigator,
    code: &str,
    robes: Option<u16>,
    reduction: u8,
) -> Result<u8, RuleError> {
    let payment = reduced_cost_choices(game, actor, code, reduction)
        .into_iter()
        .find(|payment| payment.robes == robes)
        .ok_or_else(|| RuleError("That Spell payment is not currently legal.".into()))?;
    let player = &mut game.players[actor.index()];
    if let Some(id) = robes {
        player
            .assets
            .iter_mut()
            .find(|asset| asset.card.id == id)
            .expect("Validated Robes")
            .ready = false;
        game.history.push(format!(
            "{} exhausts Robes of Endless Night to reduce this Spell's resource cost by 1. [60412]",
            actor.name()
        ));
    }
    player.resources -= payment.cost;
    Ok(payment.cost)
}

pub(super) fn pay_asset(
    game: &mut Game,
    actor: Investigator,
    id: &str,
) -> Result<PreparedAsset, RuleError> {
    pay_asset_inner(game, actor, id, PlayMode::Ordinary)
}

fn pay_asset_inner(
    game: &mut Game,
    actor: Investigator,
    id: &str,
    mode: PlayMode,
) -> Result<PreparedAsset, RuleError> {
    let legal = if mode == PlayMode::Fast {
        fast_choices(game, actor)
    } else {
        asset_choices(game, actor, mode)
    };
    if !legal.iter().any(|choice| choice.id == id) {
        return Err(RuleError("That asset play is not currently legal.".into()));
    }
    let (play, robes) = id
        .rsplit_once(":robes:")
        .map_or((id, None), |(play, robes)| {
            (play, Some(robes.parse().expect("Advertised Robes")))
        });
    let (play, replacements) = play.split_once(":replace:").unwrap_or((play, ""));
    let card_id: u16 = play
        .strip_prefix(&format!("{}:", mode.prefix()))
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| RuleError("Invalid asset identity.".into()))?;
    let replacements = if replacements.is_empty() {
        vec![]
    } else {
        replacements
            .split(',')
            .map(|id| id.parse().expect("Advertised replacement"))
            .collect()
    };
    let code = super::backpack::playable_cards(game, actor)
        .into_iter()
        .find(|card| card.id == card_id)
        .expect("Advertised playable card")
        .code
        .clone();
    let resource_cost = if mode == PlayMode::LongColtFree {
        0
    } else if mode == PlayMode::Uncage {
        game.history.push(
            "Uncage the Soul reduces the nested Spell or Ritual's resource cost by 3. [03033]"
                .into(),
        );
        pay_reduced_cost(game, actor, &code, robes, 3)?
    } else {
        pay_spell_cost(game, actor, &code, robes)?
    };
    let card = super::backpack::take_for_play(game, actor, card_id);
    game.history.push(format!("{} pays {resource_cost} resources for {}; the card waits in limbo until its play effect resolves. [RR Appendix I]", actor.name(), card.title));
    Ok(PreparedAsset {
        actor,
        card,
        resource_cost,
        replacements,
    })
}

pub(super) fn resolve_asset(
    game: &mut Game,
    mut prepared: PreparedAsset,
) -> Result<Option<PreparedAsset>, RuleError> {
    let definition = cards::definition(&prepared.card.code)
        .ok_or_else(|| RuleError("The paid asset has no selected definition.".into()))?;
    let player = &game.players[prepared.actor.index()];
    prepared
        .replacements
        .retain(|id| player.assets.iter().any(|asset| asset.card.id == *id));
    let legal = replacement_choices(player, definition);
    if legal == [Vec::<u16>::new()] {
        // Losing an asset to the intervening attack can make all replacements
        // unnecessary. Unneeded assets cannot be discarded just to play a card.
        prepared.replacements.clear();
    } else if !legal.contains(&prepared.replacements) {
        return Ok(Some(prepared));
    }
    let player = &mut game.players[prepared.actor.index()];
    let mut removed_lita = None;
    let mut discarded_assets = vec![];
    let departed_assets = prepared.replacements.clone();
    for id in prepared.replacements {
        let index = player
            .assets
            .iter()
            .position(|asset| asset.card.id == id)
            .expect("Revalidated replacement");
        let discarded = player.assets.remove(index).card;
        if discarded.code == "01117" {
            removed_lita = Some(discarded);
        } else {
            discarded_assets.push(discarded);
        }
    }
    game.history.push(format!("{} puts {} into play after its costs and any attacks of opportunity. [RR Play Action; Slots]", prepared.actor.name(), prepared.card.title));
    let asset_id = prepared.card.id;
    player.assets.push(AssetState {
        card: prepared.card,
        ready: true,
        uses: definition.uses.map_or(0, |(_, amount)| amount),
        damage: 0,
        horror: 0,
    });
    if let Some(card) = removed_lita {
        super::scenario::remove_lita(game, card);
    }
    for asset in departed_assets {
        super::tony_effects::asset_left_play(game, asset);
    }
    super::asset_entry::after_play(game, prepared.actor, asset_id);
    super::backpack::discard_replaced(game, prepared.actor, discarded_assets);
    game.capture_presentation(format!(
        "{} puts {} into play.",
        prepared.actor.name(),
        definition.title
    ));
    Ok(None)
}

pub(super) fn placement_choices(game: &Game, prepared: &PreparedAsset) -> Vec<Choice> {
    let Some(definition) = cards::definition(&prepared.card.code) else {
        return vec![];
    };
    let player = &game.players[prepared.actor.index()];
    replacement_choices(player, definition)
        .into_iter()
        .map(|ids| {
            let identities = if ids.is_empty() {
                "none".into()
            } else {
                ids.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
            };
            let names = ids
                .iter()
                .map(|id| {
                    player
                        .assets
                        .iter()
                        .find(|asset| asset.card.id == *id)
                        .expect("Legal replacement")
                        .card
                        .title
                        .as_str()
                })
                .collect::<Vec<_>>()
                .join(", ");
            Choice {
                id: format!("asset:{}:replace:{identities}", prepared.card.id),
                label: if ids.is_empty() {
                    format!(
                        "Put {} into play without discarding an asset",
                        prepared.card.title
                    )
                } else {
                    format!(
                        "Discard {names} in that order to make room for {}",
                        prepared.card.title
                    )
                },
                cancel_faces: vec![],
            }
        })
        .collect()
}

pub(super) fn choose_placement(
    game: &mut Game,
    mut prepared: PreparedAsset,
    id: &str,
) -> Result<Option<PreparedAsset>, RuleError> {
    if !placement_choices(game, &prepared)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That asset replacement is not currently legal.".into(),
        ));
    }
    let ids = id.rsplit(':').next().expect("Validated placement");
    prepared.replacements = if ids == "none" {
        vec![]
    } else {
        ids.split(',')
            .map(|id| id.parse().expect("Validated replacement identity"))
            .collect()
    };
    resolve_asset(game, prepared)
}

/// Player assets and the original Gathering story ally share slot/soak rules.
/// Lita remains scenario content and is never inserted into a selected deck.
pub(super) fn asset_definition(code: &str) -> Option<&'static CardDefinition> {
    if code == "01117" {
        Some(super::scenario::lita_definition())
    } else {
        cards::definition(code)
    }
}

pub(super) fn play_choices(game: &Game, actor: Investigator) -> Vec<Choice> {
    asset_choices(game, actor, PlayMode::Ordinary)
}

fn asset_choices(game: &Game, actor: Investigator, mode: PlayMode) -> Vec<Choice> {
    let player = &game.players[actor.index()];
    if player.defeated || player.resigned || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    super::backpack::playable_cards(game, actor)
        .into_iter()
        .flat_map(|card| {
            let Some(definition) = cards::definition(&card.code) else {
                return vec![];
            };
            if definition.cost.is_none() {
                return vec![];
            }
            if definition.kind != CardKind::Asset
                || definition.permanent
                || definition.weakness
                // These play/entry operations have distinct unfinished timing
                // consumers. Never silently spend an action on a Fast asset or
                // skip a player's entry reaction.
                || !mode.permits(definition)
                || definition.unique
                    && game
                        .players
                        .iter()
                        .flat_map(|p| &p.assets)
                        .any(|asset| asset.card.title == card.title)
            {
                return vec![];
            }
            if definition.has_trait("Mask")
                && player.assets.iter().any(|asset| {
                    cards::definition(&asset.card.code)
                        .is_some_and(|existing| existing.has_trait("Mask"))
                })
            {
                return vec![];
            }
            let mut choices = vec![];
            let payments = if mode == PlayMode::LongColtFree {
                vec![SpellPayment {
                    robes: None,
                    cost: 0,
                }]
            } else if mode.reduction() == 0 {
                spell_cost_choices(game, actor, &card.code)
            } else {
                reduced_cost_choices(game, actor, &card.code, mode.reduction())
            };
            for payment in payments {
                for discarded in replacement_choices(player, definition) {
                    let mut id = format!("{}:{}", mode.prefix(), card.id);
                    let mut label = format!("Play {} for {} resources", card.title, payment.cost);
                    if !discarded.is_empty() {
                        id.push_str(":replace:");
                        id.push_str(
                            &discarded
                                .iter()
                                .map(u16::to_string)
                                .collect::<Vec<_>>()
                                .join(","),
                        );
                        let names: Vec<_> = discarded
                            .iter()
                            .map(|id| {
                                player
                                    .assets
                                    .iter()
                                    .find(|asset| asset.card.id == *id)
                                    .expect("Owned replacement")
                                    .card
                                    .title
                                    .as_str()
                            })
                            .collect();
                        label.push_str(&format!(
                            "; discard {} in that order to make room",
                            names.join(", ")
                        ));
                    }
                    if let Some(robes) = payment.robes {
                        id.push_str(&format!(":robes:{robes}"));
                        label.push_str("; exhaust Robes of Endless Night");
                    }
                    choices.push(Choice {
                        id,
                        label,
                        cancel_faces: vec![],
                    });
                }
            }
            choices
        })
        .collect()
}

fn replacement_choices(player: &PlayerState, incoming: &CardDefinition) -> Vec<Vec<u16>> {
    if slots_fit(player, incoming, &[]) {
        return vec![vec![]];
    }
    // The selected inventory has a fixed, small number of occupied slots. All
    // legal minimal replacement sets are player choices; none is auto-selected.
    let mut subsets = vec![vec![]];
    for asset in &player.assets {
        let Some(definition) = asset_definition(&asset.card.code) else {
            continue;
        };
        if definition.permanent || definition.slots.is_empty() {
            continue;
        }
        let added: Vec<_> = subsets
            .iter()
            .map(|subset| {
                let mut next = subset.clone();
                next.push(asset.card.id);
                next
            })
            .collect();
        subsets.extend(added);
    }
    let mut choices = vec![];
    for subset in subsets {
        if !slots_fit(player, incoming, &subset) {
            continue;
        }
        if subset.iter().any(|id| {
            slots_fit(
                player,
                incoming,
                &subset
                    .iter()
                    .copied()
                    .filter(|other| other != id)
                    .collect::<Vec<_>>(),
            )
        }) {
            continue;
        }
        discard_orders(&subset, &mut vec![], &mut choices);
    }
    choices
}

fn discard_orders(remaining: &[u16], prefix: &mut Vec<u16>, orders: &mut Vec<Vec<u16>>) {
    if remaining.is_empty() {
        orders.push(prefix.clone());
        return;
    }
    for (index, id) in remaining.iter().enumerate() {
        prefix.push(*id);
        let rest: Vec<_> = remaining
            .iter()
            .enumerate()
            .filter_map(|(other, id)| (other != index).then_some(*id))
            .collect();
        discard_orders(&rest, prefix, orders);
        prefix.pop();
    }
}

fn slots_fit(player: &PlayerState, incoming: &CardDefinition, discarded: &[u16]) -> bool {
    let mut definitions: Vec<_> = player
        .assets
        .iter()
        .filter(|asset| !discarded.contains(&asset.card.id))
        .filter_map(|asset| asset_definition(&asset.card.code))
        .collect();
    definitions.push(incoming);
    let extra_arcane = definitions
        .iter()
        .filter(|definition| definition.code == "05112")
        .count();
    for slot in [
        Slot::Hand,
        Slot::Arcane,
        Slot::Ally,
        Slot::Accessory,
        Slot::Body,
    ] {
        let used = definitions
            .iter()
            .flat_map(|definition| definition.slots)
            .filter(|occupied| **occupied == slot)
            .count();
        let limit = match slot {
            Slot::Hand => 2,
            Slot::Arcane => 2 + extra_arcane,
            _ => 1,
        };
        if used > limit {
            return false;
        }
    }
    let unrestricted_arcane = definitions
        .iter()
        .filter(|definition| !definition.has_trait("Spell") && !definition.has_trait("Ritual"))
        .flat_map(|definition| definition.slots)
        .filter(|slot| **slot == Slot::Arcane)
        .count();
    unrestricted_arcane <= 2
}
