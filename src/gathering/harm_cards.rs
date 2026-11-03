//! Selected cancellation and prevention cards at real revelation/harm windows.

use super::{
    CardView, Investigator, cards, player_cards,
    runtime::{AssetState, Frame, Game, HarmAssignment, HarmOrigin, PendingRevelation},
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarmFixture {
    WardCancellation,
    WardOtherDraw,
    WardWeakness,
    WardEnemy,
    WardVoices,
    DenyAttack,
    DenyChronophobia,
    DenySpellHorror,
    DenyTablet,
    DenyTabletDisplayText,
    DenyPlayerDisplayText,
    DenyCultist,
    DenyAgenda,
    IdolAttack,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct HarmCard {
    pub owner: Investigator,
    pub card: CardView,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct HandDiscard {
    pub remaining: Vec<Investigator>,
    pub current: Option<Investigator>,
    pending: Vec<Investigator>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct IdolDecision {
    owner: Investigator,
    asset: u16,
    selected: Vec<u16>,
    remaining: Option<u8>,
}

impl IdolDecision {
    pub(super) fn owner(&self) -> Investigator {
        self.owner
    }
}

pub(super) fn idol_choices(game: &Game, decision: &IdolDecision) -> Vec<Choice> {
    if decision.remaining.is_some() {
        let harm = idol_harm(game);
        let mut choices = Vec::new();
        for (target, damage, horror) in &harm.allocations {
            for (aspect, amount) in [("damage", damage), ("horror", horror)] {
                if *amount == 0 {
                    continue;
                }
                let (target_id, target_name) = target.map_or_else(
                    || ("investigator".into(), decision.owner.name()),
                    |id| {
                        (
                            id.to_string(),
                            game.players[decision.owner.index()]
                                .assets
                                .iter()
                                .find(|asset| asset.card.id == id)
                                .expect("Assigned asset")
                                .card
                                .title
                                .as_str(),
                        )
                    },
                );
                let id = format!("idol-prevent:{aspect}:{target_id}");
                if !choices.iter().any(|choice: &Choice| choice.id == id) {
                    choices.push(Choice {
                        id,
                        label: format!("Cancel 1 assigned {aspect} on {target_name}"),
                        cancel_faces: vec![],
                    });
                }
            }
        }
        return choices;
    }
    let mut choices = vec![Choice {
        id: "idol-cancel".into(),
        label: "Cancel this Idol activation before paying its costs".into(),
        cancel_faces: vec![],
    }];
    for card in &game.players[decision.owner.index()].hand {
        if card.weakness {
            continue;
        }
        let selected = decision.selected.iter().position(|id| *id == card.id);
        if selected.is_none() && decision.selected.len() == 3 {
            continue;
        }
        choices.push(Choice {
            id: format!("idol-select:{}", card.id),
            label: selected.map_or_else(
                || {
                    format!(
                        "Select {} as discard {}",
                        card.title,
                        decision.selected.len() + 1
                    )
                },
                |index| format!("Unselect {} (discard {})", card.title, index + 1),
            ),
            cancel_faces: vec![],
        });
    }
    if !decision.selected.is_empty() {
        choices.push(Choice {
            id: "idol-pay".into(),
            label: format!(
                "Exhaust Idol and discard {} cards in the selected order",
                decision.selected.len()
            ),
            cancel_faces: vec![],
        });
    }
    choices
}

fn idol_harm(game: &Game) -> &HarmAssignment {
    game.frames
        .iter()
        .rev()
        .find_map(|frame| {
            if let Frame::Harm(harm) = frame {
                Some(harm)
            } else {
                None
            }
        })
        .expect("Idol retains assigned harm")
}

pub(super) fn choose_idol(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Idol(decision)) = game.frames.last() else {
        return Err(RuleError("No Idol activation is pending.".into()));
    };
    if !idol_choices(game, decision)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError("That Idol choice is not legal.".into()));
    }
    let Some(Frame::Idol(mut decision)) = game.frames.pop() else {
        unreachable!()
    };
    if id == "idol-cancel" {
        return Ok(());
    }
    if let Some(card_id) = id.strip_prefix("idol-select:") {
        let card_id = card_id.parse().expect("Advertised hand card");
        if let Some(index) = decision
            .selected
            .iter()
            .position(|selected| *selected == card_id)
        {
            decision.selected.remove(index);
        } else {
            decision.selected.push(card_id);
        }
    } else if id == "idol-pay" {
        let total = idol_harm(game)
            .damage
            .saturating_add(idol_harm(game).horror);
        let player = &mut game.players[decision.owner.index()];
        player
            .assets
            .iter_mut()
            .find(|asset| asset.card.id == decision.asset)
            .expect("Selected ready Idol")
            .ready = false;
        for selected in &decision.selected {
            let index = player
                .hand
                .iter()
                .position(|card| card.id == *selected)
                .expect("Selected discard");
            player.discard.push(player.hand.remove(index));
        }
        decision.remaining = Some(total.min(decision.selected.len() as u8));
        game.history.push(format!("{} exhausts Idol of Xanatos and discards {} chosen cards in the selected order. Cancel that much assigned damage/horror. [09102]", decision.owner.name(), decision.selected.len()));
    } else {
        let parts: Vec<_> = id.split(':').collect();
        let target = if parts[2] == "investigator" {
            None
        } else {
            Some(parts[2].parse().expect("Assigned asset identity"))
        };
        let Some(Frame::Harm(harm)) = game.frames.last_mut() else {
            unreachable!()
        };
        let damage = parts[1] == "damage";
        let allocation = harm
            .allocations
            .iter_mut()
            .find(|(assigned, d, h)| *assigned == target && if damage { *d > 0 } else { *h > 0 })
            .expect("Advertised assigned point");
        if damage {
            allocation.1 -= 1;
            harm.damage -= 1;
        } else {
            allocation.2 -= 1;
            harm.horror -= 1;
        }
        *decision.remaining.as_mut().expect("Paid Idol") -= 1;
    }
    if decision.remaining != Some(0) {
        game.frames.push(Frame::Idol(decision));
    }
    Ok(())
}

pub(super) fn begin_agenda_discard(game: &mut Game) {
    let remaining = super::scenario::player_order(game);
    game.frames.push(Frame::HandDiscard(HandDiscard {
        remaining,
        current: None,
        pending: vec![],
    }));
}

pub(super) fn discard_choices(game: &Game, discard: &HandDiscard) -> Vec<Choice> {
    let Some(owner) = discard.current else {
        return vec![];
    };
    let mut choices: Vec<_> = deny_cards(game, owner)
        .into_iter()
        .map(|card| Choice {
            id: format!("agenda-discard:deny:{}", card.id),
            label: "Play Deny Existence to ignore your Agenda 1 hand discard".into(),
            cancel_faces: vec![],
        })
        .collect();
    choices.push(Choice {
        id: "agenda-discard:resolve".into(),
        label: "Accept your random hand discard after everyone has decided".into(),
        cancel_faces: vec![],
    });
    choices
}

pub(super) fn choose_discard(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::HandDiscard(discard)) = game.frames.last() else {
        return Err(RuleError("No Agenda 1 discard is pending.".into()));
    };
    if !discard_choices(game, discard)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That Agenda 1 discard choice is not legal.".into(),
        ));
    }
    let owner = discard.current.expect("Advertised current owner");
    if let Some(card_id) = id.strip_prefix("agenda-discard:deny:") {
        let card_id: u16 = card_id.parse().expect("Advertised Deny");
        player_cards::pay_spell_cost(game, owner, "05032", None)?;
        let player = &mut game.players[owner.index()];
        let index = player
            .hand
            .iter()
            .position(|card| card.id == card_id)
            .expect("Validated Deny");
        let card = player.hand.remove(index);
        finish_discard_owner(game, owner);
        game.frames.push(Frame::HarmCard(HarmCard { owner, card }));
        game.history.push(format!("{} plays Deny Existence before random selection, ignoring only their Agenda 1 discard. [05032; 01105b]", owner.name()));
    } else {
        let Some(Frame::HandDiscard(discard)) = game.frames.last_mut() else {
            unreachable!()
        };
        discard.pending.push(owner);
        finish_discard_owner(game, owner);
    }
    Ok(())
}

