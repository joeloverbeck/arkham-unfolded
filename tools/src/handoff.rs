//! A bounded lexical check, not a verdict about disclosure or publication authority.
use anyhow::{Result, ensure};
use clap::Args;
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Args)]
pub struct Check {
    /// UTF-8 public comment or PR body to inspect; never modified or published.
    file: PathBuf,
}

pub fn run(args: Check) -> Result<()> {
    let mut text = String::new();
    File::open(args.file)?
        .take(1_048_577)
        .read_to_string(&mut text)?;
    ensure!(text.len() <= 1_048_576, "Public handoff exceeds 1 MiB");
    let forms = [
        "/home/",
        "/Users/",
        "~/",
        "file://",
        "/tmp/",
        "/run/user/",
        "local/verification/",
    ];
    let matches: Vec<_> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| forms.iter().any(|form| line.contains(form)))
        .map(|(index, _)| format!("line {}", index + 1))
        .collect();
    let lines = matches
        .iter()
        .take(20)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    ensure!(
        matches.is_empty(),
        "Private path forms on {} lines (first 20): {lines}. Keep runnable details in the private handoff; review the public text before publishing.",
        matches.len()
    );
    println!(
        "No listed private path forms found. This lexical check does not establish disclosure safety or publication authority."
    );
    Ok(())
}
