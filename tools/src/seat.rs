//! `seat`: one command's requests to a held agent seat (#175). The requests cross one
//! connection to the seat's local socket, and every response line is printed exactly as
//! the seat wrote it. This command belongs to the agent seat's removal set, listed in
//! `src/gathering_tabletop/seat.rs`.

use crate::process;
use anyhow::{Context, Result, anyhow, bail};
use clap::Args as ClapArgs;
use std::{
    io::{BufRead, BufReader, ErrorKind, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

#[derive(ClapArgs)]
pub struct Args {
    /// The socket the seat was started with (`--socket`).
    socket: PathBuf,
    /// Requests, one JSON object each. Without any, each standard-input line is one.
    requests: Vec<String>,
}

/// Exit 0 only when every request was answered and no answer is `refused` or `skipped`.
pub fn run(args: Args) -> Result<u8> {
    let requests = requests(&args.requests)?;
    let stream = connect(&args.socket)?;
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

fn connect(socket: &Path) -> Result<UnixStream> {
    UnixStream::connect(socket).map_err(|error| match error.kind() {
        ErrorKind::NotFound | ErrorKind::ConnectionRefused => anyhow!(
            "No agent seat is listening on {} ({error}). A seat must be started first: start one with arkham-tabletop --experiment gathering --agent-seat --socket {} --save PATH.",
            socket.display(),
            socket.display()
        ),
        _ => anyhow!(
            "cannot reach the agent seat on {}: {error}",
            socket.display()
        ),
    })
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
/// how many arrived and whether any was refused, skipped or unreadable.
fn print_responses(stream: &UnixStream) -> Result<(usize, bool)> {
    let mut reader = BufReader::new(stream);
    let mut stdout = std::io::stdout().lock();
    let (mut answered, mut failed) = (0, false);
    let mut line = Vec::new();
    loop {
        line.clear();
        if !read_line(&mut reader, &mut line)? {
            return Ok((answered, failed));
        }
        stdout.write_all(&line)?;
        stdout.flush()?;
        answered += 1;
        let status = serde_json::from_slice::<serde_json::Value>(&line)
            .ok()
            .and_then(|response| response["status"].as_str().map(str::to_owned));
        failed |= matches!(status.as_deref(), None | Some("refused" | "skipped"));
    }
}

/// One line, waiting out read timeouts; a last line cut short is returned as it
/// arrived. False at the end of the connection.
fn read_line(reader: &mut impl BufRead, line: &mut Vec<u8>) -> Result<bool> {
    loop {
        match reader.read_until(b'\n', line) {
            // Bytes read before a timeout stay in `line`.
            Ok(_) => return Ok(!line.is_empty()),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                process::check_interrupt()?
            }
            Err(error) => return Err(error).context("cannot read the seat's response"),
        }
    }
}
