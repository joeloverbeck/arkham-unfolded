---
status: accepted
---

# Persistent table areas with focused camera views

Accepted on 2026-10-29: use one persistent 3D scene with stable shared and
investigator areas. Hands and in-play components keep their physical homes as
the camera moves; focusing or inspecting another investigator changes neither
the turn nor the actor answering a pending decision. Focused views provide
readable play and Overview provides orientation. Simultaneous readability of
every card is not the selected goal.

Stable homes preserve physical relationships that a changing selected-investigator
dock would obscure. The accepted cost is camera travel, especially during
cross-investigator decisions; the current actor, pending decision and test
context remain available during ordinary navigation. A permanent split view
would consume space needed by the focused area. Exact geometry, framing and
transition timing remain adjustable presentation choices.

The shared Test area belongs exclusively to skill tests. Non-test effects use
investigator-associated resolution trays, as refined on 2026-11-05. Trays retain
the originating-investigator association through nested effects; a skill test
uses Test without duplicating its physical source. Trays clear unused space and
were selected over permanent shared staging and interactive modals. These are
presentation responsibilities, not additional rules-defined game zones.

The [tabletop presentation contract](../tabletop-presentation.md) owns current
interaction and display requirements. [ADR 0003](0003-retire-replay.md) retires
replay, [ADR 0004](0004-require-artwork-for-3d-play.md) requires complete artwork,
and [ADR 0005](0005-retire-experiments-and-select-scenarios.md) retires standalone
experiments while preserving Gathering's applicable contracts. The
[original decisions and dated evidence](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md)
retain superseded arrangements and implementation history; [issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89)
owns Gathering's work and acceptance.
