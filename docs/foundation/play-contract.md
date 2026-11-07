# Playing, preparation and continuity

**Scenario entry, accepted 2026-11-07:** [ADR 0005](../adr/0005-retire-experiments-and-select-scenarios.md) establishes selection without a live game or artwork dependency, with only The Gathering on Standard and the fixed three decks currently offered. New uses a distinct unused save; Continue remembers the last successfully admitted path; Open validates an explicitly selected file. Save and return preserves the pending decision before leaving, and failure keeps the game active. Artwork admission precedes new-attempt creation. Standalone experiment records remain untouched and unsupported; this decision does not expand Gathering's gameplay coverage or establish native acceptance.

This document defines the intended playing experience and continuity contract. Detailed implementation and source feasibility remain open; these requirements do not claim that a playable implementation already exists.

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

Card counters follow their rules-defined meaning: ammunition uses the ammo artwork, bounties use bounty artwork, and charges have their own distinct token. Do not infer a use type from a card's name or treat other types, such as supplies or offerings, as charges. One counter displays each total, including zero for ammo, charges and Bounty Contracts. A readable counter replaces only its redundant count caption; required artwork must load before 3D play. Card margins may carry multiple counter types in stable side assignments, with centered spacing that protects printed information. The [card-counter and shuffle conventions](../adr/0002-persistent-table-areas.md#gathering-card-counters-and-shuffling) preserve these selected presentation requirements separately from implementation and native acceptance.

Enemy cards that remain at locations use an [enemy sideboard](../../GLOSSARY.md), grouped by location with every card and its counters exposed. A matching marker above each location links it to its group without relying on color alone or occupying investigator standee slots. Ordinary engaged enemies remain in their investigator's threat area; location-resident Massive enemies belong on the sideboard when that content is supported. The marker counts only the sideboard group. Each enemy displays accrued damage over its effective maximum health, including zero. The [sideboard decision](../adr/0002-persistent-table-areas.md#enemy-sideboard) owns the arrangement and preserves normal zoom/Inspect for full card text.

Supplies counters use the shaped investigator-resource artwork, retain zero while their card remains in play, and remain explicitly named Supplies in inspection and contextual text. In-play assets with health and/or sanity show the corresponding Damage and/or Horror tokens above the card, displaying accrued harm over the effective maximum, including zero. Omit a harm type when the asset has no corresponding capacity. These conventions also govern future supported cards; they do not activate implementation of unselected content. The [2026-11-06 presentation decisions](../adr/0002-persistent-table-areas.md#supplies-asset-harm-player-windows-and-persistent-premonition) record the selected behavior.

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

On 2026-11-07, Joel accepted [mandatory complete artwork for 3D play](../adr/0004-require-artwork-for-3d-play.md). Every retained 3D mode must verify and successfully load its complete required collection, including images and symbol font, before play. Missing, unreadable, changed or undecodable required assets block play; there is no playable substitute mode. Startup refusal leaves existing saves unchanged and creates no new attempt. A later required-asset failure stops play while preserving the last committed save. Image and font bytes remain outside Git. This records the selected contract; its implementation and native verification remain separate.

The blocking startup window offers Choose asset folder, Retry and Quit, and repeats its diagnostic in the terminal. It distinguishes absent/wrong folder structure, invalid/incompatible manifests and incomplete or damaged collections. Show the resolved root, failure count and at most five representative expected file paths with an omitted count. The recovery UI itself must not depend on missing prepared assets. The ADR owns the recovery and configuration details.

Symbols retain stable semantic identities and readable English names for inspection, history and engineering interfaces. Effective text, ordinary captions, counts and meaningful selection cues remain part of the artwork presentation. The headless player driver and rules/continuity checks remain usable without artwork; this is not a second 3D playing mode. Missing required rules semantics also blocks honestly rather than being improvised.

The applicable constitutional anchors are `AUTH-3`, `PURP-2`, `EVID-3`–`EVID-4`, `EVID-8`, `REP-1`–`REP-2`, `BOUND-5`, `INT-1`, `OPS-1`, `OPS-6` and `OPS-7`; [the constitution](../principles/principles-for-repos.md) owns those rules.
