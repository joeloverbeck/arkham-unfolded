mod check;
mod ci;
mod desktop;
mod github;
mod handoff;
mod interaction;
mod merge;
mod native;
mod partner;
mod portal;
use arkham_tools::process;
mod profile;
mod publish;
mod reader;
mod renderer;
mod review;
mod seat;
mod tracked;
mod worktree;

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
    /// Show bounded diagnostic excerpts without changing the complete check log.
    CheckSummary(check::summary::Args),
    /// Show a recorded check's CPU use over time windows from its resource samples.
    #[command(
        after_help = "Resource measurements: docs/agents/measured-and-baseline-checks.md#resource-measurements"
    )]
    ResourceSummary(check::resources::summary::Args),
    /// Check public handoff text for listed private path forms without publishing it.
    CheckPublicHandoff(handoff::Check),
    /// Verify exact PR revisions and required checks, then perform one authorized merge.
    MergeReviewed(merge::MergeArgs),
    /// Post or replace public issue and PR text from a file, proved by exact readback.
    Publish {
        #[command(subcommand)]
        command: publish::Command,
    },
    /// Read a bounded page of whole UTF-8 lines, bound to the source hash.
    ReadContext(reader::Args),
    /// Read multiple excerpts under one shared output budget and source identity.
    ReadContextBatch(reader::BatchArgs),
    /// Search literal text within a bounded source scope and output budget.
    SearchContext(reader::SearchArgs),
    /// Capture selected working-tree files and their base in a fresh review snapshot.
    ReviewSnapshot(review::Args),
    /// Show bounded identity, selected-path and delta pages from a verified snapshot.
    ReviewSummary(review::summary::Args),
    /// Verify Git's index against the recorded review snapshot without changing either.
    ReviewVerify(review::VerifyArgs),
    /// Reject generated files in Git's index, including force-added paths.
    CheckTrackedFiles,
    /// Test the Vulkan staging workaround from the exact pinned shared producer.
    CheckRenderer,
    /// Select the repository's pinned Rust override for the CI cache key.
    PrepareCiToolchain,
    /// Build and supervise isolated native verification runs.
    #[command(
        after_help = "Native verification: docs/agents/native-verification.md#developer-native-verification"
    )]
    Native {
        #[command(subcommand)]
        command: native::Command,
    },
    /// Send requests to a held agent seat over its socket and print each response unchanged.
    #[command(after_help = "Agent seat: README.md#held-session")]
    Seat(seat::Args),
    /// Keep a held agent seat's table running while the agent partners it hosts play,
    /// until the table waits on the seat, a revision or a count of applied choices is
    /// reached, or the timeout passes.
    #[command(after_help = seat::WAIT_HELP)]
    SeatWait(seat::WaitArgs),
    /// Play investigators at the running Arkham table as an agent partner: join, wait,
    /// observe, inspect, choose, ready, say and leave.
    #[command(after_help = partner::GUIDE)]
    Partner(partner::Args),
    /// Add or remove checkouts that build into the owner's Cargo target.
    #[command(
        after_help = "Shared target: docs/agents/measured-and-baseline-checks.md#one-target-for-several-checkouts"
    )]
    Worktree {
        #[command(subcommand)]
        command: worktree::Command,
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
        Command::CheckSummary(args) => check::summary::run(args),
        Command::ResourceSummary(args) => check::resources::summary::run(args),
        Command::CheckPublicHandoff(args) => handoff::run(args).map(|_| 0),
        Command::MergeReviewed(args) => merge::run(args),
        Command::Publish { command } => publish::run(command),
        Command::ReadContext(args) => write_context_page(reader::page(args)),
        Command::ReadContextBatch(args) => write_context_page(reader::batch(args)),
        Command::SearchContext(args) => write_context_page(reader::search(args)),
        Command::ReviewSnapshot(args) => review::run(args),
        Command::ReviewSummary(args) => review::summary::run(args),
        Command::ReviewVerify(args) => review::verify(args),
        Command::CheckTrackedFiles => tracked::check().map_err(anyhow::Error::msg),
        Command::CheckRenderer => renderer::run(),
        Command::PrepareCiToolchain => ci::prepare_toolchain().map_err(anyhow::Error::msg),
        Command::Native { command } => native::run(command),
        Command::ProfileNative(args) => profile::capture(args),
        Command::Seat(args) => seat::run(args),
        Command::SeatWait(args) => seat::wait(args),
        Command::Partner(args) => partner::run(args),
        Command::Worktree { command } => worktree::run(command),
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

fn write_context_page(page: Result<String, String>) -> anyhow::Result<u8> {
    let page = page.map_err(anyhow::Error::msg)?;
    std::io::stdout().write_all(page.as_bytes())?;
    Ok(0)
}
