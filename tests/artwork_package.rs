use arkham_unfolded_experiment::{
    asset_config::AssetConfig,
    assets::{
        inspect_gathering,
        package::{import_gathering, package_gathering},
    },
};
use std::{fs, path::PathBuf};

fn small_archive(path: &std::path::Path, first: &str) {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for i in 0..149 {
        zip.start_file(
            if i == 0 {
                first.to_owned()
            } else {
                format!("unused-{i}")
            },
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"invalid artwork").unwrap();
    }
    zip.finish().unwrap();
}

fn required_manifest() -> serde_json::Value {
    let recipe: serde_json::Value =
        serde_json::from_str(include_str!("../resources/gathering-asset-selection.json")).unwrap();
    let assets: Vec<_> = recipe["assets"].as_array().unwrap().iter().map(|asset| {
        let mut identity = serde_json::json!({"role":asset["role"],"path":asset["path"],"bytes":asset["bytes"],"sha256":asset["sha256"]});
        if let Some(attribution) = asset.get("attribution") { identity["attribution"] = attribution.clone(); }
        identity
    }).collect();
    serde_json::json!({"format_version":recipe["format_version"],"collection":recipe["collection"],"assets":assets})
}

fn refusal_archive(path: &std::path::Path, member: &str, data: &[u8], link: bool) {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(&serde_json::to_vec(&required_manifest()).unwrap())
        .unwrap();
    if link {
        zip.add_symlink(member, "outside", options).unwrap();
    } else {
        zip.start_file(member, options).unwrap();
        zip.write_all(data).unwrap();
    }
    for i in 2..149 {
        zip.start_file(format!("unused-{i}"), options).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn import_refuses_while_another_process_has_artwork_in_use() {
    let dir = tempfile::tempdir().unwrap();
    let mut config =
        AssetConfig::load_at(dir.path().join("settings"), dir.path().join("data/assets"));
    let managed = config.installation_directory();
    fs::create_dir_all(&managed).unwrap();
    let lease = fs::File::create(managed.join("play.lock")).unwrap();
    lease.try_lock_shared().unwrap();
    let zip = dir.path().join("input.zip");
    small_archive(&zip, "manifest.json");
    let error = import_gathering(&zip, &mut config).unwrap_err();
    assert!(error.contains("Close active play"), "{error}");
    assert!(!config.path().exists());
}

#[test]
fn unsafe_archive_destinations_are_refused_without_changing_user_files() {
    for name in ["../escape", "/absolute", "images/../escape", "undeclared"] {
        let dir = tempfile::tempdir().unwrap();
        let prior = dir.path().join("prior");
        fs::create_dir(&prior).unwrap();
        fs::write(prior.join("unique"), b"retain historical input").unwrap();
        let save = dir.path().join("save");
        fs::write(&save, b"personal save").unwrap();
        let mut config =
            AssetConfig::load_at(dir.path().join("settings"), dir.path().join("data/assets"));
        config.save_root(&prior).unwrap();
        let settings = fs::read(config.path()).unwrap();
        let zip = dir.path().join("input.zip");
        refusal_archive(&zip, name, b"invalid artwork", false);
        let input = fs::read(&zip).unwrap();
        let error = import_gathering(&zip, &mut config).unwrap_err();
        assert!(
            error.contains("Undeclared ZIP destination"),
            "{name}: {error}"
        );
        assert_eq!(fs::read(config.path()).unwrap(), settings);
        assert_eq!(fs::read(&zip).unwrap(), input);
        assert_eq!(fs::read(&save).unwrap(), b"personal save");
        assert_eq!(
            fs::read(prior.join("unique")).unwrap(),
            b"retain historical input"
        );
        assert!(!dir.path().join("escape").exists());
        assert!(
            !arkham_unfolded_experiment::assets::package::installation_status(&config)
                .contains("unselected")
        );
    }
}

#[test]
fn corrupt_oversized_linked_and_truncated_payloads_are_refused() {
    let manifest = required_manifest();
    let asset = &manifest["assets"][0];
    for case in ["changed", "oversized", "link", "truncated"] {
        let dir = tempfile::tempdir().unwrap();
        let mut config =
            AssetConfig::load_at(dir.path().join("settings"), dir.path().join("data/assets"));
        let zip = dir.path().join("input.zip");
        let data = vec![
            0;
            if case == "oversized" {
                asset["bytes"].as_u64().unwrap() as usize + 1
            } else {
                1
            }
        ];
        refusal_archive(&zip, asset["path"].as_str().unwrap(), &data, case == "link");
        if case == "truncated" {
            let file = fs::OpenOptions::new().write(true).open(&zip).unwrap();
            file.set_len(file.metadata().unwrap().len() - 12).unwrap();
        }
        let error = import_gathering(&zip, &mut config).unwrap_err();
        let expected = match case {
            "changed" => "SHA-256",
            "oversized" => "trusted size limit",
            "link" => "link or special",
            _ => "footer",
        };
        assert!(error.contains(expected), "{case}: {error}");
        assert!(!config.path().exists());
        assert!(
            !arkham_unfolded_experiment::assets::package::installation_status(&config)
                .contains("unselected")
        );
    }
}

#[test]
fn older_collection_is_named_and_cancel_keeps_settings_absent() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let mut config =
        AssetConfig::load_at(dir.path().join("settings"), dir.path().join("data/assets"));
    let path = dir.path().join("old.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    zip.start_file("manifest.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    let mut manifest = required_manifest();
    manifest["collection"] = "arkham-unfolded-gathering-v6".into();
    zip.write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    zip.finish().unwrap();
    let error = import_gathering(&path, &mut config).unwrap_err();
    assert!(
        error.contains("Incompatible artwork collection 'arkham-unfolded-gathering-v6'"),
        "{error}"
    );
    let canceled = std::sync::atomic::AtomicBool::new(true);
    let error = arkham_unfolded_experiment::assets::package::import_gathering_cancellable(
        &path,
        &mut config,
        &canceled,
    )
    .unwrap_err();
    assert!(error.contains("Import canceled"), "{error}");
    assert!(!config.path().exists());
}

#[test]
fn duplicate_destinations_and_missing_or_excessive_metadata_are_refused() {
    for case in ["duplicate", "missing", "oversized manifest"] {
        let dir = tempfile::tempdir().unwrap();
        let mut config =
            AssetConfig::load_at(dir.path().join("settings"), dir.path().join("data/assets"));
        let path = dir.path().join("input.zip");
        refusal_archive(&path, "undeclared", b"", false);
        let mut bytes = fs::read(&path).unwrap();
        if case == "duplicate" || case == "missing" {
            let (from, to): (&[u8], &[u8]) = if case == "duplicate" {
                (b"unused-148", b"unused-147")
            } else {
                (b"manifest.json", b"not-found.txt")
            };
            for i in 0..=bytes.len() - from.len() {
                if &bytes[i..i + from.len()] == from {
                    bytes[i..i + from.len()].copy_from_slice(to);
                }
            }
            fs::write(&path, bytes).unwrap();
        } else {
            use std::io::Write;
            let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
            zip.start_file(
                "manifest.json",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
            zip.write_all(&vec![b' '; 65537]).unwrap();
            zip.finish().unwrap();
        }
        let error = import_gathering(&path, &mut config).unwrap_err();
        let expected = match case {
            "duplicate" => "duplicate",
            "missing" => "not found",
            _ => "trusted size limit",
        };
        assert!(error.to_lowercase().contains(expected), "{case}: {error}");
        assert!(!config.path().exists());
    }
}

#[test]
#[ignore = "requires ARKHAM_PACKAGE_ZIP pointing to the verified real runtime ZIP"]
fn killed_extraction_leaves_previous_selection_and_reimport_recovers() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let zip = PathBuf::from(std::env::var_os("ARKHAM_PACKAGE_ZIP").expect("real runtime ZIP"));
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let settings = dir.path().join("config/arkham-unfolded/gathering.toml");
    let default = data.join("arkham-unfolded/gathering-assets");
    let mut config = AssetConfig::load_at(settings.clone(), default.clone());
    config.save_root(&dir.path().join("prior")).unwrap();
    let original = fs::read(&settings).unwrap();
    let save = dir.path().join("attempt.json");
    fs::write(&save, b"personal save").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_arkham-assets"))
        .args(["import-gathering"])
        .arg(&zip)
        .env("XDG_DATA_HOME", &data)
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .stdout(Stdio::null())
        .stderr(fs::File::create(dir.path().join("child.log")).unwrap())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut extracting = false;
    while Instant::now() < deadline {
        extracting = fs::read_dir(config.installation_directory())
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.path().join("collection/manifest.json").is_file());
        if extracting || child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let _ = child.kill();
    let result = child.wait().unwrap();
    assert!(
        extracting && !result.success(),
        "import was not interrupted during extraction"
    );
    assert_eq!(fs::read(&settings).unwrap(), original);
    assert_eq!(fs::read(&save).unwrap(), b"personal save");
    let mut restarted = AssetConfig::load_at(settings, default);
    assert!(
        arkham_unfolded_experiment::assets::package::installation_status(&restarted)
            .contains("unselected")
    );
    let recovered = import_gathering(&zip, &mut restarted).unwrap();
    assert!(inspect_gathering(&recovered).complete);
    assert_eq!(fs::read(save).unwrap(), b"personal save");
}

// Explicit private-artwork exercise; ordinary CI remains independent of artwork.
#[test]
#[ignore = "requires ARKHAM_PACKAGE_SOURCE pointing to the real current collection"]
fn real_zip_import_survives_restart_without_touching_prior_collection_or_save() {
    let source = PathBuf::from(std::env::var_os("ARKHAM_PACKAGE_SOURCE").expect("real collection"));
    assert!(inspect_gathering(&source).complete);
    let dir = tempfile::tempdir().unwrap();
    let zip = dir.path().join("gathering-v7.zip");
    let manifest = fs::read(source.join("manifest.json")).unwrap();
    let save = dir.path().join("attempt.json");
    fs::write(&save, b"existing personal save").unwrap();
    let settings = dir.path().join("config/gathering.toml");
    let default = dir.path().join("data/gathering-assets");
    let mut config = AssetConfig::load_at(settings.clone(), default.clone());
    config.save_root(&source).unwrap();
    let previous_setting = fs::read(&settings).unwrap();
    package_gathering(&source, &zip).unwrap();
    let zip_before = fs::read(&zip).unwrap();
    let installed = import_gathering(&zip, &mut config).unwrap();
    assert!(inspect_gathering(&installed).complete);
    assert_ne!(installed, source);
    assert_eq!(
        AssetConfig::load_at(settings.clone(), default.clone()).root(),
        installed
    );
    assert_eq!(fs::read(source.join("manifest.json")).unwrap(), manifest);
    assert_eq!(fs::read(&save).unwrap(), b"existing personal save");
    assert_eq!(fs::read(&zip).unwrap(), zip_before);

    // A valid runtime root can sit beside personal/source material. Production
    // packaging must select only the independently enumerated recipe members.
    fs::write(installed.join("personal-save.json"), b"do not package").unwrap();
    let exported = dir.path().join("exported.zip");
    package_gathering(&installed, &exported).unwrap();
    let archive = zip::ZipArchive::new(fs::File::open(&exported).unwrap()).unwrap();
    let names: std::collections::BTreeSet<_> = archive
        .file_names()
        .map(|name| name.unwrap().into_owned())
        .collect();
    let manifest = required_manifest();
    let expected: std::collections::BTreeSet<_> = manifest["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["path"].as_str().unwrap().to_owned())
        .chain(std::iter::once("manifest.json".into()))
        .collect();
    assert_eq!(names, expected);
    let unrelated = dir.path().join("unrelated.zip");
    fs::write(&unrelated, b"retain unrelated output").unwrap();
    assert!(package_gathering(&installed, &unrelated).is_err());
    assert_eq!(fs::read(unrelated).unwrap(), b"retain unrelated output");

    // The state after interrupted activation: complete retained installation,
    // but the old preference is still selected. Restart must disclose it.
    fs::write(&settings, previous_setting).unwrap();
    let restarted = AssetConfig::load_at(settings, default);
    assert!(
        arkham_unfolded_experiment::assets::package::installation_status(&restarted)
            .contains("1 unselected")
    );
    assert!(installed.is_dir());
}
