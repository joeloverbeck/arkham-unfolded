//! Build outputs of the repository's own packages in the owner's target, in the layout Cargo
//! 1.100 writes. Each build of a target is one directory `<profile>/build/<package>/<hash>`,
//! named with a hash of the build's identity. Its `fingerprint` directory names the unit it
//! builds, such as `bin-arkham-driver` or `test-lib-arkham_tools`, and its `out` directory
//! holds what it built. Cargo never removes the directory of an earlier identity. A new
//! `unfolded-tabletop` revision, or a checkout's own `codegen-units` value, gives every
//! target of the repository a new identity.
//!
//! Cargo calls this layout unstable. A test runs the pinned Cargo to catch a change in it.

use super::{Outcome, present, subdirectories};
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, TryLockError},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

/// Where Cargo keeps a profile's builds, one directory per package.
const BUILDS: &str = "build";
/// Where a build records the unit it builds.
const FINGERPRINT: &str = "fingerprint";
/// Where a build puts what it built.
const OUT: &str = "out";
/// The lock Cargo 1.100 holds on a profile directory while it builds or checks there; a build
/// that starts while another process holds it waits for "file lock on build directory".
const BUILD_LOCK: &str = ".cargo-build-lock";

/// What one build builds. The newest build of each in a profile directory is kept.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Target {
    package: String,
    /// The unit its fingerprint names, such as `bin-arkham-driver`.
    unit: String,
    /// The features the fingerprint records, so that a build without the `tabletop` feature
    /// and one with it are each kept.
    features: String,
    kind: Kind,
}

/// The two kinds of build Cargo makes of one unit, each kept in its newest build.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    /// As `cargo check` and Clippy make: metadata and dependency lists, and no library or
    /// executable.
    Check,
    /// A library or executable.
    Full,
}

/// One build of one of the repository's targets.
pub(super) struct Output {
    /// The profile directory, such as `<target>/debug`.
    pub(super) profile: PathBuf,
    /// Relative to the target, such as `debug/build/arkham-tools/0123456789abcdef`.
    pub(super) name: PathBuf,
    path: PathBuf,
    target: Target,
    /// Bytes its removal frees: those of its files that have no other hard link.
    pub(super) bytes: u64,
    pub(super) unused_for: Duration,
    /// A newer build of the same target is in the same profile directory.
    superseded: bool,
}

