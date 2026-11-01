//! Offline continuity for the prepared Study attempt. Earlier experiments retain their readers.

use super::{
    LEGACY_SOURCE_REVISION, Observation, Recording, SOURCE_REVISION, Session, Stage,
    supported_source_revision,
};
use crate::{
    APP_VERSION,
    record_replacement::{self, Replacement},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    path::{Path, PathBuf},
};

const FORMAT_VERSION: u32 = 1;
const MAX_RECORD_BYTES: u64 = 8 * 1024 * 1024;

pub struct Replay {
    recording: Recording,
    index: usize,
}

impl Replay {
    pub fn new(recording: Recording) -> Result<Self, ContinuityError> {
        validate_recording(&recording)?;
        Ok(Self {
            recording,
            index: 0,
        })
    }
    pub fn current(&self) -> &Observation {
        &self.recording.moments[self.index]
    }
    /// The original committed observations, never presentation-only views.
    pub fn recording(&self) -> &Recording {
        &self.recording
    }
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        if self.index + 1 >= self.len() {
            return false;
        }
        self.index += 1;
        true
    }
    pub fn back(&mut self) -> bool {
        if self.index == 0 {
            return false;
        }
        self.index -= 1;
        true
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn len(&self) -> usize {
        self.recording.moments.len()
    }
    pub fn is_empty(&self) -> bool {
        self.recording.moments.is_empty()
    }
}

fn validate_recording(recording: &Recording) -> Result<(), ContinuityError> {
    super::validate_recording(recording).map_err(|e| ContinuityError(e.to_string()))
}

fn validate_session(session: &Session) -> Result<(), ContinuityError> {
    session
        .validate()
        .map_err(|e| ContinuityError(e.to_string()))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format_version: u32,
    kind: String,
    source_revision: String,
    app_version: String,
    payload: serde_json::Value,
    checksum: String,
}

impl Envelope {
    fn checksum(&self) -> Result<String, ContinuityError> {
        let bytes = serde_json::to_vec(&(
            self.format_version,
            &self.kind,
            &self.source_revision,
            &self.app_version,
            &self.payload,
        ))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Debug)]
pub struct ContinuityError(pub String);

impl fmt::Display for ContinuityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for ContinuityError {}

impl From<std::io::Error> for ContinuityError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
impl From<serde_json::Error> for ContinuityError {
    fn from(error: serde_json::Error) -> Self {
        Self(format!("Invalid record: {error}"))
    }
}

pub fn save_session(path: impl AsRef<Path>, session: &Session) -> Result<(), ContinuityError> {
    write_session(path.as_ref(), session, None)
}

/// Apply and save a choice before installing it as the live session.
/// Refused choices never attempt persistence; any error leaves the entire live
/// session unchanged. Filesystem side effects follow `save_session` semantics.
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

/// Explicitly starts a fresh example at the active path, retaining the old run
/// in a unique sibling file in addition to the rotating `.previous` recovery.
pub fn restart_session(
    path: impl AsRef<Path>,
    expected: &Session,
    session: &Session,
) -> Result<(), ContinuityError> {
    if !session.is_fresh() {
        return Err(ContinuityError("Restart requires a fresh example.".into()));
    }
    write_session(path.as_ref(), session, Some(expected))
}

