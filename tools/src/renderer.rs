//! Keep the existing renderer regression gate attached to Cargo's pinned source.
use anyhow::{Context, ensure};
use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn manifest(metadata: &Value) -> anyhow::Result<PathBuf> {
    let candidates: Vec<_> = metadata["packages"].as_array().context("Cargo packages")?
        .iter().filter(|package| package["name"] == "wgpu-hal" && package["version"] == "30.0.1")
        .collect();
    ensure!(candidates.len() == 1, "Expected one pinned wgpu-hal 30.0.1 package");
    let package = candidates[0];
    ensure!(package["source"].as_str().is_some_and(|source| source.starts_with("git+https://github.com/joeloverbeck/unfolded-tabletop?rev=")), "The renderer gate requires the shared Git-pinned patch");
    Ok(PathBuf::from(package["manifest_path"].as_str().context("Renderer manifest path")?))
}

pub fn run() -> anyhow::Result<u8> {
    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1", "--features", "tabletop"])
        .output().context("Locate the pinned renderer with Cargo")?;
    ensure!(output.status.success(), "Cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr));
    let path = manifest(&serde_json::from_slice(&output.stdout)?)?;
    let target = std::env::var_os("CARGO_TARGET_DIR").unwrap_or(std::env::current_dir()?.join("target").into_os_string());
    let status = Command::new("cargo")
        .args(["test", "--locked", "--manifest-path"])
        .arg(path)
        .args(["--lib", "--no-default-features", "--features", "vulkan", "vulkan::upload_memory::tests"])
        .env("CARGO_TARGET_DIR", target)
        .status().context("Run pinned renderer regression tests")?;
    Ok(status.code().and_then(|code| u8::try_from(code).ok()).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renderer_gate_refuses_registry_or_ambiguous_packages() {
        let package = serde_json::json!({"name":"wgpu-hal", "version":"30.0.1", "source":"git+https://github.com/joeloverbeck/unfolded-tabletop?rev=immutable#immutable", "manifest_path":"/cache/producer/vendor/wgpu-hal/Cargo.toml"});
        assert_eq!(manifest(&serde_json::json!({"packages":[package.clone()]})).unwrap(), PathBuf::from("/cache/producer/vendor/wgpu-hal/Cargo.toml"));
        assert!(manifest(&serde_json::json!({"packages":[package.clone(),package.clone()]})).is_err());
        let mut registry = package;
        registry["source"] = "registry+https://github.com/rust-lang/crates.io-index".into();
        assert!(manifest(&serde_json::json!({"packages":[registry]})).is_err());
    }
}
