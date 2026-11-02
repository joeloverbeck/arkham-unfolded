//! Revision-bound saves for actual Gathering preparation. Experiment readers stay separate.

use super::{Observation, Recording, SOURCE_REVISION, Session};
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

fn validate_recording(recording: &Recording) -> Result<(), ContinuityError> {
    super::validate_recording(recording).map_err(|error| ContinuityError(error.to_string()))
}

fn envelope(kind: &str, payload: serde_json::Value) -> Result<Vec<u8>, ContinuityError> {
    let mut envelope = Envelope {
        format_version: FORMAT_VERSION,
        kind: kind.into(),
        source_revision: SOURCE_REVISION.into(),
        app_version: APP_VERSION.into(),
        payload,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    Ok(serde_json::to_vec_pretty(&envelope)?)
}

pub fn save_session(path: impl AsRef<Path>, session: &Session) -> Result<(), ContinuityError> {
    session
        .validate()
        .map_err(|error| ContinuityError(error.to_string()))?;
    let bytes = envelope("gathering-session", serde_json::to_value(session)?)?;
    record_replacement::replace(
        path.as_ref(),
        &bytes,
        MAX_RECORD_BYTES,
        ContinuityError,
        |existing| {
            let Some(existing) = existing else {
                return Ok(Replacement::Replace);
            };
            let committed = decode_session(decode_envelope(existing, "gathering-session")?)?;
            if session == &committed {
                return Ok(Replacement::Unchanged);
            }
            if !session.is_continuation_of(&committed) {
                return Err(ContinuityError("A different or later Gathering decision is already saved. Reopen it before continuing.".into()));
            }
            Ok(Replacement::Replace)
        },
        |previous| load_session(previous).map(|_| ()),
    )
}

/// A save failure preserves every live choice, card and unrevealed random outcome.
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

pub fn load_session(path: impl AsRef<Path>) -> Result<Session, ContinuityError> {
    decode_session(read_envelope(path.as_ref(), "gathering-session")?)
}

fn decode_session(envelope: Envelope) -> Result<Session, ContinuityError> {
    let session: Session = serde_json::from_value(envelope.payload)?;
    session
        .validate()
        .map_err(|error| ContinuityError(error.to_string()))?;
    Ok(session)
}

pub fn save_recording(
    path: impl AsRef<Path>,
    recording: &Recording,
) -> Result<(), ContinuityError> {
    validate_recording(recording)?;
    let bytes = envelope("gathering-recording", serde_json::to_value(recording)?)?;
    record_replacement::replace(
        path.as_ref(),
        &bytes,
        MAX_RECORD_BYTES,
        ContinuityError,
        |existing| {
            let Some(existing) = existing else {
                return Ok(Replacement::Replace);
            };
            let saved = decode_recording(decode_envelope(existing, "gathering-recording")?)?;
            if &saved != recording {
                return Err(ContinuityError("A different recording already occupies this path. Recordings are immutable; choose a new path.".into()));
            }
            Ok(Replacement::Unchanged)
        },
        |previous| load_recording(previous).map(|_| ()),
    )
}

pub fn load_recording(path: impl AsRef<Path>) -> Result<Recording, ContinuityError> {
    decode_recording(read_envelope(path.as_ref(), "gathering-recording")?)
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

fn read_envelope(path: &Path, kind: &str) -> Result<Envelope, ContinuityError> {
    decode_envelope(
        &record_replacement::read(path, MAX_RECORD_BYTES, ContinuityError)?,
        kind,
    )
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
        || envelope.source_revision != SOURCE_REVISION
        || envelope.app_version != APP_VERSION
    {
        return Err(ContinuityError(
            "This record requires a different compatible application/source version.".into(),
        ));
    }
    Ok(envelope)
}

/// The previous valid save remains alongside the current save for explicit recovery.
pub fn previous_path(path: impl AsRef<Path>) -> PathBuf {
    record_replacement::previous_path(path.as_ref())
}
