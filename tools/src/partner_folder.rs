//! `partner-folder`: the agent partners' client program, installed, and the folders their
//! sessions run in, one for each tool (Cuba Libre #202, ADR 0007). The shared library
//! writes them (`unfolded_agent_players::folder`); the instructions are this checkout's.

#[path = "../../src/partner_folders.rs"]
mod partner_folders;

use anyhow::{Context, Result, bail};
use clap::Args as ClapArgs;
use partner_folders::{CLIENT, Places};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use unfolded_agent_players::folder::{self, Folders};

#[derive(ClapArgs)]
pub struct Args {
    /// The built client program to install. Default: build it.
    #[arg(long, value_name = "PROGRAM")]
    built: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<u8> {
    let data = partner_folders::data().context("cannot find the application data directory")?;
    let places = Places::of(&data);
    let built = match args.built {
        Some(built) => built,
        None => build()?,
    };
    // What the folders hold of no known writing is refused before the program changes.
    Folders {
        claude_code: &places.claude_code,
        codex: &places.codex,
        client: &places.client,
        instructions: &places.instructions(),
    }
    .write()
    .context("cannot write the partner folders")?;
    folder::install(&built, &places.client)
        .context("cannot install the partners' client program")?;
    println!(
        "Claude Code's partner folder: {}",
        places.claude_code.display()
    );
    println!("Codex's partner folder: {}", places.codex.display());
    println!("Client program: {}", places.client.display());
    Ok(0)
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
