use arkham_unfolded_experiment::gathering::{
    Session,
    continuity::{commit_choice, load_session, previous_path, restart_session, save_session},
};

#[test]
fn restart_retains_exact_bytes_after_new_attempt_autosaves() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gathering.json");
    let mut old = Session::with_seed(89);
    commit_choice(&path, &mut old, 0, "lead:tony").unwrap();
    // Valid whitespace is part of the original file custody, not its semantic identity.
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let original = serde_json::to_vec_pretty(&value).unwrap();
    std::fs::write(&path, &original).unwrap();
    old = load_session(&path).unwrap();
    let mut fresh = Session::with_seed(90);
    let archive = restart_session(&path, &old, &fresh).unwrap();
    assert_eq!(load_session(&path).unwrap(), fresh);
    assert_eq!(fresh.observe().source, old.observe().source);
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    assert_eq!(std::fs::read(previous_path(&path)).unwrap(), original);
    commit_choice(&path, &mut fresh, 0, "lead:patrice").unwrap();
    commit_choice(&path, &mut fresh, 1, "confirm-mulligan").unwrap();
    assert_ne!(std::fs::read(previous_path(&path)).unwrap(), original);
    assert_eq!(std::fs::read(&archive).unwrap(), original);
    let recovered = load_session(&archive).unwrap();
    assert_eq!(recovered, old);
    let second = restart_session(&path, &fresh, &Session::with_seed(91)).unwrap();
    assert_ne!(second, archive);
    assert_eq!(load_session(second).unwrap(), fresh);
    assert_eq!(std::fs::read(archive).unwrap(), original);
}

#[test]
fn restart_refuses_stale_missing_and_nonfresh_inputs_without_replacing_progress() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gathering.json");
    let initial = Session::with_seed(89);
    let mut current = initial.clone();
    commit_choice(&path, &mut current, 0, "lead:tony").unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let fresh = Session::with_seed(90);

    let error = restart_session(&path, &initial, &fresh).unwrap_err();
    assert!(error.to_string().contains("later decision"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let error = restart_session(&path, &current, &current).unwrap_err();
    assert!(error.to_string().contains("fresh Gathering setup"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let missing = directory.path().join("missing.json");
    assert!(
        restart_session(&missing, &current, &fresh)
            .unwrap_err()
            .to_string()
            .contains("missing")
    );
    assert!(!missing.exists());
    assert_eq!(load_session(&path).unwrap(), current);
    assert!(!std::fs::read_dir(directory.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".archived-")
    }));
}

#[test]
fn restart_refuses_a_busy_save_and_invalid_recovery_without_losing_the_current_attempt() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gathering.json");
    let current = Session::with_seed(89);
    save_session(&path, &current).unwrap();
    let original = std::fs::read(&path).unwrap();
    let fresh = Session::with_seed(90);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.path().join("gathering.json.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert!(
        restart_session(&path, &current, &fresh)
            .unwrap_err()
            .to_string()
            .contains("busy")
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    drop(lock);

    std::fs::write(previous_path(&path), b"invalid prior recovery").unwrap();
    assert!(restart_session(&path, &current, &fresh).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(load_session(&path).unwrap(), current);
    assert_eq!(
        std::fs::read(previous_path(&path)).unwrap(),
        b"invalid prior recovery"
    );
}
