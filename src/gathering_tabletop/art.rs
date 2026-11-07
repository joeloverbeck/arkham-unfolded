//! Verified, portable presentation assets for the ordinary Gathering table.
//!
//! Preparation owns archive selection. This module only consumes paths admitted
//! by the compiled Gathering collection and never searches the source archive.

use crate::assets;
use bevy::{asset::AssetPlugin, prelude::*};
use std::{collections::BTreeMap, path::PathBuf};

pub(super) struct Prepared {
    root: PathBuf,
    collection: assets::CollectionReport,
    status: String,
}

impl Prepared {
    /// Resolve the already-selected root without changing the saved preference.
    pub(super) fn new(root: PathBuf) -> Result<Self, String> {
        let root = crate::asset_config::resolve_root(root)?;
        let collection = assets::inspect_gathering(&root);
        if !collection.complete {
            return Err(collection.startup_failure(&root));
        }
        let status = collection.summary();
        Ok(Self {
            root,
            collection,
            status,
        })
    }

    pub(super) fn asset_plugin(&self) -> AssetPlugin {
        AssetPlugin {
            file_path: self.root.to_string_lossy().into_owned(),
            ..default()
        }
    }

    pub(super) fn load(&self, server: &AssetServer) -> Art {
        let mut art = Art {
            status: self.status.clone(),
            ..default()
        };
        let collection = &self.collection;
        for role in assets::gathering_image_roles() {
            if let Some(path) = collection.verified_path(role) {
                art.handles
                    .insert(role.to_owned(), server.load(path.to_owned()));
            }
        }
        art.symbol_font = collection
            .verified_path("font/symbols")
            .map(|path| server.load(path.to_owned()))
            .expect("the admitted collection contains its symbol font");
        art
    }
}

#[derive(Resource)]
pub(super) struct Art {
    handles: BTreeMap<String, Handle<Image>>,
    symbol_font: Handle<Font>,
    status: String,
}

impl Default for Art {
    fn default() -> Self {
        Self {
            handles: BTreeMap::new(),
            symbol_font: Handle::default(),
            status: "Artwork has not loaded.".to_owned(),
        }
    }
}

impl Art {
    #[cfg(test)]
    pub(super) fn with_symbol_font_for_test(font: Handle<Font>) -> Self {
        Self {
            symbol_font: font,
            ..default()
        }
    }

    #[cfg(test)]
    pub(super) fn with_images_for_test(
        images: impl IntoIterator<Item = (&'static str, Handle<Image>)>,
    ) -> Self {
        Self {
            handles: images
                .into_iter()
                .map(|(role, image)| (role.to_owned(), image))
                .collect(),
            ..default()
        }
    }

    pub(super) fn image(&self, role: &str) -> Option<Handle<Image>> {
        self.handles.get(role).cloned()
    }

    /// `back` means the reverse or hidden presentation side, not an archive
    /// filename. Location fronts are their revealed sides in the collection.
    pub(super) fn card(&self, code: &str, back: bool) -> Option<Handle<Image>> {
        self.image(&format!("card-{code}{}", if back { "-back" } else { "" }))
    }

    pub(super) fn symbol_font(&self) -> &Handle<Font> {
        &self.symbol_font
    }

    pub(super) fn status(&self) -> &str {
        &self.status
    }

    pub(super) fn text(&self, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
        crate::tabletop_harm_text::ui(value, size, color, &self.symbol_font)
    }
}

#[cfg(test)]
mod tests;
