//! One-time, exact-inventory backup/restore/cleanup for issue #140; never a home scan.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const INSTALLED: &[(&str, usize)] = &[
    ("assets", 13),
    ("assets-v2", 14),
    ("tony-assets", 24),
    ("tony-assets-v2", 25),
    ("tony-assets-v3", 28),
    ("tony-assets-v4", 29),
    ("gathering-assets-before-keys-20261106", 143),
    ("gathering-assets-before-spark-20261107", 144),
    ("gathering-assets-before-offerings-20261108", 145),
    ("gathering-assets", 146),
    ("gathering-assets-issue-137-staging", 147),
    ("gathering-assets-chaos-pouch-staging", 148),
];
const CACHED: &[&str] = &[
    "arkham-89-art-menu/gathering-assets-relocated-v1",
    "arkham-55-assets/party-assets-v1",
    "arkham-124-jacqueline-assets",
    "arkham-124-native/gathering-relocated",
    "arkham-73-assets/jacqueline-assets-v2",
    "arkham-73-assets/party-assets-v2",
    "arkham-73-assets/party-relocated-v2",
    "arkham-73-assets/party-partial-v2",
    "arkham-73-assets/tony-assets-v5",
    "arkham-77-assets/jacqueline-assets-v3",
];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Entry {
    bytes: Option<u64>,
    sha256: Option<String>,
}
type Tree = BTreeMap<PathBuf, Entry>;
#[derive(Serialize, Deserialize)]
struct Inventory {
    owner_root: PathBuf,
    collections: BTreeMap<PathBuf, Tree>,
}

