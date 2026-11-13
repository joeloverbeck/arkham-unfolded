//! Original Gathering locations, enemies and scenario consequences.
//!
//! The source contract is the original core identities, Legacy FAQ 2.5 and
//! `docs/research/issue-89-rules-scenario-tony.md` (S1–S8). These are finite
//! scenario rules; the runtime owns player decisions and interrupted tests.

use super::runtime::{Frame, Game, HostileFixture, PendingRevelation, Phase};
use super::{CardView, Investigator, cards, content};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

pub(super) const STUDY: &str = "01111";
pub(super) const HALLWAY: &str = "01112";
pub(super) const ATTIC: &str = "01113";
pub(super) const CELLAR: &str = "01114";
pub(super) const PARLOR: &str = "01115";
/// Obscuring Fog (01168): "Attached location gets +2 shroud."
pub(super) const FOG_SHROUD: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Prey {
    Any,
    HighestCombat,
    LowestRemainingHealth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EnemyDefinition {
    pub combat: u8,
    pub health: u8,
    pub evade: u8,
    pub damage: u8,
    pub horror: u8,
    pub ghoul: bool,
    pub monster: bool,
    pub elite: bool,
    pub hunter: bool,
    pub retaliate: bool,
    pub prey: Prey,
    pub spawn: Option<&'static str>,
    pub victory: u8,
}

/// Values are for the fixed three-investigator start, which elimination never
/// reduces. Original 01116 has five health per starting investigator.
pub(super) fn enemy_definition(code: &str) -> Option<EnemyDefinition> {
    let mut enemy = EnemyDefinition {
        combat: 2,
        health: 2,
        evade: 2,
        damage: 1,
        horror: 1,
        ghoul: true,
        monster: true,
        elite: false,
        hunter: false,
        retaliate: false,
        prey: Prey::Any,
        spawn: None,
        victory: 0,
    };
    match code {
        "01116" => {
            enemy.combat = 4;
            enemy.health = 15;
            enemy.evade = 4;
            enemy.damage = 2;
            enemy.horror = 2;
            enemy.elite = true;
            enemy.hunter = true;
            enemy.retaliate = true;
            enemy.prey = Prey::HighestCombat;
            enemy.victory = 2;
        }
        "01118" => {
            enemy.combat = 4;
            enemy.health = 4;
            enemy.evade = 1;
            enemy.horror = 2;
            enemy.spawn = Some(ATTIC);
            enemy.victory = 1;
        }
        "01119" => {
            enemy.combat = 3;
            enemy.health = 4;
            enemy.evade = 4;
            enemy.damage = 2;
            enemy.spawn = Some(CELLAR);
            enemy.victory = 1;
        }
        "01159" => {
            enemy.combat = 1;
            enemy.health = 1;
            enemy.evade = 3;
            enemy.horror = 0;
            enemy.ghoul = false;
            enemy.monster = false;
            enemy.hunter = true;
        }
        "01160" => {}
        "01161" => {
            enemy.combat = 3;
            enemy.health = 3;
            enemy.evade = 3;
            enemy.prey = Prey::LowestRemainingHealth;
        }
        _ => return None,
    }
    Some(enemy)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationState {
    pub code: String,
    pub revealed: bool,
    pub clues: u8,
    pub fog: Option<CardView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyState {
    pub card: CardView,
    pub location: String,
    pub engaged: Option<Investigator>,
    pub exhausted: bool,
    pub damage: u8,
    pub doom: u8,
    pub bounties: u8,
    /// Player weaknesses return to their owner's discard, not the encounter pile.
    pub owner: Option<Investigator>,
}

impl EnemyState {
    /// The same effective capacity used by damage and defeat resolution.
    pub fn health(&self) -> Option<u8> {
        enemy_health(&self.card.code)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioState {
    pub locations: Vec<LocationState>,
    pub enemies: Vec<EnemyState>,
    pub victory: Vec<CardView>,
    pub removed: Vec<CardView>,
    pub lita_in_play: bool,
    pub priest_spawned: bool,
    pub outcome: Option<ResolutionRecord>,
}

/// A visible pile member, including a location collected at scenario end.
/// This is derived from the displayed observation, never serialized into saves.
#[cfg(feature = "tabletop")]
pub(crate) struct VictoryDisplayCard<'a> {
    pub code: &'a str,
    pub title: &'a str,
    pub physical_id: Option<u16>,
    pub points: u8,
}

impl LocationState {
    fn awards_victory(&self) -> bool {
        self.revealed && self.clues == 0 && matches!(self.code.as_str(), ATTIC | CELLAR)
    }
}

impl ScenarioState {
    #[cfg(feature = "tabletop")]
    pub(crate) fn location_in_victory_display(&self, code: &str) -> bool {
        self.outcome.is_some()
            && self
                .locations
                .iter()
                .any(|room| room.code == code && room.awards_victory())
    }

    /// Eligible locations remain in the terminal map state for resolution accounting.
    /// Project their scenario-end transfer without rewriting those records or XP.
    #[cfg(feature = "tabletop")]
    pub(crate) fn victory_display(&self) -> Vec<VictoryDisplayCard<'_>> {
        self.victory
            .iter()
            .map(|card| VictoryDisplayCard {
                code: &card.code,
                title: &card.title,
                physical_id: Some(card.id),
                points: enemy_definition(&card.code).map_or(
                    u8::from(matches!(card.code.as_str(), ATTIC | CELLAR)),
                    |enemy| enemy.victory,
                ),
            })
            .chain(
                self.locations
                    .iter()
                    .filter(|room| {
                        self.outcome.is_some()
                            && room.awards_victory()
                            && !self.victory.iter().any(|card| card.code == room.code)
                    })
                    .map(|room| VictoryDisplayCard {
                        code: &room.code,
                        title: content::title(&room.code).expect("Gathering victory location"),
                        physical_id: None,
                        points: 1,
                    }),
            )
            .collect()
    }

    pub(super) fn new() -> Self {
        Self {
            locations: vec![LocationState {
                code: STUDY.into(),
                revealed: true,
                clues: 6,
                fog: None,
            }],
            enemies: vec![],
            victory: vec![],
            removed: vec![],
            lita_in_play: false,
            priest_spawned: false,
            outcome: None,
        }
    }

    pub(super) fn location(&self, code: &str) -> Option<&LocationState> {
        self.locations.iter().find(|location| location.code == code)
    }

    /// The position immediately after Trapped resolves, before anyone explores
    /// another room. Also used by the explicitly privileged research fixtures.
    pub(super) fn gathering_after_act_one() -> Self {
        let mut state = Self::new();
        state.locations = [HALLWAY, ATTIC, CELLAR, PARLOR]
            .into_iter()
            .map(|code| LocationState {
                code: code.into(),
                revealed: code == HALLWAY,
                clues: 0,
                fog: None,
            })
            .collect();
        state
    }

    pub(super) fn reveal(&mut self, code: &str) {
        if let Some(location) = self.location_mut(code)
            && !location.revealed
        {
            location.revealed = true;
            location.clues = if matches!(code, STUDY | ATTIC | CELLAR) {
                6
            } else {
                0
            };
        }
    }

    pub(super) fn location_mut(&mut self, code: &str) -> Option<&mut LocationState> {
        self.locations
            .iter_mut()
            .find(|location| location.code == code)
    }

    pub(super) fn shroud(&self, code: &str) -> Option<u8> {
        let location = self.location(code)?;
        if !location.revealed {
            return None;
        }
        let printed = match code {
            STUDY | PARLOR => 2,
            HALLWAY | ATTIC => 1,
            CELLAR => 4,
            _ => return None,
        };
        let fog = if location.fog.is_some() {
            FOG_SHROUD
        } else {
            0
        };
        Some(printed + fog)
    }

    /// Connections do not themselves grant an investigator permission to enter
    /// the unrevealed Parlor. Enemies are not subject to its "you" restriction.
    pub(super) fn connected(&self, from: &str, to: &str) -> bool {
        self.location(from).is_some()
            && self.location(to).is_some()
            && ((from == HALLWAY && matches!(to, ATTIC | CELLAR | PARLOR))
                || (to == HALLWAY && matches!(from, ATTIC | CELLAR | PARLOR)))
    }

    pub(super) fn can_move(&self, from: &str, to: &str) -> bool {
        self.connected(from, to)
            && !(to == PARLOR && self.location(PARLOR).is_some_and(|p| !p.revealed))
    }

    pub(super) fn distance(&self, from: &str, to: &str) -> Option<u8> {
        if self.location(from).is_none() || self.location(to).is_none() {
            None
        } else if from == to {
            Some(0)
        } else if self.connected(from, to) {
            Some(1)
        } else if from != STUDY && to != STUDY {
            Some(2)
        } else {
            None
        }
    }

    pub(super) fn ghouls_at(&self, location: &str) -> u8 {
        self.enemies
            .iter()
            .filter(|enemy| {
                enemy.location == location
                    && enemy_definition(&enemy.card.code).is_some_and(|definition| definition.ghoul)
            })
            .count() as u8
    }

    /// 01107 at enemy-phase end is not a Hunter step: exhaustion does not stop
    /// this movement. This does not ready enemies, engage exhausted enemies or
    /// grant another attack step. The runtime checks engagement afterward.
    pub(super) fn move_unengaged_ghouls_toward_parlor(&mut self) {
        let hallway_present = self.location(HALLWAY).is_some();
        let parlor_present = self.location(PARLOR).is_some();
        for enemy in &mut self.enemies {
            if enemy.engaged.is_some()
                || !enemy_definition(&enemy.card.code).is_some_and(|definition| definition.ghoul)
            {
                continue;
            }
            match enemy.location.as_str() {
                ATTIC | CELLAR if hallway_present => enemy.location = HALLWAY.into(),
                HALLWAY if parlor_present => enemy.location = PARLOR.into(),
                _ => {}
            }
        }
    }

    pub(super) fn round_end_ghoul_doom(&self) -> u8 {
        self.ghouls_at(HALLWAY) + self.ghouls_at(PARLOR)
    }

    pub(super) fn victory_experience(&self) -> u8 {
        let enemies: u8 = self
            .victory
            .iter()
            .filter_map(|card| enemy_definition(&card.code))
            .map(|definition| definition.victory)
            .sum();
        let locations = self
            .locations
            .iter()
            .filter(|location| location.awards_victory())
            .count() as u8;
        enemies + locations
    }
}

/// Exact scenario decisions that may interrupt the normal phase/test sequence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ScenarioDecision {
    AgendaOne,
    AgendaTwoSearch,
    Ending,
    Engagement {
        enemy: u16,
        investigators: Vec<Investigator>,
    },
    HunterMovement {
        enemy: u16,
        destinations: Vec<String>,
    },
    LitaRecipient,
    RoundEndWhen,
    RoundEndComplete,
    RoundEndAt {
        agenda: bool,
        voices: Vec<(Investigator, u16)>,
    },
}

/// Concrete handoffs to the runtime's timing and harm coordinator. None of
/// these permits an ordinary action or opens an extra player window.
pub(super) enum ScenarioStep {
    Continue,
    LeadHorror,
    EnemyEntered(u16),
    CheckEngagement(u16),
    RoundEnded,
    Finished,
}

fn choice(id: impl Into<String>, label: impl Into<String>) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        cancel_faces: vec![],
    }
}

