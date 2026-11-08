# Playing, preparation and continuity

This document defines the intended playing experience and continuity contract. Detailed implementation and source feasibility remain open; these requirements do not claim that a playable implementation already exists.

## Scenario entry

[ADR 0005](../adr/0005-retire-experiments-and-select-scenarios.md) establishes selection without a live game or artwork dependency, with only original The Gathering on Standard and the fixed three decks currently offered, visibly in development. New uses a distinct unused save; Continue remembers the last successfully admitted path; Open validates an explicitly selected file. Unsupported files remain untouched and do not block a separate New attempt. Save and return preserves the pending decision before leaving, and failure keeps the game active. Artwork admission precedes new-attempt creation. Standalone experiment records remain untouched and unsupported; this decision does not expand Gathering's gameplay coverage or establish native acceptance.

## Campaign and deck preparation

The first campaign is original Night of the Zealot, Standard, with one human controlling all three investigators:

| Investigator | Starting ArkhamDB deck |
| --- | --- |
| Tony Morgan | `5392133` |
| Patrice Hathaway | `5391062` |
| Jacqueline Fine | `5389418` |

These are fresh campaign starts using the imported starting lists. Descriptions and trauma from the user's previous campaign are not imported as current campaign state. Legitimate setup effects of the selected cards, including Arcane Research where present, still apply. Deck revisions and resolved preparation choices must be bound to the campaign rather than silently following later ArkhamDB edits.

The supported content is the starting party and cards, everything reachable through the selected campaign's legal branches, and the upgrades and weaknesses actually selected. This does not authorize preemptive implementation of the full catalog. Preparation checks campaign, investigators, decks, selected weaknesses, reachable cards, rules version and taboo version. A metadata entry or available picture is not evidence of implemented behavior.

For these starting decks, the basic weaknesses are fixed: Tony has Internal Injury (`02038`), Patrice has The 13th Vision (`05041`), and Jacqueline has Chronophobia (`02039`). Joel supplied the latter two selections from his upgraded decks on 2026-11-02 under [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). They replace the unresolved placeholders in the retained starting-list snapshots; this does not import the upgraded lists, experience or previous campaign history. Bind these explicit selections alongside the original deck revisions and implement them before play starts. No random basic-weakness draw, all-official pool construction or implementation of unselected weaknesses is required for this start. If a later accepted campaign effect actually requires another weakness, resolve its eligibility, quantity, selection and support at that boundary without silently substituting a supported card.

ArkhamDB remains the place to edit and upgrade decks. The game tracks earned, spent and unspent experience, including applicable discounts and card-specific effects. Between scenarios, Joel imports a successor deck; the program checks the transition, legality, costs and implementation support before the next scenario can begin. An unsupported chosen upgrade becomes work required before that preparation succeeds. Importing an arbitrary legal deck is not a promise that it is immediately playable.

New campaigns use the latest taboo version available through the accepted update source. The chosen version is pinned for the entire campaign. Exact rules and taboo identities are visible during preparation. Updating available data does not silently change an active campaign.

## Decisions and tabletop interaction

The program administers setup, costs, timing, bookkeeping, forced effects, movement and scenario/campaign flow. Joel retains every genuine player decision, including optional effects and legal choices of order, targets or allocation. Automation must not silently discard an opportunity or choose strategy for him.

The primary interaction is point and click: choose an action and any legal targets, then the program moves and animates the components. Free dragging is deferred. Live play uses a manual camera by default, with optional automatic following. Investigator standees show their map positions. Decks, tokens, shuffling and draws must be visibly represented without making readability or control depend on manipulating virtual components by hand.

