//! Detached review inputs. Git is read-only; paths select complete working files,
//! including staged and unstaged changes, rather than selected index hunks.
use anyhow::{Context, Result, ensure};
use clap::Args as ClapArgs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::{CString, OsStr},
    fs,
    os::unix::{
        ffi::OsStrExt,
        fs::{PermissionsExt, symlink},
    },
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(ClapArgs)]
pub struct Args {
    /// Fixed commit/ref; the comparison base is its merge-base with HEAD.
    #[arg(long)]
    base: String,
    /// Fresh directory outside the checkout, beneath an existing parent.
    #[arg(long)]
    output_dir: PathBuf,
    /// Earlier snapshot directory; verify its files before reporting the candidate delta.
    #[arg(long)]
    previous: Option<PathBuf>,
    /// Exact repository-relative file paths; directories and ignored new files are refused.
    #[arg(required = true, num_args = 1..)]
    paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct FileIdentity {
    mode: String,
    sha256: String,
}

#[derive(Clone, PartialEq, Eq)]
struct File {
    mode: String,
    bytes: Vec<u8>,
}

type Tree = BTreeMap<String, File>;
type Inventory = BTreeMap<String, FileIdentity>;

#[derive(Serialize, Deserialize)]
struct Change {
    path: String,
    kind: String,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u8,
    fixed_point: String,
    base: String,
    head: String,
    selected_paths: Vec<String>,
    files: Inventory,
    changes: Vec<Change>,
    diff_command: Vec<String>,
    patch_sha256: String,
    previous_manifest_sha256: Option<String>,
    delta: Option<Vec<Change>>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(args)
        .current_dir(root)
        .output()
        .context("run git")?;
    ensure!(
        output.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8(git(root, args)?)?
        .trim_end_matches('\n')
        .into())
}

fn check_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && Path::new(path)
                .components()
                .all(|part| { matches!(part, Component::Normal(name) if name != ".git") }),
        "expected an exact repository-relative file path: {path:?}"
    );
    ensure!(
        path.split('/').all(|part| !matches!(part, "" | "." | "..")),
        "expected a normalized file path: {path:?}"
    );
    Ok(())
}

fn base_tree(root: &Path, base: &str) -> Result<Tree> {
    let mut tree = Tree::new();
    for row in git(root, &["ls-tree", "-rz", "--full-tree", base])?
        .split(|byte| *byte == 0)
        .filter(|row| !row.is_empty())
    {
        let row = std::str::from_utf8(row).context("snapshot paths must be UTF-8")?;
        let (header, path) = row.split_once('\t').context("invalid git tree entry")?;
        check_path(path)?;
        let fields: Vec<_> = header.split(' ').collect();
        ensure!(
            fields.len() == 3 && fields[1] == "blob",
            "unsupported tree entry: {path:?} ({header})"
        );
        ensure!(
            matches!(fields[0], "100644" | "100755" | "120000"),
            "unsupported mode at {path:?}"
        );
        tree.insert(
            path.into(),
            File {
                mode: fields[0].into(),
                bytes: git(root, &["cat-file", "blob", fields[2]])?,
            },
        );
    }
    Ok(tree)
}

fn working_file(root: &Path, path: &str) -> Result<Option<File>> {
    let mut ancestor = Path::new(path).parent();
    while let Some(parent) = ancestor.filter(|parent| !parent.as_os_str().is_empty()) {
        if let Ok(metadata) = fs::symlink_metadata(root.join(parent)) {
            ensure!(
                metadata.is_dir(),
                "selected path traverses a non-directory: {path:?}"
            );
        }
        ancestor = parent.parent();
    }
    let path = root.join(path);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    if metadata.is_symlink() {
        return Ok(Some(File {
            mode: "120000".into(),
            bytes: fs::read_link(path)?.as_os_str().as_bytes().into(),
        }));
    }
    ensure!(
        metadata.is_file(),
        "select exact files, not directories or special files: {}",
        path.display()
    );
    Ok(Some(File {
        mode: if metadata.permissions().mode() & 0o111 == 0 {
            "100644"
        } else {
            "100755"
        }
        .into(),
        bytes: fs::read(path)?,
    }))
}

