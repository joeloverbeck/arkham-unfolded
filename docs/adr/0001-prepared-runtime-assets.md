---

**Current scope, 2026-11-07:** [ADR 0005](0005-retire-experiments-and-select-scenarios.md) retires the standalone Jacqueline, Tony and Party experiments. Gathering retains the applicable asset, presentation and admission contracts. References below to earlier modes, commands and observations retain their historical meaning; they do not require maintaining those modes or establish current native acceptance.
status: accepted
---

# Prepare a self-contained asset collection for the application

On 2026-11-07, [ADR 0004](0004-require-artwork-for-3d-play.md) superseded this
decision's readable-substitute and partial-collection playability commitments.
Complete verified and successfully loaded artwork is required for 3D play;
headless engineering checks remain independent. The prepared-collection,
portability and source-custody decisions remain in force. Startup recovery may
retry a newly selected folder before play. This amendment records design, not
implementation of the removal; the original rationale below remains historical.

Accepted by Joel on 2026-10-27 during the `grill-with-docs` interview; [GitHub #4](https://github.com/joeloverbeck/arkham-unfolded/issues/4) owns the bounded retrofit, verification and implementation acceptance. The decision replaces direct runtime dependence on the externally owned Tabletop Simulator source archive with a prepared asset collection whose layout belongs to Arkham Unfolded. Preparation is an explicit development step; players select an already prepared folder.

The accepted experiment at `db72a1bfcb6600ab6afe28fbb67f576a43629bea` needs only 12 PNGs and one font from the approximately 100-GB archive. Its existing loader reads archive-relative image paths and extracts the font from a Unity bundle on each launch. Copying that subset while retaining the source paths would reduce the required payload, but preserve the archive naming and Unity coupling. We instead prepare ordinary files with application-relative paths, extract the exact verified font once, and retain source identities and conversion evidence. The expected image/font payload is 12,135,789 bytes; preparation and consumer checks must establish the actual result.

The [source contract](../foundation/sources-and-verification.md#presentation-asset-preparation) governs selection, copying, conversion, provenance and safe replacement. The [play contract](../foundation/play-contract.md#presentation-assets-and-missing-artwork) governs folder selection, portability, missing assets and continuity. The source archive remains unchanged, and ordinary play cannot depend on it. Shared cards need one selected presentation asset rather than a copy per campaign. Personal saves, replays, settings and research custody stay outside the portable collection.

The default collection lives under the XDG user-data directory, normally `~/.local/share/arkham-unfolded/assets`; the chosen root is saved under the XDG user-configuration directory. A small in-app selector checks the folder and applies a saved choice on restart. Command-line overrides remain temporary. This provides a discoverable handoff without making asset preparation part of play.

The cost is maintaining a finite selection and preparation procedure, and retaining the current collection while preparing an update. The benefit sought is the same supported presentation from a small relocatable folder, with truthful completeness reporting and unchanged readable substitutes. The scope is this application's actual selected content; no general asset platform, automatic archive discovery, alternate-art system, public asset hosting or broader game support follows. Accepting this architecture does not establish that the retrofit has passed its checks or received final owner acceptance.
