---
status: accepted
---

# The partners' transport and client move to the shared library

Accepted on 2026-11-25, when Joel activated Cuba Libre's spec
[#197](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197). It lets
Cuba Libre Unfolded play against external agents through the mechanism Arkham's
partners already use. This supersedes the sentence of
[ADR 0007](0007-agent-partners-join-the-running-table.md) that moved the
partners' game-neutral half to `unfolded-tabletop` only when mudsong really uses
partners. The consumer is Cuba Libre, under the
[Cuba Libre extension](../foundation/mission.md#code-shared-with-the-sibling-games)
and Joel's reading of 2026-11-22, which covers a consumer already ticketed in an
active spec he approved: the faction seat over the socket, ticket
[#203](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/203). If #197
ends without #203 using them, the mechanism moves back to Arkham. The rest of
ADR 0007 stands.

The first move, [Cuba Libre #199](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/199),
takes the socket's serving and the client's exchange, and changes nothing a
partner can send, read or exit with.

| Shared | Arkham's |
| --- | --- |
| `unfolded_tabletop::agent_transport`: the claimed socket, one thread for each connection, requests taken by a frame and answered on any later one, the wake of the frame loop, the close | Where the socket is, when it opens and closes, what a refusal to open it says, a wait's period and the answers the transport gives in the partner protocol's words (`src/gathering_tabletop/partners.rs`) |
| `unfolded_agent_players::client`: one request, one answer, an interrupt or deadline checked while it waits | The commands, the requests they spell, the wording of failures and the exit codes (`tools/src/partner.rs`) |

Decision ownership, routing, hidden information, shared decisions, talk and the
partner's view stay in Arkham's Bevy-free library.

A partner's client that dies while its request is held is now noticed by the
transport, which ends that connection and can say so. Arkham's host does not ask
it: a wait still ends at its period, as before. Ending such a wait at once is
for the waits without a period of
[Cuba Libre #201](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/201).

**Rollback.** Return the pin and these adapters together to the revision before
this move, which holds the original code. No saved data is involved.
