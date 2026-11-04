//! Current-state persistence. Historical saves and recordings are unsupported.
use super::{SOURCE_REVISION, Session};
pub use crate::saved_state::Error as ContinuityError;
use crate::saved_state::{self, Baseline, Snapshot};
use std::path::{Path, PathBuf};

impl Snapshot for Session {
    const KIND: &'static str = "party-session";
    const SOURCE: &'static str = SOURCE_REVISION;
    fn baseline(&self) -> &Baseline {
        &self.baseline
    }
    fn validate(&self) -> Result<(), crate::RuleError> {
        Session::validate(self)
    }
}
/// Save current state and update only this Session's [write baseline](crate::continuity#file-ownership).
pub fn save_session(path: impl AsRef<Path>, session: &Session) -> Result<(), ContinuityError> {
    saved_state::save(path, session)
}
pub fn load_session(path: impl AsRef<Path>) -> Result<Session, ContinuityError> {
    saved_state::load(path)
}
pub fn previous_path(path: impl AsRef<Path>) -> PathBuf {
    crate::record_replacement::previous_path(path.as_ref())
}
pub fn commit_choice(
    path: impl AsRef<Path>,
    session: &mut Session,
    revision: u64,
    id: &str,
) -> Result<(), ContinuityError> {
    let mut candidate = session.clone();
    candidate
        .choose(revision, id)
        .map_err(|error| ContinuityError(error.to_string()))?;
    save_session(path, &candidate)?;
    *session = candidate;
    Ok(())
}
/// Archive `expected` and save a fresh `session`, updating only the fresh instance's baseline.
/// `expected` must own the current file under the [file ownership contract](crate::continuity#file-ownership).
pub fn restart_session(
    path: impl AsRef<Path>,
    expected: &Session,
    session: &Session,
) -> Result<(), ContinuityError> {
    if !session.is_fresh() {
        return Err(ContinuityError("Restart requires a fresh example.".into()));
    }
    saved_state::restart(path, expected, session).map(|_| ())
}