pub(super) fn choices(game: &Game, decision: &ScenarioDecision) -> Vec<Choice> {
    match decision {
        ScenarioDecision::RoundEndComplete => vec![choice(
            "round-end:finish",
            "Finish the round after the act and its entry reactions",
        )],
        ScenarioDecision::AgendaTwoSearch => vec![choice(
            "agenda:search",
            "Resolve Rise of the Ghouls: search the encounter deck",
        )],
        ScenarioDecision::AgendaOne => {
            let mut choices = vec![choice(
                "agenda:lead-horror",
                "The lead investigator takes 2 horror",
            )];
            if agenda_one_can_discard(game) {
                choices.push(choice(
                    "agenda:random-discard",
                    "Each investigator discards 1 random card",
                ));
            }
            choices
        }
        ScenarioDecision::Ending => vec![
            choice("resolution:burn", "Burn the house (Resolution 1)"),
            choice(
                "resolution:refuse",
                "Refuse to burn the house (Resolution 2)",
            ),
        ],
        ScenarioDecision::Engagement {
            enemy,
            investigators,
        } => investigators
            .iter()
            .map(|investigator| {
                choice(
                    format!("enemy:engage:{enemy}:{}", investigator.index()),
                    format!("The enemy engages {}", investigator.name()),
                )
            })
            .collect(),
        ScenarioDecision::HunterMovement {
            enemy,
            destinations,
        } => destinations
            .iter()
            .map(|destination| {
                choice(
                    format!("hunter:move:{enemy}:{destination}"),
                    format!(
                        "Move the Hunter to {}",
                        content::title(destination).unwrap_or(destination)
                    ),
                )
            })
            .collect(),
        ScenarioDecision::LitaRecipient => {
            let Some(record) = &game.scenario.outcome else {
                return vec![];
            };
            let mut choices: Vec<_> = game
                .players
                .iter()
                .filter(|player| !record.killed.contains(&player.investigator))
                .map(|player| {
                    choice(
                        format!("lita:recipient:{}", player.investigator.index()),
                        format!("{} earns Lita Chantler", player.investigator.name()),
                    )
                })
                .collect();
            // The accepted ending can retain the later successor choice. It
            // neither invents a successor nor assigns Lita to a killed actor.
            choices.push(choice(
                "lita:successor",
                "Choose Lita's recipient when preparing a successor investigator",
            ));
            choices
        }
        ScenarioDecision::RoundEndWhen => {
            let Some(payment) = super::group_payment::round_end_choice(game) else {
                // The Objective's optional payment is impossible, so declining
                // is the step's one legal outcome, not an offered decision.
                return vec![choice(
                    "round-end:decline-act",
                    "The Barrier does not advance: the investigators in the Hallway cannot pay its clues",
                )];
            };
            vec![
                choice(
                    "round-end:decline-act",
                    "Do not advance The Barrier at this round end",
                ),
                payment,
            ]
        }
        ScenarioDecision::RoundEndAt { agenda, voices } => {
            let mut choices = Vec::new();
            if *agenda {
                choices.push(choice(
                    "round-end:ghoul-doom",
                    "Resolve the agenda's forced Ghoul doom",
                ));
            }
            choices.extend(voices.iter().map(|(investigator, card)| {
                choice(
                    format!("round-end:voices:{card}"),
                    format!("Discard {}'s Dissonant Voices", investigator.name()),
                )
            }));
            choices
        }
    }
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<ScenarioStep, RuleError> {
    let Some(Frame::Scenario(decision)) = game.frames.last().cloned() else {
        return Err(RuleError("No scenario decision is pending.".into()));
    };
    if !choices(game, &decision)
        .iter()
        .any(|choice| choice.id == id)
    {
        return Err(RuleError(
            "That scenario choice is no longer available.".into(),
        ));
    }
    game.frames.pop();
    match decision {
        ScenarioDecision::RoundEndComplete => Ok(ScenarioStep::RoundEnded),
        ScenarioDecision::AgendaTwoSearch => {
            let found = agenda_two_ghoul(game);
            game.agenda = 3;
            if let Some(card) = found {
                match spawn_encounter_enemy(game, card, game.lead)? {
                    Some(enemy) => Ok(ScenarioStep::EnemyEntered(enemy)),
                    None => Ok(ScenarioStep::Continue),
                }
            } else {
                Ok(ScenarioStep::Continue)
            }
        }
        ScenarioDecision::AgendaOne if id == "agenda:lead-horror" => Ok(ScenarioStep::LeadHorror),
        ScenarioDecision::AgendaOne => {
            super::harm_cards::begin_agenda_discard(game);
            Ok(ScenarioStep::Continue)
        }
        ScenarioDecision::Ending => {
            finish_resolution(
                game,
                if id == "resolution:burn" {
                    Resolution::BurnHouse
                } else {
                    Resolution::RefuseToBurn
                },
            );
            Ok(ScenarioStep::Finished)
        }
        ScenarioDecision::Engagement {
            enemy,
            investigators,
        } => {
            let investigator = investigators
                .into_iter()
                .find(|investigator| id == format!("enemy:engage:{enemy}:{}", investigator.index()))
                .expect("Validated choice");
            game.scenario
                .enemies
                .iter_mut()
                .find(|state| state.card.id == enemy)
                .ok_or_else(|| RuleError("The enemy is no longer in play.".into()))?
                .engaged = Some(investigator);
            Ok(ScenarioStep::Continue)
        }
        ScenarioDecision::HunterMovement {
            enemy,
            destinations,
        } => {
            let destination = destinations
                .into_iter()
                .find(|destination| id == format!("hunter:move:{enemy}:{destination}"))
                .expect("Validated choice");
            game.scenario
                .enemies
                .iter_mut()
                .find(|state| state.card.id == enemy)
                .ok_or_else(|| RuleError("The enemy is no longer in play.".into()))?
                .location = destination;
            Ok(ScenarioStep::CheckEngagement(enemy))
        }
        ScenarioDecision::LitaRecipient => {
            if id != "lita:successor" {
                let recipient = game
                    .players
                    .iter()
                    .find(|player| id == format!("lita:recipient:{}", player.investigator.index()))
                    .expect("Validated choice")
                    .investigator;
                let record = game.scenario.outcome.as_mut().expect("Resolution record");
                record.lita_recipient = Some(recipient);
                record.lita_recipient_pending = false;
            }
            Ok(ScenarioStep::Finished)
        }
        ScenarioDecision::RoundEndWhen => {
            let step = queue_round_end_at(game);
            if matches!(step, ScenarioStep::Continue) {
                super::ethereal_form::queue_expiry(game);
            }
            Ok(step)
        }
        ScenarioDecision::RoundEndAt {
            mut agenda,
            mut voices,
        } => {
            if id == "round-end:ghoul-doom" {
                let doom = game.scenario.round_end_ghoul_doom();
                game.agenda_doom = game.agenda_doom.checked_add(doom).ok_or_else(|| {
                    RuleError("Agenda doom exceeds the supported counter range.".into())
                })?;
                agenda = false;
                game.history.push(format!("The round-end forced ability adds {doom} Ghoul doom. It does not permit agenda advancement. [01107a; RR p9]"));
            } else {
                let index = voices
                    .iter()
                    .position(|(_, card)| id == format!("round-end:voices:{card}"))
                    .expect("Validated choice");
                let (investigator, card) = voices.remove(index);
                let threats = &mut game.players[investigator.index()].threats;
                if let Some(index) = threats.iter().position(|threat| threat.id == card) {
                    game.encounter_discard.push(threats.remove(index));
                }
            }
            if agenda || !voices.is_empty() {
                game.frames
                    .push(Frame::Scenario(ScenarioDecision::RoundEndAt {
                        agenda,
                        voices,
                    }));
                Ok(ScenarioStep::Continue)
            } else {
                Ok(ScenarioStep::RoundEnded)
            }
        }
    }
}

/// Source-derived witnesses, reached only by the explicit verification
/// constructor. Every encounter/player card is moved from the actual fixed
/// inventory; only the already set-aside Priest receives its reserved ID.
pub(super) fn prepare_fixture(game: &mut Game, fixture: HostileFixture) {
    if !matches!(
        fixture,
        HostileFixture::GatheringRoundEnd
            | HostileFixture::GatheringRoundEndVoices
            | HostileFixture::GatheringPriestDefeat
            | HostileFixture::GatheringLitaDamage
            | HostileFixture::GatheringRetaliate
            | HostileFixture::GatheringLitaParley
            | HostileFixture::GatheringNonterminalDefeat
            | HostileFixture::GatheringHunters
            | HostileFixture::GatheringArrival
            | HostileFixture::GatheringAgendaTwoNoGhoul
            | HostileFixture::GatheringAncientEvilsTerminal
            | HostileFixture::GatheringDoomThreshold
            | HostileFixture::GatheringQuarryPlacement
            | HostileFixture::GatheringGhoulMovement
    ) {
        return;
    }
    for frame in std::mem::take(&mut game.frames) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        }
    }
    game.scenario = ScenarioState::gathering_after_act_one();
    game.scenario.reveal(ATTIC);
    game.scenario.reveal(CELLAR);
    game.act = 2;
    game.lead = Investigator::Tony;
    game.test = None;
    game.actor = None;
    game.history.clear();
    for player in &mut game.players {
        player.location = HALLWAY.into();
    }
    match fixture {
        HostileFixture::GatheringRoundEnd | HostileFixture::GatheringRoundEndVoices => {
            game.phase = Phase::RoundEnd;
            game.agenda = 3;
            game.agenda_doom = 8;
            for player in &mut game.players {
                player.clues = 3;
            }
            // Six Study clues were spent to open the rooms; nine of the twelve
            // Attic/Cellar clues are now carried by the investigators.
            game.scenario.location_mut(ATTIC).expect("Attic").clues = 0;
            game.scenario.location_mut(CELLAR).expect("Cellar").clues = 3;
            let minion = fixture_encounter(game, "01160");
            spawn_encounter_enemy(game, minion, Investigator::Tony).expect("Legal Minion spawn");
            game.frames
                .push(Frame::Scenario(ScenarioDecision::RoundEndWhen));
            if fixture == HostileFixture::GatheringRoundEndVoices {
                let voices = fixture_encounter(game, "01165");
                game.players[Investigator::Patrice.index()]
                    .threats
                    .push(voices);
            }
            game.history.push("Privileged S1/S5 witness: Act2, Agenda3 eight doom, nine Hallway clues, one Hallway Ghoul. The WHEN Act2 opportunity must precede AT doom and is optional. [01109a/01107a; FAQ2.2]".into());
        }
        HostileFixture::GatheringPriestDefeat
        | HostileFixture::GatheringLitaDamage
        | HostileFixture::GatheringRetaliate => {
            game.phase = Phase::Investigation;
            game.actor = Some(Investigator::Tony);
            game.agenda = 1;
            game.agenda_doom = 0;
            let priest = advance_act_two(game).expect("Legal Act2 reverse");
            // Five Attic and four Cellar clues paid the Act2 cost. Neither
            // location is empty, so only Priest2 + resolution bonus2 award XP.
            game.scenario.location_mut(ATTIC).expect("Attic").clues = 1;
            game.scenario.location_mut(CELLAR).expect("Cellar").clues = 2;
            let enemy = game
                .scenario
                .enemies
                .iter_mut()
                .find(|enemy| enemy.card.id == priest)
                .expect("Priest");
            enemy.damage = 14;
            enemy.bounties = 1;
            enemy.engaged = Some(Investigator::Tony);
            let tony = &mut game.players[Investigator::Tony.index()];
            tony.assets
                .iter_mut()
                .find(|asset| asset.card.code == "06010")
                .expect("Contracts")
                .uses = 5;
            let injury = fixture_player_card(tony, "02038");
            tony.threats.push(injury);
            for code in ["02225", "02229"] {
                let card = fixture_player_card(tony, code);
                tony.hand.push(card);
            }
            game.history.push("Privileged S4 witness: Tony basic-fights the fifteen-health Priest with fourteen damage and one bounty. Success must put it in victory and resolve the ending before after-defeat/after-test/turn-end rewards or harm. [01110a; FAQ2.2]".into());
            if fixture == HostileFixture::GatheringLitaDamage {
                game.scenario
                    .enemies
                    .iter_mut()
                    .find(|enemy| enemy.card.id == priest)
                    .expect("Priest")
                    .damage = 13;
                game.players[Investigator::Patrice.index()].location = PARLOR.into();
                take_control_of_lita(game, Investigator::Patrice).expect("Prior successful Parley");
                game.players[Investigator::Patrice.index()].location = HALLWAY.into();
                game.history.push("Lita reaction witness: Patrice controls Lita in Hallway; Tony has +1 combat. Priest has thirteen damage, so declining optional +1damage leaves it alive while using it defeats it. [01117]".into());
            } else if fixture == HostileFixture::GatheringRetaliate {
                let vision =
                    fixture_player_card(&mut game.players[Investigator::Patrice.index()], "05041");
                game.players[Investigator::Patrice.index()]
                    .threats
                    .push(vision);
                game.history.push("Retaliate witness: The 13th Vision makes Tony's unmodified 5-minus1 versus4 tie fail with zero failure margin. The ready Priest attacks him after all results and does not exhaust. [05041; RR p18]".into());
            }
        }
        HostileFixture::GatheringLitaParley => {
            game.phase = Phase::Investigation;
            game.actor = Some(Investigator::Patrice);
            game.agenda = 1;
            game.agenda_doom = 0;
            let priest = advance_act_two(game).expect("Legal Act2 reverse");
            game.scenario
                .enemies
                .iter_mut()
                .find(|enemy| enemy.card.id == priest)
                .expect("Priest")
                .engaged = Some(Investigator::Tony);
            game.scenario.location_mut(ATTIC).expect("Attic").clues = 1;
            game.scenario.location_mut(CELLAR).expect("Cellar").clues = 2;
            let patrice = &mut game.players[Investigator::Patrice.index()];
            patrice.location = PARLOR.into();
            let peter = fixture_player_card(patrice, "02033");
            patrice.assets.push(super::runtime::AssetState {
                card: peter,
                ready: true,
                uses: 0,
                damage: 0,
                horror: 0,
            });
            let rise = fixture_player_card(patrice, "02192");
            patrice.hand.push(rise);
            game.history.push("Lita Parley witness: Patrice in revealed Parlor controls Peter Sylvestre; Rise to the Occasion makes intellect2 plus3 minus1 pass difficulty4. Taking control of Lita requires resolving the ally slot limit. [01117/02033/02192; RR p19]".into());
        }
        HostileFixture::GatheringNonterminalDefeat => {
            game.phase = Phase::Investigation;
            game.actor = Some(Investigator::Tony);
            game.agenda = 1;
            game.agenda_doom = 0;
            let minion = fixture_encounter(game, "01160");
            let id = minion.id;
            spawn_encounter_enemy(game, minion, Investigator::Tony).expect("Legal Minion spawn");
            let enemy = game
                .scenario
                .enemies
                .iter_mut()
                .find(|enemy| enemy.card.id == id)
                .expect("Minion");
            enemy.damage = 1;
            enemy.bounties = 2;
            let tony = &mut game.players[Investigator::Tony.index()];
            tony.horror = 2;
            tony.assets
                .iter_mut()
                .find(|asset| asset.card.code == "06010")
                .expect("Contracts")
                .uses = 4;
            let bleeds = fixture_player_card(tony, "02225");
            tony.hand.push(bleeds);
            game.players[Investigator::Patrice.index()].horror = 3;
            game.players[Investigator::Jacqueline.index()].horror = 2;
            game.players[Investigator::Jacqueline.index()].location = ATTIC.into();
            game.history.push("Nonterminal defeat witness: Tony defeats a Ghoul Minion bearing two bounties. Forced Contracts payout precedes optional If it bleeds (cost1, printed horror1); only active investigators at Tony's location heal. [06010/02225/01160]".into());
        }
        HostileFixture::GatheringArrival => {
            game.agenda = 1;
            game.agenda_doom = 0;
            let card = fixture_encounter(game, "01118");
            spawn_encounter_enemy(game, card, Investigator::Patrice)
                .expect("Flesh-Eater spawns in the empty Attic");
            game.enable_fixture_turn(Investigator::Patrice);
            game.history.push("Privileged arrival witness: Patrice in Hallway can enter the revealed Attic containing a ready, unengaged Flesh-Eater. Its engagement and card transfer belong to her arrival. [01118; RR p10]".into());
        }
        HostileFixture::GatheringHunters => {
            game.agenda = 1;
            game.agenda_doom = 0;
            advance_act_two(game).expect("Legal Act2 reverse");
            game.scenario.location_mut(ATTIC).expect("Attic").clues = 1;
            game.scenario.location_mut(CELLAR).expect("Cellar").clues = 2;
            for (investigator, location) in [
                (Investigator::Tony, PARLOR),
                (Investigator::Patrice, CELLAR),
                (Investigator::Jacqueline, ATTIC),
            ] {
                game.players[investigator.index()].location = location.into();
            }
            for exhausted in [false, true] {
                let card = fixture_encounter(game, "01159");
                game.scenario.enemies.push(EnemyState {
                    card,
                    location: HALLWAY.into(),
                    engaged: None,
                    exhausted,
                    damage: 0,
                    doom: 0,
                    bounties: 0,
                    owner: None,
                });
            }
            super::phases::begin_enemy(game);
            game.history.push("Hunter/attack witness: ready Priest and Rats in Hallway, exhausted Rats excluded. All three investigators are equally near in separate rooms; Priest follows highest-combat Tony, Rats destination is lead choice. Enemy attacks exhaust only after completion. [01116/01159; RR pp12,17,23,25]".into());
        }
        HostileFixture::GatheringAgendaTwoNoGhoul => {
            game.phase = Phase::Mythos;
            game.agenda = 2;
            game.agenda_doom = 0;
            let cards = std::mem::take(&mut game.encounter_deck);
            for card in cards {
                if matches!(card.code.as_str(), "01118" | "01119") {
                    game.scenario.victory.push(card);
                } else if enemy_definition(&card.code).is_some_and(|definition| definition.ghoul) {
                    game.scenario.enemies.push(EnemyState {
                        card,
                        location: ATTIC.into(),
                        engaged: None,
                        exhausted: false,
                        damage: 0,
                        doom: 0,
                        bounties: 0,
                        owner: None,
                    });
                } else {
                    game.encounter_deck.push(card);
                }
            }
            game.frames
                .push(Frame::Scenario(ScenarioDecision::AgendaTwoSearch));
            game.history.push("Privileged S8 witness at Agenda2 reverse: Flesh-Eater/Icy Ghoul in victory, three Minions/Ravenous Ghoul in play, Priest set aside. The twenty available cards contain no Ghoul. Search one finite pass then rebuild; this is the recorded qualified legacy interpretation. [01106b; S8]".into());
        }
        HostileFixture::GatheringQuarryPlacement => {
            game.phase = Phase::Mythos;
            game.agenda = 1;
            game.agenda_doom = 2;
            let quarry =
                fixture_player_card(&mut game.players[Investigator::Tony.index()], "06012");
            game.frames.push(Frame::Revelation(PendingRevelation {
                card: quarry,
                drawn_by: Investigator::Tony,
                started: false,
                cancelled: false,
            }));
            game.history.push("Privileged #137 witness: Tony draws his Quarry with two doom on Agenda 1. Its revelation places enemy doom without authorizing an agenda check. [06012; RR Doom p9]".into());
        }
        HostileFixture::GatheringDoomThreshold => {
            game.phase = Phase::Mythos;
            game.agenda = 1;
            game.agenda_doom = 1;
            let quarry =
                fixture_player_card(&mut game.players[Investigator::Tony.index()], "06012");
            game.scenario.enemies.push(EnemyState {
                card: quarry,
                location: ATTIC.into(),
                engaged: None,
                exhausted: false,
                damage: 0,
                doom: 1,
                bounties: 0,
                owner: Some(Investigator::Tony),
            });
            game.frames
                .push(Frame::Phase(super::phases::PhaseDecision::MythosBegin));
            game.history.push("Privileged #137 witness: Agenda 1 has one doom and Tony's Quarry has one. Mythos adds one, checks three total, clears both bearers, then offers Agenda 1's reverse before encounter draws. [RR pp3,9,24; 01105b; 06012]".into());
        }
        HostileFixture::GatheringAncientEvilsTerminal => {
            game.phase = Phase::Mythos;
            game.agenda = 3;
            game.agenda_doom = 8;
            let quarry =
                fixture_player_card(&mut game.players[Investigator::Tony.index()], "06012");
            game.scenario.enemies.push(EnemyState {
                card: quarry,
                location: ATTIC.into(),
                engaged: None,
                exhausted: false,
                damage: 0,
                doom: 1,
                bounties: 1,
                owner: Some(Investigator::Tony),
            });
            let evils = fixture_encounter(game, "01166");
            game.frames.push(Frame::Revelation(PendingRevelation {
                card: evils,
                drawn_by: Investigator::Tony,
                started: false,
                cancelled: false,
            }));
            game.history.push("Privileged S3 witness: Act2/Agenda3 eight agenda doom plus Quarry's one doom, Ancient Evils pending during encounter draws. Its additional doom immediately advances and ends in R3 before remaining draws. [01166/01107b; RR pp9,22]".into());
        }
        HostileFixture::GatheringGhoulMovement => {
            game.phase = Phase::Enemy;
            game.agenda = 3;
            game.agenda_doom = 0;
            game.players[Investigator::Patrice.index()].location = CELLAR.into();
            game.players[Investigator::Jacqueline.index()].location = CELLAR.into();
            let minion = fixture_encounter(game, "01160");
            game.scenario.enemies.push(EnemyState {
                card: minion,
                location: ATTIC.into(),
                engaged: None,
                exhausted: true,
                damage: 0,
                doom: 0,
                bounties: 0,
                owner: None,
            });
            game.frames
                .push(Frame::Phase(super::phases::PhaseDecision::EnemyEnd));
            game.history.push("Privileged S2 witness at enemy-phase end: an exhausted unengaged Ghoul Minion in Attic moves towards Parlor despite lacking Hunter. It remains exhausted in Hallway, then readies and engages Tony at upkeep4.3 without attacking again. [01107a; RR pp10,25]".into());
        }
        _ => unreachable!(),
    }
}

