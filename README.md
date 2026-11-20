<!--
This file is the public front page. It owns what the project is, what works today,
the short launch path, the document map and attribution. Put anything else in the
document that owns it and link to that document from here; see
docs/agents/domain.md#document-ownership.
-->

# Arkham Unfolded

An English-language Arkham Horror: The Card Game application for Ubuntu, implemented in Rust with a 3D tabletop. It is an unofficial, independent project built for its owner's own play.

The aim is to let one player control their investigators while the program handles setup, legal actions, timing, card effects, tokens, movement, and campaign bookkeeping. Cards, decks, and tokens should retain the physical presence of a tabletop game. Rules explanations will be available through an inspectable action history.

## Status

**The Gathering is playable; the campaign is not complete.** Scenario selection offers only original The Gathering on Standard with the selected Tony Morgan, Patrice Hathaway and Jacqueline Fine starting decks. The owner accepted complete Gathering play on his own playtests; the [closing record of #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6868075979) states that acceptance's evidence limits and the checks it released unverified.

Agent partners are optional: Claude Code or Codex sessions can play some investigators while you play the others.

Development builds promise no save compatibility. A save loads only when the current build reproduces its recorded game exactly; otherwise it is refused unchanged.

## Launch and resume The Gathering

From this checkout:

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop
```

The first build can take several minutes. Rustup uses the compiler in `rust-toolchain.toml`; `scripts/dev` requires `flock` and a running user systemd manager. Launch opens **Scenario selection**.

**Artwork is required for 3D play and is not in this repository.** Obtain the complete `arkham-unfolded-gathering-v7` runtime ZIP, select **Import artwork** on scenario selection, and choose the ZIP once; the app verifies it, installs it per user and remembers it for later offline launches. A public download host is not selected, and local packaging does not grant redistribution rights.

Select **The Gathering**, then:

- **New attempt** starts a game at a new save path, preserving existing saves.
- **Continue** opens the most recently admitted attempt.
- **Open saved attempt** opens a file you choose.

Play begins at setup: click an investigator card to choose the lead, then select mulligan cards and confirm each hand. Decisions autosave. Card and pile menus provide Inspect and the available actions.

The [playing guide](docs/playing.md) has the rest: artwork recovery, save locations, controls, restart, agent partners, incompatible saves and the diagnostic launch.

## First complete delivery

Play the original **Night of the Zealot** campaign from beginning to end on **Standard**, controlling these three investigators together:

| Investigator | Starting ArkhamDB deck |
| --- | --- |
| Tony Morgan | [5392133](https://arkhamdb.com/deck/view/5392133) |
| Patrice Hathaway | [5391062](https://arkhamdb.com/deck/view/5391062) |
| Jacqueline Fine | [5389418](https://arkhamdb.com/deck/view/5389418) |

The delivery includes normal campaign consequences and progression, upgrades made in ArkhamDB, save/resume, and inspectable action history. Bounded investigations can establish feasibility; they do not count as delivering this campaign.

Content support follows the owner's actual use. Starting decks, campaign content, and selected upgrades and weaknesses must work before a play session begins. This is not a promise to support the entire published card pool. Public development and possible YouTube session recordings do not change that scope.

## Documents

| To learn | Read |
| --- | --- |
| How to play, save, recover and use agent partners | [Playing guide](docs/playing.md) |
| What is being built, and for whom | [Mission and scope](docs/foundation/mission.md), [play contract](docs/foundation/play-contract.md) |
| Which sources decide rules behavior and artwork | [Sources and verification](docs/foundation/sources-and-verification.md) |
| How the table looks and behaves | [Tabletop presentation contract](docs/tabletop-presentation.md) |
| Why lasting choices were made, and what terms mean | [Decision records](docs/adr/), [glossary](GLOSSARY.md) |
| What was observed and checked | [Gathering play and verification](docs/experiments/gathering-setup.md), [research notes](docs/research/) |
| How to build, check and verify | [Local builds and checks](docs/agents/local-builds-and-checks.md), [native verification](docs/agents/native-verification.md) |
| How an agent sits at the table | [Agent seat](docs/agents/agent-seat.md), [partner instructions](docs/agents/partner-instructions.md) |
| How the repository is governed | [Repository principles](docs/principles/principles-for-repos.md) |

[GitHub Issues](https://github.com/joeloverbeck/arkham-unfolded/issues) and pull requests hold active work; the documents above are not a second work queue. Agents start from `CLAUDE.md`.

## Build and check

The built application runs offline, and a clean clone needs no artwork or credentials for headless engineering checks. Run `scripts/check` before publication, and `scripts/dev COMMAND [ARG...]` for other builds and individual checks. [Local builds and checks](docs/agents/local-builds-and-checks.md) has the phases, resource limits, prerequisites and recovery.

## Attribution

Arkham Horror: The Card Game is published by Fantasy Flight Games. This is an unofficial independent project. Third-party game content and artwork remain distinct from the project's original code.

The reveal control uses [Swap bag by Lorc](https://game-icons.net/1x1/lorc/swap-bag.html), under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). Its white transparent PNG is copied unchanged into the external prepared collection; any on-screen color is a presentation tint. Source identity and attribution are also recorded in each asset recipe and prepared manifest.

The offerings counter uses [“Chalice drops” by Lorc](https://game-icons.net/1x1/lorc/chalice-drops.html), licensed under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). Artwork is unchanged; its slate-blue shield is separate application geometry. Both icon credits and links are available in **Menu → Credits** and retained in the recipe and prepared manifest.

The public [Haskell implementation](https://github.com/halogenandtoast/ArkhamHorror) is a secondary implementation reference. Official game sources and independently established expectations determine intended rules behavior.

Original project code is available under the [MIT license](LICENSE). This does not grant rights to the game's artwork, card text, rules publications, or other third-party material. The copied repository constitution remains separately identified governing text.
