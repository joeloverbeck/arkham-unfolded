---
status: accepted
---

# One agent session plays one investigator

Decided by Joel on 2026-11-25, in
[Cuba Libre #197](https://github.com/joeloverbeck/cuba-libre-unfolded/issues/197)
for both games: an agent session plays exactly one investigator, which its join
names, and its session ends when it leaves or he takes that investigator back. This
amends [ADR 0007](0007-agent-partners-join-the-running-table.md), under which one
partner played any number of investigators and by default took every one open.
Separate sessions played better, and each session's context is then about its own
investigator. The human may still play several.

The host therefore holds one claim per session: a session's hand, view and talk are
one investigator's, and no answer needs a list. A join that names none or several is
refused with the investigators open, rather than given one of them: the table never
chooses for a session. A session taken back is told so and ends, rather than staying at
the table with no investigator, where other players' talk would keep waking it.

The cost is one terminal session, with its own context and quota, for each
investigator handed over. The rule rests on identity: a join that gives an identity
already playing an investigator is refused, but the host does not yet recognise a
session that joins again without giving its own.
