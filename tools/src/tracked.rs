use std::process::Command;

fn non_rust_source(path: &[u8]) -> bool {
    let suffixes: &[&[u8]] = &[
        b"py", b"pyw", b"js", b"mjs", b"cjs", b"ts", b"tsx", b"jsx", b"sh", b"bash", b"zsh",
        b"fish", b"ps1", b"rb", b"pl", b"lua", b"c", b"cc", b"cpp", b"h", b"go", b"java", b"swift",
    ];
    let name = path.rsplit(|byte| *byte == b'/').next().unwrap_or_default();
    name.split(|byte| *byte == b'.')
        .skip(1)
        .any(|part| suffixes.contains(&part))
}

pub fn check() -> Result<u8, String> {
    let result = Command::new("git")
        .args(["ls-files", "--cached", "--full-name", "-z", "--", ":/"])
        .output()
        .map_err(|e| format!("Could not inspect tracked files: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "Could not inspect tracked files.\n{}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    // Git paths are bytes. Decoding first would lose identity for non-UTF-8 names.
    let paths: Vec<_> = result
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .collect();
    let mut forbidden: Vec<_> = paths
        .iter()
        .copied()
        .filter(|path| {
            [b".pyc", b".pyo", b".pyd"]
                .iter()
                .any(|suffix| path.ends_with(*suffix))
                || path
                    .split(|byte| *byte == b'/')
                    .any(|part| part == b"__pycache__" || part == b".pytest_cache")
        })
        .collect();
    forbidden.sort_unstable();
    forbidden.dedup();
    let mut non_rust: Vec<_> = paths
        .into_iter()
        .filter(|path| non_rust_source(path))
        .collect();
    non_rust.sort_unstable();
    non_rust.dedup();
    let failed = !forbidden.is_empty() || !non_rust.is_empty();
    for (heading, entries) in [
        (
            "Generated files are tracked; keep these outside Git:",
            forbidden,
        ),
        (
            "Non-Rust executable sources are tracked; apply the mission language contract:",
            non_rust,
        ),
    ] {
        if !entries.is_empty() {
            eprintln!("{heading}");
            for path in entries {
                eprintln!("  \"{}\"", path.escape_ascii());
            }
        }
    }
    Ok(u8::from(failed))
}
