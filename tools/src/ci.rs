use std::process::Command;

/// rust-cache includes every installed toolchain in its key. Resolve the repository
/// override before removing the runner's unused, moving stable installation.
pub fn prepare_toolchain() -> Result<u8, String> {
    let output = Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
        .map_err(|error| format!("inspect active Rust toolchain: {error}"))?;
    if !output.status.success() {
        return Err("rustup could not inspect the active toolchain".into());
    }
    let stdout = std::str::from_utf8(&output.stdout).map_err(|error| error.to_string())?;
    let selected = stdout
        .split_whitespace()
        .next()
        .ok_or("rustup returned no active toolchain")?;
    if !selected.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(format!("CI requires a pinned version, got {selected}"));
    }
    for args in [
        vec!["default", selected],
        vec!["toolchain", "uninstall", "stable"],
    ] {
        let status = Command::new("rustup")
            .args(&args)
            .status()
            .map_err(|error| format!("prepare CI toolchain: {error}"))?;
        if !status.success() {
            return Err(format!("rustup {} failed: {status}", args.join(" ")));
        }
    }
    Ok(0)
}
