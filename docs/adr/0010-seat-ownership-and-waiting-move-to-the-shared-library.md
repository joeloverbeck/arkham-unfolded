---
status: accepted
---

# Seat ownership and waiting move to the shared library

Joel accepted Cuba Libre's spec
[#197](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197) and
[activated](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197#issuecomment-7156330255)
it, with its proposals, on 2026-11-25. Of its shared mechanism, for both games, that
decides here:

- who holds each investigator, a session's identity, its wait and its choice that waits
  for the table move to `unfolded-agent-players`' `seats`, as
  [ADR 0008](0008-partner-transport-moves-to-the-shared-library.md) moved the transport
  and the client;
- one session holds at most one seat, and claims are atomic;
- a wait has no period unless its session asks for one;
- a choice is answered at once, as applied, waiting for the table or refused, or the
  session waits too; one that waited and is not applied is released with the reason;
- each event is told once: a session's own decision, its seat taken back, the end of the
  table; and the end of the match or the table costs at most one call.

The default and the longest period cost a Codex session a model call for each period in
which nothing happened. `choose` waited until the table settled, which a tool's time
limit could cut off.

The agents' corrections to #197, not Joel's decision, settle how:
[6, 11](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197#issuecomment-7156606446),
[17, 18](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197#issuecomment-7157594960),
[19](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197#issuecomment-7158811536),
[20 and 21](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197#issuecomment-7164578298)
and [23](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197#issuecomment-7166695950).

- Arkham's `choose` takes the two forms (6). `held` keeps its meaning, a choice that
  waits for other players' passes; one that waits for the table is `settling`.
- A session has one wait: its newer one, or the death of its client, ends it, and an
  accepted choice ends its listening wait (11, 17, 19, 20). An interrupted tool can
  leave a wait running that nobody reads, so a decision or a release told only to a wait
  is told again, with a complete view, until the session submits (17, 20).
- The answer to a submission, accepted or refused, and a wait tell a decision; a view in
  another answer only shows it. A wait for a decision already told is refused, since it
  would wait for its session's own move (21).
- A wait that cannot tell the decision the table waits for does not keep its session from
  being marked silent, and a listening wait tells a decision that became the session's
  after it began (23).
- A join may carry its tool's identifiers of the session, and one that already plays an
  investigator is refused (18). It is recognised by nothing else afterwards: Claude
  Code's identifier changes when a session moves to the background. This narrows the
  limit [ADR 0009](0009-one-agent-session-plays-one-investigator.md) states; a tool that
  gives no identifier leaves it as it was.

Cuba Libre's consumer is ticket
[#203](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/203), under the
boundary ADR 0008 states. If #197 ends without it, what moved comes back to Arkham,
with what was written in the shared library for it that Arkham uses.

Arkham keeps routing, hidden information, the view, shared decisions and readiness,
group choices, the hold for other players' passes, Undo, peril, table talk and its
limits, every word of the protocol, and the commit path. The
[partner instructions](../agents/partner-instructions.md) state what a partner is
answered.
