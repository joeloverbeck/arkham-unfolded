#![cfg(feature = "tabletop")]
//! The agent seat's startup contract through the real executable (#153): a startup
//! problem is one refused response on standard output, the same text on standard
//! error and an unsuccessful exit. It is never a dialog and never the scenario
//! selection window. The seated table itself is covered by the protocol tests beside
//! the seat and by the ignored real-artwork run.

use serde_json::Value;
use std::{fs, path::Path, process::Output};
#[path = "support/native_process.rs"]
mod native_process;

fn launch(directory: &Path, arguments: &[&str]) -> Output {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_arkham-tabletop"));
    command.args(arguments);
    native_process::isolate(&mut command, directory);
    native_process::output(&mut command)
}

/// The single refused response, after checking the exit and standard error.
fn refusal(output: &Output) -> String {
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "one response expected: {stdout}");
    let response: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(response["ok"], false);
    assert_eq!(response["mode"], "seat");
    assert_eq!(response["status"], "refused");
    let reason = response["error"].as_str().unwrap().to_owned();
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&reason),
        "the refusal must also reach standard error"
    );
    reason
}

#[test]
fn an_absent_collection_is_one_refused_response_and_an_unsuccessful_exit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("artwork");
    for existing in [false, true] {
        let save = directory.path().join(if existing {
            "existing.json"
        } else {
            "fresh.json"
        });
        let original = b"existing personal save whose bytes must not be inspected or repaired";
        if existing {
            fs::write(&save, original).unwrap();
        }
        let reason = refusal(&launch(
            directory.path(),
            &[
                "--experiment",
                "gathering",
                "--agent-seat",
                "--art-root",
                root.to_str().unwrap(),
                "--save",
                save.to_str().unwrap(),
            ],
        ));
        assert!(reason.contains("cannot start play"), "{reason}");
        assert!(reason.contains(root.to_str().unwrap()), "{reason}");
        assert!(
            reason.contains("Absent or wrong prepared collection structure"),
            "{reason}"
        );
        if existing {
            assert_eq!(fs::read(&save).unwrap(), original);
        } else {
            assert!(!save.exists());
        }
        assert!(!directory.path().join("config").exists());
    }
}

#[test]
fn refused_options_are_text_and_never_scenario_selection() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("attempt.json");
    let save = save.to_str().unwrap();
    for (arguments, expected) in [
        // Native play returns a refused `--attempt` launch to scenario selection.
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--attempt",
                "new",
            ],
            "--agent-seat cannot be combined with --attempt",
        ),
        // Without the direct launch the executable would open scenario selection.
        (
            vec!["--agent-seat", "--save", save],
            "add --experiment gathering",
        ),
        (
            vec!["--experiment", "gathering", "--agent-seat"],
            "--agent-seat requires an explicit --save",
        ),
        (
            vec![
                "--experiment",
                "gathering",
                "--agent-seat",
                "--save",
                save,
                "--screenshot",
                "table.png",
            ],
            "--agent-seat cannot be combined with --attempt or --screenshot",
        ),
        // The flag swallowed as another option's value still never opens a window.
        (
            vec!["--experiment", "gathering", "--save", "--agent-seat"],
            "--agent-seat is an option of its own",
        ),
    ] {
        let reason = refusal(&launch(directory.path(), &arguments));
        assert!(reason.contains(expected), "{reason}");
        assert!(!Path::new(save).exists());
        assert!(!directory.path().join("local").exists());
    }
}