fn fixture_encounter(game: &mut Game, code: &str) -> CardView {
    let index = game
        .encounter_deck
        .iter()
        .position(|card| card.code == code)
        .expect("Fixed encounter inventory");
    game.encounter_deck.remove(index)
}

fn fixture_player_card(player: &mut super::runtime::PlayerState, code: &str) -> CardView {
    for pile in [&mut player.hand, &mut player.deck, &mut player.discard] {
        if let Some(index) = pile.iter().position(|card| card.code == code) {
            return pile.remove(index);
        }
    }
    panic!("Fixed player inventory is missing {code}");
}

pub(super) fn queue_round_end_at(game: &mut Game) -> ScenarioStep {
    let voices: Vec<_> = game
        .players
        .iter()
        .filter(|player| !player.defeated && !player.resigned)
        .flat_map(|player| {
            player
                .threats
                .iter()
                .filter(|card| card.code == "01165")
                .map(|card| (player.investigator, card.id))
        })
        .collect();
    let agenda = game.agenda == 3;
    if agenda || !voices.is_empty() {
        game.frames
            .push(Frame::Scenario(ScenarioDecision::RoundEndAt {
                agenda,
                voices,
            }));
        ScenarioStep::Continue
    } else {
        ScenarioStep::RoundEnded
    }
}

