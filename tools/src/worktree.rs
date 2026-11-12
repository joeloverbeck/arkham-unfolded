//! `worktree`: checkouts that build into the owner's Cargo target, each with its own
//! `codegen-units` value for the repository's packages, and their removal once merged.
//! Why the value must differ: docs/agents/measured-and-baseline-checks.md, "One target
//! for several checkouts".

use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Subcommand};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs::{self, File},
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command as Process,
};

#[derive(Subcommand)]
pub enum Command {
    /// Create DIR with a checkout at DIR/worktree and the Cargo configuration beside it.
    Add(AddArgs),
    /// Remove checkouts made by `add` once merged, with their branches and what `add` wrote.
    Remove(RemoveArgs),
}

#[derive(Args)]
pub struct AddArgs {
    /// A directory to create, outside the owner's checkout.
    dir: PathBuf,
    /// The revision the checkout starts at.
    #[arg(long)]
    from: String,
    /// Create this branch at `--from`. Without it, the checkout is detached.
    #[arg(long)]
    branch: Option<String>,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// Directories made by `add`.
    #[arg(required = true)]
    dirs: Vec<PathBuf>,
    /// The revision each checkout's commit must already be part of.
    #[arg(long)]
    merged_into: String,
}

const CHECKOUT: &str = "worktree";
const MARKER: &str =
    "# Written by `scripts/tools worktree add`; `scripts/tools worktree remove` deletes it.";
/// Values are taken from below 250: Cargo's default of 256 stays the owner's, and the
/// baseline recipe passes 250.
const CEILING: i64 = 250;

pub fn run(command: Command) -> Result<u8> {
    let repository = Repository::here()?;
    match command {
        Command::Add(args) => add(&repository, args),
        Command::Remove(args) => remove(&repository, args),
    }
}

fn add(repository: &Repository, args: AddArgs) -> Result<u8> {
    let dir = std::path::absolute(&args.dir)?;
    ensure!(
        !dir.starts_with(&repository.owner),
        "{} is inside the owner's checkout; Cargo would read its configuration there",
        dir.display()
    );
    let packages = repository.packages(&args.from)?;
    // Held until the new worktree is registered, so a concurrent `add` sees its value.
    let _lock = repository.lock()?;
    let value = free_value(&repository.values_in_use()?)
        .context("every codegen-units value below 250 is in use by a registered worktree")?;
    if let Some(parent) = dir.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&dir).with_context(|| format!("create {}", dir.display()))?;
    let target = repository.owner.join("target");
    let checkout = dir.join(CHECKOUT);
    let created = fs::create_dir(dir.join(".cargo"))
        .map_err(anyhow::Error::from)
        .and_then(|()| {
            fs::write(config_file(&dir), config(&target, &packages, value)?)?;
            let mut git_args = vec![
                OsStr::new("worktree"),
                OsStr::new("add"),
                OsStr::new("--quiet"),
            ];
            match &args.branch {
                Some(branch) => git_args.extend([OsStr::new("-b"), OsStr::new(branch)]),
                None => git_args.push(OsStr::new("--detach")),
            }
            git_args.extend([checkout.as_os_str(), OsStr::new(&args.from)]);
            git(&repository.cwd, &git_args).map(drop)
        });
    if let Err(error) = created {
        // Undo only what this call made; the directory did not exist before it.
        let _ = fs::remove_file(config_file(&dir));
        let _ = fs::remove_dir(dir.join(".cargo"));
        let _ = fs::remove_dir(&dir);
        return Err(error);
    }
    println!("checkout: {}", checkout.display());
    println!(
        "codegen-units {value} for {}; target {}",
        packages.join(", "),
        target.display()
    );
    Ok(0)
}

fn remove(repository: &Repository, args: RemoveArgs) -> Result<u8> {
    let merged_into = git_text(
        &repository.cwd,
        &[
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new(&format!("{}^{{commit}}", args.merged_into)),
        ],
    )?;
    let worktrees = repository.worktrees()?;
    let mut failed = false;
    for dir in &args.dirs {
        match remove_one(repository, &worktrees, dir, &merged_into) {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("arkham-tools: kept {}: {error:#}", dir.display());
                failed = true;
            }
        }
    }
    Ok(u8::from(failed))
}

