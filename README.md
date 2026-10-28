# Arkham Unfolded

An English-language Arkham Horror: The Card Game application for Ubuntu, planned with a Rust rules core and an animated 3D tabletop.

**Status: bounded Rust/3D experiments. The campaign is not playable yet.** The original Jacqueline/Patrice skill-test mode remains available. The new Tony mode offers a complete turn from a prepared Gathering position, pending separate owner acceptance under [#22](https://github.com/joeloverbeck/arkham-unfolded/issues/22).

Launch the Tony turn with `cargo run --locked --features tabletop --bin arkham-tabletop -- --experiment tony`. See the [Tony guide](docs/experiments/tony-turn.md) for the prepared position, separate artwork collection, saves, undo, replay and player driver.

## Launch and play the current experiment

On this Ubuntu machine, open **Terminal** (`Ctrl` + `Alt` + `T`) and paste:

```sh
cd /home/joeloverbeck/src/arkham-unfolded
cargo run --locked --features tabletop --bin arkham-tabletop
```

If your checkout is elsewhere, replace the directory in the first line. This requires Rust/Cargo 1.97.1 or later and an Ubuntu graphics session with X11/XWayland and Vulkan. The first launch downloads and compiles dependencies, which can take several minutes. Later launches reuse the build. Keep the terminal open while playing; the native window is titled **Arkham Unfolded — Scripted Rules Experiment**.

In a fresh case:

1. Click **Commit Promise of Power**, then **Use Jacqueline: reveal 3, then choose cancellations**.
2. Choose which tokens to cancel. If a curse resolves, watch the additional token appear on the table, then click **Show test result** when ready. Other completed tests open the result automatically. The centered result window explains the outcome; click **Return to table** to continue.
3. Click **Open recorded replay**. Use **Next moment** and **Previous moment** to step through it. Those buttons appear only when another moment exists in that direction.

Click any card to open its enlarged artwork and readable rules. Use **Close card** or `Esc` to return; on a smaller window, scroll inside the inspection to see the whole card and text. Right-drag or use the arrow keys to orbit the camera; the wheel or `+`/`-` zooms, and `Home` resets it. Moving the camera manually overrides replay's automatic following.

**Close and resume:** decisions autosave to `local/experiment/curse.session.json`. Close the window and run the same command to resume. Before any decision, use **Save live session** if you want to retain that initial state. If a completed result opens on launch, click **Return to table**, then **Start another example** for a fresh run; the previous run is archived. From an unfinished case, **End this experiment** makes that restart control available.

Card, token and standee images and the symbol font load from a small prepared asset collection. Missing artwork uses readable substitutes. Use **Choose asset folder** in the side panel to select a prepared folder, check its completeness and save its location for the next launch. Scroll the panel if needed. The native folder dialog uses Ubuntu's `zenity` package. See the [experiment guide](docs/experiments/jacqueline-curse.md) for preparation, alternate cases, save selection and recovery.

## Prepare the external assets

The developer/agent prepares the selected files once; ordinary play needs only the resulting folder. From this checkout, with the original archive available:

```sh
cargo run --locked --bin arkham-assets -- prepare \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --output "$HOME/.local/share/arkham-unfolded/assets"
```

This example uses the default location. If you set an absolute `XDG_DATA_HOME`, use that directory followed by `/arkham-unfolded/assets` as the output instead. This copies the 12 selected PNGs and extracts the verified font into ordinary files (about 11.57 MiB plus a small manifest). The source archive stays unchanged. Preparation requires a fresh destination; repeating it with an identical complete collection is a no-op. Conflicting, incomplete or unknown destination contents are preserved and reported. To prepare an updated selection, choose a new output folder and select it in the app after verification.

The default runtime folder is `$XDG_DATA_HOME/arkham-unfolded/assets`, normally `~/.local/share/arkham-unfolded/assets`. A saved choice lives in `$XDG_CONFIG_HOME/arkham-unfolded/config.toml`, normally `~/.config/arkham-unfolded/config.toml`, as `asset_root = "/absolute/path/to/assets"`. Relative XDG overrides are ignored. Folder selection takes effect after restarting the app. A temporary `--art-root PATH` overrides the saved setting for that launch; `--no-art` deliberately uses substitutes.

Copy the whole prepared folder, including `manifest.json`, to relocate it. The same compatible app build can check and use that copy without the original archive:

```sh
cargo run --locked --bin arkham-assets -- check /absolute/path/to/copied-assets
cargo run --locked --features tabletop --bin arkham-tabletop -- --art-root /absolute/path/to/copied-assets
```

The check reports missing or changed files and exits unsuccessfully for an incomplete collection. The app remains playable with readable substitutes. Personal saves, recordings, research documents and configuration are separate from this folder. Asset portability does not expand the supported game content or authorize redistribution of third-party material.

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
- [Run the selected skill-test experiment](docs/experiments/jacqueline-curse.md) — native tabletop, player-equivalent driver, save/reopen and recorded replay under [issue #2](https://github.com/joeloverbeck/arkham-unfolded/issues/2)
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
