//! Preparation and verification of this application's finite external asset selection.
//! The runtime consumes prepared files; only `prepare` knows the source archive layout.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const MANIFEST: &str = "manifest.json";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
pub const CHAOS_POUCH_ROLE: &str = "model/chaos-pouch";

#[derive(Deserialize)]
struct Selection {
    format_version: u32,
    collection: String,
    assets: Vec<SelectedAsset>,
}

#[derive(Deserialize)]
struct SelectedAsset {
    role: String,
    path: String,
    sha256: String,
    bytes: u64,
    source: Source,
    transform: Transform,
    #[serde(default)]
    attribution: Option<Attribution>,
}

#[derive(Deserialize)]
struct Source {
    #[serde(default)]
    input: SourceInput,
    path: String,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SourceInput {
    #[default]
    Archive,
    RevealIcon,
    AbilityIcon,
    OfferingIcon,
    ChaosPouch,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Attribution {
    title: String,
    creator: String,
    license: String,
    license_url: String,
    source_url: String,
    download_url: String,
    modification: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Transform {
    Copy,
    PngCrop {
        source_width: u32,
        source_height: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    UnityFont {
        block_offset: usize,
        block_length: usize,
        decoded_length: usize,
        font_offset: usize,
        font_length: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    collection: String,
    assets: Vec<AssetIdentity>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AssetIdentity {
    role: String,
    path: String,
    sha256: String,
    bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attribution: Option<Attribution>,
}

impl Selection {
    fn manifest(&self) -> Manifest {
        Manifest {
            format_version: self.format_version,
            collection: self.collection.clone(),
            assets: self
                .assets
                .iter()
                .map(|asset| AssetIdentity {
                    role: asset.role.clone(),
                    path: asset.path.clone(),
                    sha256: asset.sha256.clone(),
                    bytes: asset.bytes,
                    attribution: asset.attribution.clone(),
                })
                .collect(),
        }
    }
}

fn gathering_selection() -> &'static Selection {
    static SELECTION: OnceLock<Selection> = OnceLock::new();
    SELECTION.get_or_init(|| {
        serde_json::from_str(include_str!("../resources/gathering-asset-selection.json"))
            .expect("the committed Gathering asset selection must be valid")
    })
}

/// Image roles only; fonts and models use their own decoders.
pub fn gathering_image_roles() -> impl Iterator<Item = &'static str> {
    gathering_selection()
        .assets
        .iter()
        .filter(|asset| !matches!(asset.role.as_str(), "font/symbols" | CHAOS_POUCH_ROLE))
        .map(|asset| asset.role.as_str())
}

// Exact earlier selections remain useful without claiming newly selected artwork.
fn previous_manifests(selection: &Selection) -> Vec<Manifest> {
    if selection.collection == "arkham-unfolded-gathering-v7" {
        let mut previous = selection.manifest();
        previous.collection = "arkham-unfolded-gathering-v6".into();
        previous
            .assets
            .retain(|asset| asset.role != CHAOS_POUCH_ROLE);
        let v6 = previous.clone();
        previous.collection = "arkham-unfolded-gathering-v5".into();
        previous.assets.retain(|asset| asset.role != "token/doom");
        let v5 = previous.clone();
        previous.collection = "arkham-unfolded-gathering-v4".into();
        previous
            .assets
            .retain(|asset| asset.role != "token/offering");
        let v4 = previous.clone();
        previous.collection = "arkham-unfolded-gathering-v3".into();
        previous
            .assets
            .retain(|asset| asset.role != "token/investigator-ability");
        let v3 = previous.clone();
        previous.collection = "arkham-unfolded-gathering-v2".into();
        previous.assets.retain(|asset| asset.role != "token/key");
        let v2 = previous.clone();
        previous.collection = "arkham-unfolded-gathering-v1".into();
        previous.assets.retain(|asset| {
            !matches!(
                asset.role.as_str(),
                "token/ammo" | "token/bounty" | "token/charge"
            )
        });
        return vec![v2, previous, v3, v4, v5, v6];
    }
    Vec::new()
}

/// A recognized collection can be incomplete and still supply individually verified art.
/// An unrelated or altered manifest supplies no assets. Only a complete current
/// collection can admit native play; partial inspection remains diagnostic.
#[derive(Debug)]
pub struct CollectionReport {
    pub recognized: bool,
    pub complete: bool,
    pub diagnostics: Vec<String>,
    verified: BTreeMap<String, String>,
    expected: usize,
    manifest_failure: Option<String>,
    file_failures: Vec<String>,
    structure_present: bool,
}

impl CollectionReport {
    pub fn verified_files(&self) -> impl Iterator<Item = (&str, &str)> {
        self.verified
            .iter()
            .map(|(role, path)| (role.as_str(), path.as_str()))
    }

    /// The same bounded diagnosis is used by the startup window and terminal.
    pub fn startup_failure(&self, root: &Path) -> String {
        if !root.is_dir() || !self.structure_present {
            return format!(
                "Absent or wrong prepared collection structure. Select the collection root containing manifest.json and its prepared image/font folders. {}",
                self.manifest_failure
                    .as_deref()
                    .unwrap_or("The expected payload folders are absent.")
            );
        }
        let mut message = self.manifest_failure.clone().unwrap_or_default();
        if !self.file_failures.is_empty() {
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str(&bounded_failures(&self.file_failures));
        }
        if message.is_empty() {
            message = "The required asset selection is empty or incomplete.".into();
        }
        message
    }
    /// A relative path, verified against the application's compiled selection.
    pub fn verified_path(&self, role: &str) -> Option<&str> {
        self.verified.get(role).map(String::as_str)
    }

    pub fn valid_count(&self) -> usize {
        self.verified.len()
    }

    pub fn expected_count(&self) -> usize {
        self.expected
    }

    pub fn summary(&self) -> String {
        if self.complete {
            format!(
                "Complete asset collection: {}/{} verified.",
                self.valid_count(),
                self.expected
            )
        } else if self.recognized {
            format!(
                "Incomplete asset collection: {}/{} verified. Complete artwork is required for 3D play.",
                self.valid_count(),
                self.expected
            )
        } else {
            "No compatible prepared asset collection. Complete artwork is required for 3D play."
                .into()
        }
    }
}

pub(crate) fn bounded_failures(failures: &[String]) -> String {
    let mut result = format!("{} required files affected:", failures.len());
    for failure in failures.iter().take(5) {
        result.push_str("\n  ");
        result.push_str(failure);
    }
    result.push_str(&format!(
        "\n{} additional affected files omitted.",
        failures.len().saturating_sub(5)
    ));
    result
}

/// Check only the finite expected files. This never searches or falls back to an archive.
/// Inspect Gathering's own collection without adopting an earlier experiment's subset.
pub fn inspect_gathering(root: &Path) -> CollectionReport {
    inspect_selection(root, gathering_selection())
}

fn inspect_selection(root: &Path, selection: &Selection) -> CollectionReport {
    let mut report = CollectionReport {
        recognized: false,
        complete: false,
        diagnostics: Vec::new(),
        verified: BTreeMap::new(),
        expected: selection.assets.len(),
        manifest_failure: None,
        file_failures: Vec::new(),
        structure_present: selection.assets.iter().any(|asset| {
            Path::new(&asset.path)
                .parent()
                .is_some_and(|parent| root.join(parent).is_dir())
        }),
    };
    let read_manifest = || -> Result<Manifest, AssetError> {
        let path = regular_file(root, MANIFEST)?;
        let bytes = read_bounded(&path, MAX_MANIFEST_BYTES)?;
        Ok(serde_json::from_slice(&bytes)?)
    };
    let mut current_manifest = false;
    match read_manifest() {
        Ok(manifest) if manifest == selection.manifest() => {
            report.recognized = true;
            current_manifest = true;
        }
        Ok(manifest) if previous_manifests(selection).contains(&manifest) => {
            report.recognized = true;
            report.diagnostics.push(
                "Obsolete manifest: prepare a fresh complete collection for the current selection. Earlier collections cannot admit 3D play.".into(),
            );
        }
        Ok(_) => report.diagnostics.push(
            "manifest.json does not describe the assets expected by this application.".into(),
        ),
        Err(error) => report.diagnostics.push(format!("manifest.json: {error}")),
    }
    report.manifest_failure = report.diagnostics.first().cloned();
    if !report.recognized {
        return report;
    }
    for asset in &selection.assets {
        match read_verified(root, &asset.path, asset.bytes, &asset.sha256) {
            Ok(_) => {
                report
                    .verified
                    .insert(asset.role.clone(), asset.path.clone());
            }
            Err(error) => {
                let failure = format!("{}: {error}", asset.path);
                report.file_failures.push(failure.clone());
                report.diagnostics.push(failure);
            }
        }
    }
    report.complete = current_manifest && report.verified.len() == selection.assets.len();
    report
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepareResult {
    Created,
    AlreadyPresent,
}

pub use unfolded_tabletop::assets::AssetError;

/// Prepare a fresh directory, or reuse an identical complete directory without writing it.
/// Changed, incomplete or unknown destinations are refused. Source files are never written.
/// Prepare the selected original Gathering artwork in a fresh portable collection.
pub fn prepare_gathering(
    source: &Path,
    output: &Path,
    reveal_icon: &Path,
    ability_icon: &Path,
    offering_icon: &Path,
    chaos_pouch: &Path,
) -> Result<PrepareResult, AssetError> {
    prepare_selection_with_inputs(
        source,
        output,
        gathering_selection(),
        Some(reveal_icon),
        Some(ability_icon),
        Some(offering_icon),
        Some(chaos_pouch),
    )
}

#[cfg(test)]
fn prepare_selection(
    source: &Path,
    output: &Path,
    selection: &Selection,
) -> Result<PrepareResult, AssetError> {
    prepare_selection_with_icon(source, output, selection, None)
}

#[cfg(test)]
fn prepare_selection_with_icon(
    source: &Path,
    output: &Path,
    selection: &Selection,
    reveal_icon: Option<&Path>,
) -> Result<PrepareResult, AssetError> {
    prepare_selection_with_inputs(source, output, selection, reveal_icon, None, None, None)
}

fn prepare_selection_with_inputs(
    source: &Path,
    output: &Path,
    selection: &Selection,
    reveal_icon: Option<&Path>,
    ability_icon: Option<&Path>,
    offering_icon: Option<&Path>,
    chaos_pouch: Option<&Path>,
) -> Result<PrepareResult, AssetError> {
    if fs::symlink_metadata(output).is_ok() {
        let report = inspect_selection(output, selection);
        if report.complete && exact_contents(output, selection)? {
            return Ok(PrepareResult::AlreadyPresent);
        }
        return Err(AssetError(format!(
            "Destination {} already exists and is not the identical complete collection. Choose a fresh directory; nothing was overwritten.",
            output.display()
        )));
    }

    let source = source.canonicalize()?;
    let output = resolve_output(output)?;
    if output.starts_with(&source) {
        return Err(AssetError(
            "The prepared collection must be outside the source archive.".into(),
        ));
    }

    // Read and verify every source before creating an output directory. The current
    // finite selection fits in memory; retaining it also avoids source reread races.
    let mut prepared = Vec::with_capacity(selection.assets.len());
    for asset in &selection.assets {
        let source_bytes = match asset.source.input {
            SourceInput::Archive => read_verified(
                &source,
                &asset.source.path,
                asset.source.bytes,
                &asset.source.sha256,
            ),
            SourceInput::RevealIcon
            | SourceInput::AbilityIcon
            | SourceInput::OfferingIcon
            | SourceInput::ChaosPouch => {
                let (path, flag) = match asset.source.input {
                    SourceInput::RevealIcon => (reveal_icon, "--reveal-icon"),
                    SourceInput::AbilityIcon => (ability_icon, "--ability-icon"),
                    SourceInput::OfferingIcon => (offering_icon, "--offering-icon"),
                    SourceInput::ChaosPouch => (chaos_pouch, "--chaos-pouch"),
                    SourceInput::Archive => unreachable!(),
                };
                let path = path.ok_or_else(|| {
                    AssetError(format!(
                        "Preparation requires {flag} PATH for the selected asset."
                    ))
                })?;
                if !fs::symlink_metadata(path)?.file_type().is_file() {
                    return Err(AssetError(format!(
                        "The {flag} asset must be a regular file, not a symbolic link."
                    )));
                }
                let bytes = read_bounded(path, asset.source.bytes)?;
                verify_bytes(&bytes, asset.source.bytes, &asset.source.sha256)?;
                Ok(bytes)
            }
        }
        .map_err(|error| AssetError(format!("{}: {error}", asset.source.path)))?;
        let bytes = transform(source_bytes, &asset.transform)?;
        verify_bytes(&bytes, asset.bytes, &asset.sha256)?;
        prepared.push((asset.path.as_str(), bytes));
    }

    let parent = output
        .parent()
        .ok_or_else(|| AssetError("Output needs a parent directory.".into()))?;
    fs::create_dir_all(parent)?;
    let stage = tempfile::Builder::new()
        .prefix(".arkham-assets-")
        .tempdir_in(parent)?;
    for (path, bytes) in prepared {
        let destination = stage.path().join(relative_path(path)?);
        fs::create_dir_all(destination.parent().expect("asset has a parent"))?;
        fs::write(destination, bytes)?;
    }
    fs::write(
        stage.path().join(MANIFEST),
        serde_json::to_vec_pretty(&selection.manifest())?,
    )?;
    let report = inspect_selection(stage.path(), selection);
    if !report.complete {
        return Err(AssetError(format!(
            "Prepared output did not verify: {}",
            report.diagnostics.join("; ")
        )));
    }

    unfolded_tabletop::assets::publish_directory(stage.path(), &output)?;
    Ok(PrepareResult::Created)
}

fn transform(bytes: Vec<u8>, transform: &Transform) -> Result<Vec<u8>, AssetError> {
    match *transform {
        Transform::Copy => Ok(bytes),
        Transform::PngCrop {
            source_width,
            source_height,
            x,
            y,
            width,
            height,
        } => unfolded_tabletop::assets::crop_png(
            &bytes,
            unfolded_tabletop::assets::PngCrop {
                source_width,
                source_height,
                x,
                y,
                width,
                height,
            },
        ),
        Transform::UnityFont {
            block_offset,
            block_length,
            decoded_length,
            font_offset,
            font_length,
        } => {
            // This is the existing experiment's single hash-bound extraction,
            // not a Unity parser. Unity omits the LZMA-alone eight-byte length.
            let block = slice(&bytes, block_offset, block_length)?;
            let mut lzma = Vec::with_capacity(block.len() + 8);
            lzma.extend_from_slice(slice(block, 0, 5)?);
            lzma.extend_from_slice(&(decoded_length as u64).to_le_bytes());
            lzma.extend_from_slice(&block[5..]);
            let mut decoded = Vec::with_capacity(decoded_length);
            lzma_rs::lzma_decompress(&mut Cursor::new(lzma), &mut decoded).map_err(|error| {
                AssetError(format!(
                    "The verified font bundle could not be decoded: {error}"
                ))
            })?;
            if decoded.len() != decoded_length {
                return Err(AssetError(
                    "The decoded font bundle has the wrong length.".into(),
                ));
            }
            Ok(slice(&decoded, font_offset, font_length)?.to_vec())
        }
    }
}

fn slice(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], AssetError> {
    offset
        .checked_add(length)
        .and_then(|end| bytes.get(offset..end))
        .ok_or_else(|| {
            AssetError("The selected font extraction lies outside the verified bundle.".into())
        })
}

use unfolded_tabletop::assets::{
    read_bounded, read_verified, regular_file, relative_path, verify_bytes,
};

// Resolve existing ancestors before creating anything, including an output with
// new parent directories. This prevents accidentally preparing inside the archive.
use unfolded_tabletop::assets::resolve_output;

// Reusing an existing directory is stricter than rendering it: preparation must
// refuse unknown files/directories rather than silently bless or remove them.
fn exact_contents(root: &Path, selection: &Selection) -> Result<bool, AssetError> {
    let mut expected = BTreeSet::from([PathBuf::from(MANIFEST)]);
    for asset in &selection.assets {
        let path = PathBuf::from(&asset.path);
        expected.insert(path.clone());
        for ancestor in path
            .ancestors()
            .skip(1)
            .filter(|path| !path.as_os_str().is_empty())
        {
            expected.insert(ancestor.to_path_buf());
        }
    }
    let mut pending = vec![PathBuf::new()];
    let mut observed = BTreeSet::new();
    while let Some(relative) = pending.pop() {
        for entry in fs::read_dir(root.join(&relative))? {
            let entry = entry?;
            let path = relative.join(entry.file_name());
            let kind = entry.file_type()?;
            if kind.is_symlink() || !expected.contains(&path) {
                return Ok(false);
            }
            observed.insert(path.clone());
            if kind.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok(observed == expected)
}

#[cfg(all(test, feature = "tabletop"))]
pub(crate) fn write_gathering_manifest_without_payloads(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join(MANIFEST),
        serde_json::to_vec(&gathering_selection().manifest()).unwrap(),
    )
    .unwrap();
}

#[cfg(all(test, feature = "tabletop"))]
pub(crate) fn assert_counter_recipe_roles_loaded(
    recipe: &str,
    loaded: impl Iterator<Item = String>,
) {
    let is_counter = |role: &String| {
        matches!(role.as_str(), "token/clue" | "token/resource") || role.starts_with("class/")
    };
    let selected: Selection = serde_json::from_str(recipe).unwrap();
    let expected: BTreeSet<_> = selected
        .assets
        .into_iter()
        .map(|asset| asset.role)
        .filter(is_counter)
        .collect();
    // Keep the recipe oracle independent of the renderer's role/class enumeration.
    let required = BTreeSet::from([
        "token/clue",
        "token/resource",
        "class/guardian/icon",
        "class/guardian/background",
        "class/seeker/icon",
        "class/seeker/background",
        "class/rogue/icon",
        "class/rogue/background",
        "class/mystic/icon",
        "class/mystic/background",
        "class/survivor/icon",
        "class/survivor/background",
        "class/neutral/icon",
        "class/neutral/background",
    ]);
    assert_eq!(
        expected.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        required,
        "the shipped recipe must supply both counters and all six class pairs"
    );
    let loaded: BTreeSet<_> = loaded.filter(is_counter).collect();
    assert_eq!(
        loaded, expected,
        "native image loading must request every counter asset in the shipped recipe"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "tabletop")]
    #[test]
    fn gathering_loader_retains_every_counter_and_class_asset() {
        assert_counter_recipe_roles_loaded(
            include_str!("../resources/gathering-asset-selection.json"),
            gathering_image_roles().map(str::to_owned),
        );
    }

    #[test]
    fn gathering_recipe_covers_selected_decks_and_original_scenario_faces() {
        let recipe: Selection =
            serde_json::from_str(include_str!("../resources/gathering-asset-selection.json"))
                .unwrap();
        let decks: serde_json::Value =
            serde_json::from_str(include_str!("../docs/research/issue-1-decks.json")).unwrap();
        let mut codes = BTreeSet::new();
        for deck in decks["decks"].as_array().unwrap() {
            codes.insert(deck["investigator_code"].as_str().unwrap().to_owned());
            codes.extend(deck["slots"].as_object().unwrap().keys().cloned());
        }
        // The retained deck placeholders are replaced by the owner's fixed choices.
        codes.remove("01000");
        codes.extend(["02038", "02039", "05041"].map(str::to_owned));
        // Independent original campaign inventory: 26 identities, 40 physical cards.
        codes.extend(
            [
                "01104", "01105", "01106", "01107", "01108", "01109", "01110", "01111", "01112",
                "01113", "01114", "01115", "01116", "01117", "01118", "01119", "01159", "01160",
                "01161", "01162", "01163", "01164", "01165", "01166", "01167", "01168",
            ]
            .map(str::to_owned),
        );
        assert_eq!(codes.len(), 94);
        let expected: BTreeSet<_> = codes.iter().map(|code| format!("card-{code}")).collect();
        let actual: BTreeSet<_> = recipe
            .assets
            .iter()
            .map(|asset| asset.role.clone())
            .filter(|role| {
                role.starts_with("card-")
                    && !role.ends_with("-back")
                    && !role.starts_with("card-back/")
            })
            .collect();
        assert_eq!(
            actual, expected,
            "Every selected physical card identity needs its original face"
        );
        let roles: BTreeSet<_> = recipe
            .assets
            .iter()
            .map(|asset| asset.role.as_str())
            .collect();
        for code in [
            "01105", "01106", "01107", "01108", "01109", "01110", "01111", "01112", "01113",
            "01114", "01115",
        ] {
            assert!(
                roles.contains(format!("card-{code}-back").as_str()),
                "Missing reverse/unrevealed side of {code}"
            );
        }
        for role in [
            "card-back/player",
            "card-back/encounter",
            "font/symbols",
            "standee/06003",
            "standee/06005",
            "standee/60401",
            "token/curse",
            "token/doom",
            "token/clue",
            "token/resource",
            "token/damage",
            "token/horror",
            "ui/reveal-chaos-token",
            "token/ammo",
            "token/bounty",
            "token/charge",
            "token/key",
        ] {
            assert!(
                roles.contains(role),
                "Missing shared presentation role {role}"
            );
        }
        assert_eq!(roles.len(), recipe.assets.len(), "Roles must be unique");
        let paths: BTreeSet<_> = recipe
            .assets
            .iter()
            .map(|asset| asset.path.as_str())
            .collect();
        assert_eq!(
            paths.len(),
            recipe.assets.len(),
            "Destinations must be unique"
        );
    }

    #[test]
    fn gathering_location_sides_and_standard_reference_keep_original_art_identity() {
        let selected = gathering_selection();
        for code in ["01111", "01112", "01113", "01114", "01115"] {
            let front = selected
                .assets
                .iter()
                .find(|asset| asset.role == format!("card-{code}"))
                .unwrap();
            let hidden = selected
                .assets
                .iter()
                .find(|asset| asset.role == format!("card-{code}-back"))
                .unwrap();
            assert!(
                front.source.path.ends_with("/back.png"),
                "Original revealed {code} is the archive back"
            );
            assert!(
                hidden.source.path.ends_with("/face.png"),
                "Original unrevealed {code} is the archive face"
            );
        }
        for (role, expected_hash) in [
            (
                "card-01104",
                "2c65c1d4b98f26f4268cfa4647b006ffa9a6d7030281a545e676ad41c1d9ceb5",
            ),
            (
                "card-01111",
                "f9a4874007445ac47f66275266e034ea65665722580e1672d70cc6ae9121cf10",
            ),
            (
                "card-01111-back",
                "ea1390d9486abe7ea275b7501d87ec3a4eb73c5b27d01e3824d04b40d7b25334",
            ),
        ] {
            let asset = selected
                .assets
                .iter()
                .find(|asset| asset.role == role)
                .unwrap();
            assert_eq!(
                asset.sha256, expected_hash,
                "Pinned original image for {role}"
            );
            assert_eq!(
                asset.source.sha256, expected_hash,
                "Original bytes are copied unchanged"
            );
        }
    }

    #[test]
    fn gathering_inspection_rejects_other_collections_and_preserves_missing_art_records() {
        let directory = tempfile::tempdir().unwrap();
        let manifest = directory.path().join(MANIFEST);
        let bytes = serde_json::to_vec(&gathering_selection().manifest()).unwrap();
        assert!(bytes.len() < MAX_MANIFEST_BYTES as usize);
        fs::write(&manifest, &bytes).unwrap();
        let report = inspect_gathering(directory.path());
        assert!(report.recognized && !report.complete);
        assert_eq!(report.valid_count(), 0);
        assert!(report.verified_path("card-02039").is_none());
        assert_eq!(fs::read(&manifest).unwrap(), bytes);
        let mut other = gathering_selection().manifest();
        other.collection = "retired-study-escape".into();
        let prior = serde_json::to_vec(&other).unwrap();
        fs::write(&manifest, &prior).unwrap();
        assert!(!inspect_gathering(directory.path()).recognized);
        assert_eq!(fs::read(&manifest).unwrap(), prior);
    }

    #[test]
    fn gathering_collection_relocates_without_source_and_isolates_corrupt_faces() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("prepared");
        let moved = directory.path().join("relocated");
        let selected = recipe_fixture(
            &source,
            include_str!("../resources/gathering-asset-selection.json"),
        );
        prepare_selection(&source, &output, &selected).unwrap();
        fs::rename(&output, &moved).unwrap();
        fs::remove_dir_all(&source).unwrap();
        assert!(inspect_selection(&moved, &selected).complete);
        assert_eq!(
            prepare_selection(&source, &moved, &selected).unwrap(),
            PrepareResult::AlreadyPresent
        );
        let card = selected
            .assets
            .iter()
            .find(|asset| asset.role == "card-02039")
            .unwrap();
        let manifest = fs::read(moved.join(MANIFEST)).unwrap();
        fs::write(moved.join(&card.path), b"changed image").unwrap();
        let report = inspect_selection(&moved, &selected);
        assert!(report.recognized && !report.complete);
        assert!(report.verified_path("card-02039").is_none());
        assert!(report.verified_path("card-05041").is_some());
        assert!(prepare_selection(&source, &moved, &selected).is_err());
        assert_eq!(fs::read(moved.join(MANIFEST)).unwrap(), manifest);
        assert_eq!(fs::read(moved.join(&card.path)).unwrap(), b"changed image");
    }

    fn png_fixture() -> Vec<u8> {
        use image::ImageEncoder;
        // An independently specified 4x3 atlas, including transparent colors.
        let pixels = [
            [1, 0, 0, 255],
            [2, 0, 0, 255],
            [3, 0, 0, 255],
            [4, 0, 0, 255],
            [5, 0, 0, 255],
            [6, 20, 30, 0],
            [7, 40, 50, 128],
            [8, 0, 0, 255],
            [9, 0, 0, 255],
            [10, 60, 70, 255],
            [11, 80, 90, 255],
            [12, 0, 0, 255],
        ];
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(pixels.as_flattened(), 4, 3, image::ExtendedColorType::Rgba8)
            .unwrap();
        bytes
    }

    #[test]
    fn atlas_crop_preserves_selected_pixels_and_alpha_without_resizing() {
        let bytes = transform(
            png_fixture(),
            &Transform::PngCrop {
                source_width: 4,
                source_height: 3,
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
        )
        .unwrap();
        let face = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        assert_eq!(face.dimensions(), (2, 2));
        assert_eq!(
            face.as_raw(),
            &[
                6, 20, 30, 0, 7, 40, 50, 128, 10, 60, 70, 255, 11, 80, 90, 255
            ]
        );
    }

    #[test]
    fn atlas_crop_refuses_invalid_bounds_dimensions_and_invalid_images() {
        for (source_width, source_height, x, y, width, height) in [
            (4, 3, 0, 0, 0, 1),
            (4, 3, 0, 0, 1, 0),
            (4, 3, 3, 0, 2, 1),
            (4, 3, 0, 2, 1, 2),
            (4, 3, u32::MAX, 0, 2, 1),
            (4, 3, 0, u32::MAX, 1, 2),
            (3, 3, 0, 0, 2, 2),
            (5, 3, 0, 0, 2, 2),
            (2048, 3, 0, 0, 2, 2),
        ] {
            assert!(
                transform(
                    png_fixture(),
                    &Transform::PngCrop {
                        source_width,
                        source_height,
                        x,
                        y,
                        width,
                        height,
                    }
                )
                .is_err()
            );
        }
        assert!(
            transform(
                b"not a png".to_vec(),
                &Transform::PngCrop {
                    source_width: 4,
                    source_height: 3,
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 2,
                }
            )
            .is_err()
        );
    }

    #[test]
    fn cropped_asset_identity_is_checked_before_any_collection_is_published() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("output");
        let mut selected = fixture(&source);
        let asset = &mut selected.assets[0];
        let atlas = png_fixture();
        asset.source.sha256 = format!("{:x}", Sha256::digest(&atlas));
        asset.source.bytes = atlas.len() as u64;
        fs::write(source.join(&asset.source.path), &atlas).unwrap();
        asset.transform = Transform::PngCrop {
            source_width: 4,
            source_height: 3,
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };
        let cropped = transform(atlas, &asset.transform).unwrap();
        asset.bytes = cropped.len() as u64;
        // Correct input and byte length cannot excuse a different output identity.
        assert!(prepare_selection(&source, &output, &selected).is_err());
        assert!(!output.exists());
        selected.assets[0].sha256 = format!("{:x}", Sha256::digest(&cropped));
        prepare_selection(&source, &output, &selected).unwrap();
        assert!(inspect_selection(&output, &selected).complete);
        assert_eq!(
            fs::read(output.join(&selected.assets[0].path)).unwrap(),
            cropped
        );
    }

    fn gathering_fixture(source: &Path) -> Selection {
        recipe_fixture(
            source,
            include_str!("../resources/gathering-asset-selection.json"),
        )
    }

    fn recipe_fixture(source: &Path, recipe: &str) -> Selection {
        let mut selected: Selection = serde_json::from_str(recipe).unwrap();
        fs::create_dir_all(source).unwrap();
        for (index, asset) in selected.assets.iter_mut().enumerate() {
            let bytes = format!("fixture payload for {}", asset.role).into_bytes();
            asset.sha256 = format!("{:x}", Sha256::digest(&bytes));
            asset.bytes = bytes.len() as u64;
            asset.source.path = index.to_string();
            asset.source.input = SourceInput::Archive;
            asset.source.sha256 = asset.sha256.clone();
            asset.source.bytes = asset.bytes;
            asset.transform = Transform::Copy;
            fs::write(source.join(&asset.source.path), bytes).unwrap();
        }
        selected
    }

    #[test]
    fn counter_art_corruption_preserves_other_verified_gathering_art() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selected = gathering_fixture(&source);
        prepare_selection(&source, &output, &selected).unwrap();
        let manifest = fs::read(output.join(MANIFEST)).unwrap();
        fs::remove_file(output.join("tokens/clue.png")).unwrap();
        fs::write(output.join("tokens/resource.png"), b"changed").unwrap();
        fs::remove_file(output.join("classes/rogue/icon.png")).unwrap();
        let report = inspect_selection(&output, &selected);
        assert!(report.recognized && !report.complete);
        assert_eq!(report.valid_count(), selected.assets.len() - 3);
        for missing in ["token/clue", "token/resource", "class/rogue/icon"] {
            assert!(report.verified_path(missing).is_none(), "{missing}");
        }
        assert!(report.verified_path("class/rogue/background").is_some());
        assert!(report.verified_path("class/mystic/icon").is_some());
        assert!(report.verified_path("card-06003").is_some());
        assert_eq!(fs::read(output.join(MANIFEST)).unwrap(), manifest);
    }

    #[test]
    fn gathering_refuses_missing_or_changed_doom_and_pouch_without_replacing_verified_art() {
        for (role, relative) in [
            ("token/doom", "tokens/doom.png"),
            (CHAOS_POUCH_ROLE, "models/chaos-pouch.glb"),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("source");
            let output = directory.path().join("assets");
            let selected = gathering_fixture(&source);
            prepare_selection(&source, &output, &selected).unwrap();
            let manifest = fs::read(output.join(MANIFEST)).unwrap();
            let path = output.join(relative);
            let original = fs::read(&path).unwrap();
            for changed in [false, true] {
                if changed {
                    let mut corrupt = original.clone();
                    corrupt[0] ^= 1;
                    fs::write(&path, corrupt).unwrap();
                } else {
                    fs::remove_file(&path).unwrap();
                }
                let report = inspect_selection(&output, &selected);
                assert!(report.recognized && !report.complete);
                assert_eq!(report.valid_count(), selected.assets.len() - 1);
                assert!(report.verified_path(role).is_none());
                assert!(report.verified_path("token/clue").is_some());
                assert!(report.verified_path("card-06003").is_some());
                assert_eq!(fs::read(output.join(MANIFEST)).unwrap(), manifest);
            }
            fs::write(&path, original).unwrap();
            assert!(inspect_selection(&output, &selected).complete);
        }
    }

    #[test]
    fn previous_v6_requires_pouch_without_discarding_existing_art() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let current = gathering_fixture(&source);
        let mut previous = gathering_fixture(&source);
        previous.collection = "arkham-unfolded-gathering-v6".into();
        previous
            .assets
            .retain(|asset| asset.role != CHAOS_POUCH_ROLE);
        prepare_selection(&source, &output, &previous).unwrap();
        let report = inspect_selection(&output, &current);
        assert!(report.recognized && !report.complete);
        assert_eq!((report.valid_count(), report.expected_count()), (147, 148));
        assert_eq!(report.verified_path(CHAOS_POUCH_ROLE), None);
        assert!(prepare_selection(&source, &output, &current).is_err());
    }

    #[test]
    fn explicit_pouch_is_verified_copied_and_relocates_without_authoring_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("prepared");
        let supplied = directory.path().join("authoring.glb");
        let mut selected = fixture(&source);
        selected.assets[1].role = CHAOS_POUCH_ROLE.into();
        selected.assets[1].path = "models/chaos-pouch.glb".into();
        selected.assets[1].source.input = SourceInput::ChaosPouch;
        let prepare = |file| {
            prepare_selection_with_inputs(&source, &output, &selected, None, None, None, file)
        };
        assert!(prepare(None).is_err());
        fs::write(&supplied, b"OTHER BYTES").unwrap();
        assert!(prepare(Some(supplied.as_path())).is_err());
        assert!(!output.exists());
        fs::write(&supplied, b"other bytes").unwrap();
        assert_eq!(
            prepare(Some(supplied.as_path())).unwrap(),
            PrepareResult::Created
        );
        assert_eq!(
            fs::read(output.join("models/chaos-pouch.glb")).unwrap(),
            fs::read(&supplied).unwrap()
        );
        let moved = directory.path().join("relocated");
        fs::rename(output, &moved).unwrap();
        fs::remove_dir_all(source).unwrap();
        fs::remove_file(supplied).unwrap();
        assert!(inspect_selection(&moved, &selected).complete);
    }

    #[test]
    fn gathering_previous_collections_keep_verified_art_without_claiming_the_key() {
        for (version, expected) in [
            ("v1", 140),
            ("v2", 143),
            ("v3", 144),
            ("v4", 145),
            ("v5", 146),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("source");
            let output = directory.path().join("assets");
            let recipe = include_str!("../resources/gathering-asset-selection.json");
            let selected = gathering_fixture(&source);
            let mut earlier = recipe_fixture(&source, recipe);
            earlier.collection = format!("arkham-unfolded-gathering-{version}");
            earlier.assets.retain(|asset| {
                asset.role != CHAOS_POUCH_ROLE
                    && asset.role != "token/doom"
                    && (version == "v5" || asset.role != "token/offering")
                    && (matches!(version, "v4" | "v5")
                        || asset.role != "token/investigator-ability")
                    && (matches!(version, "v3" | "v4" | "v5") || asset.role != "token/key")
                    && (version != "v1"
                        || !matches!(
                            asset.role.as_str(),
                            "token/ammo" | "token/bounty" | "token/charge"
                        ))
            });
            prepare_selection(&source, &output, &earlier).unwrap();
            let manifest = fs::read(output.join(MANIFEST)).unwrap();
            let report = inspect_selection(&output, &selected);
            assert!(report.recognized && !report.complete);
            assert_eq!(
                (report.valid_count(), report.expected_count()),
                (expected, 148)
            );
            assert!(report.verified_path("card-60507").is_some());
            assert_eq!(
                report.verified_path("token/key").is_some(),
                matches!(version, "v3" | "v4" | "v5")
            );
            assert_eq!(
                report.verified_path("token/investigator-ability").is_some(),
                matches!(version, "v4" | "v5")
            );
            assert_eq!(
                report.verified_path("token/offering").is_some(),
                version == "v5"
            );
            assert!(report.verified_path("token/doom").is_none());
            assert!(prepare_selection(&source, &output, &selected).is_err());
            assert_eq!(fs::read(output.join(MANIFEST)).unwrap(), manifest);
        }
    }

    fn fixture(source: &Path) -> Selection {
        fs::create_dir_all(source).unwrap();
        let mut assets = Vec::new();
        for (name, bytes) in [
            ("first", b"first real bytes".as_slice()),
            ("second", b"other bytes".as_slice()),
        ] {
            fs::write(source.join(name), bytes).unwrap();
            let hash = format!("{:x}", Sha256::digest(bytes));
            assets.push(SelectedAsset {
                role: name.into(),
                path: format!("pictures/{name}.png"),
                sha256: hash.clone(),
                bytes: bytes.len() as u64,
                source: Source {
                    input: SourceInput::Archive,
                    path: name.into(),
                    sha256: hash,
                    bytes: bytes.len() as u64,
                },
                transform: Transform::Copy,
                attribution: None,
            });
        }
        Selection {
            format_version: 1,
            collection: "test-only-selection".into(),
            assets,
        }
    }

    #[test]
    fn preparation_is_portable_and_identical_repeats_need_no_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("new-parent/assets");
        let selection = fixture(&source);
        assert_eq!(
            prepare_selection(&source, &output, &selection).unwrap(),
            PrepareResult::Created
        );
        assert_eq!(fs::read(source.join("first")).unwrap(), b"first real bytes");
        assert_eq!(
            fs::read(output.join("pictures/first.png")).unwrap(),
            b"first real bytes"
        );
        let moved = directory.path().join("relocated assets");
        fs::rename(&output, &moved).unwrap();
        fs::remove_dir_all(&source).unwrap();
        let before = fs::read(moved.join(MANIFEST)).unwrap();
        assert_eq!(
            prepare_selection(&source, &moved, &selection).unwrap(),
            PrepareResult::AlreadyPresent
        );
        assert_eq!(fs::read(moved.join(MANIFEST)).unwrap(), before);
        let report = inspect_selection(&moved, &selection);
        assert!(report.complete);
        assert_eq!(report.verified_path("first"), Some("pictures/first.png"));
    }

