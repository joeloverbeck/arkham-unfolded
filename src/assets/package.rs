//! Runtime-only Gathering ZIPs. The compiled recipe, never the ZIP, owns requirements.
use super::{MANIFEST, MAX_MANIFEST_BYTES, Manifest, gathering_selection, inspect_gathering};
use crate::asset_config::AssetConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const METADATA_BUDGET: u64 = 256 * 1024;
const RECEIPT: &str = "installation.json";

/// Held for the lifetime of a native attempt. OS locks are released on exit/crash.
pub struct ArtworkUse {
    _play: File,
    _installation: Option<File>,
}

/// Pin the effective collection before opening a save or starting asynchronous loading.
pub fn acquire_play(config: &AssetConfig, root: &Path) -> std::result::Result<ArtworkUse, String> {
    let acquire = || -> Result<ArtworkUse> {
        let managed = config.installation_directory();
        fs::create_dir_all(&managed)?;
        let play = lock_file(&managed.join("play.lock"))?;
        play.try_lock_shared()
            .map_err(|_| "Artwork import is in progress; retry after it finishes")?;
        config.ensure_unchanged()?;
        let installation = if managed_collection(root, &managed) {
            let file = usage_file(root)?;
            file.try_lock_shared()
                .map_err(|_| "This artwork installation is being retired; restart")?;
            Some(file)
        } else {
            None
        };
        let report = inspect_gathering(root);
        if !report.complete {
            return Err(report.startup_failure(root).into());
        }
        Ok(ArtworkUse {
            _play: play,
            _installation: installation,
        })
    };
    acquire().map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Installation {
    previous: Option<PathBuf>,
    previous_digest: Option<String>,
    admitted: bool,
}

pub fn required_collection() -> &'static str {
    &gathering_selection().collection
}

/// A cheap selection-screen description, not a completeness or rendering claim.
pub fn installation_status(config: &AssetConfig) -> String {
    let identity = super::regular_file(config.root(), MANIFEST)
        .and_then(|p| super::read_bounded(&p, MAX_MANIFEST_BYTES))
        .ok()
        .and_then(|b| serde_json::from_slice::<Manifest>(&b).ok())
        .map(|m| m.collection.chars().take(100).collect::<String>())
        .unwrap_or_else(|| "none recognized".into());
    let managed = config.installation_directory();
    let selected = config.root().parent();
    let retained_previous = selected
        .and_then(|p| read_receipt(p).ok())
        .and_then(|r| r.previous);
    let interrupted = fs::read_dir(&managed)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            entry.file_type().is_ok_and(|t| t.is_dir())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("installation-")
                && Some(entry.path().as_path()) != selected
                && retained_previous.as_deref() != Some(entry.path().join("collection").as_path())
                && read_receipt(&entry.path()).map_or(true, |r| !r.admitted)
        })
        .count();
    let mut status = format!(
        "Selected artwork: {identity}\nRequired: {} (checked before play)",
        required_collection()
    );
    if interrupted > 0 {
        status.push_str(&format!("\n{interrupted} unselected pending or unknown import directories retained. Import the ZIP again to recover; these are never selected automatically."));
    }
    status
}

/// Writes a fresh package, selecting only trusted payloads and identity/attribution metadata.
/// An existing output is never replaced, even when it contains a different package.
pub fn package_gathering(root: &Path, output: &Path) -> std::result::Result<String, String> {
    package(root, output).map_err(|e| e.to_string())
}

