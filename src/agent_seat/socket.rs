//! The held session (#175): the seat's protocol on a local Unix-domain socket, so one
//! session outlives the shell command that started it and keeps the table's camera,
//! menus, dialogs and readings between commands.
//!
//! The launching process claims the socket and starts the seat again as a fresh process
//! (never a fork of a running app) in its own session, handing it the listening socket
//! as standard input. The launching shell's standard input, output and error are not
//! passed on, so a harness waiting for their end is not held by the seat. The launcher
//! waits for admission and prints its result; the seat then answers one connection at a
//! time, in arrival order, through the same serving loop as batch form.

use super::{Game, Seat, Session, emit};
use bevy::prelude::App;
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions, Permissions},
    io::{BufRead, BufReader, ErrorKind},
    os::{
        fd::{AsFd, OwnedFd},
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::UnixListener,
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Set on the process the launcher starts, which serves instead of launching.
const DETACHED: &str = "ARKHAM_AGENT_SEAT_DETACHED";
/// A Unix-domain socket path is at most this many bytes.
const PATH_LIMIT: usize = 107;

/// Whether this process is the seat a launcher started.
pub(crate) fn detached() -> bool {
    std::env::var_os(DETACHED).is_some()
}

/// The held seat's standard error: the socket path with `.log` appended.
pub(crate) fn log_path(socket: &Path) -> PathBuf {
    let mut log = socket.as_os_str().to_owned();
    log.push(".log");
    log.into()
}

/// A socket path one seat listens on, accessible only to its owner. The path is removed
/// when the listener ends, unless something else has replaced it there.
pub(crate) struct Listener {
    listener: UnixListener,
    path: PathBuf,
    /// The socket file's device and inode while this listener owns the path.
    file: Option<(u64, u64)>,
}

impl Listener {
    /// Listen on `path`. The socket is made in a private directory and linked into place
    /// only once owner-only, so the path never exists with wider access. An existing path
    /// is refused and left unchanged.
    pub(crate) fn claim(path: &Path) -> Result<Self, String> {
        let cannot =
            |error: &dyn std::fmt::Display| format!("Cannot listen on {}: {error}", path.display());
        if path.as_os_str().len() > PATH_LIMIT {
            return Err(cannot(&format_args!(
                "a local socket path holds at most {PATH_LIMIT} bytes; choose a shorter one"
            )));
        }
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| cannot(&"the path names no directory"))?;
        let private = tempfile::Builder::new()
            .prefix(".s")
            .permissions(Permissions::from_mode(0o700))
            .tempdir_in(parent)
            .map_err(|error| cannot(&error))?;
        let made = private.path().join("s");
        let listener = UnixListener::bind(&made).map_err(|error| cannot(&error))?;
        fs::set_permissions(&made, Permissions::from_mode(0o600))
            .map_err(|error| cannot(&error))?;
        fs::hard_link(&made, path).map_err(|error| match error.kind() {
            ErrorKind::AlreadyExists => cannot(&"the path already exists; it was left unchanged"),
            _ => cannot(&error),
        })?;
        let file = fs::symlink_metadata(path).map_err(|error| cannot(&error))?;
        Ok(Self {
            listener,
            path: path.to_owned(),
            file: Some((file.dev(), file.ino())),
        })
    }

    /// The listener a launcher handed this seat as its standard input, for `path`.
    fn inherited(path: &Path) -> Result<Self, String> {
        let not_handed = |error: &dyn std::fmt::Display| {
            format!(
                "The seat was not handed a listening socket for {} ({error}); start it without {DETACHED}.",
                path.display()
            )
        };
        let listener = UnixListener::from(
            std::io::stdin()
                .as_fd()
                .try_clone_to_owned()
                .map_err(|error| not_handed(&error))?,
        );
        listener.local_addr().map_err(|error| not_handed(&error))?;
        let file = fs::symlink_metadata(path).map_err(|error| not_handed(&error))?;
        if !file.file_type().is_socket() {
            return Err(not_handed(&"the path is not a socket"));
        }
        Ok(Self {
            listener,
            path: path.to_owned(),
            file: Some((file.dev(), file.ino())),
        })
    }

    /// Answer connections one at a time, in arrival order, until `quit` or the table's
    /// own exit. A client closing its connection does not end the session. The socket
    /// is removed before the last connection closes.
    pub(crate) fn serve<G: Game>(
        mut self,
        seat: &mut Seat<G>,
        app: &mut App,
    ) -> Result<(), String> {
        eprintln!("The agent seat is listening on {}.", self.path.display());
        let last = loop {
            let stream = match self.listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) if error.kind() == ErrorKind::ConnectionAborted => continue,
                Err(error) => {
                    return Err(format!(
                        "The seat stopped accepting connections on {}: {error}",
                        self.path.display()
                    ));
                }
            };
            match seat.converse(app, BufReader::new(&stream), &stream) {
                Ok(Session::Ended) => break stream,
                Ok(Session::Open) => {}
                Err(error) => eprintln!("A connection ended before all its responses: {error}"),
            }
        };
        self.remove();
        drop(last);
        eprintln!("The agent seat's session ended.");
        seat.outcome(app)
    }

    /// The process started with this listener as standard input now owns the path.
    fn hand_over(mut self) {
        self.file = None;
    }

    fn remove(&mut self) {
        let Some(owned) = self.file.take() else {
            return;
        };
        if fs::symlink_metadata(&self.path).is_ok_and(|file| (file.dev(), file.ino()) == owned) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Start the seat held on `socket` and wait for its admission. On success print its
/// `ready` response unchanged and leave it serving; otherwise return why it could not
/// start, with no socket left and no seat running.
pub(crate) fn launch(socket: &Path) -> Result<(), String> {
    let listener = Listener::claim(socket)?;
    let log = log_path(socket);
    let mut command = Command::new("/proc/self/exe");
    if let Some(name) = std::env::args_os().next() {
        command.arg0(name);
    }
    // The same executable as this process, even if its file has since been replaced.
    let mut seat = command
        .args(std::env::args_os().skip(1))
        .env(DETACHED, "1")
        .stdin(Stdio::from(OwnedFd::from(
            listener
                .listener
                .try_clone()
                .map_err(|error| format!("Cannot hand over the socket: {error}"))?,
        )))
        .stdout(Stdio::piped())
        .stderr(replace(&log)?)
        .spawn()
        .map_err(|error| format!("Cannot start the seat: {error}"))?;
    let mut line = String::new();
    let read = BufReader::new(seat.stdout.take().expect("piped")).read_line(&mut line);
    let response: Option<Value> = serde_json::from_str(&line).ok();
    if read.is_ok()
        && response
            .as_ref()
            .is_some_and(|ready| ready["status"] == "ready")
    {
        print!("{line}");
        listener.hand_over();
        return Ok(());
    }
    // The seat did not start and is ending; the socket goes with this listener.
    let ended = seat.wait();
    drop(listener);
    match response
        .as_ref()
        .and_then(|refused| refused["error"].as_str())
    {
        Some(error) => Err(error.to_owned()),
        None => Err(format!(
            "The seat ended before admission ({}). Its diagnostics are in {}.",
            ended.map_or_else(|error| error.to_string(), |status| status.to_string()),
            log.display()
        )),
    }
}

/// The log, replaced: a fresh file accessible only to its owner.
fn replace(log: &Path) -> Result<File, String> {
    let cannot =
        |error: std::io::Error| format!("Cannot replace the log {}: {error}", log.display());
    match fs::remove_file(log) {
        Err(error) if error.kind() != ErrorKind::NotFound => return Err(cannot(error)),
        _ => {}
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(log)
        .map_err(cannot)
}

/// Become the seat a launcher started: leave the launching shell's session and process
/// group, and take the listener it handed over.
pub(crate) fn detach(socket: &Path) -> Result<Listener, String> {
    rustix::process::setsid()
        .map_err(|error| format!("The seat cannot leave the launching shell's session: {error}"))?;
    Listener::inherited(socket)
}

/// Tell the launcher the session is ready. Standard output carries nothing else.
pub(crate) fn announce(ready: &Value) -> Result<(), String> {
    emit(&mut std::io::stdout().lock(), ready)
        .map_err(|error| format!("The launcher did not receive the ready response: {error}"))
}
