//! `arkham-partner`, the one program an agent partner's session may run (Cuba Libre #202):
//! one request to the running Arkham table (#205, ADR 0007).
//! The shared client (`unfolded_agent_players::client`, ADR 0008) carries the request
//! over one connection to the table's partner socket, and its answer is printed exactly
//! as the table wrote it. The protocol and the view belong to the game
//! (`src/partners.rs`); this command only spells requests, words failures and maps
//! answers to exit codes. Deleting this file, `bin/arkham-partner.rs`, `partner_folder.rs`,
//! `tools/tests/partner.rs` and the lines in `main.rs` that name `partner_folder` removes it.

use crate::process;
use anyhow::{Error, Result, anyhow, bail};
use clap::{Args as ClapArgs, Parser, Subcommand};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use unfolded_agent_players::client::{self, Unanswered};

/// What `arkham-partner --help` gives a fresh agent session: the pointer to
/// the partner instructions and the commands. The instructions alone state the
/// protocol's rules, so this guide does not restate them (#211, #218).
const GUIDE: &str = "\
You are an agent partner: a player at a running Arkham Horror: The Card Game table
in the Arkham Unfolded window. Before you join, read the partner instructions,
which your folder holds, and read them again after your context is
compacted. They state the rules of every command: what each answer and status
means, which choices wait for other players, waiting in Claude Code and in
Codex, shared decisions, table talk and its limits, conduct, the notes to keep
and the exit codes.

  arkham-partner join INVESTIGATOR         keep the answer's \"partner\"
  arkham-partner wait PARTNER              until something needs you
  arkham-partner wait PARTNER --timeout SECONDS
  arkham-partner wait PARTNER --listen     before you decide
  arkham-partner observe PARTNER
  arkham-partner observe PARTNER --full    the complete view and recent talk
  arkham-partner inspect PARTNER CODE
  arkham-partner choose PARTNER REVISION ID
  arkham-partner choose PARTNER REVISION ID --wait    and then wait
  arkham-partner ready PARTNER REVISION [--wait]
  arkham-partner say PARTNER MESSAGE
  arkham-partner leave PARTNER";

/// Play one investigator at the running Arkham table as an agent partner: join, wait,
/// observe, inspect a card, choose, mark ready, speak or leave. The game must have agent
/// partners enabled (Menu, Agent partners).
#[derive(Parser)]
#[command(name = "arkham-partner", after_help = GUIDE)]
pub struct Args {
    /// The table's partner socket, when the pasted line names one. Default:
    /// $ARKHAM_PARTNER_SOCKET, else arkham-unfolded-partners.sock in $XDG_RUNTIME_DIR,
    /// or in the temporary directory without it.
    #[arg(long, global = true, value_name = "PATH")]
    socket: Option<PathBuf>,
    #[command(subcommand)]
    request: Request,
}

#[derive(Subcommand)]
enum Request {
    /// Join the table as one investigator. The table refuses a join that names none or
    /// several, naming the investigators open to partners. Your tool's identifier of this
    /// session goes with it, when the tool gives one: one session plays one investigator.
    Join {
        #[arg(value_parser = ["tony", "patrice", "jacqueline"], value_name = "INVESTIGATOR")]
        investigators: Vec<String>,
        /// Your identity, when you have joined before: a join by an identity that still
        /// plays an investigator is refused.
        #[arg(long, value_name = "PARTNER")]
        partner: Option<String>,
    },
    /// Wait, without a time limit unless you give one, for a decision of yours or another
    /// event for you, such as the human's message. Another agent partner's message is none.
    Wait {
        partner: String,
        /// Seconds to wait at most, then answer nothing_yet.
        #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(u64).range(1..))]
        timeout: Option<u64>,
        /// Wait before you decide: your decision that is pending now does not end this
        /// wait, and every other event for you does. Give a --timeout, since there may be
        /// no event.
        #[arg(long)]
        listen: bool,
    },
    /// Print your current view, with the changes since your last.
    Observe {
        partner: String,
        /// Print the complete view, every visible card's text and the latest table
        /// talk since joining, heard before or not.
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
        #[command(flatten)]
        then: Then,
    },
    /// Mark yourself ready in the pending shared decision at the view's revision.
    Ready {
        partner: String,
        revision: u64,
        #[command(flatten)]
        then: Then,
    },
    /// Leave the table, ending your session.
    Leave { partner: String },
    /// Say one short message to the table; its words are joined with spaces.
    Say {
        partner: String,
        #[arg(required = true, value_name = "MESSAGE")]
        words: Vec<String>,
    },
}

/// What a submission does once it is accepted.
#[derive(ClapArgs)]
struct Then {
    /// Then wait, without a time limit, for the next event for you, as `wait` does, and print
    /// only that. A refusal, or a decision of yours that comes next, is printed at once.
    #[arg(long)]
    wait: bool,
    /// Seconds to wait at most, then answer nothing_yet.
    #[arg(long, value_name = "SECONDS", requires = "wait", value_parser = clap::value_parser!(u64).range(1..))]
    timeout: Option<u64>,
}

