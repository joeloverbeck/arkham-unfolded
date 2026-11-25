---
status: accepted
---

# Seat ownership and waiting move to the shared library

Decided by Joel on 2026-11-25, in
[Cuba Libre #197](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197)
for both games. Who holds each investigator, a session's identity, its wait and its
choice that waits for the table move to `unfolded-agent-players`' `seats`, as
[ADR 0008](0008-partner-transport-moves-to-the-shared-library.md) moved the
transport and the client. Cuba Libre's consumer is ticket
[#203](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/203), under the same
boundary: if #197 ends without it, the mechanism moves back to Arkham.

What changes here comes with it:

- **A wait has no period unless its session asks for one.** The default and the longest
  period are gone. They cost a Codex session a model call for each period in which
  nothing happened.
- **A choice is answered at once,** as applied (`chosen`), waiting for the table
  (`settling`) or refused, and one command may choose and then wait. `choose` used to
  wait until the table settled, which a tool's time limit could cut off. `held` keeps its
  meaning: a choice that waits for other players' passes.
- **Each decision is told once,** by the answer to a choice or by a wait. A decision told
  only to a wait is told again to the session's next one, since an interrupted tool can
  leave a wait running that nobody reads. A plain wait for a decision already shown is
  refused.
- **A session has one wait,** which a newer one replaces and a dead client ends.
- **A join may carry its tool's identifiers of the session,** and one that already plays
  an investigator is refused. It is recognised by nothing else afterwards: Claude Code's
  identifier changes when a session moves to the background. This narrows the limit ADR
  0009 states; a tool that gives no identifier leaves it as it was.

Arkham keeps routing, hidden information, the view, shared decisions and readiness,
group choices, the hold for other players' passes, Undo, peril, table talk and its
limits, every word of the protocol, and the commit path.
