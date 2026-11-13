//! The window's side of agent partners (#205, [ADR 0007]): the private socket, the
//! threads that carry request lines to the frame loop, the human's controls for
//! partners, and what the table shows of them. What a partner may do and see is
//! `crate::partners`, which this host drives once per frame against the table.
//!
//! The socket exists only while partners are enabled. It is claimed through
//! `unfolded-tabletop`'s shared path claiming (a private directory, owner-only
//! access, replacement of a dead socket); the seat's serving loop is not used.
//! A listener thread accepts any number of connections, one thread each; every
//! request line crosses a channel polled by [`serve`], which wakes the frame loop.
//! Held waits and choices never block a frame. A partner's choice is applied only
//! once the table has settled, through [`Table::commit`], the path the human's
//! clicks take, and never moves the camera.
//!
//! The host is part of [`install`], so the agent seat can host partners too.
//!
//! [ADR 0007]: ../../docs/adr/0007-agent-partners-join-the-running-table.md

use super::*;
use crate::gathering::CardView;
use crate::partners::{self as rules, Controller, Host, Routing};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, ErrorKind, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// What the human does with partners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Input {
    /// Show or hide the Agent partners section of the menu.
    Menu,
    /// Enable partners, or turn them off.
    Enable(bool),
    /// Hand an investigator to partners, or keep it.
    Hand(Investigator),
    /// Return an investigator to the human at once.
    TakeOver(Investigator),
    /// Open the message box (#209).
    Speak,
}

/// How the table shows the investigator it waits for when a partner plays it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Deciding {
    /// Its partner is deciding.
    Thinking,
    /// Its partner has sent nothing for this many minutes.
    Silent(u64),
    /// No partner holds it: the decision waits for one to join, or a Take over.
    Unclaimed,
}

/// Agent partners as the table shows them. [`serve`] refreshes everything but
/// the menu, the queued inputs and the attributions each frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Shown {
    pub menu_open: bool,
    pub inputs: Vec<Input>,
    pub enabled: bool,
    pub selected: [bool; 3],
    /// Whether a partner has joined to play each investigator.
    pub joined: [bool; 3],
    /// The line to paste into an agent's terminal, while partners are enabled.
    pub line: Option<String>,
    /// Why partners could not be enabled.
    pub problem: Option<String>,
    /// Each partner-played investigator the table waits on, and how.
    pub deciding: Vec<(Investigator, Deciding)>,
    /// What each awaited partner's investigator may use in the open window: the
    /// titles of the cards and abilities its options use.
    pub may_use: Vec<(Investigator, Vec<String>)>,
    /// Every investigator the table waits on when a partner's is among them, for
    /// the status line.
    pub awaited: Vec<Investigator>,
    /// The pending shared decisions whose participants several players control.
    pub shared: Vec<rules::SharedState>,
    /// History entries a partner's decision made in this sitting, and its
    /// investigator. Runtime only: the save's history is replayed from decisions.
    pub attributed: Vec<(usize, Investigator)>,
}

impl Shown {
    /// Whether a partner plays `who`.
    pub(super) fn partner_plays(&self, who: Investigator) -> bool {
        self.enabled && self.selected[who.index()]
    }

    /// How the partner deciding for `who` is shown, while the table waits on it.
    pub(super) fn deciding(&self, who: Investigator) -> Option<Deciding> {
        self.deciding
            .iter()
            .find(|(deciding, _)| *deciding == who)
            .map(|(_, state)| *state)
    }

    /// What the table says of a partner deciding for `who`.
    pub(super) fn cue(&self, who: Investigator) -> Option<String> {
        Some(match self.deciding(who)? {
            Deciding::Thinking => format!("{}: partner thinking…", who.name()),
            Deciding::Silent(minutes) => {
                format!("{}: no word from its partner for {minutes} min", who.name())
            }
            Deciding::Unclaimed => format!("{}: waiting for a partner to join", who.name()),
        })
    }

