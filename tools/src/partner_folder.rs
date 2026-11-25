//! `partner-folder`: the agent partners' client program, installed, and the folder their
//! sessions run in (Cuba Libre #202, ADR 0007). The shared library writes both
//! (`unfolded_agent_players::folder`); the instructions are this checkout's, with the
//! program's path in place of its name.

use anyhow::{Context, Result, bail};
use clap::Args as ClapArgs;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use unfolded_agent_players::folder::{self, Player};

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
    let (partner, client) = (data.join("partner"), data.join("client").join(CLIENT));
    let built = match args.built {
        Some(built) => built,
        None => build()?,
    };
    folder::install(&built, &client).context("cannot install the partners' client program")?;
    Player {
        folder: &partner,
        client: &client,
        instructions: &INSTRUCTIONS.replace(CLIENT, &client.display().to_string()),
    }
    .write()
    .context("cannot write the partner folder")?;
    println!("Partner folder: {}", partner.display());
    println!("Client program: {}", client.display());
    Ok(0)
}

/// Arkham's data directory, where the game keeps its saves and names the partner folder
/// (`gathering_tabletop::partners::folder`).
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
