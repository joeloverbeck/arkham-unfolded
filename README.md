# Arkham Unfolded

An English-language Arkham Horror: The Card Game application for Ubuntu, implemented in Rust with a 3D tabletop.

**The Gathering is in development; the campaign is not complete.** Scenario selection offers only original The Gathering on Standard with the selected Tony Morgan, Patrice Hathaway and Jacqueline Fine starting decks. Full native scenario play and owner acceptance remain outstanding under [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). The [Gathering guide](docs/experiments/gathering-setup.md) records implemented behavior and its evidence limits.

## Launch and resume The Gathering

From this checkout:

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop
```

The first build can take several minutes. Ordinary launch opens **Scenario selection** without loading a game or requiring prepared artwork. Select **The Gathering** to choose:

- **New attempt:** allocates a distinct unused save path. It creates the attempt only after complete artwork and rendering admission, preserving existing saves.
- **Continue:** opens the exact most recently successfully admitted attempt, remembered across application launches. With no remembered attempt, the screen explains this and keeps New/Open available.
- **Open saved attempt:** choose an existing file with the native file picker. Cancellation keeps the previous Continue reference. Missing, corrupt, incompatible or wrong-scenario records are refused unchanged.

New saves live under `${XDG_DATA_HOME:-$HOME/.local/share}/arkham-unfolded/attempts/`. The remembered path is separate from game state, beside `gathering.toml` in the user's configuration directory. **Menu → Save and return to scenario selection** saves the pending decision before leaving; a failed save keeps the game active with its recovery controls.

**Complete artwork is required for 3D play.** Gathering verifies all 145 required files, loads its images and symbol font, then checks rendering before enabling decisions. Missing or damaged artwork offers **Choose asset folder**, **Retry** and **Quit**, with the diagnosis repeated in the terminal. Startup refusal preserves existing saves and creates no new attempt. Headless rules and engineering checks remain independent of artwork. [ADR 0004](docs/adr/0004-require-artwork-for-3d-play.md) records that contract.

Gathering uses `~/.local/share/arkham-unfolded/gathering-assets` by default, or the corresponding XDG data location. Its saved folder preference takes precedence; `--art-root PATH` is a temporary override. The v4 collection adds the investigator ability spark; earlier collections require the documented update. Existing saves and `gathering.toml` keep their formats. See [artwork preparation and selection](docs/experiments/gathering-setup.md#artwork-preparation-and-selection).

Keep the window visible and focused during **Checking graphics...**. New play begins at **The Gathering · Standard · Setup**. Click an investigator card to choose the lead, then select mulligan cards and confirm each hand. Decisions autosave to the attempt's path. Card and pile menus provide Inspect and available actions; the portrait rail moves the camera independently of the current investigator. Tab/Shift+Tab and Enter/Space navigate buttons; hold WASD to pan outside blocking dialogs.

**Menu → Restart The Gathering... → Restart** archives a valid predecessor beside its save and starts fresh shuffles at that same path. Cancel or Escape preserves the current attempt. Restart is separate from New's distinct save and cannot repair an unreadable record. Manual Save, Reopen saved attempt, History and artwork settings remain available. See [restart and recovery](docs/experiments/gathering-setup.md#restart-and-recover-an-attempt).

### Direct diagnostic launch

An explicit Gathering launch remains available for a known checkpoint:

```sh
target/debug/arkham-tabletop --experiment gathering --save /absolute/path/to/attempt.json
```

Here an existing compatible file resumes; an unused path starts setup. Omitting `--save` on this direct command selects `local/gathering/setup.session.json`. Ordinary launch without `--experiment gathering` opens selection instead. The headless driver also retains Gathering's player choices, persistence and diagnostics; use `scripts/dev cargo run --locked --bin arkham-driver -- --help` for its interface.

### Start fresh after an incompatible save

Select **New attempt**. Keep the rejected file; no migration or repair is performed. Replay-era format-1 records are unsupported, and older format-3 records can already be incompatible with the current Gathering implementation. The error `Gathering setup does not match its recorded decisions and pinned inputs` means validation could not reconstruct that record, including its recorded history. This retirement change preserves saves loadable at its baseline; it does not repair previously incompatible records. See [replay retirement](docs/adr/0003-retire-replay.md) and [earlier compatibility notes](docs/research/gathering-wagers-keys-and-token-layout.md#implementation-and-verification).

### Retired experiments

The standalone Jacqueline, Tony and prepared Study escape experiments were retired on 2026-11-07 under [#127](https://github.com/joeloverbeck/arkham-unfolded/issues/127) and [ADR 0005](docs/adr/0005-retire-experiments-and-select-scenarios.md). Their selectors and save readers are unavailable. Personal files remain untouched; their [Jacqueline](docs/experiments/jacqueline-curse.md), [Tony](docs/experiments/tony-turn.md) and [Study escape](docs/experiments/study-escape.md) observations remain historical evidence. Shared verification instructions now live in the [native-verification runbook](docs/agents/native-verification.md).

## Project aim

The aim is to let one player control their investigators while the program handles setup, legal actions, timing, card effects, tokens, movement, and campaign bookkeeping. Cards, decks, and tokens should retain the physical presence of a tabletop game. Rules explanations will be available through an inspectable action history.

## First complete delivery

Play the original **Night of the Zealot** campaign from beginning to end on **Standard**, controlling these three investigators together:

| Investigator | Starting ArkhamDB deck |
| --- | --- |
| Tony Morgan | [5392133](https://arkhamdb.com/deck/view/5392133) |
| Patrice Hathaway | [5391062](https://arkhamdb.com/deck/view/5391062) |
| Jacqueline Fine | [5389418](https://arkhamdb.com/deck/view/5389418) |

The delivery includes normal campaign consequences and progression, upgrades made in ArkhamDB, save/resume, and inspectable action history. Bounded investigations can establish feasibility; they do not count as delivering this campaign. The earlier standalone experiments are retired.

Replay was removed before experiment retirement, including playback controls, driver commands, recording APIs and stored historical observation tapes. Saves from pre-removal builds are unsupported: select **New attempt** to start again. Old personal files remain untouched. New saves retain exact continuation, action history, result details, safe Undo and restart archives. See [ADR 0003](docs/adr/0003-retire-replay.md) and the [implementation evidence](docs/research/replay-removal.md#implementation-result). Gathering retains a private compact decision journal for validation and safe Undo; it has no playback interface.

Content support follows the owner's actual use. Starting decks, campaign content, and selected upgrades and weaknesses must work before a play session begins. This is not a promise to support the entire published card pool. Public development and possible YouTube session recordings do not change that scope.

## Project documents

- [Mission and scope](docs/foundation/mission.md)
- [Play contract](docs/foundation/play-contract.md)
- [Sources and verification](docs/foundation/sources-and-verification.md)
- [Prepared asset collection decision](docs/adr/0001-prepared-runtime-assets.md)
- [Tabletop presentation contract](docs/tabletop-presentation.md) — current layout, interaction, resolution and animation requirements; [ADR 0002](docs/adr/0002-persistent-table-areas.md) explains the spatial decisions.
- [Issue #1 feasibility assessment](docs/research/issue-1-assessment.md) — accepted source baseline and evidence for the selected bounded experiment
- [Gathering play and verification](docs/experiments/gathering-setup.md) — current scenario under [issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89)
- [Scenario selection and experiment retirement](docs/adr/0005-retire-experiments-and-select-scenarios.md)
- [Native verification](docs/agents/native-verification.md) — shared launch, consent, capture and evidence procedure
- [Repository principles](docs/principles/principles-for-repos.md)

[GitHub Issues](https://github.com/joeloverbeck/arkham-unfolded/issues) and pull requests are the work-control plane. Foundations record the intended product and accepted decisions; they are not a second work queue. `CLAUDE.md` contains the agent bootstrap; both `AGENTS.md` and `AGENT.md` link to it.

## Current use and external inputs

### Local builds and checks

The built application runs offline. A clean clone needs no artwork or credentials for headless engineering checks. Complete prepared artwork and successful decoding are required for 3D play. Run `scripts/check` for tracked-file hygiene, formatting, strict Clippy, and the Rust tool, rules, continuity and tabletop tests. Rustup uses the compiler and components in `rust-toolchain.toml`. The Ubuntu PR workflow runs the same phase launchers as named steps without a display or private assets; the portal transport tests start a private `dbus-daemon`. CI compiles its small Rust toolchain-preparation entry point directly with `rustc` before restoring Cargo's cache; the full tools package builds afterward.

On Linux x86_64 and aarch64, the file-replacement lock test uses `ptrace` to observe its own driver child. The test environment must permit this child tracing. If a restricted agent or container sandbox denies it with `Operation not permitted`, rerun the unchanged check in an authorized environment that permits child tracing. Successful verification requires the tracing test to execute and pass.

Local development defaults to two Cargo jobs. Development and test builds keep file-and-line backtraces for workspace code and omit dependency debug data, reducing the memory and disk load of linking the tabletop test suite. For full variable and dependency debugging, opt in with `scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop --profile debug-full`; its executable is `target/debug-full/arkham-tabletop`.

`scripts/check` runs its phases in a transient user systemd scope: 12 GiB memory pressure threshold, 16 GiB hard memory limit, 1 GiB swap limit, six CPU equivalents and CPU weight 20. Use `scripts/dev COMMAND [ARG...]` for other local builds and individual checks, for example:

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop
scripts/dev scripts/check-app
```

