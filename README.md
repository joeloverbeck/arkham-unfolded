# Arkham Unfolded

An English-language Arkham Horror: The Card Game application for Ubuntu, planned with a Rust rules core and an animated 3D tabletop.

**Status: bounded Rust/3D experiments. The campaign is not playable yet.** The original Jacqueline/Patrice skill-test mode remains available. The new Tony mode offers a complete turn from a prepared Gathering position, pending separate owner acceptance under [#22](https://github.com/joeloverbeck/arkham-unfolded/issues/22).

See the [Tony guide](docs/experiments/tony-turn.md) for the prepared position, saves, undo, replay and player driver, including the [two-session verification walkthrough](docs/experiments/tony-turn.md#owner-verification).

## Launch and play the Tony turn

On this Ubuntu machine, open **Terminal** (`Ctrl` + `Alt` + `T`) and paste:

```sh
cd /home/joeloverbeck/src/arkham-unfolded
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony \
  --art-root "$HOME/.local/share/arkham-unfolded/tony-assets-v3"
```

The `--experiment tony` option selects the new turn. The explicit asset folder selects its matching artwork even when a previous preference points to Jacqueline's collection. If that folder has not been prepared, use the preparation instructions below; missing artwork leaves readable substitutes.

If your checkout is elsewhere, replace the directory in the first line. This requires Rust/Cargo 1.97.1 or later and an Ubuntu graphics session with X11/XWayland and Vulkan. The first launch downloads and compiles dependencies, which can take several minutes. Later launches reuse the build. Keep the terminal open while playing. The native window retains the title **Arkham Unfolded — Scripted Rules Experiment**; the heading inside must say **Tony turn / The Gathering / Standard**.

In a fresh turn:

1. Tony has three ordinary actions and an available bonus Fight/Engage action. A ready Ghoul Minion is engaged with him; Internal Injury is in his threat area. Actions that provoke show an amber **Attack of opportunity** warning with the source and amounts before you choose them.
2. For a skill test, choose a card commitment or **Commit no cards**, then **Reveal chaos token** and **Resolve skill test**. Results are random. Use **Action history and rules reasons** to inspect costs and consequences.
3. Each harm event has one visible stage: an attack dealing damage and horror shows both together. **Finish animation** completes the current stage; later harm events still appear; **Motion: reduced** retains readable stages without motion. Click **Open recorded replay**, then **Next moment** or **Previous moment** to narrate stored events. **Return to live turn** restores the current live view.

Click a card or its inspection button for enlarged artwork and readable rules; **Close** or `Esc` returns to the table. Scroll the panel or card inspection when needed. Right-drag or use the arrow keys to orbit; the wheel or `+`/`-` zooms, and `Home` resets the camera. Moving the camera manually overrides replay's automatic following.

**Close and resume:** decisions autosave to `local/experiment/tony.session.json`. Close the window and run the same command to resume. **Save live turn** also saves at a pending choice, including before resolving a revealed token. **Reopen saved turn** reloads the save. At the end, **Start another Tony turn** archives the previous run. **End Tony's turn** is available at an action boundary; it cannot skip a pending test.

For the remaining acceptance check, follow the [owner verification walkthrough](docs/experiments/tony-turn.md#owner-verification): two sessions of at most 20 minutes each, including retries.

## Prepare the Tony artwork

The developer/agent prepares the selected files once; ordinary play needs only the resulting folder. From this checkout, with the original archive available:

```sh
cargo run --locked --bin arkham-assets -- prepare-tony \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --output "$HOME/.local/share/arkham-unfolded/tony-assets-v3"
cargo run --locked --bin arkham-assets -- check-tony \
  "$HOME/.local/share/arkham-unfolded/tony-assets-v3"
```

This prepares 27 selected images and one symbol font, including the player-card back and damage/horror counters, in a separate collection. Earlier `tony-assets` and `tony-assets-v2` collections remain usable for their existing artwork but lack the new harm artwork/font; the original collection also lacks the deck back. Prepare the fresh `tony-assets-v3` folder and use the launch command above. The source archive and any existing Jacqueline collection stay unchanged. Preparation requires a fresh destination; repeating it with an identical complete collection is a no-op. Conflicting, incomplete or unknown destination contents are preserved and reported.

These folder versions preserve earlier prepared files during development; they do not select different game rules. Ordinary Tony play uses the current collection above. Reusing an older launch command can leave new artwork and glyphs unavailable even after updating the application.

**Choose asset folder** in the side panel checks a prepared folder and saves its location for the next launch. Scroll the panel if needed. The native folder dialog uses Ubuntu's `zenity` package. The preference is shared by the experiment modes; use the explicit `--art-root` commands in their guides when switching between their separate collections. Settings live in `$XDG_CONFIG_HOME/arkham-unfolded/config.toml`, normally `~/.config/arkham-unfolded/config.toml`. Relative XDG overrides are ignored. Without a command-line override or saved preference, the generic asset root is `$XDG_DATA_HOME/arkham-unfolded/assets`, normally `~/.local/share/arkham-unfolded/assets`.

A temporary `--art-root PATH` overrides the saved setting for that launch. Replace that option and its path with `--no-art` to use readable substitutes. Copy the whole prepared folder, including `manifest.json`, to relocate it:

```sh
cargo run --locked --bin arkham-assets -- check-tony /absolute/path/to/copied-assets
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony --art-root /absolute/path/to/copied-assets
```

The check reports missing or changed files and exits unsuccessfully for an incomplete collection. Personal saves, recordings, research and configuration remain separate. Asset portability does not expand supported game content or authorize redistribution of third-party material.

## Earlier Jacqueline experiment

The no-argument application mode retains the earlier Jacqueline skill-test experiment and its records. To launch it with its prepared artwork:

```sh
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --art-root "$HOME/.local/share/arkham-unfolded/assets"
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

The built experiment runs offline. A clean clone needs no artwork or credentials for the checks or readable tabletop substitutes. Run `scripts/check` for formatting, strict Clippy, and the rules, continuity and tabletop tests. Rustup uses the compiler and components in `rust-toolchain.toml`. The Ubuntu PR workflow runs the same command without a display or private assets. See the experiment guide for [native verification](docs/experiments/jacqueline-curse.md#developer-native-verification), external inputs, records and evidence limits.

The application reads a separately configured prepared asset collection. Image and font bytes are not included in Git. Gameplay remains usable with readable English substitutes when artwork is absent; absent or unsupported rules behavior must be identified before play. The [source contract](docs/foundation/sources-and-verification.md#presentation-asset-preparation) requires deliberate selection, copying/conversion and verification whenever development introduces a new asset dependency. The original archive's layout and source identities belong to that preparation step.

Once required game data is imported, gameplay, saves, and replays will work offline. ArkhamDB remains the deck editor; imports and update checks require network access. Personal saves, replays, captures, and credentials remain local and outside Git.

## Attribution

Arkham Horror: The Card Game is published by Fantasy Flight Games. This is an unofficial independent project. Third-party game content and artwork remain distinct from the project's original code.

The public [Haskell implementation](https://github.com/halogenandtoast/ArkhamHorror) is a secondary implementation reference. Official game sources and independently established expectations determine intended rules behavior.

Original project code is available under the [MIT license](LICENSE). This does not grant rights to the game's artwork, card text, rules publications, or other third-party material. The copied repository constitution remains separately identified governing text.