    #[test]
    fn explicit_reveal_icon_is_verified_before_publication_and_survives_relocation() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("new-parent/assets");
        let icon = directory.path().join("supplied-icon.png");
        let mut selected = fixture(&source);
        selected.assets[1].role = "ui/reveal-chaos-token".into();
        selected.assets[1].path = "ui/reveal-chaos-token.png".into();
        selected.assets[1].source.input = SourceInput::RevealIcon;
        fs::remove_file(source.join("second")).unwrap();

        assert!(prepare_selection_with_icon(&source, &output, &selected, None).is_err());
        assert!(prepare_selection_with_icon(&source, &output, &selected, Some(&icon)).is_err());
        fs::write(&icon, b"OTHER BYTES").unwrap();
        assert!(prepare_selection_with_icon(&source, &output, &selected, Some(&icon)).is_err());
        assert!(!output.parent().unwrap().exists());
        fs::write(&icon, b"other bytes").unwrap();
        assert_eq!(
            prepare_selection_with_icon(&source, &output, &selected, Some(&icon)).unwrap(),
            PrepareResult::Created
        );
        assert_eq!(
            fs::read(output.join("ui/reveal-chaos-token.png")).unwrap(),
            b"other bytes"
        );
        assert_eq!(fs::read(&icon).unwrap(), b"other bytes");

