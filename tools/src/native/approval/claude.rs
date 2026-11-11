//! Claude Code transcript reader for desktop approvals.
//!
//! Observed in Claude Code 2.1.322: the transcript is named `<sessionId>.jsonl`,
//! message entries carry `sessionId`, `cwd`, `isSidechain` and an RFC 3339 UTC
//! `timestamp`, and an `AskUserQuestion` answer is a `tool_result` entry whose
//! `toolUseResult.answers` maps question text to the selected label. Entries can
//! also carry `session_id`, which names the process's session from before
//! `/clear`, so it never identifies the transcript. Anything else fails closed.
use super::{Request, events};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Whether a log entry names a Claude Code session.
pub(super) fn identifies(entry: &Value) -> bool {
    entry["sessionId"].is_string()
}

/// The transcript's session, which must be the running one, and the
/// session's latest working directory.
pub(super) fn active_session(source: &Path) -> Result<(String, PathBuf)> {
    let session = session(source)?;
    ensure!(
        session
            == std::env::var("CLAUDE_CODE_SESSION_ID")
                .context("Request requires CLAUDE_CODE_SESSION_ID")?,
        "Session log is not the active Claude Code session"
    );
    let mut latest = None;
    for entry in events(source)? {
        if let Some(cwd) = entry?["cwd"].as_str() {
            latest = Some(cwd.to_owned());
        }
    }
    let latest = latest.context("Claude Code transcript records no working directory")?;
    Ok((session, PathBuf::from(latest).canonicalize()?))
}

/// The session that names the transcript file, which every entry must share.
pub(super) fn session(source: &Path) -> Result<String> {
    let session = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("Claude Code transcript must be named by its session")?;
    let mut identified = false;
    for entry in events(source)? {
        let entry = entry?;
        if entry["sessionId"].is_null() {
            continue;
        }
        ensure!(
            entry["sessionId"].as_str() == Some(session),
            "Transcript entry belongs to another Claude Code session"
        );
        identified = true;
    }
    ensure!(identified, "Claude Code transcript records no session");
    Ok(session.into())
}

/// One single-choice `AskUserQuestion` question offering the finite answers.
pub(super) fn question(title: String, [yes, no]: [String; 2]) -> Value {
    json!({
        "question":title,
        "header":"Desktop",
        "multiSelect":false,
        "options":[
            {"label":yes, "description":"Allow desktop use for this interval, starting when you answer."},
            {"label":no, "description":"Decline. No desktop use."}
        ]
    })
}

/// Whether the stored question is single-choice and offers exactly these answers.
pub(super) fn offers(question: &Value, options: [String; 2]) -> bool {
    question["multiSelect"] == false
        && question["options"].as_array().is_some_and(|offered| {
            offered
                .iter()
                .map(|o| &o["label"])
                .eq(&options.map(Value::from))
        })
}

/// The affirmative answer to the unchanged question asked in the main
/// conversation: its host-recorded time and a digest of request, call and answer.
pub(super) fn approved_answer(request: &Request) -> Result<(f64, String)> {
    let asked = json!({"questions":[request.question]});
    let text = request.question["question"]
        .as_str()
        .context("Missing approval question")?;
    let affirmative = &request.question["options"][0]["label"];
    let mut call: Option<(String, Value, f64)> = None;
    let mut answered = false;
    let mut approved = None;
    for entry in events(&request.source)? {
        let entry = entry?;
        if entry["isSidechain"] != false {
            continue;
        }
        let Some(content) = entry["message"]["content"].as_array() else {
            continue;
        };
        if entry["type"] == "assistant" {
            for item in content {
                if item["type"] == "tool_use"
                    && item["name"] == "AskUserQuestion"
                    && item["input"] == asked
                {
                    ensure!(call.is_none(), "Ambiguous repeated approval question");
                    ensure!(
                        working_directory(&entry)? == request.repository,
                        "Approval question was asked in another repository"
                    );
                    let id = item["id"].as_str().context("Missing approval call ID")?;
                    call = Some((id.into(), entry.clone(), time(&entry)?));
                }
            }
        }
        let Some((id, call_entry, asked_at)) = &call else {
            continue;
        };
        if entry["type"] != "user" {
            continue;
        }
        for item in content {
            if item["type"] != "tool_result"
                || item["tool_use_id"] != id.as_str()
                || item["is_error"] == true
            {
                continue;
            }
            // The question has exactly one answer, whatever it says.
            ensure!(!answered, "Repeated answer to the approval question");
            answered = true;
            let result = &entry["toolUseResult"];
            if result["answers"][text] != *affirmative {
                continue;
            }
            ensure!(
                result["annotations"][text].is_null(),
                "Approval answer carries notes"
            );
            let answered_at = time(&entry)?;
            ensure!(
                answered_at >= *asked_at,
                "Approval answer precedes its question"
            );
            approved = Some((answered_at, digest(&json!([request, call_entry, entry]))?));
        }
    }
    approved.context("No matching user approval in the original session log")
}

fn working_directory(entry: &Value) -> Result<PathBuf> {
    let cwd = entry["cwd"]
        .as_str()
        .context("Approval question records no working directory")?;
    Ok(PathBuf::from(cwd).canonicalize()?)
}

fn time(entry: &Value) -> Result<f64> {
    entry["timestamp"]
        .as_str()
        .and_then(unix_seconds)
        .context("Invalid approval timestamp")
}

/// Strict RFC 3339 UTC, `YYYY-MM-DDTHH:MM:SS[.fraction]Z`, as Unix seconds.
fn unix_seconds(text: &str) -> Option<f64> {
    let text = text.strip_suffix('Z')?;
    let (clock, fraction) = match text.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (text, None),
    };
    let separators = [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')];
    if !clock.is_ascii()
        || clock.len() != 19
        || separators
            .iter()
            .any(|&(index, separator)| clock.as_bytes()[index] != separator)
    {
        return None;
    }
    let field = |start: usize, end: usize| -> Option<i64> {
        let digits = &clock[start..end];
        digits
            .bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| digits.parse().ok())?
    };
    let (year, month, day) = (field(0, 4)?, field(5, 7)?, field(8, 10)?);
    let (hour, minute, second) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&month)
        || !(1..=lengths[(month - 1) as usize]).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let fraction = match fraction {
        None => 0.0,
        Some(digits)
            if (1..=9).contains(&digits.len()) && digits.bytes().all(|c| c.is_ascii_digit()) =>
        {
            format!("0.{digits}").parse::<f64>().ok()?
        }
        Some(_) => return None,
    };
    // Days from the civil date, counting March as the first month of the year.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some((days * 86_400 + hour * 3600 + minute * 60 + second) as f64 + fraction)
}

/// The bound evidence, which must stay identical even if the live log grows.
fn digest(evidence: &Value) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(evidence)?)
    ))
}
