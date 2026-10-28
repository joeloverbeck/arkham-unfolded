use arkham_unfolded_experiment::tony::{
    Session,
    continuity::{commit_choice, load_session, save_session},
};

#[test]
fn failed_save_preserves_the_pending_test_and_future_random_draw() {
    let directory = tempfile::tempdir().unwrap();
    let mut game = Session::new().unwrap();
    game.choose(0, "fight").unwrap();
    game.choose(1, "commit:none").unwrap();
    let before = game.clone();
    let blocked = directory.path().join("blocked");
    std::fs::write(&blocked, b"owner bytes").unwrap();
    assert!(commit_choice(blocked.join("session.json"), &mut game, 2, "reveal").is_err());
    assert_eq!(game, before);
    let path = directory.path().join("session.json");
    save_session(&path, &game).unwrap();
    commit_choice(&path, &mut game, 2, "reveal").unwrap();
    let mut control = before;
    control.choose(2, "reveal").unwrap();
    assert_eq!(game.observe(), control.observe());
    assert_eq!(load_session(&path).unwrap(), game);
    assert_eq!(std::fs::read(&blocked).unwrap(), b"owner bytes");
}

#[test]
fn undo_and_replay_keep_the_append_only_record_and_refuse_stale_saves() {
    use arkham_unfolded_experiment::tony::continuity::{Replay, load_recording, save_recording};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let record_path = directory.path().join("record.json");
    let mut game = Session::new().unwrap();
    save_session(&path, &game).unwrap();
    let stale = game.clone();
    commit_choice(&path, &mut game, 0, "injury").unwrap();
    let after_injury = game.observe();
    save_recording(&record_path, &game.recording()).unwrap();
    let exported = std::fs::read(&record_path).unwrap();
    commit_choice(&path, &mut game, 1, "undo").unwrap();
    let restored = game.observe();
    assert_eq!(
        (
            restored.ordinary_actions,
            restored.damage,
            restored.horror,
            restored.injury
        ),
        (3, 0, 0, true)
    );
    assert_eq!(restored.revision, 2);
    assert_eq!(game.recording().moments[1], after_injury);
    assert!(save_session(&path, &stale).is_err());
    assert!(save_recording(&record_path, &game.recording()).is_err());
    assert_eq!(std::fs::read(&record_path).unwrap(), exported);
    let mut replay = Replay::new(load_recording(&record_path).unwrap()).unwrap();
    assert!(replay.next());
    assert_eq!(replay.current(), &after_injury);
    assert!(replay.back());
    assert_eq!(load_session(&path).unwrap().observe(), restored);
}

#[test]
fn restart_archives_previous_run_and_bad_records_preserve_their_bytes() {
    use arkham_unfolded_experiment::tony::continuity::restart_session;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut old = Session::new().unwrap();
    old.choose(0, "injury").unwrap();
    save_session(&path, &old).unwrap();
    let fresh = Session::new().unwrap();
    restart_session(&path, &old, &fresh).unwrap();
    let archives: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".archived-")
        })
        .collect();
    assert_eq!(archives.len(), 1);
    assert_eq!(load_session(&archives[0]).unwrap(), old);
    assert_eq!(load_session(&path).unwrap(), fresh);
    let original = std::fs::read(&path).unwrap();
    let mut envelope: serde_json::Value = serde_json::from_slice(&original).unwrap();
    envelope["source_revision"] = serde_json::json!("unsupported-source");
    let bad = serde_json::to_vec(&envelope).unwrap();
    std::fs::write(&path, &bad).unwrap();
    assert!(load_session(&path).is_err());
    assert!(save_session(&path, &fresh).is_err());
    assert!(restart_session(&path, &old, &Session::new().unwrap()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bad);
}

#[test]
fn baseline_jacqueline_records_reopen_only_through_the_retained_compatible_mode() {
    use arkham_unfolded_experiment::continuity as legacy;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("legacy.json");
    let original = include_bytes!("fixtures/result-explanation/early.session.json");
    std::fs::write(&path, original).unwrap();
    let session = legacy::load_session(&path).unwrap();
    assert!(legacy::Replay::new(session.recording()).is_ok());
    assert!(load_session(&path).is_err());
    assert!(save_session(&path, &Session::new().unwrap()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn malformed_or_incompatible_records_with_valid_checksums_are_refused_without_mutation() {
    use sha2::{Digest, Sha256};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    save_session(&path, &Session::new().unwrap()).unwrap();
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for (pointer, value) in [
        ("/payload/state/bag", serde_json::json!([])),
        ("/payload/state/clues", serde_json::json!(255)),
        ("/source_revision", serde_json::json!("unsupported-source")),
    ] {
        let mut invalid = original.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        let checksum = Sha256::digest(
            serde_json::to_vec(&serde_json::json!([
                invalid["format_version"],
                invalid["kind"],
                invalid["source_revision"],
                invalid["app_version"],
                invalid["payload"]
            ]))
            .unwrap(),
        );
        invalid["checksum"] = serde_json::json!(format!("{checksum:x}"));
        let bytes = serde_json::to_vec(&invalid).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        assert!(load_session(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn stale_restart_cannot_replace_a_newer_active_turn() {
    use arkham_unfolded_experiment::tony::continuity::restart_session;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turn.json");
    let mut old = Session::new().unwrap();
    old.choose(0, "end_turn").unwrap();
    save_session(&path, &old).unwrap();
    let mut current = Session::new().unwrap();
    restart_session(&path, &old, &current).unwrap();
    commit_choice(&path, &mut current, 0, "injury").unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(restart_session(&path, &old, &Session::new().unwrap()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(load_session(&path).unwrap(), current);
}
