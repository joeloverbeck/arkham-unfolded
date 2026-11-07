use arkham_unfolded_experiment::gathering::{Session, continuity};

#[test]
fn gathering_saves_the_live_attempt_without_a_lifetime_playback_tape() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("current.json");
    let mut session = Session::with_seed(89);
    continuity::save_session(&path, &session).unwrap();
    continuity::commit_choice(&path, &mut session, 0, "lead:tony").unwrap();
    let view = session.observe();
    let selection = view
        .choices
        .iter()
        .find(|choice| choice.id.starts_with("mulligan:"))
        .unwrap();
    continuity::commit_choice(&path, &mut session, view.revision, &selection.id).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for retired in ["moments", "presentation_moments"] {
        assert!(
            saved["payload"].get(retired).is_none(),
            "Save still retains {retired}"
        );
    }
    let mut reopened = continuity::load_session(&path).unwrap();
    assert_eq!(reopened.observe(), session.observe());
    let revision = reopened.observe().revision;
    continuity::commit_choice(&path, &mut reopened, revision, "undo").unwrap();
    assert!(
        reopened.observe().investigators[0]
            .selected_mulligan
            .is_empty()
    );
    let revision = reopened.observe().revision;
    continuity::commit_choice(&path, &mut reopened, revision, "confirm-mulligan").unwrap();
    assert_eq!(
        continuity::load_session(&path).unwrap().observe(),
        reopened.observe()
    );
}

macro_rules! current_save_contract {
    ($name:ident, $module:path, $fresh:expr, $choice:literal) => {
        mod $name {
            use $module as mode;
            #[test]
            fn saving_a_clone_does_not_authorize_the_original_to_replace_it() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("attempt.json");
                let mut original = $fresh;
                let before = original.observe();
                let mut saved = original.clone();
                mode::continuity::save_session(&path, &saved).unwrap();
                let bytes = std::fs::read(&path).unwrap();

                // Equal rules state does not give this unsaved instance write ownership.
                assert_eq!(original, saved);
                assert!(mode::continuity::commit_choice(&path, &mut original, 0, $choice).is_err());
                assert_eq!(original.observe(), before);
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
                assert!(!mode::continuity::previous_path(&path).exists());

                mode::continuity::commit_choice(&path, &mut saved, 0, $choice).unwrap();
                assert_eq!(mode::continuity::load_session(&path).unwrap(), saved);
            }

            #[test]
            fn current_save_is_compact_and_old_files_are_refused_without_replacement() {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join("attempt.json");
                let mut session = $fresh;
                mode::continuity::save_session(&path, &session).unwrap();
                mode::continuity::commit_choice(&path, &mut session, 0, $choice).unwrap();
                let current: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                assert_eq!(current["format_version"], 3);
                for retired in ["moments", "presentation_moments", "transition", "baseline"] {
                    assert!(current["payload"].get(retired).is_none(), "Retired or transient field: {retired}");
                }
                assert_eq!(mode::continuity::load_session(&path).unwrap(), session);
                let previous = std::fs::read(mode::continuity::previous_path(&path)).unwrap();
                for version in [1, 2] {
                    let mut obsolete = current.clone();
                    obsolete["format_version"] = version.into();
                    // The format refusal precedes payload decoding and checksumming.
                    obsolete["payload"] = serde_json::json!({"moments": ["obsolete observation"]});
                    let bytes = serde_json::to_vec(&obsolete).unwrap();
                    std::fs::write(&path, &bytes).unwrap();
                    let error = mode::continuity::load_session(&path).unwrap_err().to_string();
                    assert!(error.starts_with("Unsupported save"), "{error}");
                    assert!(error.contains("different save path"));
                    assert!(mode::continuity::save_session(&path, &session).is_err());
                    let fresh = $fresh;
                    assert!(mode::continuity::restart_session(&path, &session, &fresh).is_err());
                    assert_eq!(std::fs::read(&path).unwrap(), bytes);
                    assert_eq!(std::fs::read(mode::continuity::previous_path(&path)).unwrap(), previous);
                }
            }
        }
    };
}
current_save_contract!(
    gathering,
    arkham_unfolded_experiment::gathering,
    mode::Session::with_seed(89),
    "lead:tony"
);
