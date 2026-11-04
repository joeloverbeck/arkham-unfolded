---
status: accepted
---

# Retire replay and start a new save format

Joel accepted complete replay retirement on 2026-11-04 during the
`grill-with-docs` investigation at `c946d63988e3d246cb3e841dec7ff66a24d1d461`.
Replay was originally intended for narrated screen recordings; Joel reports
that he will not use it. Remove playback, its native and driver interfaces,
standalone recording APIs, and accumulated observations retained for playback
from Jacqueline, Tony, Party and Gathering. Removing the controls while keeping
the historical snapshot machinery is not the selected endpoint.

All saves and standalone recordings produced by pre-removal builds are
unsupported by the replacement, including the currently supported Gathering
records. No migration, compatibility reader, maintained historical build, or
recovery tutorial is required for those records. Existing personal files remain
untouched; unsupported inputs must be refused without silent overwrite.
This accepts a deliberate loss of old-record support to reduce maintenance
while development is early. It does not select the compatibility policy for
future formats.

Persistence may be reworked as necessary for a clean live-play implementation.
Retain exact save/resume, current randomness, safe Undo where available, action
history, result explanations, story reading, live animations and restart
archives for new attempts. Existing record shapes, decision journals and
reconstructive algorithms are replaceable; none must survive merely to load
old saves or reuse replay infrastructure. The chosen implementation must still
preserve pending choices, hidden-information barriers, save-before-install,
failed-write recovery and stale/divergent-write refusal.

This supersedes the replay and pre-removal-record compatibility obligations in
the foundations and [ADR 0002](0002-persistent-table-areas.md). Source/deck
identities, asset custody and dated verification evidence retain their independent
purposes. External screen recording remains outside the application; no
replacement video feature is selected. The [investigation](../research/replay-removal.md)
records the affected consumers and proposed proof boundary. [Issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89)
retains its separate complete-Gathering acceptance scope. The authorized local
implementation and its checks are recorded in the investigation's
[implementation result](../research/replay-removal.md#implementation-result).
Publication and complete-Gathering owner acceptance remain separate.

## Implementation activation

Joel instructed “Proceed with the implementation” on 2026-11-04. The local
implementation starts at `c946d63988e3d246cb3e841dec7ff66a24d1d461`, retaining the
interview's eight documentation changes. Its outcome is replay retirement in
all four modes with clean current-state persistence and the live behavior above.
The first proof uses the production Session/continuity interfaces for Gathering
setup, interrupted play, Undo and stale writes; driver and tabletop tests cover
the other consumers. Existing coverage is the characterization baseline, with
new checks only for changed behavior and uncovered preservation obligations.
This selects the test seams in the investigation; internal design remains
delegated. The work temporarily displaces further Gathering play refinements.
The ordinary two-active-hour checkpoint and non-convergence limits apply.
Headless verification and a reviewable local patch are authorized. Personal
files, native desktop interaction and publication remain outside this execution.
Implementation status and evidence are retained in the linked investigation;
this activation does not close #89 or establish full-scenario owner acceptance.

Consulted: `AUTH-1`–`AUTH-3`, `PURP-2`–`PURP-6`, `WORK-1`–`WORK-6`,
`EVID-1`–`EVID-4`, `EVID-8`, `REP-1`–`REP-3`, `BOUND-4`,
`AGENT-3`–`AGENT-5`, `OPS-1`–`OPS-7`, `GOV-1`–`GOV-3` and `ADOPT-3`.
