//! Original Gathering preparation for the fixed #89 party.
//!
//! Ordinary preparation continues into scenario play; old experiment records stay separate.

mod asset_entry;
mod backpack;
pub use backpack::BackpackFixture;
mod cards;
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
pub use draws::DrawFixture;
pub use encounter_search::SearchFixture;
pub use player_cards::AssetFixture;
pub mod continuity;
pub mod inspection;
mod moments;
mod peter;
mod phases;
pub use moments::PresentationMoment;
pub use peter::PeterFixture;
mod player_cards;
mod presentation;
pub use presentation::{DiscardView, HarmAllocation, HarmView, PendingView};
mod tokens;
pub use scenario::{Resolution, ResolutionRecord};
pub use tokens::TokenFixture;
mod retry;
mod runtime;
pub use retry::RetryFixture;
mod player_events;
mod scenario;
mod selected_witnesses;
pub use selected_witnesses::SelectedWitness;
mod spell_events;
pub use player_events::PlayerEventFixture;
mod spells;
pub use spell_events::SpellEventFixture;
mod tony_cards;
mod tony_effects;
pub use tony_effects::TonyEffectFixture;
mod test_boosts;
mod test_cards;
pub use test_boosts::BoostFixture;
pub use test_cards::TestCardFixture;

pub use runtime::{
    CommitmentPresentation, HostileFixture, Phase, PlayObservation, PlayerPresentation, Skill,
    TestView, TonyBonusPresentation,
};

pub use crate::party::Investigator;
use crate::{APP_VERSION, Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};

pub const SOURCE_REVISION: &str = "c35be2c:issue-89-gathering-runtime-v4";
const DECK_PROJECTION: &str = include_str!("../docs/research/issue-1-decks.json");
const DECK_PROJECTION_SHA256: &str =
    "fcd3bb06ed278652e61bdfe43c080982b0c95649a2e4944043757d5326cbe7bb";

