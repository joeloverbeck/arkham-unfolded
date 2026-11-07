---
status: accepted
---

# Prepare a self-contained asset collection for the application

Accepted on 2026-10-27: ordinary play consumes a portable prepared asset
collection whose layout belongs to Arkham Unfolded, independently of the
externally owned source archive. Preparation is a development step; players
select an already prepared collection. The archive remains unchanged, and
personal saves, settings and research remain outside the collection.

Only a small selection from the much larger archive is needed for supported
play. Copying files while retaining archive-relative paths would preserve its
naming and Unity-bundle coupling. Instead, prepare ordinary files with
application-relative paths, extract the verified font once, and retain source
identities and conversion evidence. Shared cards use one selected asset rather
than a copy per campaign. The cost is maintaining the finite selection and
preparation procedure while keeping the active collection intact during updates.
This does not select a general asset platform or automatic archive acquisition.

The [source contract](../foundation/sources-and-verification.md#presentation-asset-preparation)
owns preparation and custody; the [play contract](../foundation/play-contract.md#presentation-assets-and-missing-artwork)
owns configuration, recovery and continuity. [ADR 0004](0004-require-artwork-for-3d-play.md)
replaces the original missing-art substitutes with mandatory artwork, and
[ADR 0005](0005-retire-experiments-and-select-scenarios.md) narrows maintained
play to Gathering. [Issue #4](https://github.com/joeloverbeck/arkham-unfolded/issues/4)
and the [original record](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0001-prepared-runtime-assets.md)
retain the retrofit evidence.
