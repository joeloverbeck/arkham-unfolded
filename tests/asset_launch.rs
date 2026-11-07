use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn check_launch(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arkham-assets"))
        .arg("check-gathering-launch")
        .args(args)
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_HOME", root.join("data"))
        .output()
        .unwrap()
}

#[test]
fn launch_check_uses_the_installed_default_and_leaves_missing_inputs_absent() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let output = check_launch(root, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&format!(
            "Artwork root: {}",
            root.join("data/arkham-unfolded/gathering-assets").display()
        )),
        "{stdout}"
    );
    assert!(stdout.contains("No compatible prepared asset collection"));
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn saved_preference_and_relative_override_select_the_checked_folder_without_writes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let settings = root.join("config/arkham-unfolded/gathering.toml");
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let saved = root.join("saved-collection");
    let original = format!("asset_root = {:?}\n", saved.to_str().unwrap());
    fs::write(&settings, &original).unwrap();

    for (args, expected) in [
        (vec![], saved),
        (
            vec!["--art-root", "override-collection"],
            root.join("override-collection"),
        ),
        (
            vec![
                "--art-root",
                "obsolete",
                "--art-root",
                "override-collection",
            ],
            root.join("override-collection"),
        ),
    ] {
        let output = check_launch(root, &args);
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains(&format!("Artwork root: {}", expected.display())),
            "{args:?}: {stdout}; {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(stdout.contains("No compatible prepared asset collection"));
        assert!(!expected.exists());
        assert_eq!(fs::read_to_string(&settings).unwrap(), original);
    }
    assert!(!root.join("data").exists());
}

#[test]
fn removed_no_art_option_is_rejected_in_either_flag_order() {
    let directory = tempfile::tempdir().unwrap();
    for args in [
        vec!["--no-art"],
        vec!["--no-art", "--art-root", "unused"],
        vec!["--art-root", "unused", "--no-art"],
    ] {
        let output = check_launch(directory.path(), &args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("Unknown Gathering artwork option: --no-art")
        );
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn malformed_settings_warn_and_use_the_default_without_repairing_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let settings = root.join("config/arkham-unfolded/gathering.toml");
    fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let original = b"asset_root = [unfinished";
    fs::write(&settings, original).unwrap();
    let output = check_launch(root, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!(
        "Artwork root: {}",
        root.join("data/arkham-unfolded/gathering-assets").display()
    )));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Asset settings: Could not read settings")
    );
    assert_eq!(fs::read(&settings).unwrap(), original);
    assert!(!root.join("data").exists());
}
