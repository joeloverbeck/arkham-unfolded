//! Physical token custody and the selected reveal replacements.
//!
//! Rules expectations: issue-89-rules-jacqueline J1–J7. A reveal declaration
//! precedes randomness; cancellation never returns a token early to the bag.

use super::{Investigator, runtime::Game};
use crate::{Choice, RuleError, TokenFace};
use serde::{Deserialize, Serialize};

mod groups;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedPremonition {
    pub controller: Investigator,
    pub card: super::CardView,
    pub faces: Vec<TokenFace>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenFixture {
    ProtectedNumbers,
    ArbiterCancellation,
    DarkProphecy,
    RobesProphecy,
    NestedProphecyCleanup,
    EyesKatarina,
    EyesIgnoredMatch,
    EyesOlive,
    EyesOliveProtected,
    EyesOliveJacqueline,
    VoiceRaCurse,
    VoiceRaLethalOpportunity,
    PremonitionVoiceOpportunity,
    Premonition,
    PremonitionOlive,
    PremonitionEyes,
    PremonitionDarkFuture,
    PremonitionDarkFutureAbsent,
    PremonitionVoice,
    JacquelineOlive,
    JacquelineOliveProtected,
    ProphecyOlive,
    ProphecyOliveProtected,
    ProphecyOliveOtherSymbol,
}

pub(super) fn configure_fixture(game: &mut Game, case: TokenFixture) {
    use super::runtime::{AssetView, Frame, PendingRevelation, Skill, TestOperation};
    assert!(game.test.is_none());
    game.history.clear();
    game.history.push(format!("Privileged token verification case {case:?}: selected physical cards are relocated with their inventory preserved. This is not ordinary play."));
    for frame in game.frames.drain(..) {
        if let Frame::Revelation(revelation) = frame {
            game.encounter_deck.push(revelation.card);
        } else {
            panic!("Token fixture expects only the base encounter frame");
        }
    }
    let jacqueline = &mut game.players[Investigator::Jacqueline.index()];
    jacqueline.deck.append(&mut jacqueline.hand);
    match case {
        TokenFixture::JacquelineOlive
        | TokenFixture::JacquelineOliveProtected
        | TokenFixture::ProphecyOlive
        | TokenFixture::ProphecyOliveProtected
        | TokenFixture::ProphecyOliveOtherSymbol => {
            let index = jacqueline
                .deck
                .iter()
                .position(|card| card.code == "04197")
                .unwrap();
            let card = jacqueline.deck.remove(index);
            jacqueline.assets.push(AssetView {
                card,
                ready: true,
                uses: 0,
                damage: 0,
                horror: 0,
            });
            if matches!(
                case,
                TokenFixture::ProphecyOlive
                    | TokenFixture::ProphecyOliveProtected
                    | TokenFixture::ProphecyOliveOtherSymbol
            ) {
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == "04032")
                    .unwrap();
                jacqueline.hand.push(jacqueline.deck.remove(index));
            }
            if matches!(
                case,
                TokenFixture::JacquelineOliveProtected | TokenFixture::ProphecyOliveProtected
            ) {
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == "60403")
                    .unwrap();
                jacqueline.threats.push(jacqueline.deck.remove(index));
            }
        }
        TokenFixture::ProtectedNumbers => {
            let index = jacqueline
                .deck
                .iter()
                .position(|card| card.code == "60403")
                .expect("Selected Dark Future");
            jacqueline.threats.push(jacqueline.deck.remove(index));
        }
        TokenFixture::ArbiterCancellation => {
            let index = jacqueline
                .deck
                .iter()
                .position(|card| card.code == "60402")
                .expect("Selected Arbiter");
            let card = jacqueline.deck.remove(index);
            jacqueline.assets.push(AssetView {
                card,
                ready: true,
                uses: 0,
                damage: 0,
                horror: 0,
            });
            jacqueline.resources -= 3;
        }
        TokenFixture::DarkProphecy
        | TokenFixture::RobesProphecy
        | TokenFixture::NestedProphecyCleanup => {
            let index = jacqueline
                .deck
                .iter()
                .position(|card| card.code == "04032")
                .expect("Selected Dark Prophecy");
            jacqueline.hand.push(jacqueline.deck.remove(index));
            if case == TokenFixture::RobesProphecy {
                let at = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == "60412")
                    .expect("Selected Robes");
                let card = jacqueline.deck.remove(at);
                jacqueline.assets.push(AssetView {
                    card,
                    ready: true,
                    uses: 0,
                    damage: 0,
                    horror: 0,
                });
                jacqueline.resources = 0;
            }
            if case == TokenFixture::NestedProphecyCleanup {
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == "60403")
                    .expect("Selected Dark Future");
                jacqueline.threats.push(jacqueline.deck.remove(index));
            }
        }
        TokenFixture::EyesOlive
        | TokenFixture::EyesOliveProtected
        | TokenFixture::EyesOliveJacqueline => {
            for code in ["11067", "04197"] {
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == code)
                    .expect("Selected asset");
                let card = jacqueline.deck.remove(index);
                jacqueline.assets.push(AssetView {
                    card,
                    ready: true,
                    uses: if code == "11067" { 9 } else { 0 },
                    damage: 0,
                    horror: 0,
                });
            }
            jacqueline.resources = 0;
            if matches!(
                case,
                TokenFixture::EyesOliveProtected | TokenFixture::EyesOliveJacqueline
            ) {
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == "60403")
                    .expect("Selected Dark Future");
                jacqueline.threats.push(jacqueline.deck.remove(index));
            }
        }
        TokenFixture::EyesKatarina | TokenFixture::EyesIgnoredMatch => {
            for code in ["11067", "11069"] {
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == code)
                    .expect("Selected asset");
                let card = jacqueline.deck.remove(index);
                let uses = super::cards::definition(code)
                    .unwrap()
                    .uses
                    .map_or(0, |(_, count)| count);
                jacqueline.assets.push(AssetView {
                    card,
                    ready: true,
                    uses,
                    damage: 0,
                    horror: 0,
                });
            }
            for (code, threat) in [("11097", false), ("60403", true)] {
                if threat && case == TokenFixture::EyesIgnoredMatch {
                    continue;
                }
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == code)
                    .expect("Selected card");
                let card = jacqueline.deck.remove(index);
                if threat {
                    jacqueline.threats.push(card);
                } else {
                    jacqueline.hand.push(card);
                }
            }
            jacqueline.resources = 0;
            jacqueline.location = super::scenario::CELLAR.into();
            jacqueline.damage = 1;
        }
        TokenFixture::VoiceRaCurse
        | TokenFixture::VoiceRaLethalOpportunity
        | TokenFixture::PremonitionVoiceOpportunity => {
            for (code, threat) in [("60416", false), ("60403", true)] {
                if threat && case != TokenFixture::VoiceRaCurse {
                    continue;
                }
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == code)
                    .expect("Selected card");
                let card = jacqueline.deck.remove(index);
                if threat {
                    jacqueline.threats.push(card);
                } else {
                    jacqueline.hand.push(card);
                }
            }
        }
        TokenFixture::Premonition
        | TokenFixture::PremonitionOlive
        | TokenFixture::PremonitionEyes
        | TokenFixture::PremonitionDarkFuture
        | TokenFixture::PremonitionDarkFutureAbsent
        | TokenFixture::PremonitionVoice => {
            for (code, threat) in [("04199", false), ("60403", true)] {
                if threat
                    && !matches!(
                        case,
                        TokenFixture::Premonition
                            | TokenFixture::PremonitionDarkFuture
                            | TokenFixture::PremonitionDarkFutureAbsent
                    )
                {
                    continue;
                }
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == code)
                    .expect("Selected card");
                let card = jacqueline.deck.remove(index);
                if threat {
                    jacqueline.threats.push(card);
                } else {
                    jacqueline.hand.push(card);
                }
            }
            let spell = if case == TokenFixture::PremonitionEyes {
                "11067"
            } else {
                "60408"
            };
            let index = jacqueline
                .deck
                .iter()
                .position(|card| card.code == spell)
                .expect("Selected investigation spell");
            let card = jacqueline.deck.remove(index);
            jacqueline.assets.push(AssetView {
                card,
                ready: true,
                uses: if case == TokenFixture::PremonitionEyes {
                    9
                } else {
                    3
                },
                damage: 0,
                horror: 0,
            });
            jacqueline.resources -= if case == TokenFixture::PremonitionEyes {
                3
            } else {
                4
            };
            jacqueline.location = super::scenario::CELLAR.into();
            jacqueline.damage = 1;
            if case != TokenFixture::Premonition {
                let index = jacqueline
                    .deck
                    .iter()
                    .position(|card| card.code == "04197")
                    .unwrap();
                let card = jacqueline.deck.remove(index);
                jacqueline.assets.push(AssetView {
                    card,
                    ready: true,
                    uses: 0,
                    damage: 0,
                    horror: 0,
                });
            }
            if case == TokenFixture::PremonitionVoice {
                for code in ["60416", "04032"] {
                    let index = jacqueline
                        .deck
                        .iter()
                        .position(|card| card.code == code)
                        .unwrap();
                    jacqueline.hand.push(jacqueline.deck.remove(index));
                }
                jacqueline.resources = 5;
            }
        }
    }
    if matches!(
        case,
        TokenFixture::EyesOlive
            | TokenFixture::EyesOliveProtected
            | TokenFixture::EyesOliveJacqueline
    ) {
        game.scenario = super::scenario::ScenarioState::new();
        game.act = 1;
        for player in &mut game.players {
            player.location = super::scenario::STUDY.into();
            player.clues = 0;
        }
        game.enable_fixture_turn(Investigator::Jacqueline);
        let mut faces = vec![
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            TokenFace::Skull,
            TokenFace::MinusOne,
        ];
        if case == TokenFixture::EyesOliveJacqueline {
            faces.extend([TokenFace::Zero, TokenFace::PlusOne]);
        }
        game.set_fixture_tokens(faces);
        return;
    }
    if case == TokenFixture::PremonitionVoiceOpportunity {
        let index = jacqueline
            .deck
            .iter()
            .position(|card| card.code == "04199")
            .expect("Selected Premonition");
        jacqueline.hand.push(jacqueline.deck.remove(index));
        jacqueline.location = super::scenario::HALLWAY.into();
        for who in [Investigator::Tony, Investigator::Patrice] {
            game.players[who.index()].location = super::scenario::CELLAR.into();
        }
        game.enable_fixture_turn(Investigator::Jacqueline);
        let priest = super::scenario::advance_act_two(game).expect("Act2 reverse");
        game.check_engagement(priest);
        game.set_fixture_tokens(vec![
            TokenFace::ElderSign,
            TokenFace::Skull,
            TokenFace::Zero,
        ]);
        return;
    }
    if case == TokenFixture::VoiceRaLethalOpportunity {
        jacqueline.damage = 5;
        game.enable_fixture_turn(Investigator::Jacqueline);
        let priest = super::scenario::advance_act_two(game).expect("Act2 reverse");
        game.check_engagement(priest);
        return;
    }
    if matches!(
        case,
        TokenFixture::Premonition
            | TokenFixture::PremonitionOlive
            | TokenFixture::PremonitionEyes
            | TokenFixture::PremonitionDarkFuture
            | TokenFixture::PremonitionDarkFutureAbsent
            | TokenFixture::PremonitionVoice
    ) {
        let card = jacqueline.hand[0].id;
        if case == TokenFixture::PremonitionEyes {
            game.scenario = super::scenario::ScenarioState::new();
            game.act = 1;
            for player in &mut game.players {
                player.location = super::scenario::STUDY.into();
                player.clues = 0;
            }
        }
        game.enable_fixture_turn(Investigator::Jacqueline);
        play_premonition(game, card).expect("Legal fast play");
        game.set_fixture_tokens(
            if matches!(
                case,
                TokenFixture::PremonitionDarkFuture | TokenFixture::PremonitionDarkFutureAbsent
            ) {
                vec![
                    TokenFace::MinusOne,
                    TokenFace::MinusTwo,
                    TokenFace::MinusOne,
                    if case == TokenFixture::PremonitionDarkFuture {
                        TokenFace::ElderSign
                    } else {
                        TokenFace::Cultist
                    },
                    TokenFace::Zero,
                    TokenFace::PlusOne,
                    TokenFace::Skull,
                ]
            } else if case == TokenFixture::PremonitionVoice {
                vec![
                    TokenFace::Skull,
                    TokenFace::ElderSign,
                    TokenFace::MinusOne,
                    TokenFace::Cultist,
                    TokenFace::Zero,
                ]
            } else if case == TokenFixture::PremonitionEyes {
                vec![
                    TokenFace::MinusOne,
                    TokenFace::MinusTwo,
                    TokenFace::Skull,
                    TokenFace::MinusOne,
                ]
            } else if case != TokenFixture::Premonition {
                vec![
                    TokenFace::ElderSign,
                    TokenFace::PlusOne,
                    TokenFace::MinusOne,
                ]
            } else {
                vec![TokenFace::ElderSign]
            },
        );
        return;
    }
    if case == TokenFixture::VoiceRaCurse {
        let event = jacqueline.hand[0].id;
        jacqueline.ordinary_actions -= 1;
        let patrice = &mut game.players[Investigator::Patrice.index()];
        let index = patrice
            .deck
            .iter()
            .position(|card| card.code == "07032")
            .expect("Promise of Power");
        patrice.discard.push(patrice.deck.remove(index));
        game.bag.push(TokenFace::Curse);
        let paid = super::spells::pay_action(
            game,
            Investigator::Jacqueline,
            &format!("play:spell:{event}"),
        )
        .expect("Voice of Ra play");
        super::spells::resolve_action(game, paid).expect("One reveal");
        game.set_fixture_tokens(vec![
            TokenFace::Curse,
            TokenFace::ElderSign,
            TokenFace::MinusOne,
        ]);
        return;
    }
    if matches!(
        case,
        TokenFixture::EyesKatarina | TokenFixture::EyesIgnoredMatch
    ) {
        let asset = jacqueline
            .assets
            .iter()
            .find(|asset| asset.card.code == "11067")
            .unwrap()
            .card
            .id;
        jacqueline.ordinary_actions -= 1;
        let paid = super::spells::pay_action(
            game,
            Investigator::Jacqueline,
            &format!("investigate:spell:{asset}"),
        )
        .expect("Legal Eyes activation");
        super::spells::resolve_action(game, paid).expect("One test");
        game.set_fixture_tokens(if case == TokenFixture::EyesKatarina {
            vec![
                TokenFace::MinusOne,
                TokenFace::MinusTwo,
                TokenFace::Skull,
                TokenFace::ElderSign,
            ]
        } else {
            vec![
                TokenFace::MinusOne,
                TokenFace::MinusOne,
                TokenFace::Skull,
                TokenFace::MinusOne,
            ]
        });
        return;
    }
    if case == TokenFixture::NestedProphecyCleanup {
        let card = jacqueline
            .threats
            .iter()
            .find(|card| card.code == "60403")
            .expect("Dark Future")
            .id;
        game.set_fixture_tokens(vec![
            TokenFace::ElderSign,
            TokenFace::Skull,
            TokenFace::Cultist,
            TokenFace::Zero,
            TokenFace::MinusOne,
            TokenFace::Zero,
            TokenFace::MinusOne,
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            TokenFace::MinusTwo,
            TokenFace::MinusThree,
        ]);
        begin_dark_future(game, Investigator::Jacqueline, card).expect("One reveal");
        return;
    }
    let index = game
        .encounter_deck
        .iter()
        .position(|card| card.code == "01163")
        .expect("Rotting Remains in encounter deck");
    let card = game.encounter_deck.remove(index);
    let encounter = card.id;
    game.frames.push(Frame::Revelation(PendingRevelation {
        card,
        drawn_by: Investigator::Jacqueline,
        started: true,
        cancelled: false,
    }));
    game.begin_test(
        TestOperation::RottingRemains { encounter },
        Investigator::Jacqueline,
        Skill::Willpower,
        3,
    )
    .expect("One test");
    game.set_fixture_tokens(vec![
        TokenFace::AutoFail,
        TokenFace::Zero,
        TokenFace::MinusOne,
    ]);
    if matches!(
        case,
        TokenFixture::DarkProphecy | TokenFixture::RobesProphecy
    ) {
        game.set_fixture_tokens(vec![
            TokenFace::ElderSign,
            TokenFace::Skull,
            TokenFace::Cultist,
            TokenFace::Zero,
            TokenFace::MinusOne,
        ]);
    }
    if matches!(
        case,
        TokenFixture::JacquelineOlive | TokenFixture::JacquelineOliveProtected
    ) {
        game.set_fixture_tokens(vec![
            if case == TokenFixture::JacquelineOlive {
                TokenFace::AutoFail
            } else {
                TokenFace::Skull
            },
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            TokenFace::Zero,
            TokenFace::PlusOne,
        ]);
    } else if matches!(
        case,
        TokenFixture::ProphecyOlive
            | TokenFixture::ProphecyOliveProtected
            | TokenFixture::ProphecyOliveOtherSymbol
    ) {
        game.set_fixture_tokens(vec![
            TokenFace::Skull,
            TokenFace::MinusOne,
            TokenFace::MinusTwo,
            TokenFace::Zero,
            TokenFace::PlusOne,
            if case == TokenFixture::ProphecyOliveOtherSymbol {
                TokenFace::Cultist
            } else {
                TokenFace::MinusOne
            },
            TokenFace::MinusTwo,
        ]);
    }
    game.history.push("Privileged Jacqueline token witness: actual selected cards relocated for a named rules case, with the source inventory preserved. This is not ordinary play.".into());
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenPurpose {
    SkillTest,
    DarkFuture {
        investigator: Investigator,
        card: u16,
    },
    VoiceRa {
        investigator: Investigator,
        event: super::CardView,
    },
    Premonition {
        investigator: Investigator,
        event: super::CardView,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenStage {
    EyesCharges,
    Declaration,
    Cancellation,
    Selection,
    Effects,
    Resolving,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Policy {
    Draw,
    Sealed,
    All,
    Prophecy,
    Olive,
    Katarina,
    Eyes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct RevealNode {
    policy: Policy,
    children: Vec<RevealNode>,
    cursor: usize,
    drawn: Vec<usize>,
    jacqueline: bool,
    cancellation_done: bool,
    selection_done: bool,
    done: bool,
    event: Option<super::CardView>,
    chosen: Vec<usize>,
    sealed: Option<SealedPremonition>,
    sealed_card_departed: bool,
}

impl RevealNode {
    fn draw() -> Self {
        Self {
            policy: Policy::Draw,
            children: vec![],
            cursor: 0,
            drawn: vec![],
            jacqueline: false,
            cancellation_done: false,
            selection_done: false,
            done: false,
            event: None,
            chosen: vec![],
            sealed: None,
            sealed_card_departed: false,
        }
    }

    fn group(policy: Policy, count: usize) -> Self {
        Self {
            policy,
            children: (0..count).map(|_| Self::draw()).collect(),
            ..Self::draw()
        }
    }

    fn at(&self, path: &[usize]) -> &Self {
        path.first()
            .map_or(self, |index| self.children[*index].at(&path[1..]))
    }

    fn at_mut(&mut self, path: &[usize]) -> &mut Self {
        if path.is_empty() {
            self
        } else {
            self.children[path[0]].at_mut(&path[1..])
        }
    }

    fn declarations(&self, path: Vec<usize>, nodes: &mut Vec<(Vec<usize>, Policy, bool)>) {
        nodes.push((path.clone(), self.policy, self.jacqueline));
        for (index, child) in self.children.iter().enumerate() {
            let mut next = path.clone();
            next.push(index);
            child.declarations(next, nodes);
        }
    }

    fn contains_sealed(&self) -> bool {
        self.policy == Policy::Sealed || self.children.iter().any(Self::contains_sealed)
    }

    fn eyes_with_groups(&self) -> bool {
        self.policy == Policy::Eyes
            && self
                .children
                .iter()
                .any(|child| matches!(child.policy, Policy::Olive | Policy::Sealed))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenResolution {
    pub revealer: Investigator,
    pub purpose: TokenPurpose,
    pub stage: TokenStage,
    pub jacqueline: bool,
    pub revealed: Vec<TokenFace>,
    pub selected: Vec<usize>,
    pub ignored: Vec<usize>,
    pub cancelled: Vec<usize>,
    root: RevealNode,
    active: Vec<usize>,
    notified_elders: Vec<usize>,
    eyes_source: Option<u16>,
    previous_roots: Vec<RevealNode>,
    batch_notified: bool,
    symbols_done: Vec<usize>,
}

impl TokenResolution {
    /// Paid events in the reveal continuation are public; unrevealed faces are not.
    pub(super) fn resolving_cards(&self) -> Vec<super::CardView> {
        fn collect(node: &RevealNode, cards: &mut Vec<super::CardView>) {
            if let Some(event) = &node.event {
                cards.push(event.clone());
            }
            for child in &node.children {
                collect(child, cards);
            }
        }
        let mut cards = vec![];
        if let TokenPurpose::VoiceRa { event, .. } | TokenPurpose::Premonition { event, .. } =
            &self.purpose
        {
            cards.push(event.clone());
        }
        collect(&self.root, &mut cards);
        for root in &self.previous_roots {
            collect(root, &mut cards);
        }
        cards
    }
}

pub(super) enum TokenStep {
    Pending,
    DrawElderSign,
    RevealedBatch {
        faces: Vec<TokenFace>,
    },
    ApplyTablet,
    Finished,
    Complete {
        resolved: Vec<TokenFace>,
        ignored: Vec<TokenFace>,
        cancelled: Vec<TokenFace>,
        eyes_match: bool,
        symbols_applied: bool,
    },
}

/// Ten physical curse tokens exist. Held and sealed tokens are unavailable for
/// adding, including a resolved curse awaiting ST8 cleanup.
pub(super) fn add_curse(game: &mut Game) -> bool {
    if curse_pool(game) == 0 {
        return false;
    }
    game.bag.push(TokenFace::Curse);
    true
}

/// The same physical custody count used to admit adding a curse to the bag.
pub(super) fn curse_pool(game: &Game) -> u8 {
    let count_faces = |faces: &[TokenFace]| {
        faces
            .iter()
            .filter(|&&face| face == TokenFace::Curse)
            .count()
    };
    let mut held = count_faces(&game.bag);
    held += game
        .premonition
        .as_ref()
        .map_or(0, |sealed| count_faces(&sealed.faces));
    if let Some(tokens) = &game.tokens {
        held += count_faces(&tokens.revealed) + sealed_curses(&tokens.root);
        held += tokens
            .previous_roots
            .iter()
            .map(sealed_curses)
            .sum::<usize>();
    }
    if let Some(test) = &game.test {
        held += count_faces(&test.revealed)
            + count_faces(&test.ignored_tokens)
            + count_faces(&test.cancelled_tokens);
    }
    10_usize.saturating_sub(held) as u8
}

fn sealed_curses(node: &RevealNode) -> usize {
    node.sealed.as_ref().map_or(0, |sealed| {
        sealed
            .faces
            .iter()
            .filter(|&&face| face == TokenFace::Curse)
            .count()
    }) + node.children.iter().map(sealed_curses).sum::<usize>()
}

pub(super) fn return_test_tokens(
    game: &mut Game,
    resolved: Vec<TokenFace>,
    ignored: Vec<TokenFace>,
    cancelled: Vec<TokenFace>,
) {
    game.bag.extend(
        resolved
            .into_iter()
            .filter(|face| *face != TokenFace::Curse),
    );
    game.bag.extend(ignored);
    game.bag.extend(cancelled);
}

/// Elimination removes owned cards without returning tokens held by another
/// investigator's still-running test. A sealed replacement already in progress
/// retains its token effect after its source card leaves play.
pub(super) fn eliminate(game: &mut Game, who: Investigator) -> Result<(), RuleError> {
    if game
        .premonition
        .as_ref()
        .is_some_and(|sealed| sealed.controller == who)
    {
        let sealed = game.premonition.take().expect("Owned Premonition");
        game.bag.extend(sealed.faces);
        game.players[who.index()].removed.push(sealed.card);
    }
    let Some(mut tokens) = game.tokens.take() else {
        return Ok(());
    };
    if tokens.revealer == who {
        game.bag.extend(
            tokens
                .revealed
                .drain(..)
                .enumerate()
                .filter_map(|(index, face)| {
                    (face != TokenFace::Curse || !tokens.symbols_done.contains(&index))
                        .then_some(face)
                }),
        );
        abandon_node(game, &mut tokens.root, who);
        for node in &mut tokens.previous_roots {
            abandon_node(game, node, who);
        }
        match tokens.purpose {
            TokenPurpose::VoiceRa { event, .. } | TokenPurpose::Premonition { event, .. } => {
                game.players[who.index()].removed.push(event)
            }
            _ => {}
        }
    } else {
        depart_sealed_source(game, &mut tokens.root, who);
        game.tokens = Some(tokens);
    }
    Ok(())
}

fn abandon_node(game: &mut Game, node: &mut RevealNode, revealer: Investigator) {
    if let Some(event) = node.event.take() {
        game.players[revealer.index()].removed.push(event);
    }
    if let Some(sealed) = node.sealed.take() {
        game.bag.extend(sealed.faces);
        if !node.sealed_card_departed {
            let owner = &mut game.players[sealed.controller.index()];
            if owner.defeated || owner.resigned || sealed.controller == revealer {
                owner.removed.push(sealed.card);
            } else {
                owner.discard.push(sealed.card);
            }
        }
    }
    for child in &mut node.children {
        abandon_node(game, child, revealer);
    }
}

fn depart_sealed_source(game: &mut Game, node: &mut RevealNode, who: Investigator) {
    if let Some(sealed) = &node.sealed
        && sealed.controller == who
        && !node.sealed_card_departed
    {
        game.players[who.index()].removed.push(sealed.card.clone());
        node.sealed_card_departed = true;
    }
    for child in &mut node.children {
        depart_sealed_source(game, child, who);
    }
}

pub(super) fn begin(
    game: &mut Game,
    revealer: Investigator,
    purpose: TokenPurpose,
) -> Result<(), RuleError> {
    if game.tokens.is_some() {
        return Err(RuleError("A token revelation is already pending.".into()));
    }
    let count = match purpose {
        TokenPurpose::SkillTest => 1,
        TokenPurpose::DarkFuture { .. } => 5,
        TokenPurpose::VoiceRa { .. } => 3,
        TokenPurpose::Premonition { .. } => 1,
    };
    let mut root = RevealNode::group(Policy::All, count);
    // Premonition replaces one position, preserving its entire sealed group
    // and the request's other positions (issue-89-premonition-followup.md).
    if let Some(sealed) = game.premonition.take() {
        root.children[0].policy = Policy::Sealed;
        root.children[0].sealed = Some(sealed);
    }
    let eyes_source = (purpose == TokenPurpose::SkillTest)
        .then(|| {
            game.test
                .as_ref()
                .and_then(|test| test.source_card.as_ref())
                .filter(|card| card.code == "11067")
                .map(|card| card.id)
        })
        .flatten();
    game.tokens = Some(TokenResolution {
        revealer,
        purpose,
        stage: if eyes_source.is_some() {
            TokenStage::EyesCharges
        } else {
            TokenStage::Declaration
        },
        jacqueline: false,
        revealed: vec![],
        selected: vec![],
        ignored: vec![],
        cancelled: vec![],
        root,
        active: vec![],
        notified_elders: vec![],
        eyes_source,
        previous_roots: vec![],
        batch_notified: false,
        symbols_done: vec![],
    });
    Ok(())
}

pub(super) fn begin_dark_future(
    game: &mut Game,
    who: Investigator,
    card: u16,
) -> Result<(), RuleError> {
    begin(
        game,
        who,
        TokenPurpose::DarkFuture {
            investigator: who,
            card,
        },
    )
}

pub(super) fn begin_voice_ra(
    game: &mut Game,
    who: Investigator,
    event: super::CardView,
) -> Result<(), RuleError> {
    begin(
        game,
        who,
        TokenPurpose::VoiceRa {
            investigator: who,
            event,
        },
    )?;
    game.players[who.index()].resources += 1;
    Ok(())
}

pub(super) fn premonition_choices(game: &Game) -> Vec<Choice> {
    if game.premonition.is_some() || game.tokens.is_some() {
        return vec![];
    }
    game.players
        .iter()
        .filter(|player| {
            !player.defeated
                && !player.resigned
                && !player.threats.iter().any(|card| card.code == "01165")
        })
        .flat_map(|player| {
            player
                .hand
                .iter()
                .filter(|card| card.code == "04199")
                .map(|card| {
                    choice(
                        format!("premonition:{}", card.id),
                        format!("{}: play Premonition", player.investigator.name()),
                    )
                })
        })
        .collect()
}

pub(super) fn play_premonition(game: &mut Game, id: u16) -> Result<(), RuleError> {
    if !premonition_choices(game)
        .iter()
        .any(|choice| choice.id == format!("premonition:{id}"))
    {
        return Err(RuleError("Premonition cannot be played now.".into()));
    }
    let owner = game
        .players
        .iter()
        .position(|player| player.hand.iter().any(|card| card.id == id))
        .expect("Offered owner");
    let player = &mut game.players[owner];
    let index = player
        .hand
        .iter()
        .position(|card| card.id == id)
        .expect("Offered event");
    let event = player.hand.remove(index);
    let investigator = player.investigator;
    begin(
        game,
        investigator,
        TokenPurpose::Premonition {
            investigator,
            event,
        },
    )
}

fn choice(id: impl Into<String>, label: impl Into<String>) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
        cancel_faces: vec![],
    }
}

fn path_name(path: &[usize]) -> String {
    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".")
}

fn path_value(value: &str) -> Vec<usize> {
    if value.is_empty() {
        vec![]
    } else {
        value
            .split('.')
            .map(|part| part.parse().expect("Offered reveal position"))
            .collect()
    }
}

fn protected(game: &Game, actor: Investigator, face: TokenFace) -> bool {
    game.players[actor.index()]
        .threats
        .iter()
        .any(|card| card.code == "60403")
        && matches!(
            face,
            TokenFace::ElderSign
                | TokenFace::Skull
                | TokenFace::Cultist
                | TokenFace::Tablet
                | TokenFace::AutoFail
        )
}

fn symbol(face: TokenFace) -> bool {
    matches!(
        face,
        TokenFace::Skull
            | TokenFace::Cultist
            | TokenFace::Tablet
            | TokenFace::AutoFail
            | TokenFace::ElderSign
            | TokenFace::Curse
    )
}

fn retained(node: &RevealNode, tokens: &TokenResolution) -> Vec<usize> {
    node.drawn
        .iter()
        .copied()
        .filter(|index| !tokens.ignored.contains(index) && !tokens.cancelled.contains(index))
        .collect()
}

fn cancellable(game: &Game, tokens: &TokenResolution) -> Vec<usize> {
    retained(tokens.root.at(&tokens.active), tokens)
        .into_iter()
        .filter(|&index| !protected(game, Investigator::Jacqueline, tokens.revealed[index]))
        .collect()
}

pub(super) fn decision_owner(game: &Game) -> Option<Investigator> {
    let tokens = game.tokens.as_ref()?;
    Some(if tokens.stage == TokenStage::Cancellation {
        Investigator::Jacqueline
    } else {
        tokens.revealer
    })
}

pub(super) fn choices(game: &Game) -> Vec<Choice> {
    let Some(tokens) = &game.tokens else {
        return vec![];
    };
    match tokens.stage {
        TokenStage::EyesCharges => {
            let available = game.players[tokens.revealer.index()]
                .assets
                .iter()
                .find(|asset| Some(asset.card.id) == tokens.eyes_source)
                .map_or(0, |asset| asset.uses.min(3));
            (0..=available)
                .map(|count| {
                    choice(
                        format!("tokens:eyes:{count}"),
                        format!(
                            "Spend {count} Eyes charges: reveal {} token(s), then choose 1",
                            count + 1
                        ),
                    )
                })
                .collect()
        }
        TokenStage::Declaration => {
            let mut result = vec![choice("test:reveal", "Resolve the declared token reveal")];
            let owner = &game.players[tokens.revealer.index()];
            let jacqueline = &game.players[Investigator::Jacqueline.index()];
            let jacqueline_available = game.jacqueline_uses == 0
                && !jacqueline.defeated
                && !jacqueline.resigned
                && jacqueline.location == owner.location;
            let mut positions = vec![];
            tokens.root.declarations(vec![], &mut positions);
            for (path, policy, used_jacqueline) in positions {
                let position = path_name(&path);
                if jacqueline_available
                    && !used_jacqueline
                    && policy != Policy::Katarina
                    && !tokens.root.at(&path).contains_sealed()
                {
                    let base = if path.is_empty() {
                        "tokens:jacqueline".to_owned()
                    } else {
                        format!("tokens:jacqueline:at:{position}")
                    };
                    result.push(choice(
                        &base,
                        format!(
                            "Jacqueline: add 2 reveals at {}",
                            if path.is_empty() {
                                "the outer reveal"
                            } else {
                                &position
                            }
                        ),
                    ));
                    for asset in &jacqueline.assets {
                        if asset.card.code == "60402" && asset.ready {
                            result.push(choice(format!("{base}:arbiter:{}", asset.card.id), "Use Jacqueline and exhaust Arbiter of Fates; preserve the round use"));
                        }
                    }
                }
                if policy != Policy::Draw {
                    continue;
                }
                if owner.resources >= 1 && !owner.threats.iter().any(|card| card.code == "01165") {
                    for card in &owner.hand {
                        if card.code == "04032" {
                            result.push(choice(format!("tokens:prophecy:{position}:{}", card.id), format!("Pay 1 for Dark Prophecy at reveal {position}: reveal 5 instead of 1")));
                        }
                    }
                }
                for asset in &owner.assets {
                    if !asset.ready {
                        continue;
                    }
                    match asset.card.code.as_str() {
                        "04197" => result.push(choice(
                            format!("tokens:olive:{position}:{}", asset.card.id),
                            format!("Exhaust Olive at reveal {position}: reveal 3 and resolve 2"),
                        )),
                        "11069" if tokens.purpose == TokenPurpose::SkillTest => {
                            result.push(choice(
                                format!("tokens:katarina:{position}:{}", asset.card.id),
                                format!(
                                    "Exhaust Katarina at reveal {position}: reveal until a symbol"
                                ),
                            ))
                        }
                        _ => {}
                    }
                }
            }
            result
        }
        TokenStage::Cancellation => {
            if let Some(choices) = groups::choices(game, tokens) {
                return choices;
            }
            let eligible = cancellable(game, tokens);
            let auto_selected = tokens
                .selected
                .iter()
                .any(|&index| tokens.revealed[index] == TokenFace::AutoFail);
            let mut result = vec![];
            for index in eligible.iter().copied() {
                let selected = tokens.selected.contains(&index);
                let face = tokens.revealed[index];
                if selected
                    || (!auto_selected
                        && tokens.selected.len() < 2
                        && (face != TokenFace::AutoFail || tokens.selected.is_empty()))
                {
                    result.push(choice(
                        format!("tokens:cancel:{index}"),
                        format!(
                            "{} {} (token {})",
                            if selected { "Keep" } else { "Cancel" },
                            face.label(),
                            index + 1
                        ),
                    ));
                }
            }
            let non_auto = eligible
                .iter()
                .filter(|&&index| tokens.revealed[index] != TokenFace::AutoFail)
                .count();
            if auto_selected
                || (!tokens.selected.is_empty() && tokens.selected.len() == non_auto.min(2))
            {
                result.push(choice(
                    "tokens:confirm",
                    "Confirm Jacqueline's cancellations",
                ));
            }
            result
        }
        TokenStage::Selection => {
            if let Some(choices) = groups::choices(game, tokens) {
                return choices;
            }
            let node = tokens.root.at(&tokens.active);
            if node.eyes_with_groups() {
                let mut result = vec![];
                for (position, child) in node.children.iter().enumerate() {
                    let members = retained(child, tokens);
                    if members.is_empty() {
                        continue;
                    }
                    let selected = tokens.selected == members;
                    if selected || tokens.selected.is_empty() {
                        let faces = members
                            .iter()
                            .map(|&index| tokens.revealed[index].label())
                            .collect::<Vec<_>>()
                            .join(", ");
                        result.push(choice(
                            format!("tokens:position:{position}"),
                            format!(
                                "{} Eyes position {}: {} [{}]",
                                if selected { "Unselect" } else { "Choose" },
                                position + 1,
                                if child.policy == Policy::Olive && !child.selection_done {
                                    "Olive group; then choose 2"
                                } else if child.policy == Policy::Sealed {
                                    "Premonition group"
                                } else {
                                    "token"
                                },
                                faces
                            ),
                        ));
                    }
                }
                if !tokens.selected.is_empty() {
                    result.push(choice("tokens:confirm", "Confirm Eyes position"));
                }
                return result;
            }
            let available = retained(node, tokens);
            let preferred: Vec<_> = available
                .iter()
                .copied()
                .filter(|&index| {
                    symbol(tokens.revealed[index]) && tokens.revealed[index] != TokenFace::ElderSign
                })
                .collect();
            let eligible = if node.policy == Policy::Prophecy && !preferred.is_empty() {
                preferred
            } else {
                available.clone()
            };
            let required = if node.policy == Policy::Olive {
                2.min(available.len())
            } else {
                1.min(available.len())
            };
            let mut result = vec![];
            for index in eligible {
                let selected = tokens.selected.contains(&index);
                if selected || tokens.selected.len() < required {
                    result.push(choice(
                        format!("tokens:keep:{index}"),
                        format!(
                            "{} {} (token {})",
                            if selected { "Unselect" } else { "Resolve" },
                            tokens.revealed[index].label(),
                            index + 1
                        ),
                    ));
                }
            }
            if tokens.selected.len() == required {
                result.push(choice("tokens:confirm", "Confirm tokens to resolve"));
            }
            result
        }
        TokenStage::Effects => pending_symbols(game, tokens)
            .into_iter()
            .map(|index| {
                choice(
                    format!("tokens:effect:{index}"),
                    format!(
                        "Resolve {} token {} next",
                        tokens.revealed[index].label(),
                        index + 1
                    ),
                )
            })
            .collect(),
        TokenStage::Resolving => vec![],
    }
}

struct Pool<'a> {
    revealed: &'a mut Vec<TokenFace>,
    ignored: &'a mut Vec<usize>,
    cancelled: &'a mut Vec<usize>,
}

impl Pool<'_> {
    fn retained(&self, node: &RevealNode) -> Vec<usize> {
        node.drawn
            .iter()
            .copied()
            .filter(|index| !self.ignored.contains(index) && !self.cancelled.contains(index))
            .collect()
    }
}

// The predeclared batch is revealed simultaneously (Legacy FAQ 2.5 p17).
// Selection does not expose later randomness incrementally.
fn draw_node(game: &mut Game, node: &mut RevealNode, pool: &mut Pool<'_>) -> Result<(), RuleError> {
    if node.policy == Policy::Sealed {
        let sealed = node.sealed.take().expect("Pending mandatory replacement");
        for face in sealed.faces {
            node.drawn.push(pool.revealed.len());
            pool.revealed.push(face);
        }
        if !node.sealed_card_departed {
            game.players[sealed.controller.index()]
                .discard
                .push(sealed.card);
        }
        game.history.push("Premonition supplies its sealed token(s) instead of this bag reveal, then is discarded. Its mandatory replacement precedes optional reactions to that original reveal. [04199; RR replacement effects; Legacy FAQ 2.5]".into());
        node.done = true;
        return Ok(());
    }
    if node.policy == Policy::Draw {
        if !game.bag.is_empty() {
            node.drawn.push(pool.revealed.len());
            pool.revealed.push(game.draw_token()?);
        }
        node.done = true;
        return Ok(());
    }
    if node.policy == Policy::Katarina {
        while !game.bag.is_empty() {
            let index = pool.revealed.len();
            let face = game.draw_token()?;
            pool.revealed.push(face);
            node.drawn.push(index);
            if symbol(face) {
                break;
            }
            pool.ignored.push(index);
        }
        node.done = true;
        return Ok(());
    }
    for child in &mut node.children {
        draw_node(game, child, pool)?;
        node.drawn.extend(child.drawn.iter().copied());
    }
    Ok(())
}

fn advance_node(
    game: &mut Game,
    node: &mut RevealNode,
    pool: &mut Pool<'_>,
    revealer: Investigator,
    path: Vec<usize>,
) -> Result<Option<(Vec<usize>, TokenStage)>, RuleError> {
    if node.done {
        return Ok(None);
    }
    if node.eyes_with_groups()
        && !node.selection_done
        && (!node.jacqueline || node.cancellation_done)
        && !pool.retained(node).is_empty()
    {
        return Ok(Some((path, TokenStage::Selection)));
    }
    while node.cursor < node.children.len() {
        let mut child_path = path.clone();
        child_path.push(node.cursor);
        if let Some(pending) = advance_node(
            game,
            &mut node.children[node.cursor],
            pool,
            revealer,
            child_path,
        )? {
            return Ok(Some(pending));
        }
        node.cursor += 1;
    }
    if node.jacqueline && !node.cancellation_done {
        let eligible = pool
            .retained(node)
            .into_iter()
            .any(|index| !protected(game, Investigator::Jacqueline, pool.revealed[index]));
        if eligible {
            return Ok(Some((path, TokenStage::Cancellation)));
        }
        node.cancellation_done = true;
    }
    if matches!(node.policy, Policy::Prophecy | Policy::Olive | Policy::Eyes)
        && !node.selection_done
    {
        let available = pool.retained(node);
        if !available.is_empty() && !(node.policy == Policy::Olive && available.len() <= 2) {
            return Ok(Some((path, TokenStage::Selection)));
        }
        node.chosen = available;
        node.selection_done = true;
    }
    if let Some(event) = node.event.take() {
        game.players[revealer.index()].discard.push(event);
    }
    node.done = true;
    Ok(None)
}

pub(super) fn resume(game: &mut Game) -> Result<TokenStep, RuleError> {
    let mut tokens = game
        .tokens
        .take()
        .ok_or_else(|| RuleError("No token reveal is pending.".into()))?;
    if tokens.purpose == TokenPurpose::SkillTest && tokens.revealer == Investigator::Jacqueline {
        let pending = tokens
            .ignored
            .iter()
            .chain(&tokens.cancelled)
            .copied()
            .find(|index| {
                tokens.revealed[*index] == TokenFace::ElderSign
                    && !tokens.notified_elders.contains(index)
            });
        if let Some(index) = pending {
            tokens.notified_elders.push(index);
            tokens.stage = TokenStage::Resolving;
            game.tokens = Some(tokens);
            return Ok(TokenStep::DrawElderSign);
        }
    }
    let mut pool = Pool {
        revealed: &mut tokens.revealed,
        ignored: &mut tokens.ignored,
        cancelled: &mut tokens.cancelled,
    };
    if let Some((path, stage)) =
        advance_node(game, &mut tokens.root, &mut pool, tokens.revealer, vec![])?
    {
        tokens.active = path;
        tokens.stage = stage;
        tokens.selected.clear();
        game.tokens = Some(tokens);
        return Ok(TokenStep::Pending);
    }
    if tokens.purpose != TokenPurpose::SkillTest
        && (!tokens.ignored.is_empty() || !tokens.cancelled.is_empty())
    {
        let mut play = game.observe();
        play.tokens = Some(tokens.clone());
        game.capture_public_view(
            "The selected chaos tokens remain after ignored and cancelled tokens are excluded.",
            play,
        );
    }
    if tokens.purpose == TokenPurpose::SkillTest {
        if !tokens.batch_notified {
            tokens.batch_notified = true;
            tokens.stage = TokenStage::Resolving;
            let faces = tokens
                .root
                .drawn
                .iter()
                .filter(|index| {
                    !tokens.ignored.contains(index) && !tokens.cancelled.contains(index)
                })
                .map(|&index| tokens.revealed[index])
                .collect();
            game.tokens = Some(tokens);
            return Ok(TokenStep::RevealedBatch { faces });
        }
        let effects = pending_symbols(game, &tokens);
        if let [index] = effects.as_slice() {
            return resolve_symbol(game, tokens, *index);
        }
        if !effects.is_empty() {
            tokens.stage = TokenStage::Effects;
            game.tokens = Some(tokens);
            return Ok(TokenStep::Pending);
        }
    }
    let eyes_match = tokens
        .previous_roots
        .iter()
        .chain(std::iter::once(&tokens.root))
        .any(|root| {
            root.policy == Policy::Eyes
                && root.chosen.iter().any(|chosen| {
                    !tokens.ignored.contains(chosen)
                        && !tokens.cancelled.contains(chosen)
                        && tokens
                            .ignored
                            .iter()
                            .any(|ignored| tokens.revealed[*ignored] == tokens.revealed[*chosen])
                })
        });
    let (mut resolved, mut ignored, mut cancelled) = (vec![], vec![], vec![]);
    for (index, face) in tokens.revealed.into_iter().enumerate() {
        if tokens.cancelled.contains(&index) {
            cancelled.push(face);
        } else if tokens.ignored.contains(&index) {
            ignored.push(face);
        } else {
            resolved.push(face);
        }
    }
    match tokens.purpose {
        TokenPurpose::SkillTest => {
            game.history.push(format!("Token revelation completes: {} resolved, {} ignored, {} cancelled. Physical tokens stay out of the bag until cleanup; resolved curses return to the token pool at ST8. [RR ST.8; Innsmouth p1; FAQ 2.5]", resolved.len(), ignored.len(), cancelled.len()));
            Ok(TokenStep::Complete {
                resolved,
                ignored,
                cancelled,
                eyes_match,
                symbols_applied: true,
            })
        }
        TokenPurpose::DarkFuture { investigator, card } => {
            if resolved.contains(&TokenFace::ElderSign) {
                let owner = &mut game.players[investigator.index()];
                if let Some(index) = owner.threats.iter().position(|held| held.id == card) {
                    owner.discard.push(owner.threats.remove(index));
                }
                game.history.push("Dark Future's own turn-end reveal retains Elder Sign: discard Dark Future. This is not a skill test. [60403]".into());
            }
            game.bag.extend(resolved);
            game.bag.extend(ignored);
            game.bag.extend(cancelled);
            Ok(TokenStep::Finished)
        }
        TokenPurpose::VoiceRa {
            investigator,
            event,
        } => {
            let symbols = resolved.iter().filter(|&&face| symbol(face)).count() as u8;
            game.players[investigator.index()].resources += 2 * symbols;
            game.history.push(format!("Voice of Ra counts {symbols} retained symbols under taboo 10. Gain {} additional resources; this is not a skill test, so a curse returns without an extra reveal. [60416]", 2 * symbols));
            game.players[investigator.index()].discard.push(event);
            game.bag.extend(resolved);
            game.bag.extend(ignored);
            game.bag.extend(cancelled);
            Ok(TokenStep::Finished)
        }
        TokenPurpose::Premonition {
            investigator,
            event,
        } => {
            game.premonition = Some(SealedPremonition {
                controller: investigator,
                card: event,
                faces: resolved,
            });
            game.bag.extend(ignored);
            game.bag.extend(cancelled);
            Ok(TokenStep::Finished)
        }
    }
}

fn pending_symbols(game: &Game, tokens: &TokenResolution) -> Vec<usize> {
    tokens
        .revealed
        .iter()
        .enumerate()
        .filter_map(|(index, face)| {
            if tokens.ignored.contains(&index)
                || tokens.cancelled.contains(&index)
                || tokens.symbols_done.contains(&index)
            {
                return None;
            }
            let applies = match face {
                TokenFace::Curse => true,
                TokenFace::Tablet => {
                    game.scenario
                        .ghouls_at(&game.players[tokens.revealer.index()].location)
                        > 0
                }
                TokenFace::ElderSign if tokens.revealer == Investigator::Tony => game.players[0]
                    .assets
                    .iter()
                    .any(|asset| asset.card.code == "06010"),
                _ => false,
            };
            applies.then_some(index)
        })
        .collect()
}

fn resolve_symbol(
    game: &mut Game,
    mut tokens: TokenResolution,
    index: usize,
) -> Result<TokenStep, RuleError> {
    tokens.symbols_done.push(index);
    match tokens.revealed[index] {
        TokenFace::Curse => {
            tokens.previous_roots.push(std::mem::replace(
                &mut tokens.root,
                RevealNode::group(Policy::All, 1),
            ));
            tokens.active.clear();
            tokens.selected.clear();
            tokens.batch_notified = false;
            tokens.stage = TokenStage::Declaration;
            game.history.push("Resolve a curse: -2 and reveal another token. The additional reveal has its own would-reveal opportunity; the curse stays held until test cleanup. [Innsmouth p1]".into());
            game.tokens = Some(tokens);
            Ok(TokenStep::Pending)
        }
        TokenFace::Tablet => {
            tokens.stage = TokenStage::Resolving;
            game.tokens = Some(tokens);
            Ok(TokenStep::ApplyTablet)
        }
        TokenFace::ElderSign => {
            let contracts = game.players[0]
                .assets
                .iter_mut()
                .find(|asset| asset.card.code == "06010")
                .expect("Tony's Contracts");
            contracts.uses = contracts
                .uses
                .checked_add(1)
                .ok_or_else(|| RuleError("Bounty counter exceeds the supported range.".into()))?;
            game.history.push(
                "Tony resolves elder sign and places 1 bounty on Bounty Contracts. [06003]".into(),
            );
            tokens.stage = TokenStage::Resolving;
            game.tokens = Some(tokens);
            game.capture_presentation("Tony's elder sign places one bounty on Bounty Contracts.");
            resume(game)
        }
        _ => unreachable!("Selected symbol consequence"),
    }
}

pub(super) fn choose(game: &mut Game, id: &str) -> Result<TokenStep, RuleError> {
    if let Some(index) = id.strip_prefix("tokens:effect:") {
        let index: usize = index
            .parse()
            .map_err(|_| RuleError("Invalid token consequence.".into()))?;
        if !choices(game).iter().any(|choice| choice.id == id) {
            return Err(RuleError("That token consequence is unavailable.".into()));
        }
        let tokens = game.tokens.take().expect("Pending token effect");
        return resolve_symbol(game, tokens, index);
    }
    if !choices(game).iter().any(|choice| choice.id == id) {
        return Err(RuleError("That token choice is unavailable.".into()));
    }
    if id == "test:reveal" {
        let mut tokens = game.tokens.take().expect("Pending reveal");
        let mut pool = Pool {
            revealed: &mut tokens.revealed,
            ignored: &mut tokens.ignored,
            cancelled: &mut tokens.cancelled,
        };
        draw_node(game, &mut tokens.root, &mut pool)?;
        tokens.stage = TokenStage::Resolving;
        game.tokens = Some(tokens);
        game.capture_presentation(
            "Physical chaos tokens are revealed before their effects resolve.",
        );
        return resume(game);
    }
    if let Some(count) = id.strip_prefix("tokens:eyes:") {
        let count: u8 = count.parse().expect("Offered charge count");
        let tokens = game.tokens.as_mut().expect("Eyes reveal");
        if let Some(asset) = game.players[tokens.revealer.index()]
            .assets
            .iter_mut()
            .find(|asset| Some(asset.card.id) == tokens.eyes_source)
        {
            asset.uses -= count;
        }
        let prior = tokens.root.children.remove(0);
        tokens.root = RevealNode::group(Policy::Eyes, usize::from(count) + 1);
        tokens.root.children[0] = prior;
        tokens.stage = TokenStage::Declaration;
        return Ok(TokenStep::Pending);
    }
    if id.starts_with("tokens:jacqueline") {
        let value = id
            .strip_prefix("tokens:jacqueline")
            .expect("Jacqueline choice");
        let (target, arbiter) = value
            .split_once(":arbiter:")
            .map_or((value, None), |(target, id)| {
                (target, Some(id.parse::<u16>().expect("Offered asset")))
            });
        let path = target
            .strip_prefix(":at:")
            .map_or_else(Vec::new, path_value);
        if let Some(id) = arbiter {
            game.players[Investigator::Jacqueline.index()]
                .assets
                .iter_mut()
                .find(|asset| asset.card.id == id)
                .expect("Offered Arbiter")
                .ready = false;
        } else {
            game.jacqueline_uses += 1;
        }
        let tokens = game.tokens.as_mut().expect("Pending reveal");
        let node = tokens.root.at_mut(&path);
        if node.policy == Policy::Draw {
            *node = RevealNode::group(Policy::All, 1);
        }
        node.children
            .extend([RevealNode::draw(), RevealNode::draw()]);
        node.jacqueline = true;
        tokens.jacqueline = true;
        return Ok(TokenStep::Pending);
    }
    for (prefix, policy, count) in [
        ("tokens:prophecy:", Policy::Prophecy, 5),
        ("tokens:olive:", Policy::Olive, 3),
        ("tokens:katarina:", Policy::Katarina, 0),
    ] {
        if let Some(value) = id.strip_prefix(prefix) {
            let (path, card) = value.rsplit_once(':').expect("Offered replacement");
            let path = path_value(path);
            let card: u16 = card.parse().expect("Offered card");
            let revealer = game.tokens.as_ref().expect("Pending reveal").revealer;
            let owner = &mut game.players[revealer.index()];
            let event = if policy == Policy::Prophecy {
                owner.resources -= 1;
                let index = owner
                    .hand
                    .iter()
                    .position(|held| held.id == card)
                    .expect("Offered event");
                Some(owner.hand.remove(index))
            } else {
                owner
                    .assets
                    .iter_mut()
                    .find(|asset| asset.card.id == card)
                    .expect("Offered asset")
                    .ready = false;
                None
            };
            let node = game
                .tokens
                .as_mut()
                .expect("Pending reveal")
                .root
                .at_mut(&path);
            *node = RevealNode::group(policy, count);
            node.event = event;
            return Ok(TokenStep::Pending);
        }
    }
    if groups::toggle(game.tokens.as_mut().expect("Pending token choice"), id) {
        return Ok(TokenStep::Pending);
    }
    if let Some(position) = id.strip_prefix("tokens:position:") {
        let position: usize = position.parse().expect("Offered Eyes position");
        let tokens = game.tokens.as_mut().expect("Pending Eyes selection");
        let drawn = retained(&tokens.root.at(&tokens.active).children[position], tokens);
        tokens.selected = if tokens.selected == drawn {
            vec![]
        } else {
            drawn
        };
        return Ok(TokenStep::Pending);
    }
    if let Some(index) = id
        .strip_prefix("tokens:cancel:")
        .or_else(|| id.strip_prefix("tokens:keep:"))
    {
        let index = index.parse().expect("Offered token");
        let selected = &mut game.tokens.as_mut().expect("Pending reveal").selected;
        if let Some(at) = selected.iter().position(|&old| old == index) {
            selected.remove(at);
        } else {
            selected.push(index);
            selected.sort_unstable();
        }
        return Ok(TokenStep::Pending);
    }
    if id == "tokens:confirm" {
        let mut tokens = game.tokens.take().expect("Pending reveal");
        match tokens.stage {
            TokenStage::Cancellation => {
                tokens
                    .cancelled
                    .extend(tokens.selected.iter().copied().filter(|&index| {
                        !protected(game, Investigator::Jacqueline, tokens.revealed[index])
                    }));
                tokens.root.at_mut(&tokens.active).cancellation_done = true;
            }
            TokenStage::Selection => {
                let available = retained(tokens.root.at(&tokens.active), &tokens);
                for index in available {
                    if !tokens.selected.contains(&index)
                        && !protected(game, tokens.revealer, tokens.revealed[index])
                    {
                        tokens.ignored.push(index);
                    }
                }
                tokens.root.at_mut(&tokens.active).selection_done = true;
                tokens.root.at_mut(&tokens.active).chosen = tokens.selected.clone();
            }
            _ => return Err(RuleError("No token selection is pending.".into())),
        }
        tokens.selected.clear();
        tokens.stage = TokenStage::Resolving;
        game.tokens = Some(tokens);
        return resume(game);
    }
    Err(RuleError("That token choice is unavailable.".into()))
}
