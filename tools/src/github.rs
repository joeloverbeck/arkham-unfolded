//! The `gh` process calls shared by repository tools that read or write GitHub.

use crate::process::{Completion, OwnedChild};
use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use std::{
    fs::{self, File},
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};

/// The repository as `gh --repo` names it.
pub const REPO: &str = "github.com/joeloverbeck/arkham-unfolded";
/// The repository's REST path prefix for `gh api`.
pub const API_REPO: &str = "repos/joeloverbeck/arkham-unfolded";

pub fn json<T: DeserializeOwned>(args: &[&str]) -> Result<T> {
    serde_json::from_slice(&gh(args)?).context("Invalid or incomplete GitHub JSON response")
}

pub fn gh(args: &[&str]) -> Result<Vec<u8>> {
    run(args, None)
}

/// Runs `gh` with `input` as its standard input, for `gh api --input -`.
pub fn gh_with_input(args: &[&str], input: &[u8]) -> Result<Vec<u8>> {
    run(args, Some(input))
}

fn run(args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
    // Files avoid pipe backpressure while the shared process owner enforces a
    // finite wait and cleans up on interruption or failure.
    let stdout = tempfile::NamedTempFile::new()?;
    let stderr = tempfile::NamedTempFile::new()?;
    let stdin = match input {
        Some(bytes) => {
            let mut file = tempfile::NamedTempFile::new()?;
            file.write_all(bytes)?;
            Stdio::from(File::open(file.path())?)
        }
        None => Stdio::null(),
    };
    let mut command = Command::new("gh");
    command
        .args(args)
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(stdin)
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
