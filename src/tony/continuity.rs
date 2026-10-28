//! Offline continuity for the bounded Tony turn. Legacy records retain their own reader.

use super::presentation::{PresentationStage, stages};
use super::{Harm, Observation, Recording, SOURCE_REVISION, Session};
use crate::APP_VERSION;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const FORMAT_VERSION: u32 = 2;
const MAX_RECORD_BYTES: u64 = 2 * 1024 * 1024;

pub struct Replay {
    recording: Recording,
    stages: Vec<PresentationStage>,
    index: usize,
}

impl Replay {
    pub fn new(recording: Recording) -> Result<Self, ContinuityError> {
        validate_recording(&recording)?;
        let mut presentation = vec![PresentationStage {
            view: recording.moments[0].clone(),
            harm: None,
        }];
        for pair in recording.moments.windows(2) {
            presentation.extend(stages(&pair[0], &pair[1]));
        }
        Ok(Self {
            recording,
            stages: presentation,
            index: 0,
        })
    }
    pub fn current(&self) -> &Observation {
        &self.stages[self.index].view
    }
    pub fn harm(&self) -> Option<&Harm> {
        self.stages[self.index].harm.as_ref()
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
        self.stages.len()
    }
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
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
        kind: "tony-session".into(),
        source_revision: SOURCE_REVISION.into(),
        app_version: APP_VERSION.into(),
        payload: serde_json::to_value(session)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    replace_record(
        path,
        &serde_json::to_vec_pretty(&envelope)?,
        "tony-session",
        Some(session),
        expected,
    )?;
    Ok(())
}

pub fn load_session(path: impl AsRef<Path>) -> Result<Session, ContinuityError> {
    let (_, envelope) = read_envelope(path.as_ref(), "tony-session")?;
    let session = serde_json::from_value(envelope.payload)?;
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
        kind: "tony-recording".into(),
        source_revision: recording.source_revision.clone(),
        app_version: recording.app_version.clone(),
        payload: serde_json::to_value(recording)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    replace_record(
        path.as_ref(),
        &serde_json::to_vec_pretty(&envelope)?,
        "tony-recording",
        None,
        None,
    )
}

pub fn load_recording(path: impl AsRef<Path>) -> Result<Recording, ContinuityError> {
    let (_, envelope) = read_envelope(path.as_ref(), "tony-recording")?;
    let recording = serde_json::from_value(envelope.payload)?;
    validate_recording(&recording)?;
    Ok(recording)
}

fn read_envelope(path: &Path, kind: &str) -> Result<(Vec<u8>, Envelope), ContinuityError> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(ContinuityError(
            "This file exceeds the bounded experiment's record limit.".into(),
        ));
    }
    let envelope: Envelope = serde_json::from_slice(&bytes)?;
    if envelope.checksum != envelope.checksum()? {
        return Err(ContinuityError(
            "Record checksum mismatch. The original file was left unchanged.".into(),
        ));
    }
    if !matches!(envelope.format_version, 1 | FORMAT_VERSION)
        || envelope.kind != kind
        || envelope.source_revision != SOURCE_REVISION
        || envelope.app_version != APP_VERSION
    {
        return Err(ContinuityError(
            "This record requires a different compatible application/source version.".into(),
        ));
    }
    // V1 had no typed harm facts. It is read as aggregate history, not upgraded
    // by interpreting its prose or guessing blocks from counter differences.
    if envelope.format_version == 1
        && (envelope.payload.get("harm").is_some()
            || envelope.payload["moments"]
                .as_array()
                .is_some_and(|moments| moments.iter().any(|m| m.get("harm").is_some())))
    {
        return Err(ContinuityError(
            "A version-one Tony record cannot contain version-two harm facts.".into(),
        ));
    }
    Ok((bytes, envelope))
}

/// The last valid save is retained next to the destination with `.previous` added.
pub fn previous_path(path: impl AsRef<Path>) -> PathBuf {
    let mut name = path.as_ref().as_os_str().to_os_string();
    name.push(".previous");
    PathBuf::from(name)
}

fn replace_record(
    path: &Path,
    bytes: &[u8],
    kind: &str,
    session: Option<&Session>,
    expected: Option<&Session>,
) -> Result<(), ContinuityError> {
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(ContinuityError(
            "This file exceeds the bounded experiment's record limit.".into(),
        ));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    // Both player clients use this lock, including the read-before-replace check.
    // The sidecar stays in place so competing processes lock the same inode.
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(PathBuf::from(lock_path))?;
    lock.try_lock()
        .map_err(|error| ContinuityError(format!("The save is busy: {error}")))?;
    let previous = previous_path(path);
    let existing = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            let (existing_bytes, envelope) = read_envelope(path, kind)?;
            if kind == "tony-recording" {
                let candidate: Envelope = serde_json::from_slice(bytes)?;
                if envelope.payload != candidate.payload {
                    return Err(ContinuityError("A different recording already occupies this path. Recordings are immutable; choose a new path.".into()));
                }
                return Ok(());
            }
            if let Some(candidate) = session {
                let committed: Session = serde_json::from_value(envelope.payload)?;
                validate_session(&committed)?;
                if candidate == &committed {
                    return Ok(());
                }
                if let Some(expected) = expected {
                    if expected != &committed {
                        return Err(ContinuityError("A different or later turn is already saved. Reopen it before starting another turn.".into()));
                    }
                } else if !candidate.is_continuation_of(&committed) {
                    return Err(ContinuityError("A different or later decision is already saved. Reopen it before continuing.".into()));
                }
            }
            Some(existing_bytes)
        }
        Ok(_) => {
            return Err(ContinuityError(
                "The record destination is not a regular file.".into(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if expected.is_some() {
                return Err(ContinuityError("The previous turn is no longer at this path. Reopen before starting another turn.".into()));
            }
            None
        }
        Err(error) => return Err(error.into()),
    };
    if existing.is_some() && previous.try_exists()? {
        load_session(&previous)?;
    }
    // Write and sync before replacing either named record. The original remains
    // intact if creating or writing the temporary file fails.
    let mut next = tempfile::NamedTempFile::new_in(parent)?;
    next.write_all(bytes)?;
    next.as_file().sync_all()?;
    if let Some(existing) = existing {
        let mut backup = tempfile::NamedTempFile::new_in(parent)?;
        backup.write_all(&existing)?;
        backup.as_file().sync_all()?;
        if expected.is_some() {
            let prefix = format!(
                "{}.archived-",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            let mut archive = tempfile::Builder::new()
                .prefix(&prefix)
                .suffix(".json")
                .tempfile_in(parent)?;
            archive.write_all(&existing)?;
            archive.as_file().sync_all()?;
            archive
                .keep()
                .map_err(|error| ContinuityError(error.error.to_string()))?;
        }
        backup
            .persist(&previous)
            .map_err(|error| ContinuityError(error.error.to_string()))?;
    }
    next.persist(path)
        .map_err(|error| ContinuityError(error.error.to_string()))?;
    Ok(())
}