Protected commands share a lock on this checkout's directory. A second command waits up to 30 seconds, then reports `flock: timeout while waiting to get lock` on stderr and exits with code 75 if the checkout is still busy; retry after the active build finishes. Lock diagnostics stay on stderr, preserving the command's stdout (including Cargo JSON) and exit status. Invoke `scripts/check` directly: it already uses `scripts/dev`, so wrapping it again would wait on its own lock. Separate checkouts have separate locks. Run the built tabletop executable separately so compilation limits and the lock do not affect play.

The repository's [VS Code settings](.vscode/settings.json) route rust-analyzer's diagnostics and build-script/procedural-macro compilation through the same launcher, with the tabletop feature enabled. These use rust-analyzer's [command overrides](https://rust-analyzer.github.io/book/configuration#rust-analyzer.check.overrideCommand). A busy editor check can be rerun with **rust-analyzer: Run flycheck** after the full check finishes; other editors need equivalent local configuration.

The launcher requires `flock` and a running user systemd manager; a failed scope launch stops the command without an unlimited retry. CPU and memory controller delegation is a host prerequisite: verify the effective cgroup limits when setting up another host, since accepted systemd properties alone do not prove enforcement. A hard-limit failure is a failed check, never permission to retry without limits. Direct Cargo commands retain the job/debug defaults but do not get the systemd limits or checkout lock. CI continues to run the named phase launchers under its existing runner settings.

