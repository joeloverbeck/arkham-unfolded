use clap::Args as ClapArgs;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[derive(ClapArgs)]
#[command(
    after_help = "For NEXT, repeat the same path and query/scope options; replace only the previous continuation arguments with those printed after NEXT."
)]
pub struct Args {
    /// UTF-8 file to search; never modified.
    path: PathBuf,
    /// Nonempty, case-sensitive literal text on one line (not a regular expression).
    #[arg(long, allow_hyphen_values = true)]
    find: String,
    /// Inclusive first line of the search scope.
    #[arg(long, default_value_t = 1)]
    start: usize,
    /// Inclusive last line of the search scope (default: EOF).
    #[arg(long)]
    end: Option<usize>,
    /// Exact ATX heading line, including # prefix; scopes through its child sections.
    #[arg(long)]
    heading: Option<String>,
    /// Lines of context around each match, clipped to the selected scope.
    #[arg(long, default_value_t = 0)]
    context: usize,
    /// Complete output limit in bytes (512..=12000), including metadata.
    #[arg(long, default_value_t = 6000, value_parser = clap::value_parser!(u32).range(512..=12000))]
    max_bytes: u32,
    /// First unread result line from NEXT; repeat the original search options.
    #[arg(long, requires = "sha256")]
    next_line: Option<usize>,
    /// Expected hash of the source, literal query and scope from the preceding page.
    #[arg(long)]
    sha256: Option<String>,
}

pub fn page(args: Args) -> Result<String, String> {
    if args.find.is_empty() || args.find.contains(['\r', '\n']) {
        return Err("--find must contain nonempty literal text on one line".into());
    }
    let data = fs::read(&args.path).map_err(|e| format!("{}: {e}", args.path.display()))?;
    let source = std::str::from_utf8(&data).map_err(|e| format!("invalid UTF-8: {e}"))?;
    let lines: Vec<_> = source.split_inclusive('\n').collect();
    let end = args.end.unwrap_or(lines.len());
    let identity = serde_json::to_vec(&(
        format!("{:x}", Sha256::digest(&data)),
        args.start,
        end,
        &args.find,
        &args.heading,
        args.context,
    ))
    .map_err(|e| e.to_string())?;
    let digest = format!("{:x}", Sha256::digest(identity));
    if args
        .sha256
        .as_ref()
        .is_some_and(|expected| *expected != digest)
    {
        return Err("source or search options changed; restart the search".into());
    }
    let empty = lines.is_empty() && args.start == 1 && args.end.is_none();
    if !empty && (args.start == 0 || args.start > end || end > lines.len()) {
        return Err(format!(
            "select an inclusive range within 1..{}",
            lines.len()
        ));
    }
    let (start, end) = match args.heading.as_deref() {
        Some(heading) => heading_scope(&lines, heading, args.start, end)?,
        None => (args.start, end),
    };
    let matches: Vec<_> = (start..=end)
        .filter(|number| lines[number - 1].contains(&args.find))
        .collect();
    let numbers = context_lines(&matches, args.context, start, end);
    let first = match args.next_line {
        Some(number) => numbers
            .binary_search(&number)
            .map_err(|_| "continuation line is outside the search results")?,
        None => 0,
    };
    let scope = if empty {
        "EMPTY FILE".to_owned()
    } else {
        format!("SELECTED {start}..{end} of {} lines", lines.len())
    };
    let mut output = format!(
        "SEARCH SHA256 {digest}\n{scope}\nMATCHES {}\n",
        matches.len()
    );
    let next = |number| {
        format!(
            "\nNEXT --next-line {number} --max-bytes {} --sha256 {digest}\n",
            args.max_bytes
        )
    };
    let reserved = next(end).len();
    let mut previous = None;
    for &number in &numbers[first..] {
        let gap = if previous.is_some_and(|last| number != last + 1) {
            "...\n"
        } else {
            ""
        };
        let row = format!("{gap}{number:6} | {}", lines[number - 1]);
        if output.len() + row.len() + reserved > args.max_bytes as usize {
            if previous.is_none() {
                return Err(format!(
                    "line {number} cannot fit; raise --max-bytes (up to 12000); no source text was emitted"
                ));
            }
            output.push_str(&next(number));
            return Ok(output);
        }
        output.push_str(&row);
        previous = Some(number);
    }
    if matches.is_empty() {
        output.push_str("NO MATCHES\n");
    }
    output.push_str(if previous.is_some_and(|n| !lines[n - 1].ends_with('\n')) {
        "\nEND (source has no final newline)\n"
    } else {
        "\nEND\n"
    });
    Ok(output)
}

fn heading_scope(
    lines: &[&str],
    heading: &str,
    start: usize,
    end: usize,
) -> Result<(usize, usize), String> {
    if heading.contains(['\r', '\n']) || heading_level(heading).is_none() {
        return Err("--heading must be an exact ATX heading line, including its # prefix".into());
    }
    let mut fence = None;
    let mut headings = Vec::new();
    for (index, source) in lines.iter().enumerate() {
        let Some(line) = markdown_line(source) else {
            continue;
        };
        let marker = fence_marker(line);
        if let Some((character, length)) = fence {
            if marker
                .is_some_and(|(c, n, tail)| c == character && n >= length && tail.trim().is_empty())
            {
                fence = None;
            }
            continue;
        }
        if let Some((character, length, tail)) = marker
            && (character != b'`' || !tail.contains('`'))
        {
            fence = Some((character, length));
            continue;
        }
        if let Some(level) = heading_level(source) {
            headings.push((index + 1, level));
        }
    }
    let mut found = headings.iter().filter(|(number, _)| {
        (start..=end).contains(number)
            && lines[number - 1].trim_end_matches(['\r', '\n']) == heading
    });
    let &(first, level) = found
        .next()
        .ok_or("heading not found in the selected range")?;
    if found.next().is_some() {
        return Err("heading is ambiguous in the selected range; narrow --start/--end".into());
    }
    let last = headings
        .iter()
        .find(|(number, depth)| *number > first && *depth <= level)
        .map_or(end, |(number, _)| end.min(number - 1));
    Ok((first, last))
}

fn markdown_line(line: &str) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    (indent <= 3).then_some(&line[indent..])
}

fn heading_level(line: &str) -> Option<usize> {
    let line = markdown_line(line)?;
    let level = line.bytes().take_while(|byte| *byte == b'#').count();
    ((1..=6).contains(&level)
        && line
            .as_bytes()
            .get(level)
            .is_none_or(|b| matches!(b, b' ' | b'\t')))
    .then_some(level)
}

fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let character = *line.as_bytes().first()?;
    if !matches!(character, b'`' | b'~') {
        return None;
    }
    let length = line.bytes().take_while(|byte| *byte == character).count();
    (length >= 3).then_some((character, length, &line[length..]))
}

fn context_lines(matches: &[usize], context: usize, start: usize, end: usize) -> Vec<usize> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &number in matches {
        let first = number.saturating_sub(context).max(start);
        let last = number.saturating_add(context).min(end);
        if let Some(range) = ranges.last_mut()
            && first <= range.1.saturating_add(1)
        {
            range.1 = range.1.max(last);
        } else {
            ranges.push((first, last));
        }
    }
    ranges
        .into_iter()
        .flat_map(|(first, last)| first..=last)
        .collect()
}
