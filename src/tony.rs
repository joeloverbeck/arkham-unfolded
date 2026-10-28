//! The local prepared Tony turn selected in GitHub #22.
//! The issue binds original card faces, RR pp3–5,9,11–13,17,20–26 and
//! FAQ2.5 §1.10. This is deliberately not a general action/card engine.

use crate::{APP_VERSION, Choice, InvestigatorView, RuleError, TokenFace};
use serde::{Deserialize, Serialize};
use std::io::Read;

pub mod continuity;

pub const SOURCE_REVISION: &str = "fc37a4f:issue-22-tony-turn-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Action,
    Commit,
    Reveal,
    Resolve,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestKind {
    Fight,
    Evade,
    Investigate,
}

impl TestKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fight => "Fight",
            Self::Evade => "Evade",
            Self::Investigate => "Investigate",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardView {
    pub id: u8,
    pub code: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhoulView {
    pub alive: bool,
    pub ready: bool,
    pub engaged: bool,
    pub damage: u8,
    pub bounties: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestView {
    pub kind: TestKind,
    pub skill_value: i32,
    pub difficulty: i32,
    pub modifier: Option<i32>,
    pub success: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub revision: u64,
    pub stage: Stage,
    pub prompt: String,
    pub choices: Vec<Choice>,
    pub ordinary_actions: u8,
    pub bonus_action: bool,
    pub damage: u8,
    pub horror: u8,
    pub health: u8,
    pub sanity: u8,
    pub resources: u8,
    pub clues: u8,
    pub study_clues: u8,
    pub contracts_bounties: u8,
    pub injury: bool,
    pub ghoul: GhoulView,
    pub hand: Vec<CardView>,
    pub committed: Vec<CardView>,
    pub discard: Vec<CardView>,
    pub deck_count: usize,
    pub held_token: Option<TokenFace>,
    pub bag_count: usize,
    pub test: Option<TestView>,
    pub history: Vec<String>,
    pub can_undo: bool,
    pub investigators: Vec<InvestigatorView>,
    pub jacqueline_used: bool,
    pub jacqueline_arcane_research: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub source_revision: String,
    pub app_version: String,
    pub moments: Vec<Observation>,
}

pub fn validate_recording(recording: &Recording) -> Result<(), RuleError> {
    if recording.source_revision != SOURCE_REVISION || recording.app_version != APP_VERSION {
        return Err(RuleError(
            "This turn recording requires a different compatible application/source version."
                .into(),
        ));
    }
    if recording.moments.is_empty()
        || recording.moments.iter().enumerate().any(|(i, m)| {
            m.revision != i as u64
                || m.health != 9
                || m.sanity != 5
                || m.clues > 3
                || m.study_clues != 6 - m.clues
                || m.damage > 8
                || m.horror > 3
                || m.ordinary_actions > 3
                || !(5..=9).contains(&m.resources)
                || !(5..=9).contains(&m.contracts_bounties)
                || m.bag_count.checked_add(usize::from(m.held_token.is_some())) != Some(16)
                || (m.stage == Stage::Resolve) != m.held_token.is_some()
                || m.held_token == Some(TokenFace::Curse)
                || m.investigators.len() != 3
                || !m.jacqueline_used
                || m.jacqueline_arcane_research != 2
                || m.hand.len() > 4
                || m.committed.len() > 2
                || m.discard.len() > 3
                || !(29..=32).contains(&m.deck_count)
                || m.choices.iter().enumerate().any(|(i, c)| {
                    c.id.is_empty() || m.choices[..i].iter().any(|earlier| earlier.id == c.id)
                })
        })
    {
        return Err(RuleError("The recorded turn sequence is invalid.".into()));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct State {
    stage: Stage,
    ordinary_actions: u8,
    bonus_action: bool,
    damage: u8,
    horror: u8,
    resources: u8,
    clues: u8,
    contracts_bounties: u8,
    injury: bool,
    ghoul: GhoulView,
    hand: Vec<CardView>,
    committed: Vec<CardView>,
    discard: Vec<CardView>,
    deck: Vec<CardView>,
    bag: Vec<TokenFace>,
    held_token: Option<TokenFace>,
    test: Option<TestView>,
    random: u64,
    draws: Option<Vec<TokenFace>>,
    draw_cursor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    revision: u64,
    seed: u64,
    fixed_draws: Option<Vec<TokenFace>>,
    state: State,
    history: Vec<String>,
    moments: Vec<Observation>,
    undo: Vec<Undo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Undo {
    state: State,
    label: String,
}

fn card(id: u8, code: &str) -> CardView {
    CardView {
        id,
        code: code.into(),
        title: match code {
            "03263" => "On the Hunt",
            "01089" => "Guts",
            "02038" => "Internal Injury",
            _ => "Unrevealed card",
        }
        .into(),
    }
}

fn standard_bag() -> Vec<TokenFace> {
    use TokenFace::*;
    vec![
        PlusOne, Zero, Zero, MinusOne, MinusOne, MinusOne, MinusTwo, MinusTwo, MinusThree,
        MinusFour, Skull, Skull, Cultist, Tablet, AutoFail, ElderSign,
    ]
}

fn prepared_deck() -> Vec<CardView> {
    // The full 35-copy pinned slot multiset remains private state. Only its
    // three-card prefix is reachable with the actions in this prepared turn.
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../docs/research/issue-1-decks.json"))
            .expect("Pinned deck evidence is JSON");
    let slots = source["decks"][0]["slots"]
        .as_object()
        .expect("Tony has deck slots");
    let mut codes = vec![
        "01089".to_string(),
        "01089".to_string(),
        "03263".to_string(),
    ];
    for (code, count) in slots {
        let outside = match code.as_str() {
            "01089" | "03263" => 2,
            "02038" | "06010" => 1,
            _ => 0,
        };
        for _ in outside..count.as_u64().expect("Slot count") {
            codes.push(code.clone());
        }
    }
    codes
        .iter()
        .enumerate()
        .map(|(i, code)| card(i as u8 + 1, code))
        .collect()
}

impl Session {
    pub fn new() -> Result<Self, RuleError> {
        let mut bytes = [0; 8];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut bytes))
            .map_err(|e| RuleError(format!("Could not obtain session randomness: {e}")))?;
        Ok(Self::from_seed(u64::from_le_bytes(bytes), None))
    }

    /// Privileged deterministic test setup; never offered by a player client.
    pub fn with_draws(draws: Vec<TokenFace>) -> Self {
        Self::from_seed(1, Some(draws))
    }

    fn from_seed(seed: u64, fixed_draws: Option<Vec<TokenFace>>) -> Self {
        let mut game = Self {
            revision: 0, seed, fixed_draws: fixed_draws.clone(),
            state: State {
                stage: Stage::Action, ordinary_actions: 3, bonus_action: true, damage: 0,
                horror: 0, resources: 5, clues: 0, contracts_bounties: 5, injury: true,
                ghoul: GhoulView { alive: true, ready: true, engaged: true, damage: 0, bounties: 1 },
                hand: vec![card(0, "03263")], committed: vec![], discard: vec![],
                deck: prepared_deck(), bag: standard_bag(), held_token: None, test: None,
                random: seed, draws: fixed_draws, draw_cursor: 0,
            },
            history: vec!["Prepared position: Tony's turn in the Study, The Gathering on Standard. Three ordinary actions and one additional Fight/Engage action against an enemy with a bounty. Stop after Tony's end-of-turn effects; companions do not take turns.".into()],
            moments: vec![],
            undo: vec![],
        };
        game.moments.push(game.observe());
        game
    }

    pub fn observe(&self) -> Observation {
        let s = &self.state;
        Observation {
            revision: self.revision,
            stage: s.stage,
            prompt: match s.stage {
                Stage::Action => "Choose Tony's next action, or end his turn.",
                Stage::Commit => "Choose cards to commit, or commit none.",
                Stage::Reveal => "Reveal one chaos token.",
                Stage::Resolve => "Resolve the revealed token and finish this test.",
                Stage::Ended => "Tony's turn is over. No next investigator or phase has begun.",
            }
            .into(),
            choices: self.choices(),
            ordinary_actions: s.ordinary_actions,
            bonus_action: s.bonus_action,
            damage: s.damage,
            horror: s.horror,
            health: 9,
            sanity: 5,
            resources: s.resources,
            clues: s.clues,
            study_clues: 6 - s.clues,
            contracts_bounties: s.contracts_bounties,
            injury: s.injury,
            ghoul: s.ghoul.clone(),
            hand: s.hand.clone(),
            committed: s.committed.clone(),
            discard: s.discard.clone(),
            deck_count: s.deck.len(),
            held_token: s.held_token,
            bag_count: s.bag.len(),
            test: s.test.clone(),
            history: self.history.clone(),
            can_undo: !self.undo.is_empty(),
            investigators: vec![
                InvestigatorView {
                    name: "Tony Morgan".into(),
                    location: "Study".into(),
                    horror: s.horror,
                    sanity: 5,
                },
                InvestigatorView {
                    name: "Patrice Hathaway".into(),
                    location: "Study".into(),
                    horror: 0,
                    sanity: 7,
                },
                InvestigatorView {
                    name: "Jacqueline Fine".into(),
                    location: "Study".into(),
                    horror: 2,
                    sanity: 9,
                },
            ],
            jacqueline_used: true,
            jacqueline_arcane_research: 2,
        }
    }

    fn choices(&self) -> Vec<Choice> {
        let s = &self.state;
        let mut choices = Vec::new();
        let mut add = |id: &str, label: &str| {
            choices.push(Choice {
                id: id.into(),
                label: label.into(),
                cancel_faces: vec![],
            })
        };
        match s.stage {
            Stage::Action => {
                if s.ghoul.alive && (s.ordinary_actions > 0 || s.bonus_action) {
                    add("fight", "Fight Ghoul Minion");
                }
                if s.ghoul.alive && !s.ghoul.engaged && (s.ordinary_actions > 0 || s.bonus_action) {
                    add("engage", "Engage Ghoul Minion");
                }
                if s.ghoul.alive && s.ghoul.engaged && s.ordinary_actions > 0 {
                    add("evade", "Evade Ghoul Minion");
                }
                if s.injury && s.ordinary_actions >= 2 {
                    add("injury", "Remove Internal Injury — two ordinary actions");
                }
                if s.ordinary_actions > 0 {
                    add("resource", "Gain 1 resource");
                    if !s.deck.is_empty() {
                        add("draw", "Draw 1 card");
                    }
                    if s.clues < 6 {
                        add("investigate", "Investigate the Study — shroud 2");
                    }
                }
                add("end_turn", "End Tony's turn");
            }
            Stage::Commit => {
                add("commit:none", "Commit no cards");
                if s.test
                    .as_ref()
                    .is_some_and(|test| test.kind != TestKind::Evade)
                {
                    let eligible: Vec<_> = s.hand.iter().filter(|c| c.code == "03263").collect();
                    for subset in 1..(1usize << eligible.len()) {
                        let cards: Vec<_> = eligible
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| subset & (1 << i) != 0)
                            .map(|(_, c)| c.id.to_string())
                            .collect();
                        add(
                            &format!("commit:{}", cards.join(",")),
                            &format!("Commit {} On the Hunt (+{})", cards.len(), cards.len()),
                        );
                    }
                }
            }
            Stage::Reveal => add("reveal", "Reveal chaos token"),
            Stage::Resolve => add("resolve", "Resolve skill test"),
            Stage::Ended => {}
        }
        if !self.undo.is_empty() {
            add("undo", "Undo last choice — no hidden information crossed");
        }
        choices
    }

    pub fn choose(&mut self, revision: u64, id: &str) -> Result<(), RuleError> {
        if revision != self.revision {
            return Err(RuleError(
                "That choice belongs to an earlier decision. Refresh the observation.".into(),
            ));
        }
        if !self.choices().iter().any(|choice| choice.id == id) {
            return Err(RuleError(
                "That choice is not legal at this decision.".into(),
            ));
        }
        let mut next = self.clone();
        if id == "undo" {
            let undo = next
                .undo
                .pop()
                .expect("Offered undo has a preceding choice");
            next.state = undo.state;
            next.history.push(format!("Undid '{}': restored its actions, cards and consequences. Earlier entries remain as recorded history.", undo.label));
        } else {
            let previous = next.state.clone();
            let label = next
                .choices()
                .into_iter()
                .find(|c| c.id == id)
                .expect("Offered choice")
                .label;
            next.apply(id)?;
            if matches!(id, "reveal" | "draw") {
                next.undo.clear();
            } else {
                next.undo.push(Undo {
                    state: previous,
                    label,
                });
            }
        }
        next.revision += 1;
        next.moments.push(next.observe());
        *self = next;
        Ok(())
    }

    fn apply(&mut self, id: &str) -> Result<(), RuleError> {
        match id {
            "fight" => {
                self.pay_enemy_action("Fight");
                self.state.test = Some(TestView {
                    kind: TestKind::Fight,
                    skill_value: 5,
                    difficulty: 2,
                    modifier: None,
                    success: None,
                });
                self.state.stage = Stage::Commit;
            }
            "evade" => {
                self.state.ordinary_actions -= 1;
                self.history.push(
                    "Evade costs one ordinary action and does not provoke an opportunity attack."
                        .into(),
                );
                self.state.test = Some(TestView {
                    kind: TestKind::Evade,
                    skill_value: 2,
                    difficulty: 2,
                    modifier: None,
                    success: None,
                });
                self.state.stage = Stage::Commit;
            }
            "investigate" => {
                self.state.ordinary_actions -= 1;
                self.history
                    .push("Investigate costs one ordinary action.".into());
                self.opportunity_attack();
                self.state.test = Some(TestView {
                    kind: TestKind::Investigate,
                    skill_value: 3,
                    difficulty: 2,
                    modifier: None,
                    success: None,
                });
                self.state.stage = Stage::Commit;
            }
            "engage" => {
                self.pay_enemy_action("Engage");
                self.state.test = None;
                self.state.ghoul.engaged = true;
                self.history.push(
                    "Tony engages Ghoul Minion. Engaging does not ready the enemy (RR p11).".into(),
                );
                self.finish_action();
            }
            "injury" => {
                self.state.ordinary_actions -= 2;
                self.state.test = None;
                self.opportunity_attack();
                self.state.injury = false;
                self.state.discard.push(card(34, "02038"));
                self.history
                    .push("Spent two ordinary actions to discard Internal Injury.".into());
                self.finish_action();
            }
            "resource" => {
                self.state.ordinary_actions -= 1;
                self.state.test = None;
                self.history
                    .push("Spent one ordinary action to gain a resource.".into());
                self.opportunity_attack();
                self.state.resources += 1;
                self.finish_action();
            }
            "draw" => {
                self.state.ordinary_actions -= 1;
                self.state.test = None;
                self.history
                    .push("Spent one ordinary action to draw a card.".into());
                self.opportunity_attack();
                let drawn = self.state.deck.remove(0);
                self.history.push(format!(
                    "Drew {}. This hidden-information reveal cannot be undone.",
                    drawn.title
                ));
                self.state.hand.push(drawn);
                self.finish_action();
            }
            "end_turn" => self.end_turn(),
            "commit:none" => {
                self.state.stage = Stage::Reveal;
                self.history.push("Tony commits no cards.".into());
            }
            choice if choice.starts_with("commit:") => {
                for id in choice[7..]
                    .split(',')
                    .map(|id| id.parse::<u8>().expect("Only offered card identifiers"))
                {
                    let index = self
                        .state
                        .hand
                        .iter()
                        .position(|c| c.id == id)
                        .expect("Offered card is in hand");
                    self.state.committed.push(self.state.hand.remove(index));
                    self.state
                        .test
                        .as_mut()
                        .expect("Commit has a test")
                        .skill_value += 1;
                }
                self.state.stage = Stage::Reveal;
                self.history.push(format!(
                    "Tony commits {} On the Hunt: +{} matching skill icons (RR ST.2).",
                    self.state.committed.len(),
                    self.state.committed.len()
                ));
            }
            "reveal" => {
                let index = if let Some(draws) = &self.state.draws {
                    let face = *draws.get(self.state.draw_cursor).ok_or_else(|| {
                        RuleError("Privileged draw sequence is exhausted.".into())
                    })?;
                    self.state
                        .bag
                        .iter()
                        .position(|f| *f == face)
                        .ok_or_else(|| RuleError("Token is absent from the Standard bag.".into()))?
                } else {
                    // SplitMix64's bijective mixing and full-period increment retain
                    // a reproducible session stream. All 16 physical tokens are
                    // present before each draw; 2^64 divides evenly into 16 bins.
                    // Selection by high four bits therefore has no modulo bias.
                    self.state.random = self.state.random.wrapping_add(0x9e3779b97f4a7c15);
                    let mut value = self.state.random;
                    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
                    ((value ^ (value >> 31)) >> 60) as usize
                };
                self.state.draw_cursor += 1;
                let face = self.state.bag.remove(index);
                self.state.held_token = Some(face);
                self.state.stage = Stage::Resolve;
                self.history.push(format!(
                    "Revealed {}. This hidden-information reveal cannot be undone.",
                    face.label()
                ));
            }
            "resolve" => {
                let face = self.state.held_token.take().expect("Resolve holds a token");
                // Original Gathering reference effects resolve at ST.4, before
                // successful Fight damage can remove the Ghoul at ST.7.
                let modifier = match face {
                    TokenFace::PlusOne => 1,
                    TokenFace::Zero | TokenFace::AutoFail => 0,
                    TokenFace::MinusOne | TokenFace::Cultist => -1,
                    TokenFace::MinusTwo | TokenFace::Tablet => -2,
                    TokenFace::MinusThree => -3,
                    TokenFace::MinusFour => -4,
                    TokenFace::Skull => -i32::from(self.state.ghoul.alive),
                    TokenFace::ElderSign => 2,
                    TokenFace::Curse => unreachable!("The Standard bag has no curse"),
                };
                match face {
                    TokenFace::Tablet if self.state.ghoul.alive => {
                        self.state.damage += 1;
                        self.history.push("Tablet: −2 and 1 damage because a Ghoul is at the Study, even if exhausted (Gathering Standard reference).".into());
                    }
                    TokenFace::ElderSign => {
                        self.state.contracts_bounties += 1;
                        self.history.push("Tony's elder sign: +2; place 1 bounty on Bounty Contracts. It is not a resource or a bounty on the enemy.".into());
                    }
                    TokenFace::Skull => self.history.push(format!("Skull: minus the number of Ghouls at the Study, including exhausted Ghouls ({modifier}).")),
                    _ => {},
                }
                let test = self.state.test.as_mut().expect("Resolve has a test");
                let value = if face == TokenFace::AutoFail {
                    0
                } else {
                    (test.skill_value + modifier).max(0)
                };
                let success = face != TokenFace::AutoFail && value >= test.difficulty;
                test.modifier = Some(modifier);
                test.success = Some(success);
                if face == TokenFace::AutoFail {
                    self.history.push("Auto-fail: the test automatically fails; modified skill value is 0 (RR ST.6).".into());
                }
                if face == TokenFace::Cultist && !success {
                    self.state.horror += 1;
                    self.history.push("Cultist: the test failed, so Tony takes 1 horror (Gathering Standard reference).".into());
                }
                self.history.push(format!(
                    "{}: {} {:+} = {} against difficulty {} — {}.",
                    test.kind.label(),
                    test.skill_value,
                    modifier,
                    value,
                    test.difficulty,
                    if success { "success" } else { "failure" }
                ));
                if success {
                    match test.kind {
                        TestKind::Fight => {
                            self.state.ghoul.damage += 1;
                            self.history.push(
                                "Successful basic Fight deals 1 damage to Ghoul Minion.".into(),
                            );
                            if self.state.ghoul.damage == 2 {
                                self.state.ghoul.alive = false;
                                self.state.ghoul.engaged = false;
                                self.state.resources += self.state.ghoul.bounties;
                                self.state.ghoul.bounties = 0;
                                self.history.push("Ghoul Minion is defeated. Bounty Contracts pays its bounty to Tony as 1 resource (mandatory).".into());
                            }
                        }
                        TestKind::Evade => {
                            self.state.ghoul.ready = false;
                            self.state.ghoul.engaged = false;
                            self.history.push(
                                "Evade succeeds: Ghoul Minion exhausts and disengages.".into(),
                            );
                        }
                        TestKind::Investigate => {
                            self.state.clues += 1;
                            self.history
                                .push("Tony discovers 1 clue from the Study.".into());
                        }
                    }
                }
                self.state.discard.append(&mut self.state.committed);
                self.state.bag.push(face);
                self.history.push("Test ends: committed cards are discarded and the chaos token returns to the bag (RR ST.8).".into());
                self.finish_action();
            }
            _ => unreachable!("Only offered choices are applied"),
        }
        Ok(())
    }

    fn finish_action(&mut self) {
        self.state.stage = Stage::Action;
        if self.state.ordinary_actions == 0 && !(self.state.bonus_action && self.state.ghoul.alive)
        {
            self.end_turn();
        }
    }

    fn pay_enemy_action(&mut self, action: &str) {
        if self.state.bonus_action && self.state.ghoul.bounties > 0 {
            self.state.bonus_action = false;
            self.history.push(format!(
                "{action} uses Tony's restricted additional action (FAQ2.5 §1.10)."
            ));
        } else {
            self.state.ordinary_actions -= 1;
            self.history
                .push(format!("{action} costs one ordinary action."));
        }
    }

    fn end_turn(&mut self) {
        self.state.stage = Stage::Ended;
        if self.state.injury {
            self.state.damage += 1;
            self.history
                .push("Internal Injury deals 1 direct damage at the end of Tony's turn.".into());
        }
        self.history
            .push("Tony's turn ends. No companion turn or enemy phase is executed.".into());
    }

    fn opportunity_attack(&mut self) {
        if self.state.ghoul.alive && self.state.ghoul.ready && self.state.ghoul.engaged {
            self.state.damage += 1;
            self.state.horror += 1;
            self.history.push("Ghoul Minion makes one attack of opportunity: 1 damage and 1 horror. It remains ready (RR p5).".into());
        }
    }

    pub fn recording(&self) -> Recording {
        Recording {
            source_revision: SOURCE_REVISION.into(),
            app_version: APP_VERSION.into(),
            moments: self.moments.clone(),
        }
    }

    pub fn same_run(&self, other: &Self) -> bool {
        self.seed == other.seed && self.fixed_draws == other.fixed_draws
    }
    pub fn is_continuation_of(&self, prior: &Self) -> bool {
        self.same_run(prior)
            && self.revision > prior.revision
            && self.moments.starts_with(&prior.moments)
    }
    pub fn is_fresh(&self) -> bool {
        self == &Self::from_seed(self.seed, self.fixed_draws.clone())
    }
    pub fn validate(&self) -> Result<(), RuleError> {
        self.validate_state(&self.state)?;
        for undo in &self.undo {
            self.validate_state(&undo.state)?;
            if undo.state.draw_cursor != self.state.draw_cursor
                || undo.state.random != self.state.random
                || undo.state.deck != self.state.deck
            {
                return Err(RuleError(
                    "An undo snapshot crosses hidden information.".into(),
                ));
            }
        }
        validate_recording(&self.recording())?;
        if self.revision.checked_add(1) != Some(self.moments.len() as u64)
            || self.moments.last() != Some(&self.observe())
        {
            return Err(RuleError(
                "The saved turn differs from its recorded continuation.".into(),
            ));
        }
        Ok(())
    }

    fn validate_state(&self, state: &State) -> Result<(), RuleError> {
        let invalid =
            || RuleError("Invalid prepared-turn state; the original record is unchanged.".into());
        if state.clues > 3
            || state.damage > 8
            || state.horror > 3
            || state.ordinary_actions > 3
            || !(5..=9).contains(&state.resources)
            || !(5..=9).contains(&state.contracts_bounties)
            || state.ghoul.damage > 2
            || state.ghoul.alive != (state.ghoul.damage < 2)
            || state.ghoul.bounties != u8::from(state.ghoul.alive)
            || (!state.ghoul.alive && state.ghoul.engaged)
            || state.draw_cursor > 4
            || state.draws != self.fixed_draws
        {
            return Err(invalid());
        }
        if let Some(draws) = &state.draws
            && (draws.len() > 4
                || state.draw_cursor > draws.len()
                || draws.contains(&TokenFace::Curse))
        {
            return Err(invalid());
        }
        if state
            .bag
            .len()
            .checked_add(usize::from(state.held_token.is_some()))
            != Some(16)
            || (state.stage == Stage::Resolve) != state.held_token.is_some()
        {
            return Err(invalid());
        }
        let mut tokens = state.bag.clone();
        tokens.extend(state.held_token);
        let expected_bag = standard_bag();
        if tokens.iter().any(|face| !expected_bag.contains(face))
            || expected_bag.iter().any(|face| {
                tokens.iter().filter(|f| *f == face).count()
                    != expected_bag.iter().filter(|f| *f == face).count()
            })
        {
            return Err(invalid());
        }
        let expected_deck = prepared_deck();
        if !(29..=32).contains(&state.deck.len())
            || !expected_deck.ends_with(&state.deck)
            || state.hand.len() > 4
            || state.committed.len() > 2
            || state.discard.len() > 3
        {
            return Err(invalid());
        }
        let mut cards = state.deck.clone();
        cards.extend(state.hand.clone());
        cards.extend(state.committed.clone());
        cards.extend(state.discard.clone());
        if state.injury {
            cards.push(card(34, "02038"));
        }
        cards.sort_by_key(|c| c.id);
        let mut expected = vec![card(0, "03263")];
        expected.extend(expected_deck);
        expected.push(card(34, "02038"));
        if cards != expected
            || state.hand.iter().any(|c| c.id > 3)
            || state.committed.iter().any(|c| c.code != "03263")
        {
            return Err(invalid());
        }
        let pending = matches!(state.stage, Stage::Commit | Stage::Reveal | Stage::Resolve);
        if pending && state.test.is_none() {
            return Err(invalid());
        }
        if !matches!(state.stage, Stage::Reveal | Stage::Resolve) && !state.committed.is_empty() {
            return Err(invalid());
        }
        if let Some(test) = &state.test {
            let base = match test.kind {
                TestKind::Fight => 5,
                TestKind::Evade => 2,
                TestKind::Investigate => 3,
            };
            if test.difficulty != 2
                || !(base..=base + 2).contains(&test.skill_value)
                || pending != test.success.is_none()
                || pending != test.modifier.is_none()
                || (pending && test.skill_value != base + state.committed.len() as i32)
                || (test.kind == TestKind::Evade && test.skill_value != 2)
                || (pending && test.kind != TestKind::Investigate && !state.ghoul.alive)
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
