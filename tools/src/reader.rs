use clap::Args as ClapArgs;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[derive(ClapArgs)]
pub struct Args {
    /// UTF-8 file to read; never modified.
    path: PathBuf,
    #[arg(long, default_value_t = 1)]
    start: usize,
    /// Inclusive last line (default: EOF).
    #[arg(long)]
    end: Option<usize>,
    /// Complete output limit, including metadata.
    #[arg(long, default_value_t = 6000, value_parser = clap::value_parser!(u32).range(512..=12000))]
    max_bytes: u32,
    /// Expected source hash from the preceding page.
    #[arg(long)]
    sha256: Option<String>,
}

pub fn page(args: Args) -> Result<String, String> {
    let data = fs::read(&args.path).map_err(|e| format!("{}: {e}", args.path.display()))?;
    let digest = format!("{:x}", Sha256::digest(&data));
    if args.sha256.as_ref().is_some_and(|hash| *hash != digest) {
        return Err("source changed; restart the read from the intended revision".into());
    }
    let text = std::str::from_utf8(&data).map_err(|e| format!("invalid UTF-8: {e}"))?;
    // Like rg -n: LF-delimited, preserving CRLF and a missing final newline.
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    let mut output = format!("SHA256 {digest}\n");
    if lines.is_empty() && args.start == 1 && args.end.is_none() {
        output.push_str("EMPTY FILE\nEND\n");
        return Ok(output);
    }
    let end = args.end.unwrap_or(lines.len());
    if args.start == 0 || args.start > end || end > lines.len() {
        return Err(format!(
            "select an inclusive range within 1..{}",
            lines.len()
        ));
    }
    output.push_str(&format!(
        "SELECTED {}..{end} of {} lines\n",
        args.start,
        lines.len()
    ));
    let continuation = |next| {
        format!(
            "\nNEXT --start {next} --end {end} --max-bytes {} --sha256 {digest}\n",
            args.max_bytes
        )
    };
    let reserved = continuation(end + 1).len();
    let mut next = args.start;
    for number in args.start..=end {
        let row = format!("{number:6} | {}", lines[number - 1]);
        if output.len() + row.len() + reserved > args.max_bytes as usize {
            break;
        }
        output.push_str(&row);
        next = number + 1;
    }
    if next == args.start {
        return Err(format!(
            "line {} cannot fit this page; raise --max-bytes (up to 12000) or inspect that line separately; no source text was emitted",
            args.start
        ));
    }
    if next <= end {
        output.push_str(&continuation(next));
    } else if lines[end - 1].ends_with('\n') {
        output.push_str("\nEND\n");
    } else {
        output.push_str("\nEND (source has no final newline)\n");
    }
    Ok(output)
}
