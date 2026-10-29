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
    /// Read a bounded page of whole UTF-8 lines, bound to the source hash.
    ReadContext(reader::Args),
    /// Reject generated files in Git's index, including force-added paths.
    CheckTrackedFiles,
    /// Select the repository's pinned Rust override for the CI cache key.
    PrepareCiToolchain,
    /// Build and supervise isolated native verification runs.
    Native {
        #[command(subcommand)]
        command: native::Command,
    },
    /// Owner-run bounded perf capture of an already running, exact executable.
    ProfileNative(profile::Args),
}

fn main() -> ExitCode {
    let command = Cli::parse().command;
    let failure_code = if matches!(command, Command::Native { .. } | Command::ProfileNative(_)) {
        1
    } else {
        2
    };
    if let Err(error) = process::install_interrupt_handler() {
        eprintln!("arkham-tools: {error}");
        return ExitCode::from(1);
    }
    let result = match command {
        Command::ReadContext(args) => reader::page(args)
            .and_then(|page| {
                std::io::stdout()
                    .write_all(page.as_bytes())
                    .map_err(|e| e.to_string())
            })
            .map(|()| 0)
            .map_err(anyhow::Error::msg),
        Command::CheckTrackedFiles => tracked::check().map_err(anyhow::Error::msg),
        Command::PrepareCiToolchain => ci::prepare_toolchain(),
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