fn remove_one(
    repository: &Repository,
    worktrees: &[Worktree],
    dir: &Path,
    merged_into: &str,
) -> Result<String> {
    let dir = fs::canonicalize(dir).context("no such directory")?;
    let written = fs::read_to_string(config_file(&dir)).unwrap_or_default();
    ensure!(
        written.lines().next() == Some(MARKER),
        "`scripts/tools worktree add` did not make it"
    );
    let checkout = fs::canonicalize(dir.join(CHECKOUT)).context("its checkout is gone")?;
    let worktree = worktrees
        .iter()
        .find(|worktree| fs::canonicalize(&worktree.path).ok().as_deref() == Some(&checkout))
        .context("its checkout is not a registered worktree")?;
    let merged = Process::new("git")
        .args(["merge-base", "--is-ancestor", &worktree.head, merged_into])
        .current_dir(&repository.cwd)
        .status()?;
    match merged.code() {
        Some(0) => {}
        Some(1) => bail!("its commit {} is not merged", worktree.head),
        _ => bail!("git merge-base failed for {}", worktree.head),
    }
    // Git refuses a checkout with changes or untracked files here.
    git(
        &repository.cwd,
        &[
            OsStr::new("worktree"),
            OsStr::new("remove"),
            checkout.as_os_str(),
        ],
    )?;
    let mut report = format!("removed {}", checkout.display());
    if let Some(branch) = &worktree.branch {
        // Deleted only while it still names the commit found merged.
        git(
            &repository.cwd,
            &[
                OsStr::new("update-ref"),
                OsStr::new("-d"),
                OsStr::new(branch),
                OsStr::new(&worktree.head),
            ],
        )?;
        report += &format!(" and {}", branch.trim_start_matches("refs/heads/"));
    }
    fs::remove_file(config_file(&dir))?;
    for kept in [dir.join(".cargo"), dir.clone()] {
        if let Err(error) = fs::remove_dir(&kept) {
            ensure!(error.kind() == ErrorKind::DirectoryNotEmpty, error);
            report += &format!("; kept {}, which holds other files", kept.display());
            return Ok(report);
        }
    }
    Ok(report + &format!("; deleted {}", dir.display()))
}

fn config_file(dir: &Path) -> PathBuf {
    dir.join(".cargo/config.toml")
}

fn config(target: &Path, packages: &[String], value: i64) -> Result<String> {
    let mut build = toml::Table::new();
    build.insert(
        "target-dir".into(),
        target
            .to_str()
            .context("the target path is not UTF-8")?
            .into(),
    );
    let mut package = toml::Table::new();
    for name in packages {
        let mut profile = toml::Table::new();
        profile.insert("codegen-units".into(), value.into());
        package.insert(name.clone(), profile.into());
    }
    let mut dev = toml::Table::new();
    dev.insert("package".into(), package.into());
    let mut profile = toml::Table::new();
    profile.insert("dev".into(), dev.into());
    let mut config = toml::Table::new();
    config.insert("build".into(), build.into());
    config.insert("profile".into(), profile.into());
    Ok(format!("{MARKER}\n{}", toml::to_string(&config)?))
}

fn free_value(in_use: &BTreeSet<i64>) -> Option<i64> {
    (1..CEILING).rev().find(|value| !in_use.contains(value))
}

/// Every `codegen-units` value in a configuration table, at any depth.
fn collect_values(table: &toml::Table, values: &mut BTreeSet<i64>) {
    for (key, value) in table {
        match value {
            toml::Value::Integer(units) if key == "codegen-units" => {
                values.insert(*units);
            }
            toml::Value::Table(inner) => collect_values(inner, values),
            _ => {}
        }
    }
}

struct Repository {
    cwd: PathBuf,
    owner: PathBuf,
    common_dir: PathBuf,
}

struct Worktree {
    path: PathBuf,
    head: String,
    branch: Option<String>,
}

