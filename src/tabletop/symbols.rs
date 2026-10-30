//! The authored symbols used by this experiment, from one verified font.

use std::path::Path;

use crate::assets::CollectionReport;
use crate::tabletop_harm_text::HarmIcon;
use bevy::prelude::{Assets, Color, Font, Handle};

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

/// Load the prepared, verified font. Unity decoding belongs to asset preparation.
pub(super) fn load(
    root: Option<&Path>,
    collection: Option<&CollectionReport>,
    fonts: &mut Assets<Font>,
) -> Option<Handle<Font>> {
    let path = collection?.verified_path("font/symbols")?;
    let bytes = std::fs::read(root?.join(path)).ok()?;
    Some(fonts.add(Font::from_bytes(bytes)))
}
