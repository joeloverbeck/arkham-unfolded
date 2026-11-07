//! The authored symbols used by this experiment, from one verified font.

use crate::tabletop_harm_text::HarmIcon;
use bevy::prelude::Color;

/// The four tested skills. Wild remains a printed icon, not a fifth skill.
#[derive(Clone, Copy)]
pub(crate) enum Skill {
    Willpower,
    Intellect,
    Combat,
    Agility,
}

impl Skill {
    pub(crate) fn named(name: &str) -> Option<Self> {
        match name {
            "willpower" => Some(Self::Willpower),
            "intellect" => Some(Self::Intellect),
            "combat" => Some(Self::Combat),
            "agility" => Some(Self::Agility),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Willpower => "willpower",
            Self::Intellect => "intellect",
            Self::Combat => "combat",
            Self::Agility => "agility",
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Willpower => "Willpower",
            Self::Intellect => "Intellect",
            Self::Combat => "Combat",
            Self::Agility => "Agility",
        }
    }

    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Willpower => "f",
            Self::Intellect => "g",
            Self::Combat => "h",
            Self::Agility => "i",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Symbol {
    Wild,
    Curse,
    Willpower,
    Horror,
}

impl Symbol {
    pub(super) fn glyph(self) -> &'static str {
        match self {
            Self::Wild => "j",
            Self::Curse => "y",
            Self::Willpower => Skill::Willpower.glyph(),
            Self::Horror => HarmIcon::Horror.glyph(),
        }
    }

    pub(super) fn color(self) -> Option<Color> {
        match self {
            Self::Horror => Some(HarmIcon::Horror.color()),
            _ => None,
        }
    }
}
