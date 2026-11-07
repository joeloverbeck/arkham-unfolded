#![cfg(feature = "tabletop")]

use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
#[path = "support/native_process.rs"]
mod native_process;

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
    command.arg("--art-root").arg(root).arg("--save").arg(save);
    native_process::isolate(&mut command, directory);
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
                let output =
                    native_process::output(&mut native(directory.path(), mode, &root, &save));
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
        let output = native_process::output(
            native(
                directory.path(),
                mode,
                &directory.path().join("art"),
                &directory.path().join("save"),
            )
            .arg("--no-art"),
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--no-art"));
        assert!(!directory.path().join("save").exists());
    }
}

#[test]
fn a_stalled_native_subprocess_keeps_diagnostics_and_leaves_no_live_descendant() {
    use arkham_tools::process::Completion;
    use std::time::{Duration, Instant};
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--exact",
        "stalled_native_fixture",
        "--ignored",
        "--nocapture",
    ]);
    native_process::isolate(&mut command, directory.path());
    let started = Instant::now();
    let (completion, output) = native_process::capture(&mut command, Duration::from_secs(2));
    assert!(matches!(completion, Completion::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(String::from_utf8_lossy(&output.stdout).contains("ISOLATED_DESCENDANT_STDOUT"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ISOLATED_PARENT_STDERR"));
    for name in ["parent.pid", "descendant.pid"] {
        let pid = fs::read_to_string(directory.path().join(name)).unwrap();
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        assert!(
            stat.is_empty()
                || stat
                    .rsplit_once(')')
                    .unwrap()
                    .1
                    .trim_start()
                    .starts_with('Z'),
            "{name} is still alive: {pid}"
        );
    }
}

#[test]
#[ignore = "headless subprocess fixture, invoked only by the timeout regression"]
fn stalled_native_fixture() {
    use std::{env, io::Write, thread, time::Duration};
    let directory = env::current_dir().unwrap();
    for name in ["DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"] {
        assert!(env::var_os(name).is_none(), "inherited {name}");
    }
    assert_eq!(
        env::var_os("XDG_RUNTIME_DIR").unwrap(),
        directory.join("no-runtime")
    );
    assert_eq!(
        env::var("DBUS_SESSION_BUS_ADDRESS").unwrap(),
        format!("unix:path={}", directory.join("no-bus").display())
    );
    assert_eq!(env::var("GDK_BACKEND").unwrap(), "x11");
    unsafe extern "C" {
        fn signal(number: i32, handler: usize) -> usize;
    }
    // SAFETY: only this isolated fixture process ignores SIGTERM to test escalation.
    unsafe {
        signal(15, 1);
    }
    if env::var_os("ARKHAM_NATIVE_DESCENDANT").is_some() {
        fs::write("descendant.pid", std::process::id().to_string()).unwrap();
        println!("ISOLATED_DESCENDANT_STDOUT");
        std::io::stdout().flush().unwrap();
    } else {
        fs::write("parent.pid", std::process::id().to_string()).unwrap();
        let mut descendant = Command::new(env::current_exe().unwrap())
            .args([
                "--exact",
                "stalled_native_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("ARKHAM_NATIVE_DESCENDANT", "1")
            .spawn()
            .unwrap();
        eprintln!("ISOLATED_PARENT_STDERR");
        descendant.wait().unwrap();
    }
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
