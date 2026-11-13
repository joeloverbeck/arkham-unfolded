//! Fixed definitions for the selected Gathering party; metadata is not behavior support.
//!
//! Original English identities: archive ArkhamDB snapshot d8cc0e7e9081c0171298f8b1c07c90ddc2b08662.
//! Deck membership and quantities remain owned by the pinned preparation inputs.
//! Effective text applies Legacy FAQ 2.5 Katarina errata and taboo 10 to Quick
//! Thinking, Dark Prophecy and Voice of Ra. See the three issue-89-rules-* notes
//! for source/face bindings, precedence and interpretation limits. Keyword/rules
//! clarifications still belong to the rules implementation, not a text parser.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum CardKind {
    Investigator,
    Asset,
    Event,
    Skill,
    Enemy,
    Treachery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Faction {
    Guardian,
    Rogue,
    Mystic,
    Survivor,
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Slot {
    Hand,
    Arcane,
    Ally,
    Accessory,
    Body,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseKind {
    Ammo,
    Charge,
    Supply,
    Resource,
    Offering,
    Bounty,
    Key,
}

impl UseKind {
    pub const ALL: [Self; 7] = [
        Self::Ammo,
        Self::Charge,
        Self::Supply,
        Self::Resource,
        Self::Offering,
        Self::Bounty,
        Self::Key,
    ];

    /// The name of these counters on a card, as Inspect and the table say it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ammo => "Ammo",
            Self::Charge => "Charges",
            Self::Supply => "Supplies",
            Self::Resource => "Resources",
            Self::Offering => "Offerings",
            Self::Bounty => "Bounties",
            Self::Key => "Keys",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EnemyValues {
    pub combat: u8,
    pub evade: u8,
    pub attack_damage: u8,
    pub attack_horror: u8,
    pub health_per_investigator: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CardDefinition {
    pub code: &'static str,
    pub title: &'static str,
    pub kind: CardKind,
    pub factions: &'static [Faction],
    pub cost: Option<u8>,
    /// Printed level. Signatures, weaknesses and investigators have no level.
    pub level: Option<u8>,
    /// Printed commitment icons: willpower, intellect, combat, agility, wild.
    pub icons: [u8; 5],
    /// Investigator base skills are distinct from commitment icons.
    pub base_skills: Option<[u8; 4]>,
    pub traits: &'static [&'static str],
    pub slots: &'static [Slot],
    pub unique: bool,
    pub weakness: bool,
    pub permanent: bool,
    pub uses: Option<(UseKind, u8)>,
    pub health: Option<u8>,
    pub sanity: Option<u8>,
    pub enemy: Option<EnemyValues>,
    /// The printed Peril keyword: while the card's drawing resolves, its
    /// investigator cannot confer with the other players (RR p.16).
    pub peril: bool,
    pub effective_text: &'static str,
}

impl CardDefinition {
    pub fn has_trait(&self, name: &str) -> bool {
        self.traits.contains(&name)
    }
}

/// Whether the card with `code` carries the printed Peril keyword.
pub(super) fn peril(code: &str) -> bool {
    definition(code).is_some_and(|card| card.peril)
}

// Each invocation supplies the printed fields; struct updates name the extra
// properties of that actual card. No runtime prose interpretation is involved.
macro_rules! card {
    ($code:literal, $title:literal, $kind:ident, [$($faction:ident),+], $cost:expr,
     $level:expr, $icons:expr, $traits:expr, $text:literal) => {
        CardDefinition {
            code: $code,
            title: $title,
            kind: CardKind::$kind,
            factions: &[$(Faction::$faction),+],
            cost: $cost,
            level: $level,
            icons: $icons,
            base_skills: None,
            traits: $traits,
            slots: &[],
            unique: false,
            weakness: false,
            permanent: false,
            uses: None,
            health: None,
            sanity: None,
            enemy: None,
            peril: false,
            effective_text: $text,
        }
    };
}

pub(super) fn definition(code: &str) -> Option<&'static CardDefinition> {
    CARDS.iter().find(|card| card.code == code)
}

const CARDS: &[CardDefinition] = &[
    card!(
        "01049",
        "Hard Knocks",
        Asset,
        [Rogue],
        Some(2),
        Some(0),
        [0, 0, 1, 1, 0],
        &["Talent"],
        "[fast] Spend 1 resource: You get +1 [combat] for this skill test.\n[fast] Spend 1 resource: You get +1 [agility] for this skill test."
    ),
    card!(
        "01065",
        "Ward of Protection",
        Event,
        [Mystic],
        Some(1),
        Some(0),
        [0, 0, 0, 0, 1],
        &["Spell", "Spirit"],
        "Fast. Play when you draw a non-weakness treachery card.\nCancel that card's revelation effect. Then, take 1 horror."
    ),
    card!(
        "01079",
        "\"Look what I found!\"",
        Event,
        [Survivor],
        Some(2),
        Some(0),
        [0, 2, 0, 0, 0],
        &["Fortune"],
        "Fast. Play after you fail a skill test by 2 or less while investigating.\nDiscover 2 clues in your location."
    ),
    card!(
        "01080",
        "Lucky!",
        Event,
        [Survivor],
        Some(1),
        Some(0),
        [0; 5],
        &["Fortune"],
        "Fast. Play when you would fail a skill test.\nGet +2 to your skill value for that test."
    ),
    card!(
        "01089",
        "Guts",
        Skill,
        [Neutral],
        None,
        Some(0),
        [2, 0, 0, 0, 0],
        &["Innate"],
        "Max 1 committed per skill test.\nIf this test is successful, draw 1 card."
    ),
    card!(
        "01093",
        "Unexpected Courage",
        Skill,
        [Neutral],
        None,
        Some(0),
        [0, 0, 0, 0, 2],
        &["Innate"],
        "Max 1 committed per skill test."
    ),
    CardDefinition {
        slots: &[Slot::Ally],
        unique: true,
        health: Some(1),
        sanity: Some(2),
        ..card!(
            "02033",
            "Peter Sylvestre",
            Asset,
            [Survivor],
            Some(3),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Ally", "Miskatonic"],
            "You get +1 [agility].\n[reaction] After your turn ends: Heal 1 horror from Peter Sylvestre."
        )
    },
    CardDefinition {
        weakness: true,
        ..card!(
            "02038",
            "Internal Injury",
            Treachery,
            [Neutral],
            None,
            None,
            [0; 5],
            &["Injury"],
            "Revelation - Place Internal Injury into play in your threat area.\nForced - At the end of your turn: Take 1 direct damage.\n[action][action]: Discard Internal Injury."
        )
    },
    CardDefinition {
        weakness: true,
        ..card!(
            "02039",
            "Chronophobia",
            Treachery,
            [Neutral],
            None,
            None,
            [0; 5],
            &["Madness"],
            "Revelation - Place Chronophobia into play in your threat area.\nForced - At the end of your turn: Take 1 direct horror.\n[action][action]: Discard Chronophobia."
        )
    },
    card!(
        "02192",
        "Rise to the Occasion",
        Skill,
        [Survivor],
        None,
        Some(0),
        [0, 0, 0, 0, 3],
        &["Innate"],
        "Commit only to a skill test you are performing, and only if the difficulty of that test is at least 2 higher than your base skill value."
    ),
    card!(
        "02225",
        "\"If it bleeds...\"",
        Event,
        [Guardian],
        Some(1),
        Some(0),
        [1, 0, 1, 0, 0],
        &[],
        "Fast. Play after you defeat a Monster enemy.\nEach investigator at your location heals horror equal to that enemy's horror value."
    ),
    // Taboo 10 adds a maximum across copies; it does not change printed level.
    card!(
        "02229",
        "Quick Thinking",
        Skill,
        [Rogue],
        None,
        Some(0),
        [0, 0, 0, 0, 1],
        &["Innate"],
        "If this skill test is successful by 2 or more, after it resolves, you may immediately take an action as if it were your turn (this action does not count toward the number of actions you can take each turn). (max once per round)."
    ),
    card!(
        "03033",
        "Uncage the Soul",
        Event,
        [Mystic],
        Some(0),
        Some(0),
        [2, 0, 0, 0, 0],
        &["Spirit"],
        "Play a Spell or Ritual card from your hand, reducing its resource cost by 3."
    ),
    card!(
        "03039",
        "Resourceful",
        Skill,
        [Survivor],
        None,
        Some(0),
        [0, 1, 1, 1, 0],
        &["Innate"],
        "If this skill test is successful, choose a [survivor] card not named Resourceful in your discard pile. Add the chosen card to your hand."
    ),
    card!(
        "03233",
        "\"Watch this!\"",
        Skill,
        [Rogue],
        None,
        Some(0),
        [1, 0, 1, 1, 0],
        &["Gambit"],
        "Commit only to a skill test you are performing. As an additional cost to commit \"Watch this!\" to a skill test, spend up to 3 resources.\nIf you succeed by 1 or more, gain twice that many resources."
    ),
    card!(
        "03263",
        "On the Hunt",
        Event,
        [Guardian],
        Some(1),
        Some(0),
        [0, 1, 1, 0, 0],
        &["Tactic"],
        "Fast. Play when you would draw an encounter card during the mythos phase.\nInstead, search the top 9 cards of the encounter deck for an enemy, spawn it engaged with you (instead of its normal spawn location), and shuffle the encounter deck. If you cannot, draw the top card of the encounter deck."
    ),
    CardDefinition {
        slots: &[Slot::Arcane],
        uses: Some((UseKind::Charge, 4)),
        ..card!(
            "04029",
            "Mists of R'lyeh",
            Asset,
            [Mystic],
            Some(2),
            Some(0),
            [0, 0, 0, 1, 0],
            &["Spell"],
            "Uses (4 charges).\n[action] Spend 1 charge: Evade. This evasion attempt uses [willpower] instead of [agility]. If you succeed, after evading the chosen enemy, you may move to a connecting location. If a [skull], [cultist], [tablet], [elder_thing], or [auto_fail] symbol is revealed during this evasion attempt, choose and discard a card from your hand."
        )
    },
    // Taboo 10 broadens the preferred category, while excluding elder sign.
    card!(
        "04032",
        "Dark Prophecy",
        Event,
        [Mystic],
        Some(1),
        Some(0),
        [1, 0, 0, 1, 0],
        &["Augury"],
        "Fast. Play when you would reveal a chaos token.\nReveal 5 chaos tokens instead of 1. Choose one of those tokens with a non-[elder_sign] symbol to resolve, and ignore the rest. (If no such token is revealed, choose any one of those tokens to resolve, and ignore the rest.)"
    ),
    CardDefinition {
        slots: &[Slot::Body],
        ..card!(
            "04037",
            "Backpack",
            Asset,
            [Neutral],
            Some(2),
            Some(0),
            [0, 0, 0, 1, 0],
            &["Item"],
            "[reaction] After Backpack enters play: Search the top 6 cards of your deck for up to 3 non-weakness Item or Supply cards, and attach them facedown to Backpack. Shuffle your deck.\nCards attached to Backpack may be played as if they were in your hand. If there are no cards attached to Backpack, discard it."
        )
    },
    card!(
        "04103",
        "Scene of the Crime",
        Event,
        [Guardian],
        Some(2),
        Some(0),
        [0, 1, 1, 0, 0],
        &["Insight", "Bold"],
        "Play only as your first action.\nDiscover 1 clue at your location (2 clues instead if there is an enemy at that location). This action does not provoke attacks of opportunity."
    ),
    CardDefinition {
        slots: &[Slot::Accessory],
        ..card!(
            "04107",
            "Lucky Cigarette Case",
            Asset,
            [Rogue],
            Some(2),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Item", "Charm"],
            "[reaction] After you succeed at a skill test by 2 or more, exhaust Lucky Cigarette Case: Draw 1 card."
        )
    },
    CardDefinition {
        permanent: true,
        ..card!(
            "04109",
            "Arcane Research",
            Asset,
            [Mystic],
            None,
            Some(0),
            [0; 5],
            &["Talent"],
            "Permanent.\nWhen you purchase Arcane Research, suffer 1 mental trauma.\nAfter each scenario of a campaign, reduce the experience cost of the first Spell card you upgrade before the next scenario by 1."
        )
    },
    card!(
        "04150",
        "Take the Initiative",
        Skill,
        [Guardian],
        None,
        Some(0),
        [0, 0, 0, 0, 3],
        &["Practiced", "Bold"],
        "Commit only to a skill test you are performing.\nTake the Initiative loses [wild] for each action that has been completed by any investigator this phase."
    ),
    CardDefinition {
        slots: &[Slot::Ally],
        unique: true,
        health: Some(1),
        sanity: Some(3),
        ..card!(
            "04197",
            "Olive McBride",
            Asset,
            [Mystic],
            Some(2),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Ally", "Witch"],
            "[reaction] When you would reveal a chaos token, exhaust Olive McBride: Reveal 3 chaos tokens instead of 1. Choose 2 of those tokens to resolve, and ignore the other."
        )
    },
    card!(
        "04199",
        "Premonition",
        Event,
        [Mystic],
        Some(0),
        Some(0),
        [0, 1, 0, 1, 0],
        &["Augury"],
        "Fast. Play during any [fast] player window.\nPut Premonition into play, reveal a random chaos token from the chaos bag, and seal it on Premonition.\nForced - When a chaos token would be revealed from the chaos bag: Resolve the token sealed here as if it were just revealed from the chaos bag, instead. Then, discard Premonition."
    ),
    card!(
        "04200",
        "Live and Learn",
        Event,
        [Survivor],
        Some(0),
        Some(0),
        [0, 0, 0, 0, 1],
        &["Spirit"],
        "Fast. Play after a skill test you failed ends (after resolving all effects from the failed test).\nAttempt that test again. You get +2 skill value for this test."
    ),
    card!(
        "04201",
        "Take Heart",
        Skill,
        [Survivor],
        None,
        Some(0),
        [0; 5],
        &["Innate"],
        "Max 1 committed per skill test.\nYou may commit Take Heart to any type of test.\nIf this test fails, the performing investigator draws 2 cards and gains 2 resources."
    ),
    card!(
        "04272",
        "Winging It",
        Event,
        [Survivor],
        Some(1),
        Some(0),
        [0; 5],
        &["Tactic", "Improvised"],
        "You may play Winging It from your discard pile. If you do, shuffle it into your deck after resolving its effects.\nInvestigate. Your location gets -1 shroud for this investigation. (If you played Winging It from your discard pile, discover 1 additional clue if you succeed.)"
    ),
    card!(
        "05032",
        "Deny Existence",
        Event,
        [Mystic],
        Some(0),
        Some(0),
        [0, 0, 0, 0, 1],
        &["Spell", "Paradox"],
        "Fast. Play when an encounter card or an enemy attack would cause you to do one of the following (choose one): Discard cards from hand, lose resources, lose actions, take damage, or take horror.\nYou ignore that aspect of the effect. (Each other aspect resolves normally.)"
    ),
    CardDefinition {
        weakness: true,
        ..card!(
            "05041",
            "The 13th Vision",
            Treachery,
            [Neutral],
            None,
            None,
            [0; 5],
            &["Omen"],
            "Revelation - Put The 13th Vision into play in your threat area.\nInvestigators at your location fail ties during skill tests.\n[action][action]: Discard The 13th Vision."
        )
    },
    card!(
        "05111",
        "Intel Report",
        Event,
        [Rogue],
        Some(2),
        Some(0),
        [0, 2, 0, 0, 0],
        &["Favor", "Service"],
        "Discover 1 clue at your location.\n[reaction] When you play Intel Report, increase its cost by 2: Change \"Discover 1 clue\" to \"Discover 2 clues.\"\n[reaction] When you play Intel Report, increase its cost by 2: Change \"at your location\" to \"at a location up to 2 connections away.\""
    ),
    CardDefinition {
        slots: &[Slot::Hand],
        ..card!(
            "05112",
            "Sign Magick",
            Asset,
            [Mystic],
            Some(3),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Ritual", "Talent"],
            "Fast.\nYou have 1 additional arcane slot, which can only be used to hold a Spell or Ritual asset."
        )
    },
    CardDefinition {
        uses: Some((UseKind::Supply, 2)),
        ..card!(
            "05117",
            "Tennessee Sour Mash",
            Asset,
            [Rogue, Survivor],
            Some(3),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Item", "Illicit"],
            "Uses (2 supplies).\n[fast] Exhaust Tennessee Sour Mash and spend 1 supply: You get +2 [willpower] for a skill test on a treachery card.\n[action] Discard Tennessee Sour Mash: Fight. You get +3 [combat] for this attack."
        )
    },
    card!(
        "05277",
        "Small Favor",
        Event,
        [Rogue],
        Some(2),
        Some(0),
        [0, 0, 2, 0, 0],
        &["Favor", "Service"],
        "Deal 1 damage to a non-Elite enemy at your location.\n[reaction] When you play Small Favor, increase its cost by 2: Change \"Deal 1 damage\" to \"Deal 2 damage.\"\n[reaction] When you play Small Favor, increase its cost by 2: Change \"at your location\" to \"at a location up to 2 connections away.\""
    ),
    CardDefinition {
        unique: true,
        health: Some(9),
        sanity: Some(5),
        base_skills: Some([2, 3, 5, 2]),
        ..card!(
            "06003",
            "Tony Morgan",
            Investigator,
            [Rogue],
            None,
            None,
            [0; 5],
            &["Criminal", "Hunter"],
            "You may take an additional action during your turn, which can only be used to engage or fight an enemy with 1 or more bounties on it.\n[elder_sign] effect: +2. Place 1 bounty on Bounty Contracts."
        )
    },
    CardDefinition {
        unique: true,
        health: Some(7),
        sanity: Some(7),
        base_skills: Some([4, 2, 2, 2]),
        ..card!(
            "06005",
            "Patrice Hathaway",
            Investigator,
            [Survivor],
            None,
            None,
            [0; 5],
            &["Performer", "Cursed"],
            "Your maximum hand size is reduced by 3.\nDuring each upkeep phase, instead of drawing 1 card, discard all non-weakness cards in your hand and draw until you have 5 cards in hand.\n[elder_sign] effect: +1. After this test ends, you may shuffle all but 1 card from your discard pile into your deck."
        )
    },
    CardDefinition {
        permanent: true,
        uses: Some((UseKind::Bounty, 6)),
        ..card!(
            "06010",
            "Bounty Contracts",
            Asset,
            [Neutral],
            None,
            None,
            [0; 5],
            &["Job"],
            "Permanent. Tony Morgan deck only. Uses (6 bounties).\n[reaction] After an enemy enters play: Move 1-3 bounties from Bounty Contracts to that enemy, to a maximum of that enemy's health.\nForced - After you defeat an enemy with 1 or more bounties on it: Move its bounties to your resource pool, as resources."
        )
    },
    CardDefinition {
        slots: &[Slot::Hand],
        uses: Some((UseKind::Ammo, 3)),
        ..card!(
            "06011",
            "Tony's .38 Long Colt",
            Asset,
            [Neutral],
            Some(3),
            None,
            [0, 1, 1, 0, 1],
            &["Item", "Weapon", "Firearm"],
            "Tony Morgan deck only. Uses (3 ammo).\n[reaction] After you play Tony's .38 Long Colt: Play another Tony's .38 Long Colt from your hand, at no cost.\n[action] Spend 1 ammo: Fight. You get +1 [combat] for each bounty on the attacked enemy. This attack deals +1 damage. If this attack defeats an enemy with 1 or more bounties on it, place 1 bounty on Bounty Contracts."
        )
    },
    CardDefinition {
        weakness: true,
        health: Some(3),
        enemy: Some(EnemyValues {
            combat: 4,
            evade: 1,
            attack_damage: 1,
            attack_horror: 2,
            health_per_investigator: false,
        }),
        ..card!(
            "06012",
            "Tony's Quarry",
            Enemy,
            [Neutral],
            None,
            None,
            [0; 5],
            &["Humanoid", "Monster", "Deep One"],
            "Spawn - Location farthest from Tony Morgan.\nAloof.\nForced - After Tony's Quarry enters play: Place 1 doom on it. Then, place 1 resource on it (from the token pool), as a bounty."
        )
    },
    CardDefinition {
        slots: &[Slot::Hand],
        unique: true,
        ..card!(
            "06016",
            "Patrice's Violin",
            Asset,
            [Neutral],
            Some(2),
            None,
            [1, 0, 0, 1, 1],
            &["Item", "Instrument"],
            "Patrice Hathaway deck only.\n[fast] Choose and discard 1 card from your hand and exhaust Patrice's Violin: Choose an investigator at your location to either gain 1 resource or draw 1 card."
        )
    },
    CardDefinition {
        weakness: true,
        unique: true,
        health: Some(2),
        enemy: Some(EnemyValues {
            combat: 5,
            evade: 5,
            attack_damage: 3,
            attack_horror: 0,
            health_per_investigator: false,
        }),
        peril: true,
        ..card!(
            "06017",
            "Watcher from Another Dimension",
            Enemy,
            [Neutral],
            None,
            None,
            [0; 5],
            &["Monster", "Extradimensional"],
            "Peril. Hidden. Hunter.\nRevelation - Secretly add this enemy to your hand.\nYou may fight or evade this enemy while it is in your hand (as if it were at your location). If you succeed, discard it from your hand. If you fail, spawn it engaged with you.\nForced - When your deck runs out of cards, if this enemy is in your hand: It attacks you (from your hand)."
        )
    },
    card!(
        "06117",
        "Read the Signs",
        Event,
        [Mystic],
        Some(2),
        Some(0),
        [1, 1, 0, 0, 0],
        &["Spell"],
        "Investigate. Add your [willpower] value to your skill value for this investigation. You may ignore any effect or keyword on your location which would trigger during this investigation. If you succeed, discover 1 additional clue at this location."
    ),
    CardDefinition {
        slots: &[Slot::Ally],
        unique: true,
        uses: Some((UseKind::Resource, 9)),
        health: Some(1),
        sanity: Some(2),
        ..card!(
            "06162",
            "Gregory Gry",
            Asset,
            [Rogue],
            Some(3),
            Some(0),
            [0, 1, 0, 0, 0],
            &["Ally", "Criminal", "Dreamer"],
            "Uses (9 resources).\n[reaction] When you initiate a skill test, spend up to 3 resources from Gregory Gry: If this test succeeds by at least that amount, gain that many resources."
        )
    },
    card!(
        "06164",
        "Ethereal Form",
        Event,
        [Mystic],
        Some(2),
        Some(0),
        [1, 0, 0, 1, 0],
        &["Spell"],
        "Evade. Add your [willpower] value to your skill value for this evasion attempt. If you succeed, disengage from each other enemy engaged with you, and for the remainder of the round, you are ethereal (enemies cannot engage or be engaged with you, and you cannot attack or deal damage to enemies)."
    ),
    card!(
        "06165",
        "Scrounge for Supplies",
        Event,
        [Survivor],
        Some(0),
        Some(0),
        [0; 5],
        &["Fortune"],
        "Choose a level 0 card in your discard pile. Add the chosen card to your hand."
    ),
    card!(
        "06201",
        "Spectral Razor",
        Event,
        [Mystic],
        Some(2),
        Some(0),
        [1, 0, 1, 0, 0],
        &["Spell"],
        "Fight. Add your [willpower] value to your skill value for this attack. Immediately before this attack, you may engage the attacked enemy. This attack deals +1 damage (+2 damage instead if the enemy is non-Elite)."
    ),
    CardDefinition {
        slots: &[Slot::Hand],
        ..card!(
            "07029",
            "Sword Cane",
            Asset,
            [Mystic],
            Some(2),
            Some(0),
            [0, 0, 1, 0, 0],
            &["Item", "Relic", "Weapon", "Melee"],
            "Playing Sword Cane does not provoke attacks of opportunity.\n[reaction] After Sword Cane enters play: Immediately trigger its \"[action]\" ability without paying its cost.\n[action] Exhaust Sword Cane: Fight/Evade. You may use your [willpower] in place of your [combat] or [agility] for this attack or evasion attempt."
        )
    },
    card!(
        "07032",
        "Promise of Power",
        Skill,
        [Mystic],
        None,
        Some(0),
        [0, 0, 0, 0, 4],
        &["Practiced", "Cursed"],
        "After you commit Promise of Power to a skill test, add 1 [curse] token to the chaos bag. If you cannot, take 2 horror instead."
    ),
    card!(
        "09029",
        "One in the Chamber",
        Event,
        [Guardian],
        Some(1),
        Some(0),
        [0, 0, 1, 1, 0],
        &["Fortune", "Tactic"],
        "Fast. Play at the end of an attack or effect that spends the last ammo from a Firearm asset you control.\nAdd 1 ammo to that asset. You get +3 skill value for your next attack with that asset this round."
    ),
    card!(
        "09068",
        "Kicking the Hornet's Nest",
        Event,
        [Rogue],
        Some(0),
        Some(0),
        [0, 1, 1, 0, 0],
        &["Gambit", "Tactic"],
        "Search the top 9 cards of the encounter deck for a non-Elite enemy and spawn it engaged with you (instead of its normal spawn location). Then, discover 1 clue at your location and gain X resources, where X is that enemy's health value. Shuffle the encounter deck."
    ),
    CardDefinition {
        slots: &[Slot::Accessory],
        unique: true,
        ..card!(
            "09102",
            "Idol of Xanatos",
            Asset,
            [Survivor],
            Some(3),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Item", "Relic"],
            "[reaction] When you are dealt damage and/or horror, exhaust Idol of Xanatos and discard up to 3 cards from your hand: Cancel that much damage/horror."
        )
    },
    CardDefinition {
        uses: Some((UseKind::Offering, 2)),
        ..card!(
            "10111",
            "Sparrow Mask",
            Asset,
            [Survivor],
            Some(1),
            Some(0),
            [1, 0, 0, 1, 0],
            &["Item", "Charm", "Mask"],
            "Limit 1 Mask per investigator.\nUses (2 offerings). Replenish 1 of these offerings after you take 1 or more damage and/or horror.\n[fast] Spend 1 offering: You get +2 [willpower] or +2 [agility] for this skill test. (Limit once per test.)"
        )
    },
    CardDefinition {
        slots: &[Slot::Arcane],
        uses: Some((UseKind::Charge, 9)),
        ..card!(
            "11067",
            "Eyes of the Dreamer",
            Asset,
            [Mystic],
            Some(3),
            Some(0),
            [0, 1, 0, 0, 0],
            &["Spell"],
            "Uses (9 charges).\n[action]: Investigate. Use your [willpower] instead of your [intellect] for this test. Before revealing chaos tokens for this test, you may spend up to 3 charges to reveal that many additional chaos tokens. Choose 1 of the revealed tokens to resolve and ignore the rest. If you succeed and the chosen token matches an ignored token, discover 1 additional clue at your location."
        )
    },
    // Mandatory FAQ 2.5 p6 erratum: the would-reveal-one-token condition.
    CardDefinition {
        slots: &[Slot::Ally],
        unique: true,
        health: Some(1),
        sanity: Some(2),
        ..card!(
            "11069",
            "Katarina Sojka",
            Asset,
            [Mystic],
            Some(2),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Ally", "Clairvoyant", "Dreamer"],
            "[reaction] When you would reveal a chaos token during a skill test you are performing, exhaust Katarina Sojka: Reveal tokens from the chaos bag until a token with a symbol is revealed. Resolve that token and ignore the rest."
        )
    },
    card!(
        "11097",
        "Confidence",
        Skill,
        [Neutral],
        None,
        Some(0),
        [0, 0, 0, 0, 1],
        &["Innate"],
        "Max 1 committed per test.\nFor each additional chaos token revealed during this skill test (even if it was canceled or ignored), Confidence gains [wild] (to a maximum of +3 [wild] icons)."
    ),
    card!(
        "60114",
        "\"Get over here!\"",
        Event,
        [Guardian],
        Some(2),
        Some(0),
        [1, 0, 1, 0, 0],
        &["Spirit", "Tactic"],
        "Engage. Fight. Choose a non-Elite enemy at your location or a connecting location. Move that enemy to your location, engage it, and attack it."
    ),
    CardDefinition {
        slots: &[Slot::Hand],
        uses: Some((UseKind::Ammo, 5)),
        ..card!(
            "60306",
            "Mauser C96",
            Asset,
            [Rogue],
            Some(4),
            Some(0),
            [0, 0, 0, 1, 0],
            &["Item", "Weapon", "Firearm", "Illicit"],
            "Uses (5 ammo).\n[action] Exhaust Mauser C96 and spend 1 ammo: Fight. You get +1 [combat] and deal +1 damage for this attack. If you succeed by 2 or more, either ready Mauser C96 or gain 1 resource."
        )
    },
    CardDefinition {
        unique: true,
        health: Some(6),
        sanity: Some(9),
        base_skills: Some([5, 3, 2, 2]),
        ..card!(
            "60401",
            "Jacqueline Fine",
            Investigator,
            [Mystic],
            None,
            None,
            [0; 5],
            &["Clairvoyant"],
            "[reaction] When an investigator at your location would reveal any number of chaos tokens: Reveal 2 additional tokens. Of the revealed tokens, choose and cancel 2 non-[auto_fail] tokens, or 1 [auto_fail] token. (Limit once per round.)\n[elder_sign] effect: +1. If this effect is canceled or ignored, draw 1 card."
        )
    },
    card!(
        "60402",
        "Arbiter of Fates",
        Asset,
        [Mystic],
        Some(3),
        None,
        [1, 0, 0, 1, 1],
        &["Talent"],
        "Jacqueline Fine deck only.\n[reaction] When you use Jacqueline Fine's [reaction] ability, exhaust Arbiter of Fates: This use of her ability does not count towards its limit."
    ),
    CardDefinition {
        weakness: true,
        ..card!(
            "60403",
            "Dark Future",
            Treachery,
            [Neutral],
            None,
            None,
            [0; 5],
            &["Omen", "Endtimes"],
            "Revelation - Put Dark Future into play in your threat area.\nYou cannot cancel or ignore [elder_sign], [skull], [cultist], [tablet], [elder_thing], and [auto_fail] tokens.\nForced - At the end of your turn: Reveal 5 random tokens from the chaos bag. If an [elder_sign] symbol is revealed, discard Dark Future."
        )
    },
    CardDefinition {
        slots: &[Slot::Arcane],
        uses: Some((UseKind::Charge, 4)),
        ..card!(
            "60407",
            "Azure Flame",
            Asset,
            [Mystic],
            Some(3),
            Some(0),
            [0, 0, 1, 0, 0],
            &["Spell"],
            "Uses (4 charges).\n[action] Spend 1 charge: Fight. This attack uses [willpower] instead of [combat] and deals +1 damage. If an [elder_sign], \"+1,\" or \"0\" token is revealed during this attack, take 1 damage."
        )
    },
    CardDefinition {
        slots: &[Slot::Arcane],
        uses: Some((UseKind::Charge, 3)),
        ..card!(
            "60408",
            "Clairvoyance",
            Asset,
            [Mystic],
            Some(4),
            Some(0),
            [0, 1, 0, 0, 0],
            &["Spell"],
            "Uses (3 charges).\n[action] Spend 1 charge: Investigate. Investigate using [willpower] instead of [intellect]. If you succeed, discover 1 additional clue at this location. If an [elder_sign], \"+1,\" or \"0\" token is revealed during this investigation, take 1 horror."
        )
    },
    CardDefinition {
        slots: &[Slot::Accessory],
        ..card!(
            "60411",
            "Crystal Pendulum",
            Asset,
            [Mystic],
            Some(2),
            Some(0),
            [1, 0, 0, 0, 0],
            &["Item", "Charm"],
            "You get +1 [willpower].\n[reaction] After a skill test at your location begins, exhaust Crystal Pendulum: Name a number. If this test succeeds by that number or fails by that number, draw 1 card."
        )
    },
    CardDefinition {
        slots: &[Slot::Body],
        health: Some(2),
        ..card!(
            "60412",
            "Robes of Endless Night",
            Asset,
            [Mystic],
            Some(3),
            Some(0),
            [0, 0, 0, 1, 0],
            &["Item", "Clothing"],
            "[reaction] When you play a Spell card, exhaust Robes of Endless Night: Reduce the cost of that card by 1."
        )
    },
    // Taboo 10 includes every symbol, including elder sign and curse.
    card!(
        "60416",
        "Voice of Ra",
        Event,
        [Mystic],
        Some(0),
        Some(0),
        [1, 0, 0, 0, 0],
        &["Spell"],
        "Gain 1 resource. Reveal 3 random tokens from the chaos bag. For each symbol revealed, gain an additional 2 resources."
    ),
    card!(
        "60419",
        "Prescient",
        Skill,
        [Mystic],
        None,
        Some(0),
        [1, 0, 0, 0, 0],
        &["Practiced", "Augury"],
        "Max 1 committed per skill test.\nAfter you commit Prescient to a skill test, name \"even,\" \"odd,\" or \"symbol.\" After this test ends, if a chaos token of the named type was revealed during this test, you may return a Spell card from your discard pile to your hand."
    ),
    CardDefinition {
        slots: &[Slot::Hand],
        uses: Some((UseKind::Key, 2)),
        ..card!(
            "60507",
            "Old Keyring",
            Asset,
            [Survivor],
            Some(1),
            Some(0),
            [0, 1, 0, 0, 0],
            &["Item", "Tool"],
            "Uses (2 keys). If there are no keys on Old Keyring, discard it.\n[action]: Investigate. Your location gets -2 shroud for this investigation. If you succeed, remove 1 key from Old Keyring."
        )
    },
    card!(
        "60513",
        "A Test of Will",
        Event,
        [Survivor],
        Some(1),
        Some(0),
        [1, 0, 0, 0, 0],
        &["Spirit"],
        "Fast. Play when an investigator at your location draws a non-weakness treachery card.\nTest [willpower] (3). If you succeed, cancel that card's revelation effect."
    ),
];
