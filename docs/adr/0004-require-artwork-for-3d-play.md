---

**Current scope, 2026-11-07:** [ADR 0005](0005-retire-experiments-and-select-scenarios.md) retires the standalone Jacqueline, Tony and Party experiments. Gathering retains the applicable asset, presentation and admission contracts. References below to earlier modes, commands and observations retain their historical meaning; they do not require maintaining those modes or establish current native acceptance.
status: accepted
---

# Require complete artwork for 3D play

On 2026-11-07, Joel selected removal of all support for playing the 3D application
without its complete prepared artwork. The original substitute presentation
addressed an anticipated difficulty acquiring artwork; Joel now has the artwork,
will not use substitutes, and does not want them offered to other players.
Maintaining alternate captions, positions and components therefore has no selected
player benefit. This supersedes the missing-art playability requirements in the
foundations and ADRs 0001 and 0002, while retaining their other decisions.

During the `grill-with-docs` interview, Joel explicitly retained the headless
player driver for engineering checks. He accepted the recommended blocking
startup error window with **Choose asset folder**, **Retry**, and **Quit**, plus
the same diagnostic in the terminal. This is an accepted design at source baseline
`0a940030ba2c59d9b4f67347b1cac8bbd3d2c33a`; implementation, native verification
and application-code publication have not occurred in this documentation change. The
[investigation](../research/required-artwork-for-3d-play.md) supports the bounded
implementation spec [#124](https://github.com/joeloverbeck/arkham-unfolded/issues/124),
published through Joel's subsequent `to-spec` invocation and related to
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). Its
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/124#issuecomment-6706875824)
owns execution and acceptance status. The issue is ready for the specified work;
neither its publication nor this record activates implementation.

The subsequent local #124 implementation uses the shared collection inspector,
blocking native recovery and render-health gate in all four modes. It retains
every required asset through runtime loading and postpones Gathering's initial
save until admission. Substitute-only presentation and launch switches are
removed. The issue handoff records the exact candidate, completed checks and
remaining native/owner verification; this implementation note grants no publication
or gameplay acceptance.

## Decision

- Require the complete prepared asset selection for the launched 3D mode,
  including its images and symbol font. Cover Gathering, Party, Tony and
  Jacqueline. Completeness covers supported reachable presentation, not only
  components visible in the opening scene, and does not require the whole source
  archive or unsupported game content.
- Verify the collection and require successful loading before admitting play.
  A manifest/file identity check alone does not establish image/font decoding
  or renderer readiness. Startup refusal must neither create a new attempt nor
  modify an existing save. Required assets must remain available to the running
  renderer; a later loading failure blocks further play and preserves the last
  committed save instead of switching to substitutes. This does not require
  continuous disk scans while successfully loaded assets remain in memory.
- Remove explicit no-art launch switches and the production presentation branches
  maintained solely for absent artwork, including their alternate captions and
  positioning. Keep ordinary titles, status, counts, effective rules text,
  accessible English names, selection feedback and legitimate empty components.
  A pile without a card is different from a card whose image failed to load.
- Keep the headless player driver, rules/continuity checks and CI independent of
  private artwork. Synthetic rendering fixtures may exercise loading and failure
  mechanics without becoming an alternate playable 3D mode or evidence that the
  real collection renders correctly.

## Failure and recovery

The blocking startup window must work without the prepared collection or symbol
font. No playable table or game actions are available behind it. Show the resolved
collection folder, the reason play cannot start, and a useful recovery instruction
both in the window and in the terminal.

Distinguish an absent collection or wrong folder structure from a recognizable
but incomplete collection. Also identify invalid/incompatible manifests and
changed, unreadable or undecodable required files honestly. For file failures,
show the total affected count and at most five representative expected paths,
followed by the number omitted. Do not dump the complete inventory. For an absent
structure, explain which prepared collection/folder to select rather than listing
every missing file. A collection for another mode is not a valid replacement.

**Choose asset folder** selects an already prepared collection and validates it;
it does not prepare, download or repair artwork. A successful selection is usable
by **Retry** during this blocked startup. **Retry** rechecks the selected folder,
allowing the user to repair it externally. Invalid selections, cancellation and
failed settings writes preserve the previous saved preference and report the
outcome. Keep command-line overrides temporary and disclose the effective folder;
an explicit recovery selection can replace the failing folder for this startup,
while a later invocation's explicit command-line path still takes precedence.
**Quit**, or closing the failure window, exits unsuccessfully without save changes.
If the graphics/window system itself cannot start, retain the terminal diagnostic
and unsuccessful exit rather than depending on an unavailable error window.

## Consequences

Requiring artwork sacrifices the ability to try the 3D application from a clean
clone without preparing its assets. Headless engineering checks remain available
from that clone. The selected recovery window adds a small startup interaction;
a terminal-only exit would be simpler but would make newcomers depend on launch
configuration to recover, particularly when the in-game selector is unreachable.

Previously tolerated partial and older collections must no longer admit play.
Some older experiments currently default to older selections; implementation must
provide truthful setup/recovery instructions without silently changing saved
preferences or deleting collections. Asset preparation, relocatability, offline
play, source-archive custody and keeping image/font payloads outside Git remain
required. No new game rules, save migration, automatic asset acquisition, public
asset hosting or broader audience-support commitment follows.

Reintroducing substitutes would require restoring and maintaining a second
presentation contract, so this removal is a deliberate product boundary rather
than a temporary missing-feature workaround. Dated substitute captures and test
reports retain their historical meaning; they are not requirements to maintain
the removed mode.

Consulted constitution revision 2026-09-30.1 in full for this foundation amendment:
`AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`,
`SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`,
`AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7`, `PUB-1`, `GOV-1`–`GOV-3` and
`ADOPT-1`–`ADOPT-4`. This is a product-scope decision, not a constitutional waiver.
