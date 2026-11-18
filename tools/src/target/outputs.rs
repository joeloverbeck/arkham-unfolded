//! Build outputs of the repository's own packages in the owner's target. Cargo gives every
//! file of one build of a target the same hash of that build's identity, which its
//! fingerprint directory `.fingerprint/<package>-<hash>` also carries, and it never removes
//! the files of an earlier identity. A new `unfolded-tabletop` revision, or a checkout's own
//! `codegen-units` value, gives every target of the repository a new identity.
//!
//! This is the layout Cargo 1.97 writes. Cargo 1.100 puts each build in
//! `build/<package>/<hash>/`, which this does not read, so it finds none there (#277).

use super::{Outcome, present, subdirectories, usage};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File, TryLockError},
    io,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

/// Where Cargo records each build of a profile, one directory per build.
const FINGERPRINTS: &str = ".fingerprint";
/// Where Cargo writes a profile's hashed files: libraries, executables and their dependency
/// lists.
const DIRECTORIES: [&str; 2] = ["deps", "examples"];
/// The lock Cargo 1.97 holds on a profile directory while it builds there.
const BUILD_LOCK: &str = ".cargo-build-lock";

/// One build of one of the repository's targets.
pub(super) struct Output {
    /// The profile directory, such as `<target>/debug`.
    pub(super) profile: PathBuf,
    /// Relative to the target, such as `debug/deps/gathering_driver-0123456789abcdef`.
    pub(super) name: PathBuf,
    files: Vec<PathBuf>,
    fingerprint: PathBuf,
    /// Bytes its removal frees: those of its files that have no other hard link.
    pub(super) bytes: u64,
    pub(super) unused_for: Duration,
    /// A newer build of the same target is in the same profile directory.
    superseded: bool,
}

/// The repository's build outputs in the target. A symbolic link where a fingerprint
/// directory, an output directory or an output's file would be is listed in `links`, never
/// followed or removed.
pub(super) struct Outputs {
    outputs: Vec<Output>,
    pub(super) links: Vec<PathBuf>,
}

impl Outputs {
    /// `packages` are the repository's own package names; other packages' builds are left out.
    pub(super) fn scan(target: &Path, packages: &[String]) -> Result<Self> {
        let mut found = Self {
            outputs: Vec::new(),
            links: Vec::new(),
        };
        let Some(root) = present(fs::canonicalize(target))
            .with_context(|| format!("resolve {}", target.display()))?
        else {
            return Ok(found);
        };
        let now = SystemTime::now();
        for child in subdirectories(&root)? {
            for profile in std::iter::once(child.clone()).chain(subdirectories(&child)?) {
                found.profile(&root, &profile, packages, now)?;
            }
        }
        // The newest build of each target in each profile directory is the one kept.
        let mut newest: HashMap<(&Path, (String, bool)), Duration> = HashMap::new();
        for output in &found.outputs {
            let unused_for = newest
                .entry((&output.profile, target_of(&output.files)))
                .or_insert(output.unused_for);
            *unused_for = (*unused_for).min(output.unused_for);
        }
        let superseded: Vec<bool> = found
            .outputs
            .iter()
            .map(|output| {
                output.unused_for > newest[&(output.profile.as_path(), target_of(&output.files))]
            })
            .collect();
        for (output, superseded) in found.outputs.iter_mut().zip(superseded) {
            output.superseded = superseded;
        }
        found
            .outputs
            .sort_by(|left, right| left.name.cmp(&right.name));
        Ok(found)
    }

    fn profile(
        &mut self,
        root: &Path,
        profile: &Path,
        packages: &[String],
        now: SystemTime,
    ) -> Result<()> {
        let Some(fingerprints) = self.directory(profile.join(FINGERPRINTS))? else {
            return Ok(());
        };
        let mut own = HashMap::new();
        for entry in read(&fingerprints)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some((package, hash)) = name.to_str().and_then(|name| name.rsplit_once('-')) else {
                continue;
            };
            if !is_hash(hash) || !packages.iter().any(|own| own == package) {
                continue;
            }
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                self.links.push(entry.path());
            } else if kind.is_dir() {
                own.insert(hash.to_owned(), entry.path());
            }
        }
        if own.is_empty() {
            return Ok(());
        }
        let mut files: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for directory in DIRECTORIES {
            let Some(directory) = self.directory(profile.join(directory))? else {
                continue;
            };
            for entry in read(&directory)? {
                let entry = entry?;
                let name = entry.file_name();
                let Some(hash) = name.to_str().and_then(hash_of) else {
                    continue;
                };
                if !own.contains_key(hash) {
                    continue;
                }
                let kind = entry.file_type()?;
                if kind.is_symlink() {
                    self.links.push(entry.path());
                } else if kind.is_file() {
                    files.entry(hash.to_owned()).or_default().push(entry.path());
                }
            }
        }
        // A build with no file in these directories, such as a build script's, is left.
        for (hash, mut paths) in files {
            paths.sort();
            let fingerprint = own[&hash].clone();
            // A build in progress elsewhere can remove files while they are read.
            let Some((bytes, newest)) = measure(&paths, &fingerprint)? else {
                continue;
            };
            let named = paths
                .iter()
                .find(|path| path.extension().is_some_and(|extension| extension == "d"))
                .map_or_else(|| paths[0].clone(), |path| path.with_extension(""));
            self.outputs.push(Output {
                profile: profile.to_path_buf(),
                name: named.strip_prefix(root)?.to_path_buf(),
                files: paths,
                fingerprint,
                bytes,
                unused_for: now.duration_since(newest).unwrap_or_default(),
                superseded: false,
            });
        }
        Ok(())
    }

    /// `path` when it is a real directory, recording a symbolic link in its place.
    fn directory(&mut self, path: PathBuf) -> Result<Option<PathBuf>> {
        let Some(metadata) = present(fs::symlink_metadata(&path))? else {
            return Ok(None);
        };
        if metadata.is_symlink() {
            self.links.push(path);
            return Ok(None);
        }
        Ok(metadata.is_dir().then_some(path))
    }

    pub(super) fn bytes(&self) -> u64 {
        self.outputs.iter().map(|output| output.bytes).sum()
    }

    /// Superseded builds unused for at least `threshold`, grouped by profile directory.
    pub(super) fn unused(&self, threshold: Duration) -> Vec<&Output> {
        let mut unused: Vec<_> = self
            .outputs
            .iter()
            .filter(|output| output.superseded && output.unused_for >= threshold)
            .collect();
        unused
            .sort_by(|left, right| (&left.profile, &left.name).cmp(&(&right.profile, &right.name)));
        unused
    }
}

