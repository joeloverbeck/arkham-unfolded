//! Bounded navigation over a verified snapshot, without its context-file inventory.
use super::{Manifest, VerifyArgs, previous_snapshot};
use anyhow::{Result, ensure};
use clap::Args as ClapArgs;
use std::io::Write;

#[derive(ClapArgs)]
pub struct Args {
    #[command(flatten)]
    snapshot: VerifyArgs,
    /// First summary row; use the NEXT arguments to continue the same snapshot.
    #[arg(long, default_value_t = 1)]
    start: usize,
    /// Complete stdout limit in UTF-8 bytes, including identities and continuation.
    #[arg(long, default_value_t = 6000, value_parser = clap::value_parser!(u32).range(512..=12000))]
    max_bytes: u32,
    /// Show only changes since --previous, while still verifying the complete snapshot.
    #[arg(long)]
    delta_only: bool,
}

pub fn run(args: Args) -> Result<u8> {
    let (manifest, hash) = previous_snapshot(&args.snapshot.snapshot)?;
    ensure!(
        hash == args.snapshot.sha256,
        "snapshot manifest does not match the recorded SHA256"
    );
    ensure!(
        !args.delta_only || manifest.delta.is_some(),
        "--delta-only requires a snapshot captured with --previous"
    );
    let rows = rows(&manifest, args.delta_only);
    ensure!(
        (1..=rows.len()).contains(&args.start),
        "start must select a summary row within 1..={}",
        rows.len()
    );
    let mut output = format!(
        "Manifest SHA256: {hash}\nSummary rows {}..{}\n",
        args.start,
        rows.len()
    );
    let next = |start| {
        format!(
            "\nNEXT --start {start} --max-bytes {} --sha256 {hash}{}\n",
            args.max_bytes,
            if args.delta_only { " --delta-only" } else { "" }
        )
    };
    let reserve = next(rows.len() + 1).len();
    let mut end = args.start;
    for (index, row) in rows.iter().enumerate().skip(args.start - 1) {
        let line = format!("{:6} | {row}\n", index + 1);
        if output.len() + line.len() + reserve > args.max_bytes as usize {
            break;
        }
        output.push_str(&line);
        end = index + 2;
    }
    ensure!(
        end > args.start,
        "summary row {} cannot fit; raise --max-bytes (up to 12000); no summary was emitted",
        args.start
    );
    if end <= rows.len() {
        output.push_str(&next(end));
    } else {
        output.push_str("\nEND\n");
    }
    std::io::stdout().write_all(output.as_bytes())?;
    Ok(0)
}

fn rows(manifest: &Manifest, delta_only: bool) -> Vec<String> {
    let mut rows = vec![
        format!("Fixed point: {}", manifest.fixed_point),
        format!("Base: {}", manifest.base),
        format!("Head: {}", manifest.head),
        format!("Patch SHA256: {}", manifest.patch_sha256),
        format!(
            "Previous manifest SHA256: {}",
            manifest
                .previous_manifest_sha256
                .as_deref()
                .unwrap_or("none")
        ),
        format!(
            "Selected paths: {} ({} changed from base)",
            manifest.selected_paths.len(),
            manifest.changes.len()
        ),
    ];
    for path in manifest.selected_paths.iter().filter(|_| !delta_only) {
        let kind = manifest
            .changes
            .iter()
            .find(|change| change.path == *path)
            .map_or("unchanged", |change| change.kind.as_str());
        rows.push(format!("{kind} {path:?}"));
    }
    if !manifest.excluded_paths.is_empty() {
        rows.push(format!(
            "Explicit exclusions: {}",
            manifest.excluded_paths.len()
        ));
        rows.extend(
            manifest
                .excluded_paths
                .iter()
                .map(|path| format!("excluded {path:?}")),
        );
    }
    if let Some(delta) = &manifest.delta {
        rows.push(format!("Changes since previous snapshot: {}", delta.len()));
        rows.extend(
            delta
                .iter()
                .map(|change| format!("{} {:?}", change.kind, change.path)),
        );
    } else {
        rows.push("Changes since previous snapshot: no previous snapshot".into());
    }
    rows
}
