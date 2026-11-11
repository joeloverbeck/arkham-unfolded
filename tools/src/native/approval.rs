//! Local accidental-reuse guard for this repository's desktop checks from Codex
//! and Claude Code sessions.
//! User-writable session logs/state are trusted inputs, not a security boundary.
mod claude;

use anyhow::{Context, Result, bail, ensure};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Subcommand)]
pub enum Command {
    /// Prepare the exact question for this session's question tool:
    /// request_user_input_async (Codex) or AskUserQuestion (Claude Code).
    Request {
        #[arg(long)]
        session_log: PathBuf,
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=10))]
        minutes: u32,
        #[arg(long)]
        purpose: String,
    },
    /// Validate the original user reply and show its unchanged deadline.
    Status {
        #[arg(long)]
        approval: String,
    },
    /// Permanently close this approval, including across helper restarts.
    Close {
        #[arg(long)]
        approval: String,
    },
}

/// The session harness whose log records the approval question and answer.
#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Harness {
    /// Requests written before Claude Code support carry no harness.
    #[default]
    Codex,
    ClaudeCode,
}

impl Harness {
    /// A Codex log opens with its session; any other log must have an entry
    /// that identifies a Claude Code session.
    fn of(source: &Path) -> Result<Self> {
        let mut entries = events(source)?;
        let first = entries.next().context("Empty session log")??;
        if first["type"] == "session_meta" {
            return Ok(Self::Codex);
        }
        for entry in std::iter::once(Ok(first)).chain(entries) {
            if claude::identifies(&entry?) {
                return Ok(Self::ClaudeCode);
            }
        }
        bail!("Unrecognized session log: no Codex or Claude Code session identity")
    }

    fn is_codex(&self) -> bool {
        *self == Self::Codex
    }
}

#[derive(Serialize, Deserialize)]
struct Request {
    id: String,
    source: PathBuf,
    /// Omitted for Codex so its stored requests and bound digests are unchanged.
    #[serde(default, skip_serializing_if = "Harness::is_codex")]
    harness: Harness,
    /// The Codex thread or Claude Code session that asked the question.
    thread: String,
    repository: PathBuf,
    minutes: u32,
    question: Value,
}

pub struct Approval {
    directory: PathBuf,
    deadline: Instant,
    pub approved_at: f64,
    pub deadline_unix_seconds: f64,
}

/// Recheck closure and expiry at every X11 operation, including startup focus.
pub struct Desktop<'a> {
    pub approval: &'a Approval,
    pub inner: crate::desktop::X11,
}

impl crate::interaction::Desktop for Desktop<'_> {
    fn focus(&mut self) -> Result<()> {
        self.approval.check()?;
        self.inner.focus()
    }
    fn verify(&mut self) -> Result<()> {
        self.approval.check()?;
        self.inner.verify()
    }
    fn geometry(&mut self) -> Result<crate::interaction::Geometry> {
        self.approval.check()?;
        self.inner.geometry()
    }
    fn pointer(&mut self) -> Result<crate::interaction::Pointer> {
        self.approval.check()?;
        self.inner.pointer()
    }
    fn capture(&mut self, path: &Path) -> Result<Value> {
        self.approval.check()?;
        self.inner.capture(path)
    }
}

fn state() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .context("Native approval requires XDG_STATE_HOME or HOME")?;
    ensure!(
        base.is_absolute(),
        "Native approval state must have an absolute path"
    );
    Ok(base.join("arkham-unfolded/native-approvals"))
}

fn directory(id: &str) -> Result<PathBuf> {
    ensure!(
        !id.is_empty() && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "Invalid approval ID"
    );
    Ok(state()?.join(id))
}

fn events(path: &Path) -> Result<impl Iterator<Item = Result<Value>>> {
    Ok(
        BufReader::new(fs::File::open(path).context("Read original session log")?)
            .lines()
            .map(|line| Ok(serde_json::from_str(&line?)?)),
    )
}