fn inventory(tree: &Tree) -> Inventory {
    tree.iter()
        .map(|(path, file)| {
            (
                path.clone(),
                FileIdentity {
                    mode: file.mode.clone(),
                    sha256: digest(&file.bytes),
                },
            )
        })
        .collect()
}

fn changes(before: &Inventory, after: &Inventory) -> Vec<Change> {
    let paths: std::collections::BTreeSet<_> = before.keys().chain(after.keys()).collect();
    paths
        .into_iter()
        .filter_map(|path| {
            let kind = match (before.get(path), after.get(path)) {
                (Some(old), Some(new)) if old == new => return None,
                (None, Some(_)) => "added",
                (Some(_), None) => "deleted",
                _ => "modified",
            };
            Some(Change {
                path: path.clone(),
                kind: kind.into(),
            })
        })
        .collect()
}

fn write_tree(root: &Path, tree: &Tree) -> Result<()> {
    fs::create_dir(root)?;
    for (path, file) in tree {
        // Validate all ancestor conflicts before creating any symlinks.
        let mut parent = Path::new(path).parent();
        while let Some(value) = parent.filter(|parent| !parent.as_os_str().is_empty()) {
            ensure!(
                !tree.contains_key(value.to_str().context("non-UTF-8 ancestor")?),
                "conflicting file and directory: {path:?}"
            );
            parent = value.parent();
        }
        let output = root.join(path);
        fs::create_dir_all(output.parent().context("file parent")?)?;
        if file.mode == "120000" {
            symlink(OsStr::from_bytes(&file.bytes), output)?;
        } else {
            fs::write(&output, &file.bytes)?;
            fs::set_permissions(
                output,
                fs::Permissions::from_mode(if file.mode == "100755" { 0o755 } else { 0o644 }),
            )?;
        }
    }
    Ok(())
}

fn read_tree(root: &Path, directory: &Path, tree: &mut Tree) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            read_tree(root, &entry.path(), tree)?;
        } else {
            let path = entry.path();
            let path = path
                .strip_prefix(root)?
                .to_str()
                .context("snapshot paths must be UTF-8")?;
            check_path(path)?;
            tree.insert(
                path.into(),
                working_file(root, path)?.context("snapshot file disappeared")?,
            );
        }
    }
    Ok(())
}

fn previous_snapshot(path: &Path) -> Result<(Manifest, String)> {
    let bytes = fs::read(path.join("manifest.json"))?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    ensure!(manifest.version == 1, "unsupported snapshot version");
    let root = path.join("candidate");
    ensure!(
        fs::symlink_metadata(&root)?.is_dir(),
        "candidate must be a directory"
    );
    let mut tree = Tree::new();
    read_tree(&root, &root, &mut tree)?;
    ensure!(
        inventory(&tree) == manifest.files,
        "candidate files do not match the manifest"
    );
    ensure!(
        digest(&fs::read(path.join("changes.patch"))?) == manifest.patch_sha256,
        "patch does not match the manifest"
    );
    Ok((manifest, digest(&bytes)))
}

fn publish(source: &Path, destination: &Path) -> Result<()> {
    let source = CString::new(source.as_os_str().as_bytes())?;
    let destination = CString::new(destination.as_os_str().as_bytes())?;
    // Linux renameat2 atomically refuses even an empty existing destination.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("publish fresh snapshot");
    }
    Ok(())
}

