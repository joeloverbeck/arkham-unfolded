---
status: accepted
---

# Retire replay and start a new save format

Accepted on 2026-11-04: remove playback, its native and driver interfaces,
standalone recording APIs and accumulated observations retained for playback.
Replay was intended for narrated screen recordings, but Joel will not use it.
Keeping the historical snapshot machinery behind hidden controls would preserve
its unwanted maintenance burden. External screen recording remains independent;
no replacement video feature is selected.

All pre-removal saves and standalone recordings are deliberately unsupported,
including Gathering records. No migration, compatibility reader, maintained
historical build or recovery tutorial is required. Personal files remain
untouched and unsupported inputs are refused without silent overwrite. This
accepts an early compatibility break, not a policy for future save formats.

Persistence may be redesigned rather than retaining old record shapes, decision
journals or reconstructive algorithms for replay's sake. Live continuity must
preserve exact pending choices and randomness, safe Undo and hidden-information
barriers, action history, result explanations, story reading, animations and
restart archives. Save-before-install, failed-write recovery and stale/divergent
write refusal remain required.

This supersedes replay and pre-removal compatibility obligations in the earlier
foundations and tabletop decision. Source/deck identities, asset custody and
dated evidence retain their independent purposes. The [play contract](../foundation/play-contract.md#saveload-and-offline-use)
owns live continuity. The [investigation and implementation evidence](../research/replay-removal.md)
and [issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) retain
execution and acceptance records; [ADR 0005](0005-retire-experiments-and-select-scenarios.md)
later retires the standalone experiment paths.