    /// Why the table waits on `who`'s partner in a window: what it may use there.
    pub(super) fn uses(&self, who: Investigator) -> Option<String> {
        let (_, titles) = self.may_use.iter().find(|(user, _)| *user == who)?;
        (!titles.is_empty()).then(|| {
            format!(
                "{} may use {} before this window closes",
                who.name(),
                titles.join(", ")
            )
        })
    }

    /// The human's part in the shared decision `id` completes, when it has one.
    pub(super) fn shared_with_human(&self, id: &str) -> Option<&rules::SharedState> {
        self.shared
            .iter()
            .find(|state| state.close == id && state.of(&Controller::Human).is_some())
    }

    /// The label of the human's readiness control in a shared decision: as
    /// listed until he is ready, then naming whose readiness is outstanding, and
    /// saying so when another player's change cleared it.
    pub(super) fn readiness_label(&self, id: &str, label: &str) -> String {
        let Some(state) = self.shared_with_human(id) else {
            return label.to_owned();
        };
        let human = state.of(&Controller::Human).expect("The human takes part");
        let names = |investigators: &[Investigator]| {
            investigators
                .iter()
                .map(|who| who.name())
                .collect::<Vec<_>>()
                .join(", ")
        };
        if human.ready {
            let done = match state.kind {
                rules::SharedKind::Window => "Passed",
                rules::SharedKind::Commitment => "Confirmed",
                rules::SharedKind::Payment { .. } => "Ready",
            };
            return format!("{done} · waiting for {}", names(&state.outstanding()));
        }
        match &human.cleared_by {
            Some(changer) => format!("{label} again · {} changed it", names(changer)),
            None => label.to_owned(),
        }
    }

    /// Each shared decision's readiness by investigator, for the decision display.
    pub(super) fn readiness_lines(&self) -> Vec<String> {
        self.shared
            .iter()
            .filter(|state| state.active)
            .map(|state| {
                let what = match state.kind {
                    rules::SharedKind::Commitment => "Commitments",
                    rules::SharedKind::Window => "Window",
                    rules::SharedKind::Payment { .. } => "Clue payment",
                };
                let players: Vec<String> = state
                    .players
                    .iter()
                    .map(|player| {
                        let names: Vec<_> =
                            player.investigators.iter().map(|who| who.name()).collect();
                        let status = match state.kind {
                            rules::SharedKind::Window if player.ready => "passed",
                            rules::SharedKind::Window => "may act",
                            _ if player.ready => "ready",
                            _ => "deciding",
                        };
                        format!("{} {status}", names.join(" and "))
                    })
                    .collect();
                format!("{what}: {}", players.join(" · "))
            })
            .collect()
    }

    /// Cards another player's investigators have pending in a shared commitment.
    pub(super) fn others_pending(&self, view: &Observation) -> Vec<(Investigator, CardView)> {
        let Some(state) = self
            .shared
            .iter()
            .find(|state| state.kind == rules::SharedKind::Commitment)
        else {
            return Vec::new();
        };
        let others: Vec<Investigator> = state
            .players
            .iter()
            .filter(|player| player.player != Controller::Human)
            .flat_map(|player| player.investigators.iter().copied())
            .collect();
        let Some(play) = &view.play else {
            return Vec::new();
        };
        let Some(test) = &play.test else {
            return Vec::new();
        };
        play.players
            .iter()
            .filter(|player| others.contains(&player.investigator))
            .flat_map(|player| {
                player
                    .hand
                    .iter()
                    .filter(|card| test.selected.contains(&card.id))
                    .map(|card| (player.investigator, card.clone()))
            })
            .collect()
    }

    /// The history line as the action history shows it.
    pub(super) fn history_line(&self, index: usize, line: String) -> String {
        match self.attributed.iter().find(|(entry, _)| *entry == index) {
            Some((_, who)) if !line.starts_with(who.name()) => format!("{}: {line}", who.name()),
            _ => line,
        }
    }
}

/// Where the socket is claimed; tests name their own.
#[derive(Resource, Clone)]
pub(super) struct PartnerSocket(pub PathBuf);