/// Effective selected-card text for readable inspection. Metadata does not imply
/// that the corresponding gameplay ability has been implemented.
pub fn card_rules(code: &str) -> Option<&'static str> {
    cards::definition(code).map(|card| card.effective_text)
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub revision: u64,
    pub stage: Stage,
    pub prompt: String,
    pub choices: Vec<Choice>,
    pub history: Vec<String>,
    pub source: PreparationSources,
    pub lead: Option<Investigator>,
    pub decision_owner: Option<Investigator>,
    pub investigators: Vec<InvestigatorView>,
    pub scenario: Option<ScenarioSetup>,
    pub standard_bag: Vec<TokenFace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_fixture: Option<HostileFixture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub play: Option<PlayObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub source_revision: String,
    pub app_version: String,
    pub moments: Vec<Observation>,
    pub presentation_moments: Vec<PresentationMoment>,
    // Verification inputs belong to the private record, never to player views.
    seed: u64,
    decisions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    verification_fixture: Option<HostileFixture>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    moments: Vec<Observation>,
    presentation_moments: Vec<PresentationMoment>,
    // Normalized live choices point into the append-only audit. Undo itself
    // never appears here, so restoring a position cannot recursively replay it.
    active_decisions: Vec<usize>,
    undo_floor: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    verification_fixture: Option<HostileFixture>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime: Option<runtime::Game>,
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
        let mut session = Self {
            seed, random: seed, revision: 0, stage: Stage::ChooseLead,
            source: PreparationSources { rules_revision: SOURCE_REVISION.into(),
                deck_projection_sha256: digest, taboo_id: 10, decks: sources },
            lead: None, mulligan_index: 0, investigators, decks,
            excluded_weaknesses: vec![vec![], vec![], vec![]], encounter_deck: vec![],
            scenario: None, decisions: vec![], moments: vec![], presentation_moments: vec![], active_decisions: vec![], undo_floor: 0, verification_fixture: None, runtime: None,
            history: vec!["Fresh campaign preparation: choose the lead investigator. The retained starting lists use the three owner-selected basic weaknesses; no previous campaign history is imported. Rules and taboo 10 are pinned to the #89 research, not claimed to be the latest available release.".into()],
        };
        session.moments.push(session.observe());
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
        }
    }

    pub fn observe_for_play(&self) -> Observation {
        self.observe()
    }

    pub fn presentation_after(&self, revision: u64) -> &[PresentationMoment] {
        let start = self
            .presentation_moments
            .partition_point(|moment| moment.revision <= revision);
        &self.presentation_moments[start..]
    }

    pub fn observe_presentation(&self, moment: &PresentationMoment) -> Observation {
        moments::project(&self.moments, &self.presentation_moments, moment)
    }

    pub fn choose(&mut self, revision: u64, id: &str) -> Result<(), RuleError> {
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
            .ok_or_else(|| {
                RuleError("That choice is not available at the current decision.".into())
            })?;
        let mut candidate = self.clone();
        if id == "undo" {
            candidate.restore_previous_position()?;
            if let Some(game) = &mut candidate.runtime {
                game.capture_presentation("Undo restores the previous position. The earlier choices remain in the recording.");
            }
        } else {
            candidate.apply(id)?;
            candidate.active_decisions.push(self.decisions.len());
            if candidate.crossed_undo_boundary(self) {
                candidate.undo_floor = candidate.active_decisions.len();
            }
        }
        candidate.revision += 1;
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
                candidate.presentation_moments.push(PresentationMoment {
                    revision: candidate.revision,
                    sequence: u32::try_from(sequence).map_err(|_| {
                        RuleError("Too many presentation steps in one decision.".into())
                    })?,
                    caption: snapshot.caption,
                    play: snapshot.play,
                });
            }
        }
        candidate.decisions.push(id.into());
        candidate.history.push(choice.label);
        candidate.moments.push(candidate.observe());
        *self = candidate;
        Ok(())
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
        active.pop();
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
        restored.moments = self.moments.clone();
        restored.presentation_moments = self.presentation_moments.clone();
        restored.active_decisions = active;
        restored.undo_floor = self.undo_floor;
        *self = restored;
        Ok(())
    }

    fn apply(&mut self, id: &str) -> Result<(), RuleError> {
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
                self.investigators[index].resources = 5;
                self.draw_opening(index, 5)?;
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
            self.draw_opening(index, removed.len())?;
            // Mulliganed cards stay out until replacement draws have finished.
            if !removed.is_empty() {
                self.decks[index].extend(removed);
                self.shuffle_deck(index);
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
                    }
                    self.investigators[owner].deck_count = self.decks[owner].len();
                }
                self.assemble_scenario()?;
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

    fn draw_opening(&mut self, index: usize, count: usize) -> Result<(), RuleError> {
        for _ in 0..count {
            loop {
                let card = self.decks[index].pop().ok_or_else(|| {
                    RuleError("The opening hand cannot be completed from the pinned deck.".into())
                })?;
                if card.weakness {
                    self.excluded_weaknesses[index].push(card);
                } else {
                    self.investigators[index].hand.push(card);
                    break;
                }
            }
        }
        self.investigators[index].deck_count = self.decks[index].len();
        Ok(())
    }

    fn random_index(&mut self, upper: usize) -> usize {
        // SplitMix64 with rejection sampling; deterministic state is saved, never
        // player-visible. Avoid modulo bias when selecting a shuffle position.
        let upper = upper as u64;
        let threshold = upper.wrapping_neg() % upper;
        loop {
            self.random = self.random.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = self.random;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            value ^= value >> 31;
            if value >= threshold {
                return (value % upper) as usize;
            }
        }
    }

    fn shuffle_deck(&mut self, index: usize) {
        for last in (1..self.decks[index].len()).rev() {
            let other = self.random_index(last + 1);
            self.decks[index].swap(last, other);
        }
    }

    fn assemble_scenario(&mut self) -> Result<(), RuleError> {
        let inventory = content::ENCOUNTER
            .iter()
            .map(|&(code, count)| (code.to_owned(), count))
            .collect();
        let encounter_inventory = quantities(&inventory)?;
        for entry in &encounter_inventory {
            self.encounter_deck.extend(std::iter::repeat_n(
                entry.code.clone(),
                usize::from(entry.quantity),
            ));
        }
        for last in (1..self.encounter_deck.len()).rev() {
            let other = self.random_index(last + 1);
            self.encounter_deck.swap(last, other);
        }
        for investigator in &mut self.investigators {
            investigator.location = Some("01111".into());
        }
        self.scenario = Some(ScenarioSetup {
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
            encounter_inventory,
            encounter_deck_count: self.encounter_deck.len(),
            skip_first_mythos: true,
        });
        Ok(())
    }

    pub fn recording(&self) -> Recording {
        Recording {
            source_revision: SOURCE_REVISION.into(),
            app_version: APP_VERSION.into(),
            moments: self.moments.clone(),
            presentation_moments: self.presentation_moments.clone(),
            seed: self.seed,
            decisions: self.decisions.clone(),
            verification_fixture: self.verification_fixture,
        }
    }

    pub fn same_run(&self, other: &Self) -> bool {
        self.seed == other.seed
            && self.source == other.source
            && self.verification_fixture == other.verification_fixture
    }

    pub fn is_continuation_of(&self, prior: &Self) -> bool {
        self.same_run(prior)
            && self.decisions.starts_with(&prior.decisions)
            && self.moments.starts_with(&prior.moments)
            && self
                .presentation_moments
                .starts_with(&prior.presentation_moments)
    }

    pub fn is_fresh(&self) -> bool {
        self.revision == 0 && self.decisions.is_empty()
    }

    pub fn validate(&self) -> Result<(), RuleError> {
        let mut reconstructed = Self::from_origin(self.seed, self.verification_fixture)?;
        for decision in &self.decisions {
            reconstructed.choose(reconstructed.revision, decision)?;
        }
        match_recorded_projection_presence(
            &mut reconstructed.moments,
            &mut reconstructed.presentation_moments,
            &self.moments,
            &self.presentation_moments,
        );
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
                session.moments = vec![session.observe()];
                return Ok(session);
            }
            session.stage = Stage::Playing;
            session.lead = Some(Investigator::Patrice);
            session.history = vec!["Privileged source-derived runtime verification; this is not an ordinary fresh campaign.".into()];
            let mut game =
                runtime::Game::fixture(fixture, seed, &session.investigators, &session.decks);
            game.presentation.clear();
            session.history.append(&mut game.history);
            session.runtime = Some(game);
            session.moments = vec![session.observe()];
        }
        Ok(session)
    }
}

