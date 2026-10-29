use anyhow::{Context, Result, ensure};
use std::process::Command;

/// rust-cache includes every installed toolchain in its key. Resolve the repository
/// override before removing the runner's unused, moving stable installation.
pub fn prepare_toolchain() -> Result<u8> {
    let output = Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
        .context("inspect active Rust toolchain")?;
    ensure!(
        output.status.success(),
        "rustup could not inspect the active toolchain"
    );
    let stdout = std::str::from_utf8(&output.stdout)?;
    let selected = stdout
        .split_whitespace()
        .next()
        .context("rustup returned no active toolchain")?;
    ensure!(
        selected.starts_with(|c: char| c.is_ascii_digit()),
        "CI requires a pinned version, got {selected}"
    );
    for args in [
        vec!["default", selected],
        vec!["toolchain", "uninstall", "stable"],
    ] {
        let status = Command::new("rustup")
            .args(&args)
            .status()
            .context("prepare CI toolchain")?;
        ensure!(
            status.success(),
            "rustup {} failed: {status}",
            args.join(" ")
        );
    }
    Ok(0)
}
