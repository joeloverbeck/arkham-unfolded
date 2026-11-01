//! Readable selected faces from the pinned original English source set in #55.
pub(super) struct Card {
    pub role: &'static str,
    pub title: &'static str,
    pub kind: &'static str,
    pub text: &'static str,
    pub landscape: bool,
}
pub(super) const CARDS: &[Card] = &[
    Card {
        role: "card/06003",
        title: "Tony Morgan",
        kind: "Investigator · Criminal. Hunter. · Willpower 2 / Intellect 3 / Combat 5 / Agility 2",
        landscape: true,
        text: "Health 9 · Sanity 5\nYou may take an additional action during your turn, which can only be used to engage or fight an enemy with 1 or more bounties on it.\nElder sign: +2. Place 1 bounty on Bounty Contracts.\nThere are no enemies in this prepared position, so the restricted action has no eligible target.",
    },
    Card {
        role: "card/06005",
        title: "Patrice Hathaway",
        kind: "Investigator · Performer. · Willpower 4 / Intellect 2 / Combat 2 / Agility 2",
        landscape: true,
        text: "Health 7 · Sanity 7\nYour maximum hand size is reduced by 3.\nDuring each upkeep phase, instead of drawing 1 card, discard all non-weakness cards in your hand and draw until you have 5 cards in hand.\nElder sign: +1. After this test ends, you may shuffle all but 1 card from your discard pile into your deck.\nThis attempt ends before upkeep.",
    },
    Card {
        role: "card/60401",
        title: "Jacqueline Fine",
        kind: "Investigator · Psychic. · Willpower 5 / Intellect 3 / Combat 2 / Agility 2",
        landscape: true,
        text: "Health 6 · Sanity 9\nReaction — When an investigator at your location would reveal any number of chaos tokens: Reveal 2 additional tokens. Of the revealed tokens, choose and cancel 2 non-auto-fail tokens, or 1 auto-fail token. (Limit once per round.)\nElder sign: +1. If this effect is canceled or ignored, draw 1 card.\nHelping another investigator does not replace that investigator's elder-sign effect.",
    },
    Card {
        role: "card/06010",
        title: "Bounty Contracts",
        kind: "Permanent asset · Job.",
        landscape: false,
        text: "Permanent. Tony Morgan deck only. Uses (6 bounties).\nReaction — After an enemy enters play: Move 1–3 bounties from Bounty Contracts to that enemy, to a maximum of that enemy's health.\nForced — After you defeat an enemy with 1 or more bounties on it: Move its bounties to your resource pool, as resources.\nNo enemy enters play in this attempt.",
    },
    Card {
        role: "card/04109",
        title: "Arcane Research",
        kind: "Permanent · Talent.",
        landscape: false,
        text: "Permanent.\nWhen you purchase Arcane Research, suffer 1 mental trauma.\nAfter each scenario of a campaign, reduce the experience cost of the first Spell card you upgrade before the next scenario by 1.\nJacqueline has two copies and starts this prepared position with two horror.",
    },
    Card {
        role: "card/03263",
        title: "On the Hunt",
        kind: "Event · Cost 1 · Combat icon / Intellect icon",
        landscape: false,
        text: "Fast. Play when you would draw an encounter card during the mythos phase.\nInstead, search the top 9 cards of the encounter deck for an enemy, spawn it engaged with you (instead of its normal spawn location), and shuffle the encounter deck. If you cannot, draw the top card of the encounter deck.\nIts event timing does not occur here. It can contribute +1 intellect to Investigate.",
    },
    Card {
        role: "card/01089",
        title: "Guts",
        kind: "Skill · Two willpower icons",
        landscape: false,
        text: "Max 1 committed per skill test.\nIf this test is successful, draw 1 card.\nThere is no willpower test in this prepared attempt; these icons cannot be committed to Investigate.",
    },
    Card {
        role: "card/07032",
        title: "Promise of Power",
        kind: "Skill · Practiced. Cursed. · Four wild icons",
        landscape: false,
        text: "After you commit Promise of Power to a skill test, add 1 curse token to the chaos bag. If you cannot, take 2 horror instead.\nIt contributes +4 intellect here, whether Patrice tests or supports another investigator.",
    },
    Card {
        role: "card/01080",
        title: "Lucky!",
        kind: "Event · Fortune. · Cost 1 · No skill icons",
        landscape: false,
        text: "Fast. Play when you would fail a skill test.\nGet +2 to your skill value for that test.\nPatrice must have this card in hand and pay 1 resource. It cannot be committed and does not cost an action.",
    },
    Card {
        role: "card/01093",
        title: "Unexpected Courage",
        kind: "Skill · Innate. · Two wild icons",
        landscape: false,
        text: "Max 1 committed per skill test.\nIts two wild icons contribute +2 intellect. A supporting investigator may ordinarily commit at most one appropriate card.",
    },
    Card {
        role: "card/60408",
        title: "Clairvoyance",
        kind: "Asset · Spell. · Cost 4 · Intellect icon",
        landscape: false,
        text: "Uses (3 charges).\nAction — Spend 1 charge: Investigate. Investigate using willpower instead of intellect. If you succeed, discover 1 additional clue at this location. If an elder sign, +1, or 0 token is revealed during this investigation, take 1 horror.\nFrom zero resources, gaining its cost and playing this non-Fast asset requires more than this turn's three actions. It may be committed for +1 intellect.",
    },
    Card {
        role: "card/11067",
        title: "Eyes of the Dreamer",
        kind: "Asset · Spell. · Cost 3 · Intellect icon",
        landscape: false,
        text: "Uses (9 charges).\nAction — Investigate. Use your willpower instead of your intellect for this test. Before revealing chaos tokens for this test, you may spend up to 3 charges to reveal that many additional chaos tokens. Choose 1 of the revealed tokens to resolve and ignore the rest. If you succeed and the chosen token matches an ignored token, discover 1 additional clue at your location.\nGaining three resources and playing this non-Fast asset requires four actions. Its intellect icon can be committed.",
    },
    Card {
        role: "card/01111",
        title: "Study",
        kind: "Revealed location · Shroud 2",
        landscape: false,
        text: "Two clues per investigator: six clues in this prepared position.\nThere is no connecting location in play until Act 1 advances. A successful ordinary Investigate discovers one clue.",
    },
    Card {
        role: "card/01104",
        title: "The Gathering",
        kind: "Original scenario · Easy / Standard",
        landscape: false,
        text: "Skull: −X, where X is the number of Ghoul enemies at your location.\nCultist: −1. If you fail, take 1 horror.\nTablet: −2. If there is a Ghoul enemy at your location, take 1 damage.\nThere are no Ghouls here: skull is 0 and tablet deals no damage. Curse applies −2 and requires an additional reveal. Held tokens stay out of the bag until test cleanup.",
    },
    Card {
        role: "card/01108",
        title: "Trapped",
        kind: "Act 1a · Group cost: 2 clues per investigator (6 total)",
        landscape: true,
        text: "The door to your study has vanished. Find another way out.\nAt the first fully settled action leaving six clues held, choose whether to spend six and advance, or finish without advancing. This declared experiment endpoint preserves unfinished turns.",
    },
    Card {
        role: "card/01108/reverse",
        title: "The Door on the Floor",
        kind: "Act 1b",
        landscape: true,
        text: "You notice that the edges of your newly purchased rug are tattered and mud-stained. Finding this odd, you shift the furniture aside and pull back the rug. To your surprise, you see the door leading out of your study. You slowly turn the knob, and the door swings open, revealing your hallway below.\n\nYou jump through the doorway, landing on your feet on soft dirt. The door to the study slams shut above you. The smell of burning wood fills the narrow hall, intermingled with the scent of rot and decay.\n\nPut into play the set-aside Hallway, Cellar, Attic, and Parlor.\nDiscard each enemy in the Study.\nPlace each investigator in the Hallway.\nRemove the Study from the game.",
    },
    Card {
        role: "card/01109",
        title: "The Barrier",
        kind: "Act 2a",
        landscape: true,
        text: "Objective — When the round ends, investigators in the hallway may, as a group, spend the requisite number of clues to advance.\nThe prepared escape attempt stops here, before further actions or phases. Act 2's end-of-round timing does not apply to Act 1.",
    },
    Card {
        role: "card/01105",
        title: "What's Going On?!",
        kind: "Agenda 1a · Doom threshold 3",
        landscape: true,
        text: "No doom threshold or pending agenda trigger exists in this prepared position.\nThis attempt ends before the enemy phase and makes no scenario-resolution or campaign claim.",
    },
    Card {
        role: "card/01112",
        title: "Hallway",
        kind: "Revealed location",
        landscape: false,
        text: "The investigators enter and reveal Hallway during Act 1's reverse. Hallway has no additional triggered effect. The attempt stops with all three investigators here.",
    },
    Card {
        role: "card/01112/unrevealed",
        title: "Hallway",
        kind: "Unrevealed location",
        landscape: false,
        text: "A set-aside location put into play when Act 1 advances; entering reveals it.",
    },
    Card {
        role: "card/01113",
        title: "Attic",
        kind: "Unrevealed location",
        landscape: false,
        text: "Put into play unrevealed when Act 1 advances. The attempt stops before anyone enters it.",
    },
    Card {
        role: "card/01114",
        title: "Cellar",
        kind: "Unrevealed location",
        landscape: false,
        text: "Put into play unrevealed when Act 1 advances. The attempt stops before anyone enters it.",
    },
    Card {
        role: "card/01115",
        title: "Parlor",
        kind: "Unrevealed location",
        landscape: false,
        text: "The entrance to the Parlor is blocked by a darkly glowing unfathomable barrier. You cannot move into the Parlor.\nThe attempt ends before further movement.",
    },
];
pub(super) fn index(role: &str) -> Option<usize> {
    CARDS.iter().position(|card| card.role == role)
}

