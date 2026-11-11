//! Current-state files. A loaded baseline, not a history tape, owns write freshness.
use crate::{
    APP_VERSION, RuleError,
    record_replacement::{self, Replacement},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Mutex,
};

const FORMAT_VERSION: u32 = 3;
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct Baseline(Mutex<Option<[u8; 32]>>);
impl Clone for Baseline {
    fn clone(&self) -> Self {
        Self(Mutex::new(*self.0.lock().expect("Save baseline lock")))
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
    fn matches(&self, bytes: &[u8]) -> bool {
        *self.0.lock().expect("Save baseline lock") == Some(Sha256::digest(bytes).into())
    }
    fn remember(&self, bytes: &[u8]) {
        *self.0.lock().expect("Save baseline lock") = Some(Sha256::digest(bytes).into());
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
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self(format!("Invalid save: {error}"))
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
    fn checksum(&self) -> Result<String, Error> {
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                self.format_version,
                &self.kind,
                &self.source_revision,
                &self.app_version,
                &self.payload,
            ))?)
        ))
    }
}
fn encode<T: Snapshot>(session: &T) -> Result<Vec<u8>, Error> {
    session.validate().map_err(|e| Error(e.to_string()))?;
    let mut envelope = Envelope {
        format_version: FORMAT_VERSION,
        kind: T::KIND.into(),
        source_revision: T::SOURCE.into(),
        app_version: APP_VERSION.into(),
        payload: serde_json::to_value(session)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    Ok(serde_json::to_vec(&envelope)?)
}
fn decode<T: Snapshot>(bytes: &[u8]) -> Result<T, Error> {
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.format_version != FORMAT_VERSION
        || envelope.kind != T::KIND
        || envelope.source_revision != T::SOURCE
        || envelope.app_version != APP_VERSION
    {
        return Err(Error(format!(
            "Unsupported save (format {}, kind {}, source {}, application {}). Records in that format are unsupported by this build, and development builds promise no save compatibility. Start a New attempt at a different save path. The original file was left unchanged.",
            envelope.format_version, envelope.kind, envelope.source_revision, envelope.app_version
        )));
    }
    if envelope.checksum != envelope.checksum()? {
        return Err(Error(
            "Save checksum mismatch. The original file was left unchanged.".into(),
        ));
    }
    // An intact record admits only when this build reproduces it exactly.
    let session = serde_json::from_value::<T>(envelope.payload)
        .map_err(|error| error.to_string())
        .and_then(|session| session.validate().map(|()| session).map_err(|e| e.0))
        .map_err(|detail| {
            Error(format!(
                "This build cannot reproduce the game recorded in this save ({detail}). Development builds promise no save compatibility: select New attempt to begin again. The original file was left unchanged."
            ))
        })?;
    session.baseline().remember(bytes);
    Ok(session)
}
pub(crate) fn load<T: Snapshot>(path: impl AsRef<Path>) -> Result<T, Error> {
    decode(&record_replacement::read(path.as_ref(), MAX_BYTES, Error)?)
}
pub(crate) fn save<T: Snapshot>(path: impl AsRef<Path>, session: &T) -> Result<(), Error> {
    save_with_policy(path.as_ref(), session, false)
}

#[cfg(feature = "tabletop")]
pub(crate) fn create<T: Snapshot>(path: &Path, session: &T) -> Result<(), Error> {
    save_with_policy(path, session, true)
}

fn save_with_policy<T: Snapshot>(path: &Path, session: &T, exclusive: bool) -> Result<(), Error> {
    let bytes = encode(session)?;
    let mut saved_bytes = bytes.clone();
    record_replacement::replace(
        path,
        &bytes,
        MAX_BYTES,
        Error,
        |existing| {
            let Some(existing) = existing else {
                return Ok(if exclusive {
                    Replacement::Create
                } else {
                    Replacement::Replace
                });
            };
            if exclusive {
                return Err(Error("The new attempt path is already occupied. No existing attempt was opened or replaced.".into()));
            }
            let committed: T = decode(existing)?;
            if serde_json::to_value(&committed)? == serde_json::to_value(session)? {
                saved_bytes = existing.to_vec();
                return Ok(Replacement::Unchanged);
            }
            if !session.baseline().matches(existing) {
                return Err(Error(
                    "A different or later decision is already saved. Reopen it before continuing."
                        .into(),
                ));
            }
            Ok(Replacement::Replace)
        },
        |previous| load::<T>(previous).map(|_| ()),
    )?;
    session.baseline().remember(&saved_bytes);
    Ok(())
}
pub(crate) fn restart<T: Snapshot>(
    path: impl AsRef<Path>,
    expected: &T,
    fresh: &T,
) -> Result<PathBuf, Error> {
    let bytes = encode(fresh)?;
    let archive = record_replacement::replace(
        path.as_ref(),
        &bytes,
        MAX_BYTES,
        Error,
        |existing| {
            let existing = existing.ok_or_else(|| {
                Error("The saved attempt is missing. Save it before restarting.".into())
            })?;
            let committed: T = decode(existing)?;
            if !expected.baseline().matches(existing)
                || serde_json::to_value(&committed)? != serde_json::to_value(expected)?
            {
                return Err(Error(
                    "A different or later decision is already saved. Reopen it before restarting."
                        .into(),
                ));
            }
            Ok(Replacement::Archive)
        },
        |previous| load::<T>(previous).map(|_| ()),
    )?
    .ok_or_else(|| Error("Restart did not retain the previous attempt.".into()))?;
    fresh.baseline().remember(&bytes);
    Ok(archive)
}