fn package(root: &Path, output: &Path) -> Result<String> {
    let report = inspect_gathering(root);
    if !report.complete {
        return Err(report.startup_failure(root).into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    let mut writer = ZipWriter::new(temporary.as_file_mut());
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let manifest = serde_json::to_vec_pretty(&gathering_selection().manifest())?;
    writer.start_file(MANIFEST, options)?;
    writer.write_all(&manifest)?;
    for asset in &gathering_selection().assets {
        let path = super::regular_file(root, &asset.path)?;
        let mut input = File::open(path)?;
        writer.start_file(&asset.path, options)?;
        copy_verified(
            &mut input,
            &mut writer,
            asset.bytes,
            Some(&asset.sha256),
            &AtomicBool::new(false),
        )?;
    }
    writer.finish()?.sync_all()?;
    let mut file = File::open(temporary.path())?;
    let (length, digest) = hash_reader(&mut file)?;
    temporary.persist_noclobber(output)?;
    File::open(parent)?.sync_all()?;
    Ok(format!(
        "{}: {length} bytes; SHA-256 {digest}",
        output.display()
    ))
}

pub fn import_gathering(
    zip: &Path,
    config: &mut AssetConfig,
) -> std::result::Result<PathBuf, String> {
    import_gathering_cancellable(zip, config, &AtomicBool::new(false))
}

/// Cancellation is observed between streaming blocks and before saving the selection.
pub fn import_gathering_cancellable(
    zip: &Path,
    config: &mut AssetConfig,
    cancel: &AtomicBool,
) -> std::result::Result<PathBuf, String> {
    import(zip, config, cancel).map_err(|e| format!("Artwork import failed: {e}. Required collection: {}. Previous artwork and saves are retained.", required_collection()))
}

fn import(path: &Path, config: &mut AssetConfig, cancel: &AtomicBool) -> Result<PathBuf> {
    if let Some(warning) = config.warning() {
        return Err(warning.into());
    }
    let descriptor = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NONBLOCK | rustix::fs::OFlags::NOFOLLOW,
        rustix::fs::Mode::empty(),
    )?;
    let mut input = File::from(descriptor);
    if !input.metadata()?.is_file() {
        return Err("Choose a regular ZIP file, not a link or special file".into());
    }
    check_container(&mut input)?;
    let mut archive = ZipArchive::new(input)?;
    let selection = gathering_selection();
    if archive.has_duplicate_names() || archive.has_overlapping_entries()? {
        return Err("ZIP has missing, duplicate, overlapping or undeclared entries".into());
    }
    let managed = config.installation_directory();
    fs::create_dir_all(&managed)?;
    let _lock = lock(&managed)?;
    let play = lock_file(&managed.join("play.lock"))?;
    play.try_lock()
        .map_err(|_| "Close active play before importing artwork")?;
    config.ensure_unchanged()?;
    canceled(cancel)?;
    // Read the bounded identity first, so recognizable older packages explain the mismatch.
    let mut manifest_bytes = Vec::new();
    copy_verified(
        &mut archive.by_name(MANIFEST)?,
        &mut manifest_bytes,
        MAX_MANIFEST_BYTES,
        None,
        cancel,
    )?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest != selection.manifest() {
        return Err(format!(
            "Incompatible artwork collection '{}'; import {}",
            manifest.collection.chars().take(100).collect::<String>(),
            required_collection()
        )
        .into());
    }
    if archive.len() != selection.assets.len() + 1 {
        return Err("ZIP has missing or undeclared entries".into());
    }
    let space = rustix::fs::statvfs(&managed)?;
    // Available space already excludes the old installation and retained download.
    let required = selection.assets.iter().map(|a| a.bytes).sum::<u64>() + METADATA_BUDGET;
    if space.f_bavail.saturating_mul(space.f_frsize) < required {
        return Err(format!("Not enough disk space: {required} additional bytes are needed while retaining existing artwork and the ZIP").into());
    }
    let stage = tempfile::Builder::new()
        .prefix("installation-")
        .tempdir_in(&managed)?;
    let root = stage.path().join("collection");
    fs::create_dir(&root)?;
    let mut seen = BTreeSet::new();
    for index in 0..archive.len() {
        canceled(cancel)?;
        let mut member = archive.by_index(index)?;
        let name = std::str::from_utf8(member.name_raw())?.to_owned();
        // Only exact recipe destinations are admitted: no alias normalization, links or directories.
        if !seen.insert(name.clone())
            || member.encrypted()
            || member
                .unix_mode()
                .is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
        {
            return Err("ZIP contains a duplicate, encrypted entry, link or special file".into());
        }
        let expected = selection.assets.iter().find(|a| a.path == name);
        let (limit, hash) = if name == MANIFEST {
            (MAX_MANIFEST_BYTES, None)
        } else if let Some(asset) = expected {
            (asset.bytes, Some(asset.sha256.as_str()))
        } else {
            return Err(format!(
                "Undeclared ZIP destination: {}",
                name.chars().take(120).collect::<String>()
            )
            .into());
        };
        let destination = root.join(&name);
        fs::create_dir_all(destination.parent().ok_or("Missing parent")?)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        copy_verified(&mut member, &mut output, limit, hash, cancel)?;
        output.sync_all()?;
        if name == MANIFEST {
            let manifest: Manifest = serde_json::from_slice(&fs::read(&destination)?)?;
            if manifest != selection.manifest() {
                return Err(format!(
                    "Incompatible artwork collection '{}'; import {}",
                    manifest.collection,
                    required_collection()
                )
                .into());
            }
        }
    }
    let report = inspect_gathering(&root);
    if !report.complete {
        return Err(report.startup_failure(&root).into());
    }
    canceled(cancel)?;
    // Reimporting the same valid managed collection is idempotent, including pending admission.
    if managed_collection(config.root(), &managed) && inspect_gathering(config.root()).complete {
        canceled(cancel)?;
        return Ok(config.root().to_owned());
    }
    let previous = config.root().to_owned();
    let previous_digest = if managed_collection(&previous, &managed) {
        let _usage = usage_file(&previous)?;
        Some(tree_digest(
            previous.parent().ok_or("Missing previous parent")?,
        )?)
    } else {
        None
    };
    let _usage = usage_file(&root)?;
    write_receipt(
        stage.path(),
        &Installation {
            previous: Some(previous),
            previous_digest,
            admitted: false,
        },
    )?;
    sync_directories(&root)?;
    File::open(stage.path())?.sync_all()?;
    File::open(&managed)?.sync_all()?;
    // Cancellation ends at this activation boundary; no lengthy work follows it.
    canceled(cancel)?;
    let installation = stage.keep();
    if let Err(error) = config.save_root(&root) {
        // This exact directory was created by this call and was never selected successfully.
        fs::remove_dir_all(installation)?;
        return Err(error.into());
    }
    File::open(config.path().parent().ok_or("Missing settings parent")?)?.sync_all()?;
    Ok(root)
}

/// Called only after the existing image/font/model/material and renderer gate admits play.
/// Unknown external installations are always retained. Drift stops managed-copy removal.
#[cfg(feature = "tabletop")]
pub(crate) fn finish_admission(
    root: &Path,
    config: &AssetConfig,
) -> std::result::Result<(), String> {
    finish(root, config).map_err(|e| {
        format!("Artwork loaded; previous installation retained or cleanup needs attention: {e}")
    })
}

#[cfg(feature = "tabletop")]
fn finish(root: &Path, config: &AssetConfig) -> Result<()> {
    let managed = config.installation_directory();
    if !managed_collection(root, &managed) {
        return Ok(());
    }
    let _lock = lock(&managed)?;
    let installation = root.parent().ok_or("Missing installation parent")?;
    let mut receipt = read_receipt(installation)?;
    if receipt.admitted {
        return Ok(());
    }
    config.ensure_unchanged()?;
    if config.root() != root {
        return Err(
            "Temporary artwork override is not the saved installation; cleanup was skipped".into(),
        );
    }
    if let (Some(previous), Some(expected)) = (&receipt.previous, &receipt.previous_digest) {
        if previous == root || !managed_location(previous, &managed) {
            return Err("Previous installation identity changed".into());
        }
        let previous_dir = previous.parent().ok_or("Missing previous installation")?;
        match fs::symlink_metadata(previous_dir) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
            Ok(_) => {
                let in_use = usage_file(previous)?;
                in_use.try_lock().map_err(
                    |_| "Previous artwork is still in use; cleanup will retry on a later launch",
                )?;
                if !managed_collection(previous, &managed)
                    || tree_digest(previous_dir)? != *expected
                {
                    return Err("Previous installation changed; no files removed".into());
                }
                fs::remove_dir_all(previous_dir)?;
                File::open(&managed)?.sync_all()?;
            }
        }
    }
    receipt.admitted = true;
    write_receipt(installation, &receipt)?;
    Ok(())
}

