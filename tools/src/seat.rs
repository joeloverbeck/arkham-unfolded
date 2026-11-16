//! `seat`: one command's requests to a held agent seat (#175). The requests cross one
//! connection to the seat's local socket, and every response line is printed exactly as
//! the seat wrote it. `seat-wait` keeps a held seat's table running while the agent
//! partners it hosts play (#218). These commands belong to the agent seat's removal
//! set, listed in `src/gathering_tabletop/seat.rs`.

use crate::process;
use anyhow::{Context, Result, anyhow, bail};
use clap::Args as ClapArgs;
use serde_json::Value;
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use unfolded_seat_client::{
    self as seat_client, Connection, Outcome, Reply, RequestError, Unreached,
};
use unfolded_seat_protocol::{Request, Status};

#[derive(ClapArgs)]
pub struct Args {
    /// The socket the seat was started with (`--socket`).
    socket: PathBuf,
    /// Requests, one JSON object each. Without any, each standard-input line is one.
    requests: Vec<String>,
}

/// Exit 0 only when every request was answered and no answer is `refused`, `missed` or
/// `skipped`.
pub fn run(args: Args) -> Result<u8> {
    let requests = seat_client::requests(&args.requests, &mut std::io::stdin().lock())?;
    let connection =
        connect(&args.socket)?.map_err(|error| anyhow!(not_listening(&args.socket, &error)))?;
    // An interrupt is honoured during a long request.
    match connection.send(
        &requests,
        &mut std::io::stdout().lock(),
        process::check_interrupt,
    )? {
        Outcome::Answered => Ok(0),
        Outcome::Failed => Ok(1),
        Outcome::Short { answered } => {
            eprintln!(
                "arkham-tools: the seat answered {answered} of {} requests; its session ended before the rest.",
                requests.len()
            );
            Ok(1)
        }
    }
}

/// The connection, or, inside, the error that shows no seat listens on `socket`:
/// a missing socket, or one a crashed or quit seat left with nothing answering.
fn connect(socket: &Path) -> Result<std::io::Result<Connection>> {
    match Connection::open(socket) {
        Ok(connection) => Ok(Ok(connection)),
        Err(Unreached::NotListening(error)) => Ok(Err(error)),
        Err(Unreached::Failed(error)) => bail!(
            "cannot reach the agent seat on {}: {error}",
            socket.display()
        ),
    }
}

fn not_listening(socket: &Path, error: &impl std::fmt::Display) -> String {
    format!(
        "No agent seat is listening on {} ({error}). A seat must be started first: start one with arkham-tabletop --experiment gathering --agent-seat --socket {} --save PATH.",
        socket.display(),
        socket.display()
    )
}

/// What `scripts/tools seat-wait --help` shows after the options.
pub const WAIT_HELP: &str = "\
It sends only observe and settle, one connection each, settling again every half
second while the table is settled, so the seat's own decisions stay with its
driver. It returns once the table waits on the seat alone: it shows no Take over,
so no partner-played investigator is awaited, or a reading, dialog or menu is
open over it (docs/agents/partner-instructions.md#decide says when the table
applies a partner's choice). --choices counts the revisions the table advanced
while it ran: with no other command acting on the seat, each is a partner's
choice, or a choice of the seat's held for partners' passes. It prints one line
whenever the revision, the stage or the awaited investigators change and, except
on exit 2 or 130, a last line naming why it returned.

Exit codes: 0 the table waits on the seat, or --revision or --choices was
reached; 3 the timeout passed; 4 no seat listens on the socket, from the start
or once it went away; 2 the command is malformed, the socket cannot be reached
for another reason, the seat did not answer a request within 10 seconds after
the timeout, reading its answer failed other than by a reset connection (which
counts as the seat having gone), or it answered something other than an
observation or a settle; 130 the command was interrupted.

Hosting an agent partner: README.md#hosting-an-agent-partner";

const MET: u8 = 0;
const TIMED_OUT: u8 = 3;
const GONE: u8 = 4;
/// The pause between settles while the table is settled: a partner's choice waits
/// at most this long for the settle that applies it.
const INTERVAL: Duration = Duration::from_millis(500);
/// Two seconds of table time per settle, so a settle under way when the timeout
/// passes ends well within [`GRACE`].
const STEPS: u32 = 120;
/// How long past the timeout an answer is still awaited from a seat that stopped.
const GRACE: Duration = Duration::from_secs(10);

#[derive(ClapArgs)]
pub struct WaitArgs {
    /// The socket the seat was started with (`--socket`).
    socket: PathBuf,
    /// Return also once the table's revision is at least N.
    #[arg(long, value_name = "N")]
    revision: Option<u64>,
    /// Return also once the table has applied N choices since this command began.
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u64).range(1..))]
    choices: Option<u64>,
    /// Seconds to keep the table running before returning with exit 3.
    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 240,
        value_parser = clap::value_parser!(u64).range(1..=590)
    )]
    timeout: u64,
}

/// One line of what the table shows, printed whenever it changes.
#[derive(serde::Serialize, PartialEq)]
struct State {
    revision: u64,
    stage: Value,
    /// Each partner-played investigator the table waits on, with its portrait's cue.
    awaiting: Vec<String>,
}

/// The last line: why the command returned.
#[derive(serde::Serialize)]
struct Stopped {
    stopped: &'static str,
    revision: Option<u64>,
    /// Choices the table applied while the command ran.
    choices: u64,
    /// What is open over the table, when that is why it waits on the seat.
    #[serde(skip_serializing_if = "Option::is_none")]
    open: Option<String>,
}

