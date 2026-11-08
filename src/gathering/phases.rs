//! Concrete original Gathering framework sequence, consumed by Game frames.
//! Phase continuations remain below nested harm, revelation and player choices.

use super::{
    Investigator,
    runtime::{Frame, Game, HarmOrigin, Phase},
    scenario::{self, EnemyAttack, ScenarioDecision, ScenarioStep},
};
use crate::{Choice, RuleError};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarry_placement_waits_for_mythos_and_agenda_one_clears_every_bearer_before_encounters() {
        use crate::gathering::{HostileFixture, Session};
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringDoomThreshold);
        let before = session.observe().play.unwrap();
        session
            .choose(session.observe().revision, "mythos:begin")
            .unwrap();
        let after = session.observe().play.unwrap();
        assert_eq!(
            after.agenda, 1,
            "Resolve the reverse before installing Agenda 2"
        );
        assert_eq!(after.agenda_doom, 0);
        assert!(after.scenario.enemies.iter().all(|enemy| enemy.doom == 0));
        assert_eq!(after.encounter_deck_count, before.encounter_deck_count);
        assert!(
            session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id == "agenda:lead-horror")
        );
        session.validate().unwrap();
        // Quarry's own revelation places doom but does not authorize an agenda
        // check (06012; RR Doom). Ancient Evils explicitly does.
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringQuarryPlacement);
        for id in ["revelation:resolve", "quarry:01113"] {
            session.choose(session.observe().revision, id).unwrap();
        }
        let play = session.observe().play.unwrap();
        assert_eq!(play.agenda, 1);
        assert_eq!(play.agenda_doom, 2);
        assert_eq!(
            play.scenario
                .enemies
                .iter()
                .map(|enemy| u16::from(enemy.doom))
                .sum::<u16>(),
            1
        );
        assert!(
            !session
                .observe()
                .choices
                .iter()
                .any(|choice| choice.id.starts_with("agenda:"))
        );
        session.validate().unwrap();
    }

    #[test]
    fn inactive_enemies_skip_but_exhausted_agenda_movement_does_not() {
        let session = crate::gathering::Session::with_hostile_fixture(
            crate::gathering::HostileFixture::GatheringGhoulMovement,
        );
        let mut game = session.runtime.unwrap();
        for player in &mut game.players {
            player.hand.clear();
            player.assets.clear();
        }
        assert!(!game.scenario.enemies.is_empty());
        for enemy in &mut game.scenario.enemies {
            enemy.exhausted = true;
            enemy.engaged = None;
            enemy.location = scenario::HALLWAY.into();
        }
        let decision = PhaseDecision::Hunters { remaining: vec![] };
        game.frames = vec![Frame::Phase(decision.clone())];
        game.agenda = 3;
        assert!(
            !choices(&game, &decision)
                .iter()
                .any(|choice| choice.id == "enemy:skip-empty")
        );
        game.agenda = 1;
        assert!(
            choices(&game, &decision)
                .iter()
                .any(|choice| choice.id == "enemy:skip-empty")
        );
        choose(&mut game, "enemy:skip-empty").unwrap();
        assert_eq!(game.phase, Phase::Upkeep);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum PhaseDecision {
    PlayerWindow {
        next: Option<Box<PhaseDecision>>,
    },
    Hunters {
        remaining: Vec<u16>,
    },
    EnemyAttacks {
        order: Vec<Investigator>,
        index: usize,
    },
    AfterAttack(EnemyAttack),
    EnemyEnd,
    UpkeepReady,
    UpkeepDraw {
        order: Vec<Investigator>,
        index: usize,
    },
    PatriceDiscard {
        order: Vec<Investigator>,
        index: usize,
        selected: Vec<u16>,
    },
    UpkeepResources,
    HandLimit {
        order: Vec<Investigator>,
        index: usize,
    },
    RoundEnd,
    MythosBegin,
    MythosAfterAgenda,
    MythosDraw {
        order: Vec<Investigator>,
        index: usize,
    },
    InvestigationBegin,
}

/// Captured player order survives nested effects, but eliminated players never
/// receive another attack/draw/hand-limit decision. Preserve the cursor's next
/// surviving player rather than restarting a phase after a lead change.
pub(super) fn eliminate(game: &mut Game, who: Investigator) {
    fn prune(decision: &mut PhaseDecision, who: Investigator) {
        if let PhaseDecision::PlayerWindow { next: Some(next) } = decision {
            prune(next, who);
            return;
        }
        let was_patrice = matches!(decision, PhaseDecision::PatriceDiscard { .. })
            && who == Investigator::Patrice;
        match decision {
            PhaseDecision::EnemyAttacks { order, index }
            | PhaseDecision::UpkeepDraw { order, index }
            | PhaseDecision::PatriceDiscard { order, index, .. }
            | PhaseDecision::HandLimit { order, index }
            | PhaseDecision::MythosDraw { order, index } => {
                if let Some(removed) = order.iter().position(|candidate| *candidate == who) {
                    order.remove(removed);
                    if removed < *index {
                        *index -= 1;
                    }
                }
                if was_patrice {
                    *decision = PhaseDecision::UpkeepDraw {
                        order: order.clone(),
                        index: *index,
                    };
                }
            }
            _ => {}
        }
    }
    for frame in &mut game.frames {
        if let Frame::Phase(decision) = frame {
            prune(decision, who);
        }
    }
}

fn choice(id: impl Into<String>, label: impl Into<String>) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        cancel_faces: vec![],
    }
}

