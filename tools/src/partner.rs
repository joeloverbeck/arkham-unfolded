//! `partner`: one agent partner request to the running Arkham table (#205, ADR 0007).
//! The request crosses one connection to the table's partner socket and its answer is
//! printed exactly as the table wrote it. The protocol and the view belong to the game
//! (`src/partners.rs`); this command only spells requests and maps answers to exit
//! codes. Deleting this file, `tools/tests/partner.rs` and the line in `main.rs` that
//! names it removes the command.

use crate::process;
use anyhow::{Context, Result, anyhow, bail};
use clap::{Args as ClapArgs, Subcommand};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, ErrorKind, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// What `scripts/tools partner --help` teaches a fresh agent session: the commands
/// and the loop in brief. The partner instructions it points to own the rest (#211).
pub const GUIDE: &str = "\
You are an agent partner: a player at a running Arkham Horror: The Card Game table
in the Arkham Unfolded window. Before you join, read
docs/agents/partner-instructions.md in this repository, and read it again after
your context is compacted. It describes every answer and status, waiting in
Claude Code and in Codex, group and shared decisions, table talk, conduct and
the notes to keep.

  scripts/tools partner join [INVESTIGATOR...]    keep the answer's \"partner\"
  scripts/tools partner wait PARTNER [--timeout SECONDS]
  scripts/tools partner wait PARTNER --listen     hear the table first
  scripts/tools partner observe PARTNER
  scripts/tools partner observe PARTNER --full    the complete view and recent talk
  scripts/tools partner inspect PARTNER CODE
  scripts/tools partner choose PARTNER REVISION ID
  scripts/tools partner ready PARTNER REVISION
  scripts/tools partner say PARTNER MESSAGE
  scripts/tools partner leave PARTNER

A wait answers \"decision\" once the table has presented one of your
investigators' decisions, even while a reading or a dialog is still open; the
table applies a choice only once it settles, so choose waits until then. In
another investigator's turn, a fast card or ability of yours wakes you before
each of that investigator's actions: use it, or pass with ready. In your own
investigators' turn, while another player may still use one first, choose
answers \"held\" at once, except for Undo, your part of a shared decision and
an act's clue payment, which are not held; a later wait answers \"chosen\" once
the held choice is applied, or \"released\" when a fast play came first and you
decide again. After
the first, complete view, each view reports the table as \"changes\" since your
last. A wait answers \"nothing_yet\" after its timeout (240 s by default, at
most 590): run it again. When the pasted line names --socket PATH, put it on
every command. Table talk is limited: at most three messages in a row from
you, until play moves on or another player speaks, and six in an exchange,
until play moves on or a player who has not yet spoken at that decision
speaks.

Exit codes: 0 answered (\"closed\" included: stop), 1 refused (decide again from
the view in the answer), 2 the table cannot be reached, did not answer a wait in
time, or the command is malformed, 3 nothing yet (wait only). The first run in a
checkout builds the tool, which can take a few minutes.";

#[derive(ClapArgs)]
pub struct Args {
    /// The table's partner socket, when the pasted line names one. Default:
    /// $ARKHAM_PARTNER_SOCKET, else arkham-unfolded-partners.sock in $XDG_RUNTIME_DIR.
    #[arg(long, global = true, value_name = "PATH")]
    socket: Option<PathBuf>,
    #[command(subcommand)]
    request: Request,
}

#[derive(Subcommand)]
enum Request {
    /// Join the table: take every investigator handed to partners that no partner
    /// plays, or only the named ones.
    Join {
        #[arg(value_parser = ["tony", "patrice", "jacqueline"])]
        investigators: Vec<String>,
    },
    /// Wait until the table has presented a decision of one of your investigators,
    /// a group choice opens, another player changes a shared decision you take
    /// part in, your held choice is applied or released, another player speaks,
    /// the investigators you play change, or the table closes; exit 3 with
    /// nothing_yet after the timeout.
    Wait {
        partner: String,
        /// Seconds to wait before answering nothing_yet.
        #[arg(long, default_value_t = 240, value_parser = clap::value_parser!(u64).range(1..=590))]
        timeout: u64,
        /// Hear the table before deciding: never answer your own pending decision,
        /// only another event or nothing_yet.
        #[arg(long)]
        listen: bool,
    },
    /// Print your current view, with the changes since your last.
    Observe {
        partner: String,
        /// Print the complete view, every visible card's text and the latest table
        /// talk heard since joining again.
        #[arg(long)]
        full: bool,
    },
    /// Print the full text of a card you can see, by its code.
    Inspect { partner: String, code: String },
    /// Choose one of your options by its id at the view's revision; the table
    /// applies it once it settles, and the answer waits until then. A choice
    /// that waits for other players' passes before your action is answered held
    /// at once.
    Choose {
        partner: String,
        revision: u64,
        id: String,
    },
    /// Mark yourself ready in the pending shared decision, on the parts your last
    /// view showed, at its revision.
    Ready { partner: String, revision: u64 },
    /// Leave the table; your investigators' decisions wait for another partner, or
    /// for the table to take them over.
    Leave { partner: String },
    /// Say one short message to the table; its words are joined with spaces.
    Say {
        partner: String,
        #[arg(required = true, value_name = "MESSAGE")]
        words: Vec<String>,
    },
}

