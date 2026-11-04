//! Concrete Gathering timing and consequences, reached through the Session seam.
//! Named hostile fixtures exercise incomplete behavior without enabling real play.

use super::{
    CardView, Investigator, InvestigatorView, cards, content,
    scenario::{ScenarioDecision, ScenarioState},
};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostileFixture {
    PatriceCancellationVision,
    PatriceCancellationRottingRemains,
    GatheringFromSetup,
    GatheringRoundEnd,
    GatheringPriestDefeat,
    GatheringAgendaTwoNoGhoul,
    GatheringAncientEvilsTerminal,
    GatheringGhoulMovement,
    GatheringRoundEndVoices,
    GatheringLitaDamage,
    GatheringRetaliate,
    GatheringLitaParley,
    GatheringNonterminalDefeat,
    GatheringHunters,
    SimultaneousAssetDefeat,
    DissonantVoicesAssetPlay,
    TestModifierFreeze,
    JacquelineProtectedTokens,
    JacquelineCancellation,
    Tokens(super::tokens::TokenFixture),
    Draws(super::draws::DrawFixture),
    Lifecycle(super::elimination::LifecycleFixture),
    Search(super::encounter_search::SearchFixture),
    Assets(super::player_cards::AssetFixture),
    TestCards(super::test_cards::TestCardFixture),
    Retry(super::retry::RetryFixture),
    Tony(super::tony_effects::TonyEffectFixture),
    Boosts(super::test_boosts::BoostFixture),
    SpellEvents(super::spell_events::SpellEventFixture),
    PlayerEvents(super::player_events::PlayerEventFixture),
    Harm(super::harm_cards::HarmFixture),
    CryptChill(super::crypt_chill::CryptChillFixture),
    Peter(super::peter::PeterFixture),
    Ethereal(super::ethereal_form::EtherealFixture),
    Backpack(super::backpack::BackpackFixture),
    SelectedWitness(super::selected_witnesses::SelectedWitness),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Willpower,
    Intellect,
    Combat,
    Agility,
}
impl Skill {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Willpower => 0,
            Self::Intellect => 1,
            Self::Combat => 2,
            Self::Agility => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Window {
    TurnSelection,
    Investigation,
    Scenario,
    EncounterReaction,
    BeforeCommitment,
    Commitment,
    BeforeReveal,
    Reveal,
    WouldFail,
    AfterSuccess,
    Results,
    VerificationBoundary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Investigation,
    Enemy,
    Upkeep,
    Mythos,
    RoundEnd,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetView {
    pub card: CardView,
    pub ready: bool,
    pub uses: u8,
    pub damage: u8,
    pub horror: u8,
}
pub(super) type AssetState = AssetView;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PlayerState {
    pub investigator: Investigator,
    pub location: String,
    pub base_skills: [u8; 4],
    pub health: u8,
    pub sanity: u8,
    pub resources: u8,
    pub clues: u8,
    pub damage: u8,
    pub horror: u8,
    pub hand: Vec<CardView>,
    pub deck: Vec<CardView>,
    pub discard: Vec<CardView>,
    pub removed: Vec<CardView>,
    pub threats: Vec<CardView>,
    pub assets: Vec<AssetState>,
    pub defeated: bool,
    pub resigned: bool,
    pub ordinary_actions: u8,
    pub completed_actions_this_phase: u8,
    pub actions_taken_this_turn: u8,
    pub restricted_tony_action_used: bool,
    pub turn_finished: bool,
    pub physical_trauma: u8,
    pub mental_trauma: u8,
    pub frozen_action_used: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerView {
    pub investigator: Investigator,
    pub location: String,
    pub resources: u8,
    pub clues: u8,
    pub damage: u8,
    pub horror: u8,
    pub hand: Vec<CardView>,
    pub deck_count: usize,
    pub discard: Vec<CardView>,
    pub threats: Vec<CardView>,
    pub assets: Vec<AssetView>,
    pub removed: Vec<CardView>,
    pub defeated: bool,
    pub resigned: bool,
    pub physical_trauma: u8,
    pub mental_trauma: u8,
    pub presentation: PlayerPresentation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerPresentation {
    pub health: u8,
    pub sanity: u8,
    pub turn_finished: bool,
    /// The ordinary three-action allowance, excluding separately granted actions.
    pub ordinary_action_allowance: u8,
    pub ordinary_actions_spent: u8,
    pub tony_bonus: Option<TonyBonusPresentation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TonyBonusPresentation {
    pub available: bool,
    pub used: bool,
    /// At least one currently offered action can spend this restricted action.
    pub eligible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevelationView {
    pub card: CardView,
    pub drawn_by: Investigator,
    pub started: bool,
    pub cancelled: bool,
}
pub(super) type PendingRevelation = RevelationView;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Frame {
    HarmCard(super::harm_cards::HarmCard),
    HandDiscard(super::harm_cards::HandDiscard),
    Idol(super::harm_cards::IdolDecision),
    CryptChill(super::crypt_chill::ChillDecision),
    GrantedAction {
        owner: Investigator,
    },
    RestoreTurn {
        previous_actor: Option<Investigator>,
        previous_phase: Phase,
    },
    // Historical serialized name; this is the shared ST6 success/failure
    // reaction window, which closes before any ST7 result is applied.
    SuccessWindow,
    AssetEntry(super::player_cards::AssetEntry),
    Backpack(super::backpack::BackpackDecision),
    AfterTest(super::retry::AfterTestState),
    RetryEvent {
        owner: Investigator,
        event: CardView,
    },
    TestCard(super::test_cards::TestCardDecision),
    EncounterSearch(super::encounter_search::SearchDecision),
    Elimination(super::elimination::EliminationDecision),
    Revelation(PendingRevelation),
    Scenario(ScenarioDecision),
    Tony(super::tony_cards::TonyDecision),
    Harm(HarmAssignment),
    DiscardCards {
        owner: Investigator,
        remaining: Vec<CardView>,
        ordered: Vec<CardView>,
    },
    AfterHarm(HarmContinuation),
    TestResults(Vec<TestResultEffect>),
    TestCleanup(Box<SkillTest>),
    QuarrySpawn {
        card: CardView,
        locations: Vec<String>,
    },
    ResumeTokens,
    Draw(super::draws::DrawState),
    GainResources {
        investigator: Investigator,
        amount: u8,
    },
    Phase(super::phases::PhaseDecision),
    Spell(super::spells::SpellDecision),
    Combat(super::combat::CombatDecision),
    AssetPlacement(super::player_cards::PreparedAsset),
    TurnEnd {
        investigator: Investigator,
        pending: Vec<u16>,
    },
    Peter {
        owner: Investigator,
    },
    ExpireEthereal,
    Opportunity {
        investigator: Investigator,
        action: String,
        pending: Vec<u16>,
        spell: Option<super::spells::SpellActivation>,
        asset: Option<super::player_cards::PreparedAsset>,
        event: Option<super::encounter_search::PaidEvent>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum TestResultEffect {
    OldKeyring {
        owner: Investigator,
        asset: u16,
    },
    Resourceful {
        owner: Investigator,
        card: u16,
    },
    Resources {
        owner: Investigator,
        amount: u8,
        source: String,
    },
    Operation,
    TakeHeart,
    Draw {
        owner: Investigator,
    },
    Horror {
        amount: u8,
        source: String,
        origin: HarmOrigin,
    },
    Damage {
        amount: u8,
        source: String,
        origin: HarmOrigin,
    },
}

/// Rules provenance supplied by the effect that produces the harm. Source text
/// remains presentation/history only and never determines card eligibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum HarmOrigin {
    EncounterCard,
    EnemyAttack { enemy: u16 },
    PlayerCard,
    GameRule,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct HarmAssignment {
    pub investigator: Investigator,
    pub damage: u8,
    pub horror: u8,
    pub direct: bool,
    pub source: String,
    pub origin: HarmOrigin,
    pub allocations: Vec<(Option<u16>, u8, u8)>,
    pub prevention_closed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum HarmContinuation {
    AgendaOne,
    ApplySymbols,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum TestOperation {
    CancelRevelation {
        event: CardView,
        owner: Investigator,
        encounter: u16,
    },
    RottingRemains {
        encounter: u16,
    },
    GraspingHands {
        encounter: u16,
    },
    CryptChill {
        encounter: u16,
    },
    DiscardThreat {
        owner: Investigator,
        card: u16,
    },
    LitaParley,
    Fight {
        enemy: u16,
        weapon: Option<u16>,
        skill: Skill,
        skill_bonus: i32,
        damage: u8,
    },
    Investigate {
        location: String,
        clues: u8,
    },
    Evade {
        enemy: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedCard {
    pub card: CardView,
    pub owner: Investigator,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SkillTest {
    pub bonus_sources: Vec<super::Contribution>,
    pub token_contributions: Vec<super::TokenContribution>,
    pub final_breakdown: Option<Box<super::TestBreakdown>>,
    pub boosts: super::test_boosts::BoostState,
    pub tony: super::tony_effects::TonyTestState,
    pub spell_drawback_triggered: bool,
    pub completed_granted_action: Option<Investigator>,
    pub is_retry: bool,
    pub card_effects: super::test_cards::TestCardState,
    pub operation: TestOperation,
    pub tester: Investigator,
    pub skill: Skill,
    pub difficulty: i32,
    pub window: Window,
    pub selected: Vec<u16>,
    pub committed: Vec<CommittedCard>,
    pub revealed: Vec<TokenFace>,
    pub modifier: i32,
    pub bonus: i32,
    pub source_card: Option<CardView>,
    pub ignored_tokens: Vec<TokenFace>,
    pub cancelled_tokens: Vec<TokenFace>,
    pub results_started: bool,
    pub eyes_match: bool,
    pub final_result: Option<(i32, bool, i32)>,
    pub final_modifiers: Option<TestModifiers>,
    pub lita_checked: bool,
    pub retaliate_checked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestModifiers {
    /// Effective icons of committed cards, including their printed conditions.
    pub cards: i32,
    /// Paid boosts, the originating ability, and applicable static modifiers.
    pub other: i32,
    /// Pending until the resolved tokens' numerical contribution is known.
    pub chaos: Option<i32>,
    pub automatic_failure: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestView {
    pub breakdown: super::TestBreakdown,
    pub source_card: Option<CardView>,
    pub ignored_tokens: Vec<TokenFace>,
    pub cancelled_tokens: Vec<TokenFace>,
    pub tester: Investigator,
    pub skill: Skill,
    pub difficulty: i32,
    pub base_skill: i32,
    pub modifiers: TestModifiers,
    pub selected: Vec<u16>,
    pub committed: Vec<CommittedCard>,
    pub revealed: Vec<TokenFace>,
    pub total: Option<i32>,
    pub would_succeed: Option<bool>,
    pub failure_margin: Option<i32>,
    /// Current legal commitment toggles, derived from offered choices.
    pub commitment: Vec<CommitmentPresentation>,
    /// Whether ST.6 has fixed this result.
    pub resolved: bool,
    /// The location named by an investigation operation.
    pub investigation_location: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentPresentation {
    pub card_id: u16,
    pub owner: Investigator,
    pub selected: bool,
    pub contribution: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestResult {
    pub breakdown: super::TestBreakdown,
    pub tester: Investigator,
    pub source: String,
    pub total: i32,
    pub difficulty: i32,
    pub success: bool,
    pub failure_margin: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayObservation {
    pub pending: super::presentation::PendingView,
    pub ethereal: Vec<Investigator>,
    pub phase: Phase,
    pub backpacks: Vec<super::backpack::AttachedCards>,
    pub searched_player: Vec<CardView>,
    pub searched_encounter: Vec<CardView>,
    pub lead: Investigator,
    pub window: Window,
    pub players: Vec<PlayerView>,
    pub pending_revelation: Option<RevelationView>,
    pub test: Option<TestView>,
    pub completed_tests: Vec<TestResult>,
    pub agenda: u8,
    pub agenda_doom: u8,
    pub act: u8,
    pub encounter_discard: Vec<CardView>,
    pub scenario: ScenarioState,
    pub bag_count: usize,
    pub curse_pool: u8,
    pub round: u16,
    pub actor: Option<Investigator>,
    pub actions_remaining: [u8; 3],
    pub encounter_deck_count: usize,
    pub tokens: Option<super::tokens::TokenResolution>,
    pub premonition: Option<super::tokens::SealedPremonition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Game {
    #[serde(skip)]
    pub presentation: Vec<super::moments::StepSnapshot>,
    pub ethereal: Vec<Investigator>,
    pub backpacks: Vec<super::backpack::AttachedCards>,
    pub tony: super::tony_effects::TonyRoundState,
    pub players: Vec<PlayerState>,
    pub lead: Investigator,
    pub agenda: u8,
    pub agenda_doom: u8,
    pub act: u8,
    pub encounter_discard: Vec<CardView>,
    pub encounter_deck: Vec<CardView>,
    pub scenario: ScenarioState,
    pub round: u16,
    pub phase: Phase,
    pub actor: Option<Investigator>,
    pub frames: Vec<Frame>,
    pub test: Option<SkillTest>,
    pub completed_tests: Vec<TestResult>,
    pub history: Vec<String>,
    pub random: u64,
    pub bag: Vec<TokenFace>,
    pub tokens: Option<super::tokens::TokenResolution>,
    pub jacqueline_uses: u8,
    pub premonition: Option<super::tokens::SealedPremonition>,
    pub action_in_progress: Option<Investigator>,
    pub stopped: Option<StoppedState>,
    fixture_tokens: Vec<TokenFace>,
    token_cursor: usize,
    ordinary_flow: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StoppedState {
    pub test: Option<SkillTest>,
    pub frames: Vec<Frame>,
    pub tokens: Option<super::tokens::TokenResolution>,
    pub action_in_progress: Option<Investigator>,
}

fn take(deck: &mut Vec<CardView>, code: &str) -> CardView {
    deck.remove(
        deck.iter()
            .position(|card| card.code == code)
            .expect("Selected fixture card exists in pinned deck"),
    )
}

impl Game {
    pub(super) fn after_setup(
        lead: Investigator,
        random: u64,
        investigators: &[InvestigatorView],
        decks: &[Vec<CardView>],
        encounters: &[String],
    ) -> Self {
        let players = investigators
            .iter()
            .enumerate()
            .map(|(index, player)| {
                let definition =
                    cards::definition(["06003", "06005", "60401"][index]).expect("Investigator");
                PlayerState {
                    investigator: player.investigator,
                    location: "01111".into(),
                    base_skills: definition.base_skills.expect("Skills"),
                    health: definition.health.expect("Health"),
                    sanity: definition.sanity.expect("Sanity"),
                    resources: player.resources,
                    clues: 0,
                    damage: player.damage,
                    horror: player.horror,
                    hand: player.hand.clone(),
                    deck: decks[index].clone(),
                    discard: vec![],
                    removed: vec![],
                    threats: vec![],
                    assets: player
                        .permanents
                        .iter()
                        .cloned()
                        .map(|card| AssetState {
                            uses: if card.code == "06010" { 6 } else { 0 },
                            card,
                            ready: true,
                            damage: 0,
                            horror: 0,
                        })
                        .collect(),
                    defeated: false,
                    resigned: false,
                    ordinary_actions: 3,
                    completed_actions_this_phase: 0,
                    actions_taken_this_turn: 0,
                    restricted_tony_action_used: false,
                    turn_finished: false,
                    physical_trauma: player.physical_trauma,
                    mental_trauma: player.mental_trauma,
                    frozen_action_used: false,
                }
            })
            .collect();
        Self {
            players,lead,agenda:1,agenda_doom:0,act:1,backpacks:vec![],ethereal:vec![],presentation:vec![],
            encounter_deck: encounters.iter().enumerate().map(|(index, code)| CardView {id:300+index as u16,code:code.clone(),title:content::title(code).expect("Encounter").into(),weakness:false}).collect(),
            encounter_discard:vec![],scenario:ScenarioState::new(),round:1,phase:Phase::Investigation,actor:None,frames:vec![],test:None,completed_tests:vec![],
            history:vec!["The first investigation phase begins directly after actual preparation; skip the first mythos phase. Investigators choose their turn order. [RR p23–24]".into()],
            random,bag:super::standard_bag(),fixture_tokens:vec![],token_cursor:0,ordinary_flow:true,tokens:None,jacqueline_uses:0,action_in_progress:None,stopped:None,premonition:None,tony:Default::default(),
        }
    }
    pub(super) fn fixture(
        fixture: HostileFixture,
        seed: u64,
        investigators: &[InvestigatorView],
        decks: &[Vec<CardView>],
    ) -> Self {
        let mut players: Vec<_> = investigators
            .iter()
            .enumerate()
            .map(|(index, investigator)| {
                let definition = cards::definition(["06003", "06005", "60401"][index])
                    .expect("Selected investigator");
                PlayerState {
                    investigator: investigator.investigator,
                    location: ["01113", "01114", "01112"][index].into(),
                    base_skills: definition.base_skills.expect("Investigator skills"),
                    health: definition.health.expect("Investigator health"),
                    sanity: definition.sanity.expect("Investigator sanity"),
                    resources: if index == 1 { 3 } else { 5 },
                    clues: 0,
                    damage: u8::from(index == 1),
                    horror: investigator.horror + u8::from(index == 0),
                    hand: vec![],
                    deck: decks[index].clone(),
                    discard: vec![],
                    removed: vec![],
                    threats: vec![],
                    assets: investigator
                        .permanents
                        .iter()
                        .cloned()
                        .map(|card| AssetState {
                            uses: if card.code == "06010" { 6 } else { 0 },
                            card,
                            ready: true,
                            damage: 0,
                            horror: 0,
                        })
                        .collect(),
                    defeated: false,
                    resigned: false,
                    ordinary_actions: 3,
                    completed_actions_this_phase: 0,
                    actions_taken_this_turn: 0,
                    restricted_tony_action_used: false,
                    turn_finished: false,
                    physical_trauma: investigator.physical_trauma,
                    mental_trauma: investigator.mental_trauma,
                    frozen_action_used: false,
                }
            })
            .collect();
        // Keep real physical cards in the remote investigators' opening hands.
        // Their locations exclude their commitments and reactions in this case.
        for index in [0, 2] {
            for _ in 0..5 {
                let position = players[index]
                    .deck
                    .iter()
                    .rposition(|card| !card.weakness)
                    .expect("Five ordinary opening cards");
                let card = players[index].deck.remove(position);
                players[index].hand.push(card);
            }
        }
        let patrice = &mut players[1];
        for code in ["60513", "04201", "01080", "01093", "02192"] {
            patrice.hand.push(take(&mut patrice.deck, code));
        }
        let violin = take(&mut patrice.deck, "06016");
        patrice.assets.push(AssetState {
            card: violin,
            ready: true,
            uses: 0,
            damage: 0,
            horror: 0,
        });
        // Draw order is a private source-derived witness, not a player ability.
        for code in ["60507", "04029", "05041"] {
            let card = take(&mut patrice.deck, code);
            patrice.deck.push(card);
        }
        let code = match fixture {
            HostileFixture::PatriceCancellationVision => "01166",
            HostileFixture::PatriceCancellationRottingRemains => "01163",
            HostileFixture::JacquelineProtectedTokens | HostileFixture::JacquelineCancellation => {
                "01163"
            }
            HostileFixture::GatheringFromSetup => unreachable!("Uses real preparation"),
            _ => "01166",
        };
        let mut encounter_deck = Vec::new();
        for &(entry, count) in content::ENCOUNTER {
            for _ in 0..count {
                encounter_deck.push(CardView {
                    id: 300 + encounter_deck.len() as u16,
                    code: entry.into(),
                    title: content::title(entry).expect("Encounter title").into(),
                    weakness: false,
                });
            }
        }
        let encounter = take(&mut encounter_deck, code);
        let mut scenario = ScenarioState::gathering_after_act_one();
        scenario.reveal("01113");
        scenario.reveal("01114");
        let mut game = Self {
            players, lead: Investigator::Patrice, agenda: 1, agenda_doom: 0, act: 2,
            encounter_discard: vec![], encounter_deck, scenario, round: 2, phase: Phase::Mythos, actor: None,
            frames: vec![Frame::Revelation(PendingRevelation { card: encounter, drawn_by: Investigator::Patrice, started: false, cancelled: false })],
            test: None, completed_tests: vec![], history: vec!["Privileged P1 verification state: Patrice is in Cellar, Tony in Attic, Jacqueline in Hallway; no other modifier or same-location supporting ability is active. This is not ordinary setup or evidence of complete playable Gathering.".into()],
            random: seed, bag: super::standard_bag(), fixture_tokens: vec![TokenFace::MinusOne, TokenFace::MinusTwo], token_cursor: 0, ordinary_flow:false,tokens:None,jacqueline_uses:0,action_in_progress:None,stopped:None,premonition:None,tony:Default::default(),backpacks:vec![],ethereal:vec![],presentation:vec![],
        };
        super::scenario::prepare_fixture(&mut game, fixture);
        if let HostileFixture::Tokens(case) = fixture {
            super::tokens::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Draws(case) = fixture {
            super::draws::configure_fixture(&mut game, case).expect("Bound draw fixture");
        }
        if let HostileFixture::Lifecycle(case) = fixture {
            super::elimination::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Search(case) = fixture {
            super::encounter_search::configure_fixture(&mut game, case)
                .expect("Bound search fixture");
        }
        if let HostileFixture::Assets(case) = fixture {
            super::player_cards::configure_fixture(&mut game, case);
        }
        if let HostileFixture::TestCards(case) = fixture {
            super::test_cards::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Retry(case) = fixture {
            super::retry::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Tony(case) = fixture {
            super::tony_effects::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Boosts(case) = fixture {
            super::test_boosts::configure_fixture(&mut game, case);
        }
        if let HostileFixture::SpellEvents(case) = fixture {
            super::spell_events::configure_fixture(&mut game, case);
        }
        if let HostileFixture::PlayerEvents(case) = fixture {
            super::player_events::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Harm(case) = fixture {
            super::harm_cards::configure_fixture(&mut game, case);
        }
        if let HostileFixture::CryptChill(case) = fixture {
            super::crypt_chill::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Peter(case) = fixture {
            super::peter::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Ethereal(case) = fixture {
            super::ethereal_form::configure_fixture(&mut game, case);
        }
        if let HostileFixture::Backpack(case) = fixture {
            super::backpack::configure_fixture(&mut game, case);
        }
        if let HostileFixture::SelectedWitness(case) = fixture {
            super::selected_witnesses::configure_fixture(&mut game, case);
        }
        if fixture == HostileFixture::TestModifierFreeze {
            let Some(Frame::Revelation(original)) = game.frames.pop() else {
                unreachable!("Prepared encounter");
            };
            game.encounter_deck.push(original.card);
            let card = take(&mut game.encounter_deck, "01162");
            game.frames.push(Frame::Revelation(PendingRevelation {
                card,
                drawn_by: Investigator::Patrice,
                started: false,
                cancelled: false,
            }));
            let peter = take(&mut game.players[1].deck, "02033");
            game.players[1].assets.push(AssetState {
                card: peter,
                ready: true,
                uses: 0,
                damage: 0,
                horror: 0,
            });
            game.set_fixture_tokens(vec![TokenFace::MinusFour]);
            game.history = vec!["Privileged modifier witness: Patrice faces actual Grasping Hands with her original one-health Peter Sylvestre. His agility bonus applies at ST6; his later defeat during damage results must not rewrite that arithmetic. [02033; 01162; RR ST6–8]".into()];
        }
        if fixture == HostileFixture::DissonantVoicesAssetPlay {
            for frame in game.frames.drain(..) {
                if let Frame::Revelation(revelation) = frame {
                    game.encounter_deck.push(revelation.card);
                }
            }
            let voices = take(&mut game.encounter_deck, "01165");
            let player = &mut game.players[Investigator::Jacqueline.index()];
            player.threats.push(voices);
            if !player.hand.iter().any(|card| card.code == "60412") {
                let robes = take(&mut player.deck, "60412");
                player.hand.push(robes);
            }
            game.enable_fixture_turn(Investigator::Jacqueline);
            game.history = vec!["Privileged Dissonant Voices witness: Jacqueline has the actual treachery in her threat area and Robes in hand. This verifies the play prohibition through the ordinary action transaction. [01165]".into()];
        }
        if fixture == HostileFixture::SimultaneousAssetDefeat {
            for frame in game.frames.drain(..) {
                if let Frame::Revelation(revelation) = frame {
                    game.encounter_deck.push(revelation.card);
                }
            }
            for player in &mut game.players {
                player.location = "01112".into();
            }
            game.scenario.location_mut("01113").expect("Attic").clues = 1;
            game.scenario.location_mut("01114").expect("Cellar").clues = 2;
            let priest = super::scenario::advance_act_two(&mut game).expect("Original Priest");
            game.scenario
                .enemies
                .iter_mut()
                .find(|enemy| enemy.card.id == priest)
                .expect("Priest")
                .engaged = Some(Investigator::Jacqueline);
            let player = &mut game.players[Investigator::Jacqueline.index()];
            for code in ["04197", "60412"] {
                let card =
                    if let Some(index) = player.hand.iter().position(|card| card.code == code) {
                        player.hand.remove(index)
                    } else {
                        take(&mut player.deck, code)
                    };
                player.assets.push(AssetState {
                    card,
                    ready: true,
                    uses: 0,
                    damage: u8::from(code == "60412"),
                    horror: 0,
                });
            }
            game.enable_fixture_turn(Investigator::Jacqueline);
            game.history=vec!["Privileged simultaneous asset defeat witness: Jacqueline controls Olive and Robes with one existing damage, and is engaged with the ready Priest. This state verifies the shared attack and discard-order continuation; it does not claim ordinary preparation reached this position. [RR p9]".into()];
        }
        if matches!(
            fixture,
            HostileFixture::GatheringPriestDefeat
                | HostileFixture::GatheringLitaDamage
                | HostileFixture::GatheringRetaliate
                | HostileFixture::GatheringLitaParley
                | HostileFixture::GatheringNonterminalDefeat
        ) {
            game.ordinary_flow = true;
        }
        if matches!(
            fixture,
            HostileFixture::JacquelineProtectedTokens | HostileFixture::JacquelineCancellation
        ) {
            let player = &mut game.players[Investigator::Jacqueline.index()];
            player.deck.append(&mut player.hand);
            if fixture == HostileFixture::JacquelineProtectedTokens {
                let weakness = take(&mut player.deck, "60403");
                player.threats.push(weakness);
            }
            game.fixture_tokens = if fixture == HostileFixture::JacquelineProtectedTokens {
                vec![TokenFace::AutoFail, TokenFace::Skull, TokenFace::Cultist]
            } else {
                vec![TokenFace::AutoFail, TokenFace::Zero, TokenFace::MinusOne]
            };
            let Some(Frame::Revelation(revelation)) = game.frames.last_mut() else {
                unreachable!()
            };
            revelation.drawn_by = Investigator::Jacqueline;
            revelation.started = true;
            let encounter = revelation.card.id;
            game.begin_test(
                TestOperation::RottingRemains { encounter },
                Investigator::Jacqueline,
                Skill::Willpower,
                3,
            )
            .expect("Fixture test");
        }
        game
    }

    pub(super) fn capture_presentation(&mut self, caption: impl Into<String>) {
        self.capture_public_view(caption, self.observe());
    }

    pub(super) fn capture_public_view(
        &mut self,
        caption: impl Into<String>,
        play: PlayObservation,
    ) {
        let caption = caption.into();
        if self
            .presentation
            .last()
            .is_some_and(|prior| prior.play == play && prior.caption == caption)
        {
            return;
        }
        self.presentation.push(super::moments::StepSnapshot {
            caption,
            play,
            draw: None,
        });
    }

    pub(super) fn observe(&self) -> PlayObservation {
        let choices = self.choices();
        let tony_eligible = choices
            .iter()
            .any(|choice| choice.id.starts_with("action:tony:"));
        PlayObservation {
            pending: super::presentation::observe(self),
            searched_encounter: super::encounter_search::visible_cards(self),
            lead: self.lead,
            window: self.test.as_ref().map_or(
                if self.frames.is_empty()
                    && self.ordinary_flow
                    && self.phase == Phase::Investigation
                {
                    if self.actor.is_some() {
                        Window::Investigation
                    } else {
                        Window::TurnSelection
                    }
                } else if self.frames.is_empty() {
                    Window::VerificationBoundary
                } else {
                    Window::EncounterReaction
                },
                |test| test.window,
            ),
            players: self
                .players
                .iter()
                .map(|p| PlayerView {
                    investigator: p.investigator,
                    location: p.location.clone(),
                    resources: p.resources,
                    clues: p.clues,
                    damage: p.damage,
                    horror: p.horror,
                    hand: p.hand.clone(),
                    deck_count: p.deck.len(),
                    discard: p.discard.clone(),
                    threats: p.threats.clone(),
                    assets: p.assets.clone(),
                    removed: {
                        let mut cards = p.removed.clone();
                        cards.sort_by_key(|card| card.id);
                        cards
                    },
                    defeated: p.defeated,
                    resigned: p.resigned,
                    physical_trauma: p.physical_trauma,
                    mental_trauma: p.mental_trauma,
                    presentation: PlayerPresentation {
                        health: p.health,
                        sanity: p.sanity,
                        turn_finished: p.turn_finished,
                        ordinary_action_allowance: 3,
                        ordinary_actions_spent: 3_u8.saturating_sub(p.ordinary_actions),
                        tony_bonus: (p.investigator == Investigator::Tony).then_some(
                            TonyBonusPresentation {
                                available: !p.restricted_tony_action_used
                                    && !p.turn_finished
                                    && !p.defeated
                                    && !p.resigned,
                                used: p.restricted_tony_action_used,
                                eligible: tony_eligible,
                            },
                        ),
                    },
                })
                .collect(),
            pending_revelation: self.frames.iter().find_map(|frame| match frame {
                Frame::Revelation(revelation) => Some(revelation.clone()),
                _ => None,
            }),
            test: self.test.as_ref().map(|test| {
                let result = (!test.revealed.is_empty()).then(|| self.result(test));
                TestView {
                    breakdown: super::test_report::breakdown(self, test),
                    source_card: test.source_card.clone(),
                    ignored_tokens: test.ignored_tokens.clone(),
                    cancelled_tokens: test.cancelled_tokens.clone(),
                    tester: test.tester,
                    skill: test.skill,
                    difficulty: test.difficulty,
                    base_skill: i32::from(
                        self.players[test.tester.index()].base_skills[test.skill.index()],
                    ),
                    modifiers: self.test_modifiers(test),
                    selected: test.selected.clone(),
                    committed: test.committed.clone(),
                    revealed: test.revealed.clone(),
                    total: result.map(|r| r.0),
                    would_succeed: result.map(|r| r.1),
                    failure_margin: result.map(|r| r.2),
                    commitment: self
                        .players
                        .iter()
                        .flat_map(|owner| {
                            owner
                                .hand
                                .iter()
                                .filter(|card| {
                                    choices
                                        .iter()
                                        .any(|choice| choice.id == format!("commit:{}", card.id))
                                })
                                .map(|card| CommitmentPresentation {
                                    card_id: card.id,
                                    owner: owner.investigator,
                                    selected: test.selected.contains(&card.id),
                                    contribution: self.commit_icons(test, card),
                                })
                        })
                        .collect(),
                    resolved: test.final_result.is_some(),
                    investigation_location: match &test.operation {
                        TestOperation::Investigate { location, .. } => Some(location.clone()),
                        _ => None,
                    },
                }
            }),
            completed_tests: self.completed_tests.clone(),
            agenda: self.agenda,
            agenda_doom: self.agenda_doom,
            act: self.act,
            encounter_discard: self.encounter_discard.clone(),
            scenario: self.scenario.clone(),
            bag_count: self.bag.len(),
            curse_pool: super::tokens::curse_pool(self),
            round: self.round,
            phase: self.phase,
            ethereal: self.ethereal.clone(),
            backpacks: self.backpacks.clone(),
            searched_player: super::backpack::searched_cards(self),
            actor: self.actor,
            actions_remaining: std::array::from_fn(|i| self.players[i].ordinary_actions),
            encounter_deck_count: self.encounter_deck.len(),
            tokens: self.tokens.clone(),
            premonition: self.premonition.clone(),
        }
    }

    pub(super) fn decision_owner(&self) -> Option<Investigator> {
        match self.frames.last() {
            Some(Frame::GrantedAction { owner }) => return Some(*owner),
            Some(Frame::AssetEntry(entry)) => return Some(entry.actor()),
            Some(Frame::AfterTest(state)) => return Some(state.test.tester),
            Some(Frame::TestCard(_)) => return super::test_cards::decision_owner(self),
            Some(Frame::EncounterSearch(decision))
                if (self.test.is_none() && self.tokens.is_none())
                    || !matches!(
                        decision,
                        super::encounter_search::SearchDecision::FinishEvent { .. }
                    ) =>
            {
                return Some(super::encounter_search::decision_owner(decision));
            }
            Some(Frame::Elimination(decision)) => {
                return match decision {
                    super::elimination::EliminationDecision::Trauma(who) => Some(*who),
                    super::elimination::EliminationDecision::Lead => None,
                };
            }
            Some(Frame::Combat(decision)) => return Some(super::combat::decision_owner(decision)),
            Some(Frame::AssetPlacement(prepared)) => return Some(prepared.actor),
            Some(Frame::Scenario(ScenarioDecision::Ending)) => return Some(self.lead),
            Some(Frame::Tony(_)) => return Some(Investigator::Tony),
            Some(Frame::QuarrySpawn { .. }) => return Some(self.lead),
            Some(Frame::Harm(harm)) => return Some(harm.investigator),
            Some(Frame::Idol(decision)) => return Some(decision.owner()),
            Some(Frame::CryptChill(decision)) => return Some(decision.owner),
            Some(Frame::HandDiscard(discard)) => return Some(discard.current.unwrap_or(self.lead)),
            Some(Frame::DiscardCards { owner, .. }) => return Some(*owner),
            Some(Frame::Draw(draw)) => return Some(draw.investigator),
            Some(Frame::Phase(decision)) if self.test.is_none() && self.tokens.is_none() => {
                return super::phases::decision_owner(self, decision);
            }
            Some(Frame::Scenario(ScenarioDecision::LitaRecipient)) if self.test.is_none() => {
                return None;
            }
            _ => {}
        }
        if self.tokens.is_some() {
            return super::tokens::decision_owner(self);
        }
        self.test
            .as_ref()
            .map(|test| test.tester)
            .or_else(|| {
                self.frames.last().map(|frame| match frame {
                    Frame::Revelation(revelation) => revelation.drawn_by,
                    Frame::HarmCard(event) => event.owner,
                    Frame::Idol(decision) => decision.owner(),
                    Frame::CryptChill(decision) => decision.owner,
                    Frame::HandDiscard(discard) => discard.current.unwrap_or(self.lead),
                    Frame::GrantedAction { owner } => *owner,
                    Frame::RestoreTurn { previous_actor, .. } => {
                        previous_actor.unwrap_or(self.lead)
                    }
                    Frame::SuccessWindow => self.test.as_ref().expect("Success reaction").tester,
                    Frame::AssetEntry(entry) => entry.actor(),
                    Frame::Backpack(decision) => decision.owner(),
                    Frame::AfterTest(state) => state.test.tester,
                    Frame::RetryEvent { owner, .. } => *owner,
                    Frame::TestCard(_) => {
                        super::test_cards::decision_owner(self).expect("Test card decision owner")
                    }
                    Frame::EncounterSearch(decision) => {
                        super::encounter_search::decision_owner(decision)
                    }
                    Frame::Elimination(_) => self.lead,
                    Frame::Scenario(_) => self.lead,
                    Frame::Tony(_) => Investigator::Tony,
                    Frame::Harm(harm) => harm.investigator,
                    Frame::DiscardCards { owner, .. } => *owner,
                    Frame::AfterHarm(_) => self.lead,
                    Frame::TestResults(_) => self.test.as_ref().expect("Test results").tester,
                    Frame::TestCleanup(test) => test.tester,
                    Frame::QuarrySpawn { .. } => self.lead,
                    Frame::ResumeTokens => Investigator::Jacqueline,
                    Frame::Draw(draw) => draw.investigator,
                    Frame::GainResources { investigator, .. } => *investigator,
                    Frame::Phase(_) => self.lead,
                    Frame::Spell(
                        super::spells::SpellDecision::MistsDiscard(owner)
                        | super::spells::SpellDecision::MistsMove(owner),
                    ) => *owner,
                    Frame::Combat(decision) => super::combat::decision_owner(decision),
                    Frame::AssetPlacement(prepared) => prepared.actor,
                    Frame::TurnEnd { investigator, .. } => *investigator,
                    Frame::Peter { owner } => *owner,
                    Frame::ExpireEthereal => self.lead,
                    Frame::Opportunity { investigator, .. } => *investigator,
                })
            })
            .or(self.actor)
    }

    fn outcome_reaction_choices(&self) -> Vec<Choice> {
        let mut choices = super::tony_effects::success_choices(self);
        choices.extend(super::test_cards::failure_choices(self));
        choices
    }

    pub(super) fn choices(&self) -> Vec<Choice> {
        if let Some(Frame::Backpack(decision)) = self.frames.last() {
            return super::backpack::choices(self, decision);
        }
        if let Some(Frame::Peter { owner }) = self.frames.last() {
            return super::peter::choices(self, *owner);
        }
        if let Some(Frame::GrantedAction { owner }) = self.frames.last() {
            return self.granted_action_choices(*owner);
        }
        if matches!(self.frames.last(), Some(Frame::SuccessWindow)) {
            let mut choices = self.outcome_reaction_choices();
            choices.push(choice(
                "success:pass",
                "Continue after test-outcome reactions",
            ));
            return choices;
        }
        if let Some(Frame::AssetEntry(entry)) = self.frames.last() {
            return super::player_cards::entry_choices(self, entry);
        }
        if let Some(Frame::AfterTest(state)) = self.frames.last() {
            return super::retry::choices(self, state);
        }
        if matches!(self.frames.last(), Some(Frame::TestCard(_))) {
            return super::test_cards::choices(self);
        }
        if let Some(Frame::EncounterSearch(decision)) = self.frames.last()
            && ((self.test.is_none() && self.tokens.is_none())
                || !matches!(
                    decision,
                    super::encounter_search::SearchDecision::FinishEvent { .. }
                ))
        {
            return super::encounter_search::choices(self, decision);
        }
        if let Some(Frame::Elimination(decision)) = self.frames.last() {
            return super::elimination::choices(self, decision);
        }
        if let Some(Frame::DiscardCards { remaining, .. }) = self.frames.last() {
            return remaining
                .iter()
                .map(|card| {
                    choice(
                        &format!("discard-card:{}", card.id),
                        &format!(
                            "Place {} next in the discard pile (later choices go on top)",
                            card.title
                        ),
                    )
                })
                .collect();
        }
        if matches!(self.frames.last(), Some(Frame::Draw(_))) {
            return super::draws::choices(self);
        }
        if let Some(Frame::Spell(decision)) = self.frames.last() {
            return super::spells::choices(self, decision);
        }
        if let Some(Frame::Combat(decision)) = self.frames.last() {
            return super::combat::choices(self, decision);
        }
        if let Some(Frame::AssetPlacement(prepared)) = self.frames.last() {
            return super::player_cards::placement_choices(self, prepared);
        }
        if let Some(Frame::Tony(decision)) = self.frames.last() {
            return super::tony_cards::choices(self, decision);
        }
        if let Some(Frame::QuarrySpawn { locations, .. }) = self.frames.last() {
            return locations
                .iter()
                .map(|location| {
                    choice(
                        &format!("quarry:{location}"),
                        &format!("Spawn Tony's Quarry at {location}"),
                    )
                })
                .collect();
        }
        if let Some(Frame::Scenario(ScenarioDecision::Ending)) = self.frames.last() {
            return super::scenario::choices(self, &ScenarioDecision::Ending);
        }
        if let Some(Frame::Harm(harm)) = self.frames.last() {
            return self.harm_choices(harm);
        }
        if let Some(Frame::Idol(decision)) = self.frames.last() {
            return super::harm_cards::idol_choices(self, decision);
        }
        if let Some(Frame::CryptChill(decision)) = self.frames.last() {
            return super::crypt_chill::choices(self, decision);
        }
        if let Some(Frame::HandDiscard(discard)) = self.frames.last() {
            return super::harm_cards::discard_choices(self, discard);
        }
        if let Some(Frame::TestResults(effects)) = self.frames.last() {
            return effects
                .iter()
                .enumerate()
                .map(|(index, effect)| {
                    let label = match effect {
                        TestResultEffect::OldKeyring { .. } => {
                            "Spend an Old Keyring key after successful investigation".into()
                        }
                        TestResultEffect::Operation => "Apply this skill test's effect".into(),
                        TestResultEffect::Resourceful { owner, .. } => format!(
                            "{} resolves Resourceful's Survivor card recovery",
                            owner.name()
                        ),
                        TestResultEffect::Resources {
                            owner,
                            amount,
                            source,
                        } => format!("{} gains {amount} resources from {source}", owner.name()),
                        TestResultEffect::TakeHeart => {
                            "Take Heart: draw 2 cards, then gain 2 resources".into()
                        }
                        TestResultEffect::Draw { owner } => {
                            format!("{} draws 1 card", owner.name())
                        }
                        TestResultEffect::Horror { amount, source, .. } => format!(
                            "Take {amount} horror from {}",
                            content::title(source).unwrap_or(source)
                        ),
                        TestResultEffect::Damage { amount, source, .. } => format!(
                            "Take {amount} damage from {}",
                            content::title(source).unwrap_or(source)
                        ),
                    };
                    choice(&format!("result:{index}"), &label)
                })
                .collect();
        }
        if self.tokens.is_some() {
            return super::tokens::choices(self);
        }
        if let Some(test) = &self.test {
            return match test.window {
                Window::BeforeCommitment | Window::BeforeReveal => {
                    let mut choices = self.player_window_choices();
                    choices.push(choice("window:pass", "Finish this player window"));
                    choices
                }
                Window::Commitment => {
                    let mut choices = self.commit_choices();
                    choices.extend(super::tony_effects::commitment_choices(self));
                    choices.push(choice("commit:confirm", "Confirm committed cards"));
                    choices
                }
                Window::Reveal => vec![choice("test:reveal", "Reveal a chaos token")],
                Window::WouldFail => {
                    let mut choices = self.lucky_choices();
                    choices.push(choice(
                        "lucky:decline",
                        "Decline Lucky! and apply the failed-test results",
                    ));
                    choices
                }
                _ => vec![],
            };
        }
        let revelation = match self.frames.last() {
            Some(Frame::HarmCard(_)) => return vec![],
            Some(Frame::ExpireEthereal) => return vec![],
            Some(Frame::Peter { owner }) => return super::peter::choices(self, *owner),
            Some(Frame::Backpack(decision)) => return super::backpack::choices(self, decision),
            Some(Frame::CryptChill(_)) => unreachable!("Handled asset loss"),
            Some(Frame::Idol(decision)) => return super::harm_cards::idol_choices(self, decision),
            Some(Frame::HandDiscard(discard)) => {
                return super::harm_cards::discard_choices(self, discard);
            }
            Some(Frame::GrantedAction { .. }) | Some(Frame::RestoreTurn { .. }) => return vec![],
            Some(Frame::SuccessWindow) => unreachable!("Handled success window"),
            Some(Frame::AssetEntry(_)) => unreachable!("Handled asset entry"),
            Some(Frame::AfterTest(_)) | Some(Frame::RetryEvent { .. }) => return vec![],
            Some(Frame::TestCard(_)) => unreachable!("Handled test card"),
            Some(Frame::EncounterSearch(_)) => unreachable!("Handled encounter search"),
            Some(Frame::Elimination(_)) => unreachable!("Handled elimination"),
            Some(Frame::Revelation(revelation)) => revelation,
            Some(Frame::Scenario(decision)) => return super::scenario::choices(self, decision),
            Some(Frame::Tony(decision)) => return super::tony_cards::choices(self, decision),
            Some(Frame::Harm(harm)) => return self.harm_choices(harm),
            Some(Frame::DiscardCards { .. }) => unreachable!("Handled discard ordering"),
            Some(Frame::AfterHarm(_)) => return vec![],
            Some(Frame::TestResults(_)) => unreachable!("Handled results"),
            Some(Frame::TestCleanup(_)) => return vec![],
            Some(Frame::ResumeTokens) => return vec![],
            Some(Frame::Draw(_)) => unreachable!("Handled draw"),
            Some(Frame::GainResources { .. }) => return vec![],
            Some(Frame::Phase(decision)) => return super::phases::choices(self, decision),
            Some(Frame::Spell(_)) => unreachable!("Handled spell"),
            Some(Frame::Combat(_)) => unreachable!("Handled combat"),
            Some(Frame::AssetPlacement(_)) => unreachable!("Handled placement"),
            Some(Frame::TurnEnd { pending, .. }) => {
                return pending
                    .iter()
                    .map(|id| {
                        choice(
                            &format!("turn-end:{id}"),
                            &format!("Resolve end-of-turn effect {id}"),
                        )
                    })
                    .collect();
            }
            Some(Frame::Opportunity { pending, .. }) => {
                return pending
                    .iter()
                    .map(|enemy| {
                        choice(
                            &format!("opportunity:{enemy}"),
                            &format!("Resolve enemy {enemy}'s attack of opportunity"),
                        )
                    })
                    .collect();
            }
            Some(Frame::QuarrySpawn { locations, .. }) => {
                return locations
                    .iter()
                    .map(|location| {
                        choice(
                            &format!("quarry:{location}"),
                            &format!("Spawn Tony's Quarry at {location}"),
                        )
                    })
                    .collect();
            }
            None if self.ordinary_flow => return self.investigation_choices(),
            None => return vec![],
        };
        let mut choices = self.cancellation_choices(revelation);
        choices.push(choice(
            "revelation:resolve",
            "Resolve the pending encounter revelation",
        ));
        choices
    }

    fn cancellation_choices(&self, revelation: &PendingRevelation) -> Vec<Choice> {
        let mut choices = super::harm_cards::revelation_choices(self, revelation);
        if !revelation.started
            && !revelation.card.weakness
            && super::scenario::enemy_definition(&revelation.card.code).is_none()
        {
            let location = &self.players[revelation.drawn_by.index()].location;
            for player in &self.players {
                if player.location == *location
                    && player.resources >= 1
                    && !player.defeated
                    && !player.resigned
                    && !player.threats.iter().any(|card| card.code == "01165")
                {
                    for card in &player.hand {
                        if card.code == "60513" {
                            choices.push(Choice { id: format!("test-of-will:{}", card.id), label: format!("{}: pay 1 resource for A Test of Will; test willpower (3) to cancel {}", player.investigator.name(), revelation.card.title), cancel_faces: vec![] });
                        }
                    }
                }
            }
        }
        choices
    }

    pub(super) fn choose(&mut self, id: &str) -> Result<(), RuleError> {
        if !self.choices().iter().any(|choice| choice.id == id) {
            return Err(RuleError(
                "That choice is not offered by the running effect.".into(),
            ));
        }
        if matches!(self.frames.last(), Some(Frame::Peter { .. })) {
            super::peter::choose(self, id)?;
            return self.settle();
        }
        if matches!(self.frames.last(), Some(Frame::Backpack(_))) {
            super::backpack::choose(self, id)?;
            return self.settle();
        }
        if let Some(Frame::GrantedAction { owner }) = self.frames.last().cloned() {
            self.frames.pop();
            if let Some(actor) = self.action_in_progress.take() {
                self.complete_action(actor);
            }
            self.frames.push(Frame::RestoreTurn {
                previous_actor: self.actor,
                previous_phase: self.phase,
            });
            self.actor = Some(owner);
            self.phase = Phase::Investigation;
            self.players[owner.index()].ordinary_actions += 1;
            self.choose_investigation(&format!(
                "action:ordinary:{}",
                id.strip_prefix("granted:").expect("Granted action")
            ))?;
        } else if matches!(self.frames.last(), Some(Frame::SuccessWindow)) {
            self.frames.pop();
            if id == "success:pass" {
                super::tony_effects::close_success_window(self);
                self.finish_test()?;
            } else {
                let index = self.frames.len();
                if id.starts_with("test-card:found:") {
                    super::test_cards::choose_failure(self, id)?;
                } else {
                    super::tony_effects::choose_success(self, id)?;
                }
                self.frames.insert(index, Frame::SuccessWindow);
            }
        } else if matches!(self.frames.last(), Some(Frame::AssetEntry(_))) {
            let Some(Frame::AssetEntry(entry)) = self.frames.pop() else {
                unreachable!()
            };
            super::player_cards::choose_entry(self, entry, id)?;
        } else if matches!(self.frames.last(), Some(Frame::AfterTest(_))) {
            super::retry::choose(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::TestCard(_))) {
            super::test_cards::choose(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::EncounterSearch(decision)) if self.test.is_none() && self.tokens.is_none() || !matches!(decision,super::encounter_search::SearchDecision::FinishEvent{..}))
        {
            super::encounter_search::choose(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::Elimination(_))) {
            super::elimination::choose(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::HandDiscard(_))) {
            super::harm_cards::choose_discard(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::Idol(_))) {
            super::harm_cards::choose_idol(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::CryptChill(_))) {
            super::crypt_chill::choose(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::DiscardCards { .. })) {
            self.choose_discard_order(id)?;
        } else if matches!(self.frames.last(), Some(Frame::Draw(_))) {
            super::draws::choose(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::Spell(_))) {
            let Some(Frame::Spell(decision)) = self.frames.pop() else {
                unreachable!()
            };
            if let super::spells::SpellStep::Move {
                investigator,
                location,
            } = super::spells::choose(self, decision, id)?
            {
                self.move_investigator(investigator, &location)?;
            }
        } else if matches!(self.frames.last(), Some(Frame::Combat(_))) {
            super::combat::choose(self, id)?;
        } else if matches!(self.frames.last(), Some(Frame::AssetPlacement(_))) {
            let Some(Frame::AssetPlacement(prepared)) = self.frames.pop() else {
                unreachable!()
            };
            if let Some(pending) = super::player_cards::choose_placement(self, prepared, id)? {
                self.frames.push(Frame::AssetPlacement(pending));
            }
        } else if matches!(self.frames.last(), Some(Frame::Harm(_))) {
            self.choose_harm(id)?;
        } else if matches!(self.frames.last(), Some(Frame::TestResults(_))) {
            self.choose_result(id)?;
        } else if self.tokens.is_some()
            && !matches!(
                self.frames.last(),
                Some(
                    Frame::QuarrySpawn { .. }
                        | Frame::Tony(_)
                        | Frame::Scenario(ScenarioDecision::Ending)
                )
            )
        {
            let step = super::tokens::choose(self, id)?;
            self.token_step(step)?;
        } else if let Some(card) = id.strip_prefix("premonition:") {
            super::tokens::play_premonition(self, card.parse().expect("Premonition identity"))?;
        } else if id.starts_with("fast-asset:") {
            super::player_cards::play_fast(self, self.actor.expect("Owner turn fast asset"), id)?;
        } else if matches!(self.frames.last(), Some(Frame::Phase(_)))
            && self.test.is_none()
            && !id.starts_with("violin:")
        {
            super::phases::choose(self, id)?;
        } else if id.starts_with("turn-end:") {
            self.choose_turn_end(id)?;
        } else if let Some(enemy) = id.strip_prefix("opportunity:") {
            let enemy: u16 = enemy.parse().expect("Enemy identity");
            let Some(Frame::Opportunity {
                investigator,
                pending,
                ..
            }) = self.frames.last_mut()
            else {
                unreachable!()
            };
            let actor = *investigator;
            pending.retain(|id| *id != enemy);
            if let Some(attack) = super::scenario::opportunity_attacks(self, actor)
                .into_iter()
                .find(|attack| attack.enemy == enemy)
            {
                self.queue_harm(
                    actor,
                    attack.damage,
                    attack.horror,
                    false,
                    &attack.source_code,
                    HarmOrigin::EnemyAttack { enemy },
                )?;
            }
        } else if let Some(location) = id.strip_prefix("quarry:") {
            let Some(Frame::QuarrySpawn { card, .. }) = self.frames.pop() else {
                unreachable!()
            };
            let enemy = card.id;
            self.scenario.enemies.push(super::scenario::EnemyState {
                card,
                location: location.into(),
                engaged: None,
                exhausted: false,
                damage: 0,
                doom: 1,
                bounties: 1,
                owner: Some(Investigator::Tony),
            });
            self.scenario_step(super::scenario::ScenarioStep::EnemyEntered(enemy))?;
        } else if matches!(self.frames.last(), Some(Frame::Scenario(_)))
            && (self.test.is_none()
                || matches!(
                    self.frames.last(),
                    Some(Frame::Scenario(ScenarioDecision::Ending))
                ))
        {
            let step = super::scenario::choose(self, id)?;
            self.scenario_step(step)?;
        } else if matches!(self.frames.last(), Some(Frame::Tony(_))) {
            super::tony_cards::apply(self, id)?;
        } else if id.starts_with("turn:")
            || id.starts_with("action:")
            || id == "turn:end"
            || id == "act:advance"
        {
            self.choose_investigation(id)?;
        } else if id.starts_with("ward:") {
            super::harm_cards::play_ward(self, id)?;
        } else if let Some(id) = id.strip_prefix("test-of-will:") {
            let id: u16 = id
                .parse()
                .map_err(|_| RuleError("Invalid card identity.".into()))?;
            let owner = self
                .players
                .iter()
                .position(|p| p.hand.iter().any(|c| c.id == id))
                .ok_or_else(|| RuleError("The event is not in a hand.".into()))?;
            let player = &mut self.players[owner];
            let event = player.hand.remove(
                player
                    .hand
                    .iter()
                    .position(|card| card.id == id)
                    .expect("Owned card"),
            );
            player.resources -= 1;
            let Some(Frame::Revelation(revelation)) = self.frames.last() else {
                return Err(RuleError("No pending encounter.".into()));
            };
            self.begin_test(
                TestOperation::CancelRevelation {
                    event,
                    owner: Investigator::ALL[owner],
                    encounter: revelation.card.id,
                },
                Investigator::ALL[owner],
                Skill::Willpower,
                3,
            )?;
        } else if id.starts_with("boost:") {
            super::test_boosts::choose(self, id)?;
            self.history.push("Pay 1 resource and place A Test of Will in limbo while its willpower (3) test resolves. The original encounter revelation has not begun. [60513; RR ST.1]".into());
        } else if id.starts_with("violin:") {
            let parts: Vec<_> = id.split(':').collect();
            let asset_id: u16 = parts[1].parse().expect("Offered asset");
            let discard_id: u16 = parts[3].parse().expect("Offered discard");
            let target = Investigator::ALL
                .into_iter()
                .find(|i| i.id() == parts[5])
                .expect("Offered target");
            let owner = self
                .players
                .iter()
                .position(|p| p.assets.iter().any(|asset| asset.card.id == asset_id))
                .expect("Offered asset owner");
            let player = &mut self.players[owner];
            player
                .assets
                .iter_mut()
                .find(|asset| asset.card.id == asset_id)
                .expect("Offered asset")
                .ready = false;
            let card = player.hand.remove(
                player
                    .hand
                    .iter()
                    .position(|card| card.id == discard_id)
                    .expect("Offered hand card"),
            );
            player.discard.push(card);
            if parts[6] == "draw" {
                self.draw_player(target, 1)?;
            } else {
                self.players[target.index()].resources += 1;
            }
            self.history.push(format!("Pay Violin's exhaust and discard costs; {} {}. The running test and pending encounter remain distinct. [06016]", target.name(), if parts[6] == "draw" { "draws one card" } else { "gains one resource" }));
        } else if id == "window:pass" {
            let test = self.test.as_mut().expect("Player window test");
            test.window = match test.window {
                Window::BeforeCommitment => Window::Commitment,
                Window::BeforeReveal => Window::Reveal,
                _ => return Err(RuleError("No player window is pending.".into())),
            };
        } else if id == "commit:confirm" {
            super::tony_effects::pay_commitment_costs(self)?;
            let test = self.test.as_mut().expect("Commitment test");
            for player in &mut self.players {
                let mut index = 0;
                while index < player.hand.len() {
                    if test.selected.contains(&player.hand[index].id) {
                        test.committed.push(CommittedCard {
                            card: player.hand.remove(index),
                            owner: player.investigator,
                        });
                    } else {
                        index += 1;
                    }
                }
            }
            test.selected.clear();
            test.window = Window::BeforeReveal;
            super::test_cards::committed(self)?;
        } else if id.starts_with("tony:watch:") {
            super::tony_effects::choose_commitment(self, id)?;
        } else if let Some(id) = id.strip_prefix("commit:") {
            let id: u16 = id.parse().expect("Offered commitment");
            super::tony_effects::reset_commitment(self, id);
            let selected = &mut self.test.as_mut().expect("Commitment test").selected;
            if let Some(index) = selected.iter().position(|&selected| selected == id) {
                selected.remove(index);
            } else {
                selected.push(id);
                selected.sort_unstable();
            }
        } else if id == "test:reveal" {
            let token = if let Some(face) = self.fixture_tokens.get(self.token_cursor).copied() {
                self.token_cursor += 1;
                let index = self
                    .bag
                    .iter()
                    .position(|&token| token == face)
                    .ok_or_else(|| RuleError("The verification token is not in the bag.".into()))?;
                self.bag.remove(index)
            } else {
                let index = self.random_index(self.bag.len());
                self.bag.remove(index)
            };
            let modifier = token.modifier().ok_or_else(|| {
                RuleError("This chaos-symbol consequence is not implemented yet.".into())
            })?;
            let test = self.test.as_mut().expect("Reveal test");
            test.revealed.push(token);
            test.modifier += modifier;
            {
                test.token_contributions.push(super::TokenContribution {
                    face: token,
                    amount: modifier,
                });
            }
            if !self.result(self.test.as_ref().expect("Test")).1 && !self.lucky_choices().is_empty()
            {
                self.test.as_mut().expect("Test").window = Window::WouldFail;
            } else {
                self.finish_test()?;
            }
        } else if id == "lucky:decline" {
            self.finish_test()?;
        } else if let Some(id) = id.strip_prefix("lucky:") {
            let id: u16 = id.parse().expect("Offered Lucky");
            let test = self.test.as_mut().expect("Would-fail test");
            let player = &mut self.players[test.tester.index()];
            let card = player.hand.remove(
                player
                    .hand
                    .iter()
                    .position(|card| card.id == id)
                    .expect("Offered Lucky"),
            );
            player.resources -= 1;
            test.record_bonus(&card.title, test.tester, 2);
            player.discard.push(card);
            test.bonus += 2;
            self.history.push("Pay 1 resource for Lucky!: add +2 to the skill value at the would-fail window. Recheck success; this does not alter the difficulty or erase Vision. [01080]".into());
            self.finish_test()?;
        } else if id == "revelation:resolve" {
            self.resolve_revelation()?;
        } else {
            return Err(RuleError(
                "This runtime consequence has not been implemented.".into(),
            ));
        }
        self.settle()
    }

    pub(super) fn begin_test(
        &mut self,
        operation: TestOperation,
        tester: Investigator,
        skill: Skill,
        difficulty: i32,
    ) -> Result<(), RuleError> {
        if matches!(operation, TestOperation::Fight { .. })
            && super::ethereal_form::is_ethereal(self, tester)
        {
            return Err(RuleError(
                "An ethereal investigator cannot attack an enemy.".into(),
            ));
        }
        if self.test.is_some() {
            return Err(RuleError(
                "A second skill test cannot start while another is running.".into(),
            ));
        }
        let source_card = match &operation {
            TestOperation::Fight {
                weapon: Some(id), ..
            } => self.players[tester.index()]
                .assets
                .iter()
                .find(|asset| asset.card.id == *id)
                .map(|asset| asset.card.clone()),
            _ => None,
        };
        self.test = Some(SkillTest {
            bonus_sources: vec![],
            token_contributions: vec![],
            final_breakdown: None,
            boosts: Default::default(),
            tony: Default::default(),
            spell_drawback_triggered: false,
            completed_granted_action: None,
            is_retry: false,
            card_effects: Default::default(),
            operation,
            tester,
            skill,
            difficulty,
            window: Window::BeforeCommitment,
            selected: vec![],
            committed: vec![],
            revealed: vec![],
            modifier: 0,
            bonus: 0,
            source_card,
            ignored_tokens: vec![],
            cancelled_tokens: vec![],
            results_started: false,
            eyes_match: false,
            final_result: None,
            final_modifiers: None,
            lita_checked: false,
            retaliate_checked: false,
        });
        super::test_cards::begin_test(self)
    }

    pub(super) fn random_index(&mut self, upper: usize) -> usize {
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

    fn token_step(&mut self, step: super::tokens::TokenStep) -> Result<(), RuleError> {
        match step {
            super::tokens::TokenStep::Pending | super::tokens::TokenStep::Finished => {}
            super::tokens::TokenStep::RevealedBatch { faces } => {
                if self.tokens.as_ref().is_some_and(|tokens| {
                    !tokens.ignored.is_empty() || !tokens.cancelled.is_empty()
                }) {
                    self.capture_presentation("The selected chaos tokens remain after ignored and cancelled tokens are excluded.");
                }
                self.frames.push(Frame::ResumeTokens);
                self.apply_spell_drawback(&faces)?;
            }
            super::tokens::TokenStep::ApplyTablet => {
                self.frames.push(Frame::ResumeTokens);
                let tester = self.test.as_ref().expect("Symbol test").tester;
                if self
                    .scenario
                    .ghouls_at(&self.players[tester.index()].location)
                    > 0
                {
                    self.queue_harm(
                        tester,
                        1,
                        0,
                        false,
                        "01104 Tablet",
                        HarmOrigin::EncounterCard,
                    )?;
                }
            }
            super::tokens::TokenStep::DrawElderSign => {
                self.capture_presentation("Jacqueline ignores or cancels an elder sign and must draw a card before the reveal continues.");
                self.frames.push(Frame::ResumeTokens);
                self.draw_player(Investigator::Jacqueline, 1)?;
            }
            super::tokens::TokenStep::Complete {
                resolved,
                ignored,
                cancelled,
                eyes_match,
                symbols_applied,
            } => {
                let contributions = resolved
                    .iter()
                    .map(|face| {
                        self.token_modifier(*face)
                            .map(|amount| super::TokenContribution {
                                face: *face,
                                amount,
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let modifier = contributions.iter().map(|row| row.amount).sum();
                let test = self.test.as_mut().expect("Token skill-test consumer");
                {
                    test.token_contributions = contributions;
                }
                test.revealed = resolved;
                test.ignored_tokens = ignored;
                test.cancelled_tokens = cancelled;
                test.modifier = modifier;
                test.eyes_match = eyes_match;
                let tester = test.tester;
                let harms = if symbols_applied {
                    vec![]
                } else {
                    super::scenario::symbol_consequences(
                        self,
                        tester,
                        &self.test.as_ref().expect("Test").revealed,
                        super::scenario::SymbolTiming::ApplySymbols,
                    )
                };
                self.frames
                    .push(Frame::AfterHarm(HarmContinuation::ApplySymbols));
                for harm in harms.into_iter().rev() {
                    self.queue_harm(
                        tester,
                        harm.damage,
                        harm.horror,
                        false,
                        "01104 Tablet",
                        HarmOrigin::EncounterCard,
                    )?;
                }
                if !symbols_applied {
                    let faces = self.test.as_ref().expect("Spell test").revealed.clone();
                    self.apply_spell_drawback(&faces)?;
                }
            }
        }
        Ok(())
    }

    fn apply_spell_drawback(&mut self, faces: &[TokenFace]) -> Result<(), RuleError> {
        if self
            .test
            .as_ref()
            .is_none_or(|test| test.spell_drawback_triggered)
        {
            return Ok(());
        }
        if let Some(drawback) = super::spells::reveal_drawback(self, faces) {
            let test = self.test.as_mut().expect("Spell test");
            test.spell_drawback_triggered = true;
            let tester = test.tester;
            if drawback.discard {
                self.frames
                    .push(Frame::Spell(super::spells::SpellDecision::MistsDiscard(
                        tester,
                    )));
            } else {
                self.queue_harm(
                    tester,
                    drawback.damage,
                    drawback.horror,
                    false,
                    drawback.source,
                    HarmOrigin::PlayerCard,
                )?;
            }
        }
        Ok(())
    }

    fn choose_turn_end(&mut self, id: &str) -> Result<(), RuleError> {
        let card_id = id[9..].parse::<u16>().expect("Threat identity");
        let Some(Frame::TurnEnd {
            investigator,
            pending,
        }) = self.frames.last_mut()
        else {
            unreachable!()
        };
        let actor = *investigator;
        pending.retain(|id| *id != card_id);
        let Some(card) = self.players[actor.index()]
            .threats
            .iter()
            .find(|card| card.id == card_id)
        else {
            return Ok(());
        };
        match card.code.as_str() {
            "02038" => self.queue_harm(actor, 1, 0, true, "02038", HarmOrigin::EncounterCard)?,
            "02039" => self.queue_harm(actor, 0, 1, true, "02039", HarmOrigin::EncounterCard)?,
            "01164" => self.begin_test(
                TestOperation::DiscardThreat {
                    owner: actor,
                    card: card_id,
                },
                actor,
                Skill::Willpower,
                3,
            )?,
            "60403" => super::tokens::begin_dark_future(self, actor, card_id)?,
            _ => unreachable!("Offered turn-end source"),
        }
        Ok(())
    }

    /// Conservative Gathering-specific Undo barriers. Deck changes catch draws
    /// from already shuffled decks; fixture cursor changes catch deterministic
    /// token witnesses that do not advance the ordinary random generator.
    pub(super) fn undo_barrier_since(&self, prior: &Self) -> bool {
        self.random != prior.random
            || self.token_cursor != prior.token_cursor
            || self.encounter_deck != prior.encounter_deck
            || self
                .players
                .iter()
                .zip(&prior.players)
                .any(|(now, before)| now.deck != before.deck)
            || self.act != prior.act
            || self.agenda != prior.agenda
            || self.scenario.outcome.is_some()
            || prior.scenario.outcome.is_some()
            || self.scenario.lita_in_play != prior.scenario.lita_in_play
            || self.scenario.priest_spawned != prior.scenario.priest_spawned
            || self.scenario.locations.iter().any(|location| {
                location.revealed
                    && !prior
                        .scenario
                        .locations
                        .iter()
                        .any(|previous| previous.code == location.code && previous.revealed)
            })
            || super::backpack::searched_cards(self) != super::backpack::searched_cards(prior)
            || super::encounter_search::visible_cards(self)
                != super::encounter_search::visible_cards(prior)
            || self.tokens.as_ref().is_some_and(|tokens| {
                !tokens.revealed.is_empty()
                    && prior
                        .tokens
                        .as_ref()
                        .is_none_or(|previous| previous.revealed != tokens.revealed)
            })
            || self.premonition != prior.premonition
            || self.frames.iter().any(|frame| {
                matches!(
                    frame,
                    Frame::Scenario(
                        ScenarioDecision::AgendaOne
                            | ScenarioDecision::AgendaTwoSearch
                            | ScenarioDecision::Ending
                            | ScenarioDecision::LitaRecipient
                    )
                ) && !prior.frames.contains(frame)
            })
    }

    pub(super) fn draw_token(&mut self) -> Result<TokenFace, RuleError> {
        if self.bag.is_empty() {
            return Err(RuleError("No chaos token remains available".into()));
        }
        let index = if let Some(face) = self.fixture_tokens.get(self.token_cursor).copied() {
            self.token_cursor += 1;
            self.bag
                .iter()
                .position(|token| *token == face)
                .ok_or_else(|| RuleError("Verification token absent from bag".into()))?
        } else {
            self.random_index(self.bag.len())
        };
        Ok(self.bag.remove(index))
    }

    pub(super) fn set_fixture_tokens(&mut self, tokens: Vec<TokenFace>) {
        self.fixture_tokens = tokens;
        self.token_cursor = 0;
    }

    fn token_modifier(&self, token: TokenFace) -> Result<i32, RuleError> {
        let tester = self.test.as_ref().expect("Token test").tester;
        Ok(match token {
            TokenFace::AutoFail => 0,
            TokenFace::Skull => -i32::from(
                self.scenario
                    .ghouls_at(&self.players[tester.index()].location),
            ),
            TokenFace::Cultist => -1,
            TokenFace::Tablet => -2,
            TokenFace::ElderSign => {
                if tester == Investigator::Tony {
                    2
                } else {
                    1
                }
            }
            token => token
                .modifier()
                .ok_or_else(|| RuleError("Token consequence is not implemented".into()))?,
        })
    }

    fn investigation_choices(&self) -> Vec<Choice> {
        if self.phase != Phase::Investigation || self.scenario.outcome.is_some() {
            return vec![];
        }
        let Some(actor) = self.actor else {
            return self
                .players
                .iter()
                .filter(|p| !p.turn_finished && !p.defeated && !p.resigned)
                .map(|p| {
                    choice(
                        &format!("turn:{}", p.investigator.id()),
                        &format!("Begin {}'s turn", p.investigator.name()),
                    )
                })
                .collect();
        };
        let player = &self.players[actor.index()];
        let mut actions = vec![
            choice("resource", "Gain 1 resource"),
            choice("draw", "Draw 1 card"),
        ];
        if player.location == "01115"
            && self
                .scenario
                .location("01115")
                .is_some_and(|location| location.revealed)
        {
            actions.push(choice("resign", "Resign: leave the house"));
        }
        if self.scenario.location(&player.location).is_some() {
            actions.push(choice("investigate", "Investigate this location"));
        }
        for location in &self.scenario.locations {
            if self.scenario.can_move(&player.location, &location.code) {
                actions.push(choice(
                    &format!("move:{}", location.code),
                    &format!("Move to {}", location.code),
                ));
            }
        }
        for enemy in &self.scenario.enemies {
            if enemy.location != player.location {
                continue;
            }
            if enemy.card.code != "06012" || enemy.engaged.is_some() {
                actions.push(choice(
                    &format!("fight:{}", enemy.card.id),
                    &format!("Fight {}", enemy.card.title),
                ));
                for asset in &player.assets {
                    if super::tony_cards::fight_operation(self, actor, asset.card.id, enemy.card.id)
                        .is_ok()
                    {
                        actions.push(choice(
                            &format!("fight:{}:weapon:{}", enemy.card.id, asset.card.id),
                            &format!("Fight {} with {}", enemy.card.title, asset.card.title),
                        ));
                    }
                }
            }
            if enemy.engaged != Some(actor) {
                actions.push(choice(
                    &format!("engage:{}", enemy.card.id),
                    &format!("Engage {}", enemy.card.title),
                ));
            }
            if enemy.engaged == Some(actor) {
                actions.push(choice(
                    &format!("evade:{}", enemy.card.id),
                    &format!("Evade {}", enemy.card.title),
                ));
            }
        }
        if actor == Investigator::Patrice {
            for card in &player.hand {
                if card.code == "06017" {
                    actions.push(choice(
                        &format!("fight:{}", card.id),
                        "Fight Watcher from Another Dimension in your hand",
                    ));
                    actions.push(choice(
                        &format!("evade:{}", card.id),
                        "Evade Watcher from Another Dimension in your hand",
                    ));
                }
            }
        }
        actions.extend(super::player_cards::play_choices(self, actor));
        actions.extend(super::player_cards::asset_action_choices(self, actor));
        actions.extend(super::encounter_search::action_choices(self, actor));
        actions.extend(super::spells::action_choices(self, actor));
        if self.scenario.lita_in_play
            && super::scenario::lita_controller(self).is_none()
            && player.location == "01115"
        {
            actions.push(choice(
                "parley:lita",
                "Parley with Lita Chantler: test intellect (4) to take control",
            ));
        }
        for owner in &self.players {
            if owner.location != player.location {
                continue;
            }
            for card in &owner.threats {
                if matches!(card.code.as_str(), "02038" | "02039" | "05041") {
                    actions.push(choice(
                        &format!("remove-threat:{}:{}", owner.investigator.id(), card.id),
                        &format!("Spend two actions to discard {}", card.title),
                    ));
                }
            }
        }
        let mut choices = Vec::new();
        for action in actions {
            if super::ethereal_form::is_ethereal(self, actor)
                && matches!(
                    super::encounter_search::action_designator(&action.id)
                        .or_else(|| action.id.split(':').next()),
                    Some("fight" | "engage")
                )
            {
                continue;
            }
            let cost = self.action_cost(actor, &action.id);
            if player.ordinary_actions >= cost {
                choices.push(choice(
                    &format!("action:ordinary:{}", action.id),
                    &format!("Spend an action: {}", action.label),
                ));
            }
            if actor == Investigator::Tony
                && !player.restricted_tony_action_used
                && player.ordinary_actions >= cost - 1
                && (matches!(
                    super::encounter_search::action_designator(&action.id),
                    Some("fight" | "engage")
                ) || action.id.starts_with("fight:")
                    || action.id.starts_with("engage:"))
            {
                let enemy_id = super::encounter_search::action_enemy(&action.id).or_else(|| {
                    action
                        .id
                        .split(':')
                        .nth(1)
                        .and_then(|id| id.parse::<u16>().ok())
                });
                if self
                    .scenario
                    .enemies
                    .iter()
                    .any(|enemy| Some(enemy.card.id) == enemy_id && enemy.bounties > 0)
                {
                    choices.push(choice(
                        &format!("action:tony:{}", action.id),
                        &format!("Spend Tony's additional bounty action: {}", action.label),
                    ));
                }
            }
        }
        if super::scenario::available_act_payment(self, false).is_some() {
            choices.push(choice(
                "act:advance",
                "Spend the required group clues to advance the act",
            ));
        }
        choices.push(choice("turn:end", "End this investigator's turn"));
        choices.extend(self.player_window_choices());
        choices
    }

    fn action_cost(&self, actor: Investigator, action: &str) -> u8 {
        let player = &self.players[actor.index()];
        let base = if action.starts_with("remove-threat:") {
            2
        } else {
            1
        };
        base + if !player.frozen_action_used
            && matches!(
                super::encounter_search::action_designator(action)
                    .or_else(|| action.split(':').next()),
                Some("move" | "fight" | "evade")
            ) {
            player
                .threats
                .iter()
                .filter(|card| card.code == "01164")
                .count() as u8
        } else {
            0
        }
    }

    fn choose_investigation(&mut self, id: &str) -> Result<(), RuleError> {
        if id == "act:advance" {
            self.frames.push(Frame::Scenario(
                super::scenario::available_act_payment(self, false).expect("Offered act payment"),
            ));
            return Ok(());
        }
        if id == "turn:end" {
            let actor = self.actor.expect("Current turn");
            let pending = self.players[actor.index()]
                .threats
                .iter()
                .filter(|card| matches!(card.code.as_str(), "02038" | "02039" | "01164" | "60403"))
                .map(|card| card.id)
                .collect();
            self.frames.push(Frame::TurnEnd {
                investigator: actor,
                pending,
            });
            return Ok(());
        }
        if let Some(actor) = id.strip_prefix("turn:") {
            self.actor = Some(
                Investigator::ALL
                    .into_iter()
                    .find(|candidate| candidate.id() == actor)
                    .expect("Offered actor"),
            );
            self.players[self.actor.expect("Selected turn").index()].actions_taken_this_turn = 0;
            return Ok(());
        }
        let actor = self.actor.expect("Action turn");
        let parts: Vec<_> = id.splitn(3, ':').collect();
        let action = parts[2];
        let cost = self.action_cost(actor, action);
        // All costs and effects are inside Session's clone-and-install transaction.
        if parts[1] == "ordinary" {
            self.players[actor.index()].ordinary_actions -= cost;
        } else {
            self.players[actor.index()].restricted_tony_action_used = true;
            self.players[actor.index()].ordinary_actions -= cost - 1;
        }
        if cost > 1 && !action.starts_with("remove-threat:") {
            self.players[actor.index()].frozen_action_used = true;
        }
        self.action_in_progress = Some(actor);
        let spell = if action.contains(":spell:") {
            Some(super::spells::pay_action(self, actor, action)?)
        } else {
            None
        };
        let event = if action.starts_with("play:tony:") {
            Some(super::encounter_search::pay_action(self, actor, action)?)
        } else {
            None
        };
        let asset = if spell.is_none() && event.is_none() && action.starts_with("play:") {
            Some(super::player_cards::pay_asset(self, actor, action)?)
        } else {
            None
        };
        if !asset
            .as_ref()
            .is_some_and(|asset| asset.card.code == "07029")
            && super::encounter_search::provokes_opportunity(action).unwrap_or_else(|| {
                super::scenario::provokes_opportunity(action.split(':').next().expect("Action"))
            })
        {
            let pending = super::scenario::opportunity_attacks(self, actor)
                .into_iter()
                .map(|attack| attack.enemy)
                .collect::<Vec<_>>();
            if !pending.is_empty() {
                self.frames.push(Frame::Opportunity {
                    investigator: actor,
                    action: action.into(),
                    pending,
                    spell,
                    asset,
                    event,
                });
                return Ok(());
            }
        }
        if let Some(spell) = spell {
            super::spells::resolve_action(self, spell)
        } else if let Some(event) = event {
            super::encounter_search::resolve_action(self, event)
        } else if let Some(asset) = asset {
            self.finish_asset(asset)
        } else {
            self.perform_action(actor, action)
        }
    }

    fn perform_action(&mut self, actor: Investigator, action: &str) -> Result<(), RuleError> {
        if action.contains(":cane:") {
            super::player_cards::activate_asset_action(self, actor, action)?;
        } else if action == "resign" {
            super::elimination::begin(self, actor, super::elimination::Cause::Resignation)?;
        } else if action == "resource" {
            self.players[actor.index()].resources += 1;
        } else if action == "draw" {
            self.draw_player(actor, 1)?;
        } else if action == "investigate" {
            let location = self.players[actor.index()].location.clone();
            let difficulty = self.scenario.shroud(&location).expect("Current location");
            self.begin_test(
                TestOperation::Investigate { location, clues: 1 },
                actor,
                Skill::Intellect,
                i32::from(difficulty),
            )?;
        } else if let Some(location) = action.strip_prefix("move:") {
            self.move_investigator(actor, location)?;
        } else if action == "parley:lita" {
            self.begin_test(TestOperation::LitaParley, actor, Skill::Intellect, 4)?;
        } else if action.starts_with("remove-threat:") {
            let parts: Vec<_> = action.split(':').collect();
            let owner = Investigator::ALL
                .into_iter()
                .find(|who| who.id() == parts[1])
                .expect("Threat owner");
            let id: u16 = parts[2].parse().expect("Threat");
            let player = &mut self.players[owner.index()];
            let index = player
                .threats
                .iter()
                .position(|card| card.id == id)
                .expect("Threat");
            player.discard.push(player.threats.remove(index));
        } else {
            let details: Vec<_> = action.split(':').collect();
            let enemy_id: u16 = details[1].parse().expect("Enemy action");
            let code = self
                .scenario
                .enemies
                .iter()
                .find(|enemy| enemy.card.id == enemy_id)
                .map(|enemy| enemy.card.code.clone())
                .or_else(|| {
                    self.players[actor.index()]
                        .hand
                        .iter()
                        .find(|card| card.id == enemy_id && card.code == "06017")
                        .map(|card| card.code.clone())
                })
                .expect("Action enemy");
            let values = super::scenario::enemy_definition(&code);
            let card_values = cards::definition(&code).and_then(|definition| definition.enemy);
            match details[0] {
                "engage" => {
                    self.scenario
                        .enemies
                        .iter_mut()
                        .find(|enemy| enemy.card.id == enemy_id)
                        .expect("Enemy")
                        .engaged = Some(actor)
                }
                "fight" if details.len() > 2 => super::tony_cards::activate_fight(
                    self,
                    actor,
                    details[3].parse().expect("Weapon"),
                    enemy_id,
                )?,
                "fight" => self.begin_test(
                    TestOperation::Fight {
                        enemy: enemy_id,
                        weapon: None,
                        skill: Skill::Combat,
                        skill_bonus: 0,
                        damage: 1,
                    },
                    actor,
                    Skill::Combat,
                    i32::from(
                        values
                            .map(|v| v.combat)
                            .or(card_values.map(|v| v.combat))
                            .expect("Combat"),
                    ),
                )?,
                "evade" => self.begin_test(
                    TestOperation::Evade { enemy: enemy_id },
                    actor,
                    Skill::Agility,
                    i32::from(
                        values
                            .map(|v| v.evade)
                            .or(card_values.map(|v| v.evade))
                            .expect("Evade"),
                    ),
                )?,
                _ => return Err(RuleError("Unsupported action".into())),
            }
        }
        Ok(())
    }

    fn finish_asset(
        &mut self,
        prepared: super::player_cards::PreparedAsset,
    ) -> Result<(), RuleError> {
        if let Some(pending) = super::player_cards::resolve_asset(self, prepared)? {
            self.frames.push(Frame::AssetPlacement(pending));
        }
        Ok(())
    }

    pub(super) fn suspend_for_resolution(&mut self) {
        if self.stopped.is_none() {
            self.stopped = Some(StoppedState {
                test: self.test.take(),
                frames: std::mem::take(&mut self.frames),
                tokens: self.tokens.take(),
                action_in_progress: self.action_in_progress.take(),
            });
        }
        self.actor = None;
    }

    pub(super) fn clear_eliminated_continuations(
        &mut self,
        who: Investigator,
    ) -> Result<(), RuleError> {
        super::tokens::eliminate(self, who)?;
        let abort_test = self.test.as_ref().is_some_and(|test| test.tester == who);
        let mut abandoned = vec![];
        if abort_test {
            abandoned.push(self.test.take().expect("Eliminated tester"));
        } else if let Some(test) = &mut self.test {
            let mut index = 0;
            while index < test.committed.len() {
                if test.committed[index].owner == who {
                    self.players[who.index()]
                        .removed
                        .push(test.committed.remove(index).card);
                } else {
                    index += 1;
                }
            }
        }
        let frames = std::mem::take(&mut self.frames);
        for frame in frames {
            match frame {
                Frame::TestCleanup(test) if test.tester == who => abandoned.push(*test),
                Frame::TestResults(_)
                | Frame::TestCard(_)
                | Frame::SuccessWindow
                | Frame::AfterHarm(HarmContinuation::ApplySymbols)
                | Frame::ResumeTokens
                | Frame::Combat(_)
                    if abort_test => {}
                Frame::AfterTest(state) if state.test.tester == who => {}
                Frame::RetryEvent { owner, event } if owner == who => {
                    self.players[who.index()].removed.push(event)
                }
                Frame::AssetEntry(entry) if entry.actor() == who => {}
                Frame::Idol(decision) if decision.owner() == who => {}
                Frame::CryptChill(decision) if decision.owner == who => {}
                Frame::GrantedAction { owner } if owner == who => {}
                Frame::Peter { owner } if owner == who => {}
                Frame::Harm(harm) if harm.investigator == who => {}
                Frame::GainResources { investigator, .. } | Frame::TurnEnd { investigator, .. }
                    if investigator == who => {}
                Frame::Revelation(revelation) if revelation.drawn_by == who => {
                    if revelation.card.weakness {
                        self.players[who.index()].removed.push(revelation.card);
                    } else {
                        self.encounter_discard.push(revelation.card);
                    }
                }
                Frame::DiscardCards {
                    owner,
                    mut remaining,
                    mut ordered,
                } if owner == who => {
                    self.players[who.index()].removed.append(&mut remaining);
                    self.players[who.index()].removed.append(&mut ordered);
                }
                Frame::AssetPlacement(prepared) if prepared.actor == who => {
                    self.players[who.index()].removed.push(prepared.card)
                }
                Frame::Opportunity {
                    investigator,
                    action,
                    pending,
                    spell,
                    asset,
                    event,
                } if investigator == who => {
                    if let Some(asset) = asset {
                        self.players[who.index()].removed.push(asset.card);
                    }
                    if let Some(spell) = spell {
                        super::spells::cancel_action(self, spell);
                    }
                    if let Some(event) = event {
                        super::encounter_search::cancel_action(self, event);
                    }
                    self.frames.push(Frame::Opportunity {
                        investigator,
                        action,
                        pending,
                        spell: None,
                        asset: None,
                        event: None,
                    });
                }
                other => self.frames.push(other),
            }
        }
        for test in abandoned {
            super::tokens::return_test_tokens(
                self,
                test.revealed,
                test.ignored_tokens,
                test.cancelled_tokens,
            );
            for committed in test.committed {
                let owner = &mut self.players[committed.owner.index()];
                if owner.defeated || owner.resigned {
                    owner.removed.push(committed.card);
                } else {
                    self.frames.push(Frame::DiscardCards {
                        owner: committed.owner,
                        remaining: vec![committed.card],
                        ordered: vec![],
                    });
                }
            }
            if let TestOperation::CancelRevelation { event, owner, .. } = test.operation
                && !test.is_retry
            {
                if self.players[owner.index()].defeated || self.players[owner.index()].resigned {
                    self.players[owner.index()].removed.push(event);
                } else {
                    self.players[owner.index()].discard.push(event);
                }
            }
        }
        Ok(())
    }

    pub(super) fn enable_fixture_turn(&mut self, investigator: Investigator) {
        self.ordinary_flow = true;
        self.phase = Phase::Investigation;
        self.actor = Some(investigator);
    }

    pub(super) fn complete_action(&mut self, actor: Investigator) {
        self.players[actor.index()].completed_actions_this_phase += 1;
        if self.is_actual_turn(actor) {
            self.players[actor.index()].actions_taken_this_turn += 1;
        }
    }

    pub(super) fn is_actual_turn(&self, actor: Investigator) -> bool {
        if let Some((previous_actor, previous_phase)) = self.frames.iter().find_map(|frame| {
            if let Frame::RestoreTurn {
                previous_actor,
                previous_phase,
            } = frame
            {
                Some((*previous_actor, *previous_phase))
            } else {
                None
            }
        }) {
            return previous_phase == Phase::Investigation && previous_actor == Some(actor);
        }
        self.phase == Phase::Investigation && self.actor == Some(actor)
    }

    pub(super) fn begin_granted_action(&mut self, owner: Investigator) {
        let boundary = self
            .frames
            .iter()
            .rposition(|frame| matches!(frame, Frame::Phase(_)))
            .map_or(0, |index| index + 1);
        self.frames.insert(boundary, Frame::GrantedAction { owner });
    }

    fn granted_action_choices(&self, owner: Investigator) -> Vec<Choice> {
        if self.players[owner.index()].defeated || self.players[owner.index()].resigned {
            return vec![];
        }
        let mut projection = self.clone();
        projection.frames.push(Frame::RestoreTurn {
            previous_actor: self.actor,
            previous_phase: self.phase,
        });
        projection.actor = Some(owner);
        projection.phase = Phase::Investigation;
        projection.players[owner.index()].ordinary_actions += 1;
        projection
            .investigation_choices()
            .into_iter()
            .filter_map(|mut choice| {
                let action = choice.id.strip_prefix("action:ordinary:")?.to_owned();
                choice.id = format!("granted:{action}");
                choice.label = format!("Use the granted action: {}", choice.label);
                Some(choice)
            })
            .collect()
    }

    pub(super) fn move_investigator(
        &mut self,
        actor: Investigator,
        location: &str,
    ) -> Result<(), RuleError> {
        self.scenario.reveal(location);
        self.players[actor.index()].location = location.into();
        for enemy in &mut self.scenario.enemies {
            if enemy.engaged == Some(actor) {
                enemy.location = location.into();
            }
        }
        for enemy in self
            .scenario
            .enemies
            .iter()
            .map(|enemy| enemy.card.id)
            .collect::<Vec<_>>()
        {
            self.check_engagement(enemy);
        }
        let (damage, horror) = super::scenario::location_entry_harm(location);
        self.queue_harm(
            actor,
            damage,
            horror,
            false,
            location,
            HarmOrigin::EncounterCard,
        )?;
        self.capture_presentation(format!(
            "{} enters {}.",
            actor.name(),
            content::title(location).unwrap_or(location)
        ));
        Ok(())
    }

    pub(super) fn scenario_step(
        &mut self,
        step: super::scenario::ScenarioStep,
    ) -> Result<(), RuleError> {
        use super::scenario::ScenarioStep;
        match step {
            ScenarioStep::Continue | ScenarioStep::Finished => {}
            ScenarioStep::EnemyEntered(enemy) => {
                if !super::tony_cards::bounty_choices(self, enemy).is_empty() {
                    self.frames
                        .push(Frame::Tony(super::tony_cards::TonyDecision::Bounties {
                            enemy,
                        }));
                }
                self.check_engagement(enemy);
            }
            ScenarioStep::CheckEngagement(enemy) => self.check_engagement(enemy),
            ScenarioStep::RoundEnded => {
                self.round += 1;
                super::phases::begin_mythos(self);
                super::ethereal_form::expire(self);
            }
            ScenarioStep::LeadHorror => {
                self.frames
                    .push(Frame::AfterHarm(HarmContinuation::AgendaOne));
                self.queue_harm(self.lead, 0, 2, false, "01105", HarmOrigin::EncounterCard)?;
            }
        }
        Ok(())
    }

    fn defeat_remaining(&mut self) -> Result<(), RuleError> {
        let mut gained = [0; 3];
        for who in Investigator::ALL {
            if !self.players[who.index()].resigned && !self.players[who.index()].defeated {
                gained[who.index()] = 1;
                super::elimination::begin(self, who, super::elimination::Cause::Agenda)?;
            }
        }
        if self.scenario.outcome.is_none() {
            super::scenario::finish_resolution(self, super::scenario::Resolution::NoResolution);
        }
        if let Some(outcome) = &mut self.scenario.outcome {
            outcome.physical_trauma_gained = gained;
        }
        Ok(())
    }

    pub(super) fn scenario_consequence(
        &mut self,
        consequence: super::scenario::ScenarioConsequence,
    ) -> Result<(), RuleError> {
        use super::scenario::ScenarioConsequence;
        match consequence {
            ScenarioConsequence::AgendaOneChoice => self
                .frames
                .push(Frame::Scenario(ScenarioDecision::AgendaOne)),
            ScenarioConsequence::AgendaTwoGhoulSearch => self
                .frames
                .push(Frame::Scenario(ScenarioDecision::AgendaTwoSearch)),
            ScenarioConsequence::ResolutionThree => {
                super::scenario::finish_resolution(self, super::scenario::Resolution::Overrun)
            }
            ScenarioConsequence::DefeatNonResigned => self.defeat_remaining()?,
        }
        Ok(())
    }

    pub(super) fn queue_harm(
        &mut self,
        investigator: Investigator,
        damage: u8,
        horror: u8,
        direct: bool,
        source: &str,
        origin: HarmOrigin,
    ) -> Result<(), RuleError> {
        if damage == 0 && horror == 0 {
            return Ok(());
        }
        self.frames.push(Frame::Harm(HarmAssignment {
            investigator,
            damage,
            horror,
            direct,
            source: source.into(),
            origin,
            allocations: vec![],
            prevention_closed: false,
        }));
        Ok(())
    }

    fn harm_choices(&self, harm: &HarmAssignment) -> Vec<Choice> {
        let damage_assigned: u8 = harm.allocations.iter().map(|(_, damage, _)| *damage).sum();
        let horror_assigned: u8 = harm.allocations.iter().map(|(_, _, horror)| *horror).sum();
        if damage_assigned == harm.damage && horror_assigned == harm.horror {
            let mut choices = super::harm_cards::prevention_choices(self, harm);
            choices.push(choice(
                "harm:confirm",
                "Apply the assigned damage and horror simultaneously",
            ));
            return choices;
        }
        let mut choices = Vec::new();
        for (kind, remaining) in [
            ("damage", harm.damage - damage_assigned),
            ("horror", harm.horror - horror_assigned),
        ] {
            if remaining == 0 {
                continue;
            }
            choices.push(choice(
                &format!("harm:{kind}:investigator"),
                &format!("Assign 1 {kind} to {}", harm.investigator.name()),
            ));
            if harm.direct {
                continue;
            }
            for asset in &self.players[harm.investigator.index()].assets {
                let definition = super::player_cards::asset_definition(&asset.card.code)
                    .expect("Asset metadata");
                let (capacity, existing) = if kind == "damage" {
                    (definition.health, asset.damage)
                } else {
                    (definition.sanity, asset.horror)
                };
                let selected: u8 = harm
                    .allocations
                    .iter()
                    .filter(|(target, _, _)| *target == Some(asset.card.id))
                    .map(|(_, damage, horror)| if kind == "damage" { *damage } else { *horror })
                    .sum();
                if capacity.is_some_and(|capacity| existing + selected < capacity) {
                    choices.push(choice(
                        &format!("harm:{kind}:{}", asset.card.id),
                        &format!("Assign 1 {kind} to {}", asset.card.title),
                    ));
                }
            }
        }
        choices
    }

    fn choose_harm(&mut self, id: &str) -> Result<(), RuleError> {
        if id.starts_with("deny:") || id.starts_with("idol:") {
            return super::harm_cards::choose_prevention(self, id);
        }
        if id == "harm:confirm" {
            let Some(Frame::Harm(harm)) = self.frames.pop() else {
                unreachable!()
            };
            let player = &mut self.players[harm.investigator.index()];
            let mut removed_lita = None;
            let mut defeated_assets = Vec::new();
            let investigator_took_harm = harm
                .allocations
                .iter()
                .any(|(target, damage, horror)| target.is_none() && (*damage > 0 || *horror > 0));
            for (target, damage, horror) in harm.allocations {
                if let Some(id) = target {
                    let asset = player
                        .assets
                        .iter_mut()
                        .find(|asset| asset.card.id == id)
                        .expect("Assigned asset");
                    asset.damage += damage;
                    asset.horror += horror;
                } else {
                    player.damage += damage;
                    player.horror += horror;
                }
            }
            let mut index = 0;
            while index < player.assets.len() {
                let asset = &player.assets[index];
                let definition = super::player_cards::asset_definition(&asset.card.code)
                    .expect("Asset metadata");
                if definition
                    .health
                    .is_some_and(|health| asset.damage >= health)
                    || definition
                        .sanity
                        .is_some_and(|sanity| asset.horror >= sanity)
                {
                    let asset = player.assets.remove(index);
                    if asset.card.code == "01117" {
                        removed_lita = Some(asset.card);
                    } else {
                        defeated_assets.push(asset.card);
                    }
                } else {
                    index += 1;
                }
            }
            let departed_assets: Vec<_> = defeated_assets
                .iter()
                .map(|card| card.id)
                .chain(removed_lita.iter().map(|card| card.id))
                .collect();
            let lethal = player.damage >= player.health || player.horror >= player.sanity;
            if investigator_took_harm && !lethal {
                for asset in &mut player.assets {
                    if asset.card.code == "10111" {
                        asset.uses = (asset.uses + 1).min(2);
                    }
                }
            }
            if lethal || player.defeated || player.resigned {
                player.removed.extend(defeated_assets);
            } else if defeated_assets.len() > 1 {
                self.frames.push(Frame::DiscardCards {
                    owner: harm.investigator,
                    remaining: defeated_assets,
                    ordered: vec![],
                });
            } else {
                player.discard.extend(defeated_assets);
            }
            if let Some(card) = removed_lita {
                super::scenario::remove_lita(self, card);
            }
            for asset in departed_assets {
                super::tony_effects::asset_left_play(self, asset);
            }
            self.history.push(format!(
                "Resolve {} damage and {} horror from {} with the selected assignments.",
                harm.damage, harm.horror, harm.source
            ));
            self.capture_presentation(format!(
                "{} resolves {} damage and {} horror from {}.",
                harm.investigator.name(),
                harm.damage,
                harm.horror,
                harm.source
            ));
            if lethal {
                super::elimination::begin(
                    self,
                    harm.investigator,
                    super::elimination::Cause::Harm,
                )?;
            }
        } else {
            let parts: Vec<_> = id.split(':').collect();
            let target = if parts[2] == "investigator" {
                None
            } else {
                Some(parts[2].parse().expect("Harm target"))
            };
            let Some(Frame::Harm(harm)) = self.frames.last_mut() else {
                unreachable!()
            };
            harm.allocations.push((
                target,
                u8::from(parts[1] == "damage"),
                u8::from(parts[1] == "horror"),
            ));
        }
        Ok(())
    }

    fn choose_discard_order(&mut self, id: &str) -> Result<(), RuleError> {
        let card_id = id
            .strip_prefix("discard-card:")
            .and_then(|id| id.parse::<u16>().ok())
            .ok_or_else(|| RuleError("Invalid discarded card identity.".into()))?;
        let Some(Frame::DiscardCards {
            remaining, ordered, ..
        }) = self.frames.last_mut()
        else {
            return Err(RuleError("No simultaneous card discard is pending.".into()));
        };
        let index = remaining
            .iter()
            .position(|card| card.id == card_id)
            .ok_or_else(|| RuleError("That card is not awaiting discard order.".into()))?;
        ordered.push(remaining.remove(index));
        if remaining.is_empty() {
            let Some(Frame::DiscardCards { owner, ordered, .. }) = self.frames.pop() else {
                unreachable!()
            };
            self.players[owner.index()].discard.extend(ordered);
            self.history.push(format!("{} orders the simultaneously discarded cards in the discard pile before the interrupted effect continues. [RR p9]",owner.name()));
        }
        Ok(())
    }

    pub(super) fn check_engagement(&mut self, enemy: u16) {
        let candidates = super::scenario::engagement_candidates(self, enemy);
        match candidates.as_slice() {
            [owner] => {
                self.scenario
                    .enemies
                    .iter_mut()
                    .find(|target| target.card.id == enemy)
                    .expect("Enemy")
                    .engaged = Some(*owner)
            }
            [] => {}
            _ => self
                .frames
                .push(Frame::Scenario(ScenarioDecision::Engagement {
                    enemy,
                    investigators: candidates,
                })),
        }
    }

    pub(super) fn player_window_choices(&self) -> Vec<Choice> {
        let mut choices = super::tokens::premonition_choices(self);
        choices.extend(super::test_boosts::choices(self));
        if let Some(actor) = self.actor {
            choices.extend(super::player_cards::fast_choices(self, actor));
        }
        for owner in &self.players {
            if owner.defeated || owner.resigned {
                continue;
            }
            for asset in &owner.assets {
                if asset.card.code != "06016" || !asset.ready {
                    continue;
                }
                for discarded in &owner.hand {
                    if discarded.weakness {
                        continue;
                    }
                    for target in &self.players {
                        if target.location != owner.location || target.defeated || target.resigned {
                            continue;
                        }
                        for reward in ["draw", "resource"] {
                            choices.push(choice(
                                &format!(
                                    "violin:{}:discard:{}:target:{}:{reward}",
                                    asset.card.id,
                                    discarded.id,
                                    target.investigator.id()
                                ),
                                &format!(
                                    "Exhaust Patrice's Violin and discard {}: {} {}",
                                    discarded.title,
                                    target.investigator.name(),
                                    if reward == "draw" {
                                        "draws 1 card"
                                    } else {
                                        "gains 1 resource"
                                    }
                                ),
                            ));
                        }
                    }
                }
            }
        }
        choices
    }

    fn commit_choices(&self) -> Vec<Choice> {
        let Some(test) = &self.test else {
            return vec![];
        };
        let mut choices = Vec::new();
        let tester = &self.players[test.tester.index()];
        for owner in &self.players {
            if owner.location != tester.location || owner.defeated || owner.resigned {
                continue;
            }
            for card in &owner.hand {
                let selected = test.selected.contains(&card.id);
                if !selected && !self.can_commit(test, owner, card) {
                    continue;
                }
                choices.push(choice(
                    &format!("commit:{}", card.id),
                    &format!(
                        "{} {} from {}",
                        if selected { "Unselect" } else { "Commit" },
                        card.title,
                        owner.investigator.name()
                    ),
                ));
            }
        }
        choices
    }

    fn can_commit(&self, test: &SkillTest, owner: &PlayerState, card: &CardView) -> bool {
        if card.weakness {
            return false;
        }
        let Some(_) = cards::definition(&card.code) else {
            return false;
        };
        if card.code != "04201" && self.commit_icons(test, card) == 0 {
            return false;
        }
        if owner.investigator != test.tester
            && owner
                .hand
                .iter()
                .any(|card| test.selected.contains(&card.id))
        {
            return false;
        }
        if card.code == "02192"
            && (owner.investigator != test.tester
                || test.difficulty < i32::from(owner.base_skills[test.skill.index()]) + 2)
        {
            return false;
        }
        if matches!(card.code.as_str(), "04150" | "03233") && owner.investigator != test.tester {
            return false;
        }
        if matches!(
            card.code.as_str(),
            "04201" | "01093" | "01089" | "11097" | "60419"
        ) && self
            .players
            .iter()
            .flat_map(|p| &p.hand)
            .any(|other| other.code == card.code && test.selected.contains(&other.id))
        {
            return false;
        }
        true
    }

    fn lucky_choices(&self) -> Vec<Choice> {
        let Some(test) = &self.test else {
            return vec![];
        };
        let owner = &self.players[test.tester.index()];
        if owner.resources < 1
            || test.revealed.contains(&TokenFace::AutoFail)
            || owner.threats.iter().any(|card| card.code == "01165")
        {
            return vec![];
        }
        owner
            .hand
            .iter()
            .filter(|card| card.code == "01080")
            .map(|card| {
                choice(
                    &format!("lucky:{}", card.id),
                    "Pay 1 resource for Lucky!: +2 skill at this would-fail window",
                )
            })
            .collect()
    }

    pub(super) fn commit_icons(&self, test: &SkillTest, card: &CardView) -> i32 {
        let definition = cards::definition(&card.code).expect("Commitment metadata");
        let base = i32::from(definition.icons[test.skill.index()] + definition.icons[4]);
        if card.code == "04150" {
            return (base
                - self
                    .players
                    .iter()
                    .map(|p| i32::from(p.completed_actions_this_phase))
                    .sum::<i32>())
            .max(0);
        }
        if card.code == "11097" {
            return base
                + (test.revealed.len() + test.ignored_tokens.len() + test.cancelled_tokens.len())
                    .saturating_sub(1)
                    .min(3) as i32;
        }
        base
    }

    fn test_modifiers(&self, test: &SkillTest) -> TestModifiers {
        if let Some(modifiers) = &test.final_modifiers {
            return modifiers.clone();
        }
        let cards = test
            .committed
            .iter()
            .map(|entry| self.commit_icons(test, &entry.card))
            .sum();
        let operation_bonus = match test.operation {
            TestOperation::Fight { skill_bonus, .. } => skill_bonus,
            _ => 0,
        };
        TestModifiers {
            cards,
            other: test.bonus + operation_bonus + super::test_cards::skill_modifier(self, test),
            chaos: (!test.revealed.is_empty()).then_some(test.modifier),
            automatic_failure: test.revealed.contains(&TokenFace::AutoFail),
        }
    }

    pub(super) fn result(&self, test: &SkillTest) -> (i32, bool, i32) {
        if let Some(result) = test.final_result {
            return result;
        }
        let player = &self.players[test.tester.index()];
        let modifiers = self.test_modifiers(test);
        let total = if modifiers.automatic_failure {
            0
        } else {
            (super::test_cards::base_skill(self, test.tester, test.skill)
                + modifiers.cards
                + modifiers.chaos.unwrap_or(0)
                + modifiers.other)
                .max(0)
        };
        let vision = self.players.iter().any(|owner| {
            owner.location == player.location
                && owner.threats.iter().any(|card| card.code == "05041")
        });
        let success = !modifiers.automatic_failure
            && (total > test.difficulty || total == test.difficulty && !vision);
        (total, success, (test.difficulty - total).max(0))
    }

    pub(super) fn draw_player(
        &mut self,
        investigator: Investigator,
        count: usize,
    ) -> Result<(), RuleError> {
        super::draws::begin(self, investigator, count)
    }

    fn choose_result(&mut self, id: &str) -> Result<(), RuleError> {
        let index = id
            .strip_prefix("result:")
            .expect("Result choice")
            .parse::<usize>()
            .expect("Effect index");
        let Some(Frame::TestResults(effects)) = self.frames.last_mut() else {
            unreachable!()
        };
        let effect = effects.remove(index);
        let tester = self.test.as_ref().expect("Test").tester;
        match effect {
            TestResultEffect::OldKeyring { owner, asset } => {
                super::test_cards::spend_key(self, owner, asset)?
            }
            TestResultEffect::Resourceful { owner, card } => {
                super::test_cards::start_resourceful(self, owner, card)?
            }
            TestResultEffect::Resources {
                owner,
                amount,
                source,
            } => {
                if !self.players[owner.index()].defeated && !self.players[owner.index()].resigned {
                    self.players[owner.index()].resources += amount;
                    self.history.push(format!(
                        "{} gains {amount} resources from {source} during test results.",
                        owner.name()
                    ));
                }
            }
            TestResultEffect::Operation => self.apply_test_operation()?,
            TestResultEffect::TakeHeart => {
                self.frames.push(Frame::GainResources {
                    investigator: tester,
                    amount: 2,
                });
                self.draw_player(tester, 2)?;
                self.history.push("Take Heart resolves on the failed test: draw two cards and gain two resources. [04201]".into());
            }
            TestResultEffect::Draw { owner } => self.draw_player(owner, 1)?,
            TestResultEffect::Horror {
                amount,
                source,
                origin,
            } => self.queue_harm(tester, 0, amount, false, &source, origin)?,
            TestResultEffect::Damage {
                amount,
                source,
                origin,
            } => self.queue_harm(tester, amount, 0, false, &source, origin)?,
        }
        Ok(())
    }

    pub(super) fn finish_test(&mut self) -> Result<(), RuleError> {
        let test = self.test.as_ref().expect("Completing test");
        let (total, success, failure_margin) = self.result(test);
        if test.final_result.is_none() {
            let mut modifiers = self.test_modifiers(test);
            // ST6 fixes the contribution even if a completed reveal retained
            // no physical tokens; an established zero is no longer pending.
            modifiers.chaos = Some(test.modifier);
            let breakdown = super::test_report::breakdown(self, test);
            let test = self.test.as_mut().expect("Completing test");
            test.final_breakdown = Some(Box::new(breakdown));
            test.final_result = Some((total, success, failure_margin));
            test.final_modifiers = Some(modifiers);
        }
        if !self.test.as_ref().expect("Completing test").lita_checked {
            self.test.as_mut().expect("Completing test").lita_checked = true;
            let test = self.test.as_ref().expect("Completing test");
            if success
                && let TestOperation::Fight { enemy, .. } = test.operation
                && let Some(controller) =
                    super::combat::lita_damage_controller(self, test.tester, enemy)
            {
                self.test.as_mut().expect("Completing test").window = Window::Results;
                self.frames
                    .push(Frame::Combat(super::combat::CombatDecision::LitaDamage {
                        controller,
                    }));
                return Ok(());
            }
        }
        let test = self.test.as_ref().expect("Completing test");
        if !test.results_started {
            if !test.tony.success_window_closed {
                if !self.outcome_reaction_choices().is_empty() {
                    self.test.as_mut().expect("Test").window = Window::AfterSuccess;
                    self.frames.push(Frame::SuccessWindow);
                    return Ok(());
                }
                super::tony_effects::close_success_window(self);
            }
            let test = self.test.as_ref().expect("Test");
            let mut effects = super::test_cards::result_effects(self, test);
            effects.extend(super::tony_effects::result_effects(self, test));
            if !success {
                for harm in super::scenario::symbol_consequences(
                    self,
                    test.tester,
                    &test.revealed,
                    super::scenario::SymbolTiming::FailedResults,
                ) {
                    effects.push(TestResultEffect::Horror {
                        amount: harm.horror,
                        source: "01104 Cultist".into(),
                        origin: HarmOrigin::EncounterCard,
                    });
                }
                if test.committed.iter().any(|card| card.card.code == "04201") {
                    effects.push(TestResultEffect::TakeHeart);
                }
                if matches!(test.operation, TestOperation::RottingRemains { .. })
                    && failure_margin > 0
                {
                    effects.push(TestResultEffect::Horror {
                        amount: failure_margin as u8,
                        source: "01163".into(),
                        origin: HarmOrigin::EncounterCard,
                    });
                }
                if matches!(test.operation, TestOperation::GraspingHands { .. })
                    && failure_margin > 0
                {
                    effects.push(TestResultEffect::Damage {
                        amount: failure_margin as u8,
                        source: "01162".into(),
                        origin: HarmOrigin::EncounterCard,
                    });
                }
            } else {
                for card in &test.committed {
                    if card.card.code == "01089" {
                        effects.push(TestResultEffect::Draw { owner: card.owner });
                    }
                }
            }
            let operation_applies = match test.operation {
                TestOperation::Fight { enemy, .. } | TestOperation::Evade { enemy }
                    if self.players[test.tester.index()]
                        .hand
                        .iter()
                        .any(|card| card.id == enemy && card.code == "06017") =>
                {
                    true
                }
                TestOperation::RottingRemains { .. } | TestOperation::GraspingHands { .. } => false,
                TestOperation::CryptChill { .. } => !success,
                TestOperation::Fight { enemy, .. } => {
                    success
                        || self
                            .scenario
                            .enemies
                            .iter()
                            .find(|target| target.card.id == enemy)
                            .and_then(|target| target.engaged)
                            .is_some_and(|victim| victim != test.tester)
                }
                _ => success,
            };
            if operation_applies {
                effects.push(TestResultEffect::Operation);
            }
            let test = self.test.as_mut().expect("Test");
            // Conditional skill text creates its delayed continuation in ST7;
            // retain only provenance when ST8 later removes physical cards.
            super::tony_effects::capture_commits(test);
            test.results_started = true;
            test.window = Window::Results;
            self.capture_presentation(if success {
                "The skill test succeeds. Apply its result effects next."
            } else {
                "The skill test fails. Apply its result effects next."
            });
            if !effects.is_empty() {
                self.frames.push(Frame::TestResults(effects));
                return Ok(());
            }
        }
        let test = self.test.as_ref().expect("Completing test");
        if !test.retaliate_checked {
            let attacker = test.tester;
            let enemy = if let TestOperation::Fight { enemy, .. } = test.operation {
                Some(enemy)
            } else {
                None
            };
            self.test
                .as_mut()
                .expect("Completing test")
                .retaliate_checked = true;
            if !success
                && let Some(enemy) = enemy
                && let Some(attack) = super::scenario::retaliate_attack(self, enemy, attacker)
            {
                self.frames.push(Frame::Combat(
                    super::combat::CombatDecision::RetaliateComplete {
                        investigator: attacker,
                    },
                ));
                self.queue_harm(
                    attacker,
                    attack.damage,
                    attack.horror,
                    false,
                    &attack.source_code,
                    HarmOrigin::EnemyAttack { enemy },
                )?;
                return Ok(());
            }
        }
        let mut test = self.test.take().expect("Completing test");
        let committed = std::mem::take(&mut test.committed);
        self.frames.push(Frame::TestCleanup(Box::new(test)));
        // All commitments leave the test simultaneously. Each owner orders
        // their own physical cards before ST8 returns tokens and the initiating
        // ability or interrupted revelation can continue. [RR p9; ST.8]
        for offset in (0..3).rev() {
            let owner = Investigator::ALL[(self.lead.index() + offset) % 3];
            let remaining: Vec<_> = committed
                .iter()
                .filter(|card| card.owner == owner)
                .map(|card| card.card.clone())
                .collect();
            let player = &mut self.players[owner.index()];
            if player.defeated || player.resigned {
                player.removed.extend(remaining);
            } else if !remaining.is_empty() {
                self.frames.push(Frame::DiscardCards {
                    owner,
                    remaining,
                    ordered: vec![],
                });
            }
        }
        Ok(())
    }

    fn complete_test_cleanup(&mut self, test: SkillTest) -> Result<(), RuleError> {
        if let Some(actor) = test.completed_granted_action {
            self.complete_action(actor);
        }
        let (total, success, failure_margin) = test.final_result.expect("Completed test result");
        let source = match &test.operation {
            TestOperation::CancelRevelation { .. } => "A Test of Will",
            TestOperation::RottingRemains { .. } => "Rotting Remains",
            TestOperation::GraspingHands { .. } => "Grasping Hands",
            TestOperation::CryptChill { .. } => "Crypt Chill",
            TestOperation::DiscardThreat { .. } => "Frozen in Fear",
            TestOperation::LitaParley => "Lita Chantler Parley",
            TestOperation::Fight { .. } => "Fight",
            TestOperation::Investigate { .. } => "Investigate",
            TestOperation::Evade { .. } => "Evade",
        };
        self.completed_tests.push(TestResult {
            breakdown: test
                .final_breakdown
                .as_deref()
                .expect("ST6 fixed the result")
                .clone(),
            tester: test.tester,
            source: source.into(),
            total,
            difficulty: test.difficulty,
            success,
            failure_margin,
        });
        super::tokens::return_test_tokens(
            self,
            test.revealed.clone(),
            test.ignored_tokens.clone(),
            test.cancelled_tokens.clone(),
        );
        match &test.operation {
            TestOperation::CancelRevelation { event, owner, .. } => {
                if !test.is_retry {
                    self.players[owner.index()].discard.push(event.clone());
                }
            }
            TestOperation::RottingRemains { encounter }
            | TestOperation::GraspingHands { encounter }
            | TestOperation::CryptChill { encounter }
                if !test.is_retry =>
            {
                self.complete_revelation(*encounter)?;
            }
            _ => {}
        }
        super::retry::begin(self, test)?;
        self.capture_presentation("The skill test finishes. Committed cards and chaos tokens return to their proper zones.");
        Ok(())
    }

    fn apply_test_operation(&mut self) -> Result<(), RuleError> {
        let test = self
            .test
            .as_ref()
            .expect("Applying skill-test results")
            .clone();
        let (total, success, _) = test.final_result.expect("ST6 outcome is fixed");
        let hand_target = match test.operation {
            TestOperation::Fight { enemy, .. } | TestOperation::Evade { enemy } => Some(enemy),
            _ => None,
        };
        if let Some(index) = hand_target.and_then(|id| {
            self.players[test.tester.index()]
                .hand
                .iter()
                .position(|card| card.id == id && card.code == "06017")
        }) {
            let card = self.players[test.tester.index()].hand.remove(index);
            if success {
                self.players[test.tester.index()].discard.push(card);
                if matches!(test.operation, TestOperation::Evade { .. })
                    && test
                        .source_card
                        .as_ref()
                        .is_some_and(|source| source.code == "06164")
                {
                    super::ethereal_form::succeeded(self, test.tester);
                }
                if matches!(test.operation, TestOperation::Evade { .. })
                    && test
                        .source_card
                        .as_ref()
                        .is_some_and(|source| source.code == "04029")
                {
                    self.frames
                        .push(Frame::Spell(super::spells::SpellDecision::MistsMove(
                            test.tester,
                        )));
                }
            } else {
                let id = card.id;
                self.scenario.enemies.push(super::scenario::EnemyState {
                    card,
                    location: self.players[test.tester.index()].location.clone(),
                    engaged: (!super::ethereal_form::is_ethereal(self, test.tester))
                        .then_some(test.tester),
                    exhausted: false,
                    damage: 0,
                    doom: 0,
                    bounties: 0,
                    owner: Some(test.tester),
                });
                self.scenario_step(super::scenario::ScenarioStep::EnemyEntered(id))?;
            }
            return Ok(());
        }
        match test.operation {
            TestOperation::CancelRevelation {
                event,
                owner,
                encounter,
            } => {
                let _ = (event, owner);
                let revelation = self
                    .frames
                    .iter_mut()
                    .find_map(|frame| match frame {
                        Frame::Revelation(revelation) if revelation.card.id == encounter => {
                            Some(revelation)
                        }
                        _ => None,
                    })
                    .ok_or_else(|| {
                        RuleError("The original encounter continuation is missing.".into())
                    })?;
                revelation.cancelled = success;
                self.history.push(if success { "The cancellation test succeeded: cancel the original encounter revelation; Take Heart gives no failure reward.".into() } else { "The cancellation test failed. After commitment/token cleanup, return to the original encounter's still-pending draw interruption; the failed cancellation itself causes no doom or horror.".into() });
            }
            TestOperation::RottingRemains { encounter } => {
                self.complete_revelation(encounter)?;
            }
            TestOperation::GraspingHands { encounter } => self.complete_revelation(encounter)?,
            TestOperation::CryptChill { .. } => {
                if !success {
                    super::crypt_chill::failed(self, test.tester)?;
                }
            }
            TestOperation::DiscardThreat { owner, card } => {
                if success {
                    let threats = &mut self.players[owner.index()].threats;
                    let index = threats
                        .iter()
                        .position(|threat| threat.id == card)
                        .expect("Pending threat");
                    self.encounter_discard.push(threats.remove(index));
                }
            }
            TestOperation::LitaParley => {
                if success {
                    super::scenario::take_control_of_lita(self, test.tester)?;
                    if super::combat::excess_ally_slot(self, test.tester) {
                        self.frames.push(Frame::Combat(
                            super::combat::CombatDecision::LitaAllySlot {
                                controller: test.tester,
                            },
                        ));
                    }
                }
            }
            TestOperation::Investigate { location, clues } => {
                if success {
                    if let Some(location) = self.scenario.location_mut(&location) {
                        let discovered = (clues + u8::from(test.eyes_match)).min(location.clues);
                        location.clues -= discovered;
                        self.players[test.tester.index()].clues += discovered;
                    }
                    super::scenario::discard_investigated_fog(self, &location);
                }
            }
            TestOperation::Evade { enemy } => {
                if success {
                    let target = self
                        .scenario
                        .enemies
                        .iter_mut()
                        .find(|target| target.card.id == enemy)
                        .ok_or_else(|| RuleError("Evaded enemy missing".into()))?;
                    target.exhausted = true;
                    target.engaged = None;
                    if test
                        .source_card
                        .as_ref()
                        .is_some_and(|source| source.code == "06164")
                    {
                        super::ethereal_form::succeeded(self, test.tester);
                    }
                    if test
                        .source_card
                        .as_ref()
                        .is_some_and(|source| source.code == "04029")
                    {
                        self.frames
                            .push(Frame::Spell(super::spells::SpellDecision::MistsMove(
                                test.tester,
                            )));
                    }
                }
            }
            TestOperation::Fight { enemy, damage, .. } => {
                if super::ethereal_form::is_ethereal(self, test.tester) {
                    return Ok(());
                }
                if success {
                    let target = self
                        .scenario
                        .enemies
                        .iter_mut()
                        .find(|target| target.card.id == enemy)
                        .ok_or_else(|| RuleError("Attacked enemy missing".into()))?;
                    target.damage += damage;
                    if target.damage
                        >= super::scenario::enemy_health(&target.card.code)
                            .ok_or_else(|| RuleError("Enemy health missing".into()))?
                    {
                        let defeated = super::scenario::remove_defeated_enemy(self, enemy)?;
                        if defeated.ends_scenario {
                            self.suspend_for_resolution();
                            self.frames.push(Frame::Scenario(ScenarioDecision::Ending));
                            return Ok(());
                        }
                        if let Some(source) = &test.source_card {
                            super::tony_cards::long_colt_defeat_reward(
                                self,
                                test.tester,
                                &source.code,
                                defeated.enemy.bounties,
                            );
                        }
                        super::combat::queue_defeat_reactions(self, test.tester, &defeated.enemy);
                    }
                    if test
                        .source_card
                        .as_ref()
                        .is_some_and(|card| card.code == "60306")
                        && total - test.difficulty >= 2
                    {
                        self.frames.push(Frame::Tony(
                            super::tony_cards::TonyDecision::MauserReward {
                                owner: test.tester,
                                asset: test.source_card.as_ref().expect("Mauser").id,
                            },
                        ));
                    }
                } else if let Some(victim) = self
                    .scenario
                    .enemies
                    .iter()
                    .find(|target| target.card.id == enemy)
                    .and_then(|target| target.engaged)
                    .filter(|victim| *victim != test.tester)
                {
                    self.queue_harm(
                        victim,
                        damage,
                        0,
                        false,
                        "failed-fight",
                        HarmOrigin::GameRule,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn take_revelation(&mut self, id: u16) -> Result<PendingRevelation, RuleError> {
        let index = self
            .frames
            .iter()
            .position(
                |frame| matches!(frame, Frame::Revelation(revelation) if revelation.card.id == id),
            )
            .ok_or_else(|| RuleError("The encounter continuation is missing.".into()))?;
        let Frame::Revelation(revelation) = self.frames.remove(index) else {
            unreachable!()
        };
        Ok(revelation)
    }

    fn complete_revelation(&mut self, id: u16) -> Result<(), RuleError> {
        let revelation = self.take_revelation(id)?;
        if revelation.card.weakness {
            self.players[revelation.drawn_by.index()]
                .discard
                .push(revelation.card);
        } else {
            self.encounter_discard.push(revelation.card);
        }
        Ok(())
    }

    fn resolve_revelation(&mut self) -> Result<(), RuleError> {
        let Some(Frame::Revelation(revelation)) = self.frames.last_mut() else {
            return Err(RuleError("No encounter revelation is pending.".into()));
        };
        let (id, code, investigator) = (
            revelation.card.id,
            revelation.card.code.clone(),
            revelation.drawn_by,
        );
        revelation.started = true;
        let result = match code.as_str() {
            "01166" => {
                let consequence = super::scenario::resolve_ancient_evils(self)?;
                self.history.push("Ancient Evils resolves after the unsuccessful/declined cancellation: add 1 doom and check advancement immediately. [01166]".into());
                self.complete_revelation(id)?;
                if let Some(consequence) = consequence {
                    self.scenario_consequence(consequence)?;
                }
                Ok(())
            }
            "01163" => self.begin_test(
                TestOperation::RottingRemains { encounter: id },
                investigator,
                Skill::Willpower,
                3,
            ),
            "01162" => self.begin_test(
                TestOperation::GraspingHands { encounter: id },
                investigator,
                Skill::Agility,
                3,
            ),
            "01167" => self.begin_test(
                TestOperation::CryptChill { encounter: id },
                investigator,
                Skill::Willpower,
                4,
            ),
            "01164" | "01165" | "01168" => {
                let card = self.take_revelation(id)?.card;
                super::scenario::place_persistent_treachery(self, card, investigator)
            }
            "02038" | "02039" | "05041" | "60403" => {
                let card = self.take_revelation(id)?.card;
                self.players[investigator.index()].threats.push(card);
                Ok(())
            }
            "06017" => {
                let card = self.take_revelation(id)?.card;
                self.players[investigator.index()].hand.push(card);
                Ok(())
            }
            "06012" => {
                let card = self.take_revelation(id)?.card;
                let origin = &self.players[Investigator::Tony.index()].location;
                let maximum = self
                    .scenario
                    .locations
                    .iter()
                    .filter_map(|location| self.scenario.distance(origin, &location.code))
                    .max();
                let locations = self
                    .scenario
                    .locations
                    .iter()
                    .filter(|location| self.scenario.distance(origin, &location.code) == maximum)
                    .map(|location| location.code.clone())
                    .collect();
                self.frames.push(Frame::QuarrySpawn { card, locations });
                Ok(())
            }
            code if super::scenario::enemy_definition(code).is_some() => {
                let card = self.take_revelation(id)?.card;
                if let Some(enemy) =
                    super::scenario::spawn_encounter_enemy(self, card, investigator)?
                {
                    self.scenario_step(super::scenario::ScenarioStep::EnemyEntered(enemy))?;
                }
                Ok(())
            }
            _ => Err(RuleError(
                "This encounter revelation is not implemented yet.".into(),
            )),
        };
        result?;
        let title = content::title(&code).unwrap_or(&code);
        let caption = match code.as_str() {
            "01162" | "01163" | "01167" => format!(
                "{title} requires {} to perform a skill test.",
                investigator.name()
            ),
            "02038" | "02039" | "05041" | "60403" => {
                format!("{title} enters {}'s threat area.", investigator.name())
            }
            "06017" => "Watcher from Another Dimension remains in Patrice's hand.".into(),
            "06012" => "Tony's Quarry awaits its farthest-location spawn choice.".into(),
            _ => format!("Resolve {title}'s revelation consequences."),
        };
        self.capture_presentation(caption);
        Ok(())
    }

    fn settle(&mut self) -> Result<(), RuleError> {
        loop {
            if matches!(self.frames.last(), Some(Frame::ExpireEthereal)) {
                self.frames.pop();
                super::ethereal_form::expire(self);
                continue;
            }
            if matches!(self.frames.last(), Some(Frame::Backpack(_))) {
                return Ok(());
            }
            if matches!(self.frames.last(), Some(Frame::Peter { .. })) {
                return Ok(());
            }
            if matches!(self.frames.last(), Some(Frame::HandDiscard(_))) {
                if super::harm_cards::advance_discard(self)? {
                    continue;
                }
                return Ok(());
            }
            if matches!(self.frames.last(), Some(Frame::HarmCard(_))) {
                super::harm_cards::finish(self)?;
                continue;
            }
            if let Some(Frame::GrantedAction { owner }) = self.frames.last() {
                if self.players[owner.index()].defeated || self.players[owner.index()].resigned {
                    self.frames.pop();
                    continue;
                }
                return Ok(());
            }
            if self.test.is_none()
                && self.tokens.is_none()
                && matches!(self.frames.last(), Some(Frame::RestoreTurn { .. }))
            {
                if let Some(actor) = self.action_in_progress.take() {
                    self.complete_action(actor);
                }
                let Some(Frame::RestoreTurn {
                    previous_actor,
                    previous_phase,
                }) = self.frames.pop()
                else {
                    unreachable!()
                };
                self.actor = previous_actor.filter(|actor| {
                    !self.players[actor.index()].defeated && !self.players[actor.index()].resigned
                });
                self.phase = previous_phase;
                continue;
            }
            if matches!(self.frames.last(), Some(Frame::SuccessWindow)) {
                if !self.outcome_reaction_choices().is_empty() {
                    return Ok(());
                }
                self.frames.pop();
                super::tony_effects::close_success_window(self);
                self.finish_test()?;
                continue;
            }
            if let Some(Frame::AfterTest(state)) = self.frames.last() {
                if !super::retry::choices(self, state).is_empty() {
                    return Ok(());
                }
                self.frames.pop();
                continue;
            }
            if self.test.is_none()
                && self.tokens.is_none()
                && matches!(self.frames.last(), Some(Frame::RetryEvent { .. }))
            {
                let Some(Frame::RetryEvent { owner, event }) = self.frames.pop() else {
                    unreachable!()
                };
                let player = &mut self.players[owner.index()];
                if player.defeated || player.resigned {
                    player.removed.push(event);
                } else {
                    player.discard.push(event);
                }
                continue;
            }
            if matches!(self.frames.last(), Some(Frame::Elimination(_))) {
                return Ok(());
            }
            if matches!(self.frames.last(), Some(Frame::TestCleanup(_))) {
                let Some(Frame::TestCleanup(test)) = self.frames.pop() else {
                    unreachable!()
                };
                self.complete_test_cleanup(*test)?;
                continue;
            }
            if let Some(Frame::DiscardCards { remaining, .. }) = self.frames.last() {
                if remaining.len() > 1 {
                    return Ok(());
                }
                let card = remaining.first().expect("Unfinished discard group").id;
                self.choose_discard_order(&format!("discard-card:{card}"))?;
                continue;
            }
            if let Some(Frame::Spell(decision)) = self.frames.last() {
                if super::spells::choices(self, decision).is_empty() {
                    self.frames.pop();
                    continue;
                }
                return Ok(());
            }
            if matches!(self.frames.last(), Some(Frame::Draw(_))) {
                if !super::draws::choices(self).is_empty() {
                    return Ok(());
                }
                super::draws::advance(self)?;
                continue;
            }
            if matches!(self.frames.last(),Some(Frame::Revelation(revelation)) if revelation.card.weakness && !revelation.started)
            {
                self.resolve_revelation()?;
                continue;
            }
            if let Some(Frame::Harm(harm)) = self.frames.last() {
                let choices = self.harm_choices(harm);
                if choices.iter().all(|choice| {
                    choice.id.ends_with(":investigator") || choice.id == "harm:confirm"
                }) {
                    self.choose_harm(&choices[0].id)?;
                    continue;
                }
                return Ok(());
            }
            if matches!(self.frames.last(), Some(Frame::ResumeTokens)) {
                self.frames.pop();
                let step = super::tokens::resume(self)?;
                self.token_step(step)?;
                continue;
            }
            if matches!(self.frames.last(), Some(Frame::GainResources { .. })) {
                let Some(Frame::GainResources {
                    investigator,
                    amount,
                }) = self.frames.pop()
                else {
                    unreachable!()
                };
                if !self.players[investigator.index()].defeated {
                    self.players[investigator.index()].resources += amount;
                    self.capture_presentation(format!(
                        "{} gains {amount} resources.",
                        investigator.name()
                    ));
                }
                continue;
            }
            if let Some(Frame::AfterHarm(_)) = self.frames.last() {
                let Some(Frame::AfterHarm(continuation)) = self.frames.pop() else {
                    unreachable!()
                };
                match continuation {
                    HarmContinuation::AgendaOne => self.agenda = 2,
                    HarmContinuation::ApplySymbols => {
                        if !self.result(self.test.as_ref().expect("Test")).1
                            && !self.lucky_choices().is_empty()
                        {
                            self.test.as_mut().expect("Test").window = Window::WouldFail;
                        } else {
                            self.finish_test()?;
                        }
                    }
                }
                continue;
            }
            if let Some(Frame::TestResults(effects)) = self.frames.last() {
                if effects.is_empty() {
                    self.frames.pop();
                    self.finish_test()?;
                    continue;
                }
                if effects.len() == 1 {
                    self.choose_result("result:0")?;
                    continue;
                }
                return Ok(());
            }
            if let Some(test) = &self.test {
                let next = match test.window {
                    Window::BeforeCommitment if self.player_window_choices().is_empty() => {
                        Some(Window::Commitment)
                    }
                    Window::Commitment if self.commit_choices().is_empty() => {
                        Some(Window::BeforeReveal)
                    }
                    Window::BeforeReveal if self.player_window_choices().is_empty() => {
                        Some(Window::Reveal)
                    }
                    _ => None,
                };
                if let Some(next) = next {
                    self.test.as_mut().expect("Running test").window = next;
                    continue;
                }
                if test.window == Window::Reveal && self.tokens.is_none() {
                    super::tokens::begin(
                        self,
                        test.tester,
                        super::tokens::TokenPurpose::SkillTest,
                    )?;
                }
                return Ok(());
            }
            if self.tokens.is_some() {
                return Ok(());
            }
            if let Some(Frame::Opportunity { investigator, .. }) = self.frames.last() {
                let live = super::scenario::opportunity_attacks(self, *investigator)
                    .into_iter()
                    .map(|attack| attack.enemy)
                    .collect::<Vec<_>>();
                let Some(Frame::Opportunity { pending, .. }) = self.frames.last_mut() else {
                    unreachable!()
                };
                pending.retain(|enemy| live.contains(enemy));
                if !pending.is_empty() {
                    return Ok(());
                }
                let Some(Frame::Opportunity {
                    investigator,
                    action,
                    spell,
                    asset,
                    event,
                    ..
                }) = self.frames.pop()
                else {
                    unreachable!()
                };
                if !self.players[investigator.index()].defeated
                    && !self.players[investigator.index()].resigned
                {
                    if let Some(spell) = spell {
                        super::spells::resolve_action(self, spell)?;
                    } else if let Some(event) = event {
                        super::encounter_search::resolve_action(self, event)?;
                    } else if let Some(asset) = asset {
                        self.finish_asset(asset)?;
                    } else {
                        self.perform_action(investigator, &action)?;
                    }
                } else {
                    if let Some(asset) = asset {
                        self.players[investigator.index()].removed.push(asset.card);
                    }
                    if let Some(spell) = spell {
                        super::spells::cancel_action(self, spell);
                    }
                    if let Some(event) = event {
                        super::encounter_search::cancel_action(self, event);
                    }
                }
                continue;
            }
            if let Some(Frame::TurnEnd {
                investigator,
                pending,
            }) = self.frames.last()
            {
                if !pending.is_empty() {
                    return Ok(());
                }
                let actor = *investigator;
                self.frames.pop();
                self.players[actor.index()].turn_finished = true;
                self.actor = None;
                if !super::peter::begin(self, actor) {
                    super::peter::finish_turn_end(self);
                }
                continue;
            }
            if self.frames.is_empty()
                && let Some(actor) = self.action_in_progress.take()
            {
                self.complete_action(actor);
            }
            if self.frames.is_empty()
                && self.test.is_none()
                && self.tokens.is_none()
                && self.phase == Phase::Investigation
                && self.actor.is_none()
                && self.scenario.outcome.is_none()
                && self
                    .players
                    .iter()
                    .all(|player| player.turn_finished || player.defeated || player.resigned)
            {
                super::phases::begin_enemy(self);
                continue;
            }
            let Some(Frame::Revelation(revelation)) = self.frames.last() else {
                return Ok(());
            };
            if revelation.cancelled {
                let id = revelation.card.id;
                self.complete_revelation(id)?;
                continue;
            }
            if !revelation.started && self.cancellation_choices(revelation).is_empty() {
                self.resolve_revelation()?;
                continue;
            }
            return Ok(());
        }
    }
}

fn choice(id: &str, label: &str) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        cancel_faces: vec![],
    }
}
