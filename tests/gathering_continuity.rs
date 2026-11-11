use arkham_unfolded_experiment::gathering::{
    Session,
    continuity::{commit_choice, load_session, previous_path, save_session},
};

fn first_choice(game: &Session) -> (u64, String) {
    let view = game.observe();
    (view.revision, view.choices[0].id.clone())
}

#[test]
fn incompatible_source_is_identified_before_decoding_a_session_or_recording_payload() {
    use sha2::{Digest, Sha256};
    let directory = tempfile::tempdir().unwrap();
    for (source, _reader) in [
        (
            "c35be2c:issue-89-gathering-setup-v1",
            "7b0113275a0ae1bdb57e7f24467e6d3ed6af68d3",
        ),
        (
            "c35be2c:issue-89-gathering-runtime-v2",
            "f431de9af84bedf20e0024370b9e6aaa8819a1ef",
        ),
        (
            "c35be2c:issue-89-gathering-runtime-v3",
            "46f4c54a018802d22889a238629bcea27871d259",
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
            let error = load_session(&path).unwrap_err().to_string();
            assert!(error.contains("unsupported"), "{error}");
            assert!(!error.contains("Invalid record"), "{error}");
            assert!(save_session(&path, &Session::with_seed(89)).is_err());
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
    let view = committed.observe();
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
        game.choose(game.observe().revision, id).unwrap();
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

/// Rewrite one saved payload and reseal it, as an earlier development build
/// would have written it. The checksum is intact, so only replay can refuse it.
fn reseal(path: &std::path::Path, edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    edit(&mut envelope["payload"]);
    let checked = serde_json::to_vec(&(
        &envelope["format_version"],
        &envelope["kind"],
        &envelope["source_revision"],
        &envelope["app_version"],
        &envelope["payload"],
    ))
    .unwrap();
    envelope["checksum"] = serde_json::json!(format!("{:x}", Sha256::digest(checked)));
    let bytes = serde_json::to_vec_pretty(&envelope).unwrap();
    std::fs::write(path, &bytes).unwrap();
    bytes
}

/// Append a decision the current build no longer offers, as if an earlier
/// build had journalled it at this exact position.
fn journal_removed_decision(payload: &mut serde_json::Value, id: &str) {
    let decisions = payload["decisions"].as_array_mut().unwrap();
    decisions.push(serde_json::json!(id));
    let index = decisions.len() - 1;
    payload["active_decisions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!(index));
    payload["revision"] = serde_json::json!(payload["revision"].as_u64().unwrap() + 1);
}

fn assert_refused_unchanged(path: &std::path::Path, bytes: &[u8], owner: &Session) {
    let error = load_session(path).unwrap_err().to_string();
    assert!(
        error.contains("Development builds promise no save compatibility"),
        "{error}"
    );
    assert!(error.contains("New attempt"), "{error}");
    assert!(!error.to_lowercase().contains("checksum"), "{error}");
    assert!(save_session(path, owner).is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert!(!previous_path(path).exists());
}

#[test]
fn earlier_build_records_that_no_longer_replay_are_refused_unchanged() {
    use arkham_unfolded_experiment::gathering::{HostileFixture, SearchFixture, TestCardFixture};
    let directory = tempfile::tempdir().unwrap();

    // Earlier builds journalled Kicking the Hornet's Nest's spawn without its
    // Then reward. That identifier is no longer accepted, even from a record.
    let mut hornet =
        Session::with_hostile_fixture(HostileFixture::Search(SearchFixture::HornetTargets));
    let view = hornet.observe();
    let card = view.play.unwrap().players[0]
        .hand
        .iter()
        .find(|card| card.code == "09068")
        .unwrap()
        .id;
    hornet
        .choose(view.revision, &format!("action:ordinary:play:tony:{card}"))
        .unwrap();
    let enemy = hornet
        .observe()
        .play
        .unwrap()
        .searched_encounter
        .iter()
        .find(|card| card.code == "01159")
        .unwrap()
        .id;
    let path = directory.path().join("hornet.json");
    save_session(&path, &hornet).unwrap();
    let bytes = reseal(&path, |payload| {
        journal_removed_decision(payload, &format!("search:enemy:{enemy}"));
    });
    assert_refused_unchanged(&path, &bytes, &hornet);

    // Earlier builds opened Crystal Pendulum's −/+ stepper as a rules step.
    let pendulum = Session::with_hostile_fixture(HostileFixture::TestCards(
        TestCardFixture::CrystalPrediction,
    ));
    let asset = pendulum.pendulum_prompts()[0].asset;
    let path = directory.path().join("pendulum.json");
    save_session(&path, &pendulum).unwrap();
    let bytes = reseal(&path, |payload| {
        journal_removed_decision(payload, &format!("test-card:pendulum:{asset}"));
    });
    assert_refused_unchanged(&path, &bytes, &pendulum);
}