pub fn validate_recording(recording: &Recording) -> Result<(), RuleError> {
    if recording.source_revision != SOURCE_REVISION
        || recording.app_version != APP_VERSION
        || recording.moments.is_empty()
    {
        return Err(RuleError(
            "Unsupported or empty Gathering setup recording.".into(),
        ));
    }
    let mut reconstructed = Session::from_origin(recording.seed, recording.verification_fixture)?;
    for decision in &recording.decisions {
        reconstructed.choose(reconstructed.revision, decision)?;
    }
    match_recorded_projection_presence(
        &mut reconstructed.moments,
        &mut reconstructed.presentation_moments,
        &recording.moments,
        &recording.presentation_moments,
    );
    if reconstructed.moments != recording.moments
        || reconstructed.presentation_moments != recording.presentation_moments
    {
        return Err(RuleError(
            "Gathering recording does not match its recorded decisions and pinned inputs.".into(),
        ));
    }
    Ok(())
}

fn match_recorded_projection_presence(
    expected: &mut [Observation],
    expected_presentation: &mut [PresentationMoment],
    recorded: &[Observation],
    recorded_presentation: &[PresentationMoment],
) {
    for (expected, recorded) in expected.iter_mut().zip(recorded) {
        if let (Some(expected), Some(recorded)) = (&mut expected.play, &recorded.play) {
            expected.match_recorded_projection_presence(recorded);
        }
    }
    for (expected, recorded) in expected_presentation.iter_mut().zip(recorded_presentation) {
        expected
            .play
            .match_recorded_projection_presence(&recorded.play);
    }
}
