//! Fail unless every declaration of the shared `unfolded-tabletop` repository in the named
//! manifests names one revision: its packages and the `wgpu-hal` patch move together.
//! It reads the manifests' text without Cargo, so a partial update fails here with each
//! revision named even before the lockfile records it. This entry point also compiles
//! directly with rustc, so it uses only the standard library.
use std::{fmt::Write as _, process::ExitCode};

const REPOSITORY: &str = "github.com/joeloverbeck/unfolded-tabletop";

#[derive(Debug)]
struct Declaration {
    location: String,
    name: String,
    revision: String,
}

/// A recursive reader for the one-line TOML values a dependency declaration uses.
struct Cursor<'a>(&'a str);

impl Cursor<'_> {
    fn skip_space(&mut self) {
        self.0 = self.0.trim_start_matches([' ', '\t']);
    }

    fn eat(&mut self, expected: char) -> bool {
        match self.0.strip_prefix(expected) {
            Some(rest) => {
                self.0 = rest;
                true
            }
            None => false,
        }
    }

    fn string(&mut self) -> Result<String, String> {
        let quote = self.0.chars().next().filter(|c| *c == '"' || *c == '\'');
        let quote = quote.ok_or("expected a string")?;
        if self.0.starts_with(&format!("{quote}{quote}{quote}")) {
            return Err("uses a multi-line string".into());
        }
        let mut content = String::new();
        let mut characters = self.0[1..].char_indices();
        while let Some((index, character)) = characters.next() {
            if character == quote {
                self.0 = &self.0[index + 2..];
                return Ok(content);
            }
            if character == '\\' && quote == '"' {
                let (_, escaped) = characters.next().ok_or("ends inside a string")?;
                let digits = match escaped {
                    'u' => 4,
                    'U' => 8,
                    _ => {
                        content.push(escaped);
                        continue;
                    }
                };
                let hex: String = characters.by_ref().take(digits).map(|(_, c)| c).collect();
                let code = u32::from_str_radix(&hex, 16)
                    .ok()
                    .filter(|_| hex.len() == digits);
                content.push(
                    code.and_then(char::from_u32)
                        .ok_or("has an invalid escape")?,
                );
            } else {
                content.push(character);
            }
        }
        Err("ends inside a string".into())
    }

    /// Reads a value, returning its text only when it is a string.
    fn value(&mut self) -> Result<Option<String>, String> {
        match self.0.chars().next() {
            Some('"' | '\'') => self.string().map(Some),
            Some('[') => {
                self.eat('[');
                loop {
                    self.skip_space();
                    if self.eat(']') {
                        return Ok(None);
                    }
                    self.value()?;
                    self.skip_space();
                    if !self.eat(',') {
                        self.skip_space();
                        return if self.eat(']') {
                            Ok(None)
                        } else {
                            Err("ends inside an array".into())
                        };
                    }
                }
            }
            Some('{') => self.table().map(|_| None),
            _ => {
                let end = self.0.find([',', ']', '}', ' ', '\t', '#']);
                let end = end.unwrap_or(self.0.len());
                if end == 0 {
                    return Err("has a missing value".into());
                }
                self.0 = &self.0[end..];
                Ok(None)
            }
        }
    }

    fn table(&mut self) -> Result<Vec<(String, Option<String>)>, String> {
        let mut pairs: Vec<(String, Option<String>)> = Vec::new();
        self.eat('{');
        self.skip_space();
        if self.eat('}') {
            return Ok(pairs);
        }
        loop {
            self.skip_space();
            let key = if self.0.starts_with(['"', '\'']) {
                self.string()?
            } else {
                let end = self.0.find(['=', ' ', '\t']).unwrap_or(self.0.len());
                let (key, rest) = self.0.split_at(end);
                self.0 = rest;
                key.to_owned()
            };
            self.skip_space();
            if key.is_empty() || !self.eat('=') {
                return Err("ends inside an inline table".into());
            }
            self.skip_space();
            let value = self.value()?;
            if pairs.iter().any(|(existing, _)| *existing == key) {
                return Err(format!("repeats `{key}`"));
            }
            pairs.push((key, value));
            self.skip_space();
            if self.eat(',') {
                self.skip_space();
                if self.eat('}') {
                    return Ok(pairs);
                }
            } else if self.eat('}') {
                return Ok(pairs);
            } else {
                return Err("ends inside an inline table".into());
            }
        }
    }
}