/// Wakes the frame loop when a request arrives; native play wakes winit's event
/// loop, and tests count calls.
#[derive(Resource, Clone)]
pub(super) struct PartnerWaker(pub Arc<dyn Fn() + Send + Sync>);

/// One request line and where its answer goes.
pub(super) struct Incoming {
    pub line: String,
    pub reply: mpsc::Sender<String>,
}

/// The partner host at this table.
#[derive(Resource)]
pub(super) struct Partners {
    host: Host<mpsc::Sender<String>>,
    inbox: Mutex<mpsc::Receiver<Incoming>>,
    outbox: mpsc::Sender<Incoming>,
    /// Requests sent by connection threads and not yet drained.
    queued: Arc<AtomicUsize>,
    transport: Option<Transport>,
    /// The attempt the partner sessions joined; a restart ends them.
    attempt: u64,
    /// The decision last routed for the display: revision, control and sharing
    /// version, and what it showed.
    routed: Option<(u64, rules::Control, u64, Routed)>,
}

/// What the table shows of a routed decision.
#[derive(Clone, Debug, Default)]
struct Routed {
    /// The players it waits for, each with the investigator it answers for.
    awaited: Vec<(Controller, Option<Investigator>)>,
    may_use: Vec<(Investigator, Vec<String>)>,
    shared: Vec<rules::SharedState>,
}

impl Default for Partners {
    fn default() -> Self {
        let (outbox, inbox) = mpsc::channel();
        Self {
            host: Host::new(),
            inbox: Mutex::new(inbox),
            outbox,
            queued: Arc::new(AtomicUsize::new(0)),
            transport: None,
            attempt: 0,
            routed: None,
        }
    }
}

impl Partners {
    /// Whether the table has partner work it can do now: a request not yet read,
    /// or a held request whose moment has come.
    fn due(&self, session: &Session, settled: bool, now: Instant) -> bool {
        self.queued.load(Ordering::Acquire) > 0 || self.host.due(session, settled, now)
    }

    fn drain(&self) -> Vec<Incoming> {
        let inbox = self.inbox.lock().expect("Partner inbox");
        let incoming: Vec<_> = inbox.try_iter().collect();
        self.queued.fetch_sub(incoming.len(), Ordering::AcqRel);
        incoming
    }

    /// What the human's choice of `id` does: commit it as usual, or record the
    /// human's part in a shared decision, which the next [`serve`] completes once
    /// every participating player is ready.
    pub(super) fn human(
        &mut self,
        session: &Session,
        revision: u64,
        id: &str,
    ) -> Result<rules::HumanChoice, String> {
        self.host.human(session, revision, id)
    }
}

impl Drop for Partners {
    fn drop(&mut self) {
        // Leaving the table ends every partner session before the socket goes.
        for (reply, answer) in self.host.close("The table closed.") {
            let _ = reply.send(answer.to_string());
        }
        self.transport = None;
    }
}

