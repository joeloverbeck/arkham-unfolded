# Gathering candidate

**Status, 2026-11-14:** Joel accepted complete Gathering play on his own playtests, and [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) is closed as completed. Its [closing record](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6868075979) states that acceptance's evidence limits and the checks it released unverified. The dated records below describe their own time, including statements that #89 remained open.

**Current entry, 2026-11-07:** ordinary launch opens [scenario selection](../../README.md#launch-and-resume-the-gathering). Select The Gathering for **New attempt**, **Continue** or **Open saved attempt**. **Menu → Save and return to scenario selection** saves before leaving and keeps the game active on failure. The explicit `--experiment gathering --save PATH` commands below remain diagnostic paths. [ADR 0005](../adr/0005-retire-experiments-and-select-scenarios.md) retires the three earlier standalone experiments; this does not establish the full native playthrough or owner acceptance under #89.

The [tabletop presentation contract](../tabletop-presentation.md) owns current
display and interaction requirements. ADR links in the dated records below pin
their original evidence rather than defining a second current contract.

**Artwork contract amendment, 2026-11-07:** [ADR 0004](../adr/0004-require-artwork-for-3d-play.md)
requires complete artwork for 3D play and a blocking startup recovery window.
Gathering refuses incomplete collections and wait for every required
image and the symbol font to load before play. **Choose asset folder** selects a
complete collection, **Retry** rechecks it, and **Quit** leaves saves unchanged.
The removed `--no-art` option is rejected. Dated substitute-presentation evidence
below describes earlier builds. Headless checks remain artwork-independent.
[Issue #124's handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/124#issuecomment-6706875824)
records verification and remaining native acceptance.

## Retrospective verification improvements — 2026-11-06

Joel authorized the four retrospective recommendations after PR #112, based on
`c743144a1bf4f57e3284e6a13a68e0722113ee7a`, as bounded enabling work for
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). The observed
bottleneck was ambiguous UI test selection, unbound check inputs, oversized
failure diagnostics and incomplete projected asset-counter coverage. Existing
`record-check`, test helpers and layout measurements are the consumers and
incumbents; this increment extends those paths. It displaces further scenario
work only through implementing and checking these four changes, then returns to
the scenario's existing acceptance boundary. No gameplay or visual redesign is
part of this follow-up. Consulted `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`,
`EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `AGENT-1`–`AGENT-5`,
`OPS-1`–`OPS-7` and `GOV-1`–`GOV-3`; the ordinary two-hour/no-increment and
three-failed-repair-cycle limits apply.

### Separately deferred asset-caption defect

The new production-path projection test exposes a pre-existing caption/card
overlap at normal investigator focus. For Peter in Patrice's first asset slot,
ready, with fallback artwork at 1280×720 and scale 1, the Damage caption ends at
screen y=283 while the card starts at y=275.988: about seven pixels of overlap.
The earlier world-anchor test passes because it does not measure rendered glyphs.
Evidence: `/tmp/retro-gathering-tests-1/check.log`; 100 other Gathering tests
passed. This is a headless geometry observation, not a new native inspection.

Joel explicitly selected **keep the visual fix separate** on 2026-11-06.
`peters_projected_harm_labels_clear_tokens_actions_and_card` is therefore an
explicitly ignored reproducer, not passing caption-clearance evidence. Run it
with `scripts/dev cargo test --locked --features tabletop --lib
peters_projected_harm_labels_clear_tokens_actions_and_card -- --ignored`.
Its complete cases include ready/exhausted, loaded token-art handles/fallback,
1280×720/1600×1000 and scales 1/2; the observed failure above occurs first, so
later caption cases are not yet established. No external artwork is required.
An additional conservative projected-bound check also overlaps an action-token
rectangle in this first case (`/tmp/retro-full-check-1/check.log`). It remains in
the deferred reproducer; a rectangle intersection alone is not native evidence
that the disk silhouettes collide. The existing world-space action-clearance
test stays active, and the companion projected token/card-clearance regression
runs normally across all cases. The previously accepted
visuals remain unchanged; fixing this defect and removing the ignore belong to
separate owner-selected work.

## Supplies, asset harm, optional windows and sealed-token follow-up

After Joel reported Peter's harm counters overlapping Patrice's action tokens,
the shared Damage/Horror disks were reduced from 0.70 to 0.496 units in diameter,
matching ammo/bounty counters, with proportionally resized artwork and 0.096
thickness. Card-mounted counters move about 0.08 units toward the upper edge;
investigator-area counters move 0.08 units closer while keeping caption anchors.
The new Peter regression checks both ready/exhausted orientations and action-row
clearance. Existing projected checks retain separate token/caption bounds and the
caption-to-card gap at both tested viewport sizes and display scales.
`/tmp/harm-token-spacing-check-2/run.json` and
`/tmp/harm-token-spacing-build/run.json` both finished with exit 0. This follow-up
supersedes the executable identity below. Joel subsequently confirmed the visuals
natively and accepted the rest of this session's implementation on 2026-11-06,
then explicitly authorized publication, PR creation, merge and pulling `main`.
That owner report accepts this bounded increment; it does not establish completion
of the entire scenario under #89. Publication consulted `AUTH-1`–`AUTH-3` and `PUB-1`.

The 2026-11-06 local implementation against
`f24111eb975ed5ac659d02698b747759bc7404bc` implements the
[accepted follow-up design](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#supplies-asset-harm-player-windows-and-persistent-premonition).
Supplies retain the shaped resource counter at zero and are named explicitly in
context and inspection. Tennessee Sour Mash is the supplies card in the selected
starting decks. Asset Damage/Horror counters show zero/current damage or horror
over the supported capacity, including Lita; absent capacities have no counter.

The native adapter commits administrative enemy-phase choices through the
existing save-before-install path. No-activity detection checks Hunter movement,
attacks, agenda movement and legal player opportunities. Existing recorded choices
remain accepted. Retained windows name and mark their actual ability sources;
Violin's discard-payment candidates are not advertised as playable abilities.

Premonition stays in its controller's open drawer, including while its mandatory
replacement is pending. The reveal control identifies its known faces and any
additional fresh reveals. Tokens travel from the card before it moves to discard;
the drawer closes after its remaining content finishes. The existing qualified
rules interpretation for Jacqueline, Olive and multiple-token reveals is unchanged.

Verification records are local:

- `/tmp/gathering-fixes-full-check-4/run.json`: `finished`, exit 0 for
  `scripts/check`, including tools, format, Clippy, application/integration tests
  and renderer tests. The application library result is 437 passed, one optional
  external-artwork test ignored. New regressions cover supplies at zero, real asset
  capacities, automatic advancement and saved history, actual opportunity markers,
  exhausted-enemy agenda movement, token origins, History pause/redraw, ordered
  discard/drawer closure, and rich-symbol caption fading.
- `/tmp/gathering-fixes-native-build/run.json`: `finished`, exit 0 for
  `scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop`.
  Executable SHA-256:
  `4ced1f01e67b9d916e77d8f86066601a08a2a52043aa74c9dcc89835cc6dc6e7`.
- Independent Standards and Spec reviews found the glyph fade, Violin payment
  cue, and inactive-enemy explanation gaps; all were corrected and re-reviewed.

These checks establish source behavior and production scene/UI transitions.
No agent desktop interaction or new artwork-loading check is claimed. Joel's
subsequent native confirmation and session acceptance are recorded above; they do
not imply that he executed every checkpoint in this suggested route: inspect Sour Mash at zero,
Peter and a single-capacity asset, pass an inactive enemy phase, then seal and consume
Premonition with and without optional fresh reveals. Inspect the token departure,
subsequent card discard, drawer retraction, History pause, and fresh-process
continuation. Use the existing bounded native-verification procedure; personal
saves and preferences were not used during these checks. Issue #89 retains full
scenario playthrough and owner acceptance.

Replay has been retired in all four modes. Saves from before this change are unsupported and remain untouched; use `--save` with a new path to start an attempt. Development builds promise no save compatibility; see [Record compatibility](#record-compatibility). New-format saves retain exact continuation, action history and applicable Undo. There is no recording export or historical reader. [Decision and scope](../adr/0003-retire-replay.md). Dated evidence below describes its original builds; references to replay in those historical reports are superseded.

This is the local implementation candidate for original Gathering
under [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). It uses the
actual three selected starting lists. **Completing mulligans begins ordinary
scenario play.** The first full round and all following mythos draws have
driver-level evidence. The corrected interface shares Party's production screen,
portrait rail with **TURN**, inspection and story-reading panels, physical cards,
captions, investigator counters, counted decks, chaos bag, Reveal control and
motion paths. The [skill test feedback verification](#skill-test-feedback-verification)
and [earlier presentation verification](#presentation-correction-verification)
distinguish completed checks from the remaining native playthrough and owner acceptance.

Joel accepted the [setup visibility, automatic live presentation and Scenario
layout refinement](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-setup-visibility-and-automatic-live-presentation)
on 2026-11-03, including a victory-pile caption with its card count and summed
Victory points, and then authorized implementation. The local candidate now
shows the initial Scenario and Test arrangement before lead selection and
through mulligans, including after restart. This is a projection of public setup
membership; actual deck assembly, random consumption, legal choices and records
retain their original order. Each physical Arcane Research has its own caption.

Ordinary live consequences finish automatically, including while panning or
changing focus. Inspection and blocking dialogs suspend them only while open;
story reading still requires Continue.
The pool and bag share one row. Scenario has matched heading spacing, adjacent
encounter piles at upper left, a victory pile at upper right, a book nearer the
far edge, and a bounded Map region. The Victory display shows its card count and
summed points, excluding resolution bonuses. Eligible locations join that displayed
pile at scenario end without changing their awards.

The [research return](../research/issue-89-rules-investigation.md) supplies the
source-derived behavior. This implementation pins that research baseline and
taboo 10. On 2026-11-02 the accepted ArkhamDB taboo source was refreshed and matched
the retained bytes exactly (`d63d719bd025040fd74281c123cc692ce521f305e78d5588e7c83eed31801673`):
latest ID 10, effective 2026-02-19. The publisher resource page still lists Legacy
FAQ 2.5 and Grimoire 1.1; the existing original-content precedence remains applicable.
This dated check is not a guarantee about a future campaign's latest version.
The original deck revisions and explicit weakness selections are
visible in the observation. Original ArkhamDB snapshots remain unchanged.

## Counter, shuffle and enemy-board verification, 2026-11-05

The local candidate on `feat/gathering-counters-and-shuffling` starts at
`92ad202b052105bc88a69bdafd317c03363f41c3`. It implements the
[accepted contract](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-card-counters-and-shuffling).
Before the post-native layout changes, `scripts/check` completed with exit 0, retained at
`/tmp/gathering-implementation-checks/final-check-2/run.json`; the normal app and
asset tools build completed with exit 0 at `final-build/run.json` in that same
directory. Standards and specification reviews of that earlier candidate found
no unresolved source findings after restoring enemy descriptions and interrupted-draw
custody. Those results do not verify the subsequent working-tree geometry.

The rejected enlarged-board candidate failed the later required check,
`/tmp/gathering-implementation-checks/final-check-3/run.json`, finished with exit
101: 428 library tests passed, two failed and one was ignored. The wider Scenario
framing introduces existing caption collisions between Chaos bag and Committed
cards, and between Deck and Agenda 1 at 1280×800. The owner-authorized extension
of 30 minutes and at most two further layout repair cycles reached its cycle
limit. Implementation stopped at that point. That historical candidate is retained
in `/tmp/gathering-implementation-review-v7-stopped`.

Its source reviews also identified two unclosed consumer paths. The
`CompactLabel` marker on bounty/doom text does not reach the `WorldLabel` query;
those shared labels remain 11 pixels. Missing damage art with a present symbol
font creates explicitly sized child spans, which the root-font adjustment does
not resize. Their actual crowded overlap was unverified.

Joel subsequently inspected the app and rejected the oversized board. The current
correction follows his revised priorities: the enemy board is 8 × 22 world units,
aligned with the scenario/map board, with navy felt and a steel-blue rim. Scenario
and Overview restore their earlier camera distances. Names, location headings
and damage labels keep their ordinary 13-pixel text; icon spans retain their
intended 1.25× size. Bounty/doom counts retain the shared token renderer's
11-pixel text. The ineffective adaptive resizing markers and arguments are removed:
shared counter labels continue through their own positioning system, and damage
text keeps the established plain/rich-text construction. Text is no longer
shrunk to make crowded captions fit.

Caption-overlap tuning is explicitly deferred to Joel's later feedback. The
updated regression checks the map-sized board, exposed physical cards, correct
group membership, visible labels and stable font sizes through the actual label
paths, with the symbol font present and absent. This replaces the former crowded
glyph non-overlap criterion; it does not assert that every caption is separated.

The compact correction's focused checks, full `scripts/check`, and normal app
build all finished with exit 0. Their retained `run.json` records are under
`/tmp/gathering-implementation-checks/compact-board-green`,
`compact-board-full-check`, and `compact-board-build`, respectively. The full
gate includes 430 passing library tests (one ignored); both Scenario caption
checks that failed on the enlarged candidate pass again. Separate Standards and
Spec reviews of `/tmp/gathering-implementation-review-v8-compact` found no
introduced findings. Its manifest SHA-256 is
`116450401d5e6cca74eb0a4fefb9d3d8c0fd8bf3eccfca241bb0251e1d5e0275`.
The rebuilt `target/debug/arkham-tabletop` SHA-256 is
`68b39b9135d3be156e12967eb9773b766f57749038055a4369528469eb8b9e7d`.
Quit and relaunch the app to use that executable with the existing save. These
checks complete the revised local implementation scope; the new palette and
native readability have not been inspected in a further agent desktop session.

Production-renderer fixtures, production-registered interaction/animation tests
and rules tests cover numeric/symbol chaos rows, typed
zero counters, separate enemy actions, keyboard cross-highlighting, the finite
twelve-enemy layout pressure (including `9+1+1+1`), shuffle ordering and Menu
pause, save preservation, and the retained anonymous pile during an interrupted
draw. The discard remains visible until its recycle beat. These checks establish
the selected scene/state behavior, not native appearance, physical input or pacing.

The collection prepared for that check verified 143/143 roles. The earlier
collection supplied 140 verified roles, with named substitutes for bounty, ammo
and charge artwork. That preparation left the source archive, personal saves and
earlier collection unchanged. The later [installation consolidation](#artwork-preparation-and-selection)
replaces those cache paths with one stable installed collection. The supplied
symbol font has no bounty glyph.

Ordinary review saves are prepared at `/tmp/gathering-presentation-review-saves`
(opening/mulligan) and `/tmp/gathering-presentation-resolution-saves`
(`evaded-enemy.json` opens on the saved result; Continue reveals its exhausted
enemy). Their production startup paths are covered by the existing review-save
checks. Joel authorized a native interval from 20:42:33 to 20:52:33 UTC. All
owned apps and portal grants were closed by 20:51:20. Isolated copies showed
Tony's centered Bounty Contracts counter at 6, his played .38 Long Colt's ammo
counter at 3, and an exhausted Swarm of Rats on the sideboard with `0/1` damage,
the matching Study marker and its own contextual actions. Hover linked the
heading and marker. Source saves and personal settings were unchanged.

The native evidence is under
`~/.cache/agent-tmp/gathering-presentation-2026-11-05/`: the relevant captures are
`mulligan-native/interaction/ammo-and-bounties.png`,
`enemy-native/interaction/enemy-board.png` and
`enemy-native/interaction/enemy-inspection.png` (a contextual menu, not the
Inspect modal). That pass exposed a header/damage clearance problem. A further
capture inside the same interval, `enemy-clearance/scenario.png`, showed the
first-row correction. Its executable was
`93940583bd3c42d730e8f5062dbe6104691cc9d793c652ca67eb22c2b698b942`.
Subsequent glyph measurements drove an intermediate 15.6 × 44 board with text
as small as six pixels. Its focused clearance test passed
(`crowded-layout-extension-2/run.json`), but the full checks failed and Joel
rejected the height in ordinary use. The compact correction above supersedes
that geometry and text policy. It has not been inspected through a new agent
desktop session.

The retained shuffle clip framed a deck that did not shuffle, so it does not
verify shuffle pacing. That agent-driven interval also did not cover native
charges, zero asset counters, enemy bounty/doom or crowded arrangements. These
remain limits of the recorded agent evidence, separate from Joel's later acceptance.

### Owner acceptance and publication

After the stable artwork installation was completed, Joel tested the application
and reported on 2026-11-05 that the requested changes were "good enough for now to
consider this finished." He explicitly authorized publication, PR creation,
merging the PR and pulling `main`. This accepts the requested chaos-result,
shuffle, counter, enemy-sideboard and artwork-handoff increment under the revised
compact-board and ordinary-text requirements. Caption-overlap tuning stays
deferred. It does not assert that he executed every prepared review checkpoint,
and #89 remains open for complete original Gathering play and acceptance.

The publication candidate includes the exact local implementation verified by
the retained checks above and the installation checks below, plus these acceptance
records. Publication follows the protected PR path and the required `Rust checks`
for the reviewed head; the PR and current #89 handoff retain its commit and CI
identities. Consulted for publication: `AUTH-1`–`3`, `OPS-5`–`7` and `PUB-1`.

## Card-draw review preparation

The [current #89 handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
owns the candidate, completed checks and remaining native observations. Prepare
the route before a desktop interval. These commands create no windows or grants:

```sh
scripts/tools record-check --output-dir /tmp/gathering-draw-review-preflight -- \
  scripts/dev cargo test --locked --features tabletop --lib \
  gathering_tabletop::tests::review_saves &&
scripts/tools record-check --output-dir /tmp/gathering-draw-review-prepare -- \
  scripts/dev cargo run --locked --example gathering_draw_review -- \
  /tmp/gathering-draw-review-saves
```

Use fresh output directories. The example refuses an existing destination and
round-trips every emitted save through the ordinary loader. Its seed-89 builder
uses legal recorded choices and is shared with the production-startup UI tests;
it does not edit decks or create privileged fixtures. Its output lists each save's
revision, first choice and label. Headless checks establish visible controls,
manual camera preservation, draw-anchor clearance at 1600×1000 and 3200×2000,
and endpoint reopening; native readability and motion still require inspection.

| Save | First action | Initial view | Observation |
| --- | --- | --- | --- |
| `opening.json` | Choose Tony as lead | `tony` | First five arrivals from deck into Tony's hand; the other hands need their own framing. |
| `mulligan.json` | Confirm the two marked replacements | `tony` | Aside pile, replacements and rejected-card return. |
| `action.json` | Draw using Tony's deck menu | `tony` | One ordinary draw and one action spent. |
| `upkeep.json` | The emitted upkeep draw choice | `tony` | One upkeep draw, then the remaining ordinary phase choices and Patrice's refill. |
| `encounter.json` | Draw the encounter card normally, declining On the Hunt | `overview` | Encounter deck → Test → actual destination or pending test/choice. |

After authorization, use the existing `native launch` command with
`--experiment gathering --load /tmp/gathering-draw-review-saves/encounter.json
--initial-view overview`, the accepted binary/art root and a fresh output
directory. The launcher copies the input save; autosaves remain isolated. Inspect
the actual client framing before the trigger. In particular, Overview obscures
opening-hand destinations behind the decision panel; use the investigator view
for those clips. For an encounter leaving Test, reframe its destination as needed
and report any unobserved segment rather than treating an endpoint as full motion.

For a fresh ten-minute interval prioritizing the unobserved paths, fill absolute
times in the compact handoff before launch:

| Elapsed cutoff | Route checkpoint |
| --- | --- |
| 0:00–2:30 | Launch encounter save, consent/calibrate, inspect framing and record one encounter. |
| 2:30–5:00 | Close that app/helper, launch upkeep save and record upkeep/refill as reachable. Any new helper keeps the original deadline. |
| 5:00 | Stop optional gameplay; inspect the saved result and action history. |
| 7:00 | Close and reopen the run-local saved endpoint in a fresh process. |
| 8:30 | Stop optional recording; preserve results and begin closure. |
| 9:30 | Admit no further input; close the portal and app and verify terminal status before 10:00. |

These are cutoffs, not permission to extend a step. If startup, consent or a real
decision consumes the budget, skip optional work and report what was not tested.
The helper's fifteen-second command allowance plus fifteen-second cleanup reserve
remains controlling. Use the [single-click capture option](../agents/native-verification.md#launch-capture-and-interact)
for still observations; keep short window recordings for motion. Never switch
targets without re-verifying process/window identity. No interval is active merely
because these saves and instructions exist.

## Native card-draw check, 2026-11-04

The authorized interval ran 16:14:18–16:24:18 UTC against local draw candidate v2
above `c313ede262de3ed0e294028b4dae8291162ec452`. Executable SHA256:
`6df9f4bd6e49244e2ebbe6ad65df3fccbacaf019f47e99bf79ce7d6012fcdc32`.
Observed: two-card mulligan replacements and return, On the Hunt drawn into Tony's
hand, Internal Injury reaching his threat area, and Replay Next/Back. Opening
reached five cards per investigator, but Overview obscured hand destinations.
Tony's framing omitted the weakness's intermediate Test reveal. Replay Next
restored following and moved to Scenario, as at the baseline.

Two temporary mulligan captions overlapped. The subsequent v4 correction keeps
one pile caption and removes return captions; its production regression passes.
The corrected appearance, encounter/upkeep motion, inspection during flight,
Return to live and fresh-process reopening were not checked natively. These
observations do not establish owner acceptance of pacing or complete scenario play.

The portal closed at 16:23:49.896 UTC after refusing the final Live click before
input. The launcher was deliberately interrupted (exit 130; child returncode -15);
cleanup finished by 16:24:03. All five encoders exited 0. Videos, captures, the
isolated revision-9 save and numbered events remain under
`~/.cache/agent-tmp/arkham-draw-native-20261104/run/`; their hashes are retained in
`/tmp/gathering-draw-native-artifacts.sha256`. Personal saves matched their retained
hashes. The dated full record is `/tmp/gathering-draw-native-evidence-2026-11-04.md`.
Current status belongs in the [handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060).

## Skill test feedback verification

The 2026-11-04 correction is local on `fix/gathering-test-feedback`, based on
`8852b2937e748c5260111ee573ec7ab24419eccf`. It implements the accepted
[result-reading and six-defect scope](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-presentation-and-skill-test-feedback).

| Evidence | Result |
| --- | --- |
| Full repository check | `/tmp/gathering-feedback-check-v4/run.json` is `finished`, exit 0. Coverage includes direct optional effects, stale input, defeat and committed-card cleanup, saved explanations, old pending tests, focus bounds, symbol notation and card geometry. |
| Production code review | Both standards and specification reviews found no remaining issues in `/tmp/gathering-feedback-review-v4/candidate`; `manifest.sha256` binds the tracked diff and four new files. This includes the prepared-save startup regression described below. |
| Prepared artwork and font | The ignored `real_collection_loads_every_verified_image_and_symbol_font` test passed separately with the actual prepared collection. This verifies loading, not native appearance. |
| Native executable | `/tmp/gathering-feedback-native-build/run.json` is `finished`, exit 0. Executable SHA256: `2344b9ed76a4da9253440f2f1c87e88b8c4822ea608a9761ad3dd2d12ac9ff85`. |
| Prepared-save startup | `/tmp/gathering-feedback-startup-check/run.json` is `finished`, exit 0. The ordinary revision-10 save opens with **Reveal chaos token** and no blocking modal. Revision 11 opens on Tony's 7-versus-2 result with **Continue**, followed by direct Quick Thinking use and decline controls. The test uses the native startup loader and production headless UI host. |
| Targeted native check | The authorized ten-minute interval exercised result reading, Quick Thinking use and decline, fresh-process result recovery, replay entry/return, Restart label and confirmation, and Hard Knocks inspection. Observations and limits are recorded below. |
| Remaining evidence | Long-result scrolling, native narrow-window layouts, enlarged comparisons of the other reference cards, physical keyboard input and complete scenario play were not exercised. Owner acceptance remains separate. |

The prepared ordinary saves reproduce the original recorded decision prefix
through current player choices; they are not verification fixtures. They are
available under `/tmp/gathering-feedback-native/`:

- `quick-thinking-before-reveal.json`, revision 10, SHA256
  `95d7d3af6c67023cc7e87051989e4aa16437696e4c6dd5c8c470ea25b5e28eb9`.
- `quick-thinking-result.json`, revision 11, SHA256
  `2e0d0f758c8a25d156978e6bb85cf66e29c5ebb591f51a272ab4cfcbc75578ff`.

The production driver successfully reopened the latter, entered replay, advanced,
and returned to live revision 11 (`driver-readback.jsonl` in the same directory).
The original `local/gathering/session.json` remains byte-identical, SHA256
`3914b8c1337a3291528d75f097d520b3c7b0cc911277587d232163c46c9b8175`.

### Native feedback check, 2026-11-04

Joel authorized the prepared ten-minute desktop check. The interval ran from
13:12:10 UTC with a fixed 13:22:10 deadline, including startup, consent and cleanup.
Both apps and portal grants were confirmed closed by 13:21:53. The two runs used
isolated copies of the prepared ordinary saves, the executable identified above,
and the actual 140/140 prepared asset collection. Client captures were 3200×2000
at the desktop's 2× window scale. No source correction was needed during the check.

Evidence is retained under
`/home/joeloverbeck/.cache/agent-tmp/gathering-feedback-native-20261104/`:

| Observed path | Evidence and result |
| --- | --- |
| Reveal and read | `live-run/interaction/resolved-result.png` and `result-still-visible.png` show the same result about 22 seconds apart: base Intellect 3, Tony's Quick Thinking +1, Patrice's Unexpected Courage +2, token +1, total 7 against 2, succeeded by 5. Token artwork, owner names, arithmetic and Continue were readable. The panel remained until Continue. |
| Use Quick Thinking | `quick-thinking-choice.png` shows direct use and decline controls. `granted-menu.png` and `granted-action-complete.png` show the granted resource action. The saved revision 13 records `tony:quick-thinking:tony` then `granted:resource`; Tony goes from 2 to 3 resources while retaining one ordinary action. |
| Card shape and symbols | `hard-knocks-inspection.png` shows the enlarged rounded artwork; `hard-knocks-symbols.png` shows font glyphs for free-triggered abilities, Combat and Agility without brackets. Wheel input scrolls the inspection body while Close stays visible. Tony's table also displays the rounded card treatment on the other public faces and backs; only Hard Knocks received the enlarged corner comparison. |
| Restart controls | `restart-menu-label.png` shows the supported trailing dots. `restart-confirmation.png` and `restart-hover.png` show the full Cancel focus outline and Restart hover highlight, with bottom clearance. Cancel returns to the unchanged live table (`restart-canceled.png`). |
| Fresh process and decline | `result-run/interaction/initial.png` restores the saved result and Continue. `result-run/interaction/reopened-choice.png` and `result-run/interaction/explicit-decline.png` show explicit decline followed by ordinary play. Saved revision 12 records `after-test:pass`, with 2 resources and one ordinary action. |
| Replay round trip | `result-run/interaction/replay-open.png` shows recorded setup, and `result-run/interaction/returned-live.png` restores the live state after decline. The interval did not navigate replay through the result itself or exercise the separate Reopen menu command. |

All unqualified capture names in the table belong to `live-run/interaction/`.
The result body fitted this window, so the successful inspection scrolling does
not establish overflow behavior of the result panel. Encounter tests, multiple-token
reports, Action glyph appearance and other consumers retain automated evidence
only. These observations do not establish full-scenario or owner acceptance.

Each run retains `run.json`, `app.log`, its isolated `session.json`, and portal
`interaction/events.jsonl`. Both portal helpers acknowledged stop and
`session-closed`, exiting 0: first at 13:19:21.593 UTC, second at 13:21:49.847 UTC.
The launchers were then deliberately interrupted for cleanup (tool exit 130);
their manifests accurately retain `interrupted-or-failed` and child `returncode`
−15. Process inspection confirmed both app PIDs absent before the deadline.
The original personal save hash remained unchanged after cleanup. The compact
route and runtime identities are retained in `/tmp/gathering-feedback-native/route.md`.

## Presentation correction verification

Current status as of 2026-11-03:

| Evidence | Current result |
| --- | --- |
| Automated checks | The full `scripts/check` passed after the retrospective follow-up: `/tmp/gathering-retro-full-check/run.json`, `finished`, exit 0. The three prepared-save startup checks also passed separately at `/tmp/gathering-retro-startup-check/run.json`. Production layout coverage compares rendered caption and heading glyphs with each other and both raised book meshes at 1280×800 and 1600×1000, at 1×/2× scale. |
| Code review | Both axes found no introduced issues in the final caption candidate. The exact source and new-file identities are retained in the [verification history](#verification-history). |
| Targeted native checks | Confirmed early Scenario setup, individual Arcane Research captions, Hard Knocks returning to ordinary play without acknowledgment, aligned bag/pool models and captions, expanded Map layout, heading clearance, and the final Agenda/Act captions clear of the book. Artwork and substitute captures covered initial Scenario layout. |
| Still unverified | Direct Act inspection, a complete native Gathering playthrough, physical keyboard and simultaneous input during motion, and owner acceptance. The targeted checks do not close these gaps or establish Party caption clearance. |

The final native capture used executable SHA256
`0ada4534ce80c8387934ef2b29b5a57331ea1b2decc7430e4badc32a9b1a12da`.
The retrospective check rebuilt the executable as
`4b42c7fc398a8e06cf9f6b55ffd7cc8fc5fae359a4bd097726b2027b7ae28ccd`;
the native observations refer to the captured binary, with no additional native
interval for the subsequent save-loader extraction.
The [history](#verification-history) retains its build/check records, the three
native intervals and the earlier failures repaired during them. Historical
pending statements describe those earlier candidates, not the current status.

The approved retrospective follow-up retains the geometry regression, adds
production-startup checks for ordinary prepared review saves and their visible
story barriers, tightens baseline evidence in code review, and documents bounded
process diagnostics. Native startup and the new headless checks share the same
save loader; no game rule or story acknowledgment changes. The checks reproduce
the revision-61 Act reading panel and verify that revision 62 exposes all four
map locations without that modal. Consulted
rules: `PURP`, `WORK`, `EVID-3`, `EVID-4`, `EVID-5`, `EVID-6`, `BOUND`, `INT`,
`AGENT`, and `OPS`.

### Verification history

<details>
<summary>Retained correction, review and native evidence</summary>

The 2026-11-03 correction starts at `9cd5d8c`. The pre-native `scripts/check`
record at `/tmp/gathering-ui-verified-check/run.json` is `finished` with exit 0;
its complete output is `check.log` in that directory. Coverage includes initial
setup, restart and reopening, unchanged setup recordings, individual permanent
captions, automatic presentation during camera navigation, retained story and
inspection barriers, empty/populated Scenario font layout at 1280×800 and
1600×1000 at 1×/2× scale, and Victory membership/counts/points in live observations,
reopening and Replay. Terminal-location witnesses are explicitly privileged
presentation inputs; the recorded Priest fight exercises the actual rules seam.

The standards review found no code findings. The requirements review identified
a missing shared heading consumer in the older Jacqueline experiment; that
consumer and its title wrapping were corrected. Both review axes found no
remaining code issues in source snapshot
`aef160b7255d970ca234ba82db1ab50b0e525c293762ec735ee72a05464958ff`
(`/tmp/gathering-ui-review/candidate-v3.patch`, plus its separately captured new
Victory test). The review does not establish native readability or owner acceptance.

The normal native build also finished with exit 0 at
`/tmp/gathering-ui-ready-build/run.json`. Executable
`target/debug/arkham-tabletop` has SHA256
`18aab4dcbcbd88c420f48cf2d2f5fbed9f31ea7f17750dc177c2f7846128f4a9`.
Prepared ordinary seed-89 review saves are `/tmp/gathering-ui-start.session.json`,
`/tmp/gathering-ui-hard-knocks.session.json` and
`/tmp/gathering-ui-expanded.session.json`. They use legal decisions and carry no
verification-fixture flag; the expanded map is at revision 61. Preparation is
recorded in `/tmp/gathering-ui-prepare.rs`.

Joel authorized a five-minute native interval on 2026-11-03, from 18:52:19 to
18:57:19 UTC. With the executable above, pointer input played Tony's Hard Knocks:
resources fell from five to three, actions from three to two, the asset appeared,
and normal decisions returned without a presentation acknowledgment. A subsequent
right-drag orbited the camera. The input grant closed at 18:54:16 UTC; the
interactive launcher then exited 130 on intentional cleanup. This does not
establish physical keyboard behavior or simultaneous input during card motion.

Three isolated static runs exited 0. The initial Scenario capture showed the
Study, standees, Act/Agenda, encounter deck, empty named pile footprints and Map
border before lead selection. The bag and pool captions shared a row. The
Jacqueline capture showed a separate Arcane Research caption for each permanent.
The expanded-map save opened the Act story screen, so that capture verifies the
retained reading barrier, not expanded-map readability. The last screenshot was
saved at 18:56:48 UTC; no further desktop activity followed the interval.
Manifests, logs, input events and images are retained under
`~/.cache/agent-tmp/gathering-ui-review-20261103-1793731939/`.

Native inspection found the Scenario heading overlapping the raised book. The
correction restores Test's original centered spacing and applies that clearance
to Scenario. The extended production layout regression compares rendered heading
glyphs against both actual page meshes; it failed before this correction and
passed afterwards (`/tmp/gathering-heading-native-regression/run.json`, exit 101;
`/tmp/gathering-heading-native-fixed/run.json`, exit 0). The owner-authorized
follow-up below checks this repair, the expanded map and the dedicated Test view
under the [desktop-interval procedure](../agents/native-verification.md#launch-capture-and-interact).
Owner acceptance remains separate. The compact Victory caption places count and
points on two lines beneath its named footprint; the encounter footprints use
Deck and Discard, as investigator areas do.

After the heading repair, the full `scripts/check` record
`/tmp/gathering-ui-post-native-check/run.json` finished with exit 0. Both review
axes found no introduced code issues in source snapshot
`45248f5d8fdbdac53ec5a4c074178882bf9e220fa26e91b6a59990ff334c77d7`
(`/tmp/gathering-ui-review/candidate-v4.patch`, plus
`new-files/src/gathering_tabletop/tests/victory-v4.rs`, SHA256
`b8631c04aa8892bb7b07919b8565142d1c5fd014aba3af2ca3d0c3398f9cea21`).
The rebuilt normal executable has SHA256
`6d262ae0ed9ac464c5967a9b00a7c0f327f82accf8b8b3b7662c116874b59958`;
`/tmp/gathering-ui-post-native-build/run.json` is `finished` with exit 0.
The isolated expanded-map follow-up save is
`/tmp/gathering-ui-expanded-followup.session.json`, at revision 62 after the legal
`turn:end` decision. `/tmp/gathering-ui-prepare-followup.rs` records preparation;
the original revision-61 input remains unchanged.

Joel authorized a two-minute follow-up from 19:10:27 to 19:12:27 UTC. Four static
runs used that rebuilt executable and exited 0. Initial Scenario captures with
artwork and substitutes confirmed that the heading cleared the book. The
revision-62 expanded map showed Hallway, Attic, Cellar, blocked Parlor, standees
and links within the Map boundary. Test focus showed aligned bag/pool models and
captions, separated from the commitment region. The final image was saved at
19:12:04 UTC and every window closed within the interval; no pointer-input grant
was used. Records and inspected images are under
`~/.cache/agent-tmp/gathering-ui-followup-20261103-1793733027/`.

The expanded view also exposed a remaining Agenda-caption collision with its
book page. Extending the production geometry check to all upper Scenario labels
reproduced a slight collision even for the short setup caption at 1280×800
(`/tmp/gathering-agenda-native-regression/run.json`, exit 101). The subsequent
correction anchors Agenda's right-aligned text left of the book and Act's
left-aligned text to its right, keeping the caption width away from both pages.
This caption correction was made after the native interval and is not verified
by the captures above.

The expanded geometry regression passed afterwards at
`/tmp/gathering-agenda-native-fixed/run.json`. The full repository check and normal
build both finished with exit 0 at `/tmp/gathering-ui-caption-check/run.json` and
`/tmp/gathering-ui-caption-build/run.json`. The resulting executable SHA256 is
`0ada4534ce80c8387934ef2b29b5a57331ea1b2decc7430e4badc32a9b1a12da`.
Both review axes found no introduced code issues in the final caption source
snapshot, `/tmp/gathering-ui-review/candidate-v5.patch` (SHA256
`186b9a34328e6426abfd888a0819ffd97883118e6729b6a91c6d632c22b1c8fa`), plus
`new-files/src/gathering_tabletop/tests/victory-v5.rs` (SHA256
`b607d9341703ad21f987d4534daea30913c2a9452652671ff32913d6a8a0332b`).
A suggested Party caption issue was withdrawn after comparison with the fixed
point: its long centered captions predate this patch, and no introduced
regression was established. That comparison does not establish native Party
caption clearance. The final capture below checks Gathering's caption alignment;
this was the final presentation change before the capture.

Joel authorized one final 60-second interval from 19:23:12 to 19:24:12 UTC. The
expanded-map static capture used that same checked executable and exited 0. The
inspected image shows `Agenda 1 / 0 doom` entirely left of its page and `Act 2`
entirely right of its page, clear of the book and neighboring labels. The
Scenario heading remains clear above the book. The image was saved at 19:23:30
UTC and the window closed within the interval. The manifest, log and `scene.png`
are retained under `~/.cache/agent-tmp/gathering-caption-20261103-1793733792/`.

</details>

## Run and resume

From the repository root:

```sh
scripts/dev cargo build --locked --bin arkham-driver &&
target/debug/arkham-driver \
  --experiment gathering --save /tmp/arkham-gathering-setup.session.json
```

Use a new file path to begin a separate setup. Repeating the command reopens that
file. The JSON driver works offline without artwork or the source archive. This
is a developer-accessible player interface.

All pre-removal records are unsupported; see [Record compatibility](#record-compatibility).

The native table uses the same session and save file:

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop \
  --experiment gathering --save /tmp/arkham-gathering-setup.session.json
```

Click an investigator card to open its contextual menu and choose the lead.
Click each opening-hand card to select or undo its mulligan; confirm the hand
using the table-wide control. During play, cards, piles, locations and revealed
tokens offer their legal actions through the same shared menu renderer as Party.
Clicking a component with only one inspection target opens it directly; piles
retain a contextual chooser for their individual cards. Otherwise choose
**Inspect** for enlarged artwork and effective rules. The encounter deck
inspection lists its starting quantities without revealing shuffled order.
Act and Agenda inspection also includes their previously revealed reverse sides.

The portrait rail and Scenario, Test and Overview buttons retain Party's camera
controls. The **TURN** badge follows the actual actor, not camera focus.
**Menu** at the upper right contains Save, Reopen, Restart, History, reduced motion
and artwork settings. Menus use Tab/Shift+Tab and Enter/Space; Esc
closes them. Unmodified WASD pans even with an ordinary button focused, dismissing
an open card-action list without choosing. Blocking Inspect, history, Menu, story,
test results, restart confirmation and folder-selection dialogs suspend camera input; closing
them restores WASD without clicking the table.

### Read a skill test result

Every completed skill test, including an encounter test, pauses at an untimed
result panel. It shows the outcome and difficulty, base skill, each committed
card and owner, ability and paid bonuses, and the individual chaos-token values.
Canceled and ignored tokens are shown separately. The full detail is available
immediately and scrolls while the outcome and controls remain visible.
**Continue** acknowledges the result; it does not roll again or apply effects.

Optional result effects appear directly with use and decline controls. For
example, a qualifying Quick Thinking offers **Take Tony's extra action
(Quick Thinking)** and **Decline Quick Thinking**. When several effects remain,
**Decline all listed effects** explicitly closes that opportunity window.
A provisional result, such as one that Lucky! can still change, is labeled as
provisional. Token cancellation retains its existing timing and controls.

**Menu → History** includes the recorded test breakdowns. Reopening at a newly
completed test offers its reading panel again without repeating saved effects.
New saves always include complete result details.

The same correction uses a supported Restart label, clearance around confirmation
button focus outlines, rounded physical cards and inspection images, and the
prepared font for recognized bracketed card-text symbols. The original artwork
and absent-font English remain intact. The accepted design and scope are in
[ADR 0002](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-presentation-and-skill-test-feedback).
The [targeted native check](#native-feedback-check-2026-11-04) covers the exercised
reading, choice and inspection paths. Its remaining checks and complete player
journey are distinct from owner acceptance.

### Restart and recover an attempt

Choose **Menu → Restart The Gathering...** in the live table, then **Restart** to
return to lead selection, opening hands and mulligans with fresh shuffles. The
selected investigators, decks, weaknesses and pinned rules stay the same, as do
artwork and motion preferences. Cancel is initially focused; Enter activates
Cancel, and Escape also cancels.

Before the active save is replaced, its exact bytes are retained beside it as
`<save filename>.archived-<unique suffix>.json`. The success notice and terminal
print the archive path. **Reopen saved attempt** resumes the new current
attempt. Later autosaves rotate `.previous` but do not replace the archive.

To resume an archived attempt, first close the running table, copy the archive
to a separate working save, then pass that copy as `--save`. Replace the example
archive path with the printed path; this creates a new recovery directory:

```sh
gathering_recovery_dir=$(mktemp -d /tmp/arkham-gathering-recovered.XXXXXX) &&
cp -- /absolute/path/session.json.archived-ABC123.json "$gathering_recovery_dir/session.json" &&
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop \
  --experiment gathering --save "$gathering_recovery_dir/session.json"
```

This preserves the archive while the recovered copy receives autosaves. **Menu → History** shows the recovered attempt's action history. Restart refuses a
missing, stale, busy or invalid save/recovery file and reports the reason inside
the confirmation. Cancel and resolve that reason before retrying; the current
live attempt and active save are retained.
Headless tests exercise the actual menu controls and save and restart protections.

For a static layout capture, use the [isolated native diagnostics](../agents/native-verification.md#launch-capture-and-interact)
with `--experiment gathering --initial-view scenario --screenshot scenario.png --quit-after-screenshot`
and the prepared artwork root above. The capture waits
for graphics readiness and exits after writing the image. This checks rendering;
it does not verify camera-button interaction or gameplay. Agree the desktop
interval before an agent launch.

Unengaged enemies occupy a separate board to the left of the map, grouped by
location in rows of three. Every card remains exposed. A colored letter/count
marker above the location matches its named group; hover or keyboard focus
highlights the link without moving the camera. Each enemy offers its own actions
and inspection. Location markers offer group access. Ordinary engaged enemies
retain their investigator threat-area positions. Scenario and Overview include
the board; enemy names and counters remain visible at those distances.

Every enemy shows damage/max health above its card, including `0/max`. Bounty
counters stay on its left and doom on its right. Ammo, spell charges and Bounty
Contracts use centered counters, including zero; charge counters combine the
supplied Spell motif with a violet disk. Loaded artwork replaces the named
substitute for that counter. The symbol font has no bounty glyph, so rules text
continues to spell out bounty/bounties.

Actual deck shuffles now lift, split, interleave, square and settle in roughly
1.3 seconds. Mulligan returns and discard recycling precede the mix; following
draws wait for it. Keeping every mulligan card does not invent a shuffle. Menu
and Inspect pause movement; reduced motion skips it and supplies a brief account.
Result panels and History show an ordinary numeric chaos token's value once,
while preserving altered contributions, symbol modifiers and ignored/canceled
status. The [verification record](#counter-shuffle-and-enemy-board-verification-2026-11-05)
separates the inspected native cases from remaining spacing and timing checks.

Newly revealed Act/Agenda reverses use Party's blocking reading layout.
**Continue** acknowledges the reading and resumes presentation; the transition's
rules effects have already been saved. Closing and restarting while that remains
the latest decision offers the reading again without applying its effects again.
Replay reveals only the displayed prefix and keeps its own navigation. This
differs from Party's separately committed `continue-act` step and is a proposed
Gathering presentation choice, not recorded owner acceptance.

The following native evidence is historical and does not verify the correction:
The [artwork-enabled setup evidence](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6590517420)
and [action/replay retest with independent audit](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6590847507)
record the exercised routes and their limits. The retest resumed a copy of the
isolated setup save, began Patrice's turn through her card menu, and gained one
resource: resources changed from 5 to 6 and actions from 3 to 2. Replay showed all
three mulligan owner labels and returned to the unchanged live state. It covered
setup replay moments, not replay of the resource action itself. The repaired
offscreen headings no longer stacked at the window edge in the inspected views.
Adjacent Arcane Research asset captions still run together in the live focused
view; broader populated-layout readability remains unverified. The older no-art
native retest below does not verify the new artwork/menu path.

### Artwork preparation and selection

Normal setup uses **Import artwork** from scenario selection or blocking startup
recovery. Choose one complete runtime ZIP; the app installs and selects it under
`$XDG_DATA_HOME/arkham-unfolded/artwork/`, normally
`~/.local/share/arkham-unfolded/artwork/`. Later ordinary launches reuse it without
`--art-root` or another folder selection. The required collection is
`arkham-unfolded-gathering-v7`; code updates that retain that identity do not require
another import. Imported files remain ordinary extracted runtime files.

For a local package from an already verified collection:

```sh
scripts/dev cargo run --locked --bin arkham-assets -- package-gathering \
  /absolute/path/to/prepared-collection /absolute/path/to/new-gathering-v7.zip
```

The producer prints exact ZIP length and SHA-256, selects only required payloads
and identity/attribution metadata, and refuses an existing output. It includes
the unchanged exported pouch and embedded textures. Keep the ZIP outside Git;
public hosting or redistribution needs separate authority. The archive's own
checksum is an identity check, not publisher authentication.

Import can be canceled from selection or its recovery progress window. Existing
artwork and personal saves survive failure. Imports are refused while an attempt
is open, including from another app process. Replacements retain the previous
managed installation until normal image/font/model/material and renderer admission
succeeds, then remove only its unchanged app-managed predecessor. External chosen
folders and download ZIPs are retained. Insufficient space, stale settings,
incompatible collections and malformed ZIPs explain the failure. Interrupted or
unknown import directories remain unselected and are disclosed; importing again
recovers a usable installation without deleting unknown remnants.

For developer preparation from the original local archive, obtain the pinned
[Lorc reveal icon](https://game-icons.net/icons/ffffff/transparent/1x1/lorc/swap-bag.png)
as `/tmp/arkham-reveal-icon.png`, retain the supplied inner-spark package and
the unchanged [Chalice drops icon](https://game-icons.net/icons/ffffff/transparent/1x1/lorc/chalice-drops.png)
as `~/Downloads/Offerings.png`, then:

```sh
scripts/dev cargo run --locked --bin arkham-assets -- prepare-gathering \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /tmp/arkham-reveal-icon.png \
  --ability-icon /home/joeloverbeck/Downloads/investigator-ability-inner-spark/inner-spark/investigator-ability-inner-spark-glyph.png \
  --offering-icon "$HOME/Downloads/Offerings.png" \
  --chaos-pouch "$HOME/Documents/Arkham Unfolded Assets/Chaos Bag/chaos-pouch.glb" \
  --output "$HOME/.local/share/arkham-unfolded/gathering-assets" &&
scripts/dev cargo run --locked --bin arkham-assets -- check-gathering \
  "$HOME/.local/share/arkham-unfolded/gathering-assets"
```

The finite recipe contains 148 roles: 94 card identities, 11 reverse/unrevealed
sides, card backs, investigator standees, token/class artwork, reveal icon and
symbol font, including bounty, ammo, the Spell charge motif, the neutral key and
the investigator ability spark, offerings icon, doom artwork and the 3D chaos pouch.
The pouch's GLB embeds all nine textures; its Blender source and previews remain
in the external authoring folder. Runtime generates filtered texture levels in
memory and renders the imported velvet sheen with a repository-owned shader.
Version 6 lacks the pouch; version 5 lacks
doom artwork; version 4 also lacks offerings;
version 3 also lacks the spark;
v2 also lacks the key; v1 also lacks bounty, ammo and charge
artwork. Older collections remain available for diagnostic inspection but cannot
admit play. Old Keyring's counter is named **Keys**.
Location roles explicitly distinguish unrevealed and revealed
faces. Preparation preserves source bytes and the archive; an identical complete
destination is reusable, while conflicting or incomplete output is refused.
Runtime loads only verified files from the prepared collection. Copy its whole
folder, including `manifest.json`, to relocate it; ordinary play needs no archive.

For an update, prepare and verify a fresh staging folder, package it, then import
the ZIP outside play. The importer owns persistent installation and activation;
development staging folders must not become ordinary launch targets. Rebuilding
the executable alone does not install external artwork. The one-time original-copy
consolidation in [#140](https://github.com/joeloverbeck/arkham-unfolded/issues/140)
requires its exact inventory, a dated Documents backup, a full fresh restore and
ordinary managed admission before separately authorized deletion. Source/authoring
inputs, personal saves, settings and enclosing evidence directories are excluded.

Before handing off an artwork change, check the collection selected by the actual
launch configuration, under the same user, working directory and HOME/XDG environment:

```sh
scripts/dev cargo run --locked --bin arkham-assets -- check-gathering-launch
```

If the launch includes `--art-root PATH`, pass that same option and path to this
check. It shares Gathering's saved-preference/default and relative-path resolution,
prints the selected absolute root, and reports configuration warnings and asset
completeness without opening a window or changing settings, saves or artwork.
An incomplete collection exits nonzero; the removed `--no-art` option is rejected. Native startup also refuses incomplete collections and requires successful decoding before play. Retain the resolved root and result in the handoff;
`check-gathering PATH` remains the check for an explicitly named staging collection.

`--art-root PATH` selects a folder for that launch. **Choose asset folder** validates and saves
the preference; startup recovery uses it immediately with **Retry**. It is stored in `$XDG_CONFIG_HOME/arkham-unfolded/gathering.toml`
(normally `~/.config/arkham-unfolded/gathering.toml`). Without an override or saved
preference, Gathering retains its legacy `gathering-assets` default. Remove obsolete
`--art-root` arguments from saved launch commands. An explicit saved choice still
takes precedence; it is not silently rewritten. Other experiments' settings and
distinct finite content selections remain separate. With a custom `XDG_DATA_HOME`,
install in that data directory or explicitly select the installed folder.
Startup reports the chosen folder and verified coverage. Absent, corrupt or undecodable required roles block play and offer **Import artwork**, **Choose asset folder**, **Retry** and **Quit**. A valid recovery import or selection applies to this startup; in-game folder selection still applies on restart. The equivalent engineering import is `arkham-assets import-gathering /absolute/path/to/package.zip`; it uses the same verification and persistence path and does not bypass native admission.

Historical installation evidence: on 2026-11-05 Joel confirmed that the compact enemy board looked good, then found
missing bounty art when launching the old cache collection. Direct inspection
found 140/143 roles there and all 143 in the newer prepared collection. He
authorized consolidation into one installed collection and cleanup of the two
obsolete Gathering copies. This local follow-up changes the default root and
handoff, not artwork identities, save paths or gameplay. The return point is the
verified installation, rebuilt app and required checks within the ordinary
`OPS-4` bound; it authorizes no new desktop session or remote publication.
Consulted: `PURP-1`–`6`, `WORK-1`–`6`, `AGENT-5`, `OPS-1`–`7`, `AUTH-1`–`3`,
`PUB-1`, `EVID-3` and `EVID-4`.

The consolidation completed locally: the installed collection verifies all 143
roles and its manifest SHA-256 is
`72c88d571603eaeca435814f24d22c2886ba3ad816f5e5a63da72f06ecc1d365`.
Its payloads matched the complete prepared copy before cleanup; the older copy
differed only in its manifest and the three missing token files. Both obsolete
Gathering cache collections were removed after verification. Source artwork,
personal saves, settings, native captures and other experiments' collections
were not changed. No linked-directory redirects were installed: the loader's
existing integrity checks still require a real collection directory.

The retained `run.json` records under `/tmp/gathering-art-consolidation/` are
`full-check`, `build`, `cleanup` and `final-installed-check`; all finished with
exit 0. The full gate includes 430 passing library tests (one ignored), including
the default-folder and explicit-preference test. The rebuilt tabletop executable
SHA-256 is `b6139fc2b3a2b7730585dd72ad31ae135a0dca2d0ededf1cac00a10354628b2c`.
This verifies installation, default selection and build checks without claiming
a further native visual session. Launch without the obsolete `--art-root`
argument to consume the installed collection.

The driver prints the current observation and legal choices. Send one JSON
request per line. At the initial decision, for example:

```json
{"op":"choose","revision":0,"id":"lead:tony"}
```

Each response supplies the current `revision` and choice identifiers. Use those
values for subsequent decisions; stale revisions and unavailable choices are
refused. Opening hands use the full shuffled decks. Select or unselect individual
cards using their offered mulligan choices, then choose `confirm-mulligan` with
the current revision. Confirming with no selected cards keeps the hand.

Other requests:

```json
{"op":"observe"}
{"op":"save"}
{"op":"reopen"}
{"op":"quit"}
```

Choices save before they change the live session. A refused choice or failed
save leaves the live state and randomness unchanged. Reopening retains pending
mulligan selections without drawing again. Saves have their own Gathering kind
and source revision, so existing experimental records are not reinterpreted.

## Implemented boundary

Setup binds Tony's Internal Injury, Patrice's The 13th Vision and Jacqueline's
Chronophobia. It applies the selected permanents, Tony's six initial bounties,
Jacqueline's two mental trauma and starting horror, and five resources each.
Opening draws and mulligans replace weaknesses without resolving them, holding
them aside until the opening-hand step finishes.

After mulligans, the scenario places Study, the act and agenda decks and the six
set-aside cards, and shuffles the exact 26-card encounter multiset. The source
inventory is 40 physical cards: the other 14 have explicit destinations. Counts
and public inventory summaries do not reveal hidden draw order.

Ordinary setup initializes the runtime from the actual prepared hands, decks,
locations and randomness. Its scenario introduction and advancement are shown live and retained in action history. The issue's
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
owns the exact implementation status and evidence.

Named hostile fixtures remain explicitly marked and rejected by the ordinary
driver and native launcher. They exercise difficult selected-card cases alongside
the ordinary preparation and round-flow checks. Their passing tests do not
establish native gameplay acceptance or prove every possible card composition.
The [Premonition follow-up](../research/issue-89-premonition-followup.md) and
[nested-group follow-up](../research/issue-89-nested-tokens-followup.md) replace the
earlier refusal boundaries with finite source-derived interpretations and witnesses.
They do not claim newly obtained official rulings for the exact combinations.

The current runtime increment adds Backpack's searched attachments, Ethereal Form,
Peter Sylvestre's end-of-turn healing and the documented Eyes of the Dreamer / Olive
composition. Safe Undo is offered until a new hidden-information or randomness
boundary; Undo remains in chronological history. The original
encounter assembly is unchanged: independent inventory checks account for every
physical copy and its setup destination, rather than trusting set names alone.

Transient presentation steps show automatic token reveals, test results, draws,
harm and scenario transitions before cleanup. The native adapter consumes public
projections for locations, enemies, assets, threats, commitments, pending costs
and token custody. Live consequences advance after their visible motion, retaining
brief holds for revealed tokens and pending harm. Completed skill tests wait at
the result panel; genuine rules decisions and newly revealed story text remain
explicit. Inspection and blocking dialogs suspend live animation while open.
Camera navigation does not pause it; Follow remains optional.

Each rules decision is saved before its presentation begins. Reopening the same
unchanged session in the same window preserves unfinished presentation. A fresh
process opens at the saved pending choice, restoring result and story reading
from explicit current facts. Animation queues and visual cursors are not saved.
The measured 250-choice terminal path is below 256 KiB in the new format; this is
a measured path, not an upper bound for every possible game.
Headless production-system checks cover these adapter behaviors; physical input,
readability, artwork and owner acceptance still require the actual window.

On 2026-11-02, a five-minute authorized native check of `fbf82d3` opened the
ordinary setup window at 3200 × 2000 with readable substitutes and healthy
foreground rendering. The helper initially refused Gathering's exact window
title; its title allowlist was corrected while preserving PID, process-start,
focus and geometry guards. The subsequent calibrated portal click landed inside
the visible lead-choice border, but the save remained at revision 0 and the
capture did not change. The pointer grant closed at 21:02:59 UTC and the isolated
launch was closed by 21:03:04, before the 21:03:42 deadline. The planned first
action and replay route was therefore **not verified**. The owner independently
reproduced the inert physical click and reported `No tab groups found` on Tab.
The original headless pointer tests missed continuous graphics-health updates:
the native monitor changes its sampling resource every frame, causing Gathering
to despawn the hovered controls before activation. A regression test reproduced
revision 0 instead of 1 with those updates; a separate test reproduced the exact
Tab navigation error after graphics admission.

The renderer now rebuilds for changes to graphics admission or its displayed
status, while preserving controls across ordinary sampling updates. The toolbar
and decision panel now register ordered Tab groups. The previously failing
tests pass, including raw mouse events at scales 1 and 2 with same-frame and
separate-frame press/release. These checks establish the reproduced regression
repair. The subsequent owner-authorized native retest of `0d5310b` passed the
bounded route: lead choice, all three mulligans, ordinary scenario entry,
Patrice gaining one resource (5 to 6 resources; 3 to 2 actions), replay from
moment 1 of 8 to 2 of 8, and return to the live table. The isolated save ended at
revision 6, stage `playing`. The pointer grant closed at 21:34:04 UTC and the
isolated process was confirmed absent by 21:34:17, before the 21:36:14 deadline.
The supervisor was explicitly interrupted for cleanup, so its exit 130 is not
normal-exit evidence. Retest receipts and captures are under
`~/.cache/agent-tmp/arkham-89-native-0d5310b/`.

This establishes the observed native pointer route, not a full scenario
playthrough, physical keyboard verification or owner acceptance. Captures also
show overlapping card labels, missing separator glyphs and right-edge clipping;
visual readability remains unfinished.
Captures and event receipts from the failed check remain local under
`~/.cache/agent-tmp/arkham-89-native-fbf82d3/`.

## Accepted camera input and restart design

Joel accepted all three recommended decisions (Q1–Q3) in the 2026-11-03
`grill-with-docs` interview against `54c8d48cf9227dc862c1c7fcaaea0dfd7051a2b8`,
in response to his inability to test Gathering comfortably. The design is
settled; Joel subsequently instructed, "Proceed with the implementation."
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89)
and its [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
remain the work-control authority. The earlier full-scenario allocations remain
exhausted; this activation selects the accepted camera/restart repair, not
full-scenario completion. Local implementation and automated verification began
at 13:48 UTC on 2026-11-03 on `fix/gathering-camera-restart`, with the repository's
default two-hour boundary (15:48 UTC) and earlier scope/non-convergence stops.
The result is one reviewable candidate that lets Joel navigate and restart his
Gathering tests. The agreed seams are production keyboard/render registration
and session save/load/replay. Native desktop use requires a fresh agreed interval;
publication, personal-save mutation and broader gameplay acceptance are separate.

Joel reports that WASD fails on load and after closing Inspect until he clicks
the table. After playing Azure Flame, each subsequent pan moves briefly before
a button takes focus again. He also needs an in-app way to start Gathering over.

Source review at the starting revision supplied this explanation, without a
native reproduction:

- Gathering's [camera adapter](../../src/gathering_tabletop.rs) treats every
  focused button as keyboard capture. Its [ordinary controls](../../src/gathering_tabletop/ui.rs)
  automatically focus the first global choice, or Overview when none exists.
  Closing Inspect rebuilds those controls.
- During a post-choice presentation, each manual camera step marks the table
  dirty, including steps after the presentation is already paused. Rebuilding
  the controls can restore button focus and block the next step. This predicts
  the reported click/pan/refocus loop without requiring an Azure Flame rules
  defect. The source fallback is Overview; the exact runtime focus targets
  reported by Joel still need reproduction.
- Party lacks those ordinary-button autofocus assignments, although it also
  blocks camera keys when a button is focused. Its accepted behavior is a
  comparison, not proof that it already satisfies the newly accepted policy.
- Gathering has no restart action. A new save filename creates a fresh setup.
  Party already preserves the old attempt through an explicit restart, using
  the shared [record replacement](../../src/record_replacement.rs) operation.

The accepted decisions are:

1. **Q1 — Camera input:** ordinary button focus and open
   card-action lists leave unmodified WASD available. Holding a key pans
   continuously, including after load, inspection and playing an asset;
   releasing it stops movement. Beginning manual navigation dismisses a
   transient action list without choosing an action. Preserve Tab/Shift+Tab,
   Enter/Space and Escape controls; arrows currently have no menu-navigation
   binding. Blocking Inspect, history, main Menu, story-reading, confirmation
   and folder-selection dialogs capture camera input explicitly. Closing them
   restores WASD without a background click. Joel selected this over keeping
   card-action lists blocking WASD.
   [ADR 0002](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#keyboard-focus-and-camera-input)
   owns the refined camera-input contract;
   the glossary's camera focus continues to mean the area framed for viewing.
2. **Q2 — Restart meaning:** add **Restart The Gathering…** in the live
   Menu during setup, play and after resolution. It starts an independent
   attempt with fresh randomness, the same selected decks, weaknesses and
   pinned rules, returning to lead selection and opening hands/mulligans.
   Artwork and motion preferences persist. Joel selected fresh shuffles over
   repeating the original deal. Confirm with Restart/Cancel, defaulting to
   Cancel; Escape cancels. Replay stays read-only; return to live play before
   restarting.
3. **Q3 — Previous attempt:** preserve its exact saved record in
   a unique archive before replacing the active save. The new attempt uses
   the same active path, so **Reopen saved attempt** reopens the new attempt.
   Archive recovery uses its launch path, which must be discoverable after
   restart. Joel selected this over adding in-app recovery or discarding the
   previous attempt. Cancellation preserves the current attempt. A stale save,
   busy file or failed replacement leaves the current live attempt and active save intact;
   the shared operation may already have left an archive or rotated recovery
   file, so this is not a claim of multi-file transactionality.

Implementation must add regressions through production keyboard and
render registration: continuous WASD across several frames and rebuilds at
startup, after Inspect, with ordinary focus, and after playing Azure Flame
during active and paused presentation; modal capture and release; and keyboard
activation without accidental game choices. Restart checks must cover cancel,
fresh setup, save refusal, reopening the new attempt and recovery of the exact
archived record. A bounded native run must then exercise Joel's actual
sequence; static review and existing camera-action
tests do not establish that physical input is repaired. This design covers
these testing obstacles, not complete-scenario correctness or acceptance.

The local implementation separates WASD capture from other keyboard capture and
pauses a presentation only on the first manual-navigation frame. Before the fix,
the registered input test reproduced no movement from the initially focused
button. After enabling pan, the Azure Flame regression separately reproduced Tab
focus being reset on every held-key frame; the one-time pause fixes that rebuild.
The shared camera's other consumers retain their previous capture behavior.

Independent review found that dismissing the reveal chooser on every manual
frame also rebuilt its ordinary Reveal button. A focused-control input regression
reproduced that stale focus; dismissal now runs only while the chooser is open.
The regression passes and retains Enter activation after panning, plus dismissal
of the open chooser without revealing a token. Its initial focus is fixture state;
the subsequent keyboard input and activation use production registration.

Restart uses the existing locked replacement operation, returning its unique
archive path. The live table resets only after replacement succeeds; confirmation
and choice callbacks are bound to the table generation. Tests exercise the actual
Menu/confirmation registration, cancellation, modal pan capture/release,
duplicate/stale callbacks, replay and graphics refusal, preference retention,
fresh setup and reopening. Continuity checks recover exact original bytes and
replay after later autosaves and repeated restarts, and cover stale/missing/busy
records and invalid recovery files.

The required `scripts/check` finished with exit 0 at
`/tmp/arkham-controls-final-check/` on 2026-11-03: 395 library tests passed,
one existing test ignored, and the integration/tool suites passed. The initial
sandboxed run at `/tmp/arkham-controls-full-check/` failed because private D-Bus
test sockets could not bind; the host-context run resolved that environmental
restriction. The earlier normal tabletop build finished with exit 0 at
`/tmp/arkham-controls-final-build/`; its executable SHA256 is
`d27668f7670a33cf6cfc6964ea77a1da96c758c08185ed73021a267e87fb3fd0`.
Each directory retains `check.log` and terminal `run.json`. Independent Standards
and Spec review found no remaining findings after the Reveal-control correction.
After Joel approved development resource protection, the full check passed again
at `/tmp/arkham-build-policy-check-v2/`, followed by the normal build at
`/tmp/arkham-build-policy-native-build/`. Both terminal records have exit 0.
The executable used for the native check below has SHA256
`0daf617a8976ca6eeba6329c75bb85cee7ef8dcca53202fab0d8fddcb516af2c`.
The resource changes preserve the reviewed gameplay source and use the
[documented build limits](../../README.md#current-use-and-external-inputs).

On 2026-11-03, Joel authorized a five-minute native interval from 16:00:51 to
16:05:51 UTC. The isolated Gathering window passed graphics admission with all
140 artwork assets verified. Joel reported that immediate WASD moved continuously
and stopped on release, then that all listed checks passed: Inspect blocked
panning and Escape restored it without a table click; panning remained continuous
after playing Azure Flame; restart cancellation, fresh setup and reopening the
new attempt worked. These physical-input results are owner testimony. The first
capture separately shows Azure Flame with four charges and Patrice with two
resources and two actions. A later capture shows card inspection after restarting
and continuing into play. Neither static capture proves continuous keyboard input.

The window closed normally at 16:05:25 UTC with exit 0, before the cleanup reserve;
the process was confirmed absent. Evidence is retained under
`~/.cache/agent-tmp/arkham-controls-native-20261103/`: `run.json`, `app.log`,
`current-state.png`, `after-restart.png` and the isolated save/archive. The archive
matches the previous attempt byte for byte; both it and the restarted attempt
load, reopen and initialize nine-moment replays through the production driver
using separate validation destinations. Their save round trips preserve exact
bytes. The new attempt has a different random seed and retains the same preparation
source/deck definitions and excluded weaknesses. Joel continued into play after
restart, so the final new save is beyond lead selection. Original personal saves
and the launch-source copy are unchanged. This accepts the reported camera/restart
route, not a complete scenario playthrough or broader presentation acceptance.

Consulted rules: `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`,
`EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `AGENT-1`–`AGENT-5`,
`OPS-1`–`OPS-7`, `PUB-1` and `GOV-1`–`GOV-3`.

## Record compatibility

All saves and standalone recordings made before replay retirement are unsupported. This includes setup-v1 and runtime-v2/v3/v4 records. No migration or historical reader is maintained. The loader refuses them without changing the file; start a new attempt at a different `--save` path.

Until Joel accepts a save-compatibility commitment, development builds promise none: a save loads only when the current build reproduces its recorded game exactly, and is otherwise refused unchanged so that a New attempt can begin. No migration, alias or legacy path is kept for records from earlier builds; a build that changes a decision step can therefore refuse version-3 saves made before it.

The new format (version 3) stores current rules/RNG state, action history, completed test explanations, explicit story knowledge and the compact decision journal used for validation and safe Undo. It stores neither historical observations nor live animation queues. Current-session archives and `.previous` recovery files use this same format. Mode, source, application and checksum checks precede loading; save replacement also checks the exact loaded/saved baseline under the file lock.
