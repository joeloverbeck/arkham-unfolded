//! Preparation and verification of this application's finite external asset selection.
//! The runtime consumes prepared files; only `prepare` knows the source archive layout.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};

const MANIFEST: &str = "manifest.json";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

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
    UnityFont {
        block_offset: usize,
        block_length: usize,
        decoded_length: usize,
        font_offset: usize,
        font_length: usize,
    },
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    collection: String,
    assets: Vec<AssetIdentity>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
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

fn selection() -> &'static Selection {
    static SELECTION: OnceLock<Selection> = OnceLock::new();
    SELECTION.get_or_init(|| {
        let selection: Selection =
            serde_json::from_str(include_str!("../resources/asset-selection.json"))
                .expect("the committed asset selection must be valid");
        let mut roles = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for asset in &selection.assets {
            assert!(relative_path(&asset.path).is_ok());
            assert!(relative_path(&asset.source.path).is_ok());
            assert!(roles.insert(&asset.role) && paths.insert(&asset.path));
        }
        selection
    })
}

fn tony_selection() -> &'static Selection {
    static SELECTION: OnceLock<Selection> = OnceLock::new();
    SELECTION.get_or_init(|| {
        serde_json::from_str(include_str!("../resources/tony-asset-selection.json"))
            .expect("the committed Tony asset selection must be valid")
    })
}

// Exact earlier selections remain useful without claiming newly selected artwork.
fn previous_manifests(selection: &Selection) -> Vec<Manifest> {
    let versions: &[u32] = match selection.collection.as_str() {
        "arkham-unfolded-tony-turn-v3" => &[1, 2],
        "arkham-unfolded-tony-turn-v4" => &[1, 2, 3],
        "arkham-unfolded-jacqueline-v2" => &[1],
        _ => return Vec::new(),
    };
    let tony = selection
        .collection
        .starts_with("arkham-unfolded-tony-turn-");
    versions
        .iter()
        .copied()
        .map(|version| {
            let mut manifest = selection.manifest();
            manifest.collection = if tony {
                format!("arkham-unfolded-tony-turn-v{version}")
            } else {
                format!("arkham-unfolded-jacqueline-v{version}")
            };
            manifest.assets.retain(|asset| {
                asset.role != "ui/reveal-chaos-token"
                    && (!tony
                        || version >= 3
                        || !matches!(
                            asset.role.as_str(),
                            "token/damage" | "token/horror" | "font/symbols"
                        ))
                    && (!tony || version >= 2 || asset.role != "card-back/player")
            });
            manifest
        })
        .collect()
}

/// A recognized collection can be incomplete and still supply individually verified art.
/// An unrelated or altered manifest supplies no assets. Gameplay can use substitutes.
#[derive(Debug)]
pub struct CollectionReport {
    pub recognized: bool,
    pub complete: bool,
    pub diagnostics: Vec<String>,
    verified: BTreeMap<String, String>,
    expected: usize,
}

impl CollectionReport {
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
                "Incomplete asset collection: {}/{} verified. Unavailable artwork uses readable substitutes.",
                self.valid_count(),
                self.expected
            )
        } else {
            "No compatible prepared asset collection. Using readable substitutes.".into()
        }
    }
}

/// Check only the finite expected files. This never searches or falls back to an archive.
pub fn inspect(root: &Path) -> CollectionReport {
    inspect_selection(root, selection())
}

/// Inspect the separately prepared Tony-turn collection, preserving legacy collections.
pub fn inspect_tony(root: &Path) -> CollectionReport {
    inspect_selection(root, tony_selection())
}

fn inspect_selection(root: &Path, selection: &Selection) -> CollectionReport {
    let mut report = CollectionReport {
        recognized: false,
        complete: false,
        diagnostics: Vec::new(),
        verified: BTreeMap::new(),
        expected: selection.assets.len(),
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
                "Earlier collection: prepare a fresh collection for the newly selected artwork. Existing verified art remains available; missing images use readable substitutes.".into(),
            );
        }
        Ok(_) => report.diagnostics.push(
            "manifest.json does not describe the assets expected by this application.".into(),
        ),
        Err(error) => report.diagnostics.push(format!("manifest.json: {error}")),
    }
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
            Err(error) => report.diagnostics.push(format!("{}: {error}", asset.path)),
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