/// Whether text spells the repository's URL, in any letter case as Cargo compares GitHub URLs.
fn names_repository(text: &str) -> bool {
    text.to_ascii_lowercase().contains(REPOSITORY)
}

/// Whether a `git` source is the repository.
fn is_repository(git: &str) -> bool {
    let git = git.to_ascii_lowercase();
    let git = git.trim_end_matches('/');
    git.strip_suffix(".git")
        .unwrap_or(git)
        .ends_with(REPOSITORY)
}

/// A dotted key's segments without surrounding space or quotes.
fn key(text: &str) -> String {
    let segments: Vec<_> = text
        .split('.')
        .map(|segment| segment.trim().trim_matches(['"', '\'']))
        .collect();
    segments.join(".")
}

/// Reads one `key = value` line. A line that spells the repository's URL must be readable: a
/// one-line inline table, or a plain value that is not a `git` key. Other unreadable lines are
/// left to Cargo, but any line whose `git` source is the repository is a declaration of it.
fn declaration(line: &str, spells_repository: bool) -> Result<Option<(String, String)>, String> {
    let unreadable = |problem: &str| {
        if spells_repository {
            Err(format!(
                "{problem}; write the declaration as a one-line inline table"
            ))
        } else {
            Ok(None)
        }
    };
    let Some((name, value)) = line.split_once('=') else {
        return unreadable("is not a `key = value` line");
    };
    let name = key(name);
    let mut cursor = Cursor(value.trim_start());
    let pairs = if cursor.0.starts_with('{') {
        match cursor.table() {
            Ok(pairs) => Some(pairs),
            Err(problem) => return unreadable(&problem),
        }
    } else {
        match cursor.value() {
            Ok(Some(git)) if (name == "git" || name.ends_with(".git")) && is_repository(&git) => {
                return Err("declares the repository outside a one-line inline table".into());
            }
            Ok(_) => None,
            Err(problem) => return unreadable(&problem),
        }
    };
    cursor.skip_space();
    if !(cursor.0.is_empty() || cursor.0.starts_with('#')) {
        return unreadable("has text after its value");
    }
    let Some(pairs) = pairs else {
        return Ok(None);
    };
    let field = |wanted: &str| pairs.iter().find(|(key, _)| key == wanted).map(|(_, v)| v);
    match (field("git"), field("rev")) {
        (Some(Some(git)), revision) if is_repository(git) => match revision {
            Some(Some(revision)) => Ok(Some((name, revision.clone()))),
            _ => Err(format!("`{name}` names no `rev`; pin the shared revision")),
        },
        _ => Ok(None),
    }
}

/// Returns a summary when every declaration across the manifests names one revision, and
/// otherwise a report naming each revision with its declarations, or each unreadable line.
fn check(manifests: &[(&str, &str)]) -> Result<String, String> {
    let mut found = Vec::new();
    let mut counts = Vec::new();
    let mut problems = String::new();
    for (manifest, text) in manifests {
        let before = found.len();
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            let spells_repository = names_repository(line);
            let location = format!("{manifest}:{}", index + 1);
            let result = if trimmed.starts_with('[') {
                if !spells_repository {
                    continue;
                }
                Err(
                    "names the repository in a table header, which this check cannot compare"
                        .into(),
                )
            } else {
                declaration(line, spells_repository)
            };
            match result {
                Ok(Some((name, revision))) => found.push(Declaration {
                    location,
                    name,
                    revision,
                }),
                Ok(None) => {}
                Err(problem) => writeln!(problems, "  {location}: {problem}").unwrap(),
            }
        }
        let count = found.len() - before;
        if count == 0 {
            writeln!(
                problems,
                "  {manifest}: declares no package from {REPOSITORY}"
            )
            .unwrap();
        }
        counts.push(format!("{manifest}: {count}"));
    }
    if !problems.is_empty() {
        return Err(format!(
            "Every declaration of {REPOSITORY} must be readable here:\n{problems}"
        ));
    }
    let mut revisions: Vec<(&str, Vec<&Declaration>)> = Vec::new();
    for pin in &found {
        match revisions
            .iter_mut()
            .find(|(revision, _)| *revision == pin.revision)
        {
            Some((_, group)) => group.push(pin),
            None => revisions.push((&pin.revision, vec![pin])),
        }
    }
    if let [(revision, _)] = revisions[..] {
        return Ok(format!(
            "All {} declarations of {REPOSITORY} name {revision} ({})",
            found.len(),
            counts.join(", ")
        ));
    }
    let mut report = format!(
        "The manifests name {} revisions of {REPOSITORY}; every declaration must name the same one:\n",
        revisions.len()
    );
    for (revision, group) in revisions {
        writeln!(report, "  {revision}").unwrap();
        for declaration in group {
            writeln!(report, "    {} {}", declaration.location, declaration.name).unwrap();
        }
    }
    Err(report)
}