        let moved = directory.path().join("relocated");
        fs::rename(&output, &moved).unwrap();
        fs::remove_dir_all(&source).unwrap();
        fs::remove_file(&icon).unwrap();
        assert!(inspect_selection(&moved, &selected).complete);
        assert_eq!(
            prepare_selection_with_icon(&source, &moved, &selected, Some(&icon)).unwrap(),
            PrepareResult::AlreadyPresent
        );
        fs::write(moved.join("ui/reveal-chaos-token.png"), b"changed").unwrap();
        assert!(prepare_selection_with_icon(&source, &moved, &selected, Some(&icon)).is_err());
        assert_eq!(
            fs::read(moved.join("ui/reveal-chaos-token.png")).unwrap(),
            b"changed"
        );
    }

    #[test]
    fn conflicts_and_unknown_contents_are_preserved() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selection = fixture(&source);
        prepare_selection(&source, &output, &selection).unwrap();
        let extra = output.join("owner-notes.txt");
        fs::write(&extra, b"keep this").unwrap();
        assert!(prepare_selection(&source, &output, &selection).is_err());
        assert_eq!(fs::read(&extra).unwrap(), b"keep this");
        fs::remove_file(&extra).unwrap();
        fs::write(output.join("pictures/first.png"), b"changed").unwrap();
        assert!(prepare_selection(&source, &output, &selection).is_err());
        assert_eq!(
            fs::read(output.join("pictures/first.png")).unwrap(),
            b"changed"
        );

        let empty = directory.path().join("empty-existing");
        fs::create_dir(&empty).unwrap();
        assert!(prepare_selection(&source, &empty, &selection).is_err());
        assert!(fs::read_dir(empty).unwrap().next().is_none());
    }

    #[test]
    fn a_failed_source_check_does_not_publish_or_touch_another_collection() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let selection = fixture(&source);
        let active = directory.path().join("active");
        prepare_selection(&source, &active, &selection).unwrap();
        let before = fs::read(active.join(MANIFEST)).unwrap();
        fs::write(source.join("second"), b"bad source").unwrap();
        let output = directory.path().join("new-parent/new-assets");
        assert!(prepare_selection(&source, &output, &selection).is_err());
        assert!(!output.parent().unwrap().exists());
        assert_eq!(fs::read(active.join(MANIFEST)).unwrap(), before);
        assert!(inspect_selection(&active, &selection).complete);
    }

    #[test]
    fn missing_and_changed_assets_are_reported_individually() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selection = fixture(&source);
        prepare_selection(&source, &output, &selection).unwrap();
        fs::remove_file(output.join("pictures/first.png")).unwrap();
        let report = inspect_selection(&output, &selection);
        assert!(report.recognized && !report.complete);
        assert_eq!(report.valid_count(), 1);
        assert!(report.verified_path("first").is_none());
        assert!(report.verified_path("second").is_some());
        // Keep the same byte count: matching length alone must not admit changed art.
        fs::write(output.join("pictures/second.png"), b"OTHER BYTES").unwrap();
        let report = inspect_selection(&output, &selection);
        assert_eq!(report.valid_count(), 0);
        assert_eq!(report.diagnostics.len(), 2);
    }

    #[test]
    fn a_manifest_cannot_declare_away_required_assets_or_redirect_reads() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selection = fixture(&source);
        prepare_selection(&source, &output, &selection).unwrap();
        let original = serde_json::to_value(selection.manifest()).unwrap();
        let mut missing = original.clone();
        missing["assets"].as_array_mut().unwrap().pop();
        let mut traversal = original.clone();
        traversal["assets"][0]["path"] = serde_json::json!("../../source/first");
        let mut wrong_hash = original;
        wrong_hash["assets"][0]["sha256"] = serde_json::json!("another hash");
        for altered in [missing, traversal, wrong_hash] {
            fs::write(output.join(MANIFEST), serde_json::to_vec(&altered).unwrap()).unwrap();
            let report = inspect_selection(&output, &selection);
            assert!(!report.recognized && !report.complete);
            assert_eq!(report.valid_count(), 0);
        }
        fs::write(output.join(MANIFEST), b"not json").unwrap();
        assert!(!inspect_selection(&output, &selection).recognized);
        assert!(!inspect_selection(&directory.path().join("absent"), &selection).recognized);
    }

    #[test]
    fn source_overlap_and_linked_payloads_are_refused() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let selection = fixture(&source);
        assert!(prepare_selection(&source, &source.join("new/assets"), &selection).is_err());
        assert!(!source.join("new").exists());
        let alias = directory.path().join("source-link");
        symlink(&source, &alias).unwrap();
        assert!(prepare_selection(&source, &alias.join("new/assets"), &selection).is_err());
        assert!(!source.join("new").exists());

        let output = directory.path().join("assets");
        prepare_selection(&source, &output, &selection).unwrap();
        fs::remove_file(output.join("pictures/first.png")).unwrap();
        symlink(source.join("first"), output.join("pictures/first.png")).unwrap();
        let report = inspect_selection(&output, &selection);
        assert!(!report.complete && report.verified_path("first").is_none());
    }

    #[test]
    fn selected_sources_remain_bound_to_the_independent_experiment_evidence() {
        let selected = gathering_selection();
        let mut expected = BTreeMap::new();
        let cards: serde_json::Value =
            serde_json::from_str(include_str!("../docs/research/issue-1-evidence.json")).unwrap();
        for card in cards["inspected_card_faces"]["faces"].as_array().unwrap() {
            expected.insert(
                card["face_path"].as_str().unwrap().to_owned(),
                card["sha256"].as_str().unwrap().to_owned(),
            );
        }
        let tokens: serde_json::Value =
            serde_json::from_str(include_str!("../docs/experiments/token-art.json")).unwrap();
        for token in tokens["faces"].as_array().unwrap().iter() {
            expected.insert(
                token["path"].as_str().unwrap().to_owned(),
                token["sha256"].as_str().unwrap().to_owned(),
            );
        }
        let standees: serde_json::Value =
            serde_json::from_str(include_str!("../docs/experiments/investigator-art.json"))
                .unwrap();
        for standee in standees["investigators"].as_array().unwrap() {
            expected.insert(
                standee["path"].as_str().unwrap().to_owned(),
                standee["sha256"].as_str().unwrap().to_owned(),
            );
        }
        let font: serde_json::Value =
            serde_json::from_str(include_str!("../docs/experiments/symbol-art.json")).unwrap();
        expected.insert(
            font["path"].as_str().unwrap().to_owned(),
            font["sha256"].as_str().unwrap().to_owned(),
        );
        // Independently selected unchanged PNG; the accepted handoff pins this identity.
        expected.insert(
            "swap-bag.png".into(),
            "54673d4cc97a84c90cab1d0a38b46183eb182c567493de676d7124ff3d70fe3e".into(),
        );
        // Independently inspected original Gathering Easy / Standard face (#77).
        expected.insert(
            "images/cards/official/en/night_of_the_zealot/Unassigned expansion/01104--Scenario/ee9f88be0fe1ad2b/face.png".into(),
            "2c65c1d4b98f26f4268cfa4647b006ffa9a6d7030281a545e676ad41c1d9ceb5".into(),
        );
        let actual: BTreeMap<_, _> = selected
            .assets
            .iter()
            .map(|asset| (asset.source.path.clone(), asset.source.sha256.clone()))
            .collect();
        for (path, hash) in expected {
            assert_eq!(actual.get(&path), Some(&hash), "{path}");
        }
        let selected_font = selected
            .assets
            .iter()
            .find(|asset| asset.role == "font/symbols")
            .unwrap();
        assert_eq!(selected_font.sha256, font["font_sha256"].as_str().unwrap());
        assert_eq!(selected_font.bytes, font["font_length"].as_u64().unwrap());
    }

    #[test]
    fn gathering_harm_art_and_symbols_are_verified_individually() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selected = gathering_fixture(&source);
        prepare_selection(&source, &output, &selected).unwrap();
        let report = inspect_selection(&output, &selected);
        assert_eq!(
            report.verified_path("token/damage"),
            Some("tokens/damage.png")
        );
        assert_eq!(
            report.verified_path("token/horror"),
            Some("tokens/horror.png")
        );
        assert_eq!(
            report.verified_path("font/symbols"),
            Some("fonts/arkham-symbols.ttf")
        );
        assert!(report.complete);

        fs::remove_file(output.join("tokens/damage.png")).unwrap();
        fs::write(output.join("tokens/horror.png"), b"changed horror").unwrap();
        fs::remove_file(output.join("fonts/arkham-symbols.ttf")).unwrap();
        let report = inspect_selection(&output, &selected);
        assert!(report.recognized && !report.complete);
        assert_eq!((report.valid_count(), report.expected_count()), (145, 148));
        assert!(report.verified_path("token/damage").is_none());
        assert!(report.verified_path("token/horror").is_none());
        assert!(report.verified_path("font/symbols").is_none());
        assert!(report.verified_path("card-back/player").is_some());
    }

    #[test]
    fn gathering_back_integrity_and_prior_collection_preserve_available_art() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selected = gathering_fixture(&source);
        prepare_selection(&source, &output, &selected).unwrap();
        assert!(inspect_selection(&output, &selected).complete);
        let back = output.join("cards/player-back.png");
        let original = fs::read(&back).unwrap();
        for replacement in [None, Some(vec![b'x'; original.len()])] {
            if let Some(bytes) = replacement {
                fs::write(&back, bytes).unwrap();
            } else {
                fs::remove_file(&back).unwrap();
            }
            let report = inspect_selection(&output, &selected);
            assert!(report.recognized && !report.complete);
            assert_eq!(report.valid_count(), 147);
            assert_eq!(report.expected_count(), 148);
            assert!(report.verified_path("card-06003").is_some());
            assert!(report.verified_path("card-back/player").is_none());
        }
        fs::remove_file(&back).unwrap();
        for later in [
            "key.jpg",
            "bounty.png",
            "ammo.png",
            "charge-motif.png",
            "investigator-ability.png",
            "offering.png",
            "doom.png",
        ] {
            fs::remove_file(output.join("tokens").join(later)).unwrap();
        }
        let previous = serde_json::to_vec(&previous_manifests(&selected)[1]).unwrap();
        fs::write(output.join(MANIFEST), &previous).unwrap();
        let report = inspect_selection(&output, &selected);
        assert!(report.recognized && !report.complete);
        assert_eq!((report.valid_count(), report.expected_count()), (140, 148));
        assert!(prepare_selection(&source, &output, &selected).is_err());
        assert_eq!(fs::read(output.join(MANIFEST)).unwrap(), previous);

        // Adding bytes cannot upgrade an earlier manifest or permit overwriting it.
        fs::write(&back, original).unwrap();
        let report = inspect_selection(&output, &selected);
        assert_eq!(report.valid_count(), 141);
        assert!(!report.complete);
        assert!(prepare_selection(&source, &output, &selected).is_err());
    }

    #[test]
    fn gathering_compatibility_does_not_accept_other_manifest_changes() {
        let directory = tempfile::tempdir().unwrap();
        let selected = gathering_fixture(&directory.path().join("source"));
        for previous in previous_manifests(&selected) {
            let original = serde_json::to_value(previous).unwrap();
            let mut missing = original.clone();
            missing["assets"].as_array_mut().unwrap().pop();
            let mut altered = original.clone();
            altered["assets"][0]["sha256"] = serde_json::json!("different");
            let mut redirected = original.clone();
            redirected["assets"][0]["path"] = serde_json::json!("../../other.png");
            let mut renamed = original;
            renamed["collection"] = serde_json::json!("arkham-unfolded-gathering-v7");
            for manifest in [missing, altered, redirected, renamed] {
                fs::write(
                    directory.path().join(MANIFEST),
                    serde_json::to_vec(&manifest).unwrap(),
                )
                .unwrap();
                let report = inspect_selection(directory.path(), &selected);
                assert!(!report.recognized && !report.complete);
                assert_eq!(report.valid_count(), 0);
            }
        }
    }
}
