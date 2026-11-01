mod check;
mod ci;
mod desktop;
mod interaction;
mod native;
mod portal;
mod process;
mod profile;
mod reader;
mod tracked;

use clap::{Parser, Subcommand};
use std::{io::Write, process::ExitCode};

#[derive(Parser)]
#[command(about = "Local Arkham repository and native verification tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Keep complete check output and an atomic completion record across session loss.
    RecordCheck(check::CheckArgs),
    /// Read a bounded page of whole UTF-8 lines, bound to the source hash.
    ReadContext(reader::Args),
    /// Read multiple excerpts under one shared output budget and source identity.
    ReadContextBatch(reader::BatchArgs),
    /// Reject generated files in Git's index, including force-added paths.
    CheckTrackedFiles,
    /// Select the repository's pinned Rust override for the CI cache key.
    PrepareCiToolchain,
    /// Build and supervise isolated native verification runs.
    #[command(
        after_help = "Procedure for all experiments: docs/experiments/jacqueline-curse.md#developer-native-verification"
    )]
    Native {
        #[command(subcommand)]
        command: native::Command,
    },
    /// Owner-run bounded perf capture of an already running, exact executable.
    ProfileNative(profile::Args),
}

fn main() -> ExitCode {
    let command = Cli::parse().command;
    let failure_code = if matches!(
        command,
        Command::Native { .. } | Command::ProfileNative(_) | Command::RecordCheck(_)
    ) {
        1
    } else {
        2
    };
    if let Err(error) = process::install_interrupt_handler() {
        eprintln!("arkham-tools: {error}");
        return ExitCode::from(1);
    }
    let result = match command {
        Command::RecordCheck(args) => check::run(args),
        Command::ReadContext(args) => reader::page(args)
            .and_then(|page| {
                std::io::stdout()
                    .write_all(page.as_bytes())
                    .map_err(|e| e.to_string())
            })
            .map(|()| 0)
            .map_err(anyhow::Error::msg),
        Command::ReadContextBatch(args) => reader::batch(args)
            .and_then(|page| {
                std::io::stdout()
                    .write_all(page.as_bytes())
                    .map_err(|e| e.to_string())
            })
            .map(|()| 0)
            .map_err(anyhow::Error::msg),
        Command::CheckTrackedFiles => tracked::check().map_err(anyhow::Error::msg),
        Command::PrepareCiToolchain => ci::prepare_toolchain().map_err(anyhow::Error::msg),
        Command::Native { command } => native::run(command),
        Command::ProfileNative(args) => profile::capture(args),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("arkham-tools: {error:#}");
            ExitCode::from(if process::interrupted() {
                130
            } else {
                failure_code
            })
        }
    }
}