/// Terminal resolution replaces all pending ordinary continuations. Explicit
/// resolution choices (such as an eligible Lita recipient) may still follow.
pub(super) fn finish_resolution(game: &mut Game, resolution: Resolution) {
    game.scenario.outcome = Some(resolution_record(game, resolution));
    game.suspend_for_resolution();
    if game
        .scenario
        .outcome
        .as_ref()
        .is_some_and(|record| record.lita_recipient_pending)
    {
        game.frames
            .push(Frame::Scenario(ScenarioDecision::LitaRecipient));
    }
    game.history.push(format!("The Gathering ends: {resolution:?}. No remaining action, test, reward or phase continuation is resolved. [RR p22; CG pp2–3]"));
    game.capture_presentation(match resolution {
        Resolution::NoResolution => {
            "You escape with Lita, who urges you to confront the threat beneath Arkham."
        }
        Resolution::BurnHouse => "The house burns. Lita leads you away to face the wider threat.",
        Resolution::RefuseToBurn => {
            "You save your home and reject Lita's demand, but listen to her warning."
        }
        Resolution::Overrun => "The barrier traps those still inside as the ghouls close in.",
    });
}

/// An agenda reverse must finish through the runtime's pending-decision path
/// before the next agenda becomes current. In particular, Agenda 1 horror must
/// retain prevention/allocation opportunities instead of becoming direct harm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ScenarioConsequence {
    AgendaOneChoice,
    AgendaTwoGhoulSearch,
    ResolutionThree,
    DefeatNonResigned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    NoResolution,
    BurnHouse,
    RefuseToBurn,
    Overrun,
}

/// The Gathering's awards and instructions for later preparation. Experience
/// and trauma here are gains from this ending, not replacements for prior state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionRecord {
    pub resolution: Resolution,
    pub house_burned: bool,
    pub ghoul_priest_alive: bool,
    pub lita_forced_to_find_others: bool,
    pub experience_gained: [u8; 3],
    pub physical_trauma_gained: [u8; 3],
    pub mental_trauma_gained: [u8; 3],
    pub killed: Vec<Investigator>,
    pub lita_recipient: Option<Investigator>,
    /// CG p3 may require choosing a successor investigator. Do not invent that
    /// investigator or silently award the card to a killed member of this party.
    pub lita_recipient_pending: bool,
}

