//! Record one check invocation so its result survives a lost tool session.

mod resources;
mod source;
pub mod summary;

use crate::process::{Completion, OwnedChild};
use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;
use std::{
    fs::{self, File},
    io::Write,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Args)]
pub struct CheckArgs {
    /// Fresh directory for check.log and run.json; its parent must exist.
    #[arg(long)]
    output_dir: PathBuf,
    /// Capture host pressure/memory and the child's visible cgroup once per second.
    #[arg(long)]
    resources: bool,
    /// Noninteractive check executable and arguments, using the current directory and environment.
    #[arg(last = true, required = true, num_args = 1.., value_name = "COMMAND")]
    command: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Running,
    Finished,
    Interrupted,
    Incomplete,
    Invalidated,
}

#[derive(Serialize)]
struct Record {
    command: Vec<String>,
    cwd: PathBuf,
    started_unix_ms: u128,
    finished_unix_ms: Option<u128>,
    status: Status,
    pid: Option<u32>,
    exit_code: Option<i32>,
    signal: Option<i32>,
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<resources::Report>,
    source: source::Report,
}

fn now() -> Result<u128> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

fn save(directory: &Path, record: &Record) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer_pretty(&mut temporary, record)?;
    writeln!(temporary)?;
    temporary.as_file().sync_all()?;
    temporary.persist(directory.join("run.json"))?;
    Ok(())
}

pub fn run(args: CheckArgs) -> Result<u8> {
    reject_nested_runner(&args.command)?;
    let mut record = Record {
        command: args.command,
        cwd: std::env::current_dir()?,
        started_unix_ms: now()?,
        finished_unix_ms: None,
        status: Status::Running,
        pid: None,
        exit_code: None,
        signal: None,
        error: None,
        resources: None,
        source: source::Report::default(),
    };
    fs::create_dir(&args.output_dir).with_context(|| {
        format!(
            "Create fresh check directory {} (parent must exist)",
            args.output_dir.display()
        )
    })?;
    let directory = fs::canonicalize(&args.output_dir)?;
    let mut resources = args
        .resources
        .then(|| resources::Capture::start(&directory));
    record.resources = resources.as_ref().map(|capture| capture.report.clone());
    save(&directory, &record)?;
    // Opening this once and cloning the handle preserves both streams in one file.
    let result = (|| {
        record.source = source::Report::start(&record.cwd, &directory)?;
        save(&directory, &record)?;
        let code = capture(&directory, &mut record, &mut resources)?;
        record.source.finish(&directory)?;
        if record.source.changed() {
            record.status = Status::Invalidated;
            record.error =
                Some("Source changed during the check; rerun on a stable candidate".into());
            return Ok(if code == 0 { 1 } else { code });
        }
        Ok(code)
    })();
    if let Some(mut capture) = resources {
        capture.finish();
        if let Some(error) = &capture.report.error {
            eprintln!("Resource capture incomplete: {error}");
        }
        record.resources = Some(capture.report);
    }
    record.finished_unix_ms = Some(now()?);
    if let Err(error) = &result {
        record.status = Status::Incomplete;
        record.error = Some(format!("{error:#}"));
    }
    save(&directory, &record).context("Persist check completion")?;
    eprintln!("Check output: {}", directory.join("check.log").display());
    eprintln!("Check result: {}", directory.join("run.json").display());
    result
}

fn reject_nested_runner(command: &[String]) -> Result<()> {
    if let [dev, check, ..] = command
        && let Ok(dev) = fs::canonicalize(dev)
        && dev.ends_with("scripts/dev")
        // scripts/dev changes to its checkout root before executing its argument.
        && let Some(root) = dev.parent().and_then(Path::parent)
        && let Ok(check) = fs::canonicalize(root.join(check))
        && check.ends_with("scripts/check")
        && dev.parent() == check.parent()
    {
        bail!(
            "scripts/check already runs scripts/dev; nesting them waits on the same checkout lock. \
             From the checkout root, use: scripts/tools record-check --output-dir <fresh-directory> -- scripts/check"
        );
    }
    Ok(())
}

fn capture(
    directory: &Path,
    record: &mut Record,
    resources: &mut Option<resources::Capture>,
) -> Result<u8> {
    let log = File::create_new(directory.join("check.log"))?;
    let mut command = Command::new(&record.command[0]);
    command
        .args(&record.command[1..])
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log.try_clone()?);
    let mut child = OwnedChild::spawn(&mut command)
        .with_context(|| format!("Start check executable {:?}", record.command[0]))?;
    record.pid = Some(child.id());
    save(directory, record)?;
    let completion = loop {
        if let Some(resources) = resources {
            resources.sample(Some(child.id()));
        }
        match child.wait(Duration::from_secs(1))? {
            Completion::TimedOut => continue,
            completion => break completion,
        }
    };
    child.cleanup()?;
    log.sync_all().context("Flush complete check output")?;
    match completion {
        Completion::Exited(status) => {
            record.exit_code = status.code();
            record.signal = status.signal();
            if let Some(code) = record.exit_code {
                record.status = Status::Finished;
                u8::try_from(code).context("Check exit code is outside 0..=255")
            } else {
                record.status = Status::Interrupted;
                Ok((128 + record.signal.unwrap_or(1)).min(255) as u8)
            }
        }
        Completion::Interrupted => {
            record.status = Status::Interrupted;
            Ok(130)
        }
        Completion::TimedOut => unreachable!("the wait loop continues while the check runs"),
    }
}