/// Settle the held seat's table repeatedly, without answering anything, until it
/// waits on the seat, `--revision` or `--choices` is reached, the timeout passes or
/// the seat goes away.
pub fn wait(args: WaitArgs) -> Result<u8> {
    let deadline = Instant::now() + Duration::from_secs(args.timeout);
    let mut stdout = std::io::stdout().lock();
    // The first request only reads the table, so a choice applied by the first
    // settle counts.
    let mut request = Request::Observe {};
    let mut start = None;
    let mut stopped = Stopped {
        stopped: "timeout",
        revision: None,
        choices: 0,
        open: None,
    };
    let mut shown = None;
    let code = loop {
        let answer = match exchange(&args.socket, &request, deadline + GRACE)? {
            Exchange::Answer(answer) => answer,
            Exchange::Gone(why) => {
                if start.is_none() {
                    eprintln!("arkham-tools: {}", not_listening(&args.socket, &why));
                } else {
                    eprintln!(
                        "arkham-tools: the agent seat on {} has gone ({why}); start it again to resume the attempt.",
                        args.socket.display()
                    );
                }
                stopped.stopped = "gone";
                break GONE;
            }
        };
        request = Request::Settle { steps: Some(STEPS) };
        let revision = answer["observation"]["revision"]
            .as_u64()
            .context("the seat's answer carries no observation revision")?;
        stopped.revision = Some(revision);
        stopped.choices = revision.saturating_sub(*start.get_or_insert(revision));
        if answer["table"]["settled"] == true {
            let state = State {
                revision,
                stage: answer["observation"]["stage"].clone(),
                awaiting: awaiting(&answer),
            };
            if shown.as_ref() != Some(&state) {
                writeln!(stdout, "{}", serde_json::to_string(&state)?)?;
                stdout.flush()?;
            }
            let open = open(&answer);
            let reached = if state.awaiting.is_empty() || open.is_some() {
                Some("seat")
            } else if args.revision.is_some_and(|wanted| revision >= wanted) {
                Some("revision")
            } else if args.choices.is_some_and(|wanted| stopped.choices >= wanted) {
                Some("choices")
            } else {
                None
            };
            shown = Some(state);
            if let Some(reached) = reached {
                stopped.stopped = reached;
                stopped.open = open;
                break MET;
            }
            process::pause(INTERVAL.min(deadline.saturating_duration_since(Instant::now())))?;
        }
        if Instant::now() >= deadline {
            break TIMED_OUT;
        }
    };
    writeln!(stdout, "{}", serde_json::to_string(&stopped)?)?;
    stdout.flush()?;
    Ok(code)
}

enum Exchange {
    Answer(Value),
    /// No seat answers on the socket any more, for this reason.
    Gone(String),
}

/// One request on a connection of its own, so the seat serves other commands
/// between this command's requests.
fn exchange(socket: &Path, request: &Request, until: Instant) -> Result<Exchange> {
    let connection = match connect(socket)? {
        Ok(connection) => connection,
        Err(error) => return Ok(Exchange::Gone(error.to_string())),
    };
    // An interrupt is honoured during a long settle.
    let reply = connection.request(request, || {
        process::check_interrupt()?;
        if Instant::now() >= until {
            bail!("it did not answer in time; it may have stopped running frames");
        }
        Ok(())
    });
    match reply {
        Ok(Reply::Answer(answer)) => match answer.status {
            Status::Observed | Status::Settled | Status::Unsettled => {
                Ok(Exchange::Answer(answer.response))
            }
            _ => bail!("the seat answered {request} with: {}", answer.line),
        },
        // A seat that ends with this request unread resets the connection.
        Ok(Reply::Ended(ended)) => Ok(Exchange::Gone(ended.to_string())),
        Err(error @ (RequestError::NotJson { .. } | RequestError::Unrecognised { .. })) => {
            Err(error.into())
        }
        Err(error) => {
            Err(anyhow::Error::new(error)
                .context(format!("the agent seat on {}", socket.display())))
        }
    }
}

/// The partner-played investigators the table waits on: it offers Take over for
/// exactly these, under each one's portrait and in the decision display
/// (`src/gathering_tabletop/ui.rs`). Each carries its portrait's cue after the
/// PARTNER mark, such as `thinking`, `silent 3 min` or `awaiting partner`.
fn awaiting(answer: &Value) -> Vec<String> {
    let labels: Vec<&str> = answer["table"]["controls"]
        .as_array()
        .map(|controls| {
            controls
                .iter()
                .filter_map(|control| control["label"].as_str())
                .collect()
        })
        .unwrap_or_default();
    let mut names: Vec<&str> = labels
        .iter()
        .filter_map(|label| label.strip_prefix("Take over "))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|name| {
            let cue = labels.iter().find_map(|label| {
                let marks = label.strip_prefix("Focus ")?.strip_prefix(name)?;
                let marks: Vec<&str> = marks.strip_prefix(" · ")?.split(" · ").collect();
                let after = marks.iter().position(|mark| *mark == "partner")? + 1;
                (after < marks.len()).then(|| marks[after..].join(" · "))
            });
            match cue {
                Some(cue) => format!("{name} ({cue})"),
                None => name.to_owned(),
            }
        })
        .collect()
}

/// What is open over a settled table: a reading, dialog or menu waits for the
/// seat, and every partner's choice waits until it closes.
fn open(answer: &Value) -> Option<String> {
    let table = &answer["table"];
    table["modal"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| table["menu"].is_object().then(|| "menu".to_owned()))
}
