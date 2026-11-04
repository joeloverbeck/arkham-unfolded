# Replay removal investigation

Investigated on 2026-11-04 at `c946d63988e3d246cb3e841dec7ff66a24d1d461`
(`Animate Gathering card draws`, PR #97). Joel accepted complete replay retirement
and a clean compatibility break during the `grill-with-docs` interview, recorded in
[ADR 0003](../adr/0003-retire-replay.md). He clarified that every old save may be
unsupported and that persistence may be reworked as necessary; compatibility must
not preserve a poor design. The investigation below records the pre-change
findings; [Implementation result](#implementation-result) records the authorized
local implementation and its verification.

[Issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) is the sole
open issue. Its [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
identifies this revision and leaves full native scenario play and owner acceptance
outstanding. The local removal does not change those acceptance statuses.

## Conclusion

Complete removal is feasible. The accepted boundary removes playback,
recording export/import APIs, and accumulated observations retained for historical
playback in all four modes: Jacqueline, Tony, Party, and Gathering. Live consumers
of those observations must first receive the smaller state they actually need.

Retain autosave/manual save, exact resume, current randomness, safe Undo, action
history, result explanations, story reading, live animations, and restart archives.
Existing persistence structures and algorithms are replaceable. Saves already
serialize current game state, which provides a useful starting point, but neither
that exact representation nor the decision-journal reconstruction is a constraint.
Choose the simplest clean design for the surviving live requirements.

This is a substantial persistence and presentation refactor. Deleting playback
controls and exports is the smaller part. The highest-risk work is replacing live
dependencies on historical observations and preserving continuity safeguards.
Static inspection establishes the dependencies, not the success, duration, or
performance benefit of a future patch.

## Three different removal scopes

| Scope | Result | Assessment |
| --- | --- | --- |
| Remove playback interfaces and exports | No native or driver replay; historical observations remain in saves behind private accessors | Cheaper, but retains repeated data and much of the coupling that prompted this investigation |
| Also retire historical observation tapes | Live play retains only independently needed state, history, Undo and current presentation information | Selected complete retirement of replay and its exclusive burden |
| Rework persistence and deterministic reconstruction as necessary | Replace historical-prefix checks, reconstructive validation or Undo when a cleaner live design warrants it | Permitted by the owner's clarification; no old-format compatibility constraint |

The first scope can be an internal implementation step toward the second. It
should not silently become the final result of a request for complete removal.

## Where the feature exists

The code contains four implementations rather than one central replay module.
Line references below refer to the investigated commit.

| Surface | Locations and removal consequence |
| --- | --- |
| Replay cursors | `src/continuity.rs:17`, `src/tony/continuity.rs:19`, `src/party/continuity.rs:21`, `src/gathering/continuity.rs:33`: remove cursor types and historical navigation |
| Native playback | `src/tabletop.rs`, `src/tony_tabletop.rs`, `src/party_tabletop.rs`, `src/gathering_tabletop.rs` and their controls/UI modules: remove replay state, menus, Previous/Next/Return-to-live, and replay-specific branches in inspection, selection freshness, camera, results and animation |
| Driver protocol | `src/bin/driver.rs:86` and `src/bin/driver/{tony,party,gathering}.rs`: remove `replay`, `next`, `back`, `live`; the latter three also expose `export` |
| Recording I/O | `src/continuity.rs:239`, `src/tony/continuity.rs:199`, `src/party/continuity.rs:208`, `src/gathering/continuity.rs:317`: retire standalone recording envelopes, readers/writers and replay-only validation |
| Rules-facing records | `Recording` and `Session::recording()` in each mode: replace live consumers before removing these APIs and their historical arrays |

Jacqueline, Tony and Party write a `.replay-<digest>.json` beside the session when
opening native replay, then read that same file back. Gathering's native replay
uses an in-memory recording; its driver exports standalone files. No ordinary
native/driver option for importing an arbitrary recording file was found;
`--load` loads a session. Recording readers also serve tests and documented
historical recovery, so they have consumers beyond the visible menu.

## The live dependencies that make deletion nontrivial

| Surviving behavior | Current historical dependency | Required replacement |
| --- | --- | --- |
| Gathering's revealed Act/Agenda faces | `src/gathering_tabletop/story.rs:113` scans previous decisions and intermediate moments; some agenda reverses are inferred from a doom reset | Explicit knowledge of publicly revealed faces and relevant transition facts, without exposing unseen reverses |
| Gathering story panel after reopening | `story.rs:65` uses the preceding observation to offer the latest revealed story again without repeating rules effects | Enough previous/current transition information to preserve that behavior |
| Gathering results after reopening | `src/gathering_tabletop/results.rs:16` consults earlier observations and presentation moments, including a frozen test before pending ST8 cleanup | The applicable frozen result and reading baseline |
| Gathering live consequence animation | `src/gathering.rs:531`, `src/gathering/moments.rs:44` project intermediate views from the previous observation and current presentation sequence | A bounded current-transition presentation batch and its base view |
| Tony result explanations | `src/tony_tabletop.rs:237` recovers resolved token information from earlier observations | Retained result/token facts sufficient for the live explanation and reopening |
| Party result visibility | `src/party_tabletop.rs:136` checks the preceding observation | Retained current result-transition information |
| Save overwrite protection | `src/continuity.rs:350`, `src/tony.rs:807`, `src/party.rs:1061`, `src/gathering.rs:822` use historical prefixes to reject stale/divergent writers | Equivalent continuation checks using the surviving authoritative save representation |
| Save validation | Validation shares recorded observations; Tony also checks harm progression at `src/tony/harm.rs:86` | Validation of the surviving state and transition invariants, without quietly deleting corruption/refusal guarantees |

These are proposed replacement responsibilities, not a selected generic event
framework or a request to the owner to design internal fields. The required
replacement is the responsibility, not a requirement to keep the present field
or reader shape.

Gathering's [Session](../../src/gathering.rs) separately holds current state,
randomness, decision audit, action history, full observations, presentation
observations, and Undo boundaries (`:251`). Loading deserializes the current
Session (`src/gathering/continuity.rs:309`). Validation reconstructs from seed and
decisions (`src/gathering.rs:835`); Undo reconstructs the permitted decision prefix
(`:627`). The decision audit and reconstruction currently implement live
responsibilities; their replacement must take over those responsibilities rather
than retaining the old mechanism by default.

Every saved observation repeats cumulative action history in all four modes.
Gathering additionally repeats completed test reports inside play observations.
Thus hiding the viewer leaves real storage, cloning and validation costs.
Gathering currently limits records to 8 MiB. Exact savings from pruning these
arrays require an implementation and measurement; none are claimed here.

Some similarly named mechanisms must remain: `draws::Playback` is live card
animation, and draw cues are already transient; restart archives are saved
attempts; `tools/src/profile.rs` records performance samples. Live camera
following also exists independently of replay. Removal needs a consumer audit,
not a blanket deletion of every occurrence of “replay” or “recording.”

## Existing records and compatibility

The normal local Gathering save was inspected only for envelope metadata and
counts. At inspection it was in mulligans at revision 5, with six decision
observations. Eight Gathering restart archives also exist. Earlier experiments
have active saves, archives and standalone replay files. This inventory does not
establish which attempts Joel values or cover every possible save location.

Joel settled the compatibility decision explicitly: **all pre-removal saves and
standalone recordings are unsupported by the replacement**, including current
Gathering records. The implementation need not preserve a decoder, migration,
compatibility shim, historical executable or supported recovery tutorial for any
of them. This supersedes the investigation's initial suggestion to maintain a
pinned older build for recovery.

The accepted trade-off is loss of supported continuation of old progress in
exchange for a smaller maintenance obligation. There is no need to investigate
which old attempt matters or ask again about migration. Existing files remain
untouched; ending support does not authorize deletion or silent overwrite.
Use an explicit new format/version and a fresh ordinary save destination when
necessary for a usable cutover. An explicitly supplied unsupported file should
receive a clear refusal and a way to start a new attempt at a different path.

The [existing historical-reader guide](../experiments/gathering-setup.md#record-compatibility)
describes the pre-removal implementation. It ceases to be supported guidance when
removal lands. Git history can retain its historical evidence without creating a
reader-maintenance obligation. The accepted reset does not select the
compatibility policy for future formats.

## Clean persistence direction

The recommended design saves the current rules position and random state,
chronological action history once, still-legal Undo state, and the facts needed
for live result/story presentation. It does not preserve every historical view.
Keep the representation local to these actual modes; no generic event framework,
database or migration platform is required by this change.

| Mode | Clean live save boundary |
| --- | --- |
| Jacqueline | Current fixture, draw cursor, bag/tokens, pending group, stage and history; no new RNG or Undo mechanism |
| Tony | Current rules state, RNG/deck/test state and existing private Undo snapshots; remove moments and legacy repairs |
| Party | Current rules state, RNG/decks/test/actor, hidden-information boundary and private Undo snapshots |
| Gathering | Current setup/runtime/RNG state, private reversible positions and explicit live presentation facts; retire the lifetime executable decision journal after its validation/Undo replacements pass |

Tony and Party already use private rules-state snapshots for Undo, clearing them
at hidden-information barriers. Jacqueline's scripted experiment has no Undo and
uses a fixture draw order/cursor rather than a random generator. Gathering alone
reconstructs its entire decision prefix for Undo and for validation. Prefer
reversible rules-position snapshots there as well, without nesting whole
Sessions or copying the complete action history into each snapshot. Preserve
multi-step Undo until the latest information barrier; this is a lifetime limit,
not a promise of constant-size storage or permission to reduce Undo to one step.

The existing locked file-replacement owner (`src/record_replacement.rs:46`)
already admits a write while holding the destination lock. A candidate can use
the exact state/identity last read or saved as its expected baseline. If the
disk already contains the candidate, a repeated save is a no-op; otherwise
require that it still matches the baseline before replacement. A first create
expects absence. This can preserve stale/divergent-write refusal without
historical observation prefixes or a new general concurrency system. Retain
save-before-install and update the expected baseline only after successful save.

Replacing Gathering's historical reconstruction is the principal unresolved
engineering risk. Its runtime currently lacks an independent current-state
validator: a save contains pending effect frames, tests, card/token zones,
randomness and stop conditions. The first implementation slice must establish
the scoped invariants needed for safe load and continuation. A checksum detects
byte corruption; it does not establish semantic state validity. Current-state
validation also does not prove that an entire historical choice sequence was
authentic or reachable. Retiring that historical proof is consistent with the
removed complete-match-record promise, but it must not be described as equivalent
evidence. Do not keep reconstructing the whole game solely to preserve an old
code shape; do not delete its safeguards before the live replacement is proved.

The current-state checks need source/mode/version identity, valid references and
indices, card/token ownership and inventory, consistency among stage/actor/tests/
pending effects, and separation of ordinary and privileged fixture states. Use
Gathering's existing hidden-information predicate (`src/gathering/runtime.rs:2038`)
to constrain Undo; a deck draw, search or room reveal can cross that boundary
without advancing the RNG. Completed-test reports and chronological history
need one owner rather than repeated copies in every Undo position.

## Contracts, documentation and test changes

Replay was explicitly selected for narration in the
[play contract](../foundation/play-contract.md#recording-and-replay-retirement).
The accepted design is now recorded in the three foundations, README's intended
delivery and ADR 0003. Current-build instructions in the README and four
experiment guides still describe the existing executable; update them alongside
implementation, including offline, compatibility, inspection and delivery text.
Keep source/rules identities and save continuity obligations that still serve
ordinary play. External screen recording can remain outside the application;
no replacement video-production capability is needed by this proposal.

[ADR 0002](../adr/0002-persistent-table-areas.md) contains replay obligations,
including the latest draw handoff. Its scope notice now links ADR 0003's
supersession while preserving the dated observations. ADR 0001's portable asset
boundary survives; excluding old personal recordings from prepared assets does
not promise playback support. The glossary's prepared-collection definition now
describes the surviving outputs. It still distinguishes a scenario attempt from
replay; no new domain term is needed.

Issue #89's body expressly includes replay and preservation of experimental
records. A future accepted brief/current handoff must explicitly supersede that
scope. Dated evidence and closed issues should retain what actually happened.
Replay removal would not establish full Gathering gameplay or owner acceptance.

Twenty-three integration-test files reference `Replay`, `save_recording`, or
`load_recording`; further tests inspect `recording()` and tabletop mode state.
Mixed tests must retain their live claims. In particular:

- `tests/gathering_record_capacity.rs` must continue checking save capacity and
  fresh loads through actual scenario progression after export checks disappear.
- Driver tests must retain real setup, round/mythos continuation, and refusal of
  privileged inputs.
- Restart and file-replacement tests must retain exact archives, recovery,
  stale/divergent-write refusal, locking, and failed-write behavior.
- Rule, token, result, discard and draw tests must continue observing the live
  consequences previously witnessed through replay snapshots.
- Replay-only and legacy-compatibility assertions/fixtures can retire. Recreate
  any independently needed live regression case in the new format; preserving
  old fixture bytes is not a new-format requirement. Legacy-file ignore rules
  should keep old private records out of Git.

## Proposed execution and proof boundary

The removal and compatibility policy are settled. Implementation can use one
coherent work item. Its first checkpoint should exercise Gathering's current-state
validation and the most coupled live presentation path: a meaningful pending
test/result or story transition, save, fresh-process resume, and the next legal
choice with unchanged randomness. Include a supported reversible choice and
Undo across save/reopen, refusal at a hidden-information barrier, and two writers
with different successors from the same saved baseline. This exposes the main
risks before broad deletion. The remaining modes need their own result/continuity
cases; success in Gathering alone does not establish them.

The setup part can use lead selection, a mulligan-card selection, save/reopen,
Undo of that selection, and mulligan confirmation through the real player seam.
Existing oracles include `tests/gathering_continuity.rs`,
`tests/gathering_undo.rs` and `tests/gathering_restart.rs` for exact continuation,
inventory substitution, competing writers, searches/reveals, busy saves and
damaged recovery. Historical reconstruction may serve temporarily as a test-only
comparison while its production replacement is developed; it is not itself the
independent rules oracle.

The final candidate must exercise all four modes for live choices, history and
results, save/reopen, applicable Undo barriers, restart/archive recovery,
failed/stale writes, and current presentation. Check that removed driver
commands fail cleanly and that ordinary operation produces no replay exports.
Use existing rule expectations and surviving live-path assertions as oracles.
Run the repository check gate for the final candidate, then inspect the affected
native menus, animations and interrupted presentation in a bounded human review.
Automated checks cannot establish appearance or control quality.

The interview produced documentation changes only: this report, ADR 0003 and
the related foundation/README/glossary/ADR scope updates. No application code,
save format, GitHub state or personal record was changed. No runtime test or
native launch was performed. The compatibility uncertainty is resolved; a
reliable implementation estimate still depends on the first current-state
validation and presentation change.
The repository's ordinary scope, non-convergence and two-active-hour checkpoint
limits apply to any later authorized implementation; this report grants no
implementation or publication allowance.

## Settled interview decisions

1. Complete replay retirement includes historical observation tapes and all
   native/driver/export surfaces across the four modes. The listed live
   capabilities remain required.
2. Every pre-removal save and recording may be unsupported. No migration or
   supported historical reader/build is required; files remain untouched.
3. Persistence can be reworked as necessary. Existing structures and algorithms
   are not constraints merely because they currently support saves or replay.

There is no remaining product or compatibility question from this interview.
Internal representation, validation strategy and module boundaries are delegated
engineering choices within the accepted live behavior. ADR 0003 records the
trade-off. A bounded implementation brief and its verification/acceptance surface
belong in the GitHub work-control plane when implementation is activated.

The complete constitution revision 2026-09-30.1 was read for the proposed
foundation change. Particularly applicable rules are `AUTH-1`–`AUTH-3`,
`PURP-2`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-4`, `EVID-8`,
`REP-1`–`REP-3`, `BOUND-4`, `AGENT-3`–`AGENT-5`, `OPS-1`–`OPS-7`,
`GOV-1`–`GOV-3` and `ADOPT-3`. This report supports the owner decision; GitHub
continues to own active work and acceptance.

## Implementation result

Joel authorized implementation on 2026-11-04. The local patch removes replay from
Jacqueline, Tony, Party and Gathering: no playback objects or controls, driver
navigation commands, standalone recording readers/writers, recording exports, or
persisted historical observation/presentation tapes remain. The driver refuses
retired commands without changing the live state. External video capture tooling
keeps its separate purpose.

`src/saved_state.rs` now owns the shared version-3 envelope and write-freshness
contract. It retains kind/source/application binding, checksum verification,
size limits, the existing destination lock, atomic replacement, `.previous`
recovery and restart archives. A Session remembers the exact successfully loaded
or saved bytes outside its serialized rules state. Different successors require
that baseline to still match disk; identical saves are idempotent. Restart checks
both the expected state and its baseline before retaining the old attempt and
installing the fresh one. All four continuity adapters preserve
clone/choose/save/install, so failed writes leave live rules and randomness intact.
Versions 1 and 2 and standalone recording kinds are refused before payload use.
No migration, historical reader or deletion of personal files is introduced.

The investigation preferred replacing Gathering's decision journal as well.
Implementation retained that compact journal for its independent live duties:
reconstruction validates the complete pending runtime and reconstructs safe Undo
positions. Unlike Tony and Party, Gathering has no independent current-state
validator covering its effect frames and nested tests. Replacing that safeguard
would introduce a second substantial rules-state implementation. Retaining it
preserves validation and multi-step Undo without retaining a single historical
view or supporting any old save. This is an intentional design choice within the
delegated implementation, not a compatibility concession. Gathering load/save and
Undo still incur reconstruction cost; this patch does not claim to remove it.
Tony and Party keep their existing private rules-position Undo snapshots;
Jacqueline keeps its scripted current state and has no new Undo mechanism.

Live presentation now has explicit inputs. Gathering stores revealed story faces,
newly revealed faces and the completed-test boundary needed on reopening; only
the current transition's animation steps exist in memory. A pending ST8 discard
order obtains its frozen result from the current cleanup frame. Tony stores the
resolved token needed by its current result explanation. Party stores whether
the latest choice settled a result. Result detail, public counters, commitment
projections and harm facts are required in the new format; obsolete projection
repair, legacy empty-commitment/result handling and missing-detail fallbacks are
removed. Historical replay fixtures are deleted; surviving live assertions build
fresh positions through legal choices.

Verification completed through `scripts/check`. The final receipt at
`/tmp/replay-removal-check3/run.json` records `finished`, exit 0: formatting,
strict Clippy, tooling tests, application tests and renderer checks passed.
Across 65 test-suite results, 829 tests passed, none failed and one real-art
test remained intentionally ignored. `git diff --check` is clean.
Earlier completed evidence:

- `/tmp/replay-removal-baseline-host/run.json`: finished, exit 0, pre-change
  Gathering continuity/Undo/restart characterization.
- `/tmp/replay-removal-red/run.json`: finished, exit 101, the new save contract
  detected the unwanted lifetime observation tape before implementation.
- `/tmp/replay-removal-headless-run5/run.json`: finished, exit 0, headless rules,
  continuity, driver and new-format refusal tests at the intermediate candidate.

The final proof includes all four modes, live results/story/draws, safe Undo and
hidden-information barriers, pending choices across fresh processes, malformed
and unsupported saves, stale/busy/failed writes, exact archive bytes and recovery
rotation. The ordinary 250-choice Gathering terminal witness requires its new
save to remain below 256 KiB, versus the retained 8 MiB admission ceiling.
These are bounded checks, not proof of every possible full-scenario interaction.
No desktop launch, physical-input review, GitHub mutation or publication is part
of this implementation. Native appearance/pacing and #89's full scenario owner
acceptance remain separate.

## Retrospective follow-up

Joel authorized the three recommended follow-ups on 2026-11-04: document and
characterize save-baseline ownership, add EOF-safe line windows to the existing
bounded reader, and report build-lock timeouts without contaminating child stdout.
The starting point is the verified local replay-removal patch over `c946d639`;
its tracked diff is retained at `/tmp/replay-retro-entry.patch` with SHA-256
`50422037ab7d68a8afeb228ce0a36178fd82091d58a3d7414450dd6f6ba018d1`.
These changes address repeated setup/read/check failures observed in this session
and briefly displace further Gathering play work. The accepted test boundaries
are the public persistence API, reader CLI and protected launcher. Existing
tests cover the unchanged behavior; new CLI behavior follows red/green tests.
The two-active-hour and non-convergence limits apply. Completion is the bounded
local patch and passing repository checks; native play and publication remain
outside this follow-up. Consulted `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`,
`EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `AGENT-5`,
`OPS-1`–`OPS-7` and `GOV-1`–`GOV-3`.

The public save/restart APIs now link to the file-ownership contract. Four
characterization cases prove that saving a clone does not authorize the original
Session to replace that file, and that the saved instance can continue normally.
The reader accepts `--lines N` and batch `lines` fields, clamps those windows to
EOF, and retains strict explicit ranges, shared byte budgets and source-bound
continuations. It was also used on the actual shortened `src/gathering/moments.rs`
with a 120-line request, returning its 83 lines successfully.

The protected launcher routes flock's diagnostics to stderr and restores the
child's original stdout before execution. The existing systemd limits, real
directory lock, 30-second timeout and child exit statuses remain in effect.
The regression exercises actual lock contention, refusal to start the child,
Cargo JSON output, and a child's own exit 75 without misreporting a timeout.

Verification receipts under `/tmp/replay-retro-*/run.json` record the expected
reader and lock diagnostic failures before implementation (`reader-red1`,
`reader-red2`, `lock-red`) and passing focused checks (`save-ownership`,
`reader-boundaries`, `lock-green`). `api-docs` finished with exit 0 with broken
Rustdoc links denied. The final `/tmp/replay-retro-full-check/run.json` is
`finished`, exit 0: 835 tests passed, none failed, and the existing real-art test
remained ignored. Formatting, strict Clippy, tools, application and renderer
checks passed. `git diff --check` is clean. At this local checkpoint the changes
were uncommitted; no native play or GitHub mutation had been performed.

## Publication authorization

Joel subsequently authorized publication, PR creation, merging and pulling from
`main` for the complete replay-removal patch and these three retrospective
follow-ups. The publication baseline is `c946d63988e3d246cb3e841dec7ff66a24d1d461`.
The PR owns the exact candidate commit, independent Standards/Spec review and
required CI evidence. This authorization includes the deliberately unsupported
old-save boundary; it does not establish native appearance, pacing or complete
Gathering owner acceptance. Issue #89 stays open for that remaining acceptance.
Publication adds consultation of `AUTH-1`–`AUTH-3`, `PUB-1` and `AGENT-1`–`AGENT-5`.

Both publication reviews identified one introduced history-access gap:
Jacqueline's native panel still selected only the latest seven entries after
replay navigation was removed. It now renders the complete retained history in
its existing scroll panel. The production-panel regression
`history_panel_keeps_every_live_entry_available` builds a legal completed session
with more than seven entries and requires every entry without changing rules
state. `/tmp/replay-publication-history-red3/run.json` records the missing-entry
failure (exit 101); `/tmp/replay-publication-history-green/run.json` records the
corrected test passing (exit 0). The PR retains the refreshed review and final
post-repair check evidence.