/// The lock Cargo holds on `profile` while it builds there, or `None` while a build holds
/// it. Held, it makes a build that starts meanwhile wait until it is dropped.
pub(super) fn lock(profile: &Path) -> Result<Option<File>> {
    let path = profile.join(BUILD_LOCK);
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    match lock.try_lock() {
        Ok(()) => Ok(Some(lock)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error).context(format!("lock {}", path.display())),
    }
}

/// Removes an output's files and then its fingerprint directory, unless they are gone or one
/// was modified since the scan. Its files are checked again and none is followed: a path
/// that is no longer a regular file is refused. Cargo rebuilds an output whose files or
/// fingerprint are missing.
pub(super) fn remove(output: &Output, threshold: Duration) -> Result<Outcome> {
    let mut found = Vec::new();
    let mut newest = None;
    for path in &output.files {
        let Some(metadata) = present(fs::symlink_metadata(path))? else {
            continue;
        };
        ensure!(
            metadata.is_file(),
            "{} is no longer a regular file",
            path.display()
        );
        newest = newest.max(Some(metadata.modified()?));
        found.push(path);
    }
    let Some(newest) = newest else {
        return Ok(Outcome::Gone);
    };
    let unused_for = SystemTime::now().duration_since(newest).unwrap_or_default();
    if unused_for < threshold {
        return Ok(Outcome::Used);
    }
    for path in found {
        if let Err(error) = fs::remove_file(path)
            && error.kind() != io::ErrorKind::NotFound
        {
            return Ok(failed(output, error));
        }
    }
    // `remove_dir_all` follows no link, including one now in the directory's place.
    if let Err(error) = fs::remove_dir_all(&output.fingerprint)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Ok(failed(output, error));
    }
    Ok(Outcome::Removed)
}

fn failed(output: &Output, error: io::Error) -> Outcome {
    Outcome::Failed {
        left: measure(&output.files, &output.fingerprint)
            .map(|measured| measured.map_or(0, |(bytes, _)| bytes)),
        error,
    }
}

/// The bytes of an output's files without another hard link and of its fingerprint
/// directory, with the time the newest of them was modified, or `None` when its files are
/// gone. Files removed meanwhile are left out.
fn measure(files: &[PathBuf], fingerprint: &Path) -> Result<Option<(u64, SystemTime)>> {
    let mut bytes = 0;
    let mut newest = None;
    for path in files {
        let Some(metadata) = present(fs::symlink_metadata(path))? else {
            continue;
        };
        newest = newest.max(Some(metadata.modified()?));
        if metadata.nlink() == 1 {
            bytes += metadata.len();
        }
    }
    let Some(mut newest) = newest else {
        return Ok(None);
    };
    if let Some(metadata) = present(fs::symlink_metadata(fingerprint))?
        && metadata.is_dir()
    {
        newest = newest.max(metadata.modified()?);
        for entry in read(fingerprint)? {
            let Some(metadata) = present(entry?.metadata())? else {
                continue;
            };
            newest = newest.max(metadata.modified()?);
        }
        bytes += usage(fingerprint, &mut HashSet::new())?;
    }
    Ok(Some((bytes, newest)))
}

fn read(directory: &Path) -> Result<fs::ReadDir> {
    fs::read_dir(directory).with_context(|| format!("read {}", directory.display()))
}

/// The target an output builds, by its crate name, and whether it is the library: one
/// `<crate>-<hash>.d` file names it, beside `lib<crate>-<hash>.rlib` for a library.
fn target_of(files: &[PathBuf]) -> (String, bool) {
    let names: Vec<_> = files
        .iter()
        .filter_map(|path| path.file_name()?.to_str())
        .collect();
    let library = names.iter().any(|name| {
        [".rlib", ".rmeta", ".so"]
            .iter()
            .any(|extension| name.ends_with(extension))
    });
    let (name, listed) = names.iter().find(|name| name.ends_with(".d")).map_or(
        (names.first().copied().unwrap_or_default(), false),
        |name| (*name, true),
    );
    let stem = name.rsplit_once('-').map_or(name, |(stem, _)| stem);
    let stem = if library && !listed {
        stem.strip_prefix("lib").unwrap_or(stem)
    } else {
        stem
    };
    (stem.to_owned(), library)
}

/// The Cargo hash in a file name `<crate>-<hash>`, with an optional extension and, for a
/// library, `lib` before the crate name.
fn hash_of(name: &str) -> Option<&str> {
    let (_, rest) = name.rsplit_once('-')?;
    let hash = rest.split_once('.').map_or(rest, |(hash, _)| hash);
    is_hash(hash).then_some(hash)
}

/// Cargo's hashes are 16 lowercase hexadecimal digits.
fn is_hash(text: &str) -> bool {
    text.len() == 16
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
