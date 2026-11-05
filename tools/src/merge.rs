//! One authorized protected-main merge; every prerequisite must succeed first.

use crate::process::{Completion, OwnedChild};
use anyhow::{Context, Result, bail, ensure};
use clap::Args;
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::HashSet,
    fs,
    process::{Command, Stdio},
    time::Duration,
};

const REPO: &str = "github.com/joeloverbeck/arkham-unfolded";

#[derive(Args)]
pub struct MergeArgs {
    /// Already reviewed and authorized PR in joeloverbeck/arkham-unfolded.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    pr: u64,
    /// Exact accepted PR head (full 40-character commit SHA).
    #[arg(long, value_parser = commit_sha)]
    head: String,
    /// Exact reviewed main tip (full 40-character commit SHA).
    #[arg(long, value_parser = commit_sha)]
    base: String,
}

fn commit_sha(value: &str) -> Result<String, String> {
    if value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_ascii_lowercase())
    } else {
        Err("expected a full 40-character commit SHA".into())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    number: u64,
    state: String,
    is_draft: bool,
    head_ref_oid: String,
    base_ref_name: String,
    base_ref_oid: String,
    merge_state_status: String,
    merge_commit: Option<Commit>,
}

#[derive(Deserialize)]
struct Commit {
    oid: String,
}

impl Pull {
    fn verify(&self, args: &MergeArgs) -> Result<()> {
        ensure!(
            self.number == args.pr,
            "GitHub returned a different PR number"
        );
        ensure!(
            self.state == "OPEN" && !self.is_draft,
            "PR must be open and ready for review"
        );
        ensure!(self.base_ref_name == "main", "PR must target main");
        ensure!(
            self.head_ref_oid == args.head,
            "PR head differs from the accepted head"
        );
        ensure!(
            self.base_ref_oid == args.base,
            "PR base differs from the reviewed main tip"
        );
        ensure!(
            self.merge_state_status == "CLEAN",
            "PR merge state is {}; expected CLEAN",
            self.merge_state_status
        );
        Ok(())
    }
}

#[derive(Deserialize)]
struct CheckPage {
    total_count: usize,
    check_runs: Vec<CheckRun>,
}

#[derive(Deserialize)]
struct CheckRun {
    id: u64,
    name: String,
    head_sha: String,
    status: String,
    conclusion: Option<String>,
    app: App,
}

#[derive(Deserialize)]
struct App {
    id: u64,
}

pub fn run(args: MergeArgs) -> Result<u8> {
    read_pull(args.pr)?.verify(&args)?;
    let pages: Vec<CheckPage> = json(&[
        "api",
        "--hostname",
        "github.com",
        &format!(
            "repos/joeloverbeck/arkham-unfolded/commits/{}/check-runs?filter=latest&per_page=100",
            args.head
        ),
        "--paginate",
        "--slurp",
    ])?;
    let count: usize = pages.iter().map(|page| page.check_runs.len()).sum();
    ensure!(
        !pages.is_empty() && pages.iter().all(|page| page.total_count == count),
        "Incomplete or changing check-runs response"
    );
    let mut ids = HashSet::new();
    ensure!(
        pages
            .iter()
            .flat_map(|page| &page.check_runs)
            .all(|check| ids.insert(check.id)),
        "Duplicate check IDs in check-runs response"
    );
    let mut checks = pages
        .iter()
        .flat_map(|page| &page.check_runs)
        .filter(|check| check.name == "Rust checks");
    let check = checks
        .next()
        .context("Required Rust checks result is missing")?;
    ensure!(
        checks.next().is_none(),
        "Ambiguous Rust checks results; inspect the current run"
    );
    ensure!(
        check.app.id == 15368,
        "Rust checks must come from GitHub Actions app 15368"
    );
    ensure!(
        check.head_sha == args.head,
        "Rust checks belongs to a different head"
    );
    ensure!(
        check.status == "completed" && check.conclusion.as_deref() == Some("success"),
        "Rust checks has not completed successfully"
    );
    // Re-read after the check query so a moved head/base cannot pass using the
    // first observation. GitHub's strict protection remains the final gate.
    read_pull(args.pr)?.verify(&args)?;
    let merge = gh(&[
        "pr",
        "merge",
        &args.pr.to_string(),
        "--repo",
        REPO,
        "--merge",
        "--match-head-commit",
        &args.head,
    ]);
    // A failed client call can still have reached GitHub. Read back once and
    // never retry the mutation automatically.
    let observed = read_pull(args.pr);
    if let Ok(pull) = &observed {
        eprintln!(
            "After merge attempt: PR #{} {}, head {}, merge {}",
            pull.number,
            pull.state,
            pull.head_ref_oid,
            pull.merge_commit
                .as_ref()
                .map_or("none", |commit| &commit.oid)
        );
    }
    merge.context(
        "Merge command failed; it was not retried. Inspect the PR before any further publication",
    )?;
    let pull = observed
        .context("Merge was attempted but readback failed; inspect the PR before retrying")?;
    ensure!(
        pull.number == args.pr
            && pull.state == "MERGED"
            && pull.head_ref_oid == args.head
            && pull.base_ref_name == "main",
        "Merge readback did not confirm the accepted PR head; inspect GitHub before retrying"
    );
    let commit = pull
        .merge_commit
        .context("Merged PR readback lacks its merge commit")?;
    commit_sha(&commit.oid).map_err(anyhow::Error::msg)?;
    println!("Merged https://{REPO}/pull/{} at {}", args.pr, commit.oid);
    Ok(0)
}

fn read_pull(pr: u64) -> Result<Pull> {
    json(&[
        "pr",
        "view",
        &pr.to_string(),
        "--repo",
        REPO,
        "--json",
        "number,state,isDraft,headRefOid,baseRefName,baseRefOid,mergeStateStatus,mergeCommit",
    ])
}

fn json<T: DeserializeOwned>(args: &[&str]) -> Result<T> {
    serde_json::from_slice(&gh(args)?).context("Invalid or incomplete GitHub JSON response")
}

fn gh(args: &[&str]) -> Result<Vec<u8>> {
    // Files avoid pipe backpressure while the shared process owner enforces a
    // finite wait and cleans up on interruption or failure.
    let stdout = tempfile::NamedTempFile::new()?;
    let stderr = tempfile::NamedTempFile::new()?;
    let mut command = Command::new("gh");
    command
        .args(args)
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null())
        .stdout(stdout.as_file().try_clone()?)
        .stderr(stderr.as_file().try_clone()?);
    let mut child = OwnedChild::spawn(&mut command).context("Start gh")?;
    let completion = child.wait(Duration::from_secs(30))?;
    child.cleanup()?;
    match completion {
        Completion::Exited(status) if status.success() => Ok(fs::read(stdout.path())?),
        Completion::Exited(status) => bail!(
            "gh {} failed ({status}): {}",
            args.join(" "),
            fs::read_to_string(stderr.path())?
        ),
        Completion::TimedOut => bail!("gh {} timed out after 30 seconds", args.join(" ")),
        Completion::Interrupted => bail!("gh {} interrupted", args.join(" ")),
    }
}
