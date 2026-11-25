---
status: accepted
---

# The partners' transport and client move to the shared library

Joel accepted and activated Cuba Libre's spec
[#197](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197) on
2026-11-25. It has Cuba Libre Unfolded play against external agents through the
mechanism Arkham's agent partners use, extracted instead of copied. The
game-neutral half of agent partners therefore moves to `unfolded-tabletop` for
Cuba Libre. This supersedes the sentence of
[ADR 0007](0007-agent-partners-join-the-running-table.md) that moved it only when
mudsong really uses partners. The rest of ADR 0007 stands.

The boundary is the
[Cuba Libre extension](../foundation/mission.md#code-shared-with-the-sibling-games)
and Joel's reading of 2026-11-22, which covers a consumer already ticketed in an
active spec he approved. The transport and the client are for the faction seat
over the socket, ticket
[#203](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/203), which the
agents' [corrections](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197#issuecomment-7156606446)
to #197 name (item 8). If #197 ends without #203 using them, they move back to
Arkham. The mission states the boundary of what those corrections added.

Arkham keeps decision ownership, routing, hidden information, shared decisions,
talk and the partner's view in its Bevy-free library, and the requests, answers,
wording and exit codes of its commands.
