//! The owner's Cargo target, which every checkout `worktree add` makes builds into: free
//! disk on its filesystem, its incremental caches, and `target-prune`, which removes those
//! caches once unused. Each checkout's own `codegen-units` value gives it caches of its own
//! that outlive the checkout: docs/agents/measured-and-baseline-checks.md, "One target for
//! several checkouts".

use crate::process;
use anyhow::{Context, Result, bail, ensure};
use clap::Args as ClapArgs;
use std::{
    collections::HashSet,
    fmt,
    fs::{self, File, TryLockError},
    io,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const GIB: u64 = 1 << 30;
/// Cargo keeps a profile's incremental caches in `target/<profile>/incremental`, or in
/// `target/<build target>/<profile>/incremental`, one directory per crate build.
const INCREMENTAL: &str = "incremental";

#[derive(ClapArgs)]
pub struct Args {
    /// List the caches unused for at least this long, in hours or days such as 12h or 7d.
    #[arg(long, default_value = "7d", value_parser = age)]
    older_than: Age,
    /// Remove the listed caches. Without it, nothing is removed.
    #[arg(long)]
    delete: bool,
}

/// A whole number of hours, from one hour to ten years.
#[derive(Clone, Copy)]
struct Age {
    hours: u64,
}

impl Age {
    fn duration(self) -> Duration {
        Duration::from_secs(self.hours * 3600)
    }
}

impl fmt::Display for Age {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        if self.hours.is_multiple_of(24) {
            write!(formatter, "{}d", self.hours / 24)
        } else {
            write!(formatter, "{}h", self.hours)
        }
    }
}

/// At least an hour, so a build in progress keeps the cache it is writing.
fn age(value: &str) -> Result<Age, String> {
    let invalid = || "expected hours or days from 1h to 3650d, such as 12h or 7d".to_owned();
    let (number, unit) = match (value.strip_suffix('h'), value.strip_suffix('d')) {
        (Some(number), _) => (number, 1),
        (_, Some(number)) => (number, 24),
        _ => return Err(invalid()),
    };
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    let hours = number
        .parse::<u64>()
        .ok()
        .and_then(|count| count.checked_mul(unit))
        .filter(|hours| (1..=3650 * 24).contains(hours))
        .ok_or_else(invalid)?;
    Ok(Age { hours })
}

/// One crate build's cache: a directory directly inside an incremental directory.
struct Cache {
    path: PathBuf,
    /// Relative to the target.
    name: PathBuf,
    bytes: u64,
    unused_for: Duration,
}

/// The target's incremental directories and their caches. A symbolic link where an
/// incremental directory or a cache would be is listed in `links`, never followed or
/// removed.
struct Caches {
    directories: Vec<(PathBuf, Vec<Cache>)>,
    links: Vec<PathBuf>,
}

impl Caches {
    fn scan(target: &Path) -> Result<Self> {
        let mut caches = Self {
            directories: Vec::new(),
            links: Vec::new(),
        };
        let Some(root) = present(fs::canonicalize(target))
            .with_context(|| format!("resolve {}", target.display()))?
        else {
            return Ok(caches);
        };
        let now = SystemTime::now();
        let mut seen = HashSet::new();
        for child in subdirectories(&root)? {
            for parent in std::iter::once(child.clone()).chain(subdirectories(&child)?) {
                let directory = parent.join(INCREMENTAL);
                let Some(metadata) = present(fs::symlink_metadata(&directory))? else {
                    continue;
                };
                if metadata.is_symlink() {
                    caches.links.push(directory);
                    continue;
                }
                if !metadata.is_dir() {
                    continue;
                }
                let mut found = Vec::new();
                for entry in fs::read_dir(&directory)
                    .with_context(|| format!("read {}", directory.display()))?
                {
                    let entry = entry?;
                    let path = entry.path();
                    let kind = entry.file_type()?;
                    if kind.is_symlink() {
                        caches.links.push(path);
                        continue;
                    }
                    if !kind.is_dir() {
                        continue;
                    }
                    // A build in progress can remove a cache while it is read.
                    let Some(metadata) = present(fs::symlink_metadata(&path))? else {
                        continue;
                    };
                    found.push(Cache {
                        name: path.strip_prefix(&root)?.to_path_buf(),
                        bytes: usage(&path, &mut seen)?,
                        unused_for: now.duration_since(metadata.modified()?).unwrap_or_default(),
                        path,
                    });
                }
                found.sort_by(|left, right| left.name.cmp(&right.name));
                caches
                    .directories
                    .push((directory.strip_prefix(&root)?.to_path_buf(), found));
            }
        }
        Ok(caches)
    }