pub(super) fn act_revealed(view: &super::Observation) -> bool {
    view.stage == crate::party::Stage::ActReveal
        || view.endpoint == Some(crate::party::Endpoint::Escaped)
}

/// Artwork availability is not permission to disclose a face. Always use the
/// displayed observation, including its own point in a recorded replay.
pub(super) fn inspectable(view: &super::Observation, index: usize) -> bool {
    let Some(card) = CARDS.get(index) else {
        return false;
    };
    match card.role {
        "card/06003" | "card/06005" | "card/60401" | "card/06010" | "card/04109" | "card/01104"
        | "card/01105" | "card/01108" => true,
        "card/01108/reverse" => act_revealed(view),
        "card/01109" => view.endpoint == Some(crate::party::Endpoint::Escaped),
        role => {
            view.locations.iter().any(|location| {
                let role_for_face = if location.code == "01112" && !location.revealed {
                    format!("card/{}/unrevealed", location.code)
                } else {
                    format!("card/{}", location.code)
                };
                role == role_for_face
            }) || view
                .investigators
                .iter()
                .flat_map(|investigator| investigator.hand.iter().chain(&investigator.discard))
                .chain(view.test.iter().flat_map(|test| &test.committed))
                .any(|visible| role == format!("card/{}", visible.code))
        }
    }
}
