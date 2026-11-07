---
status: accepted
---

# Require complete artwork for 3D play

Accepted on 2026-11-07: complete prepared artwork, successful image/font loading
and renderer readiness are prerequisites for 3D play. Completeness covers
supported reachable presentation, not only the opening scene. The original
substitutes addressed anticipated difficulty acquiring artwork; Joel has the
artwork and will not use them. Maintaining alternate captions, positions and
components would retain a second presentation contract without a selected benefit.
This supersedes the missing-art playability requirements in ADRs 0001 and 0002.

Startup refusal creates no attempt and leaves existing saves unchanged. A later
required-asset loading failure stops play and preserves the last committed save.
Recovery must work without prepared artwork and offer an in-window path to
select a valid collection and retry. A terminal-only failure would be simpler,
but would leave recovery dependent on launch configuration when in-game settings
are unreachable.

The cost is losing artwork-free 3D play from a clean clone. The headless driver,
rules/continuity checks and CI remain independent of private artwork. Ordinary
text, semantic English symbol names, counts, selection cues and legitimate empty
components remain part of presentation. Preparation, portability, offline play,
source custody and external image/font storage continue under ADR 0001; no
automatic acquisition, save migration or artwork redistribution is selected.

The [play contract](../foundation/play-contract.md#presentation-assets-and-missing-artwork)
owns admission and recovery details. [ADR 0005](0005-retire-experiments-and-select-scenarios.md)
retires the standalone experiments while preserving Gathering's artwork boundary.
The [investigation](../research/required-artwork-for-3d-play.md) and
[issue #124](https://github.com/joeloverbeck/arkham-unfolded/issues/124) retain
implementation and verification evidence.