The [tabletop presentation contract](../tabletop-presentation.md) owns detailed layout, controls, resolution surfaces, animation and display conventions. In particular, [counters and markers](../tabletop-presentation.md#counters-and-allowance-markers) retain their rules-defined meanings, and the [enemy sideboard](../tabletop-presentation.md#scenario-locations-and-enemy-sideboard) exposes location-resident enemies without occupying investigator slots. These conventions apply to supported content; documenting them does not add unselected cards or establish native acceptance.

An on-demand action history explains costs, modifiers, timing, outcomes and the relevant rules evidence. Effective card inspection shows the full text in force and identifies changes. Original card art is preserved; printed level and adjusted experience cost are shown separately when they differ.

Undo is available only before a choice reveals new hidden information. The action history must remain clear about choices and their reversal.

## Recording and replay retirement

Joel may screen-record live play using external tools. Built-in video encoding, editing, upload and publication remain outside the product scope.

On 2026-11-04, Joel accepted [complete replay retirement](../adr/0003-retire-replay.md): remove playback, recording export/read APIs and historical observation storage maintained for replay across all four modes. Live play, action history and continuity retain their own requirements. Persistence may be reworked as necessary rather than constrained by old record structures. The implementation and verification evidence is retained in the [removal report](../research/replay-removal.md#implementation-result).

## Save/load and offline use

Save/load is required. Gameplay, saving and loading work offline after the necessary data has been imported. Internet access is used for ArkhamDB and updates. Network multiplayer is outside the first release.

Autosave each committed player-decision boundary and permit a manual save at a pending decision, including during a skill test. Loading restores the exact pending choice, state and randomness without rerolling or repeating committed effects. Presentation interrupted by saving must not alter rules state. Save records identify the required game data and application/rules version. Supported formats and source versions must be explicit; unsupported records are refused without changing their bytes.

The replay-retirement change deliberately makes every save and standalone recording produced by pre-removal builds unsupported across all experiments and Gathering. No migration or maintained historical reader is required. This accepted break concerns existing records; it does not decide the compatibility policy for future formats. Personal files are not deleted or overwritten merely because support ends.

## Presentation assets and missing artwork

The application reads presentation assets from a separately configured [prepared asset collection](../../GLOSSARY.md), outside the repository. The default uses the user's application-data directory. A valid in-app folder selection is checked and saved in per-user configuration; during play it applies on restart, while the blocking startup recovery window can use a newly selected folder on Retry. Invalid selections, cancellation and failed configuration writes preserve the previous setting. Temporary command-line overrides do not replace the saved preference. Personal saves and settings remain separate from the portable collection.

The source archive is an unchanged input to [asset preparation](sources-and-verification.md#presentation-asset-preparation). Ordinary play reads only the prepared collection and does not fall back to the archive. The collection must remain usable after relocation without the source archive. Moving or replacing presentation assets must not change rules behavior, game decisions or supported-save continuity.

On 2026-11-07, Joel accepted [mandatory complete artwork for 3D play](../adr/0004-require-artwork-for-3d-play.md). Every retained 3D mode must verify and successfully load its complete required collection, including images and symbol font, before play. Missing, unreadable, changed or undecodable required assets block play; there is no playable substitute mode. Startup refusal leaves existing saves unchanged and creates no new attempt. A later required-asset failure stops play while preserving the last committed save. Image, font and 3D model bytes remain outside Git. The chaos-pouch integration selected on 2026-11-08 extends the same admission requirement to the model, its embedded dependencies and material rendering; a failed model cannot fall back to the old placeholder. This records the selected contract; its implementation and native verification remain separate.

Completeness covers all supported reachable presentation, not just the opening scene. File/manifest identity checks alone do not establish image/font decoding or renderer readiness. Required assets stay available to the renderer; successfully loaded assets in memory do not require continuous disk scans. Partial, older or wrong-content collections cannot substitute for the selected complete collection. Synthetic rendering fixtures can check loading mechanics without proving the real collection renders correctly or becoming an alternate 3D playing mode. No-art launch switches and presentation branches maintained solely for missing artwork are retired; ordinary text and legitimately empty piles remain.

The blocking startup window offers **Import artwork**, **Choose asset folder**, **Retry** and **Quit**. It works without the prepared collection or symbol font, with no playable table or game actions behind it. Show the resolved collection folder, why play cannot start and a useful recovery instruction in both the window and terminal. Distinguish absent/wrong folder structure, invalid/incompatible manifests, and recognizable but incomplete collections. Identify changed, unreadable or undecodable files honestly. For file failures show the total count and at most five representative expected paths, with the omitted count. For an absent structure, explain how to import the required package or select a prepared folder instead of listing every missing file.

**Choose asset folder** validates an already prepared collection; it does not prepare, download or repair artwork. Successful selection is usable by **Retry** during this blocked startup. Retry rechecks the selected folder, including external repairs. Invalid selection, cancellation and failed settings writes preserve the saved preference and report the outcome. Keep command-line overrides temporary and disclose the effective folder: an explicit recovery selection can replace the failing folder for this startup, while a later invocation's explicit command-line path still takes precedence. Quit or closing the failure window exits unsuccessfully without changing saves. If the window/graphics system cannot start, retain the terminal diagnosis and unsuccessful exit.

On 2026-11-08, Joel selected **app-managed import of a runtime ZIP** as the normal artwork setup: obtain one complete package for the currently supported content, choose **Import artwork**, and let the application verify and install the prepared collection in its own persistent data area. Players should not have to extract folders, select an installation path, or remember a staging directory. Installed play remains offline and independent of the development sources. [README](../../README.md) describes the implemented import and recovery flow; [issue #140](https://github.com/joeloverbeck/arkham-unfolded/issues/140) owns implementation and acceptance evidence. The [dated investigation](../research/artwork-distribution-and-installation.md) retains the comparison and agreed acceptance journey. The existing admission, save-preservation and temporary engineering-override requirements continue to apply.

Imports occur outside active play. Keep the previous installation intact during verification and retain it until the replacement passes integrity checks and successfully loads through normal rendering admission. Failed or interrupted imports preserve the previous installation and personal saves; retaining an older collection does not promise that a newer executable can play with it. After successful replacement admission, remove the app-managed previous copy so installed revisions do not accumulate. User-selected download ZIPs and the separately selected Documents backup remain user-owned and are not automatically deleted. Installed artwork is persistent application data rather than disposable cache.

Symbols retain stable semantic identities and readable English names for inspection, history and engineering interfaces. Effective text, ordinary captions, counts and meaningful selection cues remain part of the artwork presentation. The headless player driver and rules/continuity checks remain usable without artwork; this is not a second 3D playing mode. Missing required rules semantics also blocks honestly rather than being improvised.

The applicable constitutional anchors are `AUTH-3`, `PURP-2`, `EVID-3`–`EVID-4`, `EVID-8`, `REP-1`–`REP-2`, `BOUND-5`, `INT-1`, `OPS-1`, `OPS-6` and `OPS-7`; [the constitution](../principles/principles-for-repos.md) owns those rules.
