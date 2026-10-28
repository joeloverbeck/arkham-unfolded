//! The authored symbols used by this experiment, from one verified font.

use std::path::Path;

use crate::assets::CollectionReport;
use crate::tabletop_harm_text::HarmIcon;
use bevy::prelude::{Assets, Color, Font, Handle};

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
            Self::Willpower => "f",
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