A restricted agent can be denied access to the user session bus even when the user manager is running. `Failed to connect to user scope bus via local transport: Operation not permitted` means the launcher failed before the check started. Retain the failed record, then rerun the unchanged command with authorized host access to that bus, keeping `scripts/dev` and its limits in place. Use a fresh `record-check` output directory and require a finished, successful record before treating the check as passed.

For a retained check with resource measurements, prepare the small helper under the limits, then run its recorder outside the build scope:

```sh
scripts/dev cargo build --locked -p arkham-tools &&
target/debug/arkham-tools record-check --resources \
  --output-dir /absolute/path/to/new-check-record -- scripts/check
```

The output directory must be new and its parent must exist. `run.json` retains the command's status and exit code; `resources.jsonl` samples host memory, CPU counters and pressure plus the child's visible cgroup about once per second. Kernel values retain their original names and units. Each reading exposes unavailable files as errors; `resources.error` reports capture/write failures without changing the command's exit status. Sampling runs inside the recorder, so completion or interruption leaves no separate sampler process. Only a finished record with exit code zero proves check success. Resource samples can miss short spikes or the last counters before a scope disappears; an observed `memory.peak` is not a guaranteed final peak. An abruptly killed recorder leaves incomplete evidence.

In a Git checkout, the recorder fingerprints tracked and non-ignored untracked files before and after the child runs, including dirty contents, missing files, permissions and symlink targets. The record directory itself is excluded. Changed inputs set `status` to `invalidated` and make a successful child return failure from the recorder; the original child `exit_code` is retained. Fingerprint errors produce `incomplete` evidence. Outside Git, the source fields are null. These endpoint fingerprints do not detect edit-and-revert during a run, follow external symlink contents, or cover ignored build inputs; keep the candidate stable throughout verification. Submodule entries are refused rather than silently omitted.

After a recorded check fails, `record-check` automatically prints diagnostic excerpts using `check-summary`, with a 6,000-byte combined budget for paths, recorder notices and excerpts. It preserves the child's exit status, the complete log and `run.json`; unavailable excerpts do not replace the check result. Successful checks print their retained paths and any resource-capture warning.

For a running or apparently stalled check, inspect its retained record first, then bounded diagnostic excerpts. `check-summary` limits both individual lines and total output, reports omissions, and links the unchanged full log. Its recognized compiler/test/formatting markers are navigation hints, not a success verdict; unrecognized output gets a bounded tail. If the record is still running, use its exact `pid` with compact process columns; replace `RECORDED_PID` below with that number. This avoids dumping wrapper arguments or unrelated processes:

```sh
cat /absolute/path/to/new-check-record/run.json
scripts/tools check-summary /absolute/path/to/new-check-record/check.log --max-bytes 6000
ps -p RECORDED_PID -o pid,etime,stat,comm
```

