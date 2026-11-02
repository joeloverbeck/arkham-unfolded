use arkham_unfolded_experiment::gathering::{
    Session,
    continuity::{
        Replay, commit_choice, load_recording, load_session, previous_path, save_recording,
        save_session,
    },
};

fn first_choice(game: &Session) -> (u64, String) {
    let view = game.observe_for_play();
    (view.revision, view.choices[0].id.clone())
}

#[test]
fn incompatible_source_is_identified_before_decoding_a_session_or_recording_payload() {
    use sha2::{Digest, Sha256};
    let directory = tempfile::tempdir().unwrap();
    for (source, reader) in [
        (
            "c35be2c:issue-89-gathering-setup-v1",
            "7b0113275a0ae1bdb57e7f24467e6d3ed6af68d3",
        ),
        (
            "c35be2c:issue-89-gathering-runtime-v2",
            "f431de9af84bedf20e0024370b9e6aaa8819a1ef",
        ),
    ] {
        for kind in ["gathering-session", "gathering-recording"] {
            // This deliberately invalid payload tests header dispatch only. Genuine
            // historical7b producer/read/resume evidence is retained separately.
            let mut envelope = serde_json::json!({
                "format_version":1,"kind":kind,
                "source_revision":source,
                "app_version":"0.1.0",
                "payload":{"not_a_valid_session_or_recording":true},"checksum":""
            });
            let bytes = serde_json::to_vec(&(
                &envelope["format_version"],
                &envelope["kind"],
                &envelope["source_revision"],
                &envelope["app_version"],
                &envelope["payload"],
            ))
            .unwrap();
            envelope["checksum"] = serde_json::json!(format!("{:x}", Sha256::digest(bytes)));
            let path = directory.path().join(format!("{kind}.json"));
            let original = serde_json::to_vec_pretty(&envelope).unwrap();
            std::fs::write(&path, &original).unwrap();
            let error = if kind == "gathering-session" {
                load_session(&path).unwrap_err().to_string()
            } else {
                load_recording(&path).unwrap_err().to_string()
            };
            assert!(error.contains(source), "{error}");
            assert!(error.contains(reader), "{error}");
            assert!(
                error.contains("docs/experiments/gathering-setup.md"),
                "{error}"
            );
            assert!(!error.contains("Invalid record"), "{error}");
            if kind == "gathering-session" {
                assert!(save_session(&path, &Session::with_seed(89)).is_err());
            } else {
                assert!(save_recording(&path, &Session::with_seed(89).recording()).is_err());
            }
            assert_eq!(std::fs::read(path).unwrap(), original);
        }
    }
}

#[test]
fn committed_setup_choice_reopens_exactly_and_failed_save_keeps_live_randomness() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gathering.json");
    let unusable = directory.path().join("directory");
    std::fs::create_dir(&unusable).unwrap();
    let mut game = Session::with_seed(89);
    save_session(&path, &game).unwrap();
    let before = game.clone();
    let original = std::fs::read(&path).unwrap();
    let (revision, id) = first_choice(&game);

    assert!(commit_choice(&unusable, &mut game, revision, &id).is_err());
    assert_eq!(game, before);
    assert_eq!(std::fs::read(&path).unwrap(), original);

    commit_choice(&path, &mut game, revision, &id).unwrap();
    assert_eq!(load_session(&path).unwrap(), game);
    assert_eq!(load_session(previous_path(&path)).unwrap(), before);
    let committed = game.clone();
    assert!(commit_choice(&path, &mut game, revision, &id).is_err());
    assert_eq!(game, committed);
    assert_eq!(load_session(&path).unwrap(), committed);
}

#[test]
fn stale_and_divergent_setup_decisions_cannot_replace_saved_progress() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gathering.json");
    let initial = Session::with_seed(89);
    save_session(&path, &initial).unwrap();
    let mut committed = initial.clone();
    let view = committed.observe_for_play();
    commit_choice(&path, &mut committed, view.revision, &view.choices[0].id).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(save_session(&path, &initial).is_err());

    let mut divergent = initial.clone();
    divergent
        .choose(view.revision, &view.choices[1].id)
        .unwrap();
    let (revision, id) = first_choice(&divergent);
    divergent.choose(revision, &id).unwrap();
    assert!(save_session(&path, &divergent).is_err());
    assert!(save_session(&path, &Session::with_seed(90)).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    save_session(&path, &committed).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn gathering_records_remain_separate_and_replay_does_not_repeat_setup() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("gathering.json");
    let recording_path = directory.path().join("gathering.recording.json");
    let mut game = Session::with_seed(89);
    let initial = game.observe_for_play();
    let (revision, id) = first_choice(&game);
    commit_choice(&path, &mut game, revision, &id).unwrap();
    let current = game.observe_for_play();
    save_recording(&recording_path, &game.recording()).unwrap();
    let original = std::fs::read(&recording_path).unwrap();
    let mut replay = Replay::new(load_recording(&recording_path).unwrap()).unwrap();
    assert_eq!(replay.current(), &initial);
    assert!(replay.next());
    assert_eq!(replay.current(), &current);
    assert!(!replay.next());
    assert!(replay.back());
    assert_eq!(replay.current(), &initial);
    assert_eq!(load_session(&path).unwrap().observe_for_play(), current);

    assert!(arkham_unfolded_experiment::party::continuity::load_session(&path).is_err());
    assert!(load_recording(&path).is_err());
    assert!(load_session(&recording_path).is_err());
    assert!(save_recording(&recording_path, &Session::with_seed(90).recording()).is_err());
    assert_eq!(std::fs::read(&recording_path).unwrap(), original);
}

#[test]
fn same_total_encounter_substitution_is_refused_even_with_a_matching_checksum() {
    use sha2::{Digest, Sha256};

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("setup.json");
    let mut game = Session::with_seed(89);
    for id in [
        "lead:tony",
        "confirm-mulligan",
        "confirm-mulligan",
        "confirm-mulligan",
    ] {
        game.choose(game.observe_for_play().revision, id).unwrap();
    }
    save_session(&path, &game).unwrap();
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let deck = envelope["payload"]["encounter_deck"]
        .as_array_mut()
        .unwrap();
    assert_eq!(deck.len(), 26);
    // The original inventory has three Ancient Evils and three Rats. Replacing
    // one Evil with a fourth Rat preserves the total but violates that inventory.
    *deck.iter_mut().find(|card| **card == "01166").unwrap() = serde_json::json!("01159");
    let checked = serde_json::to_vec(&(
        &envelope["format_version"],
        &envelope["kind"],
        &envelope["source_revision"],
        &envelope["app_version"],
        &envelope["payload"],
    ))
    .unwrap();
    envelope["checksum"] = serde_json::json!(format!("{:x}", Sha256::digest(checked)));
    let altered = serde_json::to_vec_pretty(&envelope).unwrap();
    std::fs::write(&path, &altered).unwrap();
    let error = load_session(&path).unwrap_err().to_string();
    assert!(!error.to_lowercase().contains("checksum"), "{error}");
    assert!(save_session(&path, &game).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), altered);
}
