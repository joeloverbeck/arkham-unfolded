//! Verified, portable presentation assets for the ordinary Gathering table.
//!
//! Preparation owns archive selection. This module only consumes paths admitted
//! by the compiled Gathering collection and never searches the source archive.

use crate::{assets, tabletop_health::HealthAssets};
use bevy::{asset::AssetPlugin, prelude::*};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const DISABLED: &str = "Readable substitutes active; artwork disabled.";

pub(super) struct Prepared {
    root: Option<PathBuf>,
    collection: Option<assets::CollectionReport>,
    status: String,
}

impl Prepared {
    /// Resolve the already-selected root without changing the saved preference.
    pub(super) fn new(root: Option<PathBuf>) -> Result<Self, String> {
        let root = root
            .map(|root| {
                if root.is_absolute() {
                    Ok(root)
                } else {
                    std::env::current_dir()
                        .map(|current| current.join(root))
                        .map_err(|error| {
                            format!("Could not resolve Gathering artwork folder: {error}")
                        })
                }
            })
            .transpose()?;
        let collection = root.as_deref().map(assets::inspect_gathering);
        let status = collection
            .as_ref()
            .map_or_else(|| DISABLED.to_owned(), assets::CollectionReport::summary);
        Ok(Self {
            root,
            collection,
            status,
        })
    }

    pub(super) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub(super) fn status(&self) -> &str {
        &self.status
    }

    pub(super) fn asset_plugin(&self) -> AssetPlugin {
        AssetPlugin {
            file_path: self.root.as_ref().map_or_else(
                || "local/gathering/absent-art".to_owned(),
                |root| root.to_string_lossy().into_owned(),
            ),
            ..default()
        }
    }

    pub(super) fn load(&self, server: &AssetServer) -> Art {
        let mut art = Art {
            status: self.status.clone(),
            ..default()
        };
        let Some(collection) = &self.collection else {
            return art;
        };
        for role in assets::gathering_image_roles() {
            if let Some(path) = collection.verified_path(role) {
                art.handles
                    .insert(role.to_owned(), server.load(path.to_owned()));
            }
        }
        art.symbol_font = collection
            .verified_path("font/symbols")
            .map(|path| server.load(path.to_owned()));
        art
    }
}

#[derive(Resource)]
pub(super) struct Art {
    handles: BTreeMap<String, Handle<Image>>,
    symbol_font: Option<Handle<Font>>,
    status: String,
}

impl Default for Art {
    fn default() -> Self {
        Self {
            handles: BTreeMap::new(),
            symbol_font: None,
            status: DISABLED.to_owned(),
        }
    }
}

impl Art {
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

    pub(super) fn symbol_font(&self) -> &Option<Handle<Font>> {
        &self.symbol_font
    }

    pub(super) fn status(&self) -> &str {
        &self.status
    }

    pub(super) fn text(&self, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
        crate::tabletop_harm_text::ui(value, size, color, &self.symbol_font)
    }

    pub(super) fn health_assets(&self) -> HealthAssets {
        HealthAssets::new(
            self.handles
                .values()
                .map(|handle| handle.id().untyped())
                .chain(self.symbol_font.iter().map(|handle| handle.id().untyped())),
        )
    }
}

#[cfg(test)]
mod tests;
