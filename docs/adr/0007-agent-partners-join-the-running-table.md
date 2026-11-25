---
status: accepted
---

# Agent partners join the running table and choose at the decision level

Selected on 2026-11-13 through a design interview: an agent partner plays its
investigators inside the human player's running windowed game. While partners
are enabled, the window opens a private local socket. A partner reads a view
scoped to its own investigators and submits choices by identity at the current
revision, through the same save-then-present path as the human's clicks. A
choice is applied only once the table has settled. Partners never drive the
pointer, camera, menus or keyboard focus. The [agent seat](../agents/agent-seat.md)
remains a separate windowless process for testing the table.
[ADR 0009](0009-one-agent-session-plays-one-investigator.md) amends how many
investigators a partner plays: one.

The seat could not be reused for this. It owns its process, camera and pointer
and acts by clicking. In the human's window, that would fight their pointer and
camera and open menus on their screen. A second process sharing the save fails
too: the save refuses a writer whose baseline has changed, and the window
re-reads the save only on Reopen. Choosing at the decision level also removes
the pixel indirection between an agent and the rules.

The cost is that partner play exercises the rules and the decision model, not
table presentation. Evidence about the table still comes from the seat and from
the human's own play.

Decision ownership and the partner's view belong to the Bevy-free library,
under [ADR 0006](0006-keep-bevy-and-keep-its-slice-narrow.md)'s containment. The window only hosts the transport and the
presentation. From `unfolded-tabletop`, the partner reuses the code that claims
the socket path. It cannot reuse the seat's serving loop, which assumes the seat
owns the whole application. Partners are built in this repository first. Their
game-neutral half moves to the shared library only when mudsong really uses
partners, under the [third-consumer waiver](../foundation/mission.md#code-shared-with-the-sibling-games).
[ADR 0008](0008-partner-transport-moves-to-the-shared-library.md) supersedes that
sentence: the half moves for Cuba Libre, as
[ADR 0010](0010-seat-ownership-and-waiting-move-to-the-shared-library.md) continues.

## Considered options

- **Not selected: driving the human's table through the seat's pointer
  protocol.** It would put two pointers and one camera in conflict.
- **Not selected for the first increment: an MCP server, or a wrapper process
  driving Codex's app server.** Either would add a second interface to maintain.
  One command lets Claude Code and Codex share the same path.

## Waiting

A partner's session is woken by its waiting command ending, and does not poll. Until
2026-11-25 only Claude Code waited so; Codex re-checked at intervals, each a model call.
Joel accepted that sessions are woken and do not poll, with a folder outside the checkout
and a permission for one client program, with Cuba Libre's spec
[#197](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197). How each tool
waits, why, and what was rejected is the shared library's decision record,
[agent sessions are woken](https://github.com/joeloverbeck/unfolded-tabletop/blob/main/docs/adr/0001-agent-sessions-are-woken.md).
A session runs in its tool's partner folder with the permission for `arkham-partner`,
which lies outside both folders. What keeps a session from writing that program in each
tool, as far as it was measured, is the documentation of `unfolded_agent_players::folder`.
The [partner instructions](../agents/partner-instructions.md#how-your-tool-waits) say how
to wait in each tool. If #197 ends without Cuba Libre using the folders, they move here
from the shared library.
