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

#[derive(Deserialize)]
struct JobsPage {
    total_count: usize,
    jobs: Vec<Job>,
}

#[derive(Deserialize)]
struct Job {
    id: u64,
    run_id: u64,
    run_attempt: u64,
    head_sha: String,
    name: String,
    status: String,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    number: u64,
    name: String,
    status: String,
}

pub fn run(args: Args) -> Result<u8> {
    let start = Instant::now();
    let appear_by =
        start + Duration::from_secs(args.appear_timeout_seconds.min(args.timeout_seconds));
    let finish_by = start + Duration::from_secs(args.timeout_seconds);
    println!("waiting for the Check run of {}", args.head);
    // The run, attempt and status last printed; a rerun keeps its run and starts a new attempt.
    let mut shown: Option<(u64, u64, String)> = None;
    let mut shown_progress = None;
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
                let progress = current_jobs(&run)?;
                let observation = (run.id, run.run_attempt, progress);
                if shown_progress.as_ref() != Some(&observation) {
                    println!(
                        "elapsed {}s; run {} attempt {}: {}",
                        start.elapsed().as_secs(),
                        run.id,
                        run.run_attempt,
                        observation.2
                    );
                    shown_progress = Some(observation);
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

/// Progress is advisory: only the workflow run's conclusion can finish the watch.
/// Bind jobs to an explicit attempt so a rerun cannot mix old and current steps.
fn current_jobs(run: &Run) -> Result<String> {
    let pages: Vec<JobsPage> = json(&[
        "api",
        "--hostname",
        "github.com",
        &format!(
            "{API_REPO}/actions/runs/{}/attempts/{}/jobs?per_page=100",
            run.id, run.run_attempt
        ),
        "--paginate",
        "--slurp",
    ])?;
    let count: usize = pages.iter().map(|page| page.jobs.len()).sum();
    ensure!(
        !pages.is_empty() && pages.iter().all(|page| page.total_count == count),
        "Incomplete or changing workflow-jobs response"
    );
    let mut jobs: Vec<Job> = pages.into_iter().flat_map(|page| page.jobs).collect();
    jobs.sort_by_key(|job| job.id);
    let mut ids = HashSet::new();
    for job in &jobs {
        ensure!(
            ids.insert(job.id),
            "Duplicate job IDs in workflow-jobs response"
        );
        ensure!(
            job.run_id == run.id
                && job.run_attempt == run.run_attempt
                && job.head_sha == run.head_sha,
            "GitHub listed job {} for another head, run or attempt",
            job.id
        );
    }
    if jobs.is_empty() {
        return Ok("waiting for jobs".into());
    }
    let mut active = Vec::new();
    for mut job in jobs.into_iter().filter(|job| job.status != "completed") {
        job.steps.sort_by_key(|step| step.number);
        let steps: Vec<_> = job
            .steps
            .iter()
            .filter(|step| step.status == "in_progress")
            .map(|step| step.name.as_str())
            .collect();
        let phase = if steps.is_empty() {
            job.status
        } else {
            steps.join(", ")
        };
        active.push(format!("{}: {phase}", job.name));
    }
    Ok(if active.is_empty() {
        "jobs completed; waiting for run conclusion".into()
    } else {
        active.join("; ")
    })
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
