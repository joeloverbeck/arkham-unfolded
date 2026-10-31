//! Record one check invocation so its result survives a lost tool session.

use crate::process::{Completion, OwnedChild};
use anyhow::{Context, Result};
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
    };
    fs::create_dir(&args.output_dir).with_context(|| {
        format!(
            "Create fresh check directory {} (parent must exist)",
            args.output_dir.display()
        )
    })?;
    let directory = fs::canonicalize(&args.output_dir)?;
    save(&directory, &record)?;
    // Opening this once and cloning the handle preserves both streams in one file.
    let result = capture(&directory, &mut record);
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

fn capture(directory: &Path, record: &mut Record) -> Result<u8> {
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