/// The default socket: `$ARKHAM_PARTNER_SOCKET`, else `arkham-unfolded-partners.sock`
/// in `$XDG_RUNTIME_DIR` (or the temporary directory). The tools' `partner` command
/// finds the same path.
pub(super) fn default_socket() -> PathBuf {
    if let Some(path) = std::env::var_os("ARKHAM_PARTNER_SOCKET") {
        return PathBuf::from(path);
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("arkham-unfolded-partners.sock")
}

/// The line the human pastes into an agent's terminal. It names no investigator.
pub(super) fn line(socket: &Path) -> String {
    let option =
        if socket == default_socket() && std::env::var_os("ARKHAM_PARTNER_SOCKET").is_none() {
            String::new()
        } else {
            format!(" --socket {}", socket.display())
        };
    format!(
        "Join my Arkham Unfolded game as an agent partner: in the arkham-unfolded repository, read `scripts/tools partner --help`, then run `scripts/tools partner{option} join`."
    )
}

/// The partner host's frame: the human's partner controls, then every request
/// that arrived, then held requests, then what the table shows. Runs right after
/// the human's actions, before the scene is rebuilt.
pub(super) fn serve(world: &mut World) {
    let now = Instant::now();
    if !world.contains_resource::<Partners>() {
        world.init_resource::<Partners>();
    }
    let waker = waker(world);
    let socket = world
        .get_resource::<PartnerSocket>()
        .map_or_else(default_socket, |socket| socket.0.clone());
    let settled = settled(world);
    // Only this frame's own input and scene rebuild keep the table unsettled: work
    // waiting on settling (a choice, or a shared decision whose players are all
    // ready) can run on the next frame, which nothing else would request.
    let next_frame = !settled && {
        let pending = table_unsettled(world);
        let table = world.resource::<Table>();
        pending
            .iter()
            .all(|reason| matches!(*reason, "camera motion" | "scene rebuild" | "queued input"))
            && !table.modal()
            && !table.is_presenting()
            && !table.context.is_open()
    };
    let redraw = world.resource_scope(|world, mut partners: Mut<Partners>| {
        world.resource_scope(|_, mut table: Mut<Table>| {
            let partners = &mut *partners;
            let table = &mut *table;
            let mut answers = Vec::new();
            let mut closing = false;
            for input in std::mem::take(&mut table.partners.inputs) {
                match input {
                    Input::Menu => table.partners.menu_open = !table.partners.menu_open,
                    Input::Enable(true) if partners.transport.is_none() => {
                        match Transport::open(
                            &socket,
                            partners.outbox.clone(),
                            partners.queued.clone(),
                            waker.clone(),
                        ) {
                            Ok(transport) => {
                                partners.transport = Some(transport);
                                answers.extend(partners.host.set_enabled(true));
                                partners.attempt = table.attempt;
                                table.partners.problem = None;
                                eprintln!("Agent partners are enabled. {}", line(&socket));
                            }
                            Err(problem) => table.partners.problem = Some(problem),
                        }
                    }
                    Input::Enable(true) => {}
                    Input::Enable(false) => {
                        answers.extend(partners.host.set_enabled(false));
                        closing = true;
                    }
                    Input::Hand(who) => {
                        let selected = partners.host.selected(who);
                        partners.host.set_selected(who, !selected);
                    }
                    Input::TakeOver(who) => partners.host.set_selected(who, false),
                    Input::Speak => table.talk.open(),
                }
                table.dirty = true;
            }
            if table.leaving && partners.transport.is_some() {
                answers.extend(partners.host.close("The table closed."));
                answers.extend(partners.host.set_enabled(false));
                closing = true;
            } else if table.attempt != partners.attempt {
                answers.extend(
                    partners
                        .host
                        .close("The table restarted the attempt; join again."),
                );
                partners.attempt = table.attempt;
            }
            let mut sitting = Sitting { table, settled };
            if let Some(text) = sitting.table.talk.outgoing.take() {
                let said = partners.host.human_says(&text, &sitting);
                sitting.table.talk.said(said);
            }
            for incoming in partners.drain() {
                answers.extend(partners.host.receive(
                    &incoming.line,
                    incoming.reply,
                    &mut sitting,
                    now,
                ));
            }
            answers.extend(partners.host.poll(&mut sitting, now));
            sitting.table.talk.closed = partners.host.talk_closed(&sitting);
            if table.talk.hear(partners.host.table_talk())
                && table
                    .inspection
                    .as_ref()
                    .is_some_and(|(_, code)| code == talk::LOG)
            {
                // Menu → Table talk is open: show the new message there.
                table.dirty = true;
            }
            for (reply, answer) in answers {
                let _ = reply.send(answer.to_string());
            }
            if closing {
                partners.transport = None;
            }
            show(partners, table, &socket, now);
            next_frame && partners.host.due(&table.session, true, now)
        })
    });
    if redraw && let Some(mut redraw) = world.get_resource_mut::<Redraw>() {
        redraw.request();
    }
}

/// Whether a partner's choice may be applied now, and a wait for a decision
/// answered: the table would take the human's click. Camera motion does not
/// matter to a partner; an open menu holds only the presentation that follows.
fn settled(world: &mut World) -> bool {
    let pending = table_unsettled(world);
    let table = world.resource::<Table>();
    pending.iter().all(|reason| *reason == "camera motion")
        && !table.modal()
        && !table.is_presenting()
        && !table.context.is_open()
}

/// The seat steps frames only while the table is unsettled, so partner work it can
/// do now keeps the table unsettled.
pub(super) fn unsettled(world: &mut World) -> Option<&'static str> {
    if !world.contains_resource::<Partners>() {
        return None;
    }
    let settled = settled(world);
    let partners = world.resource::<Partners>();
    let session = &world.resource::<Table>().session;
    partners
        .due(session, settled, Instant::now())
        .then_some("partner request")
}

