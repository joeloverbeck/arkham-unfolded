//! The window's side of agent partners (#205, [ADR 0007]): the private socket, the
//! human's controls for partners, and what the table shows of them. What a partner may
//! do and see is `crate::partners`, which this host drives once per frame against the
//! table.
//!
//! The socket exists only while partners are enabled. `unfolded-tabletop`'s
//! [`agent_transport`](unfolded_tabletop::agent_transport) claims it, accepts any number
//! of connections, carries their request lines to [`serve`] and wakes the frame loop
//! ([ADR 0008]); this module gives it the partner protocol's own words.
//! Held waits and choices never block a frame. A wait tells a partner of its
//! decision once the table has presented it, while the human may still be reading
//! or browsing; its choice is applied only once the table has settled with none of
//! that open, through [`Table::commit`], the path the human's clicks take (#210).
//!
//! The host is part of [`install`], so the agent seat can host partners too.
//!
//! [ADR 0007]: ../../docs/adr/0007-agent-partners-join-the-running-table.md
//! [ADR 0008]: ../../docs/adr/0008-partner-transport-moves-to-the-shared-library.md

use super::*;
use crate::gathering::CardView;
use crate::partner_folders::{self, Places};
use crate::partners::{
    Control, Controller, Host, HumanChoice, SharedKind, SharedState, nothing_yet,
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use unfolded_tabletop::agent_transport::{
    Protocol, Reply, Transport, Unopened, Waker, window_waker,
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
    /// The line to paste into an agent's terminal, while partners are enabled and the
    /// partner folders are ready for it.
    pub line: Option<String>,
    /// Why partners could not be enabled.
    pub problem: Option<String>,
    /// What to do before a session joins, when the partner folders are not ready.
    pub notice: Option<String>,
    /// Each partner-played investigator the table waits on, and how.
    pub deciding: Vec<(Investigator, Deciding)>,
    /// What each awaited partner's investigator may use in the open window: the
    /// titles of the cards and abilities its options use.
    pub may_use: Vec<(Investigator, Vec<String>)>,
    /// Every investigator the table waits on when a partner's is among them, for
    /// the status line.
    pub awaited: Vec<Investigator>,
    /// The pending shared decisions whose participants several players control.
    pub shared: Vec<SharedState>,
    /// The label of the human's choice in his own decision while it waits for
    /// the other players with an option there to pass (F1).
    pub held: Option<String>,
    /// Why the human's held choice was not applied, while he chooses again: another
    /// player acted first (review fix G2).
    pub released: Option<String>,
    /// History entries a partner's decision made in this sitting, and its
    /// investigator. Runtime only: the save's history is replayed from decisions.
    pub attributed: Vec<(usize, Investigator)>,
}

/// What `who` may use before `before`: a heading and then the titles, since a title
/// is a card's name or an option's own label, such as "Use Jacqueline — reveal 3
/// tokens, then cancel" (#217).
pub(super) fn available(who: &str, before: &str, titles: &[String]) -> String {
    format!("Available to {who} before {before}: {}", titles.join(" · "))
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
        (!titles.is_empty()).then(|| available(who.name(), "this window closes", titles))
    }

    /// The human's part in the shared decision `id` completes, when it has one.
    pub(super) fn shared_with_human(&self, id: &str) -> Option<&SharedState> {
        self.shared
            .iter()
            .find(|state| state.close == id && state.of(&Controller::Human).is_some())
    }

    /// In another player's decision, while the human holds an option there and
    /// has not passed: what his pass comes before, such as "Tony Morgan's action"
    /// (F1). His options there, his pass among them, are the view's.
    pub(super) fn human_window(&self) -> Option<String> {
        self.shared
            .iter()
            .filter(|state| {
                state
                    .of(&Controller::Human)
                    .is_some_and(|human| !human.ready)
            })
            .find_map(|state| state.kind.before())
    }

    /// The shared clue payment the human takes part in, while partners share it.
    pub(super) fn payment_with_human(&self) -> Option<&SharedState> {
        self.shared.iter().find(|state| {
            matches!(state.kind, SharedKind::Payment { .. })
                && state.of(&Controller::Human).is_some()
        })
    }

    /// Whether the player of `who` is ready on the shared payment the human takes
    /// part in, while partners share it.
    pub(super) fn payment_ready(&self, who: Investigator) -> Option<bool> {
        self.payment_with_human()?
            .players
            .iter()
            .find(|player| player.investigators.contains(&who))
            .map(|player| player.ready)
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
                SharedKind::Window | SharedKind::Before { .. } => "Passed",
                SharedKind::Commitment => "Confirmed",
                SharedKind::Payment { .. } => "Ready",
            };
            let outstanding = state.outstanding();
            // Once every other player has passed, the decider's choice is next.
            let waiting = match state.kind.before() {
                Some(before) if outstanding.is_empty() => before,
                _ => names(&outstanding),
            };
            return format!("{done} · waiting for {waiting}");
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
                    SharedKind::Commitment => "Commitments".to_owned(),
                    SharedKind::Window => "Window".to_owned(),
                    SharedKind::Payment { .. } => "Clue payment".to_owned(),
                    SharedKind::Before { .. } => {
                        format!("Before {}", state.kind.before().unwrap_or_default())
                    }
                };
                let players: Vec<String> = state
                    .players
                    .iter()
                    .map(|player| {
                        let names: Vec<_> =
                            player.investigators.iter().map(|who| who.name()).collect();
                        let status = match state.kind {
                            SharedKind::Window | SharedKind::Before { .. } if player.ready => {
                                "passed"
                            }
                            SharedKind::Window | SharedKind::Before { .. } => "may act",
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
            .find(|state| state.kind == SharedKind::Commitment)
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

/// Wakes the frame loop when a request arrives and when the client of a held one
/// hangs up; native play wakes winit's event loop, and tests count calls.
#[derive(Resource, Clone)]
pub(super) struct PartnerWaker(pub Waker);

/// The partner host's clock, for waits, silence and pacing of partner work: the
/// monotonic clock natively; tests set it ahead to show a silent partner.
#[derive(Resource, Clone)]
pub(super) struct PartnerClock(pub Arc<dyn Fn() -> Instant + Send + Sync>);

fn now(world: &World) -> Instant {
    world
        .get_resource::<PartnerClock>()
        .map_or_else(Instant::now, |clock| (clock.0)())
}

/// The partner host at this table.
#[derive(Resource)]
pub(super) struct Partners {
    host: Host<Reply>,
    /// The socket while partners are enabled.
    transport: Option<Transport>,
    /// The attempt the partner sessions joined; a restart ends them.
    attempt: u64,
    /// The decision last routed for the display, and what it showed.
    routed: Option<RoutedAt>,
}

/// A routed decision, kept until its revision, the control or a shared
/// decision's readiness changes.
struct RoutedAt {
    revision: u64,
    control: Control,
    /// The host's sharing version.
    sharing: u64,
    /// The human's choice waiting in his own turn.
    held: Option<String>,
    routed: Routed,
}

/// What the table shows of a routed decision.
#[derive(Clone, Debug, Default)]
struct Routed {
    /// The players it waits for, each with the investigator it answers for.
    awaited: Vec<(Controller, Option<Investigator>)>,
    may_use: Vec<(Investigator, Vec<String>)>,
    shared: Vec<SharedState>,
    held: Option<String>,
}

impl Default for Partners {
    fn default() -> Self {
        Self {
            host: Host::new(),
            transport: None,
            attempt: 0,
            routed: None,
        }
    }
}

impl Partners {
    /// Whether the table has partner work it can do now: a request not yet read, a held
    /// wait whose client has died, or a held request whose moment has come.
    fn due(&self, session: &Session, (presented, settled): (bool, bool), now: Instant) -> bool {
        self.transport.as_ref().is_some_and(Transport::waiting)
            || self.host.hung_up(Reply::hung_up)
            || self.host.due(session, presented, settled, now)
    }

    /// What the human's choice of `id` does: commit it as usual, or record the
    /// human's part in a shared decision, which the next [`serve`] completes once
    /// every participating player is ready.
    pub(super) fn human(
        &mut self,
        session: &Session,
        revision: u64,
        id: &str,
    ) -> Result<HumanChoice, String> {
        self.host.human(session, revision, id)
    }
}

impl Drop for Partners {
    fn drop(&mut self) {
        // Leaving the table ends every partner session before the socket goes.
        for (reply, answer) in self.host.close("The table closed.") {
            reply.send(answer.to_string());
        }
        self.transport = None;
    }
}

/// The default socket: `$ARKHAM_PARTNER_SOCKET`, else `arkham-unfolded-partners.sock`
/// in `$XDG_RUNTIME_DIR` (or the temporary directory). `arkham-partner` finds
/// the same path.
pub(super) fn default_socket() -> PathBuf {
    if let Some(path) = std::env::var_os("ARKHAM_PARTNER_SOCKET") {
        return PathBuf::from(path);
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("arkham-unfolded-partners.sock")
}

/// What the human is to do before a session joins, when the partner folders in `data` are
/// absent or hold other instructions than this game's. What `partner-folder` wrote
/// besides, and the program, are not looked at.
fn folders_notice(data: Option<&Path>) -> Option<String> {
    let current = data.is_some_and(|data| {
        let places = Places::of(data);
        let expected = places.instructions();
        [
            places.claude_code.join("CLAUDE.md"),
            places.codex.join("AGENTS.md"),
        ]
        .into_iter()
        .all(|file| std::fs::read_to_string(file).is_ok_and(|held| held == expected))
    });
    (!current).then(|| {
        "The partner folders are absent or not this game's: run scripts/tools partner-folder, then hide and show agent partners here for the line to paste.".to_owned()
    })
}

/// Where the partner folders are looked for, when it is not Arkham's data directory: a
/// test's own.
#[derive(Resource)]
pub(super) struct PartnerData(pub(super) PathBuf);

/// The line the human pastes into an agent's terminal, once per investigator (#295).
/// It points to the partner folders' instructions, each tool's folder by its full path
/// (#306), and names no investigator and no human player (#211).
pub(super) fn line(socket: &Path) -> String {
    let option =
        if socket == default_socket() && std::env::var_os("ARKHAM_PARTNER_SOCKET").is_none() {
            String::new()
        } else {
            format!(", with `--socket {}` on every command", socket.display())
        };
    let started = partner_folders::data().map_or_else(
        || "started in Claude Code or Codex in that tool's partner folder".to_owned(),
        |data| {
            let places = Places::of(&data);
            format!(
                "started in Claude Code in {} or in Codex in {}",
                places.claude_code.display(),
                places.codex.display()
            )
        },
    );
    format!(
        "Play one investigator at the Arkham Unfolded table, one session per investigator, {started}: follow the instructions there and join{option}."
    )
}

/// The partner host's frame: the human's partner controls, then every request
/// that arrived, then held requests, then what the table shows. Runs right after
/// the human's actions, before the scene is rebuilt.
pub(super) fn serve(world: &mut World) {
    let now = now(world);
    if !world.contains_resource::<Partners>() {
        world.init_resource::<Partners>();
    }
    let waker = waker(world);
    let socket = world
        .get_resource::<PartnerSocket>()
        .map_or_else(default_socket, |socket| socket.0.clone());
    let (presented, settled) = (presented(world), settled(world));
    // Only this frame's own input and scene rebuild keep the table unsettled: work
    // waiting on settling (a choice, or a shared decision whose players are all
    // ready) can run on the next frame, which nothing else would request.
    let next_frame = !settled
        && settled_but(
            world,
            &[
                Unsettled::CameraMotion,
                Unsettled::SceneRebuild,
                Unsettled::QueuedInput,
            ],
        );
    let partner_data = world
        .get_resource::<PartnerData>()
        .map_or_else(partner_folders::data, |data| Some(data.0.clone()));
    let redraw = world.resource_scope(|world, mut partners: Mut<Partners>| {
        world.resource_scope(|_, mut table: Mut<Table>| {
            let partners = &mut *partners;
            let table = &mut *table;
            let mut answers = Vec::new();
            let mut closing = false;
            for input in std::mem::take(&mut table.partners.inputs) {
                match input {
                    Input::Menu => {
                        table.partners.menu_open = !table.partners.menu_open;
                        if partners.transport.is_some() {
                            table.partners.notice = folders_notice(partner_data.as_deref());
                        }
                    }
                    Input::Enable(true) if partners.transport.is_none() => {
                        match open(&socket, waker.clone()) {
                            Ok(transport) => {
                                partners.transport = Some(transport);
                                answers.extend(partners.host.set_enabled(true));
                                partners.attempt = table.attempt;
                                table.partners.problem = None;
                                table.partners.notice = folders_notice(partner_data.as_deref());
                                eprintln!("Agent partners are enabled. {}", line(&socket));
                                if let Some(notice) = &table.partners.notice {
                                    eprintln!("{notice}");
                                }
                            }
                            Err(problem) => table.partners.problem = Some(problem),
                        }
                    }
                    Input::Enable(true) => {}
                    Input::Enable(false) => {
                        answers.extend(partners.host.set_enabled(false));
                        table.partners.notice = None;
                        closing = true;
                    }
                    Input::Hand(who) => {
                        let selected = partners.host.selected(who);
                        answers.extend(partners.host.set_selected(who, !selected));
                    }
                    Input::TakeOver(who) => {
                        answers.extend(partners.host.set_selected(who, false));
                    }
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
            let mut sitting = Sitting {
                table,
                presented,
                settled,
                committed: false,
            };
            if let Some(text) = sitting.table.talk.outgoing.take() {
                let said = partners.host.human_says(&text, &sitting);
                sitting.table.talk.said(said);
            }
            // Before anything is told to it.
            partners.host.end_hung_up(Reply::hung_up, now);
            let arrived = partners
                .transport
                .as_ref()
                .map(Transport::take)
                .unwrap_or_default();
            for incoming in arrived {
                answers.extend(partners.host.receive(
                    &incoming.line,
                    incoming.reply,
                    &mut sitting,
                    now,
                ));
            }
            answers.extend(partners.host.poll(&mut sitting, now));
            sitting.table.talk.closed = partners.host.talk_closed(&sitting);
            let committed = sitting.committed;
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
                reply.send(answer.to_string());
            }
            if closing {
                partners.transport = None;
            }
            show(partners, table, &socket, now);
            (
                next_frame && partners.host.due(&table.session, true, true, now),
                committed,
            )
        })
    });
    let (redraw, committed) = redraw;
    if redraw && let Some(mut redraw) = world.get_resource_mut::<Redraw>() {
        redraw.request();
    }
    if committed {
        // A partner's decision does not move the camera; Follow frames what follows
        // it only when that is the human's own decision or test.
        let table = world.resource::<Table>();
        let framing = play::follow(&table.view(), table.presentation.control());
        if let Some(mut orbit) = world.get_resource_mut::<Orbit>() {
            orbit.update_follow_target(framing);
        }
    }
}

/// Whether a wait may tell a partner of its decision: the table has finished
/// presenting it. A reading counting down, or one waiting for the human, and the
/// human's Inspect or menus may still be open, so the partner thinks meanwhile
/// (#210). Camera motion does not matter to a partner.
fn presented(world: &mut World) -> bool {
    let pending = table_unsettled(world);
    let table = world.resource::<Table>();
    pending
        .iter()
        .all(|reason| matches!(reason, Unsettled::CameraMotion | Unsettled::ReadingDelay))
        && !table.is_presenting()
}

/// Whether a partner's choice may be applied now: the table would take the human's
/// click, and nothing of the human's is open over it. A reading, Inspect, the
/// history, the menu or a card's menu holds the choice until it closes, so its
/// consequences never start presenting unseen behind it (#210).
fn settled(world: &mut World) -> bool {
    settled_but(world, &[Unsettled::CameraMotion])
}

/// Whether the table is settled apart from `allowed`, with nothing of the human's
/// open over it.
fn settled_but(world: &mut World, allowed: &[Unsettled]) -> bool {
    let pending = table_unsettled(world);
    let table = world.resource::<Table>();
    pending.iter().all(|reason| allowed.contains(reason))
        && !table.modal()
        && !table.menu_open
        && !table.is_presenting()
        && !table.context.is_open()
}

/// The seat steps frames only while the table is unsettled, so partner work it can
/// do now keeps the table unsettled.
pub(super) fn unsettled(world: &mut World) -> Option<Unsettled> {
    if !world.contains_resource::<Partners>() {
        return None;
    }
    let readiness = (presented(world), settled(world));
    let partners = world.resource::<Partners>();
    let session = &world.resource::<Table>().session;
    partners
        .due(session, readiness, now(world))
        .then_some(Unsettled::PartnerRequest)
}

fn waker(world: &World) -> Option<Waker> {
    world
        .get_resource::<PartnerWaker>()
        .map(|waker| waker.0.clone())
        .or_else(|| window_waker(world))
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
        let held = partners.host.human_held(&table.session).map(str::to_owned);
        match &partners.routed {
            Some(kept)
                if kept.revision == revision
                    && kept.control == control
                    && kept.sharing == version
                    && kept.held == held =>
            {
                kept.routed.clone()
            }
            _ => {
                let routed = route(&partners.host, &table.session, &control);
                partners.routed = Some(RoutedAt {
                    revision,
                    control: control.clone(),
                    sharing: version,
                    held,
                    routed: routed.clone(),
                });
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
                        let silence = partners
                            .host
                            .silence(who, Some(revision), now)
                            .unwrap_or_default();
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
        line: (enabled && table.partners.notice.is_none()).then(|| line(socket)),
        problem: table.partners.problem.clone(),
        notice: table.partners.notice.clone(),
        deciding,
        may_use: routed.may_use,
        awaited,
        shared: routed.shared,
        held: routed.held,
        released: partners
            .host
            .human_released(&table.session)
            .map(str::to_owned),
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
fn route(host: &Host<Reply>, session: &Session, control: &Control) -> Routed {
    let awaited = host.awaited(session);
    let observation = session.observe();
    let shared = host.shared(session);
    // Before another player's decision the rules session lists no pass: what a
    // player may use there is every option of theirs (F1).
    let before_decision = |who: Investigator| {
        shared.iter().any(|state| {
            matches!(state.kind, SharedKind::Before { .. })
                && state
                    .players
                    .iter()
                    .any(|player| player.investigators.contains(&who))
        })
    };
    // Each player's options as the partner library gives them; the table only
    // names the cards and abilities they use.
    let uses = |player: &Controller| {
        let mut window = observation.clone();
        window.choices = control.choices_of(session, player);
        window
    };
    let held = host.human_held(session).map(|id| {
        observation
            .choices
            .iter()
            .find(|choice| choice.id == id)
            .map_or_else(|| id.to_owned(), |choice| choice.label.clone())
    });
    let may_use = awaited
        .iter()
        .filter(|(controller, _)| *controller != Controller::Human)
        .filter_map(|(controller, who)| {
            let who = (*who)?;
            let window = uses(controller);
            let own = format!("{} — ", who.name());
            let opportunities = if before_decision(who) {
                controls::opportunities(&window)
            } else {
                controls::window_opportunities(&window)
            };
            let titles = opportunities
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
        shared,
        held,
    }
}

/// The table as the partner host reaches it.
struct Sitting<'a> {
    table: &'a mut Table,
    presented: bool,
    settled: bool,
    /// Whether a decision was committed through this sitting.
    committed: bool,
}

impl crate::partners::Table for Sitting<'_> {
    fn session(&self) -> &Session {
        &self.table.session
    }

    fn presented(&self) -> bool {
        self.presented
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
        self.presented = false;
        self.settled = false;
        self.committed = true;
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

/// Open the partners' socket, or say why not.
fn open(path: &Path, waker: Option<Waker>) -> Result<Transport, String> {
    Transport::open(path, PartnerProtocol, waker).map_err(|refused| match refused {
        Unopened::Unclaimed(refused) => match refused.cause() {
            crate::agent_seat::socket::Unclaimable::Answered => format!(
                "Cannot listen on {}: something already answers there and was left untouched; turn agent partners off in that game, or quit the agent seat using that path",
                refused.path().display()
            ),
            _ => refused.to_string(),
        },
        Unopened::Unaccepting(error) => format!("Cannot listen on {}: {error}", path.display()),
        Unopened::Unstarted(error) => format!("Cannot start the agent-partner listener: {error}"),
        other => other.to_string(),
    })
}

/// What the transport says itself, in the partner protocol's words. A wait the table
/// does not answer within the period it asked for plus a margin (a seat that runs no
/// frames) is answered with nothing yet.
struct PartnerProtocol;

impl Protocol for PartnerProtocol {
    fn players(&self) -> &str {
        "Agent partners"
    }

    fn closed(&self) -> String {
        json!({"ok": true, "mode": "partner", "status": "closed", "reason": "The table closed."})
            .to_string()
    }

    fn unanswered(&self, request: &str) -> Option<(Duration, String)> {
        let request: Value = serde_json::from_str(request).ok()?;
        // A submission that waits, `choose` or `ready`, as much as a wait.
        let waits = request["op"] == "wait" || request["wait"] == true;
        let period = Duration::from_secs(request["timeout_seconds"].as_u64()?);
        let partner = request["partner"].as_str()?;
        waits.then(|| {
            let within = period.saturating_add(Duration::from_secs(5));
            let mut answer = nothing_yet(partner);
            // The table may never have read the choice, so nothing is known of it here.
            if request["wait"] == true {
                answer["choice"] = json!({"status": "unknown"});
                if request["op"] == "choose" {
                    answer["choice"]["id"] = request["id"].clone();
                }
            }
            (within, answer.to_string())
        })
    }
}

#[cfg(test)]
pub(super) mod tests;