fn finish_discard_owner(game: &mut Game, owner: Investigator) {
    let Some(Frame::HandDiscard(discard)) = game.frames.last_mut() else {
        unreachable!()
    };
    discard.remaining.retain(|remaining| *remaining != owner);
    discard.current = None;
}

pub(super) fn advance_discard(game: &mut Game) -> Result<bool, RuleError> {
    let Some(Frame::HandDiscard(discard)) = game.frames.last() else {
        return Err(RuleError("No Agenda 1 discard is pending.".into()));
    };
    if discard.remaining.is_empty() {
        let Some(Frame::HandDiscard(discard)) = game.frames.pop() else {
            unreachable!()
        };
        for owner in discard.pending {
            super::scenario::agenda_one_discard_player(game, owner);
        }
        game.agenda = 2;
        return Ok(true);
    }
    if discard.current.is_none() {
        let owner = discard.remaining[0];
        let Some(Frame::HandDiscard(discard)) = game.frames.last_mut() else {
            unreachable!()
        };
        discard.current = Some(owner);
        return Ok(true);
    }
    let owner = discard.current.expect("Selected owner");
    let player = &game.players[owner.index()];
    if player.defeated || player.resigned || !player.hand.iter().any(|card| card.code != "06017") {
        finish_discard_owner(game, owner);
        return Ok(true);
    }
    if deny_cards(game, owner).is_empty() {
        let Some(Frame::HandDiscard(discard)) = game.frames.last_mut() else {
            unreachable!()
        };
        discard.pending.push(owner);
        finish_discard_owner(game, owner);
        return Ok(true);
    }
    Ok(false)
}

