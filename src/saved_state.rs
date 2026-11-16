//! Current-state files: Arkham's adapter onto the shared save record.
//!
//! `unfolded_record_replacement::save` owns the envelope, its checksum, the write
//! baseline, the unchanged shortcut and validation of the previous record. This
//! file keeps what identifies a compatible save, the session's validity check,
//! when a write is exclusive (a new attempt) or archives the current record
//! (restart), the size limit and the wording of Arkham's refusals, busy and too
//! large included. File-system failures keep the operating system's words.
use crate::{APP_VERSION, RuleError};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fmt,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Mutex,
};
use unfolded_record_replacement::save::{self, Format, Identity, Write};

const FORMAT_VERSION: &str = "3";
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The shared baseline, behind a lock so a save through `&Session` can update it.
#[derive(Debug, Default)]
pub(crate) struct Baseline(Mutex<save::Baseline>);
impl Clone for Baseline {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.get()))
    }
}
// File custody is not rules state and must not affect state equality.
impl PartialEq for Baseline {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for Baseline {}
impl Baseline {
    fn get(&self) -> save::Baseline {
        self.0.lock().expect("Save baseline lock").clone()
    }
    fn set(&self, baseline: save::Baseline) {
        *self.0.lock().expect("Save baseline lock") = baseline;
    }
}

pub(crate) trait Snapshot: Serialize + DeserializeOwned {
    const KIND: &'static str;
    const SOURCE: &'static str;
    fn baseline(&self) -> &Baseline;
    fn validate(&self) -> Result<(), RuleError>;
}

#[derive(Debug)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
/// The shared record's refusals, in Arkham's words where Arkham has its own.
impl From<unfolded_record_replacement::Error> for Error {
    fn from(error: unfolded_record_replacement::Error) -> Self {
        use unfolded_record_replacement::Error as Record;
        Self(match error {
            Record::TooLarge => TOO_LARGE.into(),
            Record::Busy(cause) => busy(&cause),
            other => other.to_string(),
        })
    }
}

/// Arkham's words for a record over its limit.
const TOO_LARGE: &str = "This file exceeds the bounded experiment's record limit.";

/// Arkham's words for a record another writer holds.
fn busy(cause: &std::io::Error) -> String {
    format!("The save is busy: {cause}")
}

struct Saved<T>(PhantomData<T>);
impl<T: Snapshot> Saved<T> {
    const FORMAT: Self = Self(PhantomData);
    fn current() -> Identity {
        [
            ("format", FORMAT_VERSION),
            ("kind", T::KIND),
            ("source", T::SOURCE),
            ("application", APP_VERSION),
        ]
        .into_iter()
        .collect()
    }
}
impl<T: Snapshot> Format for Saved<T> {
    type Payload = T;
    const LIMIT: u64 = MAX_BYTES;
    fn identity(&self, _: &T) -> Identity {
        Self::current()
    }
    fn accepts(&self, identity: &Identity) -> bool {
        *identity == Self::current()
    }
    // An intact record admits only when this build reproduces it exactly.
    fn validate(&self, session: &T) -> Result<(), String> {
        session.validate().map_err(|error| error.to_string())
    }
}

/// What the player was doing when a save was refused, for its wording.
#[derive(Clone, Copy)]
enum Action {
    Continue,
    Restart,
}

fn refusal(error: save::Error, action: Action) -> Error {
    let again = match action {
        Action::Restart => "restarting",
        Action::Continue => "continuing",
    };
    Error(match error {
        save::Error::Unsupported(identity) => {
            let recorded = identity.map_or_else(
                || "an earlier save format".to_owned(),
                |identity| {
                    let value = |name| identity.get(name).unwrap_or("none");
                    format!(
                        "format {}, kind {}, source {}, application {}",
                        value("format"),
                        value("kind"),
                        value("source"),
                        value("application")
                    )
                },
            );
            format!(
                "Unsupported save ({recorded}). Records in that format are unsupported by this build, and development builds promise no save compatibility. Start a New attempt at a different save path. The original file was left unchanged."
            )
        }
        save::Error::Malformed(detail) => format!("Invalid save: {detail}"),
        save::Error::Checksum => {
            "Save checksum mismatch. The original file was left unchanged.".into()
        }
        save::Error::Unreproducible(detail) => format!(
            "This build cannot reproduce the game recorded in this save ({detail}). Development builds promise no save compatibility: select New attempt to begin again. The original file was left unchanged."
        ),
        save::Error::Invalid(detail) => detail,
        save::Error::Stale => {
            format!("A different or later decision is already saved. Reopen it before {again}.")
        }
        save::Error::Missing => format!("The saved attempt is missing. Save it before {again}."),
        save::Error::Occupied => {
            "The new attempt path is already occupied. No existing attempt was opened or replaced."
                .into()
        }
        save::Error::Orphaned => {
            "A previous attempt is kept at this path without its active save. Start the New attempt at a different save path. No record was changed."
                .into()
        }
        save::Error::NotRegularFile => "The record destination is not a regular file.".into(),
        save::Error::TooLarge => TOO_LARGE.into(),
        save::Error::Busy(cause) => busy(&cause),
        other => other.to_string(),
    })
}

pub(crate) fn load<T: Snapshot>(path: impl AsRef<Path>) -> Result<T, Error> {
    let (session, baseline) = save::load(&Saved::<T>::FORMAT, path.as_ref())
        .map_err(|error| refusal(error, Action::Continue))?
        // The words the operating system gives a missing file, as before.
        .ok_or_else(|| Error::from(std::io::Error::from_raw_os_error(2)))?;
    session.baseline().set(baseline);
    Ok(session)
}
pub(crate) fn save<T: Snapshot>(path: impl AsRef<Path>, session: &T) -> Result<(), Error> {
    write(
        path.as_ref(),
        session,
        session,
        Write::Save,
        Action::Continue,
    )
    .map(|_| ())
}

#[cfg(feature = "tabletop")]
pub(crate) fn create<T: Snapshot>(path: &Path, session: &T) -> Result<(), Error> {
    write(path, session, session, Write::Create, Action::Continue).map(|_| ())
}

/// Write `session` under `owner`'s baseline, which then becomes `session`'s.
fn write<T: Snapshot>(
    path: &Path,
    owner: &T,
    session: &T,
    mode: Write,
    action: Action,
) -> Result<Option<PathBuf>, Error> {
    let mut baseline = owner.baseline().get();
    let archived = save::write(&Saved::<T>::FORMAT, path, &mut baseline, session, mode)
        .map_err(|error| refusal(error, action))?;
    session.baseline().set(baseline);
    Ok(archived)
}

pub(crate) fn restart<T: Snapshot>(
    path: impl AsRef<Path>,
    expected: &T,
    fresh: &T,
) -> Result<PathBuf, Error> {
    // `expected` must be the session its baseline saw, not a later one.
    let baseline = expected.baseline().get();
    if baseline != save::Baseline::default()
        && !baseline
            .holds(&Saved::<T>::FORMAT, expected)
            .map_err(|error| refusal(error, Action::Restart))?
    {
        return Err(refusal(save::Error::Stale, Action::Restart));
    }
    write(
        path.as_ref(),
        expected,
        fresh,
        Write::Archive,
        Action::Restart,
    )?
    .ok_or_else(|| Error("Restart did not retain the previous attempt.".into()))
}
