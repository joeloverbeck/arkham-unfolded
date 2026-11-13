//! Original Gathering preparation for the fixed #89 party.
//!
//! Ordinary preparation continues into scenario play; old experiment records stay separate.

mod asset_entry;
mod backpack;
pub use backpack::BackpackFixture;
mod cards;
pub use cards::UseKind;
mod combat;
mod content;
mod crypt_chill;
pub use crypt_chill::CryptChillFixture;
mod draws;
mod elimination;
mod ethereal_form;
pub use ethereal_form::EtherealFixture;
mod harm_cards;
pub use elimination::LifecycleFixture;
pub use harm_cards::HarmFixture;
mod encounter_search;
mod group_payment;
pub use draws::DrawFixture;
pub use encounter_search::SearchFixture;
pub use group_payment::{
    Allocation, Contributor, GROUP_CLUE_CONTRIBUTIONS, GroupCluePayment, Ineligibility,
};
pub use player_cards::AssetFixture;
pub mod continuity;
pub mod inspection;
mod live;
mod moments;
mod option_labels;
pub use option_labels::counted;
mod ownership;
mod peter;
pub use ownership::{OptionOwner, OwnedOption};
mod phases;
pub use moments::{DeckStep, DrawBatch, DrawCue, DrawSource, PresentationMoment};
pub use peter::PeterFixture;
mod player_cards;
mod presentation;
pub use presentation::{
    DiscardView, HarmAllocation, HarmView, HeldDrawView, PendingView, TestAction, TestTarget,
};
mod slots;
mod tokens;
pub use scenario::{Resolution, ResolutionRecord};
pub use slots::SlotUsage;
pub use tokens::{RevealGroup, TokenFixture};
mod retry;
mod runtime;
pub use retry::RetryFixture;
mod player_events;
mod scenario;
mod selected_witnesses;
pub use selected_witnesses::SelectedWitness;
mod single_option;
mod spell_events;
pub use player_events::PlayerEventFixture;
mod spells;
pub use spell_events::SpellEventFixture;
mod tony_cards;
mod tony_effects;
pub use tony_effects::TonyEffectFixture;
mod test_boosts;
mod test_cards;
mod test_report;
pub use test_boosts::BoostFixture;
pub use test_cards::{PREDICTION_MARGIN, PendulumPrompt, TestCardFixture};
pub use test_report::{Contribution, TestBreakdown, TokenContribution};

pub use runtime::{
    CommitmentPresentation, HostileFixture, Phase, PlayObservation, PlayerPresentation, Skill,
    TestResult, TestView, TonyBonusPresentation,
};

mod investigator;
use crate::{Choice, RuleError, TokenFace};
pub use investigator::Investigator;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};
use unfolded_tabletop::random::SeededRandom;

pub const SOURCE_REVISION: &str = "c35be2c:issue-89-gathering-runtime-v4";
const DECK_PROJECTION: &str = include_str!("../docs/research/issue-1-decks.json");
const DECK_PROJECTION_SHA256: &str =
    "fcd3bb06ed278652e61bdfe43c080982b0c95649a2e4944043757d5326cbe7bb";

/// The generator is shared from unfolded-tabletop. Its state stays here as the
/// plain number a save holds, never player-visible; setup and play both draw
/// through these two functions.
fn random_index(state: &mut u64, upper: usize) -> usize {
    let mut random = SeededRandom::new(*state);
    let index = random.index(upper);
    *state = random.state();
    index
}

fn shuffle<T>(state: &mut u64, items: &mut [T]) {
    let mut random = SeededRandom::new(*state);
    random.shuffle(items);
    *state = random.state();
}

/// Effective selected-card text for readable inspection. Metadata does not imply
/// that the corresponding gameplay ability has been implemented.
pub fn card_rules(code: &str) -> Option<&'static str> {
    cards::definition(code).map(|card| card.effective_text)
}

