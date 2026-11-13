# Agent partners: investigation and interview record

Investigated on 2026-11-13 against `4e0c9cc7ee01ddff1252288f21dcdf2c65ae0eb8`,
with `unfolded-tabletop` at the pinned `48c9b9ab2b36d573bbc0a441b687896032a7538e`.
This note is investigation evidence and a completed design-interview record. It
is not an implementation, and it is not acceptance of co-play.

- [ADR 0007](../adr/0007-agent-partners-join-the-running-table.md) records the architectural choice.
- The [play contract](../foundation/play-contract.md#agent-partners) owns the runtime policy.
- The [presentation contract](../tabletop-presentation.md#agent-partners-and-table-talk) owns the table conventions.
- The spec issue that follows this interview owns scope, appetite and acceptance.

The interview read the complete constitution at revision 2026-09-30.1. The rules
that shaped the result:

- `AUTH-1`, `AUTH-3`: runtime choices made by a language model, under a policy the human accepts.
- `REP-2`: the boundary between what an LLM may say and what it may decide.
- `PURP-2`, `PURP-3`, `WORK-1`, `WORK-3`: outcome, primary path and the continuation after the allocation tripwire.
- `EVID-3`, `EVID-4`, `EVID-8`, `EVID-10`: first, second and return use; burden; a participant disappearing.
- `BOUND-1`, `BOUND-4`: one product; removable by deleting its module.
- `INT-1`, `EVID-6`: sharing with `unfolded-tabletop` and mudsong.
- `AGENT-5`: delegated judgement.

## The question

Joel sometimes wants a three- or four-investigator game while concentrating on a
single investigator. He asked what it would take for Claude or Codex to play the
other investigators as players at the table, with speech boxes beside the
investigator portraits. He assumed the agent seat's socket would be the route.

## Code facts that decided the shape

Read at the revision above. Line references are omitted because they drift.

**The rules session.**
- It has one pending choice list at a time. Each `Choice` has an id, a label and
  cancel faces, but no answering investigator.
- `decision_owner` is derived for display only, and nothing enforces it.
- One list can mix investigators' options. The skill-test commitment window, for
  example, lists every eligible investigator's cards, and the test's own
  investigator confirms them.
- Several decisions have no owner at all: the lead at setup, the next turn, a new
  lead after elimination, Lita's recipient, and player windows.
- A group clue payment is one template choice holding every investigator's
  contribution. Mulligans are sequential, in player order from the lead.

**The driver's observation.**
- It shows all three hands and carries no card text.
- It repeats the whole history every time: about 8 KB at mulligan, 13 KB in
  round 1 and 29 KB in round 2, where 12 KB of that is history.

**Commits and Undo.**
- A commit validates the revision, saves, then installs the result.
- The save refuses a writer whose baseline has changed. The window re-reads the
  save only on Reopen, so a second process committing to the same attempt would
  be refused or go unseen.
- Undo is global. It reverts the latest decision, whoever made it.

**The seat in `unfolded-tabletop`.**
- `Listener::claim`, which handles socket path rules, a private directory and
  replacing a dead socket, works with any protocol.
- The request protocol, the serving loop and `Seat` itself are private and
  hard-wired to the pointer protocol.
- The seat calls `app.update()` itself and drives the human's pointer, so it
  cannot attach to a windowed application run by winit.

**The native table.**
- It has no IPC today. Its only cross-thread pattern is the artwork-folder
  chooser's `mpsc` receiver, polled each frame.
- `unsettled` names every condition that is still busy. Today only the seat uses
  it.
- Input is dropped silently during announcements, while consequences resolve,
  and under modals.
- Readings wait for Continue and are not owned by any investigator.
- The portrait rail is a rebuilt panel 124 px wide at the left. To its right the
  3D scene fills the window.
- The phase banner, the nearest existing overlay, blocks every action for its
  two seconds, so it is no model for speech boxes.

## Rules sources

These are local evidence copies, outside Git. Their identities:

| Source | SHA-256 |
| --- | --- |
| Rules Reference AHC60 (`local/issue-1-evidence/sources/rules-reference.pdf`) | `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a` |
| Learn to Play AHC60, Revised Core (source archive `sce-base-save/Learn to Play--PDFUrl--70413338260775a14e18.pdf`, identical to FFG's current download) | `f82790f81af3731f11f0a7ddc267feb0644b47d20842a973f1023de709568ab0` |
| Legacy FAQ 2.5 (`local/issue-1-evidence/sources/faq-2.5.pdf`) | `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7` |

Issue #1 did not reconcile the Learn to Play, and the Rules Reference outranks
it. Neither the table-talk guidance nor any other choice below rests on the Learn
to Play alone for a rules outcome.

- **Authority and table talk.** LTP p. 14 says "each individual player has final
  authority over the actions their investigator performs", and the investigators
  may plan as a group. The same page's Table Talk section *encourages* players
  not to name, read out or allude to hidden cards; it does not forbid it.
- **Peril.** The only hard restriction is RR p. 16: during a peril encounter an
  investigator cannot confer, and others cannot play cards, trigger abilities or
  commit to that test.
- **The lead and group choices.** RR p. 14 and FAQ §1.5 make the lead the final
  arbiter of choices with several valid options. RR p. 24, step 2.2: "The
  investigators choose among themselves" who takes the next turn.
- **Player order.** RR p. 13 starts with the lead, then goes clockwise.
- **Commits to another investigator's test.** Under RR p. 26, ST.2, another
  investigator at the same location may commit one card. No rule requires
  permission, and the committing player decides.
- **Group clue payments.** RR p. 7: any or all investigators may contribute any
  number of clues.
- **One player, several investigators.** No official rule covers this. The texts
  assume one player per investigator.

The lead's arbitration of the next turn is therefore a house practice standing in
for "choose among themselves". The play contract labels it that way.

## How agent tools can wait

Read from official documentation on 2026-11-13. These are vendor facts that can
change, and some are uncertain.

**Claude Code** (code.claude.com docs: tools reference, MCP, channels, scheduled
tasks, context window).
- A Bash command that runs past its timeout moves to the background, and a
  background command re-invokes the session when it exits. An agent can
  therefore wait for its turn without spending tokens.
- An MCP call that runs past two minutes auto-backgrounds and wakes an idle
  session. Its timeouts can be configured.
- Channels can push events into a session, but they are a research preview, and
  custom channels need a development flag and do not work with `-p`.
- Auto-compaction keeps long sessions going. A partner's own notes and small
  responses limit the cost of re-reading state.

**Codex** (learn.chatgpt.com docs: configuration reference, MCP, non-interactive
mode, app server).
- No per-command shell timeout is documented. Waiting means polls bounded by
  `background_terminal_max_timeout`, five minutes by default, each costing a
  model turn.
- MCP `tool_timeout_sec` defaults to 60 s and can be raised. No push into a
  running session is documented.
- `codex app-server` lets an external process drive turns. That would be the
  reliable wait, but it is more machinery.
- Codex was not exercised: Joel's Codex usage was exhausted during the
  interview. Its path stays unverified until a partner session runs it.

A single shell command serves both tools. An MCP server and an app-server wrapper
were deferred: either would add a second interface to maintain.

## Decisions and their authority

| Decision | Authority |
| --- | --- |
| Outcome: play one investigator in a full party while agent partners play the others; occasional, per session; playing all three remains the default | Joel, explicit |
| Mission and play-contract boundary: hand chosen investigators to partners locally, with no network multiplayer | Joel, explicit |
| Assignment per session, not saved; any split; Take over; a stalled partner waits visibly | Joel, explicit |
| One session may play several investigators; separate sessions may each claim one | Joel, explicit |
| Partners are genuine players with their own judgement, who may disagree with or decline the human's requests | Joel, explicit |
| Partners speak sparingly, as players, not in character | Joel, explicit |
| Speech boxes beside the portraits; a session log; not saved | Joel, explicit |
| The human writes back and every player hears every message, including partner-to-partner messages | Joel, explicit |
| A new spec issue, prepared for `/to-spec`, decomposed with `/to-tickets`. Gathering already plays to resolution, and co-play lets Joel playtest while focusing on one investigator | Joel, explicit (the owner continuation `WORK-3` requires) |
| The window hosts the socket; partners choose at the decision level; the seat is unchanged; an ADR | Joel, explicit, on the recommendation |
| Starting from **Menu → Agent partners**, with a prompt to paste | Joel, explicit |
| The human sees everything as today; a partner sees only its own hidden information plus public information | Joel, explicit |
| No limit on waiting; partners take the time they take | Joel, explicit |
| Build in Arkham first; extract later if mudsong uses partners, which Joel expects eventually | Joel, explicit |
| The lead's player is the arbiter of group choices; the lead is chosen at setup in the window; a new lead after elimination comes from player order | Joel, explicit, on the recommendation |
| Partners know other players only by their investigators; deduction by elimination is accepted | Joel, explicit, preferring more anonymity where possible |
| Appetite of about 24 active agent hours and three of Joel's play sessions; `OPS-4` per ticket | Joel, explicit |
| Success: first use with Claude partners; a materially different second use; return use; Codex once usage allows | Joel, explicit; the second-use form delegated |
| Test seams: the partner protocol, tested headless and against the table host without a graphics card | Joel, explicit |
| The shared-decision readiness mechanism, filling seats, the table-talk manner and the chatter limit, peril silence, Undo limited to a player's own latest decision | Delegated to the agent (`AGENT-5`) |

Because Codex cannot be exercised now, the materially different second use
proposed for the spec is two separate Claude partner sessions, one per
investigator, with a partner as lead. Codex remains a disclosed, unverified path
until a session runs it.

## Open risks

- Agents talking to each other could cost tokens and drown the human's table.
  The chatter limit is untested.
- A shared decision's readiness loop could fail to settle between two agents.
  The limit on reopening is untested.
- Partner competence is the model's own. The protocol guarantees only that
  choices are legal and current.
- Long sessions depend on a partner's view staying small. Card text on first
  sight, and only changes after that, are design intents, not measurements.
