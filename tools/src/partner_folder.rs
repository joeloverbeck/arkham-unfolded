//! `partner-folder`: the agent partners' client program, installed, and the folders their
//! sessions run in, one for each tool (Cuba Libre #202, ADR 0007). The shared library
//! writes them (`unfolded_agent_players::folder`); the instructions are this checkout's.

use anyhow::{Context, Result, bail};
use clap::Args as ClapArgs;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use unfolded_agent_players::folder::{self, CODEX_WAIT, Folders};

const INSTRUCTIONS: &str = include_str!("../../docs/agents/partner-instructions.md");
const CLIENT: &str = "arkham-partner";

#[derive(ClapArgs)]
pub struct Args {
    /// The built client program to install. Default: build it.
    #[arg(long, value_name = "PROGRAM")]
    built: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<u8> {
    let data = data()?;
    let client = data.join("client").join(CLIENT);
    let built = match args.built {
        Some(built) => built,
        None => build()?,
    };
    let (claude_code, codex) = (data.join("partner/claude-code"), data.join("partner/codex"));
    // What the folders hold of no known writing is refused before the program changes.
    Folders {
        claude_code: &claude_code,
        codex: &codex,
        client: &client,
        instructions: &instructions(&client),
    }
    .write()
    .context("cannot write the partner folders")?;
    folder::install(&built, &client).context("cannot install the partners' client program")?;
    println!("Claude Code's partner folder: {}", claude_code.display());
    println!("Codex's partner folder: {}", codex.display());
    println!("Client program: {}", client.display());
    Ok(0)
}

/// The partner instructions with the program's path in place of its name, and the shared
/// library's words for how Codex waits. The game compares the folders with the same
/// (`gathering_tabletop::partners::instructions`).
fn instructions(client: &Path) -> String {
    INSTRUCTIONS
        .replace(CLIENT, &client.display().to_string())
        .replace("{CODEX_WAIT}", CODEX_WAIT)
}

/// Arkham's data directory, by the same rule as the game finds the partner folders
/// (`gathering_tabletop::partners::data`).
fn data() -> Result<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .filter(|path| path.is_absolute())
        .map(|data| data.join("arkham-unfolded"))
        .context("cannot find the application data directory")
}

/// Build the client program as `scripts/tools` builds this one.
fn build() -> Result<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("the tools have no checkout")?;
    let target = root.join("target/agent-tools");
    let status = Command::new("cargo")
        .args([
            "build",
            "--quiet",
            "--locked",
            "-p",
            "arkham-tools",
            "--bin",
        ])
        .arg(CLIENT)
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .args([
            "--config",
            r#"build.build-dir="{workspace-root}/target/agent-tools""#,
        ])
        .status()
        .context("cannot run cargo")?;
    if !status.success() {
        bail!("the client program did not build");
    }
    Ok(target.join("debug").join(CLIENT))
}