    fn bytes(&self) -> u64 {
        self.caches().map(|cache| cache.bytes).sum()
    }

    /// Each incremental directory and the bytes in it, such as `debug/incremental 3.0 MiB`.
    fn summary(&self) -> String {
        if self.directories.is_empty() {
            return "none".into();
        }
        self.directories
            .iter()
            .map(|(name, caches)| {
                let bytes = caches.iter().map(|cache| cache.bytes).sum();
                format!("{} {}", name.display(), size(bytes))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn caches(&self) -> impl Iterator<Item = &Cache> {
        self.directories.iter().flat_map(|(_, caches)| caches)
    }
}

/// Prints free disk on the target's filesystem and the size of its incremental caches, and
/// refuses when less than `floor_gib` is free.
pub fn require_free(target: &Path, floor_gib: u64) -> Result<()> {
    let free = free_bytes(target)?;
    let caches = Caches::scan(target)?;
    println!(
        "free disk {} for target {}; incremental caches: {}",
        size(free),
        target.display(),
        caches.summary()
    );
    ensure!(
        free >= floor_gib.saturating_mul(GIB),
        "free disk {} is below the {floor_gib} GiB floor. The incremental caches in {} hold {}: \
         the owner's `scripts/tools target-prune` lists those unused for a week, \
         `--older-than 12h` reaches newer ones, and `--delete` removes what it lists. \
         `--min-free-gib` sets another floor",
        size(free),
        target.display(),
        size(caches.bytes())
    );
    Ok(())
}

pub fn run(args: Args) -> Result<u8> {
    let target = crate::worktree::owner_target()?;
    let caches = Caches::scan(&target)?;
    println!(
        "target {}: incremental caches {}; free disk {}",
        target.display(),
        size(caches.bytes()),
        size(free_bytes(&target)?)
    );
    let threshold = args.older_than.duration();
    let unused: Vec<_> = caches
        .caches()
        .filter(|cache| cache.unused_for >= threshold)
        .collect();
    for cache in &unused {
        println!(
            "{:>10}  unused {:>4}  {}",
            size(cache.bytes),
            since(cache.unused_for),
            cache.name.display()
        );
    }
    let mut failed = false;
    for link in &caches.links {
        eprintln!(
            "arkham-tools: refused {}: a symbolic link where an incremental cache would be",
            link.display()
        );
        failed = true;
    }
    let listed: u64 = unused.iter().map(|cache| cache.bytes).sum();
    if !args.delete {
        println!(
            "{} caches, {}, unused for {} or more; nothing removed: run again with --delete to \
             remove them",
            unused.len(),
            size(listed),
            args.older_than
        );
        return Ok(u8::from(failed));
    }
    // Bytes freed include those of a cache only partly removed.
    let (mut count, mut freed) = (0, 0);
    for cache in unused {
        if process::interrupted() {
            bail!(
                "interrupted after removing {count} caches, {}; the rest are kept",
                size(freed)
            );
        }
        let name = cache.name.display();
        match remove(cache, threshold) {
            Ok(Outcome::Removed) => {
                println!("removed {name}");
                count += 1;
                freed += cache.bytes;
            }
            Ok(Outcome::Gone) => println!("already gone {name}"),
            Ok(Outcome::Used) => println!("kept {name}: used since it was listed"),
            Ok(Outcome::Locked) => println!("kept {name}: a rustc session holds its lock"),
            Ok(Outcome::Failed { left, error }) => {
                eprintln!(
                    "arkham-tools: removal of {name} failed, {} of {} left: {error}",
                    size(left),
                    size(cache.bytes)
                );
                freed += cache.bytes.saturating_sub(left);
                failed = true;
            }
            Err(error) => {
                eprintln!("arkham-tools: kept {name}: {error:#}");
                failed = true;
            }
        }
    }
    println!(
        "removed {count} caches; freed {}; free disk now {}",
        size(freed),
        size(free_bytes(&target)?)
    );
    Ok(u8::from(failed))
}

enum Outcome {
    Removed,
    /// Removed by another process after the scan.
    Gone,
    /// Modified since the scan.
    Used,
    /// A rustc session holds a lock file in it.
    Locked,
    /// Removal stopped at `error`, leaving `left` bytes.
    Failed {
        left: u64,
        error: io::Error,
    },
}

/// Removes a cache unless it is gone, was modified since the scan, or holds a rustc session
/// lock file that another process holds, which rustc's own clean-up also respects. The locks
/// it takes stay held until the removal ends, so a rustc starting on the crate meanwhile
/// cannot copy from the session being removed. Only the cache's own path is checked again:
/// a path that has become a symbolic link is refused, and `remove_dir_all` follows none
/// inside it.
fn remove(cache: &Cache, threshold: Duration) -> Result<Outcome> {
    let Some(metadata) = present(fs::symlink_metadata(&cache.path))? else {
        return Ok(Outcome::Gone);
    };
    ensure!(metadata.is_dir(), "it is no longer a directory");
    let unused_for = SystemTime::now()
        .duration_since(metadata.modified()?)
        .unwrap_or_default();
    if unused_for < threshold {
        return Ok(Outcome::Used);
    }
    let Some(entries) = present(fs::read_dir(&cache.path))? else {
        return Ok(Outcome::Gone);
    };
    let mut locks = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("s-") && name.ends_with(".lock") && entry.file_type()?.is_file()) {
            continue;
        }
        let Some(lock) = present(File::open(entry.path()))? else {
            continue;
        };
        match lock.try_lock() {
            Ok(()) => locks.push(lock),
            Err(TryLockError::WouldBlock) => return Ok(Outcome::Locked),
            Err(TryLockError::Error(error)) => {
                return Err(error).context(format!("lock {}", entry.path().display()));
            }
        }
    }
    let removal = fs::remove_dir_all(&cache.path);
    drop(locks);
    Ok(match removal {
        Ok(()) => Outcome::Removed,
        Err(error) => match present(fs::symlink_metadata(&cache.path))? {
            // Whatever stopped it, nothing of the cache is left.
            None => Outcome::Removed,
            Some(_) => Outcome::Failed {
                left: usage(&cache.path, &mut HashSet::new())?,
                error,
            },
        },
    })
}

/// Bytes an unprivileged process can still write on the filesystem that holds `path`, or
/// that will hold it once created.
fn free_bytes(path: &Path) -> Result<u64> {
    let existing = path
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .with_context(|| format!("none of {} exists", path.display()))?;
    let stats = rustix::fs::statvfs(existing)
        .with_context(|| format!("read free disk for {}", existing.display()))?;
    Ok(stats.f_bavail.saturating_mul(stats.f_frsize))
}

/// Real directories directly in `dir`; symbolic links are not followed.
fn subdirectories(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    let Some(entries) = present(fs::read_dir(dir))? else {
        return Ok(found);
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            found.push(entry.path());
        }
    }
    found.sort();
    Ok(found)
}

/// Bytes in the files under `dir`, counting a file with several hard links once and
/// following no symbolic link. Files a build in progress removes meanwhile are left out.
fn usage(dir: &Path, seen: &mut HashSet<(u64, u64)>) -> Result<u64> {
    let mut bytes = 0;
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
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.nlink() == 1 || seen.insert((metadata.dev(), metadata.ino())) {
                bytes += metadata.len();
            }
        }
    }
    Ok(bytes)
}

/// `None` for a path that does not exist.
fn present<T>(result: io::Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// A byte count in binary units with one decimal, such as `3.0 MiB`.
fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// How long ago, in hours below two days and in whole days after.
fn since(duration: Duration) -> String {
    let hours = duration.as_secs() / 3600;
    if hours < 48 {
        format!("{hours}h")
    } else {
        format!("{}d", hours / 24)
    }
}