pub(super) fn resolution_record(game: &Game, resolution: Resolution) -> ResolutionRecord {
    let xp = if resolution == Resolution::Overrun {
        0
    } else {
        game.scenario.victory_experience() + 2
    };
    let mut record = ResolutionRecord {
        resolution,
        house_burned: resolution == Resolution::BurnHouse,
        ghoul_priest_alive: matches!(resolution, Resolution::NoResolution | Resolution::Overrun),
        lita_forced_to_find_others: resolution == Resolution::Overrun,
        experience_gained: [xp; 3],
        physical_trauma_gained: [0; 3],
        mental_trauma_gained: [0; 3],
        killed: vec![],
        lita_recipient: None,
        lita_recipient_pending: false,
    };
    match resolution {
        Resolution::BurnHouse => {
            record.mental_trauma_gained[game.lead.index()] = 1;
            record.lita_recipient = Some(game.lead);
        }
        Resolution::RefuseToBurn => record.experience_gained[game.lead.index()] += 1,
        Resolution::NoResolution => record.lita_recipient = Some(game.lead),
        Resolution::Overrun => {
            record.killed = game
                .players
                .iter()
                .filter(|player| !player.resigned)
                .map(|player| player.investigator)
                .collect();
            record.lita_recipient_pending = record.killed.contains(&game.lead);
        }
    }
    record
}

/// The group clue requirement at this timing: 2 (Trapped) or 3 (The Barrier)
/// per investigator who started the scenario, whether or not still playing.
pub(super) fn act_requirement(game: &Game, act: u8, at_round_end: bool) -> Option<u8> {
    let per_investigator = match (act, at_round_end) {
        (1, false) => 2,
        (2, true) => 3,
        _ => return None,
    };
    u8::try_from(game.players.len())
        .ok()
        .and_then(|started| started.checked_mul(per_investigator))
}

/// The Barrier's Objective, paid with a validated allocation: the round-end
/// sequence continues after the act's nested spawn and its reactions.
pub(super) fn advance_at_round_end(
    game: &mut Game,
    allocation: super::group_payment::Allocation,
) -> Result<ScenarioStep, RuleError> {
    let Some(Frame::Scenario(ScenarioDecision::RoundEndWhen)) = game.frames.pop() else {
        return Err(RuleError("The round-end Objective is not pending.".into()));
    };
    if matches!(queue_round_end_at(game), ScenarioStep::RoundEnded) {
        // If Agenda1/2 and no Voices leave no AT ability, the
        // round still ends after Act2's nested spawn/reactions.
        game.frames
            .push(Frame::Scenario(ScenarioDecision::RoundEndComplete));
    }
    super::ethereal_form::queue_expiry(game);
    super::group_payment::spend(game, allocation, true)?;
    advance_act_two(game).map(ScenarioStep::EnemyEntered)
}

/// Original Trapped reverse. Discard is not defeat: no victory or after-defeat
/// reward is generated for the removed Study enemies.
pub(super) fn advance_act_one(game: &mut Game) {
    let enemies = std::mem::take(&mut game.scenario.enemies);
    for enemy in enemies {
        if enemy.location == STUDY {
            if let Some(owner) = enemy.owner {
                game.players[owner.index()].discard.push(enemy.card);
            } else {
                game.encounter_discard.push(enemy.card);
            }
        } else {
            game.scenario.enemies.push(enemy);
        }
    }
    if let Some(study) = game.scenario.location_mut(STUDY)
        && let Some(fog) = study.fog.take()
    {
        game.encounter_discard.push(fog);
    }
    game.scenario.locations = ScenarioState::gathering_after_act_one().locations;
    for player in &mut game.players {
        if !player.defeated && !player.resigned {
            player.location = HALLWAY.into();
        }
    }
    game.act = 2;
    game.history.push("Trapped opens the four rooms, discards Study enemies without defeating them, places the investigators in Hallway and removes Study. [01108b]".into());
    game.capture_presentation(
        "A doorway beneath the rug leads into the altered hallway. The study closes behind you.",
    );
}

/// Original Barrier reverse. Engagement/prey ties and Bounty Contracts remain
/// runtime choices immediately after this spawn; no ordinary action is spent.
pub(super) fn advance_act_two(game: &mut Game) -> Result<u16, RuleError> {
    if game.act != 2 || game.scenario.priest_spawned || game.scenario.location(PARLOR).is_none() {
        return Err(RuleError(
            "The set-aside Ghoul Priest cannot be spawned again.".into(),
        ));
    }
    game.scenario.reveal(PARLOR);
    game.scenario.lita_in_play = true;
    game.scenario.priest_spawned = true;
    // The fixed investigator cards occupy IDs below 300, encounter copies
    // occupy 300..326, and the two set-aside characters use 400 and 401.
    let card = CardView {
        id: 400,
        code: "01116".into(),
        title: content::title("01116")
            .expect("Selected scenario identity")
            .into(),
        weakness: false,
    };
    game.scenario.enemies.push(EnemyState {
        card,
        location: HALLWAY.into(),
        engaged: None,
        exhausted: false,
        damage: 0,
        doom: 0,
        bounties: 0,
        owner: None,
    });
    game.act = 3;
    game.history.push("The Barrier reveals Parlor, puts Lita Chantler there and spawns the Ghoul Priest in Hallway. Resolve engagement and eligible entry reactions before continuing the round-end sequence. [01109b; FAQ §2.2]".into());
    game.capture_presentation("You extinguish the barrier. A furious woman waits in the parlor as the Ghoul Priest enters the hallway.");
    Ok(400)
}

/// 01105b chooses the group discard or two horror to the lead. Hidden cards
/// cannot be discarded at random; the official-response FAQ for Diabolic
/// Voices (05092, 2023-01-13) requires selecting another card instead.
pub(super) fn agenda_one_can_discard(game: &Game) -> bool {
    game.players.iter().any(|player| {
        !player.defeated && !player.resigned && player.hand.iter().any(|card| card.code != "06017")
    })
}

pub(super) fn agenda_one_discard_player(game: &mut Game, who: Investigator) {
    let player_index = who.index();
    let player = &game.players[player_index];
    if player.defeated || player.resigned {
        return;
    }
    let eligible: Vec<usize> = player
        .hand
        .iter()
        .enumerate()
        .filter(|(_, card)| card.code != "06017")
        .map(|(index, _)| index)
        .collect();
    if !eligible.is_empty() {
        let index = eligible[game.random_index(eligible.len())];
        let player = &mut game.players[player_index];
        let card = player.hand.remove(index);
        player.discard.push(card);
    }
}

/// The five encounter-deck enemy identities have two distinct spawn paths. A drawn
/// enemy without a Spawn instruction engages its drawer, irrespective of prey.
/// A named Spawn location that is absent causes discard, not another placement.
pub(super) fn spawn_encounter_enemy(
    game: &mut Game,
    card: CardView,
    drawer: Investigator,
) -> Result<Option<u16>, RuleError> {
    let definition = enemy_definition(&card.code)
        .ok_or_else(|| RuleError("This is not an original Gathering encounter enemy.".into()))?;
    let location = definition
        .spawn
        .unwrap_or(&game.players[drawer.index()].location)
        .to_owned();
    if game.scenario.location(&location).is_none() {
        game.history.push(format!(
            "{} has no legal spawn location and is discarded. [RR p19]",
            card.title
        ));
        game.encounter_discard.push(card);
        return Ok(None);
    }
    let id = card.id;
    game.scenario.enemies.push(EnemyState {
        card,
        location,
        engaged: (definition.spawn.is_none() && !super::ethereal_form::is_ethereal(game, drawer))
            .then_some(drawer),
        exhausted: false,
        damage: 0,
        doom: 0,
        bounties: 0,
        owner: None,
    });
    Ok(Some(id))
}

fn apply_prey(game: &Game, code: &str, candidates: &mut Vec<Investigator>) {
    let prey = enemy_definition(code).map_or(Prey::Any, |definition| definition.prey);
    let score = |investigator: Investigator| {
        let player = &game.players[investigator.index()];
        match prey {
            Prey::HighestCombat => {
                let lita = lita_controller(game).is_some_and(|controller| {
                    game.players[controller.index()].location == player.location
                });
                u16::from(player.base_skills[2]) + u16::from(lita)
            }
            Prey::LowestRemainingHealth => {
                u16::from(u8::MAX - player.health.saturating_sub(player.damage))
            }
            Prey::Any => 0,
        }
    };
    if let Some(best) = candidates.iter().copied().map(score).max() {
        candidates.retain(|&investigator| score(investigator) == best);
    }
}

/// An empty result means no automatic engagement. A tie is a lead-investigator
/// choice, not investigator enumeration order. Exhaustion and Aloof prevent it.
pub(super) fn engagement_candidates(game: &Game, enemy_id: u16) -> Vec<Investigator> {
    let Some(enemy) = game
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == enemy_id)
    else {
        return vec![];
    };
    if enemy.exhausted || enemy.engaged.is_some() || enemy.card.code == "06012" {
        return vec![];
    }
    let mut candidates: Vec<_> = game
        .players
        .iter()
        .filter(|player| {
            !player.defeated
                && !player.resigned
                && player.location == enemy.location
                && !super::ethereal_form::is_ethereal(game, player.investigator)
        })
        .map(|player| player.investigator)
        .collect();
    apply_prey(game, &enemy.card.code, &mut candidates);
    candidates
}

