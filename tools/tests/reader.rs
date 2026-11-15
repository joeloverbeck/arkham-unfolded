//! Arkham's wiring of the shared reader commands. Their behaviour is held in
//! unfolded-tabletop's `dev-tools/tests/cli`.

use std::process::Command;

#[test]
fn help_identifies_the_batch_file_and_both_byte_limits() {
    // Arkham wires the shared commands; unfolded-tabletop's dev-tools tests hold
    // their behaviour. Every shared command must answer through arkham-tools.
    for command in [
        "read-context",
        "read-context-batch",
        "search-context",
        "record-check",
        "check-summary",
        "resource-summary",
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(result.status.success(), "{command}: {result:?}");
    }
    for command in ["read-context", "read-context-batch", "search-context"] {
        let result = Command::new(env!("CARGO_BIN_EXE_arkham-tools"))
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(result.stderr.is_empty());
        let help = String::from_utf8(result.stdout).unwrap();
        assert!(help.contains("--max-bytes <MAX_BYTES>"), "{help}");
        assert!(help.contains("512..=12000"), "{help}");
        if command == "read-context-batch" {
            assert!(help.contains("<REQUESTS_FILE>"), "{help}");
            assert!(help.contains("Path to a JSON file containing"), "{help}");
            assert!(help.contains("{path, start?, end?}"), "{help}");
            assert!(help.contains("{path, start?, lines?}"), "{help}");
        }
        if command == "read-context" {
            assert!(help.contains("--lines <LINES>"), "{help}");
        }
        if command == "search-context" {
            assert!(help.contains("--find <FIND>"), "{help}");
            assert!(help.contains("--heading <HEADING>"), "{help}");
            assert!(
                help.contains("repeat the same path and query/scope options"),
                "{help}"
            );
        }
    }
}
