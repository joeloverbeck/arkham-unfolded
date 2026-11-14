//! `seat`: one command's requests to a held agent seat (#175). The requests cross one
//! connection to the seat's local socket, and every response line is printed exactly as
//! the seat wrote it. `seat-wait` keeps a held seat's table running while the agent
//! partners it hosts play (#218). These commands belong to the agent seat's removal
//! set, listed in `src/gathering_tabletop/seat.rs`.

use crate::process;
use anyhow::{Context, Result, anyhow, bail};
use clap::Args as ClapArgs;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, ErrorKind, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

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
    let requests = requests(&args.requests)?;
    let stream =
        connect(&args.socket)?.map_err(|error| anyhow!(not_listening(&args.socket, &error)))?;
    // Wake regularly so an interrupt is honoured during a long request.
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    let (answered, failed) = thread::scope(|scope| {
        scope.spawn(|| send(&stream, &requests));
        print_responses(&stream)
    })?;
    if answered < requests.len() {
        eprintln!(
            "arkham-tools: the seat answered {answered} of {} requests; its session ended before the rest.",
            requests.len()
        );
        return Ok(1);
    }
    Ok(u8::from(failed))
}

fn requests(given: &[String]) -> Result<Vec<String>> {
    let lines: Vec<String> = if given.is_empty() {
        std::io::stdin()
            .lock()
            .lines()
            .collect::<std::io::Result<_>>()
            .context("cannot read requests from standard input")?
    } else {
        given
            .iter()
            .flat_map(|request| request.lines().map(str::to_owned))
            .collect()
    };
    // The seat answers every nonblank line and ignores blank ones.
    let requests: Vec<String> = lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if requests.is_empty() {
        bail!("no requests: give them as arguments or as standard-input lines");
    }
    Ok(requests)
}

/// The connection, or, inside, the error that shows no seat listens on `socket`:
/// a missing socket, or one a crashed or quit seat left with nothing answering.
fn connect(socket: &Path) -> Result<std::io::Result<UnixStream>> {
    match UnixStream::connect(socket) {
        Err(error)
            if !matches!(
                error.kind(),
                ErrorKind::NotFound | ErrorKind::ConnectionRefused
            ) =>
        {
            bail!(
                "cannot reach the agent seat on {}: {error}",
                socket.display()
            )
        }
        reached => Ok(reached),
    }
}

fn not_listening(socket: &Path, error: &impl std::fmt::Display) -> String {
    format!(
        "No agent seat is listening on {} ({error}). A seat must be started first: start one with arkham-tabletop --experiment gathering --agent-seat --socket {} --save PATH.",
        socket.display(),
        socket.display()
    )
}

/// Every request, then the end of this connection's input. A seat whose session ends
/// early stops reading, so a failed write only means fewer answers.
fn send(stream: &UnixStream, requests: &[String]) {
    let mut writer = stream;
    for request in requests {
        if writer.write_all(format!("{request}\n").as_bytes()).is_err() {
            return;
        }
    }
    let _ = stream.shutdown(Shutdown::Write);
}

/// Print each response line unchanged until the seat closes the connection. Returns
/// how many arrived and whether any was refused, missed, skipped or unreadable.
fn print_responses(stream: &UnixStream) -> Result<(usize, bool)> {
    let mut reader = BufReader::new(stream);
    let mut stdout = std::io::stdout().lock();
    let (mut answered, mut failed) = (0, false);
    let mut line = Vec::new();
    loop {
        line.clear();
        if !read_line(&mut reader, &mut line, None)? {
            return Ok((answered, failed));
        }
        stdout.write_all(&line)?;
        stdout.flush()?;
        answered += 1;
        let status = serde_json::from_slice::<serde_json::Value>(&line)
            .ok()
            .and_then(|response| response["status"].as_str().map(str::to_owned));
        failed |= matches!(
            status.as_deref(),
            None | Some("refused" | "missed" | "skipped")
        );
    }
}

/// One line, waiting out read timeouts, until `until` if given; a last line cut short
/// is returned as it arrived. False at the end of the connection.
fn read_line(
    reader: &mut impl BufRead,
    line: &mut Vec<u8>,
    until: Option<Instant>,
) -> Result<bool> {
    loop {
        match reader.read_until(b'\n', line) {
            // Bytes read before a timeout stay in `line`.
            Ok(_) => return Ok(!line.is_empty()),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                process::check_interrupt()?;
                if until.is_some_and(|until| Instant::now() >= until) {
                    bail!("it did not answer in time; it may have stopped running frames");
                }
            }
            Err(error) => return Err(error).context("cannot read the seat's response"),
        }
    }
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
on exit 2, a last line naming why it returned.

Exit codes: 0 the table waits on the seat, or --revision or --choices was
reached; 3 the timeout passed; 4 no seat listens on the socket, from the start
or once it went away; 2 the command is malformed, the socket cannot be reached
for another reason, the seat did not answer a request within 10 seconds after
the timeout, or it answered something other than an observation or a settle.

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
    let mut request = json!({"op": "observe"});
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
        request = json!({"op": "settle", "steps": STEPS});
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
fn exchange(socket: &Path, request: &Value, until: Instant) -> Result<Exchange> {
    let stream = match connect(socket)? {
        Ok(stream) => stream,
        Err(error) => return Ok(Exchange::Gone(error.to_string())),
    };
    // Wake regularly so an interrupt is honoured during a long settle.
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    if (&stream)
        .write_all(format!("{request}\n").as_bytes())
        .is_err()
    {
        return Ok(Exchange::Gone("it closed the connection".into()));
    }
    let _ = stream.shutdown(Shutdown::Write);
    let mut line = Vec::new();
    let answered = match read_line(&mut BufReader::new(&stream), &mut line, Some(until)) {
        // A seat that ends with this request unread resets the connection.
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == ErrorKind::ConnectionReset) =>
        {
            return Ok(Exchange::Gone(error.root_cause().to_string()));
        }
        answered => answered.with_context(|| format!("the agent seat on {}", socket.display()))?,
    };
    if !answered {
        return Ok(Exchange::Gone(
            "it closed the connection without answering".into(),
        ));
    }
    let answer: Value = serde_json::from_slice(&line).with_context(|| {
        format!(
            "the seat's answer is not JSON: {}",
            String::from_utf8_lossy(&line).trim_end()
        )
    })?;
    match answer["status"].as_str() {
        Some("observed" | "settled" | "unsettled") => Ok(Exchange::Answer(answer)),
        Some(status @ ("bye" | "exited")) => {
            Ok(Exchange::Gone(format!("its session ended with {status}")))
        }
        _ => bail!(
            "the seat answered {request} with: {}",
            String::from_utf8_lossy(&line).trim_end()
        ),
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
