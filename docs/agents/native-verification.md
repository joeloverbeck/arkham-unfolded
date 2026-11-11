# Native verification

This shared procedure covers the current application and its Gathering diagnostics. Scenario selection is the default entry; it requires no game save or private artwork. It does not establish native or owner acceptance by itself.

## Developer native verification

For slow startup, rendering or camera movement, begin with the [native-performance procedure](../../.claude/skills/diagnosing-bugs/native-performance.md) before changing the launch conditions.

### Prepare a compact native handoff

Prepare the route's saves and their production-startup UI checks before freezing
the candidate for final code review. Include completed check-record paths in the
review packet, including the real-asset check when the route depends on supplied
artwork or fonts. This keeps fixture corrections inside the reviewed candidate.
For Gathering draws, use the [draw checkpoints and timed route](../experiments/gathering-setup.md#card-draw-review-preparation), whose builder is shared with the production-startup checks.
For Gathering, replace the collection path below and use fresh output directories:

```sh
scripts/tools record-check --output-dir /tmp/gathering-review-startup -- \
  scripts/dev cargo test --locked --features tabletop --lib \
  gathering_tabletop::tests::review_saves &&
scripts/tools record-check --output-dir /tmp/gathering-review-assets -- \
  env ARKHAM_GATHERING_TEST_ASSETS=/absolute/path/to/prepared-collection \
  scripts/dev cargo test --locked --features tabletop --lib \
  gathering_tabletop::art::tests::real_collection_loads_every_verified_image_and_symbol_font \
  -- --ignored --exact
```

Require each `run.json` to be `finished` with `exit_code: 0`; retain `check.log`
with it. The real-asset test is optional for source-only checks and requires a
complete external collection; CI remains independent of private artwork. Carry
forward successful records when later changes do not invalidate their evidence.

For fixture builders that read commands, use `record-check --stdin-file PATH`;
the default is null stdin, including when the caller supplies a pipe or heredoc.
The helper retains the exact input and its hash. Add `--expect-file PATH` for each
new fixture output: an existing path is refused before execution, and a missing,
empty or non-file result makes an otherwise successful command incomplete.
Use fresh paths outside the checkout. Inspect the command's responses and load
the result through the production startup check below; file presence and a hash
do not establish that a save is usable.

Before returning an implementation for owner play, check the owner's normal
launch command, including its working directory, settings and resolved save
path. After an intentional save-compatibility break, provide a tested fresh-start
command with an unused `--save` path and the expected setup screen; update
README's launch/recovery instructions in the same handoff. Keep the existing
save bytes intact. Prepared diagnostic checkpoints have a separate purpose and
do not establish that the ordinary launch command works.

Finish source reading, tool discovery, builds, headless checks and route planning before starting a short desktop-control interval. Run `scripts/tools native session --help` to check supported operations and limits without opening a desktop session; confirm that the intended route fits them. Keep the following runnable checklist in private local evidence and link it directly to the owner in the conversation. When the owner operates the run, the reply itself also carries the exact copyable launch command and numbered route; the checklist remains the record. The issue's [current handoff](issue-tracker.md#current-handoff) contains the compact public status below. Complete any needed context transfer before starting the clock; the receiving context should be able to execute the route from the private checklist and this procedure without reconstructing the implementation transcript.

```text
Private checklist — do not upload:
Owning issue: URL; candidate commit and executable/hash; completed check records.
Retention: final evidence directory; selected records and original-path mapping.
Normal launch: owner command/cwd; resolved save; fresh or resume; startup result;
  tested replacement command and README recovery when compatibility changed.
Run: exact launch/resume command; isolated save/settings/art paths; run manifest.
Artwork, when required by the claim: resolved launch root and completeness-check result.
Prepared saves: input identity/revision; production-startup check/result;
  expected visible screen, pending modal/Continue state and available action.
Authority: accepted journey, total allowance and stop conditions.
Capabilities: helper revision; supported actions needed by this route and limits.
Human route, when the owner operates the game:
  numbered action -> expected visible result; exact copyable picker paths;
  agreed capture pauses; effective app close deadline
  one report after closing: passed / failed / not tested for each item
Route, highest-risk unverified behavior first:
  exact JSON command -> expected visible result -> required camera view/objects
  capture command -> what to inspect; mark unknown coordinates for the first capture
Time: operator (owner or agent); interactive or automatic capture;
  approval ID and original absolute UTC/Unix deadline; effective app shutdown and limiting condition;
  route checkpoints; fresh-process reopening
  allowance; final 15-second cleanup reserve (each command needs 15 seconds before it).
Recovery: app PID/start identity and window; supervisor and helper tool session IDs;
  pending request or last terminal result and inspected capture; remaining route;
  exact stop/cleanup commands with the same process visibility as launch.
```

Copy the selected JSON examples from the interaction procedure below into the route, including capture and stop, so resumption needs no source lookup. Resolve target coordinates from an inspected client capture; examples are not preapproved click locations. Fill runtime identities as they become available. Preparation grants no desktop access: diagnostic launch, focus, consent and interaction stay inside the approved interval. The owner's ordinary direct application launch remains separate from these diagnostic commands.

For Gathering artwork claims, use the [launch-configuration check](../experiments/gathering-setup.md#artwork-preparation-and-selection) with the handoff's environment and artwork flags. A complete staging collection alone does not establish what this launch will load.

### Public handoff preflight

The public current handoff contains:

```text
Candidate: commit or snapshot digest; reviewed repository-relative file scope.
Completed: check outcomes and dated public evidence links, with evidence limits.
Remaining: acceptance steps and expected visible results; next boundary.
Authority: approved scope and allowance; publication status.
Private evidence: retained locally; runnable checklist supplied directly to owner.
```

Keep host paths, runnable commands containing them, save/artwork locations and raw environment records in the private checklist. Repository-relative source paths and public issue/PR links remain useful public evidence. Before posting a public handoff or PR body, run `scripts/tools check-public-handoff BODY_FILE` and review the intended disclosure. The command reads at most 1 MiB, reports line numbers without echoing contents, and refuses these literal forms: `/home/`, `/Users/`, `~/`, `file://`, `/tmp/`, `/run/user/`, and `local/verification/`. It never edits or publishes. Passing establishes only absence of those forms; encoded paths, other mount points, credentials and prose meaning still require review. A refusal calls for keeping the detail private, not disguising the same disclosure.

### Capture preparation
Before scheduling a capture from a prepared save, load it through the production startup path in the headless UI host and check the expected visible controls and blocking dialogs, alongside the game state. A save containing expanded locations may reopen on an Act story panel; location membership alone does not establish capture readiness. Preserve that reading barrier and prepare an ordinary later decision when the route requires an unobscured map. Record the result in the handoff above. Gathering's `gathering_tabletop::tests::review_saves` checks the seed-89 initial, Hard Knocks and expanded-map inputs, including the obstructing story transition; its retained result is in the [current verification status](../experiments/gathering-setup.md#presentation-correction-verification). These checks do not establish GPU rendering or native readability.

When the owner supplies physical input, present the complete human route, expected results, capture pauses and stopping conditions before starting the timer or opening the game. Once the agreed interval begins, announce the absolute close deadline and let the owner follow that route without switching back to chat for routine instructions or intermediate answers. Request one combined report after the window closes, including anything untested. Interrupt during the run only for a planned capture pause, a failure, a stop condition or a necessary deadline reminder. The existing allowance and cleanup reserve still apply; delivering instructions late does not extend them.

Choose the operator while preparing the route. For interactive review, normally
omit `native launch --timeout-seconds` so the original approval supplies the cap;
use a shorter explicit cap for a bounded automatic capture. Check the first
launch event's `timing.shutdown_unix_seconds` before handing over. If an app cap
ends sooner, announce that earlier time and reduce the route. Restarts inherit
their original app timeout as well as the approval deadline, so a short capture
run is unsuitable as the starting point for an interactive restart. An owner-run
ordinary launch can instead use a prepared isolated command without a diagnostic
timer; this supplies no agent desktop access.

For an animation change, prepare the [short recording recipe](#record-a-short-motion-clip) before the desktop timer starts. Put the loop, reduced-motion and relevant focus-loss/resume cases early in the route, before repeating static layouts. Frame both the moving object and its stationary neighbors, with relevant labels visible; inspect the clip after cleanup when possible. Reserve time for affected fresh-process reopening checks. At a context interruption, recover only the handoff, run manifest and last acknowledged event/capture, retaining the original deadline. An acknowledgement without a terminal result can hide partial input: inspect the existing helper's events before deciding whether any retry is safe. If the remaining budget cannot fit the next operation and cleanup, stop and leave that observation pending. The deadline and cleanup rules below remain controlling.

### Build an instrumented source copy

Before compiling a temporary instrumented source copy, use the isolated build helper. From the normal repository root, build the normal executable if it is absent, then choose a new output directory outside both checkouts:

```sh
scripts/dev scripts/tools native build \
  --source /absolute/path/to/diagnostic-source \
  --output-dir /absolute/path/to/fresh-diagnostic-build \
  --reference "$PWD/target/debug/arkham-tabletop"
```

This runs offline, locked Cargo with a separate target and intermediate build directory, records the build log and hashes the normal executable before and after, including on build failure. Run only the returned diagnostic binary for that experiment. Sharing target output can replace the normal executable, and a subsequent Cargo build can report it fresh without restoring it. A different executable name is insufficient: an instrumented copy can still share the package's library artifacts and contaminate a later normal build. The helper preserves output for inspection rather than cleaning either checkout.

### Launch, capture and interact

Prepare the desktop question with the local helper before asking the user:

```sh
scripts/tools native approval request \
  --session-log /absolute/path/to/current-session.jsonl \
  --minutes 8 --purpose 'check the Gathering controls using disposable saves'
```

The source must be the active session log for this repository. The helper
recognizes its harness from the log's format:

- **Codex:** the session log of the active `CODEX_THREAD_ID` thread. Use the
  returned `request` object unchanged with `request_user_input_async`.
- **Claude Code:** the transcript named `$CLAUDE_CODE_SESSION_ID.jsonl` in
  Claude Code's `projects/<cwd-slug>/` directory, by default under
  `~/.claude`. Its latest recorded working directory must be this checkout.
  Use the returned `request` object unchanged as the `AskUserQuestion` input,
  adding no `answers` or `annotations`. Only one answer to that question is
  accepted.

Retain the returned approval ID as `native_approval`. After the user selects
the affirmative answer, read and announce its original deadline:

```sh
scripts/tools native approval status --approval "$native_approval"
```

This reads the original question call and matching user reply, including the
reply's host-recorded timestamp. The finite accepted reply is the generated
affirmative option; unrestricted prose, notes, old questions and questions asked
by subagents are not interpreted. Unsupported or missing source metadata fails
closed. Codex marks each reply's user provenance. Claude Code records no such
marker: its path relies on the harness writing tool results, the unchanged
question input and main-conversation entries, so a configured hook that answered
`AskUserQuestion` would be indistinguishable from the owner. Request,
status, close and help commands do not access the desktop. Keep the original
source log and local state under `${XDG_STATE_HOME:-$HOME/.local/state}/arkham-unfolded/native-approvals/`.
They are trusted, user-writable local inputs: this guards accidental reuse,
not deliberate tampering by a process with the same filesystem access.

Pass that ID to every diagnostic launch, capture and session. All derive the
same deadline from the original reply; helper restarts, portal grants and later
CLI deadlines cannot renew it. A new interval requires a new question and user
reply. When the interval is finished, permanently close its ID:

```sh
scripts/tools native approval close --approval "$native_approval"
```

Closure is idempotent and survives restarts. Running helpers observe closure
at cooperative checks, release their resources and stop their owned app groups.
Native calls already in progress retain their bounded-call/cleanup limits.

For a reported physical-input failure, establish the failing boundary before repeating gestures or changing camera code:

1. Check whether the **same physical button or key** works in another application. Reuse an observation already supplied by the owner; otherwise ask for one brief comparison. A working left button does not establish that right-button presses reach Arkham. A failure elsewhere identifies a wider input problem, without deciding whether its cause is hardware or desktop software.
2. If the failure is confined to Arkham, distinguish input arrival from input handling. Check window focus and, when needed, use bounded diagnostic logging of cursor movement and mouse-button press/release events at Bevy's input boundary. Missing right-button events and received events rejected by UI/camera capture are different findings. Repeat an owner gesture only when it will distinguish the remaining explanations.
3. When the expected event arrives, exercise the affected UI/camera path. Independently reproduced application defects can coexist with a desktop-input failure. A portal-generated right-drag verifies the synthetic input path only; it does not establish that the physical button works. A failed portal calibration follows the bounded recovery below and is not evidence of a camera defect.

Use the [compact native handoff](#prepare-a-compact-native-handoff) before the timed interval. Within the accepted desktop scope, confirm the intended process and client window before interaction. Budget consent, calibration, navigation, capture inspection and cleanup within the interval, and retain a runnable remaining checklist at handoff.

Use a separate save and temporary settings for diagnostics. The Rust helper below targets the current Ubuntu GNOME/XWayland desktop; capture/input needs access to X11, the desktop session bus and its RemoteDesktop/ScreenCast portal, plus `xdotool` for explicit focus. `scripts/tools` builds only the small `arkham-tools` package. Its headless tests run in `scripts/check`, using a private `dbus-daemon` for protocol tests; no real desktop grant is requested by the tests. From the repository root, build the normal application and launch a disposable run:

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop
mkdir -p "${XDG_CACHE_HOME:-$HOME/.cache}"
review_dir=$(mktemp -d "${XDG_CACHE_HOME:-$HOME/.cache}/arkham-native.XXXXXX")
scripts/tools native launch \
  --approval "$native_approval" \
  --binary target/debug/arkham-tabletop --output-dir "$review_dir/run" \
  --entry selection --art-root /absolute/path/to/complete-gathering-assets
```

The launcher copies and hashes the executable before starting it. Later builds cannot replace that running copy. By default it creates run-local configuration, data, cache and a working directory. Selection creates no Session or save. For a direct Gathering diagnostic, use `--entry gathering`; its isolated `session.json` is created only after successful admission. Adding `--load SOURCE_SAVE` copies the supplied save before loading, because that copy is also the autosave destination. Selection rejects `--load` and `--initial-view` before staging. Original saves and settings stay untouched in this default isolated mode. The output directory must be new and outside the checkout. Use the complete Gathering collection for play and confirm completeness in `run/app.log`. To test refusal, supply a missing or partial folder with an isolated save; do not expect a playable table.

For an approved check of the actual installation, add `--installation normal` and omit `--art-root`. This mode starts selection with inherited normal configuration/data, an isolated cache, and the README checkout as its working directory (or an explicit `--working-directory`). The immutable staged executable runs directly; `run.json` records its identity, actual working directory and effective environment, including HOME for XDG fallback. The helper supplies `--verification-attempts` so older executables reject the unsupported option before opening a window or reaching personal attempts. Artwork import and settings changes affect that normal installation and must belong to the approved route.

Normal mode sets `ARKHAM_VERIFICATION_ATTEMPTS_DIR` to the fresh run's `attempts/` directory. New attempts stay there; Continue and Open admit only those disposable files, protecting personal attempts during this diagnostic route. The helper captures the original reference in a private snapshot in that directory. Before updating Continue, the app checks that snapshot under the ordinary replacement lock: only the original reference or another disposable attempt from this run may be replaced. A newer owner selection remains untouched across repeated diagnostic admissions; a missing or invalid snapshot also prevents the update.

Once the child and its descendants have stopped, the supervisor restores the original Continue bytes only if the current reference still names one of this run's regular disposable files and its predecessor has not drifted. A concurrent different reference is preserved and reported. If a newer owner choice is in the predecessor, both current and predecessor remain untouched for inspection rather than restoring a stale snapshot. The original reference and recovery copy are retained in the private manifest; disposable saves remain evidence. Inspect `normal_installation.continue_restoration` for `restored`, `unchanged`, `preserved-concurrent-change`, `preserved-concurrent-history`, or `failed`. The history outcome requires reconciling the newer choice before treating cleanup as complete. A killed supervisor may leave `pending`; do not infer restoration or repeat effects from an incomplete record. Repair only after confirming the process is stopped and the current reference still belongs to the run. Normal checks require a fresh launch; `native restart` remains for isolated runs and clears the diagnostic attempts override. Neither mode renews desktop consent or its deadline.

Complete preparation before the desktop interval. Run Cargo/build/check jobs one
at a time: await the producer's terminal result before starting the next job.
For a recorded check, require `run.json` to say `finished` with exit code 0 and
unchanged source fingerprints before using its output. `scripts/check` already
uses `scripts/dev`; do not wrap it in another `scripts/dev`. A lock timeout is a
failed invocation, not evidence of a code defect. Diagnose the competing job;
do not widen the lock timeout or retry while that producer is still running.

The Gathering reveal/story and map checkpoint exporters write `launches.json`
and `launches.md` beside the saves. Use those generated disposable-launch recipes
instead of choosing an initial view by hand. Export validates the exact saved
state and arguments through production `Options::parse`, `open_table` and
`configure_diagnostics` before any desktop run. Reveal checkpoints use `test`;
story and map checkpoints use `scenario`. The regular checkpoint test also
rejects the erroneous `resolve` view for an ordinary pre-reveal checkpoint.
This is headless startup validation, not evidence of native rendering or input.

For static layout checks, first agree the desktop interval and follow
[foreground preparation](#foreground-performance-measurements) so the app remains
visible and focused during the timed run. Add
`--initial-view scenario --screenshot scenario.png --quit-after-screenshot` to
`native launch`. This requires `--entry gathering`, with artwork selected
through `--art-root` as above. The normal executable starts at the existing
Scenario focus without pointer input or a source-copy build. `overview`, `test`,
`tony`, `patrice` and `jacqueline` are also supported.

`--screenshot` takes a plain filename inside the fresh `--output-dir`, such as
`scenario.png`; the helper resolves its destination. Before the timer starts,
put the complete command with that filename in the prepared route and check
`scripts/tools native launch --help`. Launch itself remains inside the authorized
desktop interval.

For Gathering's resolution drawer, use `--initial-view resolve` with
`--load /absolute/path/to/pending.session.json`. This uses the same framing as the
in-game **Resolve** button and fails explicitly if the loaded session has no
active drawer. Selection rejects all initial-view options before staging a run. It lets
the static capture show a pending token choice without spending a desktop-control
interval navigating to the drawer.

The chosen `initial_view` is recorded in `run.json`; omitting it preserves normal
startup. This diagnostic option affects the first camera position only: Home
retains Gathering's normal home view, and saves and gameplay are unchanged.
These captures establish rendering evidence; checking focus buttons or camera
gestures still requires the interaction procedure below.

When a capture stalls, read `app.log` alongside the terminal status in `run.json`. Graphics diagnostics report elapsed time and changes in foreground, continuous-update, asset and pipeline readiness, coalesced to at most one condition report per second; unchanged conditions remain quiet. Screenshot diagnostics record request, write-start and writer-return times, with the actual image-write success or failure inside the write context. A request or writer return alone does not establish a saved image. These observations identify the last reported waiting condition without extending the launch timeout or bypassing graphics-health checks; a successful retry does not establish the earlier failure's cause.

Each executable/save copy reaches its final path only after verification. A failed copy removes its own temporary bytes and reports the destination, attempted size and underlying error; source files and earlier runs remain intact. Use a disk-backed destination with sufficient space and user quota: debug executables can be gigabytes, and filesystem free space alone does not establish quota availability. The example uses the user's cache directory; verify its backing filesystem if that location is customized.

The command stays open until the window closes, its requested timeout elapses, or the approval closes or reaches its cleanup reserve. The approval always caps `--timeout-seconds`; staging also consumes its time. Its first JSON output and `run/run.json` identify the original approval deadline, app PID, process start, staged executable hash and exact arguments. In another terminal set `review_dir` to the printed run directory (the directory ending in `/run`) and `native_pid` to the recorded **app** PID. A background launch command's `$!` is the supervisor PID, not the app PID.

Add `--output-format summary` to print compact JSON events with status, exit code, process and executable identity, artifact paths, `launch_purpose` and `timing`. Timing includes the effective shutdown, original approval deadline, cleanup reserve and limiting condition (`app_timeout` or `approval_cleanup_reserve`). Staging consumes the approval budget; verification after spawn consumes the advertised app lifetime. Shutdown is the planned start of cleanup, not a guarantee that a blocked OS call has completed. The exit code is `null` while running. The complete manifest remains in `run.json`; default `--output-format json` preserves full JSON output. Use the summary when reading launch results interactively, and consult the manifest for arguments and launch conditions.

The manifest's `environment` field retains the XDG overrides. `launch_conditions` records the helper's setup actions and allowlisted inherited/effective environment settings; `null` means unset, a string preserves the exact UTF-8 value, and `unix_bytes` preserves non-UTF-8 bytes. These settings are captured before spawn and describe the helper's launch, not a previous owner process. Capture the ordinary baseline separately while it is available, and keep unknown conditions explicit.

Run window discovery, focus, capture, process inspection and cleanup with the same desktop access and process visibility as the launch. Obtain any required tool permission for that context; a missing PID in an isolated shell does not establish that the launched process exited.

Identify visible windows by that PID (`xdotool search --onlyvisible --pid "$native_pid"`) and set `native_window` to the Arkham client ID. Title-only searches can also match editor windows. The helper verifies the exact title, PID, process start identity and focus. It reads client geometry directly from X11; older `xdotool getwindowgeometry` versions can include the frame offset twice.

To focus the verified client and capture its buffer, run the following within the desktop access established above. This changes desktop focus; follow [foreground preparation](#foreground-performance-measurements) before a timed check.

```sh
timeout 10s scripts/tools native capture \
  --approval "$native_approval" \
  --pid "$native_pid" --window "$native_window" --output "$review_dir/client.png"
```

This captures the client buffer through X11 `GetImage`, independently of the app's screenshot path. Inspect the image and reported pixel size. A new filename, timestamp or pixel hash does not establish the expected state: F12 produced new files with stale table contents during #14. Use independent captures before and after a visible transition, and inspect the actual result. Startup `--screenshot` remains a rendering check only.

For interaction, keep one helper process and its input open for the diagnostic,
using the same approval ID as launch:

```sh
scripts/tools native session \
  --approval "$native_approval" \
  --pid "$native_pid" --window "$native_window" --output-dir "$review_dir/interaction" \
  --consent-timeout-seconds 30
```

Use an interactive terminal or a tool invocation with a persistent PTY (`tty: true` for `exec_command`), then send commands through that process's returned session ID. A pipe is suitable only while its writer remains open. Starting the helper with closed stdin cancels consent and input; reopening stdin requires a new helper and grant.

The earliest of the approval deadline, optional `--deadline-unix-seconds` cap and `--timeout-seconds` applies. Startup, consent and idle input consume this same budget. A helper restart reuses the original approval ID and deadline. The helper's `session-budget` event records the approval and effective end time. The final 15 seconds are reserved for releasing buttons and closing the portal grant; each operation needs its full 15-second allowance before that reserve. Late startup or consent closes the helper without further calibration or input. A late command is refused before optional focus acquisition or input. `stop` remains available without a new operation allowance. It closes that portal session; `native approval close` ends the whole interval, including other helpers. `command-started` includes `session_remaining_ms` alongside the request number.

The output directory must be new. The helper saves `initial.png` before requesting consent. In GNOME's dialog, select the monitor containing Arkham, enable **Allow Remote Interaction**, and click **Share**. One nonpersistent, pointer-only grant covers the run. The total session limit is 600 seconds, including consent and idle input; `--timeout-seconds` can shorten it. Keep Arkham covering the monitor's central 10%. Keep the keyboard and pointer idle throughout calibration and gestures. The helper measures the portal-to-X11 mapping and checks it at an independent point; it does not reuse a previous monitor's scale. Wait for `ready`, then inspect `initial.png` before choosing a target. Coordinates are pixels relative to the captured client, not a resized image in the conversation. A calibration refusal retains the session for the bounded recovery below unless its time budget expired.

Send one JSON command per line on the same process's stdin. Each dequeued command emits `command-started` with a sequential `request` number, then a terminal result with that number and `elapsed_ms`. Keep one command outstanding. Each command has a 15-second budget checked before input and between guarded operations; focus and D-Bus calls retain their five-second call limits. A timeout attempts button release, reports `refused`, and ends the session for cleanup. This is cooperative cancellation, so an in-progress native call or cleanup can extend the response beyond the budget. A refusal can follow partial input: inspect before retrying. The input queue holds at most eight pending commands; overflow stops the input reader and cancels portal activity. EOF or a read error during pending consent cancels the request without waiting for approval.

Consent/startup has a separate 30-second limit (`--consent-timeout-seconds`,
1–120 seconds), capped by the original session deadline. A successful grant
disarms only that shorter limit; it does not reset or extend the session budget.
Sending `{"action":"stop"}` cancels pending consent without closing stdin and
interrupts queued or active input at the next cooperative check. It reports
`stopped`, then attempts request/session closure; no calibration follows a
cancelled consent. EOF remains a cancellation route. A consent timeout closes
the affected session and grants no automatic retry or extra desktop time.

Use this compact action-and-capture sequence within the original deadline:

1. Send one action using a target from the last inspected image. Wait for its matching numbered success (`clicked`, `right-dragged` or `wheeled`); `command-started` or a previous request's success is insufficient.
2. Allow the visible transition to render, then send one capture. Wait for that capture's matching numbered success and use its returned image path. A refusal follows the recovery procedure below.
3. Load and return the completed image immediately. When the tool supports sequential orchestration, combine the acknowledged action, capture and image return in one invocation, awaiting each result. Let a pending capture finish before loading its image.
4. Inspect the image before choosing the next state-changing action. Input success alone does not establish the visible result. Retain the existing cleanup reserve; combining tool calls grants no extra time.

For example, send these lines separately at steps 1 and 2, after replacing `X` and `Y` with a target identified in the inspected image:

```text
{"action":"click","focus":true,"x":X,"y":Y}
{"action":"capture","focus":true,"name":"after-result.png"}
```

For one click followed by its inspected result, the same request can capture the
verified client after a bounded delay:

```text
{"action":"click","focus":true,"x":X,"y":Y,"capture_after":{"name":"after-result.png","delay_ms":250}}
```

`delay_ms` defaults to zero and permits at most 12000 ms. The filename is validated
before focus or input. A successful `clicked` result includes the nested `capture`
result and path. If capture or its delay fails after delivery, `refused` includes
`input.status: clicked` and `capture.status: failed`. Capture again or inspect the
retained evidence; never repeat that click to repair a screenshot failure. Input,
delay and capture share the original command deadline and cleanup reserve. A
capture is a sampled frame, not proof that an animation has finished; use the
route's expected state and short recording to judge completion.

For an orbit check, choose a start and end on exposed table space in the inspected capture, then send a bounded right-drag. For example, after checking that these client coordinates are appropriate:

```text
{"action":"right-drag","focus":true,"x":800,"y":450,"dx":120,"dy":30,"duration_seconds":0.6}
{"action":"capture","focus":true,"name":"after-orbit.png"}
```

`dx` and `dy` are displacement in client pixels. The nonzero gesture is limited to `abs(dx) + abs(dy) <= 600`; `duration_seconds` defaults to 0.6 and accepts 0.2–2.0 seconds, with 20 movement steps. Desktop verification adds time to the requested movement duration. The helper validates the whole path before pressing, rechecks the window and measured pointer during movement, and releases the right button on failure or interruption. A path that fails preflight sends no press.

For a menu-overflow check, target the visible menu's scroll area in the inspected capture, then send a small vertical wheel input. After verifying these example coordinates:

```text
{"action":"scroll","focus":true,"x":400,"y":300,"direction":"down","steps":1}
{"action":"capture","focus":true,"name":"after-wheel.png"}
```

Use `direction: "up"` or `"down"` with 1–10 positive `steps`. The helper translates down to positive portal steps and up to negative steps. The legacy `wheel` action still accepts signed steps with those meanings. Start with one step and inspect the result. The helper validates the target and rechecks the pointer, focus, identity, geometry, held buttons/modifiers and deadline before sending input; it holds no button while scrolling.

Apply the same numbered acknowledgement and capture sequence to orbit and wheel checks. Compare the captures to establish the intended movement or scrolling. The helper checks process identity, focus, unchanged geometry and the measured client pointer position immediately before each press or wheel event. It refuses off-window, off-monitor and mismapped targets. It releases an attempted press even when interrupted.

Switching to a terminal or agent interface can take focus from Arkham between commands. Set `"focus":true` on the operation to reacquire the verified client immediately before its guards and input, within the same request. The default leaves focus untouched; `{"action":"focus"}` remains available for focus alone. Reacquiring focus preserves all input guards, including refusal if focus changes during a drag. Send the gesture first, let the application render, then request the capture; a capture queued immediately after release can still show the preceding frame. Inspect pixels rather than interpreting an intended-state filename as proof.

An ordinary `refused` command keeps the grant open; insufficient remaining time, command timeout or failed button release ends it. Diagnose once within the surviving session; `{"action":"calibrate","focus":true}` reacquires focus and remeasures a changed monitor mapping. Inspect a new capture before retrying a target. Stop after one failed capture/focus/click/drag/wheel attempt plus at most one bounded diagnosis attempt. If interaction remains unavailable, report the limitation and use the already-agreed owner check within its remaining allowance. Do not interpret a stale image as evidence against what the owner sees on the desktop.

Send `{"action":"stop"}` (acknowledged by `stopped`) or close stdin when finished, and wait for `session-closed`. Cleanup errors emit `session-close-failed` with a reason and return a failing exit status; report that result instead of claiming successful closure. An already absent session is accepted only for `UnknownObject` or the observed GLib `UnknownMethod` reply naming that exact missing session; other errors remain failures. EOF, fatal errors and handled interruption attempt to close the portal grant; a killed or crashed process loses its session when its D-Bus connection closes. `events.jsonl` records capture paths, pixel hashes, mapping, command acknowledgments, elapsed times, refusals and closure. It contains no artwork itself; keep it with the local captures.

The cleanup reserve reduces normal overruns but cannot guarantee interruption of a blocked native call. Inspect the actual closure timestamp and report any overrun of the announced end time. A timeout during calibration ends the session as well.

For folder persistence, exercise **Choose asset folder → check → Save folder → close**. After the supervisor exits, restart from its completed manifest:

```sh
scripts/tools native restart --run /absolute/path/to/original-run/run.json \
  --output-dir /absolute/path/to/new-restart-record --output-format summary
```

The restart reuses the verified staged executable, working directory, XDG directories and recorded entry target. It omits `--art-root` and diagnostic startup framing/capture flags so the app reads its saved settings. Every new manifest records `entry_target`; restart refuses historical manifests without it, retired targets and conflicts with the recorded arguments. Selection restarts without `--save`, including when no attempt was created. For a direct Gathering diagnostic, if the manifest records no game save at either startup or exit, restart reuses that unsaved path with `--save`; saving an artwork folder alone does not require saving a game. A previously present save must remain present.

The new directory holds this supervisor's log and manifest; the original manifest remains unchanged. The original approval and deadline still apply: expired or closed approval, changed executable, a lost save, an unfinished run or a concurrent restart is refused. Closing the original approval also stops a running restart. Use a new launch under separately accepted desktop time if the original interval has ended.

Verify that the saved path and asset status are restored. Capture helpers must use the new PID from the restart record. A screenshot establishes the inspected rendering; the actual interaction and restart establish their separate behavior. Any review of the owner's real preference uses its explicitly agreed scope instead of these diagnostic settings.

Close the verification window normally and let its supervisor report the exit. If cleanup is needed, interrupt that supervisor. Stop only the identified launch and helpers. Keep logs/captures local; record the revision, command, outcome and [retained evidence location](#retain-final-verification-evidence) in the active issue. Remove temporary files only after the needed evidence has been retained.


Selection-to-play and Return-to-selection replace the process image to respect the native event-loop lifetime. The PID can remain the same while its client window changes. A capture/input helper bound to the previous window must refuse it. Stop that helper, discover the new client, and create a new guarded session under the same approval and original deadline before further input. Consent and rediscovery consume that interval; do not bypass the window-identity checks or extend the deadline.

### Record a short motion clip

Prepare this branch only when the accepted route needs motion evidence. Before
desktop access, check `ffmpeg -hide_banner -h demuxer=x11grab`,
`ffmpeg -hide_banner -h encoder=libx264` and `ffprobe -version`; prepare fresh
clip and check-record names. The tools must support the options below. A
synthetic encoding probe can check the codec and output settings without opening
the desktop; it does not establish native capture or animation quality.

During the agreed interval, use the app PID, client window and run directory
from [launch and interaction](#launch-capture-and-interact). Start this sequence
only with at least 60 seconds left before the original absolute deadline.
Recheck the client and focus through the existing guarded capture:

```sh
timeout 10s scripts/tools native capture \
  --approval "$native_approval" \
  --pid "$native_pid" --window "$native_window" --output "$review_dir/motion-before.png"
```

Require success and inspect that frame before recording. Confirm the intended
motion is available, the window geometry is stable, and at least 45 seconds
remain. Keep the verified window ID: `0` selects the whole desktop in FFmpeg
and is not a permitted substitute. Record only that client, with no audio:

```sh
scripts/tools record-check --output-dir "$review_dir/motion-normal" -- \
  timeout --signal=INT --kill-after=2s 12s \
  ffmpeg -hide_banner -loglevel warning -nostdin -n \
  -f x11grab -framerate 20 -draw_mouse 0 -window_id "$native_window" -i "$DISPLAY" \
  -t 6 -an -vf 'pad=ceil(iw/2)*2:ceil(ih/2)*2' \
  -c:v libx264 -preset ultrafast -crf 18 -threads 2 -pix_fmt yuv420p \
  "$review_dir/motion-normal.mp4"
```

This requests six seconds of video; the external timeout bounds a stalled
encoder and leaves cleanup time. The capture must remain inside the original
desktop interval. Keep normal-focus motion separate from a deliberately
unfocused comparison. Record any focus operation, geometry change or obscured
window; stop an invalid capture instead of substituting a different window.
Use fresh names for any separately budgeted reduced-motion or resume case.
Neither finishing a clip nor a failed capture renews the interval.

After cleanup, require the recorder's `run.json` to be `finished` with exit 0
and inspect its complete log. A timed-out or interrupted file remains partial
evidence. Inspect the clip itself as well as its stream metadata:

```sh
ffprobe -v error -select_streams v:0 \
  -show_entries stream=width,height,avg_frame_rate,nb_frames:format=duration \
  -of json "$review_dir/motion-normal.mp4"
```

For Gathering's 2.4-second reminder, a six-second clip should expose more than
two cycles, with the button and adjacent panel stationary. Metadata alone does
not prove advancing frames, smooth pacing or readable content. Describe the
motion actually observed; pleasantness and owner acceptance remain separate.

### Retain final verification evidence

Before a handoff relies on local evidence, copy its final records into a fresh
`local/verification/<commit-or-review-digest>/<task>/` directory. `local/` is
ignored by Git. Native launches still use fresh staging directories outside the
checkout; retain their evidence here after their processes and grants close.
The directory name does not replace the recorded candidate and executable hashes.

Keep the completed check manifests and full logs, review snapshot and report,
native captures/recordings and input events, and the input saves/settings needed
to understand the inspected state. Retain a staged executable when the comparison
depends on those exact bytes; one copy is enough when all recorded hashes match.
Leave build caches and redundant executables disposable, and record omissions.
Follow the [source contract](../foundation/sources-and-verification.md) for private
inputs and artwork; local retention does not grant publication permission.

Verify the selected copy by content comparison and preserve its recorded hashes.
For example, a checksum dry run with `rsync --dry-run --checksum --itemize-changes`
and the same selection should report no file-content differences. Check the
recorded SHA-256 values for the executable and review manifest, and read the
copied run results before claiming retention succeeded. Preserve original
records unchanged; a small README maps their old absolute paths to retained
copies and lists any omissions. Link this location in the private runnable checklist; the public current handoff carries the candidate identity and public evidence links.
Keep it while acceptance or later comparisons rely on it; a temporary directory,
cache location or conversation attachment is not the sole retained copy.

Protocol references: [RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html) and [ScreenCast](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html). Support and evidence remain limited to the desktop and journey actually exercised.

### Foreground performance measurements

Before a timed check, establish who will keep the app visible and focused. Within an authorized desktop interval, use the verified PID/window and existing focus support: `native capture` focuses its client, and a running native session accepts `{"action":"focus"}` or `"focus":true` on an operation. Wait for that operation's result before selecting the measurement interval. When the check needs the owner's attention, establish readiness before launch instead of sending a focus request during a short countdown. Record any capture or focus operation as part of the comparison's conditions.

To measure tabletop cadence, launch with the pacing counter and keep the log:

```sh
RUST_LOG=info,wgpu=error,naga=warn,arkham_unfolded_experiment::tabletop_pacing=debug \
  target/debug/arkham-tabletop 2>&1 | tee /absolute/path/to/new-launch.log
```

`RUST_LOG` replaces Bevy's default filter, so the command restates those defaults. The target names the library crate, not the package or executable. Each `Tabletop updates: N in Xs; longest gap …` line covers one interval of main-app updates, not GPU presentation timestamps; `Tabletop cadence: idle|active` marks transitions.

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

Before a native handoff for card or artwork presentation changes, inspect the
affected prepared images and available renderer captures together: table lighting,
front crop bounds, deck/installed backs and the inspection modal. Check complete
borders without page strips or adjacent printings, intended orientation and aspect
ratio, and rounded inspection corners consistent with the 3D card. Retain the
images with their candidate/recipe identities and give them to reviewers alongside
the corresponding production components. For coordinated Netrunner work, compare
both consumers' inspection paths and shared card geometry. Hashes and successful
uploads prove identity/loading, while these images support only the visual states
shown; native input, readability in use and owner acceptance remain separate.

Before implementing a scenario, mode or tabletop UI change, identify the accepted presentation it inherits and its revision. Account for every affected section in the existing issue or PR: investigator counters, card captions, contextual and global controls, portrait/camera navigation, inspection, and deck/pile rendering as applicable. Name the production component paths and the behavior to preserve. Reuse the established implementation where it already serves that behavior; explain a necessary local adapter or an owner-authorized difference. A new scenario alone is not a reason to redesign the player's interface. Bring proposed changes to accepted behavior to the owner with their consequences before adopting them.

For repairs after a native check, retain the reference capture and its candidate identity (commit or working-tree snapshot), viewport/camera, populated state and relevant board/card/text dimensions in the existing issue handoff. Distinguish owner-accepted constraints from agent observations and deferred preferences. Supply this reference alongside the Git fixed point in later reviews, even when it comes from earlier uncommitted work in the same session. Compare repairs at comparable framing and assess tradeoffs against the owner's priorities: a passing collision check alone does not justify larger boards, smaller cards or text, or wider camera framing. An unavailable reference leaves that comparison unverified; source checks retain their narrower meaning.

For a caption-construction refactor, characterize the affected representation before moving it. Follow [`tabletop_pieces::label`](../../src/tabletop_pieces.rs) through [`tabletop_harm_text::ui`](../../src/tabletop_harm_text.rs), including any caller edits afterward. Exercise the required font's glyph spans and accessible English labels, including separators and multi-field status text. Missing-font checks must assert startup refusal, not a substitute presentation. A root `Text` assertion alone can miss glyph substitutions or retained spans. Reuse the [Gathering caption tests](../../src/gathering_tabletop/tests/cards.rs) that exercise the production UI where applicable. Record inherited artifacts separately from authorized corrections; these checks establish scene representation, while native readability and physical input retain the evidence requirements below.

Before handing a tabletop change to Joel, review the affected player journey through the native window, including existing cards and panels encountered along the way. Use the verification procedure above and record the states and viewport exercised. Apply the relevant checks below; a local change does not require replaying unrelated cases.

- **Card inspection:** original faces are upright in their intended orientation, including landscape investigators. Printed text is readable in inspection; scrolling and Close remain usable at the exercised window size. Inspect the real artwork and meaningful text when their layout is affected.
- **Existing visual behavior:** for placement changes, identify the objects authorized to move and the neighboring anchors to preserve from the owner's request and accepted baseline. Compare before and after at a comparable camera and viewport, checking the requested gap against rendered labels and card edges. Keep preserved anchors in place. When introducing an already represented object, also compare token thickness, face artwork, label backing, colors and card orientation with the established experiment with the complete prepared artwork. Preserve the useful existing behavior or explain an authorized intentional difference in the PR; two implementations alone do not justify two visual standards.
- **Populated layouts:** inspect sparse and crowded arrangements at the intended camera and viewport. Check adjacent region borders, captions, card row counts and usable space against the accepted use. Use the existing capacity fixture where applicable and report the occupancy inspected; a fixture establishes only those arrangements, while new capacity preferences remain separate from defects.
- **Decision states:** committing a card, spending a reaction and canceling or resolving a token are visibly distinguishable. Follow the transition as well as the final state.
- **Rules wording:** headings identify recognizable cards or events. Explanations connect modifiers, extra reveals and consequences to their cause. Distinguish an unchanged total from a new effect; wording such as “now has” must correspond to a change. Judge these statements against the accepted rules and recorded events.
- **Results and continuity:** the outcome and its consequences are easy to find. Dismiss and reopen inspection/results along the affected path; settled pieces stay settled, an animation already in progress continues, and panel actions preserve game state. Reopen the pending decision in a fresh process and check that continuation applies each effect once.

Read captures at the displayed scale and use actual interaction for movement and continuity claims. Report correctness and readability defects separately from preferences such as centering, decoration or reminder size; explain a preference's effect instead of making it a universal rule. Identify any unavailable interaction as unverified and follow the bounded fallback above. Tests, captures and reviewer judgment retain their separate evidence limits; Joel's acceptance remains his decision.
