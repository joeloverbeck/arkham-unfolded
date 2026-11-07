#![cfg(feature = "tabletop")]

use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

const MODES: [(&str, &str); 4] = [
    (
        "jacqueline",
        include_str!("../resources/asset-selection.json"),
    ),
    (
        "tony",
        include_str!("../resources/tony-asset-selection.json"),
    ),
    (
        "party",
        include_str!("../resources/party-asset-selection.json"),
    ),
    (
        "gathering",
        include_str!("../resources/gathering-asset-selection.json"),
    ),
];

fn native(directory: &Path, mode: &str, root: &Path, save: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-tabletop"));
    if mode != "jacqueline" {
        command.args(["--experiment", mode]);
    }
    command
        .arg("--art-root")
        .arg(root)
        .arg("--save")
        .arg(save)
        .current_dir(directory)
        .env("XDG_CONFIG_HOME", directory.join("config"))
        .env("XDG_DATA_HOME", directory.join("data"))
        .env("XDG_RUNTIME_DIR", directory.join("no-runtime"))
        .env("GDK_BACKEND", "x11")
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}", directory.join("no-bus").display()),
        )
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET");
    command
}

fn manifest(recipe: &Value) -> Value {
    let assets: Vec<Value> = recipe["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|asset| {
            let mut identity = json!({"role": asset["role"], "path": asset["path"],
            "bytes": asset["bytes"], "sha256": asset["sha256"]});
            if let Some(attribution) = asset.get("attribution") {
                identity["attribution"] = attribution.clone();
            }
            identity
        })
        .collect();
    json!({"format_version": recipe["format_version"], "collection": recipe["collection"], "assets": assets})
}

#[test]
fn native_modes_refuse_missing_invalid_and_partial_collections_before_touching_saves() {
    for (mode, recipe) in MODES {
        let recipe: Value = serde_json::from_str(recipe).unwrap();
        let selected = manifest(&recipe);
        let count = selected["assets"].as_array().unwrap().len();
        for failure in [
            "absent",
            "structure",
            "missing-manifest",
            "invalid-manifest",
            "incompatible",
            "partial",
        ] {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("artwork");
            if failure != "absent" {
                fs::create_dir(&root).unwrap();
            }
            if !matches!(failure, "absent" | "structure") {
                for asset in selected["assets"].as_array().unwrap() {
                    fs::create_dir_all(
                        root.join(asset["path"].as_str().unwrap()).parent().unwrap(),
                    )
                    .unwrap();
                }
            }
            match failure {
                "invalid-manifest" => {
                    fs::write(root.join("manifest.json"), b"{unfinished").unwrap()
                }
                "incompatible" => fs::write(
                    root.join("manifest.json"),
                    b"{\"format_version\":1,\"collection\":\"other\",\"assets\":[]}",
                )
                .unwrap(),
                "partial" => {
                    fs::write(
                        root.join("manifest.json"),
                        serde_json::to_vec(&selected).unwrap(),
                    )
                    .unwrap();
                    fs::write(
                        root.join(selected["assets"][0]["path"].as_str().unwrap()),
                        b"changed payload",
                    )
                    .unwrap();
                }
                _ => {}
            }
            for existing in [false, true] {
                let save = directory.path().join(if existing {
                    "existing.json"
                } else {
                    "fresh.json"
                });
                let original =
                    b"existing personal save whose bytes must not be inspected or repaired";
                if existing {
                    fs::write(&save, original).unwrap();
                }
                let output = native(directory.path(), mode, &root, &save)
                    .output()
                    .unwrap();
                assert!(!output.status.success(), "{mode}/{failure}");
                let diagnostic = String::from_utf8_lossy(&output.stderr);
                assert!(
                    diagnostic.contains(root.to_str().unwrap()),
                    "{mode}/{failure}: {diagnostic}"
                );
                assert!(diagnostic.contains("cannot start play"), "{diagnostic}");
                match failure {
                    "absent" | "structure" => assert!(
                        diagnostic.contains("Absent or wrong prepared collection structure"),
                        "{diagnostic}"
                    ),
                    "partial" => {
                        assert!(
                            diagnostic.contains(&format!("{count} required files affected:")),
                            "{diagnostic}"
                        );
                        assert!(
                            diagnostic.contains(&format!(
                                "{} additional affected files omitted",
                                count - 5
                            )),
                            "{diagnostic}"
                        );
                        assert!(
                            !diagnostic.contains(selected["assets"][5]["path"].as_str().unwrap()),
                            "the sixth example must be omitted: {diagnostic}"
                        );
                    }
                    _ => assert!(diagnostic.contains("manifest.json"), "{diagnostic}"),
                }
                if existing {
                    assert_eq!(fs::read(&save).unwrap(), original);
                } else {
                    assert!(!save.exists());
                }
                assert!(!directory.path().join("config").exists());
            }
        }
    }
}

#[test]
fn native_modes_reject_removed_no_art_option() {
    for (mode, _) in MODES {
        let directory = tempfile::tempdir().unwrap();
        let output = native(
            directory.path(),
            mode,
            &directory.path().join("art"),
            &directory.path().join("save"),
        )
        .arg("--no-art")
        .output()
        .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--no-art"));
        assert!(!directory.path().join("save").exists());
    }
}
