# Jacqueline's token choice: running the experiment

**Retired 2026-11-07.** [ADR 0005](../adr/0005-retire-experiments-and-select-scenarios.md) removes this standalone experiment. Commands, source paths and observations below describe the [pre-retirement revision](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/docs/experiments/jacqueline-curse.md); they are historical evidence, not current launch instructions or continuing support obligations. Use [scenario selection](../../README.md#launch-and-resume-the-gathering) for current play and the [native-verification runbook](../agents/native-verification.md) for shared developer procedures.

**Artwork contract amendment, 2026-11-07:** [ADR 0004](../adr/0004-require-artwork-for-3d-play.md)
requires complete artwork for 3D play and a blocking startup recovery window.
All four modes now refuse incomplete collections and wait for every required
image and the symbol font to load before play. **Choose asset folder** selects a
complete collection, **Retry** rechecks it, and **Quit** leaves saves unchanged.
The removed `--no-art` option is rejected. Dated substitute-presentation evidence
below describes earlier builds. Headless checks remain artwork-independent.
[Issue #124's handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/124#issuecomment-6706875824)
records verification and remaining native acceptance.

Replay has been retired in all four modes. Saves from before this change are unsupported and remain untouched; use `--save` with a new path to start an attempt. New-format saves retain exact continuation, action history and applicable Undo. There is no recording export or historical reader. [Decision and scope](../adr/0003-retire-replay.md). Dated evidence below describes its original builds; references to replay in those historical reports are superseded.

[GitHub #2](https://github.com/joeloverbeck/arkham-unfolded/issues/2) owns this experiment and its acceptance. Joel selected it after the [assessment](../research/issue-1-assessment.md) at `ef905f5`. It is one scripted skill-test case, not a playable campaign or an adopted game engine.

The task is to decide whether Patrice commits Promise of Power, decide when Jacqueline intervenes, choose which tokens to cancel, and understand the resulting horror and token destinations. You can stop at any pending decision. Then reopen and continue the saved choice. The source expectations and four worked outcomes are in the [source assessment](../research/issue-1-source-reconciliation.md).

## Build and run

The local launch below uses the complete 15-file collection prepared for #124; use your own complete folder elsewhere or prepare one below. This is the earlier Jacqueline mode; the [README](../../README.md#launch-and-play-the-tony-turn) leads with the new Tony turn. Requires Rust/Cargo 1.97.1 or later and a working Ubuntu X11/XWayland graphics session with Vulkan support. The lockfile pins the experiment's dependencies, including Bevy 0.20.0. First dependency acquisition/build requires network access and can take several minutes; the resulting application and saving run offline.

Open Terminal (`Ctrl` + `Alt` + `T`), then run the following. Replace the checkout directory if yours is elsewhere. Keep the terminal open while the native window is running.

```sh
cd /home/joeloverbeck/src/arkham-unfolded
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop --art-root "$HOME/.cache/agent-tmp/arkham-124-jacqueline-assets"
```

The window is titled **Arkham Unfolded — Scripted Rules Experiment**. For development verification, run `scripts/check` from the same directory; checks are not a prerequisite to playing. For native rendering or interaction changes, also follow [developer native verification](#developer-native-verification).

The scenario surface holds the map. A separate Test table sits between it and the investigator row, with the chaos bag at upper right and distinct sections for committed cards, revealed tokens and canceled tokens. Each compact investigator surface is as wide as the scenario table, with the name above its upper edge, permanent thick harm counters above the upper-left investigator card, deck/discard footprints at upper right, and one centered hand row along the near edge. Muted markings separate the five-card threat band, central assets and shallow hand strip. This experiment represents horror, including Jacqueline's initial 2/9; unrepresented damage reads `n/a`. Unsupported cards and piles remain explicitly unrepresented. Use portrait buttons to focus an investigator, **Scenario** for the shared map, **Test** for the test table, or **Overview** for orientation. Focusing Patrice does not change Jacqueline's test or the pending actor shown in the decision display. This corrected local candidate does not establish owner acceptance or crowded full-game capacity.

In a fresh case, focus Patrice and left-click Promise of Power to open its contextual controls, then choose **Commit +4** with the willpower symbol after reading the curse consequence. Its four wild icons contribute +4 to this willpower test; card inspection retains their wild identity. If Patrice is elsewhere, she cannot contribute in this experiment and the empty commitment prompt is skipped, leaving Jacqueline's reaction available. Use **Reveal chaos token** at the lower left: while Jacqueline's reaction is available, it offers a normal reveal that declines her reaction for this reveal, or **Use Jacqueline · reveal 3 tokens**. Opening or dismissing that chooser does not reveal or spend the reaction. Choose the reaction to reveal three and focus **Test**, then select a token to open the legal cancellation combinations, choose the complete set and click **Confirm cancellation**. Token numbers distinguish the exact revealed pieces. Selecting a physical card or token alone does not commit, reveal or cancel anything. If a curse calls for another reveal, use the shortcut when a new choice is offered; automatic continuations need no extra click. After completion, click **Show test result** when offered; other completed tests open the result automatically. Use **Return to table** to inspect the resulting position. Choose a card's **Inspect** control for enlarged artwork and readable rules; **Close card** or `Esc` dismisses inspection. Decisions save to `local/experiment/curse.session.json`; relaunching resumes it. Use **Save live session** to retain the initial state, or **Start another example** after a completed/ended case to archive it and begin again.

When only one reveal route remains, the shortcut submits it directly. After a successful saved choice it focuses Test; failed saves and stale clicks leave the game and camera unchanged. The shortcut appears only for legal reveals, and respects blocking panels and reduced motion. The physical bag and Jacqueline's existing contextual controls remain available with their existing camera behavior. A card or component whose sole operation is Inspect opens inspection directly; multiple Inspect targets retain a chooser, and a lone gameplay action never executes automatically.

The encounter discard begins empty. When Rotting Remains' revelation finishes, including a successful test with no horror, the card moves face up to that pile in the scenario area. Return to the table after the result to see its count and click the one-card pile for direct inspection. Ending an unfinished example does not resolve or discard the treachery. Save/reopen and player observations show the corresponding destination. Encounter-deck and victory-display contents remain unrepresented.

For the other prepared case, use the same complete collection:

```sh
target/debug/arkham-tabletop --case autofail --art-root /absolute/path/to/complete-jacqueline-assets
```

The normal asset root remains `$XDG_DATA_HOME/arkham-unfolded/assets-v2`, normally `~/.local/share/arkham-unfolded/assets-v2`, or the saved per-user preference in `$XDG_CONFIG_HOME/arkham-unfolded/config.toml`. The commands below prepare a fresh 15-file v3 collection including The Gathering's Easy / Standard reference; select it explicitly with `--art-root` or **Choose asset folder**. Earlier v1/v2 collections remain intact for inspection but cannot admit play. Tony uses a separate `tony.toml` preference. Startup reports the experiment, resolved folder and completeness. **Choose asset folder** opens Ubuntu's `zenity` folder dialog, checks the collection, and offers to save it. In startup recovery, Retry uses the chosen folder immediately; an in-game selection applies after restart. Canceling or a write failure preserves the previous setting. Incomplete, outdated and unrecognized collections are rejected.

```sh
curl --fail --location \
  'https://game-icons.net/icons/ffffff/transparent/1x1/lorc/swap-bag.png' \
  --output /tmp/arkham-reveal-icon.png
scripts/dev cargo run --locked --bin arkham-assets -- prepare \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /tmp/arkham-reveal-icon.png \
  --output "$HOME/.local/share/arkham-unfolded/assets-v3"
scripts/dev cargo run --locked --bin arkham-assets -- check "$HOME/.local/share/arkham-unfolded/assets-v3"
```

`--art-root PATH` selects another complete prepared collection for that launch without rewriting the preference. The source archive is never a runtime fallback. Copying the whole prepared folder, including `manifest.json`, is sufficient to relocate the selected presentation assets. Personal saves and settings stay separate. Missing or changed required files block play; original image/font bytes stay outside Git. A card's presence or absence never changes legal choices, rules outcomes or record compatibility. The [evidence manifest](../research/issue-1-evidence.json) preserves the source identities for the three original faces.

The reveal icon is [Swap bag by Lorc](https://game-icons.net/1x1/lorc/swap-bag.html), licensed [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). Preparation checks the unchanged 5,829-byte PNG against SHA-256 `54673d4cc97a84c90cab1d0a38b46183eb182c567493de676d7124ff3d70fe3e`, copies it to `ui/reveal-chaos-token.png`, and preserves attribution in the recipe and manifest. The application may tint its white silhouette and never needs the download location during play. The earlier Jacqueline v1 collection remains recognized: its 13 files can be inspected, but the incomplete collection cannot admit play. Its check reports incomplete new requirements without modifying anything. Saved folder preferences remain unchanged; choose the fresh folder to use the new image.

The five revealed token faces preserve the archive's consistent 1024-pixel circular set originally selected under `sce-base-save`, with source identities and hashes in [token-art.json](token-art.json). Their prepared copies live under `tokens/chaos/`. English face and status labels remain visible beside the graphics. Missing or changed required token images block play. Alternate designs were found under `lovecrafter_3077` and `rise_rapture_rise`, including smaller square renditions; the selected circular set is pinned for this experiment. Archive categories describe provenance and do not certify publisher artwork.

Card inspection and results use the prepared Arkham font for wild, curse, willpower and horror symbols; the test summary shows willpower on both the current value and difficulty. [symbol-art.json](symbol-art.json) preserves the exact source bundle, glyph mappings and extraction evidence. The preparation command extracts that same verified font once into `fonts/arkham-symbols.ttf`; ordinary play loads the font directly and does not decode Unity data. The font must decode and register before play; accessible labels retain the plain-English meaning.

Tony, Patrice and Jacqueline use their original miniature-card portraits as upright standees with a rounded base and rectangular holder. The [shared renderer](../../src/tabletop_standees.rs) is the standard for both experiments and subsequent scenario presentation; its class colors match the investigator surfaces. Both sides show the color portrait without mirroring; camera position does not imply a state change. [investigator-art.json](investigator-art.json) records the selected images, pixel-art alternatives and model search. No investigator-specific sculpt or UV mapping was found among the catalogued model resources; Unity geometry was not decoded. All required portraits must load before play.

[The finite preparation recipe](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/resources/asset-selection.json) binds these selected source files to their application-relative destinations and expected hashes. Card faces live under `cards/<card-code>/`, portraits under `investigators/<investigator-code>/`, and the reaction marker under `markers/`. Preparation and `arkham-assets check` verify the complete selection. If an existing output is incomplete or differs, prepare a fresh output and select it; the command never repairs a collection by overwriting unknown contents. The same folder/check/save/restart path is used for an update. Prior collections are not automatically deleted.

Jacqueline's investigator card is displayed in landscape on the table and in inspection by rotating its unchanged source image counterclockwise. Inspection enlarges the original art alongside readable rules; on smaller windows its content scrolls while the Close control stays visible. Close card or Escape returns to the table.

Promise of Power gains a highlighted frame when committed. A small labeled reminder to the right of Jacqueline's card flips between available and used, following her once-per-round reaction in live play. It uses the archive's Universal Action / Ability Token as its visual base; it is an application reminder, and does not exhaust Jacqueline's investigator card. Canceled tokens move to a separate group with faded faces and visible crosses, while resolved tokens retain their faces.

Component actions appear beside the selected card or token; choices such as declining an optional effect remain in the decision controls. `Esc` or clicking empty space closes contextual controls without a game action. Hold unmodified `W`/`A`/`S`/`D` to pan forward/left/back/right across the table relative to the camera, preserving tilt and zoom; release to stop. Right-drag or arrow keys orbit; the wheel or `+`/`-` zooms; `Home` returns to the overview. Camera keys are suspended while a menu or modal captures keyboard input. Manual camera movement overrides automatic following and closes the contextual menu. Replay's Next and Previous controls pause on recorded moments; cards remain inspectable, but game actions require returning to live play. Explain opens the action history. F12 captures the window under `local/experiment/`; use the independent capture procedure below for development verification.

A completed test is saved immediately. When a curse resolves, the table stays visible to show its additional token reveal, with the explanation **Curse: −2. Reveal another token.** Choose **Show test result** when ready; the same pacing applies when reopening a completed curse test. Other completed tests open the centered result window automatically. The window shows the outcome and skill values, horror and recorded explanation. Zero additional horror is marked unchanged; a horror increase shows the before and after totals. Opening or closing panels preserves the token objects and any reveal animation already in progress, so settled tokens do not appear again. Return to the table to continue inspecting; Show test result opens it again. A resolved curse applies −2 and requires another reveal: in the prepared curse example, the additional −2 token makes the result `9 − 2 − 2 = 5`. Canceling the curse prevents that additional reveal, as described in the [Innsmouth rules, page 1](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc81_rules_insert.pdf).

The tabletop resumes its existing case-specific save on the next launch. Start another example retains the prior run and replaces the active save only after the explicit button press. Reopen reports an incompatible/corrupt file rather than starting over. To inspect a particular session directly, use `--load PATH`.

Joel [accepted the reveal and discard follow-up with #22 on 2026-10-31](https://github.com/joeloverbeck/arkham-unfolded/issues/22#issuecomment-6536278638). Its [review steps](tony-turn.md#reveal-and-discard-follow-up) cover the reaction chooser without accidental reveals, Test focus, resolved Rotting Remains' discard and direct inspection, and replay suppression. The offered review ceiling was ten minutes across both experiments, including retries; it did not reset #22's original 40-minute gameplay allowance or #2's earlier allowance. No further review round is selected.

The reference is permanently at Test's upper-left, beside Rotting Remains when that source card is present. Click it for full inspection. This reference does not expand the fixture's supported token effects. During the commitment window, Promise of Power's teal edge and `+4` willpower badge identify its legal contribution; the action still displays its curse consequence. Contextual actions use vertical rows with wrapped details.

## Player-equivalent agent interface

The JSON-lines driver uses the same `Session::observe` and `Session::choose` functions as the tabletop:

```sh
scripts/dev cargo build --locked --bin arkham-driver &&
target/debug/arkham-driver --case curse --save local/experiment/agent.session.json
```

It emits an initial `observation`, including its revision and currently legal choice IDs. Send a command on each line, for example `{"op":"observe"}` or `{"op":"choose","revision":0,"id":"commit-promise"}`. Always use the current returned revision and a returned choice ID. Other operations are `save` and `quit`. A refused command returns an error and the unchanged observation. Successful choices autosave before being acknowledged.

The driver refuses to start over an existing save; use `--load PATH` to resume it. `--patrice-away` and `--used-jacqueline` are privileged starting-fixture variants for the independent refusal checks, unavailable as player operations and incompatible with `--load`. No future draw order is returned in player observations. The script is intentionally preserved privately inside saves so reopening does not change future outcomes.

## Scope of the prepared state

All three investigators are at the Study unless the separate fixture setup moves Patrice to the Hallway. Jacqueline has 2 starting horror from Arcane Research, no other relevant modifiers, and an unused reaction. Patrice has only Promise of Power available to commit; the other investigators have no committable cards. Rotting Remains tests willpower 3. The poor legal auto-fail choice deals 3 additional horror, leaving Jacqueline at 5 of 9 sanity.

The bag retains the original 16-token Standard composition. This experiment uses a privileged **priority draw order**, not random sampling: either curse/−1/0/−2 or auto-fail/curse/0/−2. If Promise is declined, the curse was never added and is skipped. Every actual reveal removes an available token; no unsupported symbol effect is improvised. The untouched bag symbols are not an implementation claim. The player observation omits future draws, and player commands cannot replace the fixture.

Declining Jacqueline for one reveal does not spend her round limit. If a resolved curse requires another reveal, she may still intervene then. All three cancellation pairs are available for a group without auto-fail; when auto-fail is present, she can cancel it alone or the other two tokens. A canceled curse's effects do not occur; a resolved curse goes to the pool at test end. These are source-derived interpretations within the accepted baseline, not bespoke publisher rulings.

## Continuity and evidence limits

Saves include the pending choice, held tokens, per-round use and remaining fixture order, bound to the application and source versions. The player protocol exposes observations rather than the full private save. This experiment has no gameplay Undo.

Runtime saves and screenshots belong under ignored `local/experiment/` or another explicitly selected local path. Pre-removal saves are unsupported; no historical reader or migration is maintained. An unsupported version or corrupt record must be reported without overwriting it. A clean clone can run the artwork-independent headless checks; native play requires a complete prepared collection. The raw reference PDFs/API responses remain separately retained under ignored `local/issue-1-evidence/`; their custody limitations remain as stated in the assessment.

The preceding valid save is retained as `<save-path>.previous`. An explicit new example also archives the prior run to a unique sibling file. To recover, reopen a valid retained file with `--load`; do not edit or overwrite a corrupt original to make it parse. This experiment verifies ordinary close/reopen and refusal behavior, not power-loss durability or long-term cross-version migration.

Passing automated checks establishes the exercised decisions and continuity boundaries only. Native rendering evidence establishes only the inspected session. Human acceptance of playing and narrating remains separate; the current review allowance and any explicitly selected extensions are recorded on #2. Full campaign branches, other token-replacement cards, unrestricted random play and recurrent usefulness remain unproven. At the 12-active-agent-hour bound, or if the governing assumptions fail, return the evidence to #2 without activating more work.

## Developer native verification

The shared procedure moved to the [native-verification runbook](../agents/native-verification.md#developer-native-verification) when this experiment was retired.

### Prepare a compact native handoff

See [Prepare a compact native handoff](../agents/native-verification.md#prepare-a-compact-native-handoff).

### Build an instrumented source copy

See [Build an instrumented source copy](../agents/native-verification.md#build-an-instrumented-source-copy).

### Launch, capture and interact

See [Launch, capture and interact](../agents/native-verification.md#launch,-capture-and-interact).

### Record a short motion clip

See [Record a short motion clip](../agents/native-verification.md#record-a-short-motion-clip).

### Retain final verification evidence

See [Retain final verification evidence](../agents/native-verification.md#retain-final-verification-evidence).

### Foreground performance measurements

See [Foreground performance measurements](../agents/native-verification.md#foreground-performance-measurements).

### Capture a laggy camera

See [Capture a laggy camera](../agents/native-verification.md#capture-a-laggy-camera).

### Visual and wording review

See [Visual and wording review](../agents/native-verification.md#visual-and-wording-review).

