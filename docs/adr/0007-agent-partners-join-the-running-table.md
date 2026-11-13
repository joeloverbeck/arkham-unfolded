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
pointer, camera, menus or keyboard focus. The [agent seat](../../README.md#agent-seat)
remains a separate windowless process for testing the table.

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

## Considered options

- **Not selected: driving the human's table through the seat's pointer
  protocol.** It would put two pointers and one camera in conflict.
- **Not selected for the first increment: an MCP server, or a wrapper process
  driving Codex's app server.** Either would add a second interface to maintain.
  One shell command lets Claude Code and Codex share the same path. Claude Code
  can wait for it in the background without spending tokens; Codex re-checks it
  periodically.