fn managed_collection(root: &Path, managed: &Path) -> bool {
    managed_location(root, managed)
        && root.parent().is_some_and(|p| {
            fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !m.is_symlink())
                && read_receipt(p).is_ok()
        })
}
fn managed_location(root: &Path, managed: &Path) -> bool {
    root.file_name().is_some_and(|n| n == "collection")
        && root.parent().is_some_and(|p| {
            p.parent() == Some(managed)
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("installation-"))
        })
}
fn read_receipt(installation: &Path) -> Result<Installation> {
    let path = super::regular_file(installation, RECEIPT)?;
    Ok(serde_json::from_slice(&super::read_bounded(
        &path,
        MAX_MANIFEST_BYTES,
    )?)?)
}
fn write_receipt(installation: &Path, receipt: &Installation) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(installation)?;
    file.write_all(&serde_json::to_vec(receipt)?)?;
    file.as_file().sync_all()?;
    file.persist(installation.join(RECEIPT))?;
    File::open(installation)?.sync_all()?;
    Ok(())
}
fn lock(managed: &Path) -> Result<File> {
    let file = lock_file(&managed.join("import.lock"))?;
    file.try_lock()
        .map_err(|_| "Another artwork operation is in progress")?;
    Ok(file)
}
fn usage_file(root: &Path) -> Result<File> {
    lock_file(
        &root
            .parent()
            .ok_or("Missing installation parent")?
            .join("usage.lock"),
    )
}
fn lock_file(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?)
}
fn canceled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err("Import canceled".into())
    } else {
        Ok(())
    }
}
fn copy_verified(
    input: &mut impl Read,
    output: &mut impl Write,
    limit: u64,
    expected: Option<&str>,
    cancel: &AtomicBool,
) -> Result<()> {
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    let mut hash = Sha256::new();
    loop {
        canceled(cancel)?;
        let length = input.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        total += length as u64;
        if total > limit {
            return Err("Decompressed entry exceeds its trusted size limit".into());
        }
        hash.update(&buffer[..length]);
        output.write_all(&buffer[..length])?;
    }
    if let Some(expected) = expected
        && (total != limit || format!("{:x}", hash.finalize()) != expected)
    {
        return Err("Artwork length or SHA-256 differs from the required collection".into());
    }
    Ok(())
}
fn hash_reader(input: &mut impl Read) -> Result<(u64, String)> {
    let mut hash = Sha256::new();
    let mut count = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let length = input.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        count += length as u64;
        hash.update(&buffer[..length]);
    }
    Ok((count, format!("{:x}", hash.finalize())))
}
fn tree_digest(root: &Path) -> Result<String> {
    let mut paths = Vec::new();
    fn visit(root: &Path, relative: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(root.join(relative))? {
            let entry = entry?;
            let path = relative.join(entry.file_name());
            let kind = entry.file_type()?;
            if kind.is_dir() {
                visit(root, &path, paths)?;
            } else if !kind.is_file() {
                return Err("Unexpected link or special file in installation".into());
            }
            paths.push(path);
        }
        Ok(())
    }
    visit(root, Path::new(""), &mut paths)?;
    paths.sort();
    let mut hash = Sha256::new();
    for path in paths {
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update([0]);
        if root.join(&path).is_file() {
            let (length, digest) = hash_reader(&mut File::open(root.join(path))?)?;
            hash.update(length.to_le_bytes());
            hash.update(digest);
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn sync_directories(root: &Path) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            sync_directories(&entry.path())?;
        }
    }
    File::open(root)?.sync_all()?;
    Ok(())
}