fn deny_cards(game: &Game, owner: Investigator) -> Vec<&CardView> {
    let player = &game.players[owner.index()];
    if player.defeated || player.resigned || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    player
        .hand
        .iter()
        .filter(|card| card.code == "05032")
        .collect()
}

fn take_player_card(game: &mut Game, owner: Investigator, code: &str) -> CardView {
    let player = &mut game.players[owner.index()];
    if let Some(index) = player.hand.iter().position(|card| card.code == code) {
        player.hand.remove(index)
    } else {
        let index = player
            .deck
            .iter()
            .position(|card| card.code == code)
            .expect("Selected fixture card");
        player.deck.remove(index)
    }
}

pub(super) fn configure_fixture(game: &mut Game, case: HarmFixture) {
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    for player in &mut game.players {
        player.location = "01112".into();
    }
    if case == HarmFixture::IdolAttack {
        configure_idol(game);
        return;
    }
    if matches!(
        case,
        HarmFixture::DenyAttack
            | HarmFixture::DenyChronophobia
            | HarmFixture::DenySpellHorror
            | HarmFixture::DenyTablet
            | HarmFixture::DenyTabletDisplayText
            | HarmFixture::DenyPlayerDisplayText
            | HarmFixture::DenyCultist
            | HarmFixture::DenyAgenda
    ) {
        configure_deny(game, case);
        return;
    }
    let who = Investigator::Jacqueline;
    let ward = take_player_card(game, who, "01065");
    game.players[who.index()].hand.push(ward);
    if case == HarmFixture::WardCancellation {
        let deny = take_player_card(game, who, "05032");
        game.players[who.index()].hand.push(deny);
    }
    game.players[who.index()].resources = 0;
    for code in ["60412", "04197"] {
        let card = take_player_card(game, who, code);
        game.players[who.index()].assets.push(AssetState {
            card,
            ready: true,
            uses: cards::definition(code).unwrap().uses.map_or(0, |(_, n)| n),
            damage: 0,
            horror: 0,
        });
    }
    let card = if case == HarmFixture::WardWeakness {
        take_player_card(game, who, "02039")
    } else {
        let code = if case == HarmFixture::WardEnemy {
            "01159"
        } else {
            "01163"
        };
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == code)
            .expect("Original encounter");
        game.encounter_deck.remove(index)
    };
    game.frames.push(Frame::Revelation(PendingRevelation {
        card,
        drawn_by: if case == HarmFixture::WardOtherDraw {
            Investigator::Patrice
        } else {
            who
        },
        started: false,
        cancelled: false,
    }));
    if case == HarmFixture::WardVoices {
        let index = game
            .encounter_deck
            .iter()
            .position(|card| card.code == "01165")
            .expect("Voices");
        let card = game.encounter_deck.remove(index);
        game.players[who.index()].threats.push(card);
    }
    game.history = vec!["Privileged Ward witness: Jacqueline has her actual Ward, ready Robes, Olive and zero resources. The named pending draw tests original Ward's own non-weakness treachery restriction and its subsequent horror, not ordinary setup. [01065; 60412]".into()];
}

