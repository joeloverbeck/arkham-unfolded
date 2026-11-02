# Playing, preparation and continuity

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

New campaigns use the latest taboo version available through the accepted update source. The chosen version is pinned for the entire campaign and its replay. Exact rules and taboo identities are visible during preparation. Updating available data does not silently change an active campaign.

## Decisions and tabletop interaction

The program administers setup, costs, timing, bookkeeping, forced effects, movement and scenario/campaign flow. Joel retains every genuine player decision, including optional effects and legal choices of order, targets or allocation. Automation must not silently discard an opportunity or choose strategy for him.

The primary interaction is point and click: choose an action and any legal targets, then the program moves and animates the components. Free dragging is deferred. Live play uses a manual camera by default, with optional automatic following. Investigator standees show their map positions. Decks, tokens, shuffling and draws must be visibly represented without making readability or control depend on manipulating virtual components by hand.

An on-demand action history explains costs, modifiers, timing, outcomes and the relevant rules evidence. Effective card inspection shows the full text in force and identifies changes. Original card art is preserved; printed level and adjusted experience cost are shown separately when they differ.

Undo is available only before a choice reveals new hidden information. A replay is not permission to rewrite the recorded match or conceal a different outcome.

## Recording and replay

The complete match record preserves decisions, random outcomes and game-state progression, bound to the inputs and rules that governed the original play. Replaying a match reproduces its game events and outcomes rather than applying current rules or drawing new randomness.

Replay progresses through meaningful presentation moments. “Next” advances to the next moment and pauses, allowing Joel to narrate a recording. The replay camera automatically pans and zooms to relevant events, with manual override and the ability to go back. It need not reproduce the original camera movements or the time spent thinking. Continuous playback is optional deferred work. The recording boundary is a replay presentation that Joel can screen-record and narrate; built-in video encoding, editing, upload and publication are outside the first slice.

## Save/load and offline use

Save/load is required. Gameplay, saving, loading and replay work offline after the necessary data has been imported. Internet access is used for ArkhamDB and updates. Network multiplayer is outside the first release.

Autosave each committed player-decision boundary and permit a manual save at a pending decision, including during a skill test. Loading restores the exact pending choice, state and randomness without rerolling or repeating committed effects. Presentation interrupted by saving must not alter rules state. Records identify the required game data and application/rules version. Updates must preserve original records and an explicit way to resume or replay them, through a compatible version or a verified migration. The compatibility approach needs assessment before implementation; this does not promise that every future build can directly open every historical format.

## Presentation assets and missing artwork

The application reads presentation assets from a separately configured [prepared asset collection](../../GLOSSARY.md), outside the repository. The default uses the user's application-data directory. An in-app folder selection is checked, saved in per-user configuration and applied on restart; canceling or a failed configuration write preserves the previous setting. Temporary command-line overrides do not replace the saved preference. Personal saves, replays and settings remain separate from the portable collection.

The source archive is an unchanged input to [asset preparation](sources-and-verification.md#presentation-asset-preparation). Ordinary play reads only the prepared collection and does not fall back to the archive. The collection must remain usable after relocation without the source archive. Moving or replacing presentation assets must not change rules behavior, recorded decisions or save/replay compatibility. Replay uses the selected presentation assets; exact historical artwork appearance is not a replay guarantee.

Missing, unreadable or changed individual assets, or an absent collection, must leave the game playable using readable English substitutes for cards, tokens, symbols and other necessary components. The application reports whether the expected assets are complete or substitutes are in use. A playable scene with substitutes does not establish a complete asset collection. Image and font bytes remain outside Git.

Symbols have stable semantic identities and readable names; artwork is a presentation choice. Encounter-set symbols, for example, must remain understandable without their glyphs. Missing required rules semantics is different from missing art: preparation reports the unsupported behavior and blocks honestly rather than improvising it.

The applicable constitutional anchors are `AUTH-3`, `PURP-2`, `EVID-3`–`EVID-4`, `EVID-8`, `REP-1`–`REP-2`, `BOUND-5`, `INT-1`, `OPS-1`, `OPS-6` and `OPS-7`; [the constitution](../principles/principles-for-repos.md) owns those rules.