fn write_session(
    path: &Path,
    session: &Session,
    expected: Option<&Session>,
) -> Result<(), ContinuityError> {
    validate_session(session)?;
    let mut envelope = Envelope {
        format_version: FORMAT_VERSION,
        kind: "party-session".into(),
        source_revision: SOURCE_REVISION.into(),
        app_version: APP_VERSION.into(),
        payload: serde_json::to_value(session)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    replace_record(
        path,
        &serde_json::to_vec_pretty(&envelope)?,
        "party-session",
        Some(session),
        expected,
    )?;
    Ok(())
}

pub fn load_session(path: impl AsRef<Path>) -> Result<Session, ContinuityError> {
    let (_, envelope) = read_envelope(path.as_ref(), "party-session")?;
    decode_session(envelope)
}

fn decode_session(envelope: Envelope) -> Result<Session, ContinuityError> {
    let session: Session = serde_json::from_value(envelope.payload)?;
    if envelope.source_revision == LEGACY_SOURCE_REVISION
        && (session.state.stage == Stage::ActReveal
            || session
                .undo
                .iter()
                .any(|state| state.stage == Stage::ActReveal)
            || session
                .moments
                .iter()
                .any(|view| view.stage == Stage::ActReveal))
    {
        return Err(ContinuityError(
            "Act reveal requires the current party record version.".into(),
        ));
    }
    validate_session(&session)?;
    Ok(session)
}

pub fn save_recording(
    path: impl AsRef<Path>,
    recording: &Recording,
) -> Result<(), ContinuityError> {
    validate_recording(recording)?;
    let mut envelope = Envelope {
        format_version: FORMAT_VERSION,
        kind: "party-recording".into(),
        source_revision: recording.source_revision.clone(),
        app_version: recording.app_version.clone(),
        payload: serde_json::to_value(recording)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    replace_record(
        path.as_ref(),
        &serde_json::to_vec_pretty(&envelope)?,
        "party-recording",
        None,
        None,
    )
}

pub fn load_recording(path: impl AsRef<Path>) -> Result<Recording, ContinuityError> {
    let (_, envelope) = read_envelope(path.as_ref(), "party-recording")?;
    decode_recording(envelope)
}

fn decode_recording(envelope: Envelope) -> Result<Recording, ContinuityError> {
    let recording: Recording = serde_json::from_value(envelope.payload)?;
    if recording.source_revision != envelope.source_revision
        || recording.app_version != envelope.app_version
    {
        return Err(ContinuityError(
            "The recording and its envelope identify different versions.".into(),
        ));
    }
    validate_recording(&recording)?;
    Ok(recording)
}

fn read_envelope(path: &Path, kind: &str) -> Result<(Vec<u8>, Envelope), ContinuityError> {
    let bytes = record_replacement::read(path, MAX_RECORD_BYTES, ContinuityError)?;
    let envelope = decode_envelope(&bytes, kind)?;
    Ok((bytes, envelope))
}

fn decode_envelope(bytes: &[u8], kind: &str) -> Result<Envelope, ContinuityError> {
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.checksum != envelope.checksum()? {
        return Err(ContinuityError(
            "Record checksum mismatch. The original file was left unchanged.".into(),
        ));
    }
    if envelope.format_version != FORMAT_VERSION
        || envelope.kind != kind
        || !supported_source_revision(&envelope.source_revision)
        || envelope.app_version != APP_VERSION
    {
        return Err(ContinuityError(
            "This record requires a different compatible application/source version.".into(),
        ));
    }
    Ok(envelope)
}

/// The last valid save is retained next to the destination with `.previous` added.
pub fn previous_path(path: impl AsRef<Path>) -> PathBuf {
    record_replacement::previous_path(path.as_ref())
}

fn replace_record(
    path: &Path,
    bytes: &[u8],
    kind: &str,
    session: Option<&Session>,
    expected: Option<&Session>,
) -> Result<(), ContinuityError> {
    record_replacement::replace(
        path,
        bytes,
        MAX_RECORD_BYTES,
        ContinuityError,
        |existing| {
            let Some(existing) = existing else {
                if expected.is_some() {
                    return Err(ContinuityError("The previous turn is no longer at this path. Reopen before starting another turn.".into()));
                }
                return Ok(Replacement::Replace);
            };
            let envelope = decode_envelope(existing, kind)?;
            if kind == "party-recording" {
                let candidate: Envelope = serde_json::from_slice(bytes)?;
                if decode_recording(envelope)? != decode_recording(candidate)? {
                    return Err(ContinuityError("A different recording already occupies this path. Recordings are immutable; choose a new path.".into()));
                }
                return Ok(Replacement::Unchanged);
            }
            if let Some(candidate) = session {
                let committed = decode_session(envelope)?;
                if candidate == &committed {
                    return Ok(Replacement::Unchanged);
                }
                if let Some(expected) = expected {
                    if expected != &committed {
                        return Err(ContinuityError("A different or later turn is already saved. Reopen it before starting another turn.".into()));
                    }
                } else if !candidate.is_continuation_of(&committed) {
                    return Err(ContinuityError("A different or later decision is already saved. Reopen it before continuing.".into()));
                }
            }
            Ok(if expected.is_some() {
                Replacement::Archive
            } else {
                Replacement::Replace
            })
        },
        |previous| load_session(previous).map(|_| ()),
    )
}