fn enemy_label(game: &Game, id: u16) -> String {
    game.scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == id)
        .map(|enemy| {
            format!(
                "{} in {}",
                enemy.card.title,
                super::content::title(&enemy.location).unwrap_or("this location")
            )
        })
        .unwrap_or_else(|| "the enemy".into())
}

pub(super) fn decision_owner(game: &Game, decision: &PhaseDecision) -> Option<Investigator> {
    match decision {
        PhaseDecision::PlayerWindow { .. } => None,
        PhaseDecision::EnemyAttacks { order, index }
        | PhaseDecision::UpkeepDraw { order, index }
        | PhaseDecision::HandLimit { order, index }
        | PhaseDecision::MythosDraw { order, index } => {
            order.get(*index).copied().or(Some(game.lead))
        }
        PhaseDecision::AfterAttack(attack) => Some(attack.investigator),
        PhaseDecision::PatriceDiscard { .. } => Some(Investigator::Patrice),
        _ => Some(game.lead),
    }
}

pub(super) fn choices(game: &Game, decision: &PhaseDecision) -> Vec<Choice> {
    match decision {
        PhaseDecision::PlayerWindow { .. } => {
            let mut choices = game.player_window_choices();
            choices.push(choice("phase-window:pass", "Finish this player window"));
            choices
        }
        PhaseDecision::Hunters { remaining } => {
            let choices: Vec<_> = remaining
                .iter()
                .filter(|id| !scenario::hunter_destinations(game, **id).is_empty())
                .map(|id| {
                    choice(
                        format!("enemy:hunter:{id}"),
                        format!("Resolve Hunter movement for {}", enemy_label(game, *id)),
                    )
                })
                .collect();
            if choices.is_empty() {
                let mut choices = vec![choice("enemy:attacks", "Begin enemy attacks")];
                if !scenario::enemy_phase_has_activity(game)
                    && game.player_window_choices().is_empty()
                {
                    choices.push(choice("enemy:skip-empty", "Enemy phase — no activity"));
                }
                choices
            } else {
                choices
            }
        }
        PhaseDecision::EnemyAttacks { order, index } => {
            let Some(&investigator) = order.get(*index) else {
                return vec![choice(
                    "enemy:attacks-complete",
                    "Finish the enemy attack step",
                )];
            };
            let attacks = scenario::enemy_phase_attacks(game, investigator);
            if attacks.is_empty() {
                vec![choice(
                    "enemy:next-investigator",
                    format!("Finish {}'s enemy attacks", investigator.name()),
                )]
            } else {
                attacks
                    .into_iter()
                    .map(|attack| {
                        choice(
                            format!("enemy:attack:{}", attack.enemy),
                            format!(
                                "{} resolves the attack from {}",
                                investigator.name(),
                                enemy_label(game, attack.enemy)
                            ),
                        )
                    })
                    .collect()
            }
        }
        PhaseDecision::AfterAttack(_) => {
            vec![choice("enemy:attack-complete", "Finish this enemy attack")]
        }
        PhaseDecision::EnemyEnd => vec![choice("enemy:end", "End the enemy phase")],
        PhaseDecision::UpkeepReady => vec![choice(
            "upkeep:ready",
            "Reset actions and ready exhausted cards",
        )],
        PhaseDecision::UpkeepDraw { order, index } => match order.get(*index) {
            Some(investigator) => vec![choice(
                "upkeep:draw",
                format!("Resolve {}'s upkeep draw", investigator.name()),
            )],
            None => vec![choice("upkeep:draws-complete", "Finish upkeep draws")],
        },
        PhaseDecision::PatriceDiscard { selected, .. } => {
            let remaining: Vec<_> = game.players[Investigator::Patrice.index()]
                .hand
                .iter()
                .filter(|card| !card.weakness && !selected.contains(&card.id))
                .map(|card| {
                    choice(
                        format!("upkeep:patrice-discard:{}", card.id),
                        format!("Place {} next in Patrice's discard order", card.title),
                    )
                })
                .collect();
            if remaining.is_empty() {
                vec![choice(
                    "upkeep:patrice-refill",
                    "Discard the selected cards and refill Patrice's hand",
                )]
            } else {
                remaining
            }
        }
        PhaseDecision::UpkeepResources => vec![choice(
            "upkeep:resources",
            "Each remaining investigator gains 1 resource",
        )],
        PhaseDecision::HandLimit { order, index } => {
            let Some(&investigator) = order.get(*index) else {
                return vec![choice(
                    "upkeep:hand-limits-complete",
                    "Finish upkeep hand-size checks",
                )];
            };
            let player = &game.players[investigator.index()];
            let limit = if investigator == Investigator::Patrice {
                5
            } else {
                8
            };
            if player.hand.len() <= limit {
                vec![choice(
                    "upkeep:next-hand",
                    format!("{} is within the hand limit", investigator.name()),
                )]
            } else {
                player
                    .hand
                    .iter()
                    .filter(|card| !card.weakness && card.code != "06017")
                    .map(|card| {
                        choice(
                            format!("upkeep:discard:{}", card.id),
                            format!(
                                "{} discards {} for hand size",
                                investigator.name(),
                                card.title
                            ),
                        )
                    })
                    .collect()
            }
        }
        PhaseDecision::RoundEnd => vec![choice("round:end", "Reach the end-of-round timing point")],
        PhaseDecision::MythosBegin => vec![choice(
            "mythos:begin",
            "Place the next mythos doom and check the agenda",
        )],
        PhaseDecision::MythosAfterAgenda => vec![choice(
            "mythos:encounters",
            "Begin encounter draws after the agenda resolves",
        )],
        PhaseDecision::MythosDraw { order, index } => match order.get(*index) {
            Some(investigator) => vec![choice(
                "mythos:draw",
                format!("{} draws an encounter card", investigator.name()),
            )],
            None => vec![choice(
                "mythos:draws-complete",
                "Finish mythos encounter draws",
            )],
        },
        PhaseDecision::InvestigationBegin => vec![choice(
            "investigation:begin",
            "Begin the investigation phase",
        )],
    }
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<(), RuleError> {
    let Some(Frame::Phase(decision)) = game.frames.last().cloned() else {
        return Err(RuleError("No framework step is pending.".into()));
    };
    if !choices(game, &decision)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That framework choice is no longer available.".into(),
        ));
    }
    if id == "enemy:skip-empty" {
        // Keep the ordinary framework path (and old recorded choices) intact.
        // Admission above proves there are no remaining effects or opportunities.
        choose(game, "enemy:attacks")?;
        while let Some(Frame::Phase(PhaseDecision::EnemyAttacks { order, index })) =
            game.frames.last()
        {
            let next = if *index < order.len() {
                "enemy:next-investigator"
            } else {
                "enemy:attacks-complete"
            };
            choose(game, next)?;
        }
        choose(game, "enemy:end")?;
        game.capture_presentation("Enemy phase — no activity. Upkeep begins.");
        return Ok(());
    }
    game.frames.pop();
    match decision {
        PhaseDecision::PlayerWindow { next } => {
            if let Some(next) = next {
                push(game, *next);
            }
        }
        PhaseDecision::Hunters { mut remaining } => {
            if id == "enemy:attacks" {
                player_window(
                    game,
                    Some(PhaseDecision::EnemyAttacks {
                        order: scenario::player_order(game),
                        index: 0,
                    }),
                );
            } else {
                let enemy = remaining
                    .iter()
                    .copied()
                    .find(|enemy| id == format!("enemy:hunter:{enemy}"))
                    .expect("Validated Hunter");
                remaining.retain(|candidate| *candidate != enemy);
                push(game, PhaseDecision::Hunters { remaining });
                let destinations = scenario::hunter_destinations(game, enemy);
                if destinations.len() == 1 {
                    game.scenario
                        .enemies
                        .iter_mut()
                        .find(|state| state.card.id == enemy)
                        .expect("Ready Hunter")
                        .location = destinations[0].clone();
                    game.check_engagement(enemy);
                } else {
                    game.frames
                        .push(Frame::Scenario(ScenarioDecision::HunterMovement {
                            enemy,
                            destinations,
                        }));
                }
            }
        }
        PhaseDecision::EnemyAttacks { order, index } => {
            if id == "enemy:attacks-complete" {
                player_window(game, Some(PhaseDecision::EnemyEnd));
            } else if id == "enemy:next-investigator" {
                let another = index + 1 < order.len();
                let next = PhaseDecision::EnemyAttacks {
                    order,
                    index: index + 1,
                };
                if another {
                    player_window(game, Some(next));
                } else {
                    push(game, next);
                }
            } else {
                let attack = scenario::enemy_phase_attacks(game, order[index])
                    .into_iter()
                    .find(|attack| id == format!("enemy:attack:{}", attack.enemy))
                    .expect("Validated attack");
                push(game, PhaseDecision::EnemyAttacks { order, index });
                push(game, PhaseDecision::AfterAttack(attack.clone()));
                game.queue_harm(
                    attack.investigator,
                    attack.damage,
                    attack.horror,
                    false,
                    &attack.source_code,
                    HarmOrigin::EnemyAttack {
                        enemy: attack.enemy,
                    },
                )?;
            }
        }
        PhaseDecision::AfterAttack(attack) => scenario::complete_enemy_attack(game, &attack),
        PhaseDecision::EnemyEnd => {
            let moved = scenario::finish_enemy_phase(game);
            game.phase = Phase::Upkeep;
            for player in &mut game.players {
                player.completed_actions_this_phase = 0;
            }
            player_window(game, Some(PhaseDecision::UpkeepReady));
            for enemy in moved.into_iter().rev() {
                game.check_engagement(enemy);
            }
            game.capture_presentation("The enemy phase ends. Upkeep begins.");
        }
        PhaseDecision::UpkeepReady => {
            for player in &mut game.players {
                if !player.defeated && !player.resigned {
                    player.ordinary_actions = 3;
                    player.turn_finished = false;
                    player.restricted_tony_action_used = false;
                    for asset in &mut player.assets {
                        asset.ready = true;
                    }
                }
            }
            let engage = scenario::ready_enemies_for_upkeep(game);
            push(
                game,
                PhaseDecision::UpkeepDraw {
                    order: scenario::player_order(game),
                    index: 0,
                },
            );
            for enemy in engage.into_iter().rev() {
                game.check_engagement(enemy);
            }
            game.capture_presentation("Ready exhausted cards during upkeep.");
        }
        PhaseDecision::UpkeepDraw { order, index } => {
            if let Some(&investigator) = order.get(index) {
                if investigator == Investigator::Patrice {
                    push(
                        game,
                        PhaseDecision::PatriceDiscard {
                            order,
                            index,
                            selected: vec![],
                        },
                    );
                } else {
                    push(
                        game,
                        PhaseDecision::UpkeepDraw {
                            order,
                            index: index + 1,
                        },
                    );
                    game.draw_player(investigator, 1)?;
                }
            } else {
                push(game, PhaseDecision::UpkeepResources);
            }
        }
        PhaseDecision::PatriceDiscard {
            order,
            index,
            mut selected,
        } => {
            if id == "upkeep:patrice-refill" {
                let player = &mut game.players[Investigator::Patrice.index()];
                for card_id in selected {
                    let at = player
                        .hand
                        .iter()
                        .position(|card| card.id == card_id)
                        .expect("Selected upkeep card");
                    player.discard.push(player.hand.remove(at));
                }
                // FAQ1.6/Legacy2.5: one simultaneous draw, then revelations;
                // do not refill again after a drawn weakness leaves the hand.
                let count = 5usize.saturating_sub(player.hand.len());
                game.capture_presentation(
                    "Patrice discards her non-weakness hand before drawing her replacement hand.",
                );
                push(
                    game,
                    PhaseDecision::UpkeepDraw {
                        order,
                        index: index + 1,
                    },
                );
                game.draw_player(Investigator::Patrice, count)?;
            } else {
                let card = id
                    .rsplit(':')
                    .next()
                    .and_then(|id| id.parse().ok())
                    .expect("Validated card");
                selected.push(card);
                push(
                    game,
                    PhaseDecision::PatriceDiscard {
                        order,
                        index,
                        selected,
                    },
                );
            }
        }
        PhaseDecision::UpkeepResources => {
            for player in &mut game.players {
                if !player.defeated && !player.resigned {
                    player.resources = player
                        .resources
                        .checked_add(1)
                        .ok_or_else(|| RuleError("Resource counter overflow.".into()))?;
                }
            }
            push(
                game,
                PhaseDecision::HandLimit {
                    order: scenario::player_order(game),
                    index: 0,
                },
            );
        }
        PhaseDecision::HandLimit { order, index } => {
            if id == "upkeep:hand-limits-complete" {
                push(game, PhaseDecision::RoundEnd);
            } else if id == "upkeep:next-hand" {
                push(
                    game,
                    PhaseDecision::HandLimit {
                        order,
                        index: index + 1,
                    },
                );
            } else {
                let player = &mut game.players[order[index].index()];
                let at = player
                    .hand
                    .iter()
                    .position(|card| id == format!("upkeep:discard:{}", card.id))
                    .expect("Validated hand discard");
                player.discard.push(player.hand.remove(at));
                push(game, PhaseDecision::HandLimit { order, index });
            }
        }
        PhaseDecision::RoundEnd => {
            game.phase = Phase::RoundEnd;
            if game.act == 2 {
                game.frames
                    .push(Frame::Scenario(ScenarioDecision::RoundEndWhen));
            } else if matches!(scenario::queue_round_end_at(game), ScenarioStep::RoundEnded) {
                game.round += 1;
                begin_mythos(game);
                super::ethereal_form::expire(game);
            } else {
                super::ethereal_form::queue_expiry(game);
            }
        }
        PhaseDecision::MythosBegin => {
            game.agenda_doom = game
                .agenda_doom
                .checked_add(1)
                .ok_or_else(|| RuleError("Agenda doom counter overflow.".into()))?;
            game.capture_presentation(
                "Place one doom for the mythos phase, then check the agenda threshold.",
            );
            if let Some(consequence) = scenario::check_agenda(game)? {
                // Agenda consequences may eliminate the lead. Determine the
                // following player order only after those effects finish.
                push(game, PhaseDecision::MythosAfterAgenda);
                game.scenario_consequence(consequence)?;
            } else {
                push(
                    game,
                    PhaseDecision::MythosDraw {
                        order: scenario::player_order(game),
                        index: 0,
                    },
                );
            }
        }
        PhaseDecision::MythosAfterAgenda => push(
            game,
            PhaseDecision::MythosDraw {
                order: scenario::player_order(game),
                index: 0,
            },
        ),
        PhaseDecision::MythosDraw { order, index } => {
            scenario::rebuild_empty_encounter(game);
            if let Some(&investigator) = order.get(index) {
                push(
                    game,
                    PhaseDecision::MythosDraw {
                        order,
                        index: index + 1,
                    },
                );
                super::encounter_search::begin_mythos_draw(game, investigator)?;
            } else {
                player_window(game, Some(PhaseDecision::InvestigationBegin));
            }
        }
        PhaseDecision::InvestigationBegin => {
            begin_investigation(game);
        }
    }
    Ok(())
}

