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

/// What `scripts/tools partner --help` teaches a fresh agent session.
pub const GUIDE: &str = "\
You are an agent partner: a player at a running Arkham Horror: The Card Game table \
(The Gathering, Standard) in the Arkham Unfolded window. Another player plays the \
other investigators. Play your investigators on your own judgement, toward the \
team's success. You only choose among the options the table lists for your \
investigators; you never touch the window, saves or settings.

1. Join once:
     scripts/tools partner join
   Takes every investigator handed to partners that no partner plays (or name them:
   join patrice). The answer's \"partner\" is your identity: keep it, every later
   command needs it. \"investigators\" says whom you play.

2. Then loop:
     scripts/tools partner wait PARTNER
   Blocks until one of your investigators has a decision at a settled table
   (status \"decision\"), a group choice opens (\"group_choice\", once per choice),
   another player speaks (\"table_talk\"), the investigators you play change
   (\"control_changed\"), or
   the table closes (\"closed\": stop). After its timeout (240 s by default, --timeout
   SECONDS up to 590) it prints \"nothing_yet\" and exits 3: run it again.
   Read the answer's \"view\": \"decision.options\" lists only your options (\"id\",
   \"label\", \"investigator\"); \"decision.yours\" says the table waits for you;
   \"test\", \"encounter_card\", \"harm\" and \"resolving\" say what it is about.
   Your first view, from join, is complete: \"investigators\", \"locations\",
   \"enemies\", \"act\", \"agenda\" and the rest of the table. Every later view
   keeps the decision whole and reports the table only as \"changes\" since your
   last view: \"at\" names where (such as investigators.patrice.resources), with
   \"was\" and \"now\", \"now\" alone, or \"added\" and \"removed\". \"cards\" gives
   each card's full text the first time you see it.
     scripts/tools partner choose PARTNER REVISION ID
   with the view's \"revision\" and one option's \"id\". It waits until the table
   has finished presenting, then prints \"chosen\" with the new view. A stale,
   unlisted or not-yours choice prints \"refused\" with the reason and the current
   view, and exits 1: decide again from that view.
     scripts/tools partner observe PARTNER
   prints the current view, with the changes since your last, at any time;
     scripts/tools partner observe PARTNER --full
   prints the complete view and every visible card's text again, for example
   after your context was compacted.
     scripts/tools partner inspect PARTNER CODE
   prints the full text of a card you can see, by its \"code\", as the table's
   Inspect shows it, with its current counters; a card you cannot see is refused.

3. When you are done: scripts/tools partner leave PARTNER

Group choices, such as who takes the next turn, belong to the lead investigator's
player (after the lead is eliminated, the next investigator's in player order).
The view's \"decision.group_choice\" names its \"arbiter\" and what is being chosen;
only the arbiter's player has its options. In this version the other player makes
shared decisions (such as committing cards to a test with several participants);
the view's \"decision.note\" says so. Wait for your next decision. Other players are
named only by their investigators.

Undo: while your own latest decision is the latest one and nothing has followed
it, the view's \"decision.undo\" offers it; choose its \"id\" (\"undo\") to revert it.

Table talk: every player hears every message, and a message never makes a decision.
     scripts/tools partner say PARTNER MESSAGE
says one short message (up to 280 characters) as your investigators. Speak as a
player, not in character, and only when it matters: plans, warnings, offers and
replies. Do not name or read out the cards in your hand. Another player's messages
reach you once, in the next answer's \"view.table_talk\", each with its \"speaker\"
(that player's investigators), \"text\" and \"round\". You may send at most 3
messages in a row until play moves on or another player speaks, and no one may
speak while a peril encounter resolves; a refused say exits 1.

Waiting costs nothing: in Claude Code, give wait a Bash timeout above its own (for
example 300000 ms) or let it run in the background, which re-invokes you when it
exits; in Codex, run wait again whenever it answers nothing_yet. Views give
changes, not the history: keep brief notes of the table and your plan as you play.

Exit codes: 0 answered, 1 refused, 2 the table cannot be reached or the command
is malformed, 3 nothing yet (wait only). The first run in a checkout builds the
tool, which can take a few minutes; later runs start at once.";

#[derive(ClapArgs)]
pub struct Args {
    /// The table's partner socket, when the player's line names one. Default:
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
    /// Wait until one of your investigators has a decision at a settled table, a
    /// group choice opens, the investigators you play change, or the table closes;
    /// exit 3 with nothing_yet after the timeout.
    Wait {
        partner: String,
        /// Seconds to wait before answering nothing_yet.
        #[arg(long, default_value_t = 240, value_parser = clap::value_parser!(u64).range(1..=590))]
        timeout: u64,
    },
    /// Print your current view, with the changes since your last.
    Observe {
        partner: String,
        /// Print the complete view and every visible card's text again.
        #[arg(long)]
        full: bool,
    },
    /// Print the full text of a card you can see, by its code.
    Inspect { partner: String, code: String },
    /// Choose one of your options by its id at the view's revision.
    Choose {
        partner: String,
        revision: u64,
        id: String,
    },
    /// Leave the table; your investigators wait for another partner or the player.
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
        Request::Wait { partner, timeout } => (
            json!({"op": "wait", "partner": partner, "timeout_seconds": timeout}),
            // The table answers at its timeout; this only bounds a table that stops.
            Some(Duration::from_secs(timeout + 30)),
        ),
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
            "No Arkham table hosts agent partners on {} ({error}). The player enables partners in the game (Menu, Agent partners); use the --socket their line names, if any.",
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