// This package is < 2 GiB and needs no ZIP64/multi-volume container. Bound the
// central directory before ZipArchive allocates entry metadata. The ZIP library
// remains responsible for decoding, CRCs, offsets and overlapping-entry checks.
fn check_container(input: &mut File) -> Result<()> {
    let length = input.metadata()?.len();
    let payload = gathering_selection()
        .assets
        .iter()
        .map(|a| a.bytes)
        .sum::<u64>();
    if length > payload + METADATA_BUDGET || length < 22 {
        return Err("ZIP size is outside the supported package bounds".into());
    }
    let tail_length = length.min(65_557);
    input.seek(SeekFrom::End(-(tail_length as i64)))?;
    let mut tail = vec![0; tail_length as usize];
    input.read_exact(&mut tail)?;
    let offset = (0..=tail.len() - 22)
        .rev()
        .find(|&i| {
            &tail[i..i + 4] == b"PK\x05\x06"
                && i + 22 + u16::from_le_bytes([tail[i + 20], tail[i + 21]]) as usize == tail.len()
        })
        .ok_or("Missing ZIP directory footer")?;
    let footer = &tail[offset..];
    let u16_at = |i| u16::from_le_bytes([footer[i], footer[i + 1]]) as usize;
    let u32_at = |i| u32::from_le_bytes(footer[i..i + 4].try_into().expect("footer field")) as u64;
    if u16_at(4) != 0
        || u16_at(6) != 0
        || u16_at(8) != u16_at(10)
        || u16_at(10) == 0
        || u16_at(10) > gathering_selection().assets.len() + 1
        || u32_at(12) > METADATA_BUDGET
        || u32_at(16) + u32_at(12) != length - tail_length + offset as u64
    {
        return Err("Unsupported ZIP directory: expected one complete runtime package".into());
    }
    input.rewind()?;
    Ok(())
}

#[cfg(all(test, feature = "tabletop"))]
#[path = "package_tests.rs"]
mod tests;
