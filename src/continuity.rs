//! Current-state persistence. Historical saves and recordings are unsupported.
//!
//! # File ownership
//!
//! A successful load or save remembers the exact file bytes in the returned or
//! supplied Session. Replacing different contents requires that write baseline
//! to still match disk. Saving identical state also refreshes the baseline.
//! Cloning copies the baseline at that moment; saving a clone does not update
//! the original. Save the instance that will continue play, or load it back.
//! Session equality compares rules state, not file ownership. Failed writes
//! preserve the baseline; `commit_choice` installs its candidate only after saving.
use super::{SOURCE_REVISION, Session};
pub use crate::saved_state::Error as ContinuityError;
use crate::saved_state::{self, Baseline, Snapshot};
use std::path::{Path, PathBuf};

use crate::{Fixture, Stage, TokenStatus};
impl Snapshot for Session {
    const KIND: &'static str = "session";
    const SOURCE: &'static str = SOURCE_REVISION;
    fn baseline(&self) -> &Baseline {
        &self.baseline
    }
    fn validate(&self) -> Result<(), crate::RuleError> {
        validate_session(self)
    }
}
fn validate_session(session: &Session) -> Result<(), crate::RuleError> {
    // Check bounds used by observation/choice construction before invoking them.
    // This is a bounded save-format check, not a second rules implementation.
    let order = &session.fixture.order;
    if (*order != Fixture::curse().order && *order != Fixture::auto_fail().order)
        || (session.stage == Stage::Commit && !session.fixture.patrice_here)
        || session.order_cursor > order.len()
        || session.tokens.len() > 4
        || session
            .tokens
            .iter()
            .enumerate()
            .any(|(id, token)| token.id as usize != id)
        || session.group.len() > 3
        || session.group.iter().enumerate().any(|(index, &id)| {
            id as usize >= session.tokens.len() || session.group[..index].contains(&id)
        })
        || (session.stage == Stage::Cancellation
            && (session.group.len() != 3
                || session
                    .group
                    .iter()
                    .any(|&id| session.tokens[id as usize].status != TokenStatus::Pending)))
        || session.bag.len() > 17
        || !(9..=10).contains(&session.curses_in_pool)
        || session.extra_draws > 1
        || session.horror_dealt > 3
        || !(-12..=1).contains(&session.modifier)
        || session.revision >= 64
    {
        return Err(crate::RuleError(
            "The saved session has an invalid shape.".into(),
        ));
    }
    Ok(())
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
    if session != &Session::new(session.fixture.clone()) {
        return Err(ContinuityError("Restart requires a fresh example.".into()));
    }
    saved_state::restart(path, expected, session).map(|_| ())
}
