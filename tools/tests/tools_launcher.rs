use std::{
    fs::{self, File},
    path::Path,
    process::{Command, Output},
};

fn launch(root: &Path, inherited: bool) -> Output {
    let mut command = Command::new("timeout");
    command
        .arg("15s")
        .arg(root.join("scripts/tools"))
        .arg("argument with spaces")
        .current_dir(root.join("caller"))
        .env("CARGO_NET_OFFLINE", "true")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET_DIR")
        .env_remove("CARGO_BUILD_BUILD_DIR");
    if inherited {
        command
            .env("CARGO_TARGET_DIR", root.join("application-target"))
            .env("CARGO_BUILD_BUILD_DIR", root.join("application-build"));
    }
    command.output().unwrap()
}

fn assert_output(output: Output, expected: &str) {
    assert!(
        output.status.success(),
        "status: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn launcher_rebuilds_without_waiting_for_application_builds_or_changing_child_environment() {
    for inherited in [false, true] {
        let directory = tempfile::Builder::new()
            .prefix("arkham tool's launcher ")
            .tempdir()
            .unwrap();
        let root = directory.path();
        fs::create_dir(root.join("scripts")).unwrap();
        fs::create_dir(root.join("caller")).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/tools"),
            root.join("scripts/tools"),
        )
        .unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"arkham-tools\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[[bin]]\nname=\"arkham-tools\"\npath=\"main.rs\"\n",
        )
        .unwrap();
        fs::write(
            root.join("Cargo.lock"),
            "version = 4\n[[package]]\nname = \"arkham-tools\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let source = r#"fn main() {
    println!("first|{}|{}|{}|{}", std::env::args().nth(1).unwrap(),
        std::env::current_dir().unwrap().display(),
        std::env::var("CARGO_TARGET_DIR").unwrap_or_default(),
        std::env::var("CARGO_BUILD_BUILD_DIR").unwrap_or_default());
}"#;
        fs::write(root.join("main.rs"), source).unwrap();
        let suffix = format!(
            "|argument with spaces|{}|{}|{}\n",
            root.join("caller").display(),
            if inherited {
                root.join("application-target").display().to_string()
            } else {
                String::new()
            },
            if inherited {
                root.join("application-build").display().to_string()
            } else {
                String::new()
            },
        );
        assert_output(launch(root, inherited), &format!("first{suffix}"));

        // Keep the application's output and intermediate locks held throughout
        // the rebuild. A stale-binary shortcut cannot satisfy the new output.
        let mut locks = Vec::new();
        for name in ["target", "application-target", "application-build"] {
            let debug = root.join(name).join("debug");
            fs::create_dir_all(&debug).unwrap();
            let lock = File::create(debug.join(".cargo-lock")).unwrap();
            lock.lock().unwrap();
            locks.push(lock);
        }
        fs::write(root.join("main.rs"), source.replace("first|", "updated|")).unwrap();
        assert_output(launch(root, inherited), &format!("updated{suffix}"));

        fs::write(root.join("main.rs"), "this is not valid Rust\n").unwrap();
        let failure = launch(root, inherited);
        assert_eq!(failure.status.code(), Some(101));
        assert!(failure.stdout.is_empty(), "an obsolete helper was executed");
        assert!(!failure.stderr.is_empty());
    }
}
