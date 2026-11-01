//! The local prepared Study escape attempt selected in GitHub #55.
//! Governing inputs are bound by issue #55 at 7083162; this is not a campaign engine.

use crate::{APP_VERSION, Choice, RuleError, TokenFace, TokenStatus, VisibleToken};
use serde::{Deserialize, Serialize};
use std::io::Read;

pub mod continuity;
pub const SOURCE_REVISION: &str = "7083162:issue-55-study-escape-v2-act-reveal";
const LEGACY_SOURCE_REVISION: &str = "7083162:issue-55-study-escape-v1";

fn supported_source_revision(revision: &str) -> bool {
    matches!(revision, SOURCE_REVISION | LEGACY_SOURCE_REVISION)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Investigator {
    Tony,
    Patrice,
    Jacqueline,
}
impl Investigator {
    pub const ALL: [Self; 3] = [Self::Tony, Self::Patrice, Self::Jacqueline];
    pub fn index(self) -> usize {
        match self {
            Self::Tony => 0,
            Self::Patrice => 1,
            Self::Jacqueline => 2,
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Tony => "tony",
            Self::Patrice => "patrice",
            Self::Jacqueline => "jacqueline",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Tony => "Tony Morgan",
            Self::Patrice => "Patrice Hathaway",
            Self::Jacqueline => "Jacqueline Fine",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    SelectTurn,
    Action,
    Commit,
    BeforeReveal,
    Cancellation,
    Resolve,
    Lucky,
    AfterTest,
    Closing,
    ActReveal,
    Result,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    Escaped,
    Declined,
    TurnsEnded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationView {
    pub name: String,
    pub code: String,
    pub revealed: bool,
    pub clues: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardView {
    pub id: u16,
    pub owner: Investigator,
    pub code: String,
    pub title: String,
    pub intellect_icons: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigatorView {
    pub investigator: Investigator,
    pub location: String,
    pub ordinary_actions: u8,
    pub turn_finished: bool,
    pub resources: u8,
    pub clues: u8,
    pub damage: u8,
    pub horror: u8,
    pub health: u8,
    pub sanity: u8,
    pub intellect: u8,
    pub hand: Vec<CardView>,
    pub discard: Vec<CardView>,
    pub deck_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestView {
    pub investigator: Investigator,
    pub skill_value: i32,
    pub difficulty: i32,
    pub modifier: i32,
    pub result: Option<bool>,
    pub committed: Vec<CardView>,
    pub total: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub revision: u64,
    pub stage: Stage,
    pub prompt: String,
    pub choices: Vec<Choice>,
    pub unavailable: Vec<Choice>,
    pub actor: Option<Investigator>,
    pub investigators: Vec<InvestigatorView>,
    pub study_clues: u8,
    pub jacqueline_used: bool,
    pub tokens: Vec<VisibleToken>,
    pub bag_count: usize,
    pub curses_in_pool: u8,
    pub test: Option<TestView>,
    pub history: Vec<String>,
    pub can_undo: bool,
    pub locations: Vec<LocationView>,
    pub endpoint: Option<Endpoint>,
    pub contracts_bounties: u8,
    pub tony_bonus_action: bool,
    pub jacqueline_arcane_research: u8,
}
impl Observation {
    pub fn investigator(&self, investigator: Investigator) -> &InvestigatorView {
        &self.investigators[investigator.index()]
    }

    /// Tokens physically held outside the bag during the current test. `tokens`
    /// also retains completed-test evidence in existing saves and recordings.
    pub fn held_tokens(&self) -> &[VisibleToken] {
        if self.test.as_ref().is_some_and(|test| test.result.is_none()) {
            &self.tokens
        } else {
            &[]
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub source_revision: String,
    pub app_version: String,
    pub moments: Vec<Observation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct State {
    stage: Stage,
    actor: Option<Investigator>,
    investigators: Vec<InvestigatorView>,
    study_clues: u8,
    jacqueline_used: bool,
    bag: Vec<TokenFace>,
    tokens: Vec<VisibleToken>,
    group: Vec<u8>,
    curses_in_pool: u8,
    test: Option<TestView>,
    random: u64,
    draws: Option<Vec<TokenFace>>,
    draw_cursor: usize,
    decks: Vec<Vec<CardView>>,
    endpoint: Option<Endpoint>,
    contracts_bounties: u8,
    hidden_epoch: u64,
}

fn reversible_decision(state: &State) -> bool {
    !matches!(state.stage, Stage::Result | Stage::ActReveal)
        && !(state.stage == Stage::Resolve
            && state
                .tokens
                .iter()
                .filter(|token| token.status == TokenStatus::Pending)
                .count()
                == 1)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    revision: u64,
    seed: u64,
    fixed_draws: Option<Vec<TokenFace>>,
    state: State,
    history: Vec<String>,
    moments: Vec<Observation>,
    undo: Vec<State>,
}

fn card(id: u16, owner: Investigator, code: &str) -> CardView {
    let (title, intellect_icons) = match code {
        "03263" => ("On the Hunt", 1),
        "07032" => ("Promise of Power", 4),
        "60408" => ("Clairvoyance", 1),
        "01089" => ("Guts", 0),
        "01080" => ("Lucky!", 0),
        "01093" => ("Unexpected Courage", 2),
        "11067" => ("Eyes of the Dreamer", 1),
        _ => ("Unrevealed card", 0),
    };
    CardView {
        id,
        owner,
        code: code.into(),
        title: title.into(),
        intellect_icons,
    }
}

fn prepared_deck(owner: Investigator) -> Vec<CardView> {
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../docs/research/issue-1-decks.json"))
            .expect("Pinned deck JSON");
    let slots = source["decks"][owner.index()]["slots"]
        .as_object()
        .expect("Pinned slots");
    let (hand, prefix, permanent): (&str, [&str; 3], Option<(&str, u64)>) = match owner {
        Investigator::Tony => ("03263", ["01089", "01089", "03263"], Some(("06010", 1))),
        Investigator::Patrice => ("07032", ["01080", "01080", "01093"], None),
        Investigator::Jacqueline => ("60408", ["60408", "11067", "11067"], Some(("04109", 2))),
    };
    let mut codes: Vec<String> = prefix.iter().map(|code| (*code).into()).collect();
    for (code, count) in slots {
        let outside = u64::from(code == hand)
            + prefix.iter().filter(|&&value| value == code).count() as u64
            + permanent
                .filter(|(value, _)| *value == code)
                .map_or(0, |(_, count)| count);
        for _ in outside..count.as_u64().expect("Slot count") {
            codes.push(code.clone());
        }
    }
    codes
        .iter()
        .enumerate()
        .map(|(i, code)| card(owner.index() as u16 * 100 + i as u16 + 1, owner, code))
        .collect()
}

impl Session {
    pub fn new() -> Result<Self, RuleError> {
        let mut seed = [0; 8];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut file| file.read_exact(&mut seed))
            .map_err(|error| RuleError(format!("Could not obtain session randomness: {error}")))?;
        Ok(Self::from_seed(u64::from_le_bytes(seed), None))
    }
    /// Privileged deterministic verification input; never offered by player clients.
    pub fn with_draws(draws: Vec<TokenFace>) -> Self {
        Self::from_seed(1, Some(draws))
    }
    /// Privileged reproducible randomness for verification; no player control.
    #[doc(hidden)]
    pub fn with_seed(seed: u64) -> Self {
        Self::from_seed(seed, None)
    }
    fn from_seed(seed: u64, draws: Option<Vec<TokenFace>>) -> Self {
        use TokenFace::*;
        let decks: Vec<_> = Investigator::ALL.into_iter().map(prepared_deck).collect();
        let investigators = Investigator::ALL
            .into_iter()
            .map(|investigator| {
                let (code, health, sanity, intellect, horror) = match investigator {
                    Investigator::Tony => ("03263", 9, 5, 3, 0),
                    Investigator::Patrice => ("07032", 7, 7, 2, 0),
                    Investigator::Jacqueline => ("60408", 6, 9, 3, 2),
                };
                InvestigatorView {
                    investigator,
                    location: "Study".into(),
                    ordinary_actions: 3,
                    turn_finished: false,
                    resources: 0,
                    clues: 0,
                    damage: 0,
                    horror,
                    health,
                    sanity,
                    intellect,
                    hand: vec![card(investigator.index() as u16 * 100, investigator, code)],
                    discard: vec![],
                    deck_count: decks[investigator.index()].len(),
                }
            })
            .collect();
        let mut game = Self {
            revision: 0,
            seed,
            fixed_draws: draws.clone(),
            state: State {
                stage: Stage::SelectTurn,
                actor: None,
                investigators,
                study_clues: 6,
                jacqueline_used: false,
                bag: vec![
                    PlusOne, Zero, Zero, MinusOne, MinusOne, MinusOne, MinusTwo, MinusTwo,
                    MinusThree, MinusFour, Skull, Skull, Cultist, Tablet, AutoFail, ElderSign,
                ],
                tokens: vec![],
                group: vec![],
                curses_in_pool: 10,
                test: None,
                random: seed,
                draws,
                draw_cursor: 0,
                decks,
                endpoint: None,
                contracts_bounties: 6,
                hidden_epoch: 0,
            },
            history: vec![
                "Prepared Study escape attempt: choose the investigator whose turn begins.".into(),
            ],
            moments: vec![],
            undo: vec![],
        };
        game.moments.push(game.observe());
        game
    }
    pub fn observe(&self) -> Observation {
        let prompt = match self.state.stage {
            Stage::SelectTurn => "Choose the next investigator.",
            Stage::Action => "Choose an action.",
            Stage::Commit => {
                "Choose matching cards to commit; each supporting investigator may commit one."
            }
            Stage::BeforeReveal => "Reveal a chaos token, or use Jacqueline's reaction.",
            Stage::Cancellation => "Choose which tokens Jacqueline cancels.",
            Stage::Resolve => "Choose the next token to resolve.",
            Stage::Lucky => "This test would fail. Play Lucky! for one resource, or decline.",
            Stage::AfterTest => {
                "Patrice may shuffle all but one card of her discard into her deck. Choose the retained card or decline."
            }
            Stage::Closing => {
                "The party holds six clues. Spend them to advance Act 1, or finish without advancing."
            }
            Stage::ActReveal => {
                "Act 1 advances: The Door on the Floor. Read the reverse, then continue."
            }
            Stage::Result => "Finish the skill test.",
            Stage::Ended => "The prepared attempt has ended.",
        };
        Observation {
            revision: self.revision,
            stage: self.state.stage,
            prompt: prompt.into(),
            choices: self.choices(),
            unavailable: self.unavailable(),
            actor: self.state.actor,
            investigators: self.state.investigators.clone(),
            study_clues: self.state.study_clues,
            jacqueline_used: self.state.jacqueline_used,
            tokens: self.state.tokens.clone(),
            bag_count: self.state.bag.len(),
            curses_in_pool: self.state.curses_in_pool,
            test: self.state.test.clone(),
            history: self.history.clone(),
            can_undo: !self.undo.is_empty(),
            locations: if self.state.endpoint == Some(Endpoint::Escaped) {
                [
                    ("Hallway", "01112"),
                    ("Cellar", "01114"),
                    ("Attic", "01113"),
                    ("Parlor", "01115"),
                ]
                .into_iter()
                .map(|(name, code)| LocationView {
                    name: name.into(),
                    code: code.into(),
                    revealed: name == "Hallway",
                    clues: 0,
                })
                .collect()
            } else {
                vec![LocationView {
                    name: "Study".into(),
                    code: "01111".into(),
                    revealed: true,
                    clues: self.state.study_clues,
                }]
            },
            endpoint: self.state.endpoint,
            contracts_bounties: self.state.contracts_bounties,
            tony_bonus_action: true,
            jacqueline_arcane_research: 2,
        }
    }
    /// Current player choices may omit obsolete mechanical undo stops from an
    /// older save. The stored observation and replay remain exactly as recorded.
    pub fn observe_for_play(&self) -> Observation {
        let mut view = self.observe();
        view.can_undo = self.undo.iter().any(reversible_decision);
        if !view.can_undo {
            view.choices.retain(|choice| choice.id != "undo");
        }
        view
    }

    fn choices(&self) -> Vec<Choice> {
        let mut choices = vec![];
        let mut add = |id: String, label: String| {
            choices.push(Choice {
                id,
                label,
                cancel_faces: vec![],
            })
        };
        match self.state.stage {
            Stage::SelectTurn => {
                for investigator in &self.state.investigators {
                    if !investigator.turn_finished {
                        add(
                            format!("select:{}", investigator.investigator.id()),
                            format!("Begin {}'s turn", investigator.investigator.name()),
                        );
                    }
                }
            }
            Stage::Action => {
                let actor = self.state.actor.expect("Action actor");
                if self.state.investigators[actor.index()].ordinary_actions > 0 {
                    add(
                        "investigate".into(),
                        "Investigate Study (one action; intellect against shroud 2)".into(),
                    );
                    add("draw".into(), "Draw one card (one action)".into());
                    add("resource".into(), "Gain one resource (one action)".into());
                }
                add("end-turn".into(), "End this investigator's turn".into());
            }
            Stage::Commit => {
                let actor = self
                    .state
                    .actor
                    .expect("An active test has an investigator");
                let cards: Vec<_> = self
                    .state
                    .investigators
                    .iter()
                    .filter(|i| i.location == "Study")
                    .flat_map(|i| &i.hand)
                    .filter(|c| c.intellect_icons > 0)
                    .collect();
                for mask in 0..(1usize << cards.len()) {
                    let selected: Vec<_> = cards
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| mask & (1 << i) != 0)
                        .map(|(_, card)| *card)
                        .collect();
                    if Investigator::ALL.into_iter().any(|owner| {
                        owner != actor
                            && selected.iter().filter(|card| card.owner == owner).count() > 1
                    }) {
                        continue;
                    }
                    add(
                        if selected.is_empty() {
                            "commit:none".into()
                        } else {
                            format!(
                                "commit:{}",
                                selected
                                    .iter()
                                    .map(|c| c.id.to_string())
                                    .collect::<Vec<_>>()
                                    .join(",")
                            )
                        },
                        if selected.is_empty() {
                            "Commit no cards".into()
                        } else {
                            format!(
                                "Commit {}",
                                selected
                                    .iter()
                                    .map(|c| format!("{}'s {}", c.owner.name(), c.title))
                                    .collect::<Vec<_>>()
                                    .join(" and ")
                            )
                        },
                    );
                }
            }
            Stage::BeforeReveal => {
                add("reveal".into(), "Reveal one chaos token".into());
                if !self.state.jacqueline_used {
                    add(
                        "use-jacqueline".into(),
                        "Use Jacqueline: reveal three, then choose cancellations".into(),
                    );
                }
            }
            Stage::Cancellation => {
                let group = &self.state.group;
                let auto = group
                    .iter()
                    .copied()
                    .find(|&id| self.state.tokens[id as usize].face == TokenFace::AutoFail);
                let sets: Vec<Vec<u8>> = if let Some(auto) = auto {
                    vec![
                        vec![auto],
                        group.iter().copied().filter(|&id| id != auto).collect(),
                    ]
                } else {
                    (0..group.len())
                        .flat_map(|a| (a + 1..group.len()).map(move |b| vec![group[a], group[b]]))
                        .collect()
                };
                for set in sets {
                    add(
                        format!(
                            "cancel:{}",
                            set.iter().map(u8::to_string).collect::<Vec<_>>().join(",")
                        ),
                        format!(
                            "Cancel {}",
                            set.iter()
                                .map(|&id| self.state.tokens[id as usize].face.label())
                                .collect::<Vec<_>>()
                                .join(" and ")
                        ),
                    );
                }
            }
            Stage::Resolve => {
                for token in &self.state.tokens {
                    if token.status == TokenStatus::Pending {
                        add(
                            format!("resolve:{}", token.id),
                            format!("Resolve {}", token.face.label()),
                        );
                    }
                }
            }
            Stage::Result => add(
                "finish-test".into(),
                "Finish test and return held tokens".into(),
            ),
            Stage::Lucky => {
                let actor = self.state.actor.expect("Test actor");
                for card in &self.state.investigators[actor.index()].hand {
                    if card.code == "01080" {
                        add(
                            format!("lucky:{}", card.id),
                            "Play Lucky! (fast; pay one resource for +2 skill)".into(),
                        );
                    }
                }
                add("decline-lucky".into(), "Do not play Lucky!".into());
            }
            Stage::AfterTest => {
                for card in &self.state.investigators[Investigator::Patrice.index()].discard {
                    add(
                        format!("shuffle:retain:{}", card.id),
                        format!(
                            "Keep {} in the discard; shuffle the other cards into Patrice's deck",
                            card.title
                        ),
                    );
                }
                add(
                    "decline-shuffle".into(),
                    "Do not shuffle Patrice's discard".into(),
                );
            }
            Stage::Closing => {
                add(
                    "advance".into(),
                    "Spend six group clues and advance Act 1".into(),
                );
                add(
                    "decline-advance".into(),
                    "Finish the attempt without advancing".into(),
                );
            }
            Stage::ActReveal => add("continue-act".into(), "Continue".into()),
            Stage::Ended => {}
        }
        if !self.undo.is_empty() {
            add(
                "undo".into(),
                "Undo the last decision before new hidden information".into(),
            );
        }
        for choice in &mut choices {
            if let Some(ids) = choice.id.strip_prefix("cancel:") {
                choice.cancel_faces = ids
                    .split(',')
                    .map(|id| {
                        self.state.tokens[id.parse::<usize>().expect("Offered cancellation")].face
                    })
                    .collect();
            }
        }
        choices
    }
    pub fn choose(&mut self, revision: u64, id: &str) -> Result<(), RuleError> {
        if revision != self.revision {
            return Err(RuleError(
                "This choice is stale. Observe the current decision.".into(),
            ));
        }
        let choice = self
            .observe_for_play()
            .choices
            .into_iter()
            .find(|choice| choice.id == id)
            .ok_or_else(|| RuleError(self.refusal_reason(id)))?;
        let mut candidate = self.clone();
        candidate.history.push(choice.label);
        if id == "undo" {
            let index = candidate
                .undo
                .iter()
                .rposition(reversible_decision)
                .expect("Offered undo");
            candidate.state = candidate.undo[index].clone();
            candidate.undo.truncate(index);
            candidate.history.push("Undo restores the prior decision, costs and randomness. Earlier history remains recorded.".into());
        } else {
            let previous = candidate.state.clone();
            let reversible = reversible_decision(&previous);
            candidate.apply(id)?;
            // Result is retained for old saves, but new decisions include all
            // mandatory result/cleanup work before the next genuine choice.
            if candidate.state.stage == Stage::Result {
                candidate.apply("finish-test")?;
            }
            // Token reveals, ordinary draws, and Jacqueline's canceled-elder draw
            // reveal hidden information at commitment, never at animation end.
            let hidden = candidate.state.hidden_epoch != previous.hidden_epoch;
            if hidden {
                candidate.undo.clear();
            } else if reversible {
                candidate.undo.push(previous);
            }
        }
        // Preserve old observations on load; retire their purely mechanical
        // undo stops only inside the next successfully saved decision.
        candidate.undo.retain(reversible_decision);
        candidate.revision += 1;
        candidate.moments.push(candidate.observe());
        *self = candidate;
        Ok(())
    }
    fn apply(&mut self, id: &str) -> Result<(), RuleError> {
        if let Some(name) = id.strip_prefix("select:") {
            self.state.actor = Investigator::ALL
                .into_iter()
                .find(|actor| actor.id() == name);
            self.state.stage = Stage::Action;
        } else if id == "end-turn" {
            let actor = self.state.actor.expect("Turn actor");
            self.state.investigators[actor.index()].turn_finished = true;
            self.state.actor = None;
            if self.state.investigators.iter().all(|i| i.turn_finished) {
                self.state.stage = Stage::Ended;
                self.state.endpoint = Some(Endpoint::TurnsEnded);
                self.history.push("All three turns ended before six clues were collected. Stop before the enemy phase; this is no victory, defeat or scenario resolution.".into());
            } else {
                self.state.stage = Stage::SelectTurn;
            }
        } else if id == "draw" || id == "resource" {
            let actor = self.state.actor.expect("Action actor");
            self.state.investigators[actor.index()].ordinary_actions -= 1;
            if id == "draw" {
                self.draw_card(actor)?;
            } else {
                self.state.investigators[actor.index()].resources += 1;
            }
        } else if id == "investigate" {
            let actor = self.state.actor.expect("Action actor");
            let investigator = &mut self.state.investigators[actor.index()];
            investigator.ordinary_actions -= 1;
            self.state.tokens.clear();
            self.state.group.clear();
            self.state.test = Some(TestView {
                investigator: actor,
                skill_value: investigator.intellect.into(),
                difficulty: 2,
                modifier: 0,
                result: None,
                committed: vec![],
                total: None,
            });
            self.state.stage = Stage::Commit;
            if !self
                .state
                .investigators
                .iter()
                .any(|i| i.hand.iter().any(|card| card.intellect_icons > 0))
            {
                self.state.stage = Stage::BeforeReveal;
            }
        } else if let Some(ids) = id.strip_prefix("commit:") {
            let selected: Vec<u16> = ids.split(',').filter_map(|id| id.parse().ok()).collect();
            let test = self.state.test.as_mut().expect("Commit test");
            for investigator in &mut self.state.investigators {
                let mut index = 0;
                while index < investigator.hand.len() {
                    if selected.contains(&investigator.hand[index].id) {
                        let card = investigator.hand.remove(index);
                        test.skill_value += i32::from(card.intellect_icons);
                        if card.code == "07032" {
                            self.state.bag.push(TokenFace::Curse);
                            self.state.curses_in_pool -= 1;
                        }
                        test.committed.push(card);
                    } else {
                        index += 1;
                    }
                }
            }
            self.history.push(format!("Committed icons bring intellect to {} against Study shroud 2. Promise of Power adds its curse when committed. [RR ST.2; 07032]", test.skill_value));
            self.state.stage = Stage::BeforeReveal;
        } else if id == "reveal" || id == "use-jacqueline" {
            let reaction = id == "use-jacqueline";
            if reaction {
                self.state.jacqueline_used = true;
            }
            self.draw(if reaction { 3 } else { 1 })?;
            self.state.stage = if reaction {
                Stage::Cancellation
            } else {
                Stage::Resolve
            };
        } else if let Some(ids) = id.strip_prefix("cancel:") {
            let mut canceled_elder = false;
            for id in ids.split(',') {
                let token = &mut self.state.tokens[id.parse::<usize>().expect("Offered token")];
                token.status = TokenStatus::Canceled;
                canceled_elder |= token.face == TokenFace::ElderSign;
            }
            if canceled_elder && self.state.actor == Some(Investigator::Jacqueline) {
                self.draw_card(Investigator::Jacqueline)?;
            }
            self.state.stage = Stage::Resolve;
        } else if let Some(id) = id.strip_prefix("resolve:") {
            let token = &mut self.state.tokens[id.parse::<usize>().expect("Offered token")];
            let face = token.face;
            token.status = TokenStatus::Resolved;
            let modifier = match face {
                TokenFace::Skull => 0,
                TokenFace::Cultist => -1,
                TokenFace::Tablet => -2,
                TokenFace::ElderSign => {
                    if self.state.actor == Some(Investigator::Tony) {
                        self.state.contracts_bounties += 1;
                        2
                    } else {
                        1
                    }
                }
                _ => face.modifier().expect("Numbered token"),
            };
            self.state.test.as_mut().expect("Resolve test").modifier += modifier;
            self.state.stage = if face == TokenFace::Curse {
                Stage::BeforeReveal
            } else if self
                .state
                .tokens
                .iter()
                .any(|token| token.status == TokenStatus::Pending)
            {
                Stage::Resolve
            } else {
                if self.can_play_lucky() {
                    Stage::Lucky
                } else {
                    Stage::Result
                }
            };
        } else if let Some(id) = id.strip_prefix("lucky:") {
            let actor = self.state.actor.expect("Test actor");
            let investigator = &mut self.state.investigators[actor.index()];
            let index = investigator
                .hand
                .iter()
                .position(|card| card.id.to_string() == id)
                .expect("Offered Lucky");
            let card = investigator.hand.remove(index);
            investigator.resources -= 1;
            investigator.discard.push(card);
            self.state.test.as_mut().expect("Test").skill_value += 2;
            self.history.push("Lucky! is fast: pay one resource, add +2 to the skill value and determine success again. It does not cost an action. [01080; RR modifiers]".into());
            self.state.stage = Stage::Result;
        } else if id == "decline-lucky" {
            self.state.stage = Stage::Result;
        } else if let Some(retained) = id.strip_prefix("shuffle:retain:") {
            let retained: u16 = retained.parse().expect("Offered retained card");
            let index = Investigator::Patrice.index();
            let discard = std::mem::take(&mut self.state.investigators[index].discard);
            for card in discard {
                if card.id == retained {
                    self.state.investigators[index].discard.push(card);
                } else {
                    self.state.decks[index].push(card);
                }
            }
            for i in (1..self.state.decks[index].len()).rev() {
                let j = self.random_index(i + 1);
                self.state.decks[index].swap(i, j);
            }
            self.state.investigators[index].deck_count = self.state.decks[index].len();
            self.history.push("Patrice shuffles all but the chosen retained discard into her deck after the test. No card is revealed; this shuffle may be undone before any later hidden revelation. [06005]".into());
            self.post_action();
        } else if id == "decline-shuffle" {
            self.post_action();
        } else if id == "advance" {
            self.state.hidden_epoch += 1;
            for investigator in &mut self.state.investigators {
                investigator.clues = 0;
            }
            self.state.stage = Stage::ActReveal;
            self.history.push("Spend all six group clues and reveal Act 1b: The Door on the Floor. Read its story and instructions before continuing. [01108 reverse; RR act advancement]".into());
        } else if id == "continue-act" {
            for investigator in &mut self.state.investigators {
                investigator.location = "Hallway".into();
            }
            self.state.endpoint = Some(Endpoint::Escaped);
            self.history.push("Resolve The Door on the Floor: put Hallway, Cellar, Attic and Parlor into play. No enemies are in Study to discard. Place all investigators in Hallway, reveal Hallway, remove Study, and expose Act 2: The Barrier. Stop before another action; unfinished turns and limits are preserved. [01108 reverse; RR locations]".into());
            self.state.stage = Stage::Ended;
        } else if id == "decline-advance" {
            self.state.endpoint = Some(Endpoint::Declined);
            self.history.push("The party declines Act 1 advancement and finishes this prepared attempt. Keep Study, all six held clues and unfinished turns; no defeat or scenario resolution occurs.".into());
            self.state.stage = Stage::Ended;
        } else if id == "finish-test" {
            let test = self.state.test.as_mut().expect("Result test");
            let automatic_failure = self.state.tokens.iter().any(|token| {
                token.face == TokenFace::AutoFail && token.status == TokenStatus::Resolved
            });
            let total = if automatic_failure {
                0
            } else {
                (test.skill_value + test.modifier).max(0)
            };
            let success = !automatic_failure && total >= test.difficulty;
            test.result = Some(success);
            test.total = Some(total);
            if success {
                self.state.investigators[test.investigator.index()].clues += 1;
                self.state.study_clues -= 1;
            } else if self.state.tokens.iter().any(|token| {
                token.face == TokenFace::Cultist && token.status == TokenStatus::Resolved
            }) {
                self.state.investigators[test.investigator.index()].horror += 1;
            }
            for card in &test.committed {
                self.state.investigators[card.owner.index()]
                    .discard
                    .push(card.clone());
            }
            test.committed.clear();
            for token in &self.state.tokens {
                if token.face == TokenFace::Curse && token.status == TokenStatus::Resolved {
                    self.state.curses_in_pool += 1;
                } else {
                    self.state.bag.push(token.face);
                }
            }
            self.history.push(format!("Intellect {} {:+}; {}total {} against shroud 2: {}. Committed cards are discarded; held tokens return at ST.8, except resolved curse tokens return to the pool.", test.skill_value, test.modifier, if automatic_failure { "auto-fail sets the " } else { "" }, total, if success { "discover one clue" } else { "no clue discovered" }));
            let patrice_elder = test.investigator == Investigator::Patrice
                && self.state.tokens.iter().any(|token| {
                    token.face == TokenFace::ElderSign && token.status == TokenStatus::Resolved
                })
                && self.state.investigators[Investigator::Patrice.index()]
                    .discard
                    .len()
                    > 1;
            if patrice_elder {
                self.state.stage = Stage::AfterTest;
            } else {
                self.post_action();
            }
        }
        Ok(())
    }
    fn post_action(&mut self) {
        self.state.stage = if self
            .state
            .investigators
            .iter()
            .map(|i| u16::from(i.clues))
            .sum::<u16>()
            == 6
        {
            Stage::Closing
        } else {
            Stage::Action
        };
    }
    fn draw(&mut self, count: usize) -> Result<(), RuleError> {
        self.state.group.clear();
        for _ in 0..count {
            let index = if let Some(draws) = &self.state.draws {
                let face = draws
                    .get(self.state.draw_cursor)
                    .ok_or_else(|| RuleError("Verification draws exhausted.".into()))?;
                self.state.draw_cursor += 1;
                self.state
                    .bag
                    .iter()
                    .position(|token| token == face)
                    .ok_or_else(|| {
                        RuleError("The requested physical token is not in the bag.".into())
                    })?
            } else {
                self.random_index(self.state.bag.len())
            };
            let face = self.state.bag.remove(index);
            self.state.hidden_epoch += 1;
            let id = self.state.tokens.len() as u8;
            self.state.tokens.push(VisibleToken {
                id,
                face,
                status: TokenStatus::Pending,
            });
            self.state.group.push(id);
            self.history.push(format!("Revealed {}.", face.label()));
        }
        Ok(())
    }
    fn random_index(&mut self, length: usize) -> usize {
        let bound = length as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            self.state.random = self.state.random.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = self.state.random;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            value ^= value >> 31;
            if value >= threshold {
                return (value % bound) as usize;
            }
        }
    }
    fn can_play_lucky(&self) -> bool {
        let test = self.state.test.as_ref().expect("Test");
        let investigator = &self.state.investigators[test.investigator.index()];
        let failed = self.state.tokens.iter().any(|token| {
            token.face == TokenFace::AutoFail && token.status == TokenStatus::Resolved
        }) || (test.skill_value + test.modifier).max(0) < test.difficulty;
        failed
            && investigator.resources >= 1
            && investigator.hand.iter().any(|card| card.code == "01080")
    }
    fn draw_card(&mut self, actor: Investigator) -> Result<(), RuleError> {
        let deck = &mut self.state.decks[actor.index()];
        if deck.is_empty() {
            return Err(RuleError(
                "This prepared deck has no supported draw.".into(),
            ));
        }
        let card = deck.remove(0);
        if card.title == "Unrevealed card" {
            return Err(RuleError("The prepared attempt would reveal unsupported tail content; stop and review its reachable scope.".into()));
        }
        self.history.push(format!(
            "{} draws {}. This reveals hidden information.",
            actor.name(),
            card.title
        ));
        let investigator = &mut self.state.investigators[actor.index()];
        investigator.hand.push(card);
        investigator.deck_count = deck.len();
        self.state.hidden_epoch += 1;
        Ok(())
    }
    fn unavailable(&self) -> Vec<Choice> {
        let mut reasons = vec![
            (
                "fight".into(),
                "Fight requires an enemy target; there is no enemy in this prepared position."
                    .into(),
            ),
            (
                "evade".into(),
                "Evade requires an engaged enemy; there is no enemy in this prepared position."
                    .into(),
            ),
            (
                "engage".into(),
                "Engage requires an enemy target; there is no enemy in this prepared position."
                    .into(),
            ),
            (
                "move".into(),
                if self.state.endpoint == Some(Endpoint::Escaped) {
                    "The prepared attempt has finished after the map transition; actions in the new locations are outside this experiment.".into()
                } else {
                    "Study has no available connected destination before Act 1 advances.".into()
                },
            ),
        ];
        for investigator in &self.state.investigators {
            for card in &investigator.hand {
                let reason = match card.code.as_str() {
                    "60408" | "11067" => format!("{} costs {} resources and one action to play. {} has {} resources and {} actions remaining.", card.title, if card.code == "60408" { 4 } else { 3 }, investigator.investigator.name(), investigator.resources, investigator.ordinary_actions),
                    "03263" => "On the Hunt is played when you would draw an encounter card during the mythos phase; that timing does not occur here.".into(),
                    "01080" => "Lucky! is fast, costs one resource and is played when its investigator would fail a skill test; it has no commitment icons.".into(),
                    _ => "A skill card is committed to an appropriate test, not played as an action.".into(),
                };
                reasons.push((format!("play:{}", card.id), reason));
            }
        }
        reasons
            .into_iter()
            .map(|(id, label)| Choice {
                id,
                label,
                cancel_faces: vec![],
            })
            .collect()
    }
    fn refusal_reason(&self, id: &str) -> String {
        if let Some(choice) = self
            .unavailable()
            .into_iter()
            .find(|choice| choice.id == id)
        {
            return choice.label;
        }
        if id.starts_with("commit:") {
            return "Commit only matching intellect or wild icons from investigators at the testing investigator's location. Each supporting investigator may commit at most one card, and each physical card only once.".into();
        }
        if id.starts_with("select:") {
            return "An investigator may take one turn; finish the current turn and its pending decisions before selecting an eligible investigator.".into();
        }
        if matches!(id, "investigate" | "draw" | "resource" | "end-turn") {
            return "This action requires an action window and an ordinary action remaining; End turn cannot skip a pending test or consequence.".into();
        }
        "That choice is not legal at this decision. Observe the currently offered choices.".into()
    }
    pub fn recording(&self) -> Recording {
        Recording {
            source_revision: SOURCE_REVISION.into(),
            app_version: APP_VERSION.into(),
            moments: self.moments.clone(),
        }
    }
    pub fn is_continuation_of(&self, previous: &Self) -> bool {
        self.seed == previous.seed
            && self.fixed_draws == previous.fixed_draws
            && self.revision > previous.revision
            && self.moments.starts_with(&previous.moments)
    }
    pub fn same_run(&self, other: &Self) -> bool {
        self.seed == other.seed && self.fixed_draws == other.fixed_draws
    }
    pub fn is_fresh(&self) -> bool {
        self == &Self::from_seed(self.seed, self.fixed_draws.clone())
    }
    pub fn validate(&self) -> Result<(), RuleError> {
        self.validate_state(&self.state)?;
        for prior in &self.undo {
            self.validate_state(prior)?;
            if prior.hidden_epoch != self.state.hidden_epoch {
                return Err(RuleError(
                    "An undo snapshot crosses revealed hidden information.".into(),
                ));
            }
        }
        validate_recording(&self.recording())?;
        if self.revision.checked_add(1) != Some(self.moments.len() as u64)
            || self.moments.last() != Some(&self.observe())
        {
            return Err(RuleError(
                "The party save differs from its recorded continuation.".into(),
            ));
        }
        Ok(())
    }
    fn validate_state(&self, state: &State) -> Result<(), RuleError> {
        let invalid = || {
            RuleError("Invalid prepared party state; original record bytes are unchanged.".into())
        };
        if state.investigators.len() != 3
            || state.decks.len() != 3
            || state.study_clues > 6
            || state.curses_in_pool > 10
            || !(6..=9).contains(&state.contracts_bounties)
            || state.tokens.len() > 17
            || state.group.len() > 3
            || state.bag.len() > 17
            || state.draws != self.fixed_draws
            || state
                .draws
                .as_ref()
                .is_some_and(|draws| state.draw_cursor > draws.len())
            || state
                .group
                .iter()
                .any(|&id| id as usize >= state.tokens.len())
            || state
                .group
                .iter()
                .enumerate()
                .any(|(i, id)| state.group[..i].contains(id))
            || state
                .tokens
                .iter()
                .enumerate()
                .any(|(i, token)| token.id as usize != i)
            || (state.stage == Stage::Ended) != state.endpoint.is_some()
        {
            return Err(invalid());
        }
        let pending = matches!(
            state.stage,
            Stage::Commit
                | Stage::BeforeReveal
                | Stage::Cancellation
                | Stage::Resolve
                | Stage::Lucky
                | Stage::Result
        );
        let needs_actor = !matches!(state.stage, Stage::SelectTurn | Stage::Ended);
        if needs_actor && state.actor.is_none()
            || (state.stage == Stage::SelectTurn && state.actor.is_some())
            || (pending && state.test.is_none())
            || (state.stage == Stage::AfterTest
                && state.test.as_ref().is_none_or(|test| {
                    test.investigator != Investigator::Patrice || test.result.is_none()
                }))
            || state.test.as_ref().is_some_and(|test| {
                test.difficulty != 2
                    || !(2..=14).contains(&test.skill_value)
                    || !(-10..=5).contains(&test.modifier)
                    || test.committed.len() > 6
                    || test.total.is_some_and(|total| !(0..=20).contains(&total))
            })
            || (pending && state.actor != state.test.as_ref().map(|test| test.investigator))
        {
            return Err(invalid());
        }
        if (state.stage == Stage::Cancellation
            && (!state.jacqueline_used
                || state.group.len() != 3
                || state
                    .group
                    .iter()
                    .any(|&id| state.tokens[id as usize].status != TokenStatus::Pending)))
            || (state.stage == Stage::Resolve
                && !state
                    .tokens
                    .iter()
                    .any(|token| token.status == TokenStatus::Pending))
            || (state.stage == Stage::Commit && !state.tokens.is_empty())
            || (matches!(state.stage, Stage::Lucky | Stage::Result)
                && state
                    .tokens
                    .iter()
                    .any(|token| token.status == TokenStatus::Pending))
            || (state.stage == Stage::AfterTest
                && (state.investigators[Investigator::Patrice.index()]
                    .discard
                    .len()
                    < 2
                    || !state.tokens.iter().any(|token| {
                        token.face == TokenFace::ElderSign && token.status == TokenStatus::Resolved
                    })))
        {
            return Err(invalid());
        }
        let held = pending;
        for face in [
            TokenFace::PlusOne,
            TokenFace::Zero,
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            TokenFace::MinusThree,
            TokenFace::MinusFour,
            TokenFace::Skull,
            TokenFace::Cultist,
            TokenFace::Tablet,
            TokenFace::AutoFail,
            TokenFace::ElderSign,
            TokenFace::Curse,
        ] {
            let count = state.bag.iter().filter(|&&token| token == face).count()
                + if held {
                    state
                        .tokens
                        .iter()
                        .filter(|token| token.face == face)
                        .count()
                } else {
                    0
                };
            let expected = match face {
                TokenFace::Zero | TokenFace::MinusTwo | TokenFace::Skull => 2,
                TokenFace::MinusOne => 3,
                TokenFace::Curse => 10 - state.curses_in_pool as usize,
                _ => 1,
            };
            if count != expected {
                return Err(invalid());
            }
        }
        let total_clues: u16 = state.investigators.iter().map(|i| u16::from(i.clues)).sum();
        if total_clues + u16::from(state.study_clues)
            != if state.endpoint == Some(Endpoint::Escaped) || state.stage == Stage::ActReveal {
                0
            } else {
                6
            }
            || state.stage == Stage::Closing && total_clues != 6
            || state.endpoint == Some(Endpoint::Declined) && total_clues != 6
            || state.endpoint == Some(Endpoint::TurnsEnded)
                && (total_clues >= 6 || state.investigators.iter().any(|i| !i.turn_finished))
        {
            return Err(invalid());
        }
        for owner in Investigator::ALL {
            let index = owner.index();
            let investigator = &state.investigators[index];
            let (health, sanity, intellect, start_horror, initial_code) = match owner {
                Investigator::Tony => (9, 5, 3, 0, "03263"),
                Investigator::Patrice => (7, 7, 2, 0, "07032"),
                Investigator::Jacqueline => (6, 9, 3, 2, "60408"),
            };
            if investigator.investigator != owner
                || investigator.health != health
                || investigator.sanity != sanity
                || investigator.intellect != intellect
                || investigator.ordinary_actions > 3
                || investigator.resources > 3
                || investigator.damage != 0
                || !(start_horror..=start_horror + 3).contains(&investigator.horror)
                || investigator.clues > 3
                || investigator.hand.len() > 4
                || investigator.discard.len() > 4
                || investigator.deck_count != state.decks[index].len()
                || investigator.location
                    != if state.endpoint == Some(Endpoint::Escaped) {
                        "Hallway"
                    } else {
                        "Study"
                    }
            {
                return Err(invalid());
            }
            let mut actual: Vec<_> = investigator
                .hand
                .iter()
                .chain(&investigator.discard)
                .chain(&state.decks[index])
                .cloned()
                .collect();
            if let Some(test) = &state.test {
                actual.extend(
                    test.committed
                        .iter()
                        .filter(|card| card.owner == owner)
                        .cloned(),
                );
            }
            let mut expected = prepared_deck(owner);
            expected.push(card(index as u16 * 100, owner, initial_code));
            actual.sort_by_key(|card| card.id);
            expected.sort_by_key(|card| card.id);
            if actual != expected
                || investigator
                    .hand
                    .iter()
                    .any(|card| card.title == "Unrevealed card")
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

pub fn validate_recording(recording: &Recording) -> Result<(), RuleError> {
    if !supported_source_revision(&recording.source_revision)
        || recording.app_version != APP_VERSION
        || recording.moments.is_empty()
        || recording.moments.iter().enumerate().any(|(i, moment)| {
            moment.revision != i as u64
                || (recording.source_revision == LEGACY_SOURCE_REVISION
                    && moment.stage == Stage::ActReveal)
                || moment.investigators.len() != 3
                || moment.tokens.len() > 17
                || moment.study_clues > 6
                || moment.bag_count > 17
                || moment.curses_in_pool > 10
                || moment
                    .investigators
                    .iter()
                    .enumerate()
                    .any(|(index, investigator)| {
                        investigator.investigator.index() != index
                            || investigator.ordinary_actions > 3
                            || investigator.resources > 3
                            || investigator.clues > 3
                            || investigator.hand.len() > 4
                    })
                || moment.choices.iter().enumerate().any(|(index, choice)| {
                    choice.id.is_empty()
                        || moment.choices[..index]
                            .iter()
                            .any(|other| other.id == choice.id)
                })
        })
    {
        return Err(RuleError(
            "The party recording is incompatible or invalid.".into(),
        ));
    }
    Ok(())
}
