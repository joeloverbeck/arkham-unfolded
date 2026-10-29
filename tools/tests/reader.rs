use std::{fs, process::Command};

fn read(path: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
        .arg("read-context")
        .arg(path)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn continuations_cover_selected_unicode_lines_once_within_the_complete_byte_limit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.txt");
    let lines: Vec<_> = (1..=80)
        .map(|n| format!("  source {n}: café 漢字 — {}\n", "x".repeat(n % 37)))
        .collect();
    let source = lines.concat();
    fs::write(&path, &source).unwrap();
    let mut args = "--start 3 --end 79 --max-bytes 512".to_owned();
    let mut observed = Vec::new();
    let mut pages = 0;
    loop {
        let output = read(&path, &args.split_whitespace().collect::<Vec<_>>());
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.len() <= 512);
        let text = String::from_utf8(output.stdout).unwrap();
        for row in text.lines() {
            if let Some((number, content)) = row.split_once(" | ") {
                observed.push((
                    number.trim().parse::<usize>().unwrap(),
                    format!("{content}\n"),
                ));
            }
        }
        pages += 1;
        assert!(pages <= 80, "continuation did not advance");
        match text.lines().find_map(|line| line.strip_prefix("NEXT ")) {
            Some(next) => args = next.to_owned(),
            None => {
                assert!(text.ends_with("\nEND\n"));
                break;
            }
        }
    }
    assert!(pages > 1);
    assert_eq!(
        observed,
        lines
            .into_iter()
            .enumerate()
            .map(|(n, line)| (n + 1, line))
            .skip(2)
            .take(77)
            .collect::<Vec<_>>()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), source);
}

#[test]
fn changed_source_refuses_continuation_without_emitting_content() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.txt");
    fs::write(&path, "original\n".repeat(100)).unwrap();
    let first = String::from_utf8(read(&path, &["--max-bytes", "512"]).stdout).unwrap();
    let continuation = first
        .lines()
        .find_map(|line| line.strip_prefix("NEXT "))
        .unwrap();
    fs::write(&path, "changed\n".repeat(100)).unwrap();
    let result = read(&path, &continuation.split_whitespace().collect::<Vec<_>>());
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(String::from_utf8_lossy(&result.stderr).contains("source changed"));
}

#[test]
fn invalid_ranges_limits_files_and_oversized_lines_emit_no_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.txt");
    let missing = read(&path, &[]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    for data in [b"before\n\xff\nafter\n".to_vec(), vec![b'x'; 13_000]] {
        fs::write(&path, data).unwrap();
        let result = read(&path, &["--max-bytes", "12000"]);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
    }
    fs::write(&path, "one\ntwo\n").unwrap();
    for args in [
        &["--start", "0"][..],
        &["--end", "3"],
        &["--start", "2", "--end", "1"],
        &["--max-bytes", "511"],
        &["--max-bytes", "12001"],
    ] {
        let result = read(&path, args);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn empty_crlf_and_missing_final_newline_are_explicit_and_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.txt");
    fs::write(&path, "").unwrap();
    let empty = read(&path, &[]);
    assert!(empty.status.success());
    assert_eq!(
        String::from_utf8(empty.stdout).unwrap(),
        "SHA256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\nEMPTY FILE\nEND\n"
    );
    fs::write(&path, "first\r\nlast without newline").unwrap();
    let result = read(&path, &[]);
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("     1 | first\r\n"));
    assert!(output.ends_with("     2 | last without newline\nEND (source has no final newline)\n"));
}