/// Return the next connecting locations for this Hunter step. The actual
/// movement and resulting engagement are resolved by the runtime in sequence.
pub(super) fn hunter_destinations(game: &Game, enemy_id: u16) -> Vec<String> {
    let Some(enemy) = game
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == enemy_id)
    else {
        return vec![];
    };
    let hunter = enemy_definition(&enemy.card.code).is_some_and(|definition| definition.hunter)
        || enemy.card.code == "06017";
    if !hunter || enemy.exhausted || enemy.engaged.is_some() {
        return vec![];
    }
    let distances: Vec<_> = game
        .players
        .iter()
        .filter(|player| !player.defeated && !player.resigned)
        .filter_map(|player| {
            game.scenario
                .distance(&enemy.location, &player.location)
                .map(|distance| (player.investigator, distance))
        })
        .collect();
    let Some(nearest) = distances.iter().map(|(_, distance)| *distance).min() else {
        return vec![];
    };
    if nearest == 0 {
        return vec![];
    }
    let mut candidates: Vec<_> = distances
        .into_iter()
        .filter(|(_, distance)| *distance == nearest)
        .map(|(investigator, _)| investigator)
        .collect();
    apply_prey(game, &enemy.card.code, &mut candidates);
    let mut destinations = Vec::new();
    for investigator in candidates {
        let destination = if nearest == 1 {
            game.players[investigator.index()].location.clone()
        } else {
            HALLWAY.into()
        };
        if !destinations.contains(&destination) {
            destinations.push(destination);
        }
    }
    destinations
}