fn configure_idol(game: &mut Game) {
    let owner = Investigator::Patrice;
    let card = take_player_card(game, owner, "09102");
    game.players[owner.index()].assets.push(AssetState {
        card,
        ready: true,
        uses: 0,
        damage: 0,
        horror: 0,
    });
    let watcher = take_player_card(game, owner, "06017");
    game.players[owner.index()].hand.push(watcher);
    let index = game
        .encounter_deck
        .iter()
        .position(|card| card.code == "01160")
        .expect("Minion");
    let card = game.encounter_deck.remove(index);
    let enemy = card.id;
    super::scenario::spawn_encounter_enemy(game, card, owner).expect("Minion spawn");
    game.frames.clear();
    game.queue_harm(
        owner,
        1,
        1,
        false,
        "01160",
        HarmOrigin::EnemyAttack { enemy },
    )
    .expect("Actual Minion attack values");
    game.history = vec!["Privileged Idol witness: Patrice has her actual ready Idol and hidden Watcher while a Ghoul Minion's 1 damage/1 horror await assignment. Hand choice, discard order and canceled aspects remain genuine decisions. [09102; 01160; RR p7,21]".into()];
}

fn configure_deny(game: &mut Game, case: HarmFixture) {
    let who = Investigator::Jacqueline;
    let deny = take_player_card(game, who, "05032");
    game.players[who.index()].hand.push(deny);
    game.players[who.index()].resources = 0;
    let olive = take_player_card(game, who, "04197");
    game.players[who.index()].assets.push(AssetState {
        card: olive,
        ready: true,
        uses: 0,
        damage: 0,
        horror: 0,
    });
    match case {
        HarmFixture::DenyAttack => {
            let index = game
                .encounter_deck
                .iter()
                .position(|card| card.code == "01160")
                .expect("Ghoul Minion");
            let card = game.encounter_deck.remove(index);
            let id = card.id;
            super::scenario::spawn_encounter_enemy(game, card, who).expect("Fixture Minion spawn");
            game.frames.clear();
            game.queue_harm(
                who,
                1,
                1,
                false,
                "01160",
                HarmOrigin::EnemyAttack { enemy: id },
            )
            .expect("Minion attack harm");
        }
        HarmFixture::DenyChronophobia => {
            let card = take_player_card(game, who, "02039");
            game.players[who.index()].threats.push(card);
            game.queue_harm(who, 0, 1, true, "02039", HarmOrigin::EncounterCard)
                .expect("Chronophobia harm");
        }
        HarmFixture::DenySpellHorror | HarmFixture::DenyPlayerDisplayText => game
            .queue_harm(who, 0, 1, false, "01065", HarmOrigin::PlayerCard)
            .expect("Ward horror"),
        HarmFixture::DenyTablet | HarmFixture::DenyTabletDisplayText => game
            .queue_harm(who, 1, 0, false, "01104 Tablet", HarmOrigin::EncounterCard)
            .expect("Tablet damage"),
        HarmFixture::DenyCultist => game
            .queue_harm(who, 0, 1, false, "01104 Cultist", HarmOrigin::EncounterCard)
            .expect("Cultist horror"),
        HarmFixture::DenyAgenda => {
            let other_deny = take_player_card(game, Investigator::Patrice, "05032");
            game.players[Investigator::Patrice.index()]
                .hand
                .push(other_deny);
            game.lead = who;
            game.agenda = 1;
            game.frames.push(Frame::Scenario(
                super::scenario::ScenarioDecision::AgendaOne,
            ));
        }
        _ => unreachable!(),
    }
    if let Some(Frame::Harm(harm)) = game.frames.last_mut() {
        if case == HarmFixture::DenyTabletDisplayText {
            harm.source = "Tablet symbol on The Gathering reference".into();
        }
        if case == HarmFixture::DenyPlayerDisplayText {
            harm.source = "01104 Tablet".into();
        }
    }
    game.history = vec![if case == HarmFixture::DenyAgenda {
        "Privileged Agenda 1 witness: Jacqueline is lead and she and Patrice hold their actual Deny copies. The real reverse-side group-discard choice must offer each prevention before any random selection, then resolve remaining discards together. [05032; 01105b; RR p17]".into()
    } else {
        "Privileged Deny witness: the named source has reached its actual damage/horror assignment with Jacqueline's actual Deny and Olive. This does not assert ordinary setup reached the preceding attack or effect. [05032; RR p7; FAQ1.12]".into()
    }];
}

