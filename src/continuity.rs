//! Offline continuity for the bounded issue #2 experiment.

use crate::{
    APP_VERSION, Fixture, Observation, Recording, SOURCE_REVISION, Session, Stage, TokenStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const FORMAT_VERSION: u32 = 1;
const MAX_RECORD_BYTES: u64 = 2 * 1024 * 1024;

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
    if recording.source_revision != SOURCE_REVISION || recording.app_version != APP_VERSION {
        return Err(ContinuityError(
            "This recording requires a different compatible application/source version.".into(),
        ));
    }
    if recording.moments.is_empty()
        || recording.moments.len() > 64
        || recording.moments.iter().enumerate().any(|(index, moment)| {
            moment.revision != index as u64
                || moment.investigators.len() != 3
                || moment.tokens.len() > 4
                || moment
                    .tokens
                    .iter()
                    .enumerate()
                    .any(|(id, token)| token.id as usize != id)
                || moment.choices.iter().enumerate().any(|(index, choice)| {
                    choice.id.is_empty()
                        || moment.choices[..index]
                            .iter()
                            .any(|earlier| earlier.id == choice.id)
                })
        })
    {
        return Err(ContinuityError(
            "The recorded decision sequence is invalid.".into(),
        ));
    }
    Ok(())
}

fn validate_session(session: &Session) -> Result<(), ContinuityError> {
    // Check bounds used by observation/choice construction before invoking them.
    // This is a bounded save-format check, not a second rules implementation.
    let order = &session.fixture.order;
    if (*order != Fixture::curse().order && *order != Fixture::auto_fail().order)
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
        return Err(ContinuityError(
            "The saved session has an invalid shape.".into(),
        ));
    }
    validate_recording(&session.recording())?;
    // Validate the exact stored continuation, including a legacy empty Commit
    // moment. Its live projection may omit that prompt, but never rewrites it.
    if session.moments.last() != Some(&session.observe_at_stage(session.stage)) {
        return Err(ContinuityError(
            "The saved session does not match its last recorded decision.".into(),
        ));
    }
    Ok(())
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
    write_session(path.as_ref(), session, false)
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
pub fn restart_session(path: impl AsRef<Path>, session: &Session) -> Result<(), ContinuityError> {
    if session != &Session::new(session.fixture.clone()) {
        return Err(ContinuityError("Restart requires a fresh example.".into()));
    }
    write_session(path.as_ref(), session, true)
}

fn write_session(path: &Path, session: &Session, restart: bool) -> Result<(), ContinuityError> {
    validate_session(session)?;
    let mut envelope = Envelope {
        format_version: FORMAT_VERSION,
        kind: "session".into(),
        source_revision: SOURCE_REVISION.into(),
        app_version: APP_VERSION.into(),
        payload: serde_json::to_value(session)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    replace_record(
        path,
        &serde_json::to_vec_pretty(&envelope)?,
        "session",
        Some(session),
        restart,
    )?;
    Ok(())
}

pub fn load_session(path: impl AsRef<Path>) -> Result<Session, ContinuityError> {
    let (_, envelope) = read_envelope(path.as_ref(), "session")?;
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
        kind: "recording".into(),
        source_revision: recording.source_revision.clone(),
        app_version: recording.app_version.clone(),
        payload: serde_json::to_value(recording)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    replace_record(
        path.as_ref(),
        &serde_json::to_vec_pretty(&envelope)?,
        "recording",
        None,
        false,
    )
}

pub fn load_recording(path: impl AsRef<Path>) -> Result<Recording, ContinuityError> {
    let (_, envelope) = read_envelope(path.as_ref(), "recording")?;
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
    if envelope.format_version != FORMAT_VERSION
        || envelope.kind != kind
        || envelope.source_revision != SOURCE_REVISION
        || envelope.app_version != APP_VERSION
    {
        return Err(ContinuityError(
            "This record requires a different compatible application/source version.".into(),
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
    restart: bool,
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
            if kind == "recording" {
                let candidate: Envelope = serde_json::from_slice(bytes)?;
                if envelope.payload != candidate.payload {
                    return Err(ContinuityError("A different recording already occupies this path. Recordings are immutable; choose a new path.".into()));
                }
                return Ok(());
            }
            if let Some(candidate) = session {
                let committed: Session = serde_json::from_value(envelope.payload)?;
                validate_session(&committed)?;
                if !restart && candidate == &committed {
                    return Ok(());
                }
                if !restart
                    && (committed.fixture != candidate.fixture
                        || !candidate.moments.starts_with(&committed.moments)
                        || (candidate.revision == committed.revision && candidate != &committed))
                {
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
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
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
        if restart {
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