fn push(game: &mut Game, decision: PhaseDecision) {
    game.frames.push(Frame::Phase(decision));
}

fn player_window(game: &mut Game, next: Option<PhaseDecision>) {
    if game.player_window_choices().is_empty() {
        if let Some(next) = next {
            push(game, next);
        }
    } else {
        push(
            game,
            PhaseDecision::PlayerWindow {
                next: next.map(Box::new),
            },
        );
    }
}

pub(super) fn begin_investigation(game: &mut Game) {
    game.phase = Phase::Investigation;
    game.actor = None;
    for player in &mut game.players {
        player.completed_actions_this_phase = 0;
    }
    player_window(game, None);
    game.capture_presentation("The investigation phase begins.");
}
pub(super) fn begin_enemy(game: &mut Game) {
    game.phase = Phase::Enemy;
    game.actor = None;
    for player in &mut game.players {
        player.completed_actions_this_phase = 0;
    }
    game.frames.push(Frame::Phase(PhaseDecision::Hunters {
        remaining: super::scenario::ready_hunters(game),
    }));
    game.capture_presentation("The enemy phase begins.");
}
pub(super) fn begin_mythos(game: &mut Game) {
    game.phase = Phase::Mythos;
    game.jacqueline_uses = 0;
    for player in &mut game.players {
        player.frozen_action_used = false;
        player.completed_actions_this_phase = 0;
    }
    game.frames.push(Frame::Phase(PhaseDecision::MythosBegin));
    game.capture_presentation("A new round begins with the mythos phase.");
}