/// The printed title of a selected player or scenario card.
pub fn card_title(code: &str) -> Option<&'static str> {
    content::title(code)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    ChooseLead,
    Mulligan,
    GameplayUnavailable,
    Playing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardView {
    pub id: u16,
    pub code: String,
    pub title: String,
    pub weakness: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardQuantity {
    pub code: String,
    pub title: String,
    pub quantity: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckSource {
    pub investigator: Investigator,
    pub deck_id: u64,
    pub revision: String,
    pub original_sha256: String,
    pub basic_weakness: String,
    /// Public deck composition, ordered by identity, never by shuffled position.
    pub inventory: Vec<CardQuantity>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparationSources {
    pub rules_revision: String,
    pub deck_projection_sha256: String,
    pub taboo_id: u8,
    pub decks: Vec<DeckSource>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigatorView {
    pub investigator: Investigator,
    pub resources: u8,
    pub physical_trauma: u8,
    pub mental_trauma: u8,
    pub damage: u8,
    pub horror: u8,
    pub hand: Vec<CardView>,
    pub deck_count: usize,
    pub permanents: Vec<CardView>,
    pub bounties: u8,
    pub selected_mulligan: Vec<u16>,
    pub mulligan_complete: bool,
    pub location: Option<String>,
}

impl InvestigatorView {
    /// Printed capacity from the pinned investigator definition, including setup.
    pub fn health(&self) -> u8 {
        self.definition().health.expect("Investigator health")
    }

    pub fn sanity(&self) -> u8 {
        self.definition().sanity.expect("Investigator sanity")
    }

    fn definition(&self) -> &'static cards::CardDefinition {
        let code = match self.investigator {
            Investigator::Tony => "06003",
            Investigator::Patrice => "06005",
            Investigator::Jacqueline => "60401",
        };
        cards::definition(code).expect("Selected investigator definition")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioSetup {
    pub title: String,
    pub difficulty: String,
    pub reference: String,
    pub agenda: Vec<String>,
    pub act: Vec<String>,
    pub locations: Vec<String>,
    pub study_clues: u8,
    pub set_aside: Vec<String>,
    /// Initial membership only. This is not the hidden shuffled encounter order.
    pub encounter_inventory: Vec<CardQuantity>,
    pub encounter_deck_count: usize,
    pub skip_first_mythos: bool,
}

impl ScenarioSetup {
    /// Fixed, public setup membership. No shuffle or investigator decision is
    /// performed here; the session assembles the actual deck after mulligans.
    fn initial() -> Result<Self, RuleError> {
        let inventory = content::ENCOUNTER
            .iter()
            .map(|&(code, count)| (code.to_owned(), count))
            .collect();
        let encounter_inventory = quantities(&inventory)?;
        Ok(Self {
            title: "The Gathering".into(),
            difficulty: "Standard".into(),
            reference: "01104".into(),
            agenda: ["01105", "01106", "01107"].map(String::from).to_vec(),
            act: ["01108", "01109", "01110"].map(String::from).to_vec(),
            locations: vec!["01111".into()],
            study_clues: 6,
            set_aside: ["01112", "01113", "01114", "01115", "01116", "01117"]
                .map(String::from)
                .to_vec(),
            encounter_deck_count: encounter_inventory
                .iter()
                .map(|entry| usize::from(entry.quantity))
                .sum(),
            encounter_inventory,
            skip_first_mythos: true,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub revision: u64,
    pub stage: Stage,
    pub prompt: String,
    pub choices: Vec<Choice>,
    pub history: Vec<String>,
    pub source: PreparationSources,
    pub lead: Option<Investigator>,
    /// Display focus only: whose decision the table names and frames. Who may
    /// answer each option is [`Session::ownership`].
    pub decision_owner: Option<Investigator>,
    pub investigators: Vec<InvestigatorView>,
    pub scenario: Option<ScenarioSetup>,
    pub standard_bag: Vec<TokenFace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_fixture: Option<HostileFixture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play: Option<PlayObservation>,
    /// The group clue payment the listed `act:pay:<contributions>` choice offers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_clue_payment: Option<GroupCluePayment>,
}

#[cfg(feature = "tabletop")]
impl Observation {
    /// Presentation only: keep serialized observations and random consumption
    /// unchanged while showing the known starting arrangement during setup.
    pub(crate) fn scenario_for_display(&self) -> std::borrow::Cow<'_, ScenarioSetup> {
        self.scenario.as_ref().map_or_else(
            || std::borrow::Cow::Owned(ScenarioSetup::initial().expect("fixed Gathering catalog")),
            std::borrow::Cow::Borrowed,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    seed: u64,
    random: u64,
    revision: u64,
    stage: Stage,
    source: PreparationSources,
    lead: Option<Investigator>,
    mulligan_index: usize,
    investigators: Vec<InvestigatorView>,
    decks: Vec<Vec<CardView>>,
    excluded_weaknesses: Vec<Vec<CardView>>,
    encounter_deck: Vec<String>,
    scenario: Option<ScenarioSetup>,
    decisions: Vec<String>,
    history: Vec<String>,
    live: live::LiveState,
    #[serde(skip)]
    transition: live::Transition,
    #[serde(skip)]
    baseline: crate::saved_state::Baseline,
    // Normalized live choices point into the append-only audit. Undo itself
    // never appears here, so restoring a position cannot recursively replay it.
    active_decisions: Vec<usize>,
    undo_floor: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    verification_fixture: Option<HostileFixture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime: Option<runtime::Game>,
}

/// Table tests only: place an investigator's clues and location where a witness
/// needs them. No decision is recorded, so Undo cannot reproduce the placement.
#[cfg(all(test, feature = "tabletop"))]
impl Session {
    pub(crate) fn place_for_test(&mut self, who: Investigator, clues: u8, location: &str) {
        let player = &mut self.runtime.as_mut().expect("Scenario play").players[who.index()];
        player.clues = clues;
        player.location = location.into();
    }
}

#[derive(Deserialize)]
struct DeckProjection {
    decks: Vec<DeckInput>,
}

#[derive(Deserialize)]
struct DeckInput {
    id: u64,
    version: String,
    raw_sha256: String,
    slots: BTreeMap<String, u8>,
}

fn quantities(slots: &BTreeMap<String, u8>) -> Result<Vec<CardQuantity>, RuleError> {
    slots
        .iter()
        .map(|(code, &quantity)| {
            Ok(CardQuantity {
                code: code.clone(),
                title: content::title(code)
                    .ok_or_else(|| RuleError(format!("Unknown setup identity {code}.")))?
                    .into(),
                quantity,
            })
        })
        .collect()
}

fn standard_bag() -> Vec<TokenFace> {
    use TokenFace::*;
    vec![
        PlusOne, Zero, Zero, MinusOne, MinusOne, MinusOne, MinusTwo, MinusTwo, MinusThree,
        MinusFour, Skull, Skull, Cultist, Tablet, AutoFail, ElderSign,
    ]
}

impl Session {
    pub fn new() -> Result<Self, RuleError> {
        let mut seed = [0; 8];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut file| file.read_exact(&mut seed))
            .map_err(|error| RuleError(format!("Could not obtain setup randomness: {error}")))?;
        Self::from_seed(u64::from_le_bytes(seed))
    }

    /// Privileged reproducible test input; never exposed by a player client.
    #[doc(hidden)]
    pub fn with_seed(seed: u64) -> Self {
        Self::from_seed(seed).expect("Pinned Gathering setup inputs")
    }

    fn from_seed(seed: u64) -> Result<Self, RuleError> {
        let digest = format!("{:x}", Sha256::digest(DECK_PROJECTION.as_bytes()));
        if digest != DECK_PROJECTION_SHA256 {
            return Err(RuleError(
                "The pinned deck projection changed; reassess preparation before starting.".into(),
            ));
        }
        let projection: DeckProjection = serde_json::from_str(DECK_PROJECTION)
            .map_err(|e| RuleError(format!("Invalid pinned deck projection: {e}")))?;
        let mut sources = Vec::new();
        let mut investigators = Vec::new();
        let mut decks = Vec::new();
        for (index, mut deck) in projection.decks.into_iter().enumerate() {
            let investigator = Investigator::ALL[index];
            let weakness = ["02038", "05041", "02039"][index];
            if index > 0 {
                if deck.slots.remove("01000") != Some(1) {
                    return Err(RuleError(
                        "The expected unresolved weakness slot changed.".into(),
                    ));
                }
                deck.slots.insert(weakness.into(), 1);
            }
            sources.push(DeckSource {
                investigator,
                deck_id: deck.id,
                revision: deck.version,
                original_sha256: deck.raw_sha256,
                basic_weakness: weakness.into(),
                inventory: quantities(&deck.slots)?,
            });
            let mut cards = Vec::new();
            let mut permanents = Vec::new();
            let mut id = index as u16 * 100;
            for (code, count) in deck.slots {
                for _ in 0..count {
                    let card = CardView {
                        id,
                        title: content::title(&code).expect("Checked identity").into(),
                        weakness: content::weakness(&code),
                        code: code.clone(),
                    };
                    id += 1;
                    if content::permanent(&code) {
                        permanents.push(card);
                    } else {
                        cards.push(card);
                    }
                }
            }
            investigators.push(InvestigatorView {
                investigator,
                resources: 0,
                physical_trauma: 0,
                mental_trauma: if index == 2 { 2 } else { 0 },
                damage: 0,
                horror: if index == 2 { 2 } else { 0 },
                hand: vec![],
                deck_count: cards.len(),
                permanents,
                bounties: if index == 0 { 6 } else { 0 },
                selected_mulligan: vec![],
                mulligan_complete: false,
                location: None,
            });
            decks.push(cards);
        }
        let session = Self {
            seed, random: seed, revision: 0, stage: Stage::ChooseLead,
            source: PreparationSources { rules_revision: SOURCE_REVISION.into(),
                deck_projection_sha256: digest, taboo_id: 10, decks: sources },
            lead: None, mulligan_index: 0, investigators, decks,
            excluded_weaknesses: vec![vec![], vec![], vec![]], encounter_deck: vec![],
            scenario: None, decisions: vec![], live: Default::default(), transition: Default::default(), baseline: Default::default(), active_decisions: vec![], undo_floor: 0, verification_fixture: None, runtime: None,
            history: vec!["Fresh campaign preparation: choose the lead investigator. The retained starting lists use the three owner-selected basic weaknesses; no previous campaign history is imported. Rules and taboo 10 are pinned to the #89 research, not claimed to be the latest available release.".into()],
        };
        Ok(session)
    }

    fn decision_owner(&self) -> Option<Investigator> {
        if let Some(game) = &self.runtime {
            return game.decision_owner();
        }
        (self.stage == Stage::Mulligan).then(|| {
            Investigator::ALL[(self.lead.expect("Mulligan lead").index() + self.mulligan_index) % 3]
        })
    }

    pub fn observe(&self) -> Observation {
        let mut choices = Vec::new();
        let mut group_clue_payment = None;
        let prompt = match self.stage {
            Stage::ChooseLead => {
                for investigator in Investigator::ALL {
                    choices.push(Choice {
                        id: format!("lead:{}", investigator.id()),
                        label: format!("Choose {} as lead investigator", investigator.name()),
                        cancel_faces: vec![],
                    });
                }
                "Choose the lead investigator. Player order is Tony, Patrice, Jacqueline, starting with the lead."
            }
            Stage::Mulligan => {
                let investigator =
                    &self.investigators[self.decision_owner().expect("Mulligan owner").index()];
                for card in &investigator.hand {
                    let verb = if investigator.selected_mulligan.contains(&card.id) {
                        "Keep"
                    } else {
                        "Replace"
                    };
                    choices.push(Choice {
                        id: format!("mulligan:{}", card.id),
                        label: format!("{verb} {}", card.title),
                        cancel_faces: vec![],
                    });
                }
                choices.push(Choice {
                    id: "confirm-mulligan".into(),
                    label: if investigator.selected_mulligan.is_empty() {
                        "Keep this opening hand".into()
                    } else {
                        format!(
                            "Replace {} selected cards",
                            investigator.selected_mulligan.len()
                        )
                    },
                    cancel_faces: vec![],
                });
                "Choose any opening-hand cards to replace, then confirm the one allowed mulligan. Replacements reveal new information and cannot be undone."
            }
            Stage::GameplayUnavailable => {
                "Setup is assembled. Gameplay is unavailable: selected rules coverage and native play verification remain incomplete. This is not a playable Gathering session or a scenario ending."
            }
            Stage::Playing => {
                let game = self.runtime.as_ref().expect("Runtime stage");
                choices = game.choices();
                group_clue_payment = group_payment::offered(game, &choices);
                if self.verification_fixture.is_none() && game.scenario.outcome.is_some() {
                    "The Gathering has ended. Review the recorded resolution and campaign consequences."
                } else if self.verification_fixture.is_none() && choices.is_empty() {
                    "No legal choice is available at this pending Gathering decision. Keep this save for review."
                } else if self.verification_fixture.is_none() {
                    "Choose the next Gathering decision. Changes save before their consequences are presented."
                } else if game.scenario.outcome.is_some() {
                    "Privileged verification reached the recorded Gathering resolution. Its consequences are shown in the resolution record."
                } else if choices.is_empty() {
                    "This privileged verification reached its boundary. This is not a scenario resolution."
                } else {
                    "Privileged rules verification: choose among the offered opportunities."
                }
            }
        };
        if self.active_decisions.len() > self.undo_floor {
            choices.push(Choice {
                id: "undo".into(),
                label: "Undo the last choice; no hidden information or random outcome crossed"
                    .into(),
                cancel_faces: vec![],
            });
        }
        let mut investigators = self.investigators.clone();
        if let Some(game) = &self.runtime {
            for player in &game.players {
                let view = &mut investigators[player.investigator.index()];
                view.resources = player.resources;
                view.damage = player.damage;
                view.horror = player.horror;
                view.physical_trauma = player.physical_trauma;
                view.mental_trauma = player.mental_trauma;
                view.hand = player.hand.clone();
                view.deck_count = player.deck.len();
                view.location = Some(player.location.clone());
            }
        }
        Observation {
            revision: self.revision,
            stage: self.stage,
            prompt: prompt.into(),
            choices,
            history: self.history.clone(),
            source: self.source.clone(),
            lead: self.runtime.as_ref().map(|game| game.lead).or(self.lead),
            decision_owner: self.decision_owner(),
            investigators,
            scenario: self.scenario.clone(),
            standard_bag: standard_bag(),
            verification_fixture: self.verification_fixture,
            play: self.runtime.as_ref().map(runtime::Game::observe),
            group_clue_payment,
        }
    }

    /// Who may answer each listed option of the pending decision, in listed
    /// order. Undo is left out: it reverts the previous decision rather than
    /// answering this one. Derived from the session each time; never saved.
    pub fn ownership(&self) -> Vec<OwnedOption> {
        let choices: Vec<_> = self
            .observe()
            .choices
            .into_iter()
            .filter(|choice| choice.id != "undo")
            .collect();
        let owners = match self.stage {
            Stage::ChooseLead => vec![OptionOwner::Table; choices.len()],
            Stage::Mulligan => {
                vec![
                    OptionOwner::Investigator(self.decision_owner().expect("Mulligan owner"));
                    choices.len()
                ]
            }
            Stage::GameplayUnavailable => vec![],
            Stage::Playing => {
                ownership::owners(self.runtime.as_ref().expect("Runtime stage"), &choices)
            }
        };
        choices
            .into_iter()
            .zip(owners)
            .map(|(choice, owner)| OwnedOption {
                id: choice.id,
                owner,
            })
            .collect()
    }

    /// The owner of an option the pending decision accepts: a listed option,
    /// or a number or allocation named in place of a listed option's
    /// placeholder, which belongs to the owner of that listed option.
    pub fn option_owner(&self, id: &str) -> Option<OptionOwner> {
        let owned = self.ownership();
        if let Some(option) = owned.iter().find(|option| option.id == id) {
            return Some(option.owner.clone());
        }
        self.runtime.as_ref()?.direct_choice(id)?;
        owned
            .into_iter()
            .find(|option| {
                [GROUP_CLUE_CONTRIBUTIONS, PREDICTION_MARGIN]
                    .iter()
                    .filter_map(|placeholder| option.id.strip_suffix(placeholder))
                    .any(|template| id.starts_with(template))
            })
            .map(|option| option.owner)
    }

    /// The revision a choice must name, as [`Self::observe`] reports it.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Only the latest live transition, never historical steps; empty after loading.
    pub fn presentation_after(&self, revision: u64) -> &[PresentationMoment] {
        if revision < self.revision {
            &self.transition.steps
        } else {
            &[]
        }
    }

    pub fn observe_presentation(&self, moment: &PresentationMoment) -> Observation {
        let current = self.observe();
        moments::project(
            self.transition.before.as_ref().unwrap_or(&current),
            &current,
            &self.transition.steps,
            moment,
        )
    }

    #[cfg(feature = "tabletop")]
    pub(crate) fn live_state(&self) -> &live::LiveState {
        &self.live
    }

    #[cfg(feature = "tabletop")]
    pub(crate) fn pending_test_view(&self) -> Option<TestView> {
        let mut game = self.runtime.clone()?;
        if game.test.is_none() {
            game.test = game.frames.iter().rev().find_map(|frame| match frame {
                runtime::Frame::TestCleanup(test) => Some((**test).clone()),
                _ => None,
            });
        }
        game.observe().test
    }

    pub fn choose(&mut self, revision: u64, id: &str) -> Result<(), RuleError> {
        self.choose_with_draws(revision, id).map(|_| ())
    }

    pub fn pendulum_prompts(&self) -> Vec<PendulumPrompt> {
        self.runtime
            .as_ref()
            .map(test_cards::prediction_prompts)
            .unwrap_or_default()
    }

    /// Return visual cues only for this successfully installed decision.
    pub fn choose_with_draws(
        &mut self,
        revision: u64,
        id: &str,
    ) -> Result<Vec<moments::DrawCue>, RuleError> {
        if revision != self.revision {
            return Err(RuleError(
                "This choice is stale. Observe the current decision.".into(),
            ));
        }
        let choice = self
            .observe()
            .choices
            .into_iter()
            .find(|choice| choice.id == id)
            .or_else(|| {
                self.runtime
                    .as_ref()
                    .and_then(|game| game.direct_choice(id))
            })
            .ok_or_else(|| {
                RuleError("That choice is not available at the current decision.".into())
            })?;
        let mut candidate = self.clone();
        let mut setup_draws = Vec::new();
        let mut draws = Vec::new();
        if id == "undo" {
            candidate.restore_previous_position()?;
            if let Some(game) = &mut candidate.runtime {
                game.capture_presentation("Undo restores the previous position. The action history retains the earlier choices.");
            }
            candidate.history.push(choice.label);
        } else {
            candidate.apply(id, &mut setup_draws)?;
            candidate.history.push(choice.label);
            // The steps that follow with one legal outcome belong to this
            // decision: they are neither recorded decisions nor Undo destinations.
            candidate.resolve_single_options()?;
            candidate.active_decisions.push(self.decisions.len());
            if candidate.crossed_undo_boundary(self) {
                candidate.undo_floor = candidate.active_decisions.len();
            }
        }
        candidate.revision += 1;
        candidate.transition = live::Transition {
            before: Some(self.observe()),
            steps: vec![],
        };
        for batch in setup_draws {
            draws.push(moments::DrawCue {
                revision: candidate.revision,
                sequence: candidate.runtime.as_ref().map(|_| 0),
                batch,
            });
        }
        if self.runtime.is_none()
            && let Some(game) = &mut candidate.runtime
        {
            game.capture_presentation("While investigating disappearances at the lead investigator's home, you hear chanting and movement below. The study's door vanishes.");
        }
        if let Some(game) = &mut candidate.runtime {
            for (sequence, snapshot) in std::mem::take(&mut game.presentation)
                .into_iter()
                .enumerate()
            {
                let sequence = u32::try_from(sequence).map_err(|_| {
                    RuleError("Too many presentation steps in one decision.".into())
                })?;
                if let Some(batch) = snapshot.draw {
                    draws.push(moments::DrawCue {
                        revision: candidate.revision,
                        sequence: Some(sequence),
                        batch,
                    });
                }
                candidate.transition.steps.push(PresentationMoment {
                    revision: candidate.revision,
                    sequence,
                    caption: snapshot.caption,
                    play: snapshot.play,
                    revealed_story: vec![],
                });
            }
        }
        candidate.decisions.push(id.into());
        let after = candidate.observe();
        candidate
            .live
            .capture(&self.observe(), &mut candidate.transition.steps, &after);
        *self = candidate;
        Ok(draws)
    }

    fn crossed_undo_boundary(&self, prior: &Self) -> bool {
        self.random != prior.random
            || self.decks != prior.decks
            || self.encounter_deck != prior.encounter_deck
            || match (&self.runtime, &prior.runtime) {
                (Some(current), Some(previous)) => current.undo_barrier_since(previous),
                (None, None) => false,
                _ => true,
            }
    }

    fn restore_previous_position(&mut self) -> Result<(), RuleError> {
        if self.active_decisions.len() <= self.undo_floor {
            return Err(RuleError(
                "Undo cannot cross revealed information or a random outcome.".into(),
            ));
        }
        let mut active = self.active_decisions.clone();
        active.pop().expect("A reversible decision");
        let mut restored = Self::from_origin(self.seed, self.verification_fixture)?;
        for &index in &active {
            let decision = self
                .decisions
                .get(index)
                .filter(|decision| decision.as_str() != "undo")
                .ok_or_else(|| RuleError("Invalid active decision in the Undo record.".into()))?;
            restored.choose(restored.revision, decision)?;
        }
        // The rules position rewinds; the chronological evidence never does.
        restored.revision = self.revision;
        restored.decisions = self.decisions.clone();
        restored.history = self.history.clone();
        restored.baseline = self.baseline.clone();
        restored.live = self.live.clone();
        restored.active_decisions = active;
        restored.undo_floor = self.undo_floor;
        *self = restored;
        Ok(())
    }

    /// Perform every following step that has exactly one legal outcome, so the
    /// next observation offers a genuine decision or a pause.
    fn resolve_single_options(&mut self) -> Result<(), RuleError> {
        const LIMIT: usize = 10_000;
        for _ in 0..LIMIT {
            let Some(game) = &self.runtime else {
                return Ok(());
            };
            let choices = game.choices();
            let Some(only) = single_option::forced(&choices) else {
                return Ok(());
            };
            let (id, entry) = (only.id.clone(), single_option::history_entry(&only.label));
            if let Some(game) = &mut self.runtime {
                game.capture_before_automatic_step(&entry);
            }
            self.apply(&id, &mut Vec::new())?;
            self.history.push(entry);
        }
        Err(RuleError(
            "Automatic steps did not reach a decision; keep this save for review.".into(),
        ))
    }

    fn apply(&mut self, id: &str, draws: &mut Vec<moments::DrawBatch>) -> Result<(), RuleError> {
        if let Some(game) = &mut self.runtime {
            game.choose(id)?;
            self.history.append(&mut game.history);
            return Ok(());
        }
        if let Some(name) = id.strip_prefix("lead:") {
            self.lead = Investigator::ALL.into_iter().find(|i| i.id() == name);
            for offset in 0..3 {
                let index = (self.lead.expect("Offered lead").index() + offset) % 3;
                self.shuffle_deck(index);
                let count = self.decks[index].len();
                self.investigators[index].resources = 5;
                let (cards, mut steps) = self.draw_opening(index, 5)?;
                steps.insert(0, DeckStep::Shuffle { count });
                draws.push(moments::DrawBatch {
                    source: moments::DrawSource::Investigator(
                        self.investigators[index].investigator,
                    ),
                    cards,
                    returned: vec![],
                    steps,
                });
            }
            self.stage = Stage::Mulligan;
            self.history.push("Draw five opening cards each. Set aside and replace weaknesses without resolving revelation; they stay outside each deck until all opening hands and mulligans are complete. [RR p27, setup step 8]".into());
        } else if let Some(id) = id.strip_prefix("mulligan:") {
            let id: u16 = id.parse().expect("Offered card ID");
            let index = self.decision_owner().expect("Offered mulligan").index();
            let selected = &mut self.investigators[index].selected_mulligan;
            if let Some(position) = selected.iter().position(|&existing| existing == id) {
                selected.remove(position);
            } else {
                selected.push(id);
                selected.sort_unstable();
            }
        } else {
            let index = self.decision_owner().expect("Offered confirmation").index();
            let selected = std::mem::take(&mut self.investigators[index].selected_mulligan);
            let hand = std::mem::take(&mut self.investigators[index].hand);
            let (removed, kept): (Vec<_>, Vec<_>) = hand
                .into_iter()
                .partition(|card| selected.contains(&card.id));
            self.investigators[index].hand = kept;
            let (cards, steps) = self.draw_opening(index, removed.len())?;
            if !cards.is_empty() {
                draws.push(moments::DrawBatch {
                    source: moments::DrawSource::Investigator(
                        self.investigators[index].investigator,
                    ),
                    cards,
                    returned: removed.clone(),
                    steps,
                });
            }
            // Mulliganed cards stay out until replacement draws have finished.
            if !removed.is_empty() {
                self.decks[index].extend(removed);
                self.shuffle_deck(index);
                let steps = &mut draws.last_mut().expect("Mulligan replacement").steps;
                steps.push(DeckStep::Return);
                steps.push(DeckStep::Shuffle {
                    count: self.decks[index].len(),
                });
            }
            self.investigators[index].deck_count = self.decks[index].len();
            self.investigators[index].mulligan_complete = true;
            self.mulligan_index += 1;
            if self.mulligan_index == 3 {
                for owner in 0..3 {
                    let excluded = std::mem::take(&mut self.excluded_weaknesses[owner]);
                    if !excluded.is_empty() {
                        self.decks[owner].extend(excluded);
                        self.shuffle_deck(owner);
                        draws.push(DrawBatch {
                            source: DrawSource::Investigator(
                                self.investigators[owner].investigator,
                            ),
                            cards: vec![],
                            returned: vec![],
                            steps: vec![DeckStep::Shuffle {
                                count: self.decks[owner].len(),
                            }],
                        });
                    }
                    self.investigators[owner].deck_count = self.decks[owner].len();
                }
                self.assemble_scenario()?;
                draws.push(DrawBatch {
                    source: DrawSource::Encounter,
                    cards: vec![],
                    returned: vec![],
                    steps: vec![DeckStep::Shuffle {
                        count: self.encounter_deck.len(),
                    }],
                });
                self.history.push("Opening hands and mulligans are complete; set-aside weaknesses return to their owners' shuffled decks without revelation. The original Gathering components are assembled: Study with six clues; 26 encounter cards; 14 cards in their other setup destinations. [RR p27; CG p2]".into());
                let mut game = runtime::Game::after_setup(
                    self.lead.expect("Setup lead"),
                    self.random,
                    &self.investigators,
                    &self.decks,
                    &self.encounter_deck,
                );
                self.history.append(&mut game.history);
                self.runtime = Some(game);
                self.stage = Stage::Playing;
            }
        }
        Ok(())
    }

    fn draw_opening(
        &mut self,
        index: usize,
        count: usize,
    ) -> Result<(Vec<CardView>, Vec<DeckStep>), RuleError> {
        let mut drawn = Vec::with_capacity(count);
        let mut steps = Vec::with_capacity(count);
        for _ in 0..count {
            loop {
                let card = self.decks[index].pop().ok_or_else(|| {
                    RuleError("The opening hand cannot be completed from the pinned deck.".into())
                })?;
                if card.weakness {
                    self.excluded_weaknesses[index].push(card);
                } else {
                    steps.push(DeckStep::Draw {
                        card: drawn.len(),
                        remaining: self.decks[index].len(),
                    });
                    drawn.push(card.clone());
                    self.investigators[index].hand.push(card);
                    break;
                }
            }
        }
        self.investigators[index].deck_count = self.decks[index].len();
        Ok((drawn, steps))
    }

    fn shuffle_deck(&mut self, index: usize) {
        shuffle(&mut self.random, &mut self.decks[index]);
    }

    fn assemble_scenario(&mut self) -> Result<(), RuleError> {
        let scenario = ScenarioSetup::initial()?;
        for entry in &scenario.encounter_inventory {
            self.encounter_deck.extend(std::iter::repeat_n(
                entry.code.clone(),
                usize::from(entry.quantity),
            ));
        }
        shuffle(&mut self.random, &mut self.encounter_deck);
        for investigator in &mut self.investigators {
            investigator.location = Some("01111".into());
        }
        self.scenario = Some(scenario);
        Ok(())
    }

    pub fn same_run(&self, other: &Self) -> bool {
        self.seed == other.seed
            && self.source == other.source
            && self.verification_fixture == other.verification_fixture
    }

    pub fn is_fresh(&self) -> bool {
        self.revision == 0 && self.decisions.is_empty()
    }

    /// Admission is exact replay: every recorded decision must still be offered
    /// by this build and reproduce the recorded state. No earlier-build
    /// identifier or meaning is reinterpreted.
    pub fn validate(&self) -> Result<(), RuleError> {
        let mut reconstructed = Self::from_origin(self.seed, self.verification_fixture)?;
        for decision in &self.decisions {
            reconstructed.choose(reconstructed.revision, decision)?;
        }
        if &reconstructed != self {
            return Err(RuleError(
                "Gathering setup does not match its recorded decisions and pinned inputs.".into(),
            ));
        }
        Ok(())
    }
}

impl Session {
    /// Private verification setup, unavailable from ordinary player preparation.
    #[doc(hidden)]
    pub fn with_hostile_fixture(fixture: HostileFixture) -> Self {
        Self::from_origin(89, Some(fixture)).expect("Bound hostile fixture")
    }

    pub fn is_verification_fixture(&self) -> bool {
        self.verification_fixture.is_some()
    }

    fn from_origin(seed: u64, fixture: Option<HostileFixture>) -> Result<Self, RuleError> {
        let mut session = Self::from_seed(seed)?;
        if let Some(fixture) = fixture {
            session.verification_fixture = Some(fixture);
            if fixture == HostileFixture::GatheringFromSetup {
                session.history.push("Privileged verification continues from these actual opening hands through the rules runtime. This record remains excluded from ordinary player clients.".into());
                session.live = Default::default();
                return Ok(session);
            }
            session.stage = Stage::Playing;
            session.lead = Some(Investigator::Patrice);
            session.history = vec!["Privileged source-derived runtime verification; this is not an ordinary fresh campaign.".into()];
            let mut game =
                runtime::Game::fixture(fixture, seed, &session.investigators, &session.decks);
            // A privileged witness poses its named step as constructed, even a
            // single-option one; the steps that follow its first choice resolve.
            game.presentation.clear();
            session.history.append(&mut game.history);
            session.runtime = Some(game);
            let view = session.observe();
            session.live.capture(&view, &mut [], &view);
            session.live.new_story.clear();
        }
        Ok(session)
    }
}

#[cfg(test)]
mod saved_randomness {
    //! Numbers recorded at 4e7a014, while the generator still lived in this
    //! repository. A saved game stores the generator's state, so it continues
    //! only while a given state keeps producing these numbers.
    use super::*;

    /// Wide enough that about half of the generator's values are rejected: the
    /// eleven indexes below took fifteen steps.
    const WIDE: usize = (1 << 63) + 1;
    const RANGES: [usize; 11] = [1, 2, 6, 16, 33, 52, 1000, WIDE, WIDE, WIDE, WIDE];
    const INDEXES: [usize; 11] = [
        0,
        1,
        4,
        8,
        10,
        13,
        244,
        790_593_979_517_193_502,
        3_061_971_736_353_442_415,
        7_495_161_214_544_515_997,
        3_495_176_742_473_237_818,
    ];
    const STATE_AFTER_INDEXES: u64 = 4_990_025_626_462_012_820;

    #[test]
    fn play_generator_repeats_its_recorded_indexes() {
        let mut session = Session::with_hostile_fixture(HostileFixture::GatheringRoundEnd);
        let game = session.runtime.as_mut().expect("Fixture runtime");
        game.random = 89;
        let indexes = RANGES.map(|upper| game.random_index(upper));
        assert_eq!((indexes, game.random), (INDEXES, STATE_AFTER_INDEXES));
    }

    #[test]
    fn setup_generator_repeats_its_recorded_deck_shuffle_from_a_saved_state() {
        let mut session = Session::with_seed(89);
        assert_eq!(session.random, 89, "A fresh generator starts at its seed");
        session.random = STATE_AFTER_INDEXES;
        let dealt: Vec<u16> = session.decks[0].iter().map(|card| card.id).collect();
        session.shuffle_deck(0);
        let order: Vec<usize> = session.decks[0]
            .iter()
            .map(|card| dealt.iter().position(|id| *id == card.id).unwrap())
            .collect();
        assert_eq!(
            order,
            [
                9, 10, 21, 3, 2, 5, 26, 16, 11, 4, 14, 1, 19, 25, 13, 18, 29, 7, 6, 22, 20, 17, 8,
                15, 24, 12, 31, 33, 0, 23, 27, 32, 28, 30
            ],
            "Each shuffled card's position in the deck before the shuffle"
        );
        assert_eq!(session.random, 12_278_733_189_936_530_505);
    }
}