impl Repository {
    fn here() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let common_dir = PathBuf::from(git_text(
            &cwd,
            &[
                OsStr::new("rev-parse"),
                OsStr::new("--path-format=absolute"),
                OsStr::new("--git-common-dir"),
            ],
        )?);
        let mut repository = Self {
            cwd,
            owner: PathBuf::new(),
            common_dir,
        };
        repository.owner = repository
            .worktrees()?
            .into_iter()
            .next()
            .context("git lists no worktrees")?
            .path;
        Ok(repository)
    }

    /// Git lists the owner's checkout first.
    fn worktrees(&self) -> Result<Vec<Worktree>> {
        let listing = git(
            &self.cwd,
            &[
                OsStr::new("worktree"),
                OsStr::new("list"),
                OsStr::new("--porcelain"),
                OsStr::new("-z"),
            ],
        )?;
        let listing = String::from_utf8(listing).context("a worktree path is not UTF-8")?;
        let mut worktrees = Vec::new();
        for record in listing.split("\0\0").filter(|record| !record.is_empty()) {
            let mut fields = record.split('\0');
            let path = fields
                .next()
                .and_then(|field| field.strip_prefix("worktree "))
                .context("unexpected `git worktree list` output")?;
            let mut worktree = Worktree {
                path: path.into(),
                head: String::new(),
                branch: None,
            };
            for field in fields {
                if let Some(head) = field.strip_prefix("HEAD ") {
                    worktree.head = head.into();
                } else if let Some(branch) = field.strip_prefix("branch ") {
                    worktree.branch = Some(branch.into());
                }
            }
            worktrees.push(worktree);
        }
        Ok(worktrees)
    }

    /// Values set in any Cargo configuration file that applies to a registered worktree.
    fn values_in_use(&self) -> Result<BTreeSet<i64>> {
        let mut read = BTreeSet::new();
        let mut values = BTreeSet::new();
        for worktree in self.worktrees()? {
            for directory in worktree.path.ancestors() {
                let file = config_file(directory);
                if !read.insert(file.clone()) {
                    continue;
                }
                let text = match fs::read_to_string(&file) {
                    Ok(text) => text,
                    Err(error) if error.kind() == ErrorKind::NotFound => continue,
                    Err(error) => return Err(error).context(format!("read {}", file.display())),
                };
                let table: toml::Table = text
                    .parse()
                    .with_context(|| format!("parse {}", file.display()))?;
                collect_values(&table, &mut values);
            }
        }
        Ok(values)
    }

    /// The workspace's own packages at `revision`.
    fn packages(&self, revision: &str) -> Result<Vec<String>> {
        let manifest = |path: &str| -> Result<toml::Table> {
            let text = git_text(
                &self.cwd,
                &[
                    OsStr::new("show"),
                    OsStr::new(&format!("{revision}:{path}")),
                ],
            )?;
            text.parse()
                .with_context(|| format!("parse {path} at {revision}"))
        };
        let root = manifest("Cargo.toml")?;
        let mut packages = Vec::new();
        let mut push_name = |manifest: &toml::Table| {
            if let Some(name) = manifest
                .get("package")
                .and_then(|package| package.get("name"))
                .and_then(toml::Value::as_str)
            {
                packages.push(name.to_owned());
            }
        };
        push_name(&root);
        let members = root
            .get("workspace")
            .and_then(|workspace| workspace.get("members"))
            .and_then(toml::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        for member in members {
            let member = member
                .as_str()
                .context("a workspace member is not a path")?;
            ensure!(
                !member.contains('*'),
                "workspace member globs are not supported: {member}"
            );
            push_name(&manifest(&format!("{member}/Cargo.toml"))?);
        }
        ensure!(
            !packages.is_empty(),
            "no packages in Cargo.toml at {revision}"
        );
        Ok(packages)
    }

    fn lock(&self) -> Result<File> {
        let path = self.common_dir.join("arkham-tools-worktree.lock");
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        file.lock()
            .with_context(|| format!("lock {}", path.display()))?;
        Ok(file)
    }
}

fn git(root: &Path, args: &[&OsStr]) -> Result<Vec<u8>> {
    let output = Process::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("run git")?;
    ensure!(
        output.status.success(),
        "git {}: {}",
        args.iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" "),
        String::from_utf8_lossy(&output.stderr).trim_end()
    );
    Ok(output.stdout)
}

fn git_text(root: &Path, args: &[&OsStr]) -> Result<String> {
    Ok(String::from_utf8(git(root, args)?)?
        .trim_end_matches('\n')
        .into())
}
