---
status: accepted
---

# One agent session plays one investigator

Decided by Joel on 2026-11-25, in
[Cuba Libre #197](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197)
for both games: an agent session plays exactly one investigator, which its join
names. The session ends when it leaves, when he takes that investigator back or turns
partners off, or when the table closes or the attempt restarts. The decision amends
[ADR 0007](0007-agent-partners-join-the-running-table.md), under which one partner
played any number of investigators and by default took every one open.
Separate sessions played better, and each session's context is then about its own
investigator. The human may still play several.

The host therefore holds one claim per session: a session's hand, view and talk are
one investigator's, and no answer needs a list. A join that names none or several is
refused rather than given one of those open: the table never chooses for a session.
The refusal names the investigators open, or says there is nothing to play when none
is. A session taken back is told so and ends, rather than staying at the table with no
investigator, where other players' talk would keep waking it.

The cost is one terminal session, with its own context and quota, for each
investigator handed over. The rule binds a session only by the identity its join
gives: a join that gives one already playing an investigator is refused first,
whatever it names, and any other join counts as a new session's.