/// The repository's build outputs in the target. A symbolic link where a profile's `build`
/// directory, a package's directory or a build would be, or anywhere inside a build, is listed
/// in `links`, never followed or removed.
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
        let mut newest: HashMap<(&Path, &Target), Duration> = HashMap::new();
        for output in &found.outputs {
            let unused_for = newest
                .entry((&output.profile, &output.target))
                .or_insert(output.unused_for);
            *unused_for = (*unused_for).min(output.unused_for);
        }
        let superseded: Vec<bool> = found
            .outputs
            .iter()
            .map(|output| output.unused_for > newest[&(output.profile.as_path(), &output.target)])
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
        let Some(builds) = self.directory(profile.join(BUILDS))? else {
            return Ok(());
        };
        for package in packages {
            let Some(directory) = self.directory(builds.join(package))? else {
                continue;
            };
            let Some(entries) = present(fs::read_dir(&directory))? else {
                continue;
            };
            for entry in entries {
                let entry = entry?;
                if !entry.file_name().to_str().is_some_and(is_hash) {
                    continue;
                }
                let kind = entry.file_type()?;
                if kind.is_symlink() {
                    self.links.push(entry.path());
                } else if kind.is_dir() {
                    self.record(root, profile, package, entry.path(), now)?;
                }
            }
        }
        Ok(())
    }

    /// Records the build in `path` unless it holds a link, names no unit it could supersede,
    /// such as a build script's, records no features, or has built nothing yet.
    fn record(
        &mut self,
        root: &Path,
        profile: &Path,
        package: &str,
        path: PathBuf,
        now: SystemTime,
    ) -> Result<()> {
        // A build in progress elsewhere can remove a build while it is read.
        let Some(contents) = walk(&path)? else {
            return Ok(());
        };
        if !contents.links.is_empty() {
            self.links.extend(contents.links);
            return Ok(());
        }
        let Some((unit, features)) = unit(&path.join(FINGERPRINT))? else {
            return Ok(());
        };
        let Some(kind) = kind_of(&path.join(OUT))? else {
            return Ok(());
        };
        self.outputs.push(Output {
            profile: profile.to_path_buf(),
            name: path.strip_prefix(root)?.to_path_buf(),
            path,
            target: Target {
                package: package.to_owned(),
                unit,
                features,
                kind,
            },
            bytes: contents.bytes,
            unused_for: now.duration_since(contents.newest).unwrap_or_default(),
            superseded: false,
        });
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

/// Removes a build's directory, unless it is gone or something in it was modified since the
/// scan. It is checked again first: a build that is no longer a real directory, or now holds a
/// symbolic link, is refused, and `remove_dir_all` follows no link. Cargo rebuilds a build
/// whose directory is missing.
pub(super) fn remove(output: &Output, threshold: Duration) -> Result<Outcome> {
    let Some(metadata) = present(fs::symlink_metadata(&output.path))? else {
        return Ok(Outcome::Gone);
    };
    ensure!(metadata.is_dir(), "it is no longer a directory");
    let Some(contents) = walk(&output.path)? else {
        return Ok(Outcome::Gone);
    };
    if let Some(link) = contents.links.first() {
        bail!("{} is a symbolic link", link.display());
    }
    let unused_for = SystemTime::now()
        .duration_since(contents.newest)
        .unwrap_or_default();
    if unused_for < threshold {
        return Ok(Outcome::Used);
    }
    let removal = fs::remove_dir_all(&output.path);
    Ok(match removal {
        Ok(()) => Outcome::Removed,
        Err(error) => match present(fs::symlink_metadata(&output.path)) {
            // Whatever stopped it, nothing of the build is left.
            Ok(None) => Outcome::Removed,
            Ok(Some(_)) => Outcome::Failed {
                left: walk(&output.path).map(|left| left.map_or(0, |left| left.bytes)),
                error,
            },
            Err(unchecked) => Outcome::Failed {
                left: Err(unchecked),
                error,
            },
        },
    })
}

/// What a walk through one build's directory found.
struct Contents {
    /// Bytes of its files that have no other hard link.
    bytes: u64,
    /// When the directory or anything in it was last modified.
    newest: SystemTime,
    /// Symbolic links in it, none followed.
    links: Vec<PathBuf>,
}

/// Walks `dir`, or `None` when it is gone. Entries removed meanwhile are left out.
fn walk(dir: &Path) -> Result<Option<Contents>> {
    let Some(metadata) = present(fs::symlink_metadata(dir))? else {
        return Ok(None);
    };
    let mut contents = Contents {
        bytes: 0,
        newest: metadata.modified()?,
        links: Vec::new(),
    };
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Some(entries) = present(fs::read_dir(&dir))? else {
            continue;
        };
        for entry in entries {
            let entry = entry?;
            // Reads the entry itself, not what a link names.
            let Some(metadata) = present(entry.metadata())? else {
                continue;
            };
            contents.newest = contents.newest.max(metadata.modified()?);
            if metadata.is_symlink() {
                contents.links.push(entry.path());
            } else if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.nlink() == 1 {
                contents.bytes += metadata.len();
            }
        }
    }
    contents.links.sort();
    Ok(Some(contents))
}

/// The unit a build's fingerprint directory names by its one `<unit>.json` file, with the
/// features that file records. `None` for a build script's compilation or run, which no
/// build supersedes, or when there is no such file or it holds no features, as while a
/// build writes it.
fn unit(fingerprint: &Path) -> Result<Option<(String, String)>> {
    let Some(entries) = present(fs::read_dir(fingerprint))? else {
        return Ok(None);
    };
    let mut units = HashSet::new();
    for entry in entries {
        let name = entry?.file_name();
        if let Some(unit) = name.to_str().and_then(|name| name.strip_suffix(".json")) {
            units.insert(unit.to_owned());
        }
    }
    if units.len() != 1 {
        return Ok(None);
    }
    let unit = units.into_iter().next().unwrap_or_default();
    if unit.starts_with("build-script-") || unit.starts_with("run-build-script-") {
        return Ok(None);
    }
    let Some(record) = present(fs::read(fingerprint.join(format!("{unit}.json"))))? else {
        return Ok(None);
    };
    let features = serde_json::from_slice::<serde_json::Value>(&record)
        .ok()
        .and_then(|record| Some(record.get("features")?.as_str()?.to_owned()));
    Ok(features.map(|features| (unit, features)))
}

/// The kind of build whose `out` directory this is: a check when it holds only metadata and
/// dependency lists, or `None` when it holds no file.
fn kind_of(out: &Path) -> Result<Option<Kind>> {
    let Some(entries) = present(fs::read_dir(out))? else {
        return Ok(None);
    };
    let mut kind = None;
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let metadata = name
            .to_str()
            .is_some_and(|name| name.ends_with(".rmeta") || name.ends_with(".d"));
        kind = match (kind, metadata) {
            (Some(Kind::Full), _) | (_, false) => Some(Kind::Full),
            _ => Some(Kind::Check),
        };
    }
    Ok(kind)
}

/// Cargo's hashes are 16 lowercase hexadecimal digits.
fn is_hash(text: &str) -> bool {
    text.len() == 16
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