fn main() -> ExitCode {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: check-shared-pins MANIFEST...");
        return ExitCode::from(2);
    }
    let mut texts = Vec::new();
    for path in &paths {
        match std::fs::read_to_string(path) {
            Ok(text) => texts.push(text),
            Err(error) => {
                eprintln!("check-shared-pins: read {path}: {error}");
                return ExitCode::from(2);
            }
        }
    }
    let manifests: Vec<_> = paths
        .iter()
        .map(String::as_str)
        .zip(texts.iter().map(String::as_str))
        .collect();
    match check(&manifests) {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(report) => {
            eprint!("check-shared-pins: {report}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTHER: &str = "9ab6f1ebc3c4162a09681d5d215f26c04cd93cd8";

    fn manifests() -> (String, String) {
        let tools = env!("CARGO_MANIFEST_DIR");
        (
            std::fs::read_to_string(format!("{tools}/../Cargo.toml")).unwrap(),
            std::fs::read_to_string(format!("{tools}/Cargo.toml")).unwrap(),
        )
    }

    /// The revision the repository's manifests currently share.
    fn current(text: &str) -> String {
        let line = text.lines().find(|line| line.contains(REPOSITORY)).unwrap();
        declaration(line, true).unwrap().unwrap().1
    }

    /// Moves only the named declaration to another revision.
    fn drift(text: &str, name: &str) -> String {
        let revision = current(text);
        let mut moved = 0;
        let lines: Vec<_> = text
            .lines()
            .map(|line| {
                if line.starts_with(&format!("{name} = ")) && line.contains(REPOSITORY) {
                    moved += 1;
                    line.replace(&revision, OTHER)
                } else {
                    line.to_owned()
                }
            })
            .collect();
        assert_eq!(moved, 1, "{name}");
        lines.join("\n")
    }

    fn run(root: &str, tools: &str) -> Result<String, String> {
        check(&[("Cargo.toml", root), ("tools/Cargo.toml", tools)])
    }

    #[test]
    fn the_repository_manifests_agree() {
        let (root, tools) = manifests();
        let summary = run(&root, &tools).unwrap();
        assert!(summary.contains(&current(&root)), "{summary}");
    }

    #[test]
    fn one_drifting_declaration_in_either_manifest_names_both_revisions() {
        let (root, tools) = manifests();
        let revision = current(&root);
        for (name, report) in [
            (
                "unfolded-random",
                run(&drift(&root, "unfolded-random"), &tools),
            ),
            (
                "unfolded-dev-tools",
                run(&root, &drift(&tools, "unfolded-dev-tools")),
            ),
            ("wgpu-hal", run(&drift(&root, "wgpu-hal"), &tools)),
        ] {
            let report = report.unwrap_err();
            assert!(report.contains("name 2 revisions"), "{report}");
            assert!(
                report.contains(&revision) && report.contains(OTHER),
                "{report}"
            );
            let moved = report.split_once(&format!("  {OTHER}\n")).unwrap().1;
            assert_eq!(moved.lines().count(), 1, "{report}");
            assert!(moved.trim_end().ends_with(&format!(" {name}")), "{report}");
        }
    }

    #[test]
    fn a_coordinated_update_to_another_revision_passes() {
        let (root, tools) = manifests();
        let revision = current(&root);
        let summary = run(
            &root.replace(&revision, OTHER),
            &tools.replace(&revision, OTHER),
        );
        let summary = summary.unwrap();
        assert!(
            summary.contains(OTHER) && !summary.contains(&revision),
            "{summary}"
        );
    }

    #[test]
    fn declarations_the_check_cannot_read_are_refused_not_skipped() {
        let url = format!("https://{REPOSITORY}");
        let pinned = format!("a = {{ git = \"{url}\", rev = \"{OTHER}\" }}");
        for (case, text) in [
            (
                "dotted table",
                format!("[dependencies.a]\ngit = \"{url}\"\nrev = \"{OTHER}\""),
            ),
            (
                "branch",
                format!("a = {{ git = \"{url}\", branch = \"main\" }}"),
            ),
            ("no revision", format!("a = {{ git = \"{url}\" }}")),
            (
                "multi-line",
                format!("a = {{ git = \"{url}\", features = [\n  \"x\",\n] }}"),
            ),
            (
                "unterminated",
                format!("a = {{ git = \"{url}, rev = \"{OTHER}\" }}"),
            ),
            (
                "repeated",
                format!("a = {{ git = \"{url}\", rev = \"{OTHER}\", rev = \"x\" }}"),
            ),
            ("trailing text", format!("{pinned} b")),
            (
                "quoted git key",
                format!("[dependencies.b]\n\"git\" = \"{url}\""),
            ),
            ("spaced dotted key", format!("b . git = \"{url}\"")),
            ("quoted dotted key", format!("b.\"git\" = \"{url}\"")),
            (
                "letter case",
                "[dependencies.b]\ngit = \"https://github.com/JoelOverbeck/unfolded-tabletop\""
                    .to_owned(),
            ),
            (
                "escaped table form",
                "b.git = \"https://github.com/joeloverbeck/unfolded\\u002Dtabletop\"".to_owned(),
            ),
            (
                "escaped without revision",
                "b = { git = \"https://github.com/joeloverbeck/unfolded\\u002Dtabletop\" }"
                    .to_owned(),
            ),
            (
                "patch header",
                format!("[patch.\"{url}\"]\nunfolded-assets = {{ path = \"x\" }}"),
            ),
            (
                "continuation",
                format!("b = {{\n  version = \"1\", git = \"{url}\", rev = \"{OTHER}\" }}"),
            ),
            (
                "invalid escape",
                format!("b = {{ git = \"{url}\", rev = \"\\u00\" }}"),
            ),
        ] {
            // A readable pin keeps the manifest from failing only for declaring none.
            let report = run(&pinned, &format!("{pinned}\n{text}")).unwrap_err();
            assert!(report.starts_with("Every declaration"), "{case}: {report}");
            assert!(report.contains("  tools/Cargo.toml:"), "{case}: {report}");
            assert!(!report.contains("declares no package"), "{case}: {report}");
        }
        let report = run(&pinned, "a = \"1\"").unwrap_err();
        assert!(
            report.contains("tools/Cargo.toml: declares no package"),
            "{report}"
        );
    }

    #[test]
    fn unusual_spellings_of_the_repository_join_the_comparison() {
        let pinned = format!("a = {{ git = \"https://{REPOSITORY}\", rev = \"{OTHER}\" }}");
        for git in [
            "https://github.com/JoelOverbeck/Unfolded-Tabletop",
            "https://github.com/joeloverbeck/unfolded\\u002Dtabletop",
            "ssh://git@github.com/joeloverbeck/unfolded-tabletop.git",
        ] {
            let other = format!("b = {{ git = \"{git}\", rev = \"0510364\" }}");
            let report = run(&pinned, &format!("{pinned}\n{other}")).unwrap_err();
            assert!(report.contains("name 2 revisions"), "{git}: {report}");
            assert!(report.contains("tools/Cargo.toml:2 b"), "{git}: {report}");
        }
    }

    #[test]
    fn other_values_and_spellings_are_read_as_toml() {
        let url = format!("https://{REPOSITORY}");
        let text = [
            format!("# {url} at any revision"),
            format!("homepage = \"{url}\""),
            "other = { git = \"https://github.com/joeloverbeck/other\", branch = \"main\" }".into(),
            format!("a = {{ git = \"{url}.git\", rev = \"{OTHER}\", features = [\"x\", \"y\"], optional = true }} # pin"),
            format!("b = {{ version = \"1\", git = '{url}/', rev = '{OTHER}', default-features = false, }}"),
            format!("\"c\" = {{ rev = \"{OTHER}\", git = \"{url}\", package = \"d\" }}"),
        ]
        .join("\n");
        let summary = run(
            &text,
            &format!("a = {{ git = \"{url}\", rev = \"{OTHER}\" }}"),
        )
        .unwrap();
        assert!(summary.starts_with("All 4 declarations"), "{summary}");
        assert!(
            summary.ends_with("(Cargo.toml: 3, tools/Cargo.toml: 1)"),
            "{summary}"
        );
    }
}