fn session_identity(source: &Path) -> Result<(String, PathBuf)> {
    let first = events(source)?
        .next()
        .context("Empty Codex session log")??;
    ensure!(
        first["type"] == "session_meta",
        "Missing Codex session identity"
    );
    let payload = &first["payload"];
    Ok((
        payload["id"].as_str().context("Missing session ID")?.into(),
        PathBuf::from(
            payload["cwd"]
                .as_str()
                .context("Missing session repository")?,
        )
        .canonicalize()?,
    ))
}

pub fn run(command: Command) -> Result<u8> {
    match command {
        Command::Request {
            session_log,
            minutes,
            purpose,
        } => {
            ensure!(
                !purpose.trim().is_empty()
                    && purpose.len() <= 500
                    && !purpose.contains(['\n', '\r']),
                "Purpose must be one nonempty line of at most 500 bytes"
            );
            let source = session_log.canonicalize()?;
            let harness = Harness::of(&source)?;
            let (thread, repository) = match harness {
                Harness::Codex => {
                    let (thread, repository) = session_identity(&source)?;
                    ensure!(
                        thread
                            == std::env::var("CODEX_THREAD_ID")
                                .context("Request requires CODEX_THREAD_ID")?,
                        "Session log is not the active Codex thread"
                    );
                    (thread, repository)
                }
                Harness::ClaudeCode => claude::active_session(&source)?,
            };
            ensure!(
                repository == super::repository().canonicalize()?,
                "Session log belongs to another repository"
            );
            let root = state()?;
            fs::create_dir_all(&root)?;
            let pending = tempfile::Builder::new()
                .prefix("desktop-")
                .rand_bytes(16)
                .tempdir_in(root)?;
            let id = pending
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let title = format!(
                "May I use your desktop for {minutes} minutes now to {purpose}? Approval request: {id}."
            );
            let question = match harness {
                Harness::Codex => json!({"title":title, "options":options(minutes)}),
                Harness::ClaudeCode => claude::question(title, options(minutes)),
            };
            let request = Request {
                id: id.clone(),
                source,
                harness,
                thread,
                repository,
                minutes,
                question: question.clone(),
            };
            fs::write(
                pending.path().join("request.json"),
                serde_json::to_vec_pretty(&request)?,
            )?;
            let _ = pending.keep();
            super::emit(&json!({"approval":id,"request":{"questions":[question]}}))?;
        }
        Command::Status { approval } => {
            let grant = Approval::admit(Some(&approval))?;
            super::emit(&json!({"approval":approval,"approved_at":grant.approved_at,
                "deadline_unix_seconds":grant.deadline_unix_seconds,"status":"active"}))?;
        }
        Command::Close { approval } => {
            let directory = directory(&approval)?;
            ensure!(
                directory.join("request.json").is_file(),
                "Unknown approval ID"
            );
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join("closed"))
            {
                Ok(file) => file.sync_all()?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
            super::emit(&json!({"approval":approval,"status":"closed"}))?;
        }
    }
    Ok(0)
}