fn hash(path: &Path) -> Result<(u64, String)> {
    let mut input = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 65536];
    let mut bytes = 0;
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        digest.update(&buffer[..n]);
    }
    Ok((bytes, format!("{:x}", digest.finalize())))
}
fn no_links(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        ensure!(
            !fs::symlink_metadata(ancestor)?.is_symlink(),
            "Link in {}",
            path.display()
        );
    }
    Ok(())
}
fn tree(root: &Path) -> Result<Tree> {
    no_links(root)?;
    ensure!(root.is_dir(), "Not a directory: {}", root.display());
    fn visit(root: &Path, relative: &Path, out: &mut Tree) -> Result<()> {
        for item in fs::read_dir(root.join(relative))? {
            let item = item?;
            let path = relative.join(item.file_name());
            let kind = item.file_type()?;
            if kind.is_dir() {
                out.insert(
                    path.clone(),
                    Entry {
                        bytes: None,
                        sha256: None,
                    },
                );
                visit(root, &path, out)?;
            } else {
                ensure!(
                    kind.is_file(),
                    "Link or special file: {}",
                    item.path().display()
                );
                let (bytes, sha256) = hash(&item.path())?;
                out.insert(
                    path,
                    Entry {
                        bytes: Some(bytes),
                        sha256: Some(sha256),
                    },
                );
            }
        }
        Ok(())
    }
    let mut out = Tree::new();
    visit(root, Path::new(""), &mut out)?;
    Ok(out)
}
fn selected() -> BTreeSet<PathBuf> {
    INSTALLED
        .iter()
        .map(|(p, _)| Path::new(".local/share/arkham-unfolded").join(p))
        .chain(CACHED.iter().map(|p| Path::new(".cache/agent-tmp").join(p)))
        .collect()
}
fn validate_collection(
    root: &Path,
    actual: &Tree,
    expected_count: Option<usize>,
    partial: bool,
) -> Result<()> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
    let assets = manifest["assets"]
        .as_array()
        .context("Missing manifest assets")?;
    if let Some(count) = expected_count {
        ensure!(
            assets.len() == count,
            "Changed asset count at {}",
            root.display()
        );
    }
    let recipe: serde_json::Value = serde_json::from_str(include_str!(
        "../../../resources/gathering-asset-selection.json"
    ))?;
    let mut known: BTreeSet<(u64, String)> = recipe["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["bytes"].as_u64().unwrap(),
                a["sha256"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    known.insert((
        6488,
        "c30319ea053f82dfde7d150800a4d37552c1e8a73931635b336c8de404034994".into(),
    ));
    let mut expected = BTreeSet::from([PathBuf::from("manifest.json")]);
    for asset in assets {
        let path = PathBuf::from(asset["path"].as_str().context("Missing payload path")?);
        ensure!(
            path.components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "Unsafe manifest path"
        );
        if partial && path == Path::new("tokens/clue.png") {
            ensure!(!actual.contains_key(&path), "Partial fixture changed");
            continue;
        }
        let bytes = asset["bytes"].as_u64().context("Missing length")?;
        let digest = asset["sha256"]
            .as_str()
            .context("Missing digest")?
            .to_owned();
        ensure!(
            known.contains(&(bytes, digest.clone())),
            "Unexpected historical payload: {}",
            path.display()
        );
        ensure!(
            actual.get(&path)
                == Some(&Entry {
                    bytes: Some(bytes),
                    sha256: Some(digest)
                }),
            "Changed payload: {}",
            root.join(&path).display()
        );
        ensure!(expected.insert(path.clone()), "Duplicate manifest path");
        for parent in path
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            expected.insert(parent.to_owned());
        }
    }
    ensure!(
        expected == actual.keys().cloned().collect(),
        "Unexpected or missing contents at {}",
        root.display()
    );
    Ok(())
}
fn inventory(owner_root: PathBuf) -> Result<Inventory> {
    let mut collections = BTreeMap::new();
    for relative in selected() {
        let root = owner_root.join(&relative);
        let contents = tree(&root)?;
        let count = INSTALLED
            .iter()
            .find(|(p, _)| root.file_name().is_some_and(|n| n == *p))
            .map(|(_, n)| *n);
        validate_collection(
            &root,
            &contents,
            count,
            relative.ends_with("party-partial-v2"),
        )?;
        collections.insert(relative, contents);
    }
    let current = owner_root
        .join(".local/share/arkham-unfolded/gathering-assets-chaos-pouch-staging/manifest.json");
    ensure!(
        hash(&current)?.1 == "8ac085b6b04afa57e8f333baddaf160ff341e1f16eb829050f7ccd20aa7b37bc",
        "Current input drifted"
    );
    Ok(Inventory {
        owner_root,
        collections,
    })
}
fn fresh_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    Ok(())
}
fn verify(base: &Path, plan: &Inventory) -> Result<()> {
    for (relative, expected) in &plan.collections {
        ensure!(
            &tree(&base.join(relative))? == expected,
            "Drift at {}",
            base.join(relative).display()
        );
    }
    Ok(())
}
fn copy(base: &Path, target: &Path, plan: &Inventory) -> Result<()> {
    fs::create_dir(target)?;
    for (relative, entries) in &plan.collections {
        let source = base.join(relative);
        let dest = target.join(relative);
        fs::create_dir_all(&dest)?;
        for (path, entry) in entries {
            if entry.bytes.is_none() {
                fs::create_dir_all(dest.join(path))?;
            } else {
                fs::create_dir_all(dest.join(path).parent().unwrap())?;
                let mut output = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(dest.join(path))?;
                std::io::copy(&mut File::open(source.join(path))?, &mut output)?;
                output.sync_all()?;
            }
        }
    }
    verify(target, plan)?;
    verify(base, plan)?;
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    asset_root: Option<PathBuf>,
}
struct SelectionGuard {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    root: PathBuf,
}
impl SelectionGuard {
    fn load(path: PathBuf, default: PathBuf) -> Result<Self> {
        let bytes = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                ensure!(
                    metadata.is_file() && !metadata.is_symlink(),
                    "Normal artwork settings are not a regular file"
                );
                Some(fs::read(&path)?)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let saved = bytes
            .as_ref()
            .map(|bytes| -> Result<Settings> {
                ensure!(bytes.len() <= 65536, "Unexpectedly large settings file");
                Ok(toml::from_str(std::str::from_utf8(bytes)?)?)
            })
            .transpose()?
            .and_then(|s| s.asset_root);
        let root = saved.unwrap_or(default);
        ensure!(
            root.is_absolute(),
            "Normal artwork selection must be absolute"
        );
        Ok(Self { path, bytes, root })
    }
    fn check(&self, active: &Path) -> Result<()> {
        let current = Self::load(self.path.clone(), self.root.clone())?;
        ensure!(
            current.bytes == self.bytes,
            "Artwork settings changed; no further originals removed"
        );
        ensure!(
            self.root == active,
            "Ordinary launch does not select the admitted installation"
        );
        Ok(())
    }
}
fn remove_verified(
    base: &Path,
    plan: &Inventory,
    selection: &SelectionGuard,
    active: &Path,
) -> Result<()> {
    selection.check(active)?;
    verify(base, plan)?;
    for (relative, expected) in &plan.collections {
        let target = base.join(relative);
        ensure!(
            &tree(&target)? == expected,
            "Drift immediately before removal: {}",
            target.display()
        );
        selection.check(active)?;
        fs::remove_dir_all(&target)?;
        println!("Removed {}", target.display());
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let owner_root = PathBuf::from(std::env::var_os("HOME").context("HOME is required")?);
    ensure!(owner_root.is_absolute(), "HOME must be absolute");
    if args.len() == 2 && args[0] == "inventory" {
        let plan = inventory(owner_root)?;
        fresh_json(Path::new(&args[1]), &plan)?;
        println!(
            "Verified {} exact runtime collections",
            plan.collections.len()
        );
        return Ok(());
    }
    ensure!(
        args.len() == 4 || args.len() == 5,
        "Usage: issue140_artwork inventory PLAN | backup PLAN DOCUMENTS_BACKUP RESTORE | remove PLAN DOCUMENTS_BACKUP RESTORE ADMITTED_ROOT"
    );
    let plan_path = Path::new(&args[1]);
    let plan: Inventory = serde_json::from_slice(&fs::read(plan_path)?)?;
    ensure!(
        plan.owner_root == owner_root
            && plan.collections.keys().cloned().collect::<BTreeSet<_>>() == selected(),
        "Inventory does not match exact issue #140 scope"
    );
    let backup = Path::new(&args[2]);
    let restore = Path::new(&args[3]);
    ensure!(
        backup.parent() == Some(owner_root.join("Documents").as_path())
            && backup
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("Arkham-runtime-backup-")),
        "Backup must be a fresh dated Documents directory"
    );
    if args[0] == "backup" && args.len() == 4 {
        verify(&owner_root, &plan)?;
        fs::create_dir(backup)?;
        copy(&owner_root, &backup.join("collections"), &plan)?;
        copy(&backup.join("collections"), restore, &plan)?;
        verify(&owner_root, &plan)?;
        fresh_json(&backup.join("inventory.json"), &plan)?;
        fresh_json(
            &backup.join("restore-proof.json"),
            &serde_json::json!({"inventory_sha256":hash(plan_path)?.1,"restore":restore,"collections":plan.collections.len()}),
        )?;
        println!(
            "Backup and fresh restore match all {} original trees",
            plan.collections.len()
        );
    } else if args[0] == "remove" && args.len() == 5 {
        let active = Path::new(&args[4]);
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| owner_root.join(".local/share"));
        ensure!(
            data == owner_root.join(".local/share"),
            "XDG data location differs from the selected cleanup scope"
        );
        let play = OpenOptions::new()
            .read(true)
            .write(true)
            .open(data.join("arkham-unfolded/artwork/play.lock"))?;
        play.try_lock()
            .context("Close active play before cleanup")?;
        let config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| owner_root.join(".config"));
        ensure!(
            config_dir.is_absolute(),
            "XDG config location must be absolute"
        );
        let selection = SelectionGuard::load(
            config_dir.join("arkham-unfolded/gathering.toml"),
            data.join("arkham-unfolded/gathering-assets"),
        )?;
        selection.check(active)?;
        ensure!(
            active.file_name().is_some_and(|n| n == "collection")
                && active.parent().and_then(Path::parent)
                    == Some(
                        owner_root
                            .join(".local/share/arkham-unfolded/artwork")
                            .as_path()
                    ),
            "Expected admitted managed installation"
        );
        let receipt: serde_json::Value = serde_json::from_slice(&fs::read(
            active.parent().unwrap().join("installation.json"),
        )?)?;
        ensure!(
            receipt["admitted"] == true,
            "Stable installation has not passed native admission"
        );
        validate_collection(active, &tree(active)?, Some(148), false)?;
        let proof: serde_json::Value =
            serde_json::from_slice(&fs::read(backup.join("restore-proof.json"))?)?;
        ensure!(
            proof["inventory_sha256"] == hash(plan_path)?.1
                && proof["restore"] == restore.to_str().context("Invalid restore path")?,
            "Restore proof mismatch"
        );
        ensure!(
            hash(plan_path)? == hash(&backup.join("inventory.json"))?,
            "Backup inventory changed"
        );
        verify(&backup.join("collections"), &plan)?;
        verify(restore, &plan)?;
        remove_verified(&owner_root, &plan, &selection, active)?;
    } else {
        bail!("Unknown operation or argument count");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_refuses_drift_and_keeps_enclosing_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let relative = PathBuf::from("evidence/runtime");
        fs::create_dir_all(base.join(&relative)).unwrap();
        fs::write(base.join("evidence/log"), b"retain").unwrap();
        fs::write(base.join(&relative).join("art"), b"original").unwrap();
        let plan = Inventory {
            owner_root: base.to_owned(),
            collections: BTreeMap::from([(relative.clone(), tree(&base.join(&relative)).unwrap())]),
        };
        let active = base.join("managed/collection");
        let selection = SelectionGuard::load(base.join("settings"), active.clone()).unwrap();
        fs::write(base.join(&relative).join("new"), b"unexpected").unwrap();
        assert!(remove_verified(base, &plan, &selection, &active).is_err());
        assert!(base.join(&relative).join("art").exists());
        fs::remove_file(base.join(&relative).join("new")).unwrap();
        fs::write(&selection.path, b"asset_root = '/different'\n").unwrap();
        assert!(remove_verified(base, &plan, &selection, &active).is_err());
        assert!(base.join(&relative).exists());
        fs::remove_file(&selection.path).unwrap();
        let linked = base.join("linked-settings");
        fs::write(&linked, format!("asset_root = '{}'\n", active.display())).unwrap();
        std::os::unix::fs::symlink(&linked, &selection.path).unwrap();
        assert!(SelectionGuard::load(selection.path.clone(), active.clone()).is_err());
        assert!(remove_verified(base, &plan, &selection, &active).is_err());
        assert!(base.join(&relative).exists());
        fs::remove_file(&selection.path).unwrap();
        assert!(remove_verified(base, &plan, &selection, &base.join(&relative)).is_err());
        assert!(base.join(&relative).exists());
        remove_verified(base, &plan, &selection, &active).unwrap();
        assert!(!base.join(relative).exists());
        assert_eq!(fs::read(base.join("evidence/log")).unwrap(), b"retain");
    }
    #[test]
    fn backup_restore_compares_empty_directories_and_refuses_links() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("source");
        fs::create_dir_all(base.join("runtime/empty")).unwrap();
        fs::write(base.join("runtime/historical"), b"unique").unwrap();
        let plan = Inventory {
            owner_root: base.clone(),
            collections: BTreeMap::from([(
                PathBuf::from("runtime"),
                tree(&base.join("runtime")).unwrap(),
            )]),
        };
        let backup = dir.path().join("backup");
        let restore = dir.path().join("restore");
        copy(&base, &backup, &plan).unwrap();
        copy(&backup, &restore, &plan).unwrap();
        verify(&restore, &plan).unwrap();
        fs::remove_dir(restore.join("runtime/empty")).unwrap();
        assert!(verify(&restore, &plan).is_err());
        std::os::unix::fs::symlink(base.join("runtime/historical"), base.join("runtime/link"))
            .unwrap();
        assert!(tree(&base.join("runtime")).is_err());
    }
}