fn waker(world: &World) -> Option<PartnerWaker> {
    if let Some(waker) = world.get_resource::<PartnerWaker>() {
        return Some(waker.clone());
    }
    let proxy = world.get_resource::<bevy::winit::EventLoopProxyWrapper>()?;
    let proxy = (**proxy).clone();
    Some(PartnerWaker(Arc::new(move || {
        let _ = proxy.send_event(bevy::winit::WinitUserEvent::WakeUp);
    })))
}

/// Refresh what the table shows of partners, marking the scene for a rebuild only
/// when that changes.
fn show(partners: &mut Partners, table: &mut Table, socket: &Path, now: Instant) {
    let control = partners.host.control();
    if table.presentation.set_control(control.clone()) {
        table.dirty = true;
    }
    let revision = table.session.revision();
    let version = partners.host.sharing_version();
    let routed = if control.solo() {
        Routed::default()
    } else {
        match &partners.routed {
            Some((at, routed, shared, shown))
                if *at == revision && *routed == control && *shared == version =>
            {
                shown.clone()
            }
            _ => {
                let routed = route(&partners.host, &table.session, &control);
                partners.routed = Some((revision, control.clone(), version, routed.clone()));
                routed
            }
        }
    };
    let deciding: Vec<(Investigator, Deciding)> = routed
        .awaited
        .iter()
        .filter_map(|(controller, who)| {
            let who = (*who)?;
            Some((
                who,
                match controller {
                    Controller::Human => return None,
                    Controller::Unclaimed => Deciding::Unclaimed,
                    Controller::Partner(_) => {
                        let silence = partners.host.silence(who, now).unwrap_or_default();
                        match silence.as_secs() / 60 {
                            0 => Deciding::Thinking,
                            minutes => Deciding::Silent(minutes),
                        }
                    }
                },
            ))
        })
        .collect();
    let awaited = if deciding.is_empty() {
        Vec::new()
    } else {
        routed.awaited.iter().filter_map(|(_, who)| *who).collect()
    };
    let enabled = partners.host.enabled();
    let shown = Shown {
        menu_open: table.partners.menu_open,
        inputs: Vec::new(),
        enabled,
        selected: Investigator::ALL.map(|who| partners.host.selected(who)),
        joined: Investigator::ALL
            .map(|who| matches!(control.controller(who), Controller::Partner(_))),
        line: enabled.then(|| line(socket)),
        problem: table.partners.problem.clone(),
        deciding,
        may_use: routed.may_use,
        awaited,
        shared: routed.shared,
        // Compared below with what the table shows, so copied, not moved out of it.
        attributed: table.partners.attributed.clone(),
    };
    if shown != table.partners {
        table.dirty = true;
    }
    table.partners = shown;
}