impl Approval {
    pub fn admit(id: Option<&str>) -> Result<Self> {
        let id = id.context("Desktop access requires --approval from native approval request")?;
        let directory = directory(id)?;
        ensure!(
            !directory.join("closed").exists(),
            "Desktop approval is closed"
        );
        let request: Request = serde_json::from_slice(
            &fs::read(directory.join("request.json")).context("Unknown approval ID")?,
        )?;
        ensure!(
            request.id == id && (1..=10).contains(&request.minutes),
            "Invalid approval request"
        );
        let names_request = |title: &Value| {
            title
                .as_str()
                .is_some_and(|title| title.ends_with(&format!("Approval request: {id}.")))
        };
        ensure!(
            match request.harness {
                Harness::Codex =>
                    request.question["options"] == json!(options(request.minutes))
                        && names_request(&request.question["title"]),
                Harness::ClaudeCode =>
                    claude::offers(&request.question, options(request.minutes))
                        && names_request(&request.question["question"]),
            },
            "Approval question does not bind its ID and duration"
        );
        ensure!(
            request.repository == super::repository().canonicalize()?,
            "Approval belongs to another repository"
        );
        let (approved_at, proof) = match request.harness {
            Harness::Codex => {
                ensure!(
                    session_identity(&request.source)?
                        == (request.thread.clone(), request.repository.clone()),
                    "Approval source identity changed"
                );
                approved_reply(&request)?
            }
            Harness::ClaudeCode => {
                ensure!(
                    claude::session(&request.source)? == request.thread,
                    "Approval source identity changed"
                );
                claude::approved_answer(&request)?
            }
        };
        let deadline_unix_seconds = approved_at + f64::from(request.minutes) * 60.0;
        let wall_now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64();
        ensure!(
            approved_at <= wall_now,
            "Approval timestamp is in the future"
        );
        ensure!(
            deadline_unix_seconds > wall_now,
            "Desktop approval has expired"
        );
        // The selected source events must remain identical even if the live log grows.
        let binding = directory.join("binding");
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&binding)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(proof.as_bytes())?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                ensure!(
                    fs::read_to_string(binding)? == proof,
                    "Approval evidence changed"
                );
            }
            Err(error) => return Err(error.into()),
        }
        let grant = Self {
            directory,
            deadline: Instant::now() + Duration::from_secs_f64(deadline_unix_seconds - wall_now),
            approved_at,
            deadline_unix_seconds,
        };
        grant.check()?;
        Ok(grant)
    }

    pub fn check(&self) -> Result<()> {
        ensure!(
            !self.directory.join("closed").exists(),
            "Desktop approval is closed"
        );
        ensure!(
            Instant::now() < self.deadline
                && SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64()
                    < self.deadline_unix_seconds,
            "Desktop approval has expired"
        );
        Ok(())
    }

    pub fn remaining(&self) -> Result<Duration> {
        self.check()?;
        Ok(self.deadline.saturating_duration_since(Instant::now()))
    }
}

/// The finite answers; only the first one approves.
fn options(minutes: u32) -> [String; 2] {
    [format!("Yes, {minutes} minutes now"), "Not now".into()]
}

fn approved_reply(request: &Request) -> Result<(f64, String)> {
    let mut call: Option<(String, Value)> = None;
    let mut approved = None;
    for event in events(&request.source)? {
        let event = event?;
        if event["type"] != "response_item" {
            continue;
        }
        let payload = &event["payload"];
        if payload["type"] == "function_call" && payload["name"] == "request_user_input_async" {
            let args: Value =
                serde_json::from_str(payload["arguments"].as_str().unwrap_or("null"))?;
            if args == json!({"questions":[request.question]}) {
                ensure!(call.is_none(), "Ambiguous repeated approval question");
                call = Some((
                    payload["call_id"]
                        .as_str()
                        .context("Missing approval call ID")?
                        .into(),
                    event.clone(),
                ));
            }
        }
        let Some((call_id, call_event)) = &call else {
            continue;
        };
        if payload["type"] != "message" || payload["role"] != "user" {
            continue;
        }
        let metadata = &payload["internal_chat_message_metadata_passthrough"];
        let content = payload["content"]
            .as_array()
            .context("Missing user content")?;
        for (index, item) in content.iter().enumerate() {
            if metadata["content_item_metadata"][index]["provenance"]["type"] != "user" {
                continue;
            }
            let Some(text) = item["text"].as_str() else {
                continue;
            };
            let Some(reply) = text
                .strip_prefix("<send_user_message_question_reply>")
                .and_then(|s| s.trim().strip_suffix("</send_user_message_question_reply>"))
            else {
                continue;
            };
            let replies: Vec<Value> = serde_json::from_str(reply.trim())?;
            for reply in replies {
                let question_id =
                    serde_json::to_string(&json!(["request_user_input_async", call_id, 0]))?;
                if reply["questionItemId"] != question_id
                    || reply["question"] != request.question["title"]
                    || reply["answer"] != request.question["options"][0]
                {
                    continue;
                }
                ensure!(approved.is_none(), "Ambiguous repeated user approval");
                let time = metadata["create_time"]
                    .as_f64()
                    .context("Missing original approval timestamp")?;
                ensure!(time.is_finite() && time > 0.0, "Invalid approval timestamp");
                let proof = format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(&json!([request, call_event, event]))?)
                );
                approved = Some((time, proof));
            }
        }
    }
    approved.context("No matching user approval in the original session log")
}