A live PID shows process presence, not check success. Use a finished `run.json` even when the tool session's terminal status was lost; losing that status alone does not justify rerunning unchanged work. If the process has gone away without a finished record, retain the result as incomplete.

For a behavior-preserving refactor, run focused characterizations before moving production code. If a later comparison needs a temporary baseline tree, record its revision and the test/helper files copied onto it. Moving the tree also moves its vendored renderer dependency; sharing `target/` alone does not preserve that dependency's source identity.

When the dependency inputs are identical, the baseline can use the original checkout's renderer path. First require `git diff --exit-code BASE CANDIDATE -- Cargo.toml Cargo.lock .cargo/config.toml rust-toolchain.toml vendor/wgpu-hal` to exit zero with no differences, and verify the temporary tree retains those inputs. If any differ, use the baseline's own dependencies and a separate target directory. Keep the toolchain, features and profile consistent with the comparison being made.

For the unchanged-dependency case, create `/tmp/baseline-checks/` and put this override in `/tmp/baseline-checks/canonical-renderer.toml`, replacing the checkout path if needed:

```toml
[patch.crates-io]
wgpu-hal = { path = "/home/joeloverbeck/src/arkham-unfolded/vendor/wgpu-hal" }
```

Run from the original checkout so both builds use its launcher lock. Replace the temporary baseline path and test filter with the prepared tree and focused characterizations; use fresh record directories for each attempt:

```sh
scripts/tools record-check --output-dir /tmp/baseline-checks/baseline -- \
  scripts/dev cargo test --locked --offline \
  --manifest-path /tmp/caption-baseline/Cargo.toml \
  --target-dir /home/joeloverbeck/src/arkham-unfolded/target \
  --config /tmp/baseline-checks/canonical-renderer.toml \
  --features tabletop --lib gathering_tabletop::tests::cards \
  --message-format=json-render-diagnostics
```

Require a finished, successful baseline record before returning to the candidate. Run the same command with a fresh output directory and without `--manifest-path` and `--config` to check the original checkout. Inspect Cargo's retained `compiler-artifact` records and their `fresh` fields for the renderer dependencies; a shared path is not a cache-hit guarantee. The first baseline build may still recompile them. Keep these large JSON records on disk and use bounded reads rather than printing whole artifact lines. This comparison establishes the selected tests' behavior and observed cache reuse; it does not replace the candidate's required checks or native acceptance.

Repository helpers live in the independent `arkham-tools` Cargo package. `scripts/tools --help` lists the bounded reader, Git-index guard, native verification and profiling commands; building these helpers does not compile Bevy. The launcher isolates its build artifacts from application compilation so context reads can proceed during a build, while Cargo still rebuilds changed helper sources. Its invocation parameters leave the caller's build-directory environment available to commands such as `record-check`. The index guard rejects generated caches and recognizable non-Rust executable source/template filenames, including force-added files. Inline code, templates and invocation-only shell remain subject to review under the [implementation-language contract](docs/foundation/mission.md#implementation-language). See the experiment guide for [native verification](docs/agents/native-verification.md#developer-native-verification), external inputs, records and evidence limits.

The application reads a separately configured prepared asset collection. Image and font bytes are not included in Git. The [selected artwork contract](docs/adr/0004-require-artwork-for-3d-play.md) requires complete artwork and successful loading before 3D play, with informative refusal and recovery instead of substitutes. Absent or unsupported rules behavior must also be identified before play. The [source contract](docs/foundation/sources-and-verification.md#presentation-asset-preparation) requires deliberate selection, copying/conversion and verification whenever development introduces a new asset dependency. The original archive's layout and source identities belong to that preparation step.

Once required game data is imported, gameplay and saves work offline. ArkhamDB remains the deck editor; imports and update checks require network access. Personal saves, captures, and credentials remain local and outside Git.

## Attribution

Arkham Horror: The Card Game is published by Fantasy Flight Games. This is an unofficial independent project. Third-party game content and artwork remain distinct from the project's original code.

The reveal control uses [Swap bag by Lorc](https://game-icons.net/1x1/lorc/swap-bag.html), under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). Its white transparent PNG is copied unchanged into the external prepared collection; any on-screen color is a presentation tint. Source identity and attribution are also recorded in each asset recipe and prepared manifest.

The public [Haskell implementation](https://github.com/halogenandtoast/ArkhamHorror) is a secondary implementation reference. Official game sources and independently established expectations determine intended rules behavior.

Original project code is available under the [MIT license](LICENSE). This does not grant rights to the game's artwork, card text, rules publications, or other third-party material. The copied repository constitution remains separately identified governing text.