/// Route the pending decision for the display: whom it waits for, what each
/// awaited partner may use in a window (the line the table shows without
/// partners as "Available now"), and its shared decisions.
fn route(host: &Host<mpsc::Sender<String>>, session: &Session, control: &rules::Control) -> Routed {
    let awaited = host.awaited(session);
    let routing = Routing::of(session, control);
    let observation = session.observe();
    let may_use = awaited
        .iter()
        .filter(|(controller, _)| *controller != Controller::Human)
        .filter_map(|(controller, who)| {
            let who = (*who)?;
            let theirs: Vec<&str> = routing
                .options()
                .filter(|(_, _, player)| *player == controller)
                .map(|(id, ..)| id)
                .collect();
            let mut window = observation.clone();
            window.choices.retain(|choice| {
                theirs.contains(&choice.id.as_str())
                    || matches!(choice.id.as_str(), "window:pass" | "phase-window:pass")
            });
            let own = format!("{} — ", who.name());
            let titles = controls::window_opportunities(&window)
                .into_iter()
                .map(|(_, label)| {
                    label
                        .strip_prefix(&own)
                        .map_or(label.clone(), str::to_owned)
                })
                .collect();
            Some((who, titles))
        })
        .collect();
    Routed {
        awaited,
        may_use,
        shared: host.shared(session),
    }
}

/// The table as the partner host reaches it.
struct Sitting<'a> {
    table: &'a mut Table,
    settled: bool,
}

impl rules::Table for Sitting<'_> {
    fn session(&self) -> &Session {
        &self.table.session
    }

    fn settled(&self) -> bool {
        self.settled
    }

    fn commit(&mut self, revision: u64, id: &str, by: Investigator) -> Result<(), String> {
        let before = self.table.session.observe();
        let label = before
            .choices
            .iter()
            .find(|choice| choice.id == id)
            .map(|choice| choice.label.clone());
        self.table.commit(revision, id)?;
        self.settled = false;
        let history = self.table.session.observe().history;
        let entry = (before.history.len()..history.len())
            .find(|index| Some(&history[*index]) == label.as_ref())
            .unwrap_or(before.history.len());
        if entry < history.len() {
            self.table.partners.attributed.push((entry, by));
        }
        // A partner's decision shows as an action line beside its investigator's
        // portrait (#209); the human's own decisions get none.
        if let Some(label) = label
            && matches!(
                self.table.presentation.control().controller(by),
                Controller::Partner(_)
            )
        {
            let round = before.play.as_ref().map(|play| play.round);
            self.table.talk.lines.push(talk::Line { by, label, round });
        }
        Ok(())
    }
}

/// The socket while partners are enabled: the claimed path, its listener thread
/// and the connections it accepted. Dropping it ends them and removes the path.
struct Transport {
    stop: Arc<AtomicBool>,
    listener: Option<JoinHandle<()>>,
    connections: Connections,
}

/// The connections being served, by number, kept to cut them when partners end.
type Connections = Arc<Mutex<Vec<(u64, UnixStream)>>>;

impl Transport {
    fn open(
        path: &Path,
        outbox: mpsc::Sender<Incoming>,
        queued: Arc<AtomicUsize>,
        waker: Option<PartnerWaker>,
    ) -> Result<Self, String> {
        let claimed = crate::agent_seat::socket::Listener::claim(path).map_err(|error| {
            error.replace(
                "an agent seat already answers there and was left untouched; quit that seat or choose another path",
                "something already answers there and was left untouched; turn agent partners off in that game, or quit the agent seat using that path",
            )
        })?;
        claimed
            .unix_listener()
            .set_nonblocking(true)
            .map_err(|error| format!("Cannot listen on {}: {error}", path.display()))?;
        let stop = Arc::new(AtomicBool::new(false));
        let connections: Connections = Arc::new(Mutex::new(Vec::new()));
        let listener = {
            let (stop, connections) = (stop.clone(), connections.clone());
            thread::Builder::new()
                .name("agent partners".into())
                .spawn(move || {
                    // The claim lives as long as this thread and is dropped with it.
                    let claimed = claimed;
                    let mut number = 0;
                    while !stop.load(Ordering::Acquire) {
                        match claimed.unix_listener().accept() {
                            Ok((stream, _)) => {
                                number += 1;
                                accept(
                                    number,
                                    stream,
                                    &connections,
                                    &outbox,
                                    &queued,
                                    waker.as_ref(),
                                );
                            }
                            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(20));
                            }
                            Err(error) => {
                                eprintln!("Agent partners: a connection failed: {error}");
                                thread::sleep(Duration::from_millis(100));
                            }
                        }
                    }
                })
                .map_err(|error| format!("Cannot start the agent-partner listener: {error}"))?
        };
        Ok(Self {
            stop,
            listener: Some(listener),
            connections,
        })
    }
}