/// Exit 0 when the table answered, 1 when it refused, 3 when a wait found nothing yet.
pub fn run(args: Args) -> Result<u8> {
    let socket = args.socket.unwrap_or_else(default_socket);
    let (request, answer_within) = match args.request {
        Request::Join { investigators } => {
            (json!({"op": "join", "investigators": investigators}), None)
        }
        Request::Wait {
            partner,
            timeout,
            listen,
        } => {
            let mut request = json!({"op": "wait", "partner": partner, "timeout_seconds": timeout});
            if listen {
                request["listen"] = json!(true);
            }
            // The table answers at its timeout; this only bounds a table that stops.
            (request, Some(Duration::from_secs(timeout + 30)))
        }
        Request::Observe { partner, full } => {
            let mut request = json!({"op": "observe", "partner": partner});
            if full {
                request["full"] = json!(true);
            }
            (request, None)
        }
        Request::Inspect { partner, code } => (
            json!({"op": "inspect", "partner": partner, "card": code}),
            None,
        ),
        Request::Choose {
            partner,
            revision,
            id,
        } => (
            json!({"op": "choose", "partner": partner, "revision": revision, "id": id}),
            None,
        ),
        Request::Ready { partner, revision } => (
            json!({"op": "ready", "partner": partner, "revision": revision}),
            None,
        ),
        Request::Leave { partner } => (json!({"op": "leave", "partner": partner}), None),
        Request::Say { partner, words } => (
            json!({"op": "say", "partner": partner, "text": words.join(" ")}),
            None,
        ),
    };
    let stream = connect(&socket)?;
    // Wake regularly so an interrupt is honoured during a long wait.
    stream.set_read_timeout(Some(Duration::from_millis(200)))?;
    (&stream)
        .write_all(format!("{request}\n").as_bytes())
        .context("cannot send the request to the table")?;
    let _ = stream.shutdown(Shutdown::Write);
    let deadline = answer_within.map(|within| Instant::now() + within);
    let mut line = Vec::new();
    read_line(&mut BufReader::new(&stream), &mut line, deadline)?;
    if line.is_empty() {
        bail!("the table closed the connection without answering");
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&line)?;
    if !line.ends_with(b"\n") {
        stdout.write_all(b"\n")?;
    }
    stdout.flush()?;
    let status = serde_json::from_slice::<Value>(&line)
        .ok()
        .and_then(|answer| answer["status"].as_str().map(str::to_owned));
    Ok(match status.as_deref() {
        Some("refused") | None => 1,
        Some("nothing_yet") => 3,
        Some(_) => 0,
    })
}

/// The socket the game opens unless told otherwise; the game's
/// `gathering_tabletop::partners::default_socket` names the same path.
fn default_socket() -> PathBuf {
    if let Some(path) = std::env::var_os("ARKHAM_PARTNER_SOCKET") {
        return PathBuf::from(path);
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("arkham-unfolded-partners.sock")
}

fn connect(socket: &Path) -> Result<UnixStream> {
    UnixStream::connect(socket).map_err(|error| match error.kind() {
        ErrorKind::NotFound | ErrorKind::ConnectionRefused => anyhow!(
            "No Arkham table hosts agent partners on {} ({error}). Partners are enabled in the game (Menu, Agent partners); use the --socket the pasted line names, if any.",
            socket.display()
        ),
        _ => anyhow!("cannot reach the table on {}: {error}", socket.display()),
    })
}

/// One line, waiting out read timeouts until `deadline`; empty at the end of the
/// connection.
fn read_line(
    reader: &mut impl BufRead,
    line: &mut Vec<u8>,
    deadline: Option<Instant>,
) -> Result<()> {
    loop {
        match reader.read_until(b'\n', line) {
            Ok(_) => return Ok(()),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                process::check_interrupt()?;
                if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                    bail!(
                        "the table did not answer the wait in time; it may have stopped running frames"
                    );
                }
            }
            Err(error) => return Err(error).context("cannot read the table's answer"),
        }
    }
}
