//! `ci-watch`: one bounded watcher that follows the `Check` workflow run of an exact head
//! until it completes, so that head's result is read once, from that head's run.

use crate::{
    github::{API_REPO, commit_sha, json},
    process,
};
use anyhow::{Result, bail, ensure};
use clap::Args as ClapArgs;
use serde::Deserialize;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

#[derive(ClapArgs)]
pub struct Args {
    /// Exact head whose run to follow (full 40-character commit SHA).
    #[arg(long, value_parser = commit_sha)]
    head: String,
    /// Seconds between observations.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=300))]
    interval_seconds: u64,
    /// Seconds to wait for a run of the head to appear.
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u64).range(1..=3600))]
    appear_timeout_seconds: u64,
    /// Seconds to wait in all for the run to complete.
    #[arg(long, default_value_t = 3600, value_parser = clap::value_parser!(u64).range(1..=14400))]
    timeout_seconds: u64,
}

#[derive(Deserialize)]
struct Page {
    total_count: usize,
    workflow_runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    id: u64,
    head_sha: String,
    status: String,
    conclusion: Option<String>,
    event: String,
    run_attempt: u64,
    html_url: String,
}

pub fn run(args: Args) -> Result<u8> {
    let start = Instant::now();
    let appear_by =
        start + Duration::from_secs(args.appear_timeout_seconds.min(args.timeout_seconds));
    let finish_by = start + Duration::from_secs(args.timeout_seconds);
    println!("waiting for the Check run of {}", args.head);
    // The run, attempt and status last printed; a rerun keeps its run and starts a new attempt.
    let mut shown: Option<(u64, u64, String)> = None;
    loop {
        let (newest, count) = newest_run(&args.head)?;
        match newest {
            Some(run) => {
                let state = (run.id, run.run_attempt, run.status.clone());
                if shown.as_ref() != Some(&state) {
                    let of = if count > 1 {
                        format!(", newest of {count}")
                    } else {
                        String::new()
                    };
                    println!(
                        "run {} attempt {} ({}{of}): {}",
                        run.id, run.run_attempt, run.event, run.status
                    );
                    shown = Some(state);
                }
                if run.status == "completed" {
                    let conclusion = run.conclusion.as_deref().unwrap_or("none");
                    println!(
                        "conclusion {conclusion} for head {}: {}",
                        run.head_sha, run.html_url
                    );
                    return Ok(if conclusion == "success" { 0 } else { 1 });
                }
            }
            None => ensure!(
                shown.is_none(),
                "the run of {} is no longer listed",
                args.head
            ),
        }
        let deadline = if shown.is_some() {
            finish_by
        } else {
            appear_by
        };
        let now = Instant::now();
        if now >= deadline {
            match shown {
                None => bail!(
                    "no Check run of {} appeared within {} seconds; CI runs for the head of an \
                     open pull request and for a push to main",
                    args.head,
                    args.appear_timeout_seconds.min(args.timeout_seconds)
                ),
                Some((id, _, status)) => bail!(
                    "run {id} was still {status} after {} seconds; no conclusion is claimed",
                    args.timeout_seconds
                ),
            }
        }
        process::pause((deadline - now).min(Duration::from_secs(args.interval_seconds)))?;
    }
}

/// The newest `Check` run of `head` and how many there are, once GitHub's answer is complete
/// and names only that head.
fn newest_run(head: &str) -> Result<(Option<Run>, usize)> {
    let pages: Vec<Page> = json(&[
        "api",
        "--hostname",
        "github.com",
        &format!("{API_REPO}/actions/workflows/check.yml/runs?head_sha={head}&per_page=100"),
        "--paginate",
        "--slurp",
    ])?;
    let count: usize = pages.iter().map(|page| page.workflow_runs.len()).sum();
    ensure!(
        !pages.is_empty() && pages.iter().all(|page| page.total_count == count),
        "Incomplete or changing workflow-runs response"
    );
    let runs: Vec<Run> = pages
        .into_iter()
        .flat_map(|page| page.workflow_runs)
        .collect();
    let mut ids = HashSet::new();
    for run in &runs {
        ensure!(
            ids.insert(run.id),
            "Duplicate run IDs in workflow-runs response"
        );
        ensure!(
            run.head_sha == head,
            "GitHub listed run {} of head {} as a run of {head}",
            run.id,
            run.head_sha
        );
    }
    Ok((runs.into_iter().max_by_key(|run| run.id), count))
}
