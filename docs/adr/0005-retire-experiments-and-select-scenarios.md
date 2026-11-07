---
status: accepted
---

# Retire the experiments and select actual scenarios

During the 2026-11-07 `grill-with-docs` interview at
`9847d72791ce934f5438603eb2b38a2771674dba`, Joel selected retirement of the
standalone Jacqueline, Tony and Party experiments. They served their stepping
stone purpose, and maintaining them now adds work without an intended playing
use. Retire their executable paths and exclusively owned implementation while
preserving the investigator rules, presentation support and persistence behavior
that The Gathering actually consumes. Hiding the old launch choices alone would
retain the unwanted maintenance burden.

Joel also selected an in-app scenario-selection screen now, rather than deferring
it until another scenario exists. Initially show only The Gathering, visibly in
development, with the existing Standard difficulty and three fixed investigator
decks. Selecting it offers New attempt, Continue and Open saved attempt. New
creates a separate save; Continue reopens the most recently selected saved
attempt; Open selects an existing file. Unsupported files remain untouched and
do not block a separate New attempt. The in-game Menu can return to selection
after safely saving, preserving the pending decision and staying in play if
saving fails.

The [investigation](../research/experiment-retirement.md) maps the shared
dependencies, save compatibility risk, tooling, verification seams and
documentation changes. In particular, Gathering imports investigator identity
from Party and shared native support from Jacqueline. Save validation also
depends on reconstructed history text. Those dependencies must be separated
without changing currently loadable Gathering attempts; a green build alone
cannot establish preservation.

The decision retains the headless driver, complete-artwork admission/recovery,
existing Gathering collections/preferences and personal-file custody. It retires
old-mode save readers without migration or deletion of personal records. Dated
experiment evidence remains historical, while shared verification instructions
move out of the retired Jacqueline guide. The implementation spec
[#127](https://github.com/joeloverbeck/arkham-unfolded/issues/127) supersedes the
four-mode maintenance boundary of [#124](https://github.com/joeloverbeck/arkham-unfolded/issues/124)
and the experiment-specific reach of ADRs 0002–0004, preserving their independent
contracts. [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) retains
complete Gathering gameplay and owner acceptance.

Joel confirmed the complete contract in Q5 on 2026-11-07: “Yes, ready for to-spec
on that basis.” This accepts the selected behavior, the production verification
boundary and the prospective engineering/owner-review limits recorded in the
investigation. The design interview is complete; no further scope or test-seam
confirmation is pending. Joel subsequently invoked `to-spec`, publishing #127
with `ready-for-agent`. Its [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/127#issuecomment-6709073042)
owns implementation and acceptance status; publication does not activate
implementation. GitHub Issues and pull requests remain the active work-control
plane.

Consulted constitution revision 2026-09-30.1: `AUTH-1`–`AUTH-3`,
`PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`,
`SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`,
`INT-1`–`INT-4`, `AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7`, and
`GOV-1`–`GOV-3`. No waiver is selected. This is a bounded maintenance and startup
experience decision supporting the existing scenario, not a campaign expansion.
