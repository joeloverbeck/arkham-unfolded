//! The authored symbols used by this experiment, from one verified font.

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