#[derive(Clone, Copy)]
enum HarmAspect {
    Damage,
    Horror,
}

impl HarmAspect {
    fn name(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::Horror => "horror",
        }
    }
    fn amount(self, harm: &HarmAssignment) -> u8 {
        match self {
            Self::Damage => harm.damage,
            Self::Horror => harm.horror,
        }
    }
    fn ignore(self, harm: &mut HarmAssignment) {
        match self {
            Self::Damage => {
                harm.damage = 0;
                for (_, amount, _) in &mut harm.allocations {
                    *amount = 0;
                }
            }
            Self::Horror => {
                harm.horror = 0;
                for (_, _, amount) in &mut harm.allocations {
                    *amount = 0;
                }
            }
        }
    }
}

/// The producer's rules provenance controls eligibility. Changing a history
/// label cannot turn player-card or game-rule harm into an encounter effect.
fn deny_source(harm: &HarmAssignment) -> bool {
    matches!(
        harm.origin,
        HarmOrigin::EncounterCard | HarmOrigin::EnemyAttack { .. }
    )
}

pub(super) fn prevention_choices(game: &Game, harm: &HarmAssignment) -> Vec<Choice> {
    if harm.prevention_closed
        || harm.allocations.iter().map(|(_, n, _)| *n).sum::<u8>() != harm.damage
        || harm.allocations.iter().map(|(_, _, n)| *n).sum::<u8>() != harm.horror
    {
        return vec![];
    }
    let player = &game.players[harm.investigator.index()];
    if player.defeated || player.resigned {
        return vec![];
    }
    let mut choices: Vec<_> = deny_cards(game, harm.investigator)
        .into_iter()
        .filter(|_| deny_source(harm))
        .flat_map(|card| {
            [HarmAspect::Damage, HarmAspect::Horror]
                .into_iter()
                .filter(|aspect| aspect.amount(harm) > 0)
                .map(move |aspect| Choice {
                    id: format!("deny:{}:{}", card.id, aspect.name()),
                    label: format!(
                        "Play Deny Existence to ignore all {} {} from {}",
                        aspect.amount(harm),
                        aspect.name(),
                        harm.source
                    ),
                    cancel_faces: vec![],
                })
        })
        .collect();
    if harm.damage.saturating_add(harm.horror) > 0 && player.hand.iter().any(|card| !card.weakness)
    {
        choices.extend(player.assets.iter().filter(|asset| asset.card.code == "09102" && asset.ready).map(|asset| Choice {
            id: format!("idol:{}", asset.card.id), label: "Use Idol of Xanatos; choose up to 3 cards to discard and which assigned damage/horror to cancel".into(), cancel_faces: vec![],
        }));
    }
    choices
}

pub(super) fn choose_prevention(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Harm(harm)) = game.frames.last() else {
        return Err(RuleError("No assigned harm awaits prevention.".into()));
    };
    if !prevention_choices(game, harm)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError("That prevention choice is not legal.".into()));
    }
    let owner = harm.investigator;
    if let Some(asset) = id.strip_prefix("idol:") {
        game.frames.push(Frame::Idol(IdolDecision {
            owner,
            asset: asset.parse().expect("Advertised Idol"),
            selected: vec![],
            remaining: None,
        }));
        return Ok(());
    }
    let parts: Vec<_> = id.split(':').collect();
    let card_id: u16 = parts[1].parse().expect("Advertised Deny");
    let aspect = if parts[2] == "damage" {
        HarmAspect::Damage
    } else {
        HarmAspect::Horror
    };
    player_cards::pay_spell_cost(game, owner, "05032", None)?;
    let player = &mut game.players[owner.index()];
    let index = player
        .hand
        .iter()
        .position(|card| card.id == card_id)
        .expect("Validated Deny");
    let card = player.hand.remove(index);
    let Some(Frame::Harm(harm)) = game.frames.last_mut() else {
        unreachable!()
    };
    aspect.ignore(harm);
    game.history.push(format!("{} uses Deny Existence to ignore the {} aspect of {}. Other aspects and assigned targets remain. [05032; RR p7]", owner.name(), aspect.name(), harm.source));
    game.frames.push(Frame::HarmCard(HarmCard { owner, card }));
    Ok(())
}