/// The caller resolves each listed Hunter once, including lead choices among
/// tied destinations. No movement is implied by the order of these IDs.
pub(super) fn ready_hunters(game: &Game) -> Vec<u16> {
    game.scenario
        .enemies
        .iter()
        .filter(|enemy| !hunter_destinations(game, enemy.card.id).is_empty())
        .map(|enemy| enemy.card.id)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum AttackTiming {
    EnemyPhase,
    Opportunity,
    Retaliate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct EnemyAttack {
    pub enemy: u16,
    pub source_code: String,
    pub investigator: Investigator,
    pub damage: u8,
    pub horror: u8,
    pub timing: AttackTiming,
}

/// Original RR p25 uses seated player order, starting with the current lead,
/// independently of the chosen investigation-turn order.
pub(super) fn player_order(game: &Game) -> Vec<Investigator> {
    (0..3)
        .map(|offset| Investigator::ALL[(game.lead.index() + offset) % 3])
        .filter(|investigator| {
            let player = &game.players[investigator.index()];
            !player.defeated && !player.resigned
        })
        .collect()
}

fn attack_values(code: &str) -> Option<(u8, u8)> {
    enemy_definition(code)
        .map(|definition| (definition.damage, definition.horror))
        .or_else(|| {
            cards::definition(code)?
                .enemy
                .map(|enemy| (enemy.attack_damage, enemy.attack_horror))
        })
}

fn attack(
    game: &Game,
    enemy: &EnemyState,
    investigator: Investigator,
    timing: AttackTiming,
) -> Option<EnemyAttack> {
    let player = &game.players[investigator.index()];
    if enemy.exhausted || player.defeated || player.resigned {
        return None;
    }
    let (damage, horror) = attack_values(&enemy.card.code)?;
    Some(EnemyAttack {
        enemy: enemy.card.id,
        source_code: enemy.card.code.clone(),
        investigator,
        damage,
        horror,
        timing,
    })
}

/// Each investigator resolves these in their chosen order. Re-evaluate an
/// attack immediately before resolving it: a prior attack reaction may remove
/// an enemy or eliminate its target. Assign damage/horror simultaneously.
pub(super) fn enemy_phase_attacks(game: &Game, investigator: Investigator) -> Vec<EnemyAttack> {
    game.scenario
        .enemies
        .iter()
        .filter(|enemy| enemy.engaged == Some(investigator))
        .filter_map(|enemy| attack(game, enemy, investigator, AttackTiming::EnemyPhase))
        .collect()
}

/// Check the action's designator before calling opportunity_attacks. Ordinary
/// draw/resource/move/play/engage actions and undesignated action abilities
/// provoke; fight/evade/parley/resign do not. Free abilities are not actions.
pub(super) fn provokes_opportunity(action_designator: &str) -> bool {
    !matches!(action_designator, "fight" | "evade" | "parley" | "resign")
}

/// RR p5: after all action costs, before its effect, once per enemy even for a
/// two-action ability. The acting investigator chooses the attacks' order.
pub(super) fn opportunity_attacks(game: &Game, investigator: Investigator) -> Vec<EnemyAttack> {
    game.scenario
        .enemies
        .iter()
        .filter(|enemy| enemy.engaged == Some(investigator))
        .filter_map(|enemy| attack(game, enemy, investigator, AttackTiming::Opportunity))
        .collect()
}

/// RR p18: after all failed-attack test results, a ready Retaliate enemy attacks
/// the attacker, even when engaged with somebody else. It does not exhaust.
pub(super) fn retaliate_attack(
    game: &Game,
    enemy_id: u16,
    attacker: Investigator,
) -> Option<EnemyAttack> {
    let enemy = game
        .scenario
        .enemies
        .iter()
        .find(|enemy| enemy.card.id == enemy_id)?;
    if !enemy_definition(&enemy.card.code).is_some_and(|definition| definition.retaliate) {
        return None;
    }
    attack(game, enemy, attacker, AttackTiming::Retaliate)
}

/// Call only after the complete attack and abilities triggered by that attack.
/// Opportunity and Retaliate attacks never exhaust their source (RR pp5,18,25).
pub(super) fn complete_enemy_attack(game: &mut Game, attack: &EnemyAttack) {
    if attack.timing == AttackTiming::EnemyPhase
        && let Some(enemy) = game
            .scenario
            .enemies
            .iter_mut()
            .find(|enemy| enemy.card.id == attack.enemy)
    {
        enemy.exhausted = true;
    }
}

/// Agenda3's end-of-enemy-phase forced movement, after ordinary attacks.
/// Returned IDs require a normal engagement check, not an enemy-entry reaction
/// or another attack. Exhausted moved Ghouls remain exhausted and unengaged.
/// Includes agenda movement even when the moving ghoul is exhausted.
pub(super) fn enemy_phase_has_activity(game: &Game) -> bool {
    if !ready_hunters(game).is_empty()
        || player_order(game)
            .into_iter()
            .any(|owner| !enemy_phase_attacks(game, owner).is_empty())
    {
        return true;
    }
    if game.agenda == 3 {
        let mut after = game.scenario.clone();
        after.move_unengaged_ghouls_toward_parlor();
        if after.enemies != game.scenario.enemies {
            return true;
        }
    }
    false
}

pub(super) fn finish_enemy_phase(game: &mut Game) -> Vec<u16> {
    if game.agenda != 3 {
        return vec![];
    }
    let before: Vec<_> = game
        .scenario
        .enemies
        .iter()
        .map(|enemy| (enemy.card.id, enemy.location.clone()))
        .collect();
    game.scenario.move_unengaged_ghouls_toward_parlor();
    game.scenario
        .enemies
        .iter()
        .filter(|enemy| {
            !enemy.exhausted
                && before
                    .iter()
                    .any(|(id, location)| *id == enemy.card.id && *location != enemy.location)
        })
        .map(|enemy| enemy.card.id)
        .collect()
}

/// Part of upkeep4.3's simultaneous readying. Ready assets in the same framework
/// operation; then resolve these normal engagement checks before upkeep draws.
pub(super) fn ready_enemies_for_upkeep(game: &mut Game) -> Vec<u16> {
    for enemy in &mut game.scenario.enemies {
        enemy.exhausted = false;
    }
    game.scenario
        .enemies
        .iter()
        .filter(|enemy| !engagement_candidates(game, enemy.card.id).is_empty())
        .map(|enemy| enemy.card.id)
        .collect()
}

pub(super) struct DefeatedEnemy {
    pub enemy: EnemyState,
    /// 01110a's If condition interrupts before after-defeat rewards or any
    /// remaining attack/test/phase continuation. Resolve the ending choice now.
    pub ends_scenario: bool,
}

/// The caller establishes defeat from resolved damage or an explicit effect.
/// Moving the defeated card to victory/discard happens before Act 3's terminal
/// interrupt. The captured state preserves bounty information for nonterminal
/// rewards without looking up a card that has already left play.
pub(super) fn remove_defeated_enemy(
    game: &mut Game,
    enemy_id: u16,
) -> Result<DefeatedEnemy, RuleError> {
    let index = game
        .scenario
        .enemies
        .iter()
        .position(|enemy| enemy.card.id == enemy_id)
        .ok_or_else(|| RuleError("The defeated enemy is no longer in play.".into()))?;
    let enemy = game.scenario.enemies.remove(index);
    let ends_scenario = game.act == 3 && enemy.card.code == "01116";
    if enemy_definition(&enemy.card.code).is_some_and(|definition| definition.victory > 0) {
        game.scenario.victory.push(enemy.card.clone());
    } else if let Some(owner) = enemy.owner {
        game.players[owner.index()].discard.push(enemy.card.clone());
    } else {
        game.encounter_discard.push(enemy.card.clone());
    }
    game.capture_presentation(if ends_scenario {
        "The Priest falls and the swarm retreats. Lita demands that the house be burned.".into()
    } else {
        format!("{} is defeated and leaves play.", enemy.card.title)
    });
    Ok(DefeatedEnemy {
        enemy,
        ends_scenario,
    })
}

pub(super) fn enemy_health(code: &str) -> Option<u8> {
    enemy_definition(code)
        .map(|definition| definition.health)
        .or_else(|| {
            let definition = cards::definition(code)?;
            let enemy = definition.enemy?;
            definition.health.map(|health| {
                if enemy.health_per_investigator {
                    health * 3
                } else {
                    health
                }
            })
        })
}

/// Move the resolved persistent treachery to its actual destination. The
/// runtime removes the pending revelation frame without discarding it again.
pub(super) fn place_persistent_treachery(
    game: &mut Game,
    card: CardView,
    drawer: Investigator,
) -> Result<(), RuleError> {
    match card.code.as_str() {
        "01164" | "01165" => game.players[drawer.index()].threats.push(card),
        "01168" => {
            let code = game.players[drawer.index()].location.clone();
            let location = game
                .scenario
                .location_mut(&code)
                .ok_or_else(|| RuleError("The investigator's location is not in play.".into()))?;
            if location.fog.is_none() {
                location.fog = Some(card);
            } else {
                // The limit prevents attaching a second copy. It does not
                // replace the existing attachment or create another +2 shroud.
                game.encounter_discard.push(card);
            }
        }
        _ => {
            return Err(RuleError(
                "This treachery does not use the persistent Gathering placement.".into(),
            ));
        }
    }
    Ok(())
}

/// Forced after each entry, including repeat entries into an already revealed
/// room. These are ordinary damage/horror, so the runtime offers prevention
/// and legal asset allocation before checking defeat.
pub(super) fn location_entry_harm(location: &str) -> (u8, u8) {
    match location {
        ATTIC => (0, 1),
        CELLAR => (1, 0),
        _ => (0, 0),
    }
}

/// 01168 triggers on a successful investigation even when the investigator
/// discovers no clue. The runtime calls this in the after-success sequence.
pub(super) fn discard_investigated_fog(game: &mut Game, location: &str) {
    if let Some(location) = game.scenario.location_mut(location)
        && let Some(fog) = location.fog.take()
    {
        game.encounter_discard.push(fog);
    }
}

pub(super) fn lita_controller(game: &Game) -> Option<Investigator> {
    game.players
        .iter()
        .find(|player| {
            !player.defeated
                && !player.resigned
                && player.assets.iter().any(|asset| asset.card.code == "01117")
        })
        .map(|player| player.investigator)
}

pub(super) fn lita_definition() -> &'static cards::CardDefinition {
    static LITA: cards::CardDefinition = cards::CardDefinition {
        code: "01117",
        title: "Lita Chantler",
        kind: cards::CardKind::Asset,
        factions: &[cards::Faction::Neutral],
        cost: Some(0),
        level: None,
        icons: [0; 5],
        base_skills: None,
        traits: &["Ally"],
        slots: &[cards::Slot::Ally],
        unique: true,
        weakness: false,
        permanent: false,
        uses: None,
        health: Some(3),
        sanity: Some(3),
        enemy: None,
        peril: false,
        effective_text: "While you control Lita Chantler, she gains:\n\"Each investigator at your location gets +1 [combat].\n[reaction] When an investigator at your location successfully attacks a [[Monster]] enemy: That investigator deals +1 damage.\"",
    };
    &LITA
}

/// Official-response Lita FAQ (01117, 2017-02-04): during The Gathering,
/// temporarily controlled Lita is removed from the game when she leaves play,
/// not put in either discard pile. This does not change an ending's award.
pub(super) fn remove_lita(game: &mut Game, card: CardView) {
    assert_eq!(
        card.code, "01117",
        "Only the temporary Gathering story asset"
    );
    game.scenario.lita_in_play = false;
    game.scenario.removed.push(card);
}

/// Successful Parley takes control of the actual story asset. The runtime must
/// enforce the ally slot immediately through the same replacement decision as
/// other asset acquisition; it must not leave an over-limit asset state settled.
pub(super) fn take_control_of_lita(
    game: &mut Game,
    investigator: Investigator,
) -> Result<(), RuleError> {
    let player = &game.players[investigator.index()];
    if !game.scenario.lita_in_play
        || lita_controller(game).is_some()
        || player.location != PARLOR
        || player.defeated
        || player.resigned
    {
        return Err(RuleError(
            "Lita cannot be taken under this investigator's control.".into(),
        ));
    }
    game.players[investigator.index()]
        .assets
        .push(super::runtime::AssetState {
            card: CardView {
                id: 401,
                code: "01117".into(),
                title: "Lita Chantler".into(),
                weakness: false,
            },
            ready: true,
            uses: 0,
            damage: 0,
            horror: 0,
        });
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SymbolTiming {
    ApplySymbols,
    FailedResults,
}

pub(super) struct SymbolHarm {
    pub damage: u8,
    pub horror: u8,
}

/// Original Standard reference 01104; RR p26 distinguishes ST4 symbol effects
/// from ST7 failed-test results. Supply only tokens actually resolved, excluding
/// canceled and ignored tokens. Separate entries preserve separate effects and
/// their prevention opportunities; do not merge two Tablet/Cultist resolutions.
pub(super) fn symbol_consequences(
    game: &Game,
    tester: Investigator,
    resolved: &[TokenFace],
    timing: SymbolTiming,
) -> Vec<SymbolHarm> {
    let ghoul = game
        .scenario
        .ghouls_at(&game.players[tester.index()].location)
        > 0;
    resolved
        .iter()
        .filter_map(|face| match (timing, face) {
            (SymbolTiming::ApplySymbols, TokenFace::Tablet) if ghoul => Some(SymbolHarm {
                damage: 1,
                horror: 0,
            }),
            (SymbolTiming::FailedResults, TokenFace::Cultist) => Some(SymbolHarm {
                damage: 0,
                horror: 1,
            }),
            _ => None,
        })
        .collect()
}

pub(super) fn resolve_ancient_evils(
    game: &mut Game,
) -> Result<Option<ScenarioConsequence>, RuleError> {
    game.agenda_doom = game
        .agenda_doom
        .checked_add(1)
        .ok_or_else(|| RuleError("Agenda doom exceeds the supported counter range.".into()))?;
    game.history.push(
        "Ancient Evils places one doom and permits an immediate agenda check. [01166; RR p9]"
            .into(),
    );
    game.capture_presentation("Ancient Evils adds one doom. Check whether the agenda advances.");
    check_agenda(game)
}

/// Call at mythos 1.3, or when an effect explicitly permits advancement. Ordinary
/// doom placement (including Agenda 3's round-end effect) must not call this.
pub(super) fn check_agenda(game: &mut Game) -> Result<Option<ScenarioConsequence>, RuleError> {
    let threshold = match game.agenda {
        1 => 3,
        2 => 7,
        3 => 10,
        _ => {
            return Err(RuleError(
                "The Gathering has no such current agenda.".into(),
            ));
        }
    };
    // Within the selected content, non-agenda doom can be on Tony's Quarry.
    // All enemy doom is counted rather than filtering only the expected source.
    let total = u16::from(game.agenda_doom)
        + game
            .scenario
            .enemies
            .iter()
            .map(|enemy| u16::from(enemy.doom))
            .sum::<u16>();
    if total < threshold {
        return Ok(None);
    }
    game.agenda_doom = 0;
    for enemy in &mut game.scenario.enemies {
        enemy.doom = 0;
    }
    game.history.push(format!(
        "Agenda {} advances at {total} total doom; remove doom from all cards before resolving its reverse. [RR pp3,9]",
        game.agenda
    ));
    game.capture_presentation(match game.agenda {
        1 => "The house decays around you, its familiar rooms giving way to earth.",
        2 => "A ghoul breaks through the floor, exposing tunnels beneath the house.",
        _ => "Creatures erupt from the ground as the infestation breaks free.",
    });
    Ok(Some(match game.agenda {
        1 => ScenarioConsequence::AgendaOneChoice,
        2 => ScenarioConsequence::AgendaTwoGhoulSearch,
        3 if game.act < 3 => ScenarioConsequence::ResolutionThree,
        3 => ScenarioConsequence::DefeatNonResigned,
        _ => unreachable!(),
    }))
}

/// Agenda 2b's bounded discard-until operation. Its caller must route the found
/// enemy through the ordinary encounter-draw/spawn path for the lead.
///
/// The no-match branch uses the explicitly qualified legacy interpretation in
/// S8: finish after one pass, then rebuild. It never restores victory/set-aside
/// enemies or keeps reshuffling inside the same search.
pub(super) fn agenda_two_ghoul(game: &mut Game) -> Option<CardView> {
    let recycled = game.encounter_discard.len();
    game.encounter_deck.append(&mut game.encounter_discard);
    shuffle_encounter(game);
    game.capture_shuffle(super::DrawSource::Encounter, recycled, vec![]);
    let mut found = None;
    while let Some(card) = game.encounter_deck.pop() {
        if enemy_definition(&card.code).is_some_and(|enemy| enemy.ghoul) {
            found = Some(card);
            break;
        }
        game.encounter_discard.push(card);
    }
    if found.is_none() {
        let recycled = game.encounter_discard.len();
        game.encounter_deck.append(&mut game.encounter_discard);
        shuffle_encounter(game);
        game.capture_shuffle(super::DrawSource::Encounter, recycled, vec![]);
    }
    game.history.push(if found.is_some() {
        "Agenda 2 finds a Ghoul; the lead draws it with its ordinary spawn instructions. [01106b]".into()
    } else {
        "Agenda 2 finds no available Ghoul after discarding the deck once. Finish the search, then rebuild the encounter deck. [01106b; qualified legacy interpretation S8]".into()
    });
    found
}

fn shuffle_encounter(game: &mut Game) {
    super::shuffle(&mut game.random, &mut game.encounter_deck);
}

/// Call after the draw/effect that exhausted the deck has fully resolved, or
/// before another encounter draw. Never rebuild inside an unresolved search.
pub(super) fn rebuild_empty_encounter(game: &mut Game) {
    if game.encounter_deck.is_empty() && !game.encounter_discard.is_empty() {
        let recycled = game.encounter_discard.len();
        game.encounter_deck.append(&mut game.encounter_discard);
        shuffle_encounter(game);
        game.capture_shuffle(super::DrawSource::Encounter, recycled, vec![]);
    }
}

/// Readable original-face rules for the selected scenario identities. This is
/// inspection text, not a parser or a promise that every ability is executable.
/// The caller supplies a visible card identity; this never examines deck order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InspectionFace {
    Front,
    RevealedLocation,
    StoryReverse,
}

pub(super) fn inspect(code: &str, face: InspectionFace) -> Option<String> {
    let title = content::title(code)?;
    if face == InspectionFace::StoryReverse {
        let details = match code {
            "01105" => {
                "Agenda 1\nReverse: The lead investigator must decide (choose one): Either each investigator discards 1 card at random from his or her hand, or the lead investigator takes 2 horror."
            }
            "01106" => {
                "Agenda 2\nReverse: Shuffle the encounter discard pile into the encounter deck. Discard cards from the top of the encounter deck until a Ghoul enemy is discarded. The lead investigator draws that enemy."
            }
            "01107" => {
                "Agenda 3\nReverse:\nIf the investigators are at Act 1 or 2, they are trapped inside the house as the ghouls tear them apart. (Resolution 3)\nIf the investigators are at Act 3, they barely escape with their lives, allowing the ghouls to run rampant. Each investigator that has not resigned is defeated and suffers 1 physical trauma."
            }
            "01108" => {
                "Act 1\nReverse:\nPut into play the set-aside Hallway, Cellar, Attic, and Parlor.\nDiscard each enemy in the Study.\nPlace each investigator in the Hallway.\nRemove the Study from the game."
            }
            "01109" => {
                "Act 2\nReverse:\nThe barrier blocking passage into the parlor has vanished. Reveal the Parlor.\nPut the set-aside Lita Chantler into play in the Parlor.\nSpawn the set-aside Ghoul Priest in the Hallway."
            }
            "01110" => {
                "Act 3\nReverse: The lead investigator must decide (choose one):\nIt was never much of a home. Burn it down! (Resolution 1)\nThis hell-pit is my home! No way are we burning it! (Resolution 2)"
            }
            _ => return None,
        };
        return Some(format!("{title} · {code}\n{details}"));
    }
    let details = match code {
        "01104" => {
            "Scenario reference — Easy / Standard face\n[Skull] −X. X is the number of Ghoul enemies at your location.\n[Cultist] −1. If you fail, take 1 horror.\n[Tablet] −2. If there is a Ghoul enemy at your location, take 1 damage."
        }
        "01105" => "Agenda 1 · Doom threshold 3\nFront: No additional rules text.",
        "01106" => "Agenda 2 · Doom threshold 7\nFront: No additional rules text.",
        "01107" => {
            "Agenda 3 · Doom threshold 10\nFront:\nForced — At the end of the enemy phase: Each unengaged Ghoul enemy moves 1 location towards the Parlor.\nForced — At the end of the round: Place 1 doom on this agenda for each Ghoul enemy in the Hallway or Parlor."
        }
        "01108" => "Act 1 · Clue threshold 2 per investigator\nFront: No additional rules text.",
        "01109" => {
            "Act 2 · Clue threshold 3 per investigator\nFront:\nObjective — When the round ends, investigators in the Hallway may, as a group, spend the requisite number of clues to advance."
        }
        "01110" => "Act 3\nFront: Objective — If the Ghoul Priest is defeated, advance.",
        STUDY => {
            "Location\nUnrevealed: No rules text.\nRevealed: Shroud 2 · Clues 2 per investigator. No additional rules text.\nNo connecting-location symbols."
        }
        HALLWAY => {
            "Location\nUnrevealed: No rules text.\nRevealed: Shroud 1 · Clues 0. No additional rules text.\nConnections: Attic, Cellar, Parlor."
        }
        ATTIC => {
            "Location\nUnrevealed: No rules text.\nRevealed: Shroud 1 · Clues 2 per investigator · Victory 1\nForced — After you enter the Attic: Take 1 horror.\nConnection: Hallway."
        }
        CELLAR => {
            "Location\nUnrevealed: No rules text.\nRevealed: Shroud 4 · Clues 2 per investigator · Victory 1\nForced — After you enter the Cellar: Take 1 damage.\nConnection: Hallway."
        }
        PARLOR => {
            "Location\nUnrevealed: The entrance to the Parlor is blocked by a darkly glowing unfathomable barrier. You cannot move into the Parlor.\nRevealed: Shroud 2 · Clues 0\n[Action] Resign. \"This is too much for me!\" You run out the front door, fleeing in panic.\nWhile Lita Chantler is not controlled by a player, she gains: \"[Action]: Parley. Test [Intellect] (4). If you succeed, take control of Lita Chantler.\"\nConnection: Hallway."
        }
        "01116" => {
            "Enemy · Humanoid. Monster. Ghoul. Elite.\nFight 4 · Health 5 per investigator · Evade 4\nAttack: 2 damage, 2 horror · Victory 2\nPrey — Highest [Combat].\nHunter. Retaliate."
        }
        "01117" => {
            "Unique · The Zealot\nAsset · Ally. · Cost 0 · Ally slot\nHealth 3 · Sanity 3\nWhile you control Lita Chantler, she gains:\n\"Each investigator at your location gets +1 [Combat].\n[Reaction] When an investigator at your location successfully attacks a Monster enemy: That investigator deals +1 damage.\""
        }
        "01118" => {
            "Enemy · Humanoid. Monster. Ghoul.\nFight 4 · Health 4 · Evade 1\nAttack: 1 damage, 2 horror · Victory 1\nSpawn — Attic."
        }
        "01119" => {
            "Enemy · Humanoid. Monster. Ghoul.\nFight 3 · Health 4 · Evade 4\nAttack: 2 damage, 1 horror · Victory 1\nSpawn — Cellar."
        }
        "01159" => {
            "Enemy · Creature.\nFight 1 · Health 1 · Evade 3\nAttack: 1 damage, 0 horror\nHunter."
        }
        "01160" => {
            "Enemy · Humanoid. Monster. Ghoul.\nFight 2 · Health 2 · Evade 2\nAttack: 1 damage, 1 horror\nNo additional rules text."
        }
        "01161" => {
            "Enemy · Humanoid. Monster. Ghoul.\nFight 3 · Health 3 · Evade 3\nAttack: 1 damage, 1 horror\nPrey — Lowest remaining health."
        }
        "01162" => {
            "Treachery · Hazard.\nRevelation — Test [Agility] (3). For each point you fail by, take 1 damage."
        }
        "01163" => {
            "Treachery · Terror.\nRevelation — Test [Willpower] (3). For each point you fail by, take 1 horror."
        }
        "01164" => {
            "Treachery · Terror.\nRevelation — Put Frozen in Fear into play in your threat area.\nThe first time you perform one of the following actions (move, fight, or evade) each round, it costs 1 additional action.\nForced — At the end of your turn: Test [Willpower] (3). If you succeed, discard Frozen in Fear."
        }
        "01165" => {
            "Treachery · Terror.\nRevelation — Put Dissonant Voices into play in your threat area.\nYou cannot play assets or events.\nForced — At the end of the round: Discard Dissonant Voices."
        }
        "01166" => {
            "Treachery · Omen.\nRevelation — Place 1 doom on the current agenda. This effect can cause the current agenda to advance."
        }
        "01167" => {
            "Treachery · Hazard.\nRevelation — Test [Willpower] (4). If you fail, choose and discard 1 asset you control (if you cannot, take 2 damage instead)."
        }
        "01168" => {
            "Treachery · Hazard.\nRevelation — Attach to your location. Limit 1 per location.\nAttached location gets +2 shroud.\nForced — After attached location is successfully investigated: Discard Obscuring Fog."
        }
        _ => return None,
    };
    let public_details = if matches!(code, STUDY | HALLWAY | ATTIC | CELLAR | PARLOR) {
        if face == InspectionFace::RevealedLocation {
            format!(
                "Location\nRevealed:{}",
                details.split_once("\nRevealed:").expect("Location faces").1
            )
        } else {
            details
                .split("\nRevealed:")
                .next()
                .expect("Location faces")
                .to_owned()
        }
    } else {
        details.to_owned()
    };
    Some(format!("{title} · {code}\n{public_details}"))
}
