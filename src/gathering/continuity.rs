//! # File ownership
//!
//! A successful load or save remembers the exact file bytes in the returned or
//! supplied Session. Replacing different contents requires that write baseline
//! to still match disk. Saving identical state also refreshes the baseline.
//! Cloning copies the baseline at that moment; saving a clone does not update
//! the original. Save the instance that will continue play, or load it back.
//! Session equality compares rules state, not file ownership. Failed writes
//! preserve the baseline; `commit_choice` installs its candidate only after saving.
//! Current-state persistence. Historical saves and recordings are unsupported.
use super::{SOURCE_REVISION, Session};
pub use crate::saved_state::Error as ContinuityError;
use crate::saved_state::{self, Baseline, Snapshot};
use std::path::{Path, PathBuf};

impl Snapshot for Session {
    const KIND: &'static str = "gathering-session";
    const SOURCE: &'static str = SOURCE_REVISION;
    fn baseline(&self) -> &Baseline {
        &self.baseline
    }
    fn validate(&self) -> Result<(), crate::RuleError> {
        Session::validate(self)
    }
}
/// Save current state and update only this Session's [write baseline](crate::gathering::continuity#file-ownership).
pub fn save_session(path: impl AsRef<Path>, session: &Session) -> Result<(), ContinuityError> {
    saved_state::save(path, session)
}
pub fn load_session(path: impl AsRef<Path>) -> Result<Session, ContinuityError> {
    saved_state::load(path)
}
pub fn previous_path(path: impl AsRef<Path>) -> PathBuf {
    crate::record_replacement::previous_path(path.as_ref())
}
/// A save failure preserves every live choice, card and unrevealed random outcome.
pub fn commit_choice(
    path: impl AsRef<Path>,
    session: &mut Session,
    revision: u64,
    id: &str,
) -> Result<(), ContinuityError> {
    commit_choice_with_draws(path, session, revision, id).map(|_| ())
}

pub fn commit_choice_with_draws(
    path: impl AsRef<Path>,
    session: &mut Session,
    revision: u64,
    id: &str,
) -> Result<Vec<super::moments::DrawCue>, ContinuityError> {
    let mut candidate = session.clone();
    let draws = candidate
        .choose_with_draws(revision, id)
        .map_err(|error| ContinuityError(error.to_string()))?;
    save_session(path, &candidate)?;
    *session = candidate;
    Ok(draws)
}

/// Archive `expected` and save a fresh `session`, updating only the fresh instance's baseline.
/// `expected` must own the current file under the [file ownership contract](crate::gathering::continuity#file-ownership).
pub fn restart_session(
    path: impl AsRef<Path>,
    expected: &Session,
    session: &Session,
) -> Result<PathBuf, ContinuityError> {
    if !session.is_fresh() || session.is_verification_fixture() {
        return Err(ContinuityError(
            "Restart requires a fresh Gathering setup.".into(),
        ));
    }
    if session.source != expected.source {
        return Err(ContinuityError(
            "Restart must preserve the selected decks and rules.".into(),
        ));
    }
    saved_state::restart(path, expected, session)
}