pub fn run(args: Args) -> Result<u8> {
    let root = PathBuf::from(git_text(Path::new("."), &["rev-parse", "--show-toplevel"])?)
        .canonicalize()?;
    let output_parent = args
        .output_dir
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .context("output parent must exist")?;
    let output = output_parent.join(
        args.output_dir
            .file_name()
            .context("output directory needs a name")?,
    );
    ensure!(
        !output.starts_with(&root),
        "snapshot must be outside the checkout"
    );
    ensure!(
        fs::symlink_metadata(&output)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "output must be a fresh directory: {}",
        output.display()
    );
    let fixed_point = git_text(
        &root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{}^{{commit}}", args.base),
        ],
    )?;
    let head = git_text(&root, &["rev-parse", "HEAD"])?;
    let base = git_text(&root, &["merge-base", &fixed_point, &head])?;
    ensure!(
        git(&root, &["ls-files", "-u", "-z"])?.is_empty(),
        "resolve index conflicts before snapshotting"
    );
    let before = base_tree(&root, &base)?;
    let listed = git(
        &root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let available: std::collections::BTreeSet<_> = listed.split(|byte| *byte == 0).collect();
    let mut candidate = before.clone();
    let mut selected_paths = args.paths;
    selected_paths.sort();
    selected_paths.dedup();
    for path in &selected_paths {
        check_path(path)?;
        ensure!(
            before.contains_key(path) || available.contains(path.as_bytes()),
            "path is unknown or ignored: {path:?}"
        );
        if let Some(file) = working_file(&root, path)? {
            candidate.insert(path.clone(), file);
        } else {
            candidate.remove(path);
        }
    }
    let files = inventory(&candidate);
    let selected_changes = changes(&inventory(&before), &files);
    ensure!(!selected_changes.is_empty(), "selected candidate is empty");
    let previous = args
        .previous
        .as_deref()
        .map(previous_snapshot)
        .transpose()
        .context("verify previous snapshot")?;
    let delta = previous
        .as_ref()
        .map(|(manifest, _)| changes(&manifest.files, &files));
    let staging = tempfile::Builder::new()
        .prefix(".review-snapshot-")
        .tempdir_in(output_parent)?;
    write_tree(&staging.path().join("base"), &before)?;
    write_tree(&staging.path().join("candidate"), &candidate)?;
    let diff_command = [
        "git",
        "diff",
        "--no-index",
        "--binary",
        "--no-ext-diff",
        "--no-textconv",
        "--no-renames",
        "--",
        "base",
        "candidate",
    ];
    let diff = Command::new(diff_command[0])
        .args(&diff_command[1..])
        .current_dir(staging.path())
        .output()?;
    ensure!(
        matches!(diff.status.code(), Some(0 | 1)),
        "snapshot diff failed: {}",
        String::from_utf8_lossy(&diff.stderr)
    );
    fs::write(staging.path().join("changes.patch"), &diff.stdout)?;
    let manifest = Manifest {
        version: 1,
        fixed_point,
        base,
        head: head.clone(),
        selected_paths,
        files,
        changes: selected_changes,
        diff_command: diff_command.map(str::to_owned).to_vec(),
        patch_sha256: digest(&diff.stdout),
        previous_manifest_sha256: previous.map(|(_, hash)| hash),
        delta,
    };
    // Freeze only after checking that the selected working inputs stayed stable.
    for path in &manifest.selected_paths {
        ensure!(
            working_file(&root, path)?.as_ref() == candidate.get(path),
            "working input changed during capture: {path:?}"
        );
    }
    ensure!(
        git_text(&root, &["rev-parse", "HEAD"])? == head,
        "HEAD changed during capture"
    );
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    fs::write(staging.path().join("manifest.json"), &bytes)?;
    publish(staging.path(), &output)?;
    println!(
        "Snapshot: {}\nManifest SHA256: {}\n{} changed files",
        output.display(),
        digest(&bytes),
        manifest.changes.len()
    );
    if let Some(delta) = &manifest.delta {
        println!("Changes since previous snapshot: {}", delta.len());
        for change in delta {
            println!("{} {:?}", change.kind, change.path);
        }
    }
    Ok(0)
}
