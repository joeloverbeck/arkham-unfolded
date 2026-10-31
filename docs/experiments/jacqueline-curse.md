# Jacqueline's token choice: running the experiment

[GitHub #2](https://github.com/joeloverbeck/arkham-unfolded/issues/2) owns this experiment and its acceptance. Joel selected it after the [assessment](../research/issue-1-assessment.md) at `ef905f5`. It is one scripted skill-test case, not a playable campaign or an adopted game engine.

The task is to decide whether Patrice commits Promise of Power, decide when Jacqueline intervenes, choose which tokens to cancel, and understand the resulting horror and token destinations. You can stop at any pending decision. Then reopen a saved choice and narrate the same events through replay. The source expectations and four worked outcomes are in the [source assessment](../research/issue-1-source-reconciliation.md).

## Build and run

This is the earlier Jacqueline mode; the [README](../../README.md#launch-and-play-the-tony-turn) leads with the new Tony turn. Requires Rust/Cargo 1.97.1 or later and a working Ubuntu X11/XWayland graphics session with Vulkan support. The lockfile pins the experiment's dependencies, including Bevy 0.20.0. First dependency acquisition/build requires network access and can take several minutes; the resulting application, saving and replay run offline.

Open Terminal (`Ctrl` + `Alt` + `T`), then run the following. Replace the checkout directory if yours is elsewhere. Keep the terminal open while the native window is running.

```sh
cd /home/joeloverbeck/src/arkham-unfolded
cargo run --locked --features tabletop --bin arkham-tabletop
```

The window is titled **Arkham Unfolded — Scripted Rules Experiment**. For development verification, run `scripts/check` from the same directory; checks are not a prerequisite to playing. For native rendering or interaction changes, also follow [developer native verification](#developer-native-verification).

The scenario surface holds the map. A separate Test table sits between it and the investigator row, with the chaos bag at upper right and distinct sections for committed cards, revealed tokens and canceled tokens. Each compact investigator surface is as wide as the scenario table, with the name above its upper edge, permanent thick harm counters above the upper-left investigator card, deck/discard footprints at upper right, and one centered hand row along the near edge. Muted markings separate the five-card threat band, central assets and shallow hand strip. This experiment represents horror, including Jacqueline's initial 2/9; unrepresented damage reads `n/a`. Unsupported cards and piles remain explicitly unrepresented. Use portrait buttons to focus an investigator, **Scenario** for the shared map, **Test** for the test table, or **Overview** for orientation. Focusing Patrice does not change Jacqueline's test or the pending actor shown in the decision display. This corrected local candidate does not establish owner acceptance or crowded full-game capacity.

In a fresh case, focus Patrice and left-click Promise of Power to open its contextual controls, then choose **Commit +4** with the willpower symbol after reading the curse consequence. Its four wild icons contribute +4 to this willpower test; card inspection retains their wild identity. If Patrice is elsewhere, she cannot contribute in this experiment and the empty commitment prompt is skipped, leaving Jacqueline's reaction available. Use **Reveal chaos token** at the lower left: while Jacqueline's reaction is available, it offers a normal reveal that declines her reaction for this reveal, or **Use Jacqueline · reveal 3 tokens**. Opening or dismissing that chooser does not reveal or spend the reaction. Choose the reaction to reveal three and focus **Test**, then select a token to open the legal cancellation combinations, choose the complete set and click **Confirm cancellation**. Token numbers distinguish the exact revealed pieces. Selecting a physical card or token alone does not commit, reveal or cancel anything. If a curse calls for another reveal, use the shortcut when a new choice is offered; automatic continuations need no extra click. After completion, click **Show test result** when offered; other completed tests open the result automatically. Use **Return to table**, then **Open recorded replay** to step through stored moments. Choose a card's **Inspect** control for enlarged artwork and readable rules; **Close card** or `Esc` dismisses inspection. Decisions save to `local/experiment/curse.session.json`; relaunching resumes it. Use **Save live session** to retain the initial state, or **Start another example** after a completed/ended case to archive it and begin again.

When only one reveal route remains, the shortcut submits it directly. After a successful saved choice it focuses Test; failed saves and stale clicks leave the game and camera unchanged. The shortcut appears only for legal reveals, is absent in replay, and respects blocking panels and reduced motion. The physical bag and Jacqueline's existing contextual controls remain available with their existing camera behavior. A card or component whose sole operation is Inspect opens inspection directly; multiple Inspect targets retain a chooser, and a lone gameplay action never executes automatically.

The encounter discard begins empty. When Rotting Remains' revelation finishes, including a successful test with no horror, the card moves face up to that pile in the scenario area. Return to the table after the result to see its count and click the one-card pile for direct inspection. Ending an unfinished example does not resolve or discard the treachery. Save/reopen, player observations and recorded replay show the corresponding destination; existing recorded values remain intact. Encounter-deck and victory-display contents remain unrepresented.

For the other prepared case or a deliberate artwork-absence check:

```sh
cargo run --locked --features tabletop --bin arkham-tabletop -- --case autofail
cargo run --locked --features tabletop --bin arkham-tabletop -- --no-art
```

The normal asset root is `$XDG_DATA_HOME/arkham-unfolded/assets-v2`, normally `~/.local/share/arkham-unfolded/assets-v2`, or the saved per-user preference in `$XDG_CONFIG_HOME/arkham-unfolded/config.toml`. Prepare this 14-file collection with the commands below; Tony uses a separate `tony.toml` preference. Startup reports the experiment, resolved folder and completeness. **Choose asset folder** in the side panel opens Ubuntu's `zenity` folder dialog, checks the selected collection, and offers to save it for the next launch. Scroll the panel if necessary. Restart to apply the choice. Canceling or a write failure preserves the previous setting; a failed write is reported. An incomplete recognized collection requires explicit confirmation to use readable substitutes; an unrecognized collection cannot be saved as a valid selection. The status in the panel distinguishes complete assets from substitutes.

```sh
curl --fail --location \
  'https://game-icons.net/icons/ffffff/transparent/1x1/lorc/swap-bag.png' \
  --output /tmp/arkham-reveal-icon.png
cargo run --locked --bin arkham-assets -- prepare \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /tmp/arkham-reveal-icon.png \
  --output "$HOME/.local/share/arkham-unfolded/assets-v2"
cargo run --locked --bin arkham-assets -- check "$HOME/.local/share/arkham-unfolded/assets-v2"
```

`--art-root PATH` selects another prepared collection for that launch without rewriting the preference; `--no-art` deliberately uses substitutes. The source archive is never a runtime fallback. Copying the whole prepared folder, including `manifest.json`, is sufficient to relocate the selected presentation assets. Personal saves and settings stay separate. Missing or changed files use readable substitutes, and original image/font bytes stay outside Git. A card's presence or absence never changes legal choices, rules outcomes or record compatibility. The [evidence manifest](../research/issue-1-evidence.json) preserves the source identities for the three original faces.

The reveal icon is [Swap bag by Lorc](https://game-icons.net/1x1/lorc/swap-bag.html), licensed [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). Preparation checks the unchanged 5,829-byte PNG against SHA-256 `54673d4cc97a84c90cab1d0a38b46183eb182c567493de676d7124ff3d70fe3e`, copies it to `ui/reveal-chaos-token.png`, and preserves attribution in the recipe and manifest. The application may tint its white silhouette and never needs the download location during play. The earlier Jacqueline v1 collection remains recognized: its 13 verified files remain usable, and the missing icon uses the readable button. Its check reports incomplete new requirements without modifying anything. Saved folder preferences remain unchanged; choose the fresh folder to use the new image.

The five revealed token faces preserve the archive's consistent 1024-pixel circular set originally selected under `sce-base-save`, with source identities and hashes in [token-art.json](token-art.json). Their prepared copies live under `tokens/chaos/`. English face and status labels remain visible beside the graphics. Missing or changed token images use the simple labeled pieces. Alternate designs were found under `lovecrafter_3077` and `rise_rapture_rise`, including smaller square renditions; the selected circular set is pinned for this experiment. Archive categories describe provenance and do not certify publisher artwork.

Card inspection and results use the prepared Arkham font for wild, curse, willpower and horror symbols; the test summary shows willpower on both the current value and difficulty. [symbol-art.json](symbol-art.json) preserves the exact source bundle, glyph mappings and extraction evidence. The preparation command extracts that same verified font once into `fonts/arkham-symbols.ttf`; ordinary play loads the font directly and does not decode Unity data. Missing or changed font data uses English words, and accessible labels retain the plain-English meaning.

Tony, Patrice and Jacqueline use their original miniature-card portraits as upright standees with a rounded base and rectangular holder. The [shared renderer](../../src/tabletop_standees.rs) is the standard for both experiments and subsequent scenario presentation; its class colors match the investigator surfaces. Both sides show the color portrait without mirroring; camera position does not imply a state change. [investigator-art.json](investigator-art.json) records the selected images, pixel-art alternatives and model search. No investigator-specific sculpt or UV mapping was found among the catalogued model resources; Unity geometry was not decoded. Missing portraits individually use the labeled simple stand-in.

[The finite preparation recipe](../../resources/asset-selection.json) binds these selected source files to their application-relative destinations and expected hashes. Card faces live under `cards/<card-code>/`, portraits under `investigators/<investigator-code>/`, and the optional reaction marker under `markers/`. Preparation and `arkham-assets check` verify the complete selection. If an existing output is incomplete or differs, prepare a fresh output and select it; the command never repairs a collection by overwriting unknown contents. The same folder/check/save/restart path is used for an update. Prior collections are not automatically deleted.

Jacqueline's investigator card is displayed in landscape on the table and in inspection by rotating its unchanged source image counterclockwise. Inspection enlarges the original art alongside readable rules; on smaller windows its content scrolls while the Close control stays visible. Close card or Escape returns to the table.

Promise of Power gains a highlighted frame when committed. A small labeled reminder to the right of Jacqueline's card flips between available and used, following her once-per-round reaction in live play and replay. It uses the archive's Universal Action / Ability Token as an optional visual base; it is an application reminder, and does not exhaust Jacqueline's investigator card. Canceled tokens move to a separate group with faded faces and visible crosses, while resolved tokens retain their faces. Replay navigation appears only when another moment exists in that direction.

Component actions appear beside the selected card or token; choices such as declining an optional effect remain in the decision controls. `Esc` or clicking empty space closes contextual controls without a game action. Hold unmodified `W`/`A`/`S`/`D` to pan forward/left/back/right across the table relative to the camera, preserving tilt and zoom; release to stop. Right-drag or arrow keys orbit; the wheel or `+`/`-` zooms; `Home` returns to the overview. Camera keys are suspended while a menu or modal captures keyboard input. Manual camera movement overrides automatic following and closes the contextual menu. Replay's Next and Previous controls pause on recorded moments; cards remain inspectable, but game actions require returning to live play. Explain opens the action history. F12 captures the window under `local/experiment/`; use the independent capture procedure below for development verification.

A completed test is saved immediately. When a curse resolves, the table stays visible to show its additional token reveal, with the explanation **Curse: −2. Reveal another token.** Choose **Show test result** when ready; the same pacing applies in replay and when reopening a completed curse test. Other completed tests open the centered result window automatically. The window shows the outcome and skill values, horror and recorded explanation. Zero additional horror is marked unchanged; a horror increase shows the before and after totals. Opening or closing panels preserves the token objects and any reveal animation already in progress, so settled tokens do not appear again. Return to the table to continue inspecting or replaying; Show test result opens it again. Replay presents the recorded result through the same window. A resolved curse applies −2 and requires another reveal: in the prepared curse example, the additional −2 token makes the result `9 − 2 − 2 = 5`. Canceling the curse prevents that additional reveal, as described in the [Innsmouth rules, page 1](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc81_rules_insert.pdf).

The tabletop resumes its existing case-specific save on the next launch. Start another example retains the prior run and replaces the active save only after the explicit button press. Reopen reports an incompatible/corrupt file rather than starting over. To inspect a particular session directly, use `--load PATH`.

Joel [accepted the reveal and discard follow-up with #22 on 2026-10-31](https://github.com/joeloverbeck/arkham-unfolded/issues/22#issuecomment-6536278638). Its [review steps](tony-turn.md#reveal-and-discard-follow-up) cover the reaction chooser without accidental reveals, Test focus, resolved Rotting Remains' discard and direct inspection, and replay suppression. The offered review ceiling was ten minutes across both experiments, including retries; it did not reset #22's original 40-minute gameplay allowance or #2's earlier allowance. No further review round is selected.

## Player-equivalent agent interface

The JSON-lines driver uses the same `Session::observe` and `Session::choose` functions as the tabletop:

```sh
cargo run --locked --bin arkham-driver -- --case curse --save local/experiment/agent.session.json
```

It emits an initial `observation`, including its revision and currently legal choice IDs. Send a command on each line, for example `{"op":"observe"}` or `{"op":"choose","revision":0,"id":"commit-promise"}`. Always use the current returned revision and a returned choice ID. Other operations are `save`, `replay`, `next`, `back`, `live` and `quit`. A refused command returns an error and the unchanged observation. Replay refuses decisions and past-state saves. Successful choices autosave before being acknowledged.

The driver refuses to start over an existing save; use `--load PATH` to resume it. `--patrice-away` and `--used-jacqueline` are privileged starting-fixture variants for the independent refusal checks, unavailable as player operations and incompatible with `--load`. No future draw order is returned in player observations. The script is intentionally preserved privately inside saves so reopening does not change future outcomes.

## Scope of the prepared state

All three investigators are at the Study unless the separate fixture setup moves Patrice to the Hallway. Jacqueline has 2 starting horror from Arcane Research, no other relevant modifiers, and an unused reaction. Patrice has only Promise of Power available to commit; the other investigators have no committable cards. Rotting Remains tests willpower 3. The poor legal auto-fail choice deals 3 additional horror, leaving Jacqueline at 5 of 9 sanity.

The bag retains the original 16-token Standard composition. This experiment uses a privileged **priority draw order**, not random sampling: either curse/−1/0/−2 or auto-fail/curse/0/−2. If Promise is declined, the curse was never added and is skipped. Every actual reveal removes an available token; no unsupported symbol effect is improvised. The untouched bag symbols are not an implementation claim. The player observation omits future draws, and player commands cannot replace the fixture.

Declining Jacqueline for one reveal does not spend her round limit. If a resolved curse requires another reveal, she may still intervene then. All three cancellation pairs are available for a group without auto-fail; when auto-fail is present, she can cancel it alone or the other two tokens. A canceled curse's effects do not occur; a resolved curse goes to the pool at test end. These are source-derived interpretations within the accepted baseline, not bespoke publisher rulings.

## Continuity and evidence limits

Saves include the pending choice, held tokens, per-round use and remaining fixture order, bound to the application and source versions. The player protocol exposes observations rather than the full private save. Replays retain the observed moments and do not execute today's rules again. Going back in a replay does not undo or edit the original match.

An older save pending an empty commitment with Patrice elsewhere reopens at Jacqueline's reaction/reveal decision. Its original commitment moment and save bytes remain intact when loading or manually saving; the next genuine choice appends a new revision through the existing save-before-install path. Replay continues to show the historical moments exactly as recorded. This change does not add gameplay undo to this experiment.

Runtime saves, recordings and screenshots belong under ignored `local/experiment/` or another explicitly selected local path. Preserve original records and a compatible build; there is no general migration promise. An unsupported version or corrupt record must be reported without overwriting it. A clean clone can run the source-independent checks and display substitutes without the owner's archive. The raw reference PDFs/API responses remain separately retained under ignored `local/issue-1-evidence/`; their custody limitations remain as stated in the assessment.

The preceding valid save is retained as `<save-path>.previous`. An explicit new example also archives the prior run to a unique sibling file. To recover, reopen a valid retained file with `--load`; do not edit or overwrite a corrupt original to make it parse. Recording exports are immutable and named by their content; an attempt to replace one with a different match is refused. This experiment verifies ordinary close/reopen and refusal behavior, not power-loss durability or long-term cross-version migration.

Passing automated checks establishes the exercised decisions and continuity boundaries only. Native rendering evidence establishes only the inspected session. Human acceptance of playing and narrating remains separate; the current review allowance and any explicitly selected extensions are recorded on #2. Full campaign branches, other token-replacement cards, unrestricted random play and recurrent usefulness remain unproven. At the 12-active-agent-hour bound, or if the governing assumptions fail, return the evidence to #2 without activating more work.

## Developer native verification

For slow startup, rendering or camera movement, begin with the [native-performance procedure](../../.claude/skills/diagnosing-bugs/native-performance.md) before changing the launch conditions.

### Build an instrumented source copy

Before compiling a temporary instrumented source copy, use the isolated build helper. From the normal repository root, build the normal executable if it is absent, then choose a new output directory outside both checkouts:

```sh
scripts/tools native build \
  --source /absolute/path/to/diagnostic-source \
  --output-dir /absolute/path/to/fresh-diagnostic-build \
  --reference "$PWD/target/debug/arkham-tabletop"
```

This runs offline, locked Cargo with a separate target and intermediate build directory, records the build log and hashes the normal executable before and after, including on build failure. Run only the returned diagnostic binary for that experiment. Sharing target output can replace the normal executable, and a subsequent Cargo build can report it fresh without restoring it. A different executable name is insufficient: an instrumented copy can still share the package's library artifacts and contaminate a later normal build. The helper preserves output for inspection rather than cleaning either checkout.

### Launch, capture and interact

For a reported physical-input failure, establish the failing boundary before repeating gestures or changing camera code:

1. Check whether the **same physical button or key** works in another application. Reuse an observation already supplied by the owner; otherwise ask for one brief comparison. A working left button does not establish that right-button presses reach Arkham. A failure elsewhere identifies a wider input problem, without deciding whether its cause is hardware or desktop software.
2. If the failure is confined to Arkham, distinguish input arrival from input handling. Check window focus and, when needed, use bounded diagnostic logging of cursor movement and mouse-button press/release events at Bevy's input boundary. Missing right-button events and received events rejected by UI/camera capture are different findings. Repeat an owner gesture only when it will distinguish the remaining explanations.
3. When the expected event arrives, exercise the affected UI/camera path. Independently reproduced application defects can coexist with a desktop-input failure. A portal-generated right-drag verifies the synthetic input path only; it does not establish that the physical button works. A failed portal calibration follows the bounded recovery below and is not evidence of a camera defect.

Before requesting an idle-desktop interval, finish the build and relevant headless checks, stage the executable, and confirm that the intended app process and client window are still present. Prepare a route through the affected behavior with expected visible results, reaching the highest-risk unverified transitions early and sharing setup where possible. Keep the app's owner-review lifetime separate from the short desktop-control interval. Budget consent, calibration, navigation, capture inspection and cleanup within that interval. Keep the accepted deadline and give a runnable remaining checklist at handoff.

Use a separate save and temporary settings for diagnostics. The Rust helper below targets the current Ubuntu GNOME/XWayland desktop; capture/input needs access to X11, the desktop session bus and its RemoteDesktop/ScreenCast portal, plus `xdotool` for explicit focus. `scripts/tools` builds only the small `arkham-tools` package. Its headless tests run in `scripts/check`, using a private `dbus-daemon` for protocol tests; no real desktop grant is requested by the tests. From the repository root, build the normal application and launch a disposable run:

```sh
cargo build --locked --features tabletop --bin arkham-tabletop
mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}"
review_dir=$(mktemp -d "${XDG_CACHE_HOME:-$HOME/.cache}/arkham-native.XXXXXX")
scripts/tools native launch \
  --binary target/debug/arkham-tabletop --output-dir "$review_dir/run" \
  --experiment jacqueline --no-art
```

The launcher copies and hashes the executable before starting it. Later builds cannot replace that running copy. It creates run-local configuration, data, cache, working directory and `session.json`; `--load SOURCE_SAVE` copies the supplied save before loading, because the loaded path is also the autosave destination. Original saves and settings stay untouched. The output directory must be new and outside the checkout. No-art checks readable substitutes; for real artwork use `--art-root /absolute/path/to/prepared-assets` and confirm completeness in `run/app.log`.

For static layout checks, add `--initial-view scenario --screenshot scenario.png --quit-after-screenshot` to `native launch`. The normal executable starts at the existing Scenario focus without pointer input or a source-copy build. `overview`, `test`, `tony`, `patrice` and `jacqueline` are also supported. The chosen `initial_view` is recorded in `run.json`; omitting it preserves normal startup. This diagnostic option affects the first camera position only: Home retains the experiment's normal home view, and saves and gameplay are unchanged. These captures establish rendering evidence; checking focus buttons or camera gestures still requires the interaction procedure below.

Each executable/save copy reaches its final path only after verification. A failed copy removes its own temporary bytes and reports the destination, attempted size and underlying error; source files and earlier runs remain intact. Use a disk-backed destination with sufficient space and user quota: debug executables can be gigabytes, and filesystem free space alone does not establish quota availability. The example uses the user's cache directory; verify its backing filesystem if that location is customized.

The command stays open until the window closes and stops its own process group after 1800 seconds (30 minutes; override with `--timeout-seconds`, maximum 3600), or on interruption. This longer default leaves time to read an owner checklist and inspect results; it grants no desktop-control permission. Set a shorter explicit timeout for automated startup runs. Its first JSON output and `run/run.json` identify the app PID, process start, staged executable hash and exact arguments. In another terminal set `review_dir` to the printed run directory (the directory ending in `/run`) and `native_pid` to the recorded **app** PID. A background launch command's `$!` is the supervisor PID, not the app PID.

The manifest's `environment` field retains the XDG overrides. `launch_conditions` records the helper's setup actions and allowlisted inherited/effective environment settings; `null` means unset, a string preserves the exact UTF-8 value, and `unix_bytes` preserves non-UTF-8 bytes. These settings are captured before spawn and describe the helper's launch, not a previous owner process. Capture the ordinary baseline separately while it is available, and keep unknown conditions explicit.

Run window discovery, focus, capture, process inspection and cleanup with the same desktop access and process visibility as the launch. Obtain any required tool permission for that context; a missing PID in an isolated shell does not establish that the launched process exited.

Identify visible windows by that PID (`xdotool search --onlyvisible --pid "$native_pid"`) and set `native_window` to the Arkham client ID. Title-only searches can also match editor windows. The helper verifies the exact title, PID, process start identity and focus. It reads client geometry directly from X11; older `xdotool getwindowgeometry` versions can include the frame offset twice.

For capture alone, run:

```sh
timeout 10s scripts/tools native capture \
  --pid "$native_pid" --window "$native_window" --output "$review_dir/client.png"
```

This captures the client buffer through X11 `GetImage`, independently of the app's screenshot path. Inspect the image and reported pixel size. A new filename, timestamp or pixel hash does not establish the expected state: F12 produced new files with stale table contents during #14. Use independent captures before and after a visible transition, and inspect the actual result. Startup `--screenshot` remains a rendering check only.

For interaction, keep one helper process and its input open for the entire diagnostic. After coordinating the interval, set and announce its absolute end time once. This example begins a three-minute interval now:

```sh
deadline_unix=$(date -u -d '+3 minutes' +%s)
date -u -d "@$deadline_unix" '+Desktop control ends at %H:%M:%S UTC'
scripts/tools native session \
  --pid "$native_pid" --window "$native_window" --output-dir "$review_dir/interaction" \
  --deadline-unix-seconds "$deadline_unix"
```

Use an interactive terminal or a tool invocation with a persistent PTY (`tty: true` for `exec_command`), then send commands through that process's returned session ID. A pipe is suitable only while its writer remains open. Starting the helper with closed stdin cancels consent and input; reopening stdin requires a new helper and grant.

The earlier of the absolute deadline and `--timeout-seconds` applies. Startup, consent and idle input all consume this same budget. Keep the original `deadline_unix` if a helper restart is necessary within the agreed interval. The helper's `session-budget` event records its effective end time and remaining time. The final 15 seconds are reserved for releasing buttons and closing the grant; each operation needs its full 15-second allowance before that reserve. Late startup or consent closes the helper without further calibration or input. A late command is refused before optional focus acquisition or input. `stop` remains available without a new operation allowance. `command-started` includes `session_remaining_ms` alongside the request number.

The output directory must be new. The helper saves `initial.png` before requesting consent. In GNOME's dialog, select the monitor containing Arkham, enable **Allow Remote Interaction**, and click **Share**. One nonpersistent, pointer-only grant covers the run. The total session limit is 600 seconds, including consent and idle input; `--timeout-seconds` can shorten it. Keep Arkham covering the monitor's central 10%. Keep the keyboard and pointer idle throughout calibration and gestures. The helper measures the portal-to-X11 mapping and checks it at an independent point; it does not reuse a previous monitor's scale. Wait for `ready`, then inspect `initial.png` before choosing a target. Coordinates are pixels relative to the captured client, not a resized image in the conversation. A calibration refusal retains the session for the bounded recovery below unless its time budget expired.

Send one JSON command per line on the same process's stdin. Each dequeued command emits `command-started` with a sequential `request` number, then a terminal result with that number and `elapsed_ms`. Wait for the terminal result before sending another command. Each command has a 15-second budget checked before input and between guarded operations; focus and D-Bus calls retain their five-second call limits. A timeout attempts button release, reports `refused`, and ends the session for cleanup. This is cooperative cancellation, so an in-progress native call or cleanup can extend the response beyond the budget. A refusal can follow partial input: inspect before retrying. The input queue holds at most eight pending commands; overflow stops the input reader and cancels portal activity. EOF or a read error during pending consent cancels the request without waiting for approval. For example, after replacing `X` and `Y` with a target identified in the inspected image:

```text
{"action":"click","focus":true,"x":X,"y":Y}
{"action":"capture","focus":true,"name":"after-result.png"}
```

For an orbit check, choose a start and end on exposed table space in the inspected capture, then send a bounded right-drag. For example, after checking that these client coordinates are appropriate:

```text
{"action":"right-drag","focus":true,"x":800,"y":450,"dx":120,"dy":30,"duration_seconds":0.6}
{"action":"capture","focus":true,"name":"after-orbit.png"}
```

`dx` and `dy` are displacement in client pixels. The nonzero gesture is limited to `abs(dx) + abs(dy) <= 600`; `duration_seconds` defaults to 0.6 and accepts 0.2–2.0 seconds, with 20 movement steps. Desktop verification adds time to the requested movement duration. The helper validates the whole path before pressing, rechecks the window and measured pointer during movement, and releases the right button on failure or interruption. A path that fails preflight sends no press.

Wait for `clicked` or `right-dragged` and allow the visible transition to settle before capturing; inspect the result before choosing another action. A completed gesture is input evidence only: compare the captures to establish camera movement. The helper checks process identity, focus, unchanged geometry and the measured client pointer position immediately before each press. It refuses off-window, off-monitor and mismapped targets. It releases an attempted press even when interrupted.

Switching to a terminal or agent interface can take focus from Arkham between commands. Set `"focus":true` on the operation to reacquire the verified client immediately before its guards and input, within the same request. The default leaves focus untouched; `{"action":"focus"}` remains available for focus alone. Reacquiring focus preserves all input guards, including refusal if focus changes during a drag. Send the gesture first, let the application render, then request the capture; a capture queued immediately after release can still show the preceding frame. Inspect pixels rather than interpreting an intended-state filename as proof.

An ordinary `refused` command keeps the grant open; insufficient remaining time, command timeout or failed button release ends it. Diagnose once within the surviving session; `{"action":"calibrate","focus":true}` reacquires focus and remeasures a changed monitor mapping. Inspect a new capture before retrying a target. Stop after one failed capture/focus/click/drag attempt plus at most one bounded diagnosis attempt. If interaction remains unavailable, report the limitation and use the already-agreed owner check within its remaining allowance. Do not interpret a stale image as evidence against what the owner sees on the desktop.

Send `{"action":"stop"}` (acknowledged by `stopped`) or close stdin when finished, and wait for `session-closed`. EOF, fatal errors and handled interruption close the portal grant; a killed or crashed process loses its session when its D-Bus connection closes. `events.jsonl` records capture paths, pixel hashes, mapping, command acknowledgments, elapsed times, refusals and closure. It contains no artwork itself; keep it with the local captures.

The cleanup reserve reduces normal overruns but cannot guarantee interruption of a blocked native call. Inspect the actual closure timestamp and report any overrun of the announced end time. A timeout during calibration ends the session as well.

For folder persistence, exercise **Choose asset folder → check → Save folder → close**. After the supervisor exits, relaunch the staged executable using the same working directory, XDG directories and save recorded in `run.json`, omitting both `--art-root` and `--no-art`. Verify that the saved path and asset status are restored; do not create a fresh isolated run for this restart check. Capture helpers must use the new PID. A screenshot establishes the inspected rendering; the actual interaction and restart establish their separate behavior. Any review of the owner's real preference uses its explicitly agreed scope instead of these diagnostic settings.

Close the verification window normally and let the launch supervisor report its exit. If cleanup is needed, interrupt that supervisor; for a manually restarted app, confirm its recorded identity before terminating it. Stop only the identified launch and helpers. Keep logs/captures local; record the revision, command, outcome and evidence location in the active issue. Remove temporary files only after the needed evidence has been retained.

Protocol references: [RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html) and [ScreenCast](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html). Support and evidence remain limited to the desktop and journey actually exercised.

### Foreground performance measurements

Before a timed check, establish who will keep the app visible and focused. Within an authorized desktop interval, use the verified PID/window and existing focus support: `native capture` focuses its client, and a running native session accepts `{"action":"focus"}` or `"focus":true` on an operation. Wait for that operation's result before selecting the measurement interval. When the check needs the owner's attention, establish readiness before launch instead of sending a focus request during a short countdown. Record any capture or focus operation as part of the comparison's conditions.

Record usable foreground duration or sample count alongside total elapsed time and measured rates. Background time does not satisfy the graphics check. A supervisor timeout without enough foreground evidence is **inconclusive**, not evidence of GPU failure; retain any independently reported renderer error separately. Keep the original absolute deadline, cleanup reserve and existing focus guards. Focus loss consumes the remaining allowance; it does not authorize a longer run or another attempt.

### Capture a laggy camera

When a running app is actually laggy and an owner-run CPU capture is needed, identify its PID and exact executable first. For a staged run, use `run.json`; for an ordinary launch, use `pgrep -a -x arkham-tabletop` and `readlink /proc/PID/exe`, then select the intended window/process. Do not rebuild that executable during the capture. With `perf`, `sudo` and `timeout` installed, run:

```sh
scripts/tools profile-native \
  --pid APP_PID --executable /absolute/path/to/arkham-tabletop \
  --output-dir /tmp/arkham-camera-profile-UNUSED
```

The output directory must not exist. The command verifies process identity, announces the interaction, authenticates sudo, then counts down five seconds. **As the countdown starts, switch to the app and continuously right-drag to orbit and scroll to zoom. Continue through the ten-second capture.** The terminal reports completion, the `perf.data` path and byte size. `--seconds` accepts 1–60 and `--countdown` 1–10. Logs and failed/partial captures are preserved separately from a completed result; use a fresh directory for a retry.

This wrapper does not change kernel profiling settings or run a permanent service. A nonempty capture proves collection, not that it contains the reported lag or explains its cause. Keep the capture local and record whether the symptom occurred during this interval before interpreting it.

### Visual and wording review

Before handing a tabletop change to Joel, review the affected player journey through the native window, including existing cards and panels encountered along the way. Use the verification procedure above and record the states and viewport exercised. Apply the relevant checks below; a local change does not require replaying unrelated cases.

- **Card inspection:** original faces are upright in their intended orientation, including landscape investigators. Printed text is readable in inspection; scrolling and Close remain usable at the exercised window size. Inspect the real artwork and the readable substitutes when their layout is affected.
- **Existing visual behavior:** when a change introduces an already represented object, compare it with the established experiment at a comparable camera and viewport. Check token thickness, face artwork, label backing, colors and card orientation in both art and substitute modes as relevant. Preserve the useful existing behavior or explain the intentional difference in the PR; two implementations alone do not justify two visual standards.
- **Decision states:** committing a card, spending a reaction and canceling or resolving a token are visibly distinguishable. Follow the transition as well as the final state. Replay navigation reflects whether another moment exists in that direction.
- **Rules wording:** headings identify recognizable cards or events. Explanations connect modifiers, extra reveals and consequences to their cause. Distinguish an unchanged total from a new effect; wording such as “now has” must correspond to a change. Judge these statements against the accepted rules and recorded events.
- **Results and continuity:** the outcome and its consequences are easy to find. Dismiss and reopen inspection/results along the affected path; settled pieces stay settled, an animation already in progress continues, and panel actions preserve game state. Walk the corresponding recorded moments forward and back, including manual camera override when camera or replay behavior changes.

Read captures at the displayed scale and use actual interaction for movement and continuity claims. Report correctness and readability defects separately from preferences such as centering, decoration or reminder size; explain a preference's effect instead of making it a universal rule. Identify any unavailable interaction as unverified and follow the bounded fallback above. Tests, captures and reviewer judgment retain their separate evidence limits; Joel's acceptance remains his decision.