impl Then {
    fn add_to(&self, request: &mut Value) {
        if self.wait {
            request["wait"] = json!(true);
        }
        if let Some(seconds) = self.timeout {
            request["timeout_seconds"] = json!(seconds);
        }
    }
}

/// Exit 0 when the table answered, 1 when it refused, 3 when a wait found nothing yet.
pub fn run(args: Args) -> Result<u8> {
    let socket = args.socket.unwrap_or_else(default_socket);
    let request = match args.request {
        Request::Join {
            investigators,
            partner,
        } => {
            let mut request = json!({"op": "join", "investigators": investigators});
            if let Some(partner) = partner {
                request["partner"] = json!(partner);
            }
            if let Some(session) = client::session() {
                request["session"] = json!(session);
            }
            request
        }
        Request::Wait {
            partner,
            timeout,
            listen,
        } => {
            let mut request = json!({"op": "wait", "partner": partner});
            if let Some(seconds) = timeout {
                request["timeout_seconds"] = json!(seconds);
            }
            if listen {
                request["listen"] = json!(true);
            }
            request
        }
        Request::Observe { partner, full } => {
            let mut request = json!({"op": "observe", "partner": partner});
            if full {
                request["full"] = json!(true);
            }
            request
        }
        Request::Inspect { partner, code } => {
            json!({"op": "inspect", "partner": partner, "card": code})
        }
        Request::Choose {
            partner,
            revision,
            id,
            then,
        } => {
            let mut request =
                json!({"op": "choose", "partner": partner, "revision": revision, "id": id});
            then.add_to(&mut request);
            request
        }
        Request::Ready {
            partner,
            revision,
            then,
        } => {
            let mut request = json!({"op": "ready", "partner": partner, "revision": revision});
            then.add_to(&mut request);
            request
        }
        Request::Leave { partner } => json!({"op": "leave", "partner": partner}),
        Request::Say { partner, words } => {
            json!({"op": "say", "partner": partner, "text": words.join(" ")})
        }
    };
    // The table answers at the period it was asked for; this only bounds a table that
    // stops.
    let deadline = request["timeout_seconds"].as_u64().and_then(|seconds| {
        Instant::now().checked_add(Duration::from_secs(seconds.saturating_add(30)))
    });
    let answer = client::request(&socket, &request.to_string(), || {
        // An interrupt is honoured during a long wait.
        process::check_interrupt()?;
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            bail!("the table did not answer the wait in time; it may have stopped running frames");
        }
        Ok(())
    })
    .map_err(|unanswered| worded(unanswered, &socket))?;
    answer.write_to(&mut std::io::stdout().lock())?;
    Ok(match answer.status().as_deref() {
        Some("refused") | None => 1,
        Some("nothing_yet") => 3,
        Some(_) => 0,
    })
}

/// Why the table gave no answer, in this command's words.
fn worded(unanswered: Unanswered<Error>, socket: &Path) -> Error {
    match unanswered {
        Unanswered::NotListening(error) => anyhow!(
            "No Arkham table hosts agent partners on {} ({error}). Partners are enabled in the game (Menu, Agent partners); use the --socket the pasted line names, if any.{}",
            socket.display(),
            outside_partner_folders(),
        ),
        Unanswered::Unreached(error) => {
            anyhow!(
                "cannot reach the table on {}: {error}{}",
                socket.display(),
                outside_partner_folders(),
            )
        }
        Unanswered::Unsent(error) => {
            Error::new(error).context("cannot send the request to the table")
        }
        Unanswered::Stopped(error) => error,
        Unanswered::Unreadable(error) => {
            Error::new(error).context("cannot read the table's answer")
        }
        Unanswered::Closed => anyhow!("the table closed the connection without answering"),
        other => anyhow!("{other}"),
    }
}

/// #306: a command run outside both partner folders lacks its tool's permission for this
/// program, and the tool's sandbox can refuse the table's socket, so an unreached table
/// names where a partner session starts. Inside either folder, or when Arkham's data
/// directory is unknown, it adds nothing.
fn outside_partner_folders() -> String {
    let (Some(data), Ok(here)) = (crate::partner_folders::data(), std::env::current_dir()) else {
        return String::new();
    };
    let places = crate::partner_folders::Places::of(&data);
    let real = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    let here = real(&here);
    if [&places.claude_code, &places.codex]
        .into_iter()
        .any(|folder| here.starts_with(real(folder)))
    {
        return String::new();
    }
    format!(
        " This command ran in {}, outside the partner folders, where neither tool loads its permission for this program: start a partner session in Claude Code in {} or in Codex in {}, then join.",
        here.display(),
        places.claude_code.display(),
        places.codex.display()
    )
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