#[derive(Debug)]
pub struct AssetError(pub String);

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for AssetError {}

impl From<std::io::Error> for AssetError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<serde_json::Error> for AssetError {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}

/// Prepare a fresh directory, or reuse an identical complete directory without writing it.
/// Changed, incomplete or unknown destinations are refused. Source files are never written.
pub fn prepare(
    source: &Path,
    output: &Path,
    reveal_icon: &Path,
) -> Result<PrepareResult, AssetError> {
    prepare_selection_with_icon(source, output, selection(), Some(reveal_icon))
}

/// Prepare only the finite assets selected for the Tony-turn experiment.
pub fn prepare_tony(
    source: &Path,
    output: &Path,
    reveal_icon: &Path,
) -> Result<PrepareResult, AssetError> {
    prepare_selection_with_icon(source, output, tony_selection(), Some(reveal_icon))
}

#[cfg(test)]
fn prepare_selection(
    source: &Path,
    output: &Path,
    selection: &Selection,
) -> Result<PrepareResult, AssetError> {
    prepare_selection_with_icon(source, output, selection, None)
}

fn prepare_selection_with_icon(
    source: &Path,
    output: &Path,
    selection: &Selection,
    reveal_icon: Option<&Path>,
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
    // finite selection is about 12 MB; retaining it also avoids source reread races.
    let mut prepared = Vec::with_capacity(selection.assets.len());
    for asset in &selection.assets {
        let source_bytes = match asset.source.input {
            SourceInput::Archive => read_verified(
                &source,
                &asset.source.path,
                asset.source.bytes,
                &asset.source.sha256,
            ),
            SourceInput::RevealIcon => {
                let path = reveal_icon.ok_or_else(|| {
                    AssetError(
                        "Preparation requires --reveal-icon PATH for the selected pouch image."
                            .into(),
                    )
                })?;
                if !fs::symlink_metadata(path)?.file_type().is_file() {
                    return Err(AssetError(
                        "The reveal icon must be a regular file, not a symbolic link.".into(),
                    ));
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

    // Ubuntu's atomic no-replace rename protects even a destination created by
    // another process after the initial check. An incomplete directory is never published.
    rustix::fs::renameat_with(
        rustix::fs::CWD,
        stage.path(),
        rustix::fs::CWD,
        &output,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|error| {
        AssetError(format!(
            "Could not publish {} without replacing existing data: {error}",
            output.display()
        ))
    })?;
    Ok(PrepareResult::Created)
}

fn transform(bytes: Vec<u8>, transform: &Transform) -> Result<Vec<u8>, AssetError> {
    match *transform {
        Transform::Copy => Ok(bytes),
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

fn relative_path(value: &str) -> Result<&Path, AssetError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(AssetError(
            "Asset paths must be ordinary relative paths inside the collection.".into(),
        ));
    }
    Ok(path)
}

fn regular_file(root: &Path, relative: &str) -> Result<PathBuf, AssetError> {
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(AssetError(
            "The root must be a directory, not a symbolic link.".into(),
        ));
    }
    let relative = relative_path(relative)?;
    let mut path = root.to_path_buf();
    let mut parts = relative.components().peekable();
    while let Some(part) = parts.next() {
        path.push(part);
        let kind = fs::symlink_metadata(&path)?.file_type();
        if kind.is_symlink()
            || if parts.peek().is_some() {
                !kind.is_dir()
            } else {
                !kind.is_file()
            }
        {
            return Err(AssetError(
                "Assets must be regular files with no symbolic links in their relative paths."
                    .into(),
            ));
        }
    }
    Ok(path)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, AssetError> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(AssetError("File exceeds its expected size limit.".into()));
    }
    Ok(bytes)
}

fn verify_bytes(bytes: &[u8], expected_length: u64, expected_hash: &str) -> Result<(), AssetError> {
    if bytes.len() as u64 != expected_length
        || format!("{:x}", Sha256::digest(bytes)) != expected_hash
    {
        return Err(AssetError(
            "File size or SHA-256 differs from the selected asset.".into(),
        ));
    }
    Ok(())
}

fn read_verified(root: &Path, path: &str, length: u64, hash: &str) -> Result<Vec<u8>, AssetError> {
    let path = regular_file(root, path)?;
    if fs::metadata(&path)?.len() != length {
        return Err(AssetError(
            "File size differs from the selected asset.".into(),
        ));
    }
    let bytes = read_bounded(&path, length)?;
    verify_bytes(&bytes, length, hash)?;
    Ok(bytes)
}

// Resolve existing ancestors before creating anything, including an output with
// new parent directories. This prevents accidentally preparing inside the archive.
fn resolve_output(output: &Path) -> Result<PathBuf, AssetError> {
    let mut existing = std::path::absolute(output)?;
    let mut suffix = Vec::new();
    while !existing.try_exists()? {
        suffix.push(
            existing
                .file_name()
                .ok_or_else(|| AssetError("Invalid output path.".into()))?
                .to_os_string(),
        );
        if !existing.pop() {
            return Err(AssetError("Invalid output path.".into()));
        }
    }
    let mut resolved = existing.canonicalize()?;
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn tony_fixture(source: &Path) -> Selection {
        let mut selected: Selection =
            serde_json::from_str(include_str!("../resources/tony-asset-selection.json")).unwrap();
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
    fn earlier_jacqueline_collection_keeps_verified_art_without_the_reveal_icon() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let mut earlier = fixture(&source);
        earlier.collection = "arkham-unfolded-jacqueline-v1".into();
        prepare_selection(&source, &output, &earlier).unwrap();
        let original = fs::read(output.join(MANIFEST)).unwrap();
        let mut selected = fixture(&source);
        selected.collection = "arkham-unfolded-jacqueline-v2".into();
        let mut icon = fixture(&source).assets.remove(0);
        icon.role = "ui/reveal-chaos-token".into();
        icon.path = "ui/reveal-chaos-token.png".into();
        selected.assets.push(icon);

        let report = inspect_selection(&output, &selected);
        assert!(report.recognized && !report.complete);
        assert_eq!(report.verified_path("first"), Some("pictures/first.png"));
        assert!(report.verified_path("ui/reveal-chaos-token").is_none());
        assert!(prepare_selection(&source, &output, &selected).is_err());
        assert_eq!(fs::read(output.join(MANIFEST)).unwrap(), original);
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
        let selected = selection();
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
        for token in tokens["faces"]
            .as_array()
            .unwrap()
            .iter()
            .chain(std::iter::once(&tokens["reaction_reminder"]))
        {
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
        let actual: BTreeMap<_, _> = selected
            .assets
            .iter()
            .map(|asset| (asset.source.path.clone(), asset.source.sha256.clone()))
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(selected.assets.len(), 14);
        assert_eq!(
            selected.assets.iter().map(|asset| asset.bytes).sum::<u64>(),
            12_141_618
        );
        let selected_font = selected
            .assets
            .iter()
            .find(|asset| asset.role == "font/symbols")
            .unwrap();
        assert_eq!(selected_font.sha256, font["font_sha256"].as_str().unwrap());
        assert_eq!(selected_font.bytes, font["font_length"].as_u64().unwrap());
        for image in selected
            .assets
            .iter()
            .filter(|asset| asset.role != "font/symbols")
        {
            assert_eq!(image.sha256, image.source.sha256);
            assert_eq!(image.bytes, image.source.bytes);
            assert!(matches!(image.transform, Transform::Copy));
        }
    }

    #[test]
    fn tony_harm_art_and_symbols_are_verified_individually() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selected = tony_fixture(&source);
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
        assert_eq!((report.valid_count(), report.expected_count()), (26, 29));
        assert!(report.verified_path("token/damage").is_none());
        assert!(report.verified_path("token/horror").is_none());
        assert!(report.verified_path("font/symbols").is_none());
        assert!(report.verified_path("card-back/player").is_some());
    }

    #[test]
    fn tony_prior_versions_remain_readable_without_overwriting() {
        for (version, count) in [(1, 24), (2, 25), (3, 28)] {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("source");
            let output = directory.path().join("assets");
            let selected = tony_fixture(&source);
            let mut earlier = tony_fixture(&source);
            earlier.collection = format!("arkham-unfolded-tony-turn-v{version}");
            earlier.assets.retain(|asset| {
                asset.role != "ui/reveal-chaos-token"
                    && (version >= 3
                        || !matches!(
                            asset.role.as_str(),
                            "token/damage" | "token/horror" | "font/symbols"
                        ))
                    && (version >= 2 || asset.role != "card-back/player")
            });
            prepare_selection(&source, &output, &earlier).unwrap();
            let original_manifest = fs::read(output.join(MANIFEST)).unwrap();
            let report = inspect_selection(&output, &selected);
            assert!(report.recognized && !report.complete, "Tony V{version}");
            assert_eq!((report.valid_count(), report.expected_count()), (count, 29));
            assert!(report.verified_path("card/06003").is_some());
            assert_eq!(report.verified_path("token/damage").is_some(), version == 3);
            assert_eq!(report.verified_path("font/symbols").is_some(), version == 3);
            assert!(report.verified_path("ui/reveal-chaos-token").is_none());
            assert!(prepare_selection(&source, &output, &selected).is_err());
            assert_eq!(fs::read(output.join(MANIFEST)).unwrap(), original_manifest);
        }
    }

    #[test]
    fn tony_compatibility_is_bound_to_the_exact_prior_collection() {
        // Independently captured from V1 at b2cbdbd and V2 at 26f283f;
        // V3 and Jacqueline V1 from the installed manifests before this change.
        let hashes = previous_manifests(tony_selection())
            .iter()
            .map(|previous| {
                format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(previous).unwrap())
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            hashes,
            [
                "127d652276550b909d08b16010e1380145d6c20030a915aabb889454f353e865",
                "106bd1fc4c107f60d9a426064009d52cebf841de0d4e22174cfcc867ef887bcc",
                "af2750b1cd8ec1b300964c289790886d944095c5a617160e5d9f428b165c9f36",
            ]
        );
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&previous_manifests(selection())[0]).unwrap())
            ),
            "20e809159119b8505be659bc2a588df008effd93a83e5f969e74efa444d518e2"
        );
    }

    #[test]
    fn tony_back_integrity_and_prior_collection_preserve_available_art() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let output = directory.path().join("assets");
        let selected = tony_fixture(&source);
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
            assert_eq!(report.valid_count(), 28);
            assert_eq!(report.expected_count(), 29);
            assert!(report.verified_path("card/06003").is_some());
            assert!(report.verified_path("card-back/player").is_none());
        }
        fs::remove_file(&back).unwrap();
        let previous = serde_json::to_vec(&previous_manifests(&selected)[1]).unwrap();
        fs::write(output.join(MANIFEST), &previous).unwrap();
        let report = inspect_selection(&output, &selected);
        assert!(report.recognized && !report.complete);
        assert_eq!((report.valid_count(), report.expected_count()), (28, 29));
        assert!(prepare_selection(&source, &output, &selected).is_err());
        assert_eq!(fs::read(output.join(MANIFEST)).unwrap(), previous);

        // Adding bytes cannot upgrade an earlier manifest or permit overwriting it.
        fs::write(&back, original).unwrap();
        let report = inspect_selection(&output, &selected);
        assert_eq!(report.valid_count(), 29);
        assert!(!report.complete);
        assert!(prepare_selection(&source, &output, &selected).is_err());
    }

    #[test]
    fn tony_compatibility_does_not_accept_other_manifest_changes() {
        let directory = tempfile::tempdir().unwrap();
        let selected = tony_fixture(&directory.path().join("source"));
        for previous in previous_manifests(&selected) {
            let original = serde_json::to_value(previous).unwrap();
            let mut missing = original.clone();
            missing["assets"].as_array_mut().unwrap().pop();
            let mut altered = original.clone();
            altered["assets"][0]["sha256"] = serde_json::json!("different");
            let mut redirected = original.clone();
            redirected["assets"][0]["path"] = serde_json::json!("../../other.png");
            let mut renamed = original;
            renamed["collection"] = serde_json::json!("arkham-unfolded-tony-turn-v4");
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
