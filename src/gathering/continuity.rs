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
const PRESERVED_APP_VERSION: &str = "0.1.0";
const PRESERVED_READERS: [(&str, &str); 3] = [
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
];

pub struct Replay {
    recording: Recording,
    positions: Vec<ReplayPosition>,
    index: usize,
    projected: Option<Observation>,
    draws: Vec<super::moments::DrawCue>,
}

enum ReplayPosition {
    Decision(usize),
    Presentation(usize),
}

impl Replay {
    pub fn new(recording: Recording) -> Result<Self, ContinuityError> {
        let draws = super::recording_draws(&recording)
            .map_err(|error| ContinuityError(error.to_string()))?;
        let mut positions =
            Vec::with_capacity(recording.moments.len() + recording.presentation_moments.len());
        let mut presentation = 0;
        for (index, decision) in recording.moments.iter().enumerate() {
            while recording
                .presentation_moments
                .get(presentation)
                .is_some_and(|moment| moment.revision == decision.revision)
            {
                positions.push(ReplayPosition::Presentation(presentation));
                presentation += 1;
            }
            positions.push(ReplayPosition::Decision(index));
        }
        Ok(Self {
            recording,
            positions,
            index: 0,
            projected: None,
            draws,
        })
    }

    pub fn current(&self) -> &Observation {
        match self.positions[self.index] {
            ReplayPosition::Decision(index) => &self.recording.moments[index],
            ReplayPosition::Presentation(_) => self
                .projected
                .as_ref()
                .expect("Selected presentation projection"),
        }
    }

    pub fn recording(&self) -> &Recording {
        &self.recording
    }

    pub fn current_draws(&self) -> impl Iterator<Item = &super::moments::DrawCue> {
        self.draws.iter().filter(|cue| {
            cue.revision == self.current().revision
                && cue.sequence == self.current_presentation_sequence()
        })
    }

    /// The displayed transient's cutoff within its decision revision.
    pub fn current_presentation_sequence(&self) -> Option<u32> {
        match self.positions[self.index] {
            ReplayPosition::Decision(_) => None,
            ReplayPosition::Presentation(index) => {
                Some(self.recording.presentation_moments[index].sequence)
            }
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> bool {
        if self.index + 1 >= self.len() {
            return false;
        }
        self.index += 1;
        self.project_current();
        true
    }

    pub fn back(&mut self) -> bool {
        if self.index == 0 {
            return false;
        }
        self.index -= 1;
        self.project_current();
        true
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    fn project_current(&mut self) {
        self.projected = match self.positions[self.index] {
            ReplayPosition::Decision(_) => None,
            ReplayPosition::Presentation(index) => Some(super::moments::project(
                &self.recording.moments,
                &self.recording.presentation_moments,
                &self.recording.presentation_moments[index],
            )),
        };
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
    // Whitespace is not record identity. Compact encoding keeps the same
    // payload/checksum readable by existing readers without spending the
    // bounded record capacity on repeated snapshot indentation.
    Ok(serde_json::to_vec(&envelope)?)
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
    .map(|_| ())
}

/// Starts a fresh attempt only after preserving the exact expected saved record.
/// The returned archive remains independent of subsequent `.previous` rotation.
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
    session
        .validate()
        .map_err(|error| ContinuityError(error.to_string()))?;
    if session.source != expected.source {
        return Err(ContinuityError(
            "Restart must preserve the selected decks and rules.".into(),
        ));
    }
    let bytes = envelope("gathering-session", serde_json::to_value(session)?)?;
    record_replacement::replace(
        path.as_ref(),
        &bytes,
        MAX_RECORD_BYTES,
        ContinuityError,
        |existing| {
            let existing = existing.ok_or_else(|| ContinuityError(
                "The saved attempt is missing. Save the current attempt before restarting.".into(),
            ))?;
            let committed = decode_session(decode_envelope(existing, "gathering-session")?)?;
            if &committed != expected {
                return Err(ContinuityError("A different or later Gathering decision is already saved. Reopen it before restarting.".into()));
            }
            Ok(Replacement::Archive)
        },
        |previous| load_session(previous).map(|_| ()),
    )?
    .ok_or_else(|| ContinuityError("Restart did not retain the previous attempt.".into()))
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
    .map(|_| ())
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
        if envelope.format_version == 1
            && envelope.app_version == PRESERVED_APP_VERSION
            && envelope.kind == kind
            && let Some((source, reader)) = PRESERVED_READERS
                .iter()
                .find(|(source, _)| *source == envelope.source_revision)
        {
            return Err(ContinuityError(format!(
                "This Gathering record requires source {source}, preserved by reader commit {reader}. This build writes {SOURCE_REVISION}. The original file was left unchanged; use the exact resume/replay instructions in docs/experiments/gathering-setup.md#record-compatibility. No migration was performed."
            )));
        }
        return Err(ContinuityError(format!(
            "This record requires application {} / source {} / format {} / kind {}; this reader accepts application {APP_VERSION} / source {SOURCE_REVISION} / format {FORMAT_VERSION} / kind {kind}. The original file was left unchanged.",
            envelope.app_version, envelope.source_revision, envelope.format_version, envelope.kind
        )));
    }
    Ok(envelope)
}

/// The previous valid save remains alongside the current save for explicit recovery.
pub fn previous_path(path: impl AsRef<Path>) -> PathBuf {
    record_replacement::previous_path(path.as_ref())
}