/// Serve one accepted connection on a thread of its own.
fn accept(
    number: u64,
    stream: UnixStream,
    connections: &Connections,
    outbox: &mpsc::Sender<Incoming>,
    queued: &Arc<AtomicUsize>,
    waker: Option<&PartnerWaker>,
) {
    let Ok(kept) = stream.try_clone() else {
        return;
    };
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    connections
        .lock()
        .expect("Partner connections")
        .push((number, kept));
    let (registry, outbox, queued, waker) = (
        connections.clone(),
        outbox.clone(),
        queued.clone(),
        waker.cloned(),
    );
    let spawned = thread::Builder::new()
        .name("agent partner".into())
        .spawn(move || {
            converse(&stream, &outbox, &queued, waker.as_ref());
            // Every answer is written: end the connection for its partner too.
            let _ = stream.shutdown(Shutdown::Both);
            registry
                .lock()
                .expect("Partner connections")
                .retain(|(kept, _)| *kept != number);
        });
    if spawned.is_err() {
        connections
            .lock()
            .expect("Partner connections")
            .retain(|(kept, _)| *kept != number);
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(listener) = self.listener.take() {
            let _ = listener.join();
        }
        // Connections write the answers already sent to them, then end when their
        // partner closes its side; a connection still open after a second is cut.
        let deadline = Instant::now() + Duration::from_secs(1);
        while !self
            .connections
            .lock()
            .expect("Partner connections")
            .is_empty()
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(10));
        }
        for (_, stream) in self
            .connections
            .lock()
            .expect("Partner connections")
            .drain(..)
        {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

/// One connection: each request line goes to the frame loop, and its answer comes
/// back as one line. A wait the table does not answer within its period plus a
/// margin (a seat that runs no frames) is answered here with nothing yet.
fn converse(
    stream: &UnixStream,
    outbox: &mpsc::Sender<Incoming>,
    queued: &AtomicUsize,
    waker: Option<&PartnerWaker>,
) {
    let mut writer = stream;
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else {
            return;
        };
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        let limit = (request["op"] == "wait").then(|| {
            request["timeout_seconds"]
                .as_u64()
                .map_or(rules::DEFAULT_WAIT, Duration::from_secs)
                .min(rules::LONGEST_WAIT)
                + Duration::from_secs(5)
        });
        let (reply, answer) = mpsc::channel();
        queued.fetch_add(1, Ordering::AcqRel);
        if outbox.send(Incoming { line, reply }).is_err() {
            queued.fetch_sub(1, Ordering::AcqRel);
            let _ = writeln!(writer, "{}", closed());
            return;
        }
        if let Some(waker) = waker {
            (waker.0)();
        }
        let answer = match limit {
            Some(limit) => answer.recv_timeout(limit).map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => json!({
                    "ok": true, "mode": "partner", "status": "nothing_yet",
                    "partner": request["partner"],
                })
                .to_string(),
                mpsc::RecvTimeoutError::Disconnected => closed(),
            }),
            None => answer.recv().map_err(|_| closed()),
        };
        let ended = answer.is_err();
        let text = answer.unwrap_or_else(|text| text);
        if writeln!(writer, "{text}")
            .and_then(|()| writer.flush())
            .is_err()
            || (ended && text.contains("\"closed\""))
        {
            return;
        }
    }
}

fn closed() -> String {
    json!({"ok": true, "mode": "partner", "status": "closed", "reason": "The table closed."})
        .to_string()
}

#[cfg(test)]
mod tests;
