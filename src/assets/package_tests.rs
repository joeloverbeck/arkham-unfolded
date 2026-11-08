use super::*;

#[test]
#[ignore = "requires ARKHAM_PACKAGE_ZIP pointing to the verified real runtime ZIP"]
fn replacement_retains_prior_until_admission_and_refuses_in_use_or_drifted_cleanup() {
    let zip = PathBuf::from(std::env::var_os("ARKHAM_PACKAGE_ZIP").expect("real runtime ZIP"));
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("config/settings");
    let default = dir.path().join("data/assets");
    let mut config = AssetConfig::load_at(settings.clone(), default.clone());
    let prior = import_gathering(&zip, &mut config).unwrap();
    let lease = acquire_play(&config, &prior).unwrap();
    assert!(
        import_gathering(&zip, &mut config)
            .unwrap_err()
            .contains("Close active play")
    );
    drop(lease);
    // A damaged managed installation requires replacement; it must still be
    // retained through replacement failure, unlike an idempotent healthy import.
    fs::write(prior.join("manifest.json"), b"damaged previous manifest").unwrap();
    let next = import_gathering(&zip, &mut config).unwrap();
    assert_ne!(next, prior);
    assert_eq!(
        fs::read(prior.join("manifest.json")).unwrap(),
        b"damaged previous manifest"
    );
    assert!(!read_receipt(next.parent().unwrap()).unwrap().admitted);
    let restarted = AssetConfig::load_at(settings, default);
    assert_eq!(restarted.root(), next);
    assert!(inspect_gathering(&next).complete);
    // Decoder/renderer failure does not call finish_admission. Restart alone
    // must retain the prior directory and its exact bytes.
    assert!(prior.is_dir());
    let usage = usage_file(&prior).unwrap();
    usage.try_lock_shared().unwrap();
    assert!(
        finish_admission(&next, &restarted)
            .unwrap_err()
            .contains("still in use")
    );
    assert!(prior.exists());
    drop(usage);
    fs::write(prior.join("unexpected"), b"new owner data").unwrap();
    assert!(
        finish_admission(&next, &restarted)
            .unwrap_err()
            .contains("changed")
    );
    assert!(prior.join("unexpected").exists());
    fs::remove_file(prior.join("unexpected")).unwrap();
    finish_admission(&next, &restarted).unwrap();
    assert!(!prior.parent().unwrap().exists());
    assert!(read_receipt(next.parent().unwrap()).unwrap().admitted);
    finish_admission(&next, &restarted).unwrap();
    assert!(inspect_gathering(&next).complete);
}

#[test]
#[ignore = "requires ARKHAM_PACKAGE_ZIP pointing to the verified real runtime ZIP"]
fn stale_settings_and_interrupted_activation_preserve_prior_and_recover() {
    let zip = PathBuf::from(std::env::var_os("ARKHAM_PACKAGE_ZIP").expect("real runtime ZIP"));
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("config/settings");
    let default = dir.path().join("data/assets");
    let mut config = AssetConfig::load_at(settings.clone(), default.clone());
    let old = import_gathering(&zip, &mut config).unwrap();
    let save = dir.path().join("save.json");
    fs::write(&save, b"personal save").unwrap();
    let mut other = AssetConfig::load_at(settings.clone(), default.clone());
    other.save_root(&dir.path().join("external")).unwrap();
    let changed_settings = fs::read(&settings).unwrap();
    assert!(
        import_gathering(&zip, &mut config)
            .unwrap_err()
            .contains("settings changed")
    );
    assert!(acquire_play(&config, &old).is_err());
    assert_eq!(fs::read(&settings).unwrap(), changed_settings);
    // Abrupt extraction can leave a partial app-owned stage; never select it.
    let partial = config
        .installation_directory()
        .join("installation-interrupted");
    fs::create_dir(&partial).unwrap();
    fs::write(partial.join("partial"), b"unfinished").unwrap();
    assert!(installation_status(&other).contains("unselected"));
    let new = import_gathering(&zip, &mut other).unwrap();
    assert_ne!(new, partial);
    assert!(inspect_gathering(&new).complete);
    assert!(old.exists());
    assert_eq!(fs::read(partial.join("partial")).unwrap(), b"unfinished");
    assert_eq!(fs::read(save).unwrap(), b"personal save");
}
