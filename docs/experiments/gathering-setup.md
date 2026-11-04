# Gathering candidate

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
layout refinement](../adr/0002-persistent-table-areas.md#gathering-setup-visibility-and-automatic-live-presentation)
on 2026-11-03, including a victory-pile caption with its card count and summed
Victory points, and then authorized implementation. The local candidate now
shows the initial Scenario and Test arrangement before lead selection and
through mulligans, including after restart. This is a projection of public setup
membership; actual deck assembly, random consumption, legal choices and records
retain their original order. Each physical Arcane Research has its own caption.

Ordinary live consequences finish automatically, including while panning or
changing focus. Inspection and blocking dialogs suspend them only while open;
story reading still requires Continue. Manual event stepping belongs to Replay.
The pool and bag share one row. Scenario has matched heading spacing, adjacent
encounter piles at upper left, a victory pile at upper right, a book nearer the
far edge, and a bounded Map region. The Victory display shows its card count and
summed points, excluding resolution bonuses. Eligible locations join that displayed
pile at scenario end; the adapter projects this from older terminal observations
without rewriting their records or awards.

The [research return](../research/issue-89-rules-investigation.md) supplies the
source-derived behavior. This implementation pins that research baseline and
taboo 10. On 2026-11-02 the accepted ArkhamDB taboo source was refreshed and matched
the retained bytes exactly (`d63d719bd025040fd74281c123cc692ce521f305e78d5588e7c83eed31801673`):
latest ID 10, effective 2026-02-19. The publisher resource page still lists Legacy
FAQ 2.5 and Grimoire 1.1; the existing original-content precedence remains applicable.
This dated check is not a guarantee about a future campaign's latest version.
The original deck revisions and explicit weakness selections are
visible in the observation. Original ArkhamDB snapshots remain unchanged.

## Skill test feedback verification

The 2026-11-04 correction is local on `fix/gathering-test-feedback`, based on
`8852b2937e748c5260111ee573ec7ab24419eccf`. It implements the accepted
[result-reading and six-defect scope](../adr/0002-persistent-table-areas.md#gathering-presentation-and-skill-test-feedback).

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
under the [desktop-interval procedure](jacqueline-curse.md#launch-capture-and-interact).
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

Records from previous setup-v1, runtime-v2 and runtime-v3 checkpoints use the preserved readers described
under [Record compatibility](#record-compatibility), rather than this build.

The native table uses the same session and save file:

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop \
  --experiment gathering --save /tmp/arkham-gathering-setup.session.json \
  --art-root /home/joeloverbeck/.cache/agent-tmp/arkham-89-art-menu/gathering-assets-v1
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
Replay limits those images and texts to the moment being viewed.

The portrait rail and Scenario, Test and Overview buttons retain Party's camera
controls. The **TURN** badge follows the actual actor, not camera focus.
**Menu** at the upper right contains Save, Reopen, Restart, History, Replay, reduced motion
and artwork settings. Save, Reopen, History and Replay operate on the full session; replay permits
inspection but no live choices. Menus use Tab/Shift+Tab and Enter/Space; Esc
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
Replay displays recorded results and keeps its navigation controls. Older
records keep their known totals but label missing detail as unavailable; a test
already in progress in an older save finishes without a partial explanation.
Tests begun after resuming capture full detail.

The same correction uses a supported Restart label, clearance around confirmation
button focus outlines, rounded physical cards and inspection images, and the
prepared font for recognized bracketed card-text symbols. The original artwork
and absent-font English remain intact. The accepted design and scope are in
[ADR 0002](../adr/0002-persistent-table-areas.md#gathering-presentation-and-skill-test-feedback).
The [targeted native check](#native-feedback-check-2026-11-04) covers the exercised
reading, choice and inspection paths. Its remaining checks and complete player
journey are distinct from owner acceptance.

### Restart and recover an attempt

Choose **Menu → Restart The Gathering...** in the live table, then **Restart** to
return to lead selection, opening hands and mulligans with fresh shuffles. The
selected investigators, decks, weaknesses and pinned rules stay the same, as do
artwork and motion preferences. Cancel is initially focused; Enter activates
Cancel, and Escape also cancels. Return from replay to the live table first.

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
  --experiment gathering --save "$gathering_recovery_dir/session.json" \
  --art-root /home/joeloverbeck/.cache/agent-tmp/arkham-89-art-menu/gathering-assets-v1
```

This preserves the archive while the recovered copy receives autosaves. **Open
recorded replay** then shows the recovered attempt's history. Restart refuses a
missing, stale, busy or invalid save/recovery file and reports the reason inside
the confirmation. Cancel and resolve that reason before retrying; the current
live attempt and active save are retained.
Headless tests exercise the actual menu controls and save/replay protections.

For a static layout capture, use the [isolated native diagnostics](jacqueline-curse.md#launch-capture-and-interact)
with `--experiment gathering --initial-view scenario --screenshot scenario.png --quit-after-screenshot`
and the prepared artwork root above. The capture waits
for graphics readiness and exits after writing the image. This checks rendering;
it does not verify camera-button interaction or gameplay. Agree the desktop
interval before an agent launch.

Two full-scenario additions are prepared for owner review. Several unengaged
enemies occupy a small local pile beside their room; clicking the room or any
enemy offers every enemy's inspections and legal actions. Engaged enemies use
the investigator's threat area. An exhausted enemy in a narrow room pile is
identified in text instead of rotating into the neighboring location.

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

The README uses the verified collection already prepared on this machine. To
prepare another portable collection from the original local archive, first use
the [existing reveal-icon download](../../README.md#prepare-the-tony-artwork), then:

```sh
scripts/dev cargo run --locked --bin arkham-assets -- prepare-gathering \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /tmp/arkham-reveal-icon.png \
  --output "$HOME/.local/share/arkham-unfolded/gathering-assets-v1"
scripts/dev cargo run --locked --bin arkham-assets -- check-gathering \
  "$HOME/.local/share/arkham-unfolded/gathering-assets-v1"
```

The finite recipe contains 140 roles: 94 card identities, 11 reverse/unrevealed
sides, card backs, investigator standees, token/class artwork, reveal icon and
symbol font. Location roles explicitly distinguish unrevealed and revealed
faces. Preparation preserves source bytes and the archive; an identical complete
destination is reusable, while conflicting or incomplete output is refused.
Runtime loads only verified files from the prepared collection. Copy its whole
folder, including `manifest.json`, to relocate it; ordinary play needs no archive.

`--art-root PATH` selects a folder for that launch. **Choose asset folder** saves
the next-launch preference in `$XDG_CONFIG_HOME/arkham-unfolded/gathering.toml`
(normally `~/.config/arkham-unfolded/gathering.toml`). Without an override or saved
preference, Gathering uses `$XDG_DATA_HOME/arkham-unfolded/gathering-assets-v1`
(normally `~/.local/share/arkham-unfolded/gathering-assets-v1`). Other experiments'
settings remain separate. `--no-art` is available for diagnostic substitute mode.
Startup reports the chosen folder and verified coverage; absent or corrupt roles
use readable substitutes rather than an unchecked image.

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
{"op":"replay"}
{"op":"next"}
{"op":"back"}
{"op":"live"}
{"op":"quit"}
```

Choices save before they change the live session. A refused choice or failed
save leaves the live state and randomness unchanged. Reopening retains pending
mulligan selections; replay shows recorded observations without drawing again.
Replay does not accept live game choices. Records have their own Gathering kind
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
locations and randomness. Its scenario introduction and advancement moments are
recorded. The issue's
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
boundary; Undo remains in chronological history and replay. The original
encounter assembly is unchanged: independent inventory checks account for every
physical copy and its setup destination, rather than trusting set names alone.

Recorded presentation moments retain automatic token reveals, test results,
draws, harm and scenario transitions before cleanup. Replay can inspect these
moments without recomputing outcomes. The native adapter consumes the same public
projections for locations, enemies, assets, threats, commitments, pending costs
and token custody. Live consequences advance after their visible motion, retaining
brief holds for revealed tokens and pending harm. Each moment is rendered before
advancement; routine captions do not impose a reading-length delay or require
acknowledgment. Completed skill tests wait at the result panel described above;
genuine rules decisions and newly revealed story text remain explicit.
Inspection and blocking dialogs suspend playback and card movement only
while open. Camera navigation does not pause playback. Follow is optional in live
play and resumes on replay navigation after a manual override.

Each rules decision is saved before its presentation begins. Reopening that
session in the same window preserves an unfinished presentation; a fresh process
opens at the saved pending choice, with the intervening moments retained in
Replay. The visual cursor is not a separately persisted game decision. Compact
JSON keeps the measured 250-choice defeat recording below the existing 8 MiB
limit. This is a measured path, not an upper bound for every possible game.
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
   [ADR 0002](../adr/0002-persistent-table-areas.md#keyboard-focus-and-camera-input)
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

New Gathering sessions and recordings identify
`c35be2c:issue-89-gathering-runtime-v4`. This identifies the revised token-composition
semantics; it does not claim a new rules or taboo update.
The envelope format and other experiments' readers are unchanged.

Historical records remain readable and resumable with their preserved source:

| Record source identity | Preserved reader commit |
| --- | --- |
| `c35be2c:issue-89-gathering-setup-v1` | `7b0113275a0ae1bdb57e7f24467e6d3ed6af68d3` |
| `c35be2c:issue-89-gathering-runtime-v2` | `f431de9af84bedf20e0024370b9e6aaa8819a1ef` |
| `c35be2c:issue-89-gathering-runtime-v3` | `46f4c54a018802d22889a238629bcea27871d259` |

This applies to **ordinary setup records as well as verification fixtures**. The
current ordinary path continues into gameplay; historical readers retain their
original setup boundary. The current reader refuses their source identity before
decoding the Session or Recording payload, names the required preserved reader,
and leaves the original bytes unchanged. No personal-save migration is performed.
Do not edit a record's version fields to bypass the check: runtime fields and
the exact replayed decision history differ.

From a repository clone containing the required commit, prepare its separate reader.
The example selects runtime-v3; use the matching commit from the table for older sources:

```sh
gathering_reader_commit=46f4c54a018802d22889a238629bcea27871d259
gathering_reader_dir=$(mktemp -d /tmp/arkham-gathering-reader.XXXXXX)
git archive "$gathering_reader_commit" | tar -x -C "$gathering_reader_dir"
cargo build --locked --manifest-path "$gathering_reader_dir/Cargo.toml" \
  --target-dir "$gathering_reader_dir/target" --bin arkham-driver
```

Build dependencies must already be cached for an offline build; add `--offline`
in that case. The separate target directory keeps the preserved reader's build
artifacts apart from the current application. The source commit, rather than the
temporary checkout, is the durable recovery point.

Copy an ordinary session before resuming it, replacing the example original path:

```sh
gathering_resume_dir=$(mktemp -d /tmp/arkham-gathering-resume.XXXXXX)
cp -- /absolute/path/original.session.json "$gathering_resume_dir/session.json"
"$gathering_reader_dir/target/debug/arkham-driver" --experiment gathering \
  --load "$gathering_resume_dir/session.json" --save "$gathering_resume_dir/session.json"
```

The driver prints the exact pending observation. Use its offered revision and
choice ID to continue. For recorded replay, send `{"op":"replay"}`, then
`{"op":"next"}` or `{"op":"back"}`; `{"op":"live"}` returns to the saved
live state and `{"op":"quit"}` exits. A completed historical setup record stays at its
original unavailable-gameplay boundary. It does not become a new playable run.

For native ordinary setup resume, build and run the same preserved source with
`--features tabletop --bin arkham-tabletop`, passing the copied save:

```sh
cargo run --locked --manifest-path "$gathering_reader_dir/Cargo.toml" \
  --target-dir "$gathering_reader_dir/target" --features tabletop --bin arkham-tabletop -- \
  --experiment gathering --save "$gathering_resume_dir/session.json" --no-art
```

The native Replay controls replay that session. Verification fixtures remain
excluded from both ordinary launchers. For a standalone exported recording, or
a developer's preserved verification session, the following temporary Rust
example uses the same preserved continuity APIs. It does not create fixtures or
make them ordinary player sessions. Create it only in the archived reader:

```sh
mkdir -p "$gathering_reader_dir/examples"
cat > "$gathering_reader_dir/examples/preserved_record.rs" <<'RS'
use arkham_unfolded_experiment::gathering::{Recording, continuity::{
    Replay, load_recording, load_session, commit_choice,
}};
use std::{error::Error, io};
fn replay(recording: Recording) -> Result<(), Box<dyn Error>> {
    let mut replay = Replay::new(recording)?;
    loop {
        println!("{}", serde_json::to_string_pretty(replay.current())?);
        eprintln!("next | back | quit");
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 { break; }
        match line.trim() {
            "next" => { replay.next(); }, "back" => { replay.back(); },
            "quit" => break, _ => eprintln!("Use next, back or quit."),
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let (kind, path) = (args.get(1).ok_or("session|recording required")?,
                        args.get(2).ok_or("copied record path required")?);
    if kind == "recording" { return replay(load_recording(path)?); }
    if kind != "session" { return Err("Use session or recording".into()); }
    let mut session = load_session(path)?;
    loop {
        println!("{}", serde_json::to_string_pretty(&session.observe_for_play())?);
        eprintln!("choose REVISION ID | replay | quit");
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 { break; }
        let parts: Vec<_> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["choose", revision, id] => commit_choice(path, &mut session, revision.parse()?, id)?,
            ["replay"] => replay(session.recording())?, ["quit"] => break,
            _ => eprintln!("Use an offered revision/ID, replay or quit."),
        }
    }
    Ok(())
}
RS
cargo run --locked --manifest-path "$gathering_reader_dir/Cargo.toml" \
  --target-dir "$gathering_reader_dir/target" --example preserved_record -- \
  recording /absolute/path/original.recording.json
```

Use `session "$gathering_resume_dir/session.json"` instead of the final two
arguments for a copied verification session. Original record files and the
current application remain unchanged by this recovery procedure. Historical
producer evidence covers pending and completed ordinary setup, pending runtime
revelation and pending skill test: the preserved reader loaded/replayed all four
and resumed each nonterminal case. The new reader rejects all eight original
session/recording files with the explicit compatibility message. This establishes
the preserved-reader route, not a migration or native visual acceptance.

The runtime-v3 change repeats that compatibility probe against actual runtime-v2
source at `f431de9`: eight historical session/recording files cover the same four
positions. Its reader loaded and replayed all cases and resumed nonterminal
sessions; the new reader refused both loading and replacement without changing
any original bytes. These probes use temporary developer records, not personal
saves.

The runtime-v4 change repeats the same producer, load, replay and resume probe
against actual runtime-v3 source at `46f4c54`. All eight historical files remain
unchanged; the current reader refuses both loading and replacement and names that
preserved reader. Ordinary historical setup still stops at its original gate.