pub(super) fn revelation_choices(game: &Game, revelation: &PendingRevelation) -> Vec<Choice> {
    if revelation.started
        || revelation.cancelled
        || revelation.card.weakness
        || !matches!(
            revelation.card.code.as_str(),
            "01162" | "01163" | "01164" | "01165" | "01166" | "01167" | "01168"
        )
    {
        return vec![];
    }
    let owner = revelation.drawn_by;
    let player = &game.players[owner.index()];
    if player.defeated || player.resigned || player.threats.iter().any(|card| card.code == "01165")
    {
        return vec![];
    }
    player.hand.iter().filter(|card| card.code == "01065").flat_map(|card| {
        player_cards::spell_cost_choices(game, owner, "01065").into_iter().map(move |payment| {
            let suffix = payment.robes.map_or_else(String::new, |id| format!(":robes:{id}"));
            Choice {
                id: format!("ward:{}{suffix}", card.id),
                label: format!("Play Ward of Protection{} ({} resources); cancel {}'s revelation, then take 1 horror", if payment.robes.is_some() { " exhausting Robes of Endless Night" } else { "" }, payment.cost, revelation.card.title),
                cancel_faces: vec![],
            }
        })
    }).collect()
}

pub(super) fn play_ward(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Revelation(revelation)) = game.frames.last() else {
        return Err(RuleError("No revelation offers Ward of Protection.".into()));
    };
    if !revelation_choices(game, revelation)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That Ward of Protection play is not legal.".into(),
        ));
    }
    let owner = revelation.drawn_by;
    let parts: Vec<_> = id.split(':').collect();
    let card_id: u16 = parts[1].parse().expect("Advertised Ward identity");
    let robes = parts
        .get(3)
        .map(|id| id.parse().expect("Advertised Robes identity"));
    player_cards::pay_spell_cost(game, owner, "01065", robes)?;
    let player = &mut game.players[owner.index()];
    let index = player
        .hand
        .iter()
        .position(|card| card.id == card_id)
        .expect("Validated Ward");
    let card = player.hand.remove(index);
    let Some(Frame::Revelation(revelation)) = game.frames.last_mut() else {
        unreachable!()
    };
    revelation.cancelled = true;
    game.history.push(format!(
        "{} cancels {}'s revelation with Ward of Protection, then takes 1 horror. [01065]",
        owner.name(),
        revelation.card.title
    ));
    game.frames.push(Frame::HarmCard(HarmCard { owner, card }));
    game.queue_harm(owner, 0, 1, false, "01065", HarmOrigin::PlayerCard)
}

pub(super) fn finish(game: &mut Game) -> Result<(), RuleError> {
    let Some(Frame::HarmCard(event)) = game.frames.pop() else {
        return Err(RuleError("No harm card awaits completion.".into()));
    };
    let player = &mut game.players[event.owner.index()];
    if player.defeated || player.resigned {
        player.removed.push(event.card);
    } else {
        player.discard.push(event.card);
    }
    Ok(())
}

pub(super) fn eliminate(game: &mut Game, who: Investigator) {
    for frame in std::mem::take(&mut game.frames) {
        match frame {
            Frame::HarmCard(event) if event.owner == who => {
                game.players[who.index()].removed.push(event.card)
            }
            Frame::HandDiscard(mut discard) => {
                discard.remaining.retain(|owner| *owner != who);
                discard.pending.retain(|owner| *owner != who);
                if discard.current == Some(who) {
                    discard.current = None;
                }
                game.frames.push(Frame::HandDiscard(discard));
            }
            Frame::Idol(decision) if decision.owner == who => {}
            other => game.frames.push(other),
        }
    }
}
