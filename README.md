# Arkham Unfolded

An English-language Arkham Horror: The Card Game application for Ubuntu, planned with a Rust rules core and an animated 3D tabletop.

**Status: bounded Rust/3D experiments. The campaign is not playable yet.** The original Jacqueline/Patrice skill-test mode remains available. Tony mode offers an owner-accepted complete turn from a prepared Gathering position under [#22](https://github.com/joeloverbeck/arkham-unfolded/issues/22).

See the [Tony guide](docs/experiments/tony-turn.md) for the prepared position, saves, undo, replay and player driver, including the [two-session verification walkthrough](docs/experiments/tony-turn.md#owner-verification).

The candidate [prepared Study escape attempt](docs/experiments/study-escape.md) under [#55](https://github.com/joeloverbeck/arkham-unfolded/issues/55) adds coordinated turns for all three investigators and an Act 1 closing decision. Select it with `--experiment party`; its owner acceptance is pending.

## Launch and play the Tony turn

On this Ubuntu machine, open **Terminal** (`Ctrl` + `Alt` + `T`) and paste:

```sh
cd /home/joeloverbeck/src/arkham-unfolded
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony
```

The `--experiment tony` option selects the new turn and its own artwork preference. Without a saved preference, it uses the current Tony collection below. The terminal reports the experiment, resolved artwork folder and completeness at startup. If artwork has not been prepared, use the preparation instructions below; missing artwork leaves readable substitutes.

All three native experiments check rendering before enabling play. Keep the window visible and focused while **Checking graphics...** is shown. A healthy scene normally needs several seconds; startup refuses play if it cannot become ready within 30 seconds of visible foreground operation. A later sustained rendering failure also stops play and exits unsuccessfully. The terminal records the reason, and the last saved decision remains intact. This checks observed progress; it does not repair or certify the graphics driver. See the [graphics investigation and evidence limits](docs/research/nvidia-mapping-and-startup-health.md).

On Linux/Vulkan with the observed NVIDIA RTX 2080 Ti, internal upload buffers use system memory to avoid a measured stall when the CPU writes into GPU BAR memory. This [small patch to the pinned renderer dependency](vendor/wgpu-hal/ARKHAM_PATCH.md) applies automatically to all three experiments; use the ordinary launch commands below.

If your checkout is elsewhere, replace the directory in the first line. This requires Rust/Cargo 1.97.1 or later and an Ubuntu graphics session with X11/XWayland and Vulkan. The first launch downloads and compiles dependencies, which can take several minutes. Later launches reuse the build. Keep the terminal open while playing. The native window retains the title **Arkham Unfolded — Scripted Rules Experiment**; the heading inside must say **Tony turn / The Gathering / Standard**.

The shared scenario surface holds the map and test area. Tony, Patrice and Jacqueline have separate personal surfaces in a centered row, with hands along their near edges. The portrait buttons focus each investigator; **Scenario**, **Test** and **Overview** move between the shared areas and the whole table. Focusing an investigator does not change who is acting. The current actor, decision and test stay visible while you navigate.

In **Scenario** view, the agenda and act stand together at the far center as an open book, with the agenda on the left. Smaller investigator standees sit beside their location, with two positions along each side and a clear gap from the card. The earlier Jacqueline experiment uses named Study and Hallway markers for its supplied locations. Full card inspection remains available; manual orbit can naturally place figures in front of cards from some angles.

In a fresh turn:

1. Tony has three ordinary actions and an available bonus Fight/Engage action. A ready Ghoul Minion is engaged with him; Internal Injury is in his threat area. Left-click a component to open its nearby labeled actions, then click the action to perform it. Selecting the component alone changes no game state. Actions that provoke show an amber **Attack of opportunity** warning with the source and amounts before you choose them.
2. When a skill test has legal card commitments, select the exact hand cards, then choose **Commit selected**, or choose **Commit no cards**. Commitment controls show **+N** followed by the tested skill's symbol (or its readable name when the font is unavailable). With no legal commitments, the empty prompt is skipped; Tony's Evade proceeds to **Reveal token**, with **Undo** still available before revealing. Click **Reveal chaos token** at the lower left from any camera view. It reveals through the same rules as the physical bag and focuses **Test** after the decision saves successfully. Then select the revealed token and choose **Resolve test**. Results are random. Use **Action history and rules reasons** to inspect costs and consequences.
3. Each harm event has one visible stage: an attack dealing damage and horror shows both together. **Finish animation** completes the current stage; later harm events still appear; **Motion: reduced** retains readable stages without motion. Click **Open recorded replay**, then **Next moment** or **Previous moment** to narrate stored events. **Return to live turn** restores the current live view.

Choose **Inspect** from a card's contextual controls for the existing enlarged artwork and readable rules; **Close** or `Esc` returns to the table. `Esc` or clicking empty space dismisses contextual actions without choosing one. Scroll the utility panel or card inspection when needed. Hold unmodified `W`/`A`/`S`/`D` to pan forward/left/back/right across the table relative to the camera; release to stop. Right-drag or use the arrow keys to orbit; the wheel or `+`/`-` zooms, and `Home` resets the camera. Manual camera movement overrides automatic following. Camera keys are suspended while a menu or modal captures keyboard input. Recorded replay permits inspection; return to live play before choosing game actions.

**Menu** at the upper right opens history, motion settings, save/reopen, recorded replay and artwork settings.

In a contextual menu, the yellow outline marks keyboard focus. **Tab** moves to the next option, **Shift+Tab** moves to the previous option, **Enter** or **Space** activates it, and **Esc** closes the menu. Arrow keys do not navigate these menus; camera keys are suspended while the menu has focus.

In both experiments, clicking a card or component opens inspection immediately when **Inspect** is its only option, including during replay. A single discarded card opens directly; several cards retain their chooser. Tony's idle chaos bag opens its scenario-reference inspection. Otherwise nearby actions share the same resting color, brighten on hover and show keyboard focus with a separate outline. Inspectable cards gain a brief fading hover frame; it clears when the pointer leaves. A short blue marker remains while that card's menu is open, and gold corner marks identify pending commitments or token cancellations. **Motion: reduced** in Tony and **Card feedback motion: reduced** in Jacqueline make the hover frame static and suppress reveal-button motion. These are the interaction conventions for future experiments and the full game as those consumers are implemented.

The encounter discard starts empty in both prepared experiments. Defeated Ghoul Minion and fully resolved Rotting Remains appear face up in its scenario-area pile and remain inspectable through reopening and replay. Internal Injury goes to Tony's own discard. The reveal shortcut appears only at a legal reveal decision; in Jacqueline it first offers normal reveal or her reaction when that choice remains available. The physical bag controls remain usable.

**Close and resume:** decisions autosave to `local/experiment/tony.session.json`. Close the window and run the same command to resume. **Save live turn** also saves at a pending choice, including before resolving a revealed token. **Reopen saved turn** reloads the save. At the end, **Start another Tony turn** archives the previous run. **End Tony's turn** is available at an action boundary; it cannot skip a pending test.

Joel [accepted the experiment and its final refinements on 2026-10-31](https://github.com/joeloverbeck/arkham-unfolded/issues/22#issuecomment-6536278638). The [owner verification walkthrough](docs/experiments/tony-turn.md#owner-verification) remains available as a reference for the exercised playing paths.

## Prepare the Tony artwork

The developer/agent prepares the selected files once; ordinary play needs only the resulting folder. From this checkout, with the original archive available:

```sh
curl --fail --location \
  https://game-icons.net/icons/ffffff/transparent/1x1/lorc/swap-bag.png \
  --output /tmp/arkham-reveal-icon.png
cargo run --locked --bin arkham-assets -- prepare-tony \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /tmp/arkham-reveal-icon.png \
  --output "$HOME/.local/share/arkham-unfolded/tony-assets-v4"
cargo run --locked --bin arkham-assets -- check-tony \
  "$HOME/.local/share/arkham-unfolded/tony-assets-v4"
```

This prepares 28 selected images and one symbol font, including the player-card back, damage/horror counters and reveal icon, in a separate collection. Preparation verifies the exact downloaded icon before creating output; ordinary play needs no download. Earlier Tony collections remain usable for their existing artwork, with readable substitutes for missing assets. In particular, `tony-assets-v3` lacks only the reveal icon. Prepare the fresh `tony-assets-v4` folder and use the launch command above. The source archive and any existing Jacqueline collection stay unchanged. Preparation requires a fresh destination; repeating it with an identical complete collection is a no-op. Conflicting, incomplete or unknown destination contents are preserved and reported.

These folder versions preserve earlier prepared files during development; they do not select different game rules. The ordinary launch command stays the same when the application's default collection changes. An explicitly saved folder or an old command containing `--art-root` continues to select that folder; use **Choose asset folder** to change a saved selection.

**Choose asset folder** in Tony's **Menu**, or Jacqueline's side panel, checks a prepared folder and saves its location for the next launch of that experiment. Scroll the controls if needed. The native folder dialog uses Ubuntu's `zenity` package. Settings are separate: Tony uses `$XDG_CONFIG_HOME/arkham-unfolded/tony.toml`; Jacqueline retains `config.toml` in the same directory. The normal configuration directory is `~/.config/arkham-unfolded`. Existing `config.toml` contents remain unchanged and apply only to Jacqueline.

Without a command-line override or saved preference, Tony uses `$XDG_DATA_HOME/arkham-unfolded/tony-assets-v4` and Jacqueline uses `$XDG_DATA_HOME/arkham-unfolded/assets-v2`. The normal data directory is `~/.local/share`; relative XDG overrides are ignored. Preparation examples above use that normal location; with a custom `XDG_DATA_HOME`, prepare there or save the actual folder through the app.

A temporary `--art-root PATH` overrides the saved setting for that launch. Replace that option and its path with `--no-art` to use readable substitutes. Copy the whole prepared folder, including `manifest.json`, to relocate it:

```sh
cargo run --locked --bin arkham-assets -- check-tony /absolute/path/to/copied-assets
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony --art-root /absolute/path/to/copied-assets
```

The check reports missing or changed files and exits unsuccessfully for an incomplete collection. Personal saves, recordings, research and configuration remain separate. Asset portability does not expand supported game content or authorize redistribution of third-party material.

## Earlier Jacqueline experiment

The no-argument application mode retains the earlier Jacqueline skill-test experiment, its records and its own artwork preference:

```sh
cargo run --locked --features tabletop --bin arkham-tabletop
```

Its saves use `local/experiment/curse.session.json`. See the [Jacqueline guide](docs/experiments/jacqueline-curse.md#build-and-run) for the first clicks, alternate case and legacy asset preparation. Tony and Jacqueline records use separate readers; there is no automatic migration.

## Project aim

The aim is to let one player control their investigators while the program handles setup, legal actions, timing, card effects, tokens, movement, and campaign bookkeeping. Cards, decks, and tokens should retain the physical presence of a tabletop game. Rules explanations will be available through an inspectable action history.

## First complete delivery

Play the original **Night of the Zealot** campaign from beginning to end on **Standard**, controlling these three investigators together:

| Investigator | Starting ArkhamDB deck |
| --- | --- |
| Tony Morgan | [5392133](https://arkhamdb.com/deck/view/5392133) |
| Patrice Hathaway | [5391062](https://arkhamdb.com/deck/view/5391062) |
| Jacqueline Fine | [5389418](https://arkhamdb.com/deck/view/5389418) |

The delivery includes normal campaign consequences and progression, upgrades made in ArkhamDB, save/resume, and recorded sessions that can be replayed in meaningful steps with automatic camera direction for narration. Smaller experiments will investigate feasibility; they will not count as delivering this campaign.

Content support follows the owner's actual use. Starting decks, campaign content, and selected upgrades and weaknesses must work before a play session begins. This is not a promise to support the entire published card pool. Public development and possible YouTube session recordings do not change that scope.

## Project documents

- [Mission and scope](docs/foundation/mission.md)
- [Play contract](docs/foundation/play-contract.md)
- [Sources and verification](docs/foundation/sources-and-verification.md)
- [Prepared asset collection decision](docs/adr/0001-prepared-runtime-assets.md)
- [Issue #1 feasibility assessment](docs/research/issue-1-assessment.md) — accepted source baseline and evidence for the selected bounded experiment
- [Run the Tony turn and verify its native experience](docs/experiments/tony-turn.md) — prepared turn under [issue #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22)
- [Run the earlier Jacqueline skill-test experiment](docs/experiments/jacqueline-curse.md) — native tabletop, player-equivalent driver, save/reopen and recorded replay under [issue #2](https://github.com/joeloverbeck/arkham-unfolded/issues/2)
- [Repository principles](docs/principles/principles-for-repos.md)

[GitHub Issues](https://github.com/joeloverbeck/arkham-unfolded/issues) and pull requests are the work-control plane. Foundations record the intended product and accepted decisions; they are not a second work queue. `CLAUDE.md` contains the agent bootstrap; both `AGENTS.md` and `AGENT.md` link to it.

## Current use and external inputs

The built experiment runs offline. A clean clone needs no artwork or credentials for the checks or readable tabletop substitutes. Run `scripts/check` for tracked-file hygiene, formatting, strict Clippy, and the Rust tool, rules, continuity and tabletop tests. Rustup uses the compiler and components in `rust-toolchain.toml`. The Ubuntu PR workflow runs the same phase launchers as named steps without a display or private assets; the portal transport tests start a private `dbus-daemon`. CI compiles its small Rust toolchain-preparation entry point directly with `rustc` before restoring Cargo's cache; the full tools package builds afterward.

Repository helpers live in the independent `arkham-tools` Cargo package. `scripts/tools --help` lists the bounded reader, Git-index guard, native verification and profiling commands; building these helpers does not compile Bevy. The index guard rejects generated caches and recognizable non-Rust executable source/template filenames, including force-added files. Inline code, templates and invocation-only shell remain subject to review under the [implementation-language contract](docs/foundation/mission.md#implementation-language). See the experiment guide for [native verification](docs/experiments/jacqueline-curse.md#developer-native-verification), external inputs, records and evidence limits.

The application reads a separately configured prepared asset collection. Image and font bytes are not included in Git. Gameplay remains usable with readable English substitutes when artwork is absent; absent or unsupported rules behavior must be identified before play. The [source contract](docs/foundation/sources-and-verification.md#presentation-asset-preparation) requires deliberate selection, copying/conversion and verification whenever development introduces a new asset dependency. The original archive's layout and source identities belong to that preparation step.

Once required game data is imported, gameplay, saves, and replays will work offline. ArkhamDB remains the deck editor; imports and update checks require network access. Personal saves, replays, captures, and credentials remain local and outside Git.

## Attribution

Arkham Horror: The Card Game is published by Fantasy Flight Games. This is an unofficial independent project. Third-party game content and artwork remain distinct from the project's original code.

The reveal control uses [Swap bag by Lorc](https://game-icons.net/1x1/lorc/swap-bag.html), under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). Its white transparent PNG is copied unchanged into the external prepared collection; any on-screen color is a presentation tint. Source identity and attribution are also recorded in each asset recipe and prepared manifest.

The public [Haskell implementation](https://github.com/halogenandtoast/ArkhamHorror) is a secondary implementation reference. Official game sources and independently established expectations determine intended rules behavior.

Original project code is available under the [MIT license](LICENSE). This does not grant rights to the game's artwork, card text, rules publications, or other third-party material. The copied repository constitution remains separately identified governing text.
