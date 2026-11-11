//! Publish public GitHub text exactly as checked, and act further only after an
//! exact readback. The body travels as JSON on standard input, never as a `gh`
//! field argument, so the bytes checked are the bytes GitHub stores.

use crate::{
    github::{self, API_REPO},
    handoff,
};
use anyhow::{Result, bail, ensure};
use clap::{Args, Subcommand};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Command {
    /// Comment on an issue or PR; with --close, then close the issue as completed.
    Comment(CommentArgs),
    /// Replace one existing comment's body, such as the current handoff.
    EditComment(EditArgs),
    /// Replace a pull request's body.
    PrBody(PrArgs),
}

#[derive(Args)]
pub struct CommentArgs {
    /// Issue or pull request number.
    #[arg(long)]
    issue: u64,
    /// UTF-8 file holding the exact comment text.
    #[arg(long)]
    body_file: PathBuf,
    /// Close the issue once the comment reads back exactly; an issue already
    /// closed stays closed. Refused for pull requests.
    #[arg(long)]
    close: bool,
}

#[derive(Args)]
pub struct EditArgs {
    /// Comment ID, the number after `#issuecomment-` in its URL.
    #[arg(long)]
    id: u64,
    /// UTF-8 file holding the exact replacement text.
    #[arg(long)]
    body_file: PathBuf,
}

#[derive(Args)]
pub struct PrArgs {
    /// Pull request number.
    #[arg(long)]
    pr: u64,
    /// UTF-8 file holding the exact replacement body.
    #[arg(long)]
    body_file: PathBuf,
}

#[derive(Deserialize)]
struct Written {
    html_url: String,
    #[serde(default)]
    id: u64,
}

#[derive(Deserialize)]
struct Stored {
    body: Option<String>,
}

#[derive(Deserialize)]
struct Issue {
    state: String,
    #[serde(default)]
    pull_request: Option<Value>,
}

pub fn run(command: Command) -> Result<u8> {
    match command {
        Command::Comment(args) => comment(args),
        Command::EditComment(args) => {
            let text = checked(&args.body_file)?;
            let path = format!("{API_REPO}/issues/comments/{}", args.id);
            let written: Written = write("PATCH", &path, &json!({ "body": text }))?;
            verify(&path, &text)?;
            println!("Updated {} (body matched readback)", written.html_url);
            Ok(0)
        }
        Command::PrBody(args) => {
            let text = checked(&args.body_file)?;
            let path = format!("{API_REPO}/pulls/{}", args.pr);
            let written: Written = write("PATCH", &path, &json!({ "body": text }))?;
            verify(&path, &text)?;
            println!("Updated {} (body matched readback)", written.html_url);
            Ok(0)
        }
    }
}

fn comment(args: CommentArgs) -> Result<u8> {
    let text = checked(&args.body_file)?;
    let issue_path = format!("{API_REPO}/issues/{}", args.issue);
    let before = if args.close {
        let issue: Issue = read(&issue_path)?;
        ensure!(
            issue.pull_request.is_none(),
            "#{} is a pull request; --close closes issues only. Nothing was posted.",
            args.issue
        );
        Some(issue)
    } else {
        None
    };
    let written: Written = write(
        "POST",
        &format!("{issue_path}/comments"),
        &json!({ "body": text }),
    )?;
    verify(&format!("{API_REPO}/issues/comments/{}", written.id), &text)?;
    println!("Commented {} (body matched readback)", written.html_url);
    match before {
        Some(issue) if issue.state == "closed" => {
            println!("#{} was already closed; left closed", args.issue);
        }
        Some(_) => {
            write::<Value>(
                "PATCH",
                &issue_path,
                &json!({ "state": "closed", "state_reason": "completed" }),
            )?;
            let after: Issue = read(&issue_path)?;
            ensure!(
                after.state == "closed",
                "#{} reads as {} after closing; inspect it before retrying",
                args.issue,
                after.state
            );
            println!("Closed #{} as completed", args.issue);
        }
        None => {}
    }
    Ok(0)
}

/// The file's text, refused before any GitHub call when it is empty or fails
/// the public handoff preflight.
fn checked(path: &Path) -> Result<String> {
    let text = handoff::read_text(path)?;
    ensure!(!text.trim().is_empty(), "The body file is empty");
    handoff::check_text(&text)?;
    Ok(text)
}

fn read<T: DeserializeOwned>(path: &str) -> Result<T> {
    github::json(&["api", "--hostname", "github.com", path])
}

fn write<T: DeserializeOwned>(method: &str, path: &str, payload: &Value) -> Result<T> {
    let output = github::gh_with_input(
        &[
            "api",
            "--hostname",
            "github.com",
            "--method",
            method,
            path,
            "--input",
            "-",
        ],
        &serde_json::to_vec(payload)?,
    )?;
    Ok(serde_json::from_slice(&output)?)
}

fn verify(path: &str, sent: &str) -> Result<()> {
    let stored = read::<Stored>(path)?.body.unwrap_or_default();
    if stored == sent {
        return Ok(());
    }
    let line = sent
        .lines()
        .zip(stored.lines())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| sent.lines().count().min(stored.lines().count()))
        + 1;
    bail!(
        "GitHub stored different text from line {line} ({} bytes sent, {} stored); nothing further was changed, so inspect the remote text before retrying",
        sent.len(),
        stored.len()
    )
}
