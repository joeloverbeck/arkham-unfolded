# Prepared Study escape attempt

[Issue #55](https://github.com/joeloverbeck/arkham-unfolded/issues/55) owns this local experiment and its acceptance. It starts from an arranged position in original The Gathering on Standard. It does not implement scenario setup or a complete scenario or campaign.

## Launch

```sh
cd /home/joeloverbeck/src/arkham-unfolded
cargo run --locked --features tabletop --bin arkham-tabletop -- --experiment party
```

The window says **Prepared Study Escape**. All three investigators start in Study, with six clues on the location and none held. Each has one turn with three ordinary actions. Choose their order, Investigate, Draw, gain a Resource or end a turn early. Finish one investigator's turn before beginning another; ending early does not reserve actions for later. The named turn indicator and portrait badge remain visible while other investigators provide legal commitments or reactions, including after their own turns have ended. Supporting choices name their performing investigator. Camera focus does not select the active investigator. Card inspection and history explain the available effects and consequences.

Before play, **Checking graphics…** verifies progress through the real scene and renderer. Keep the window visible and focused; game choices are unavailable during the check. A failed check exits with a reason in the terminal and preserves the existing save. Background/minimized time and ordinary idle are not treated as poor performance. Reopening retries the check and the same pending game decision. The shared guard also stops play if active rendering later remains too slow; it does not reset the GPU or change the driver.

The first fully settled action leaving six clues with the party offers a closing choice: spend six to advance Act 1, or finish without advancing. Advancement puts Hallway, Cellar, Attic and Parlor into play, places everyone in revealed Hallway, removes Study and exposes Act 2. The attempt then stops, preserving unfinished turns and remaining actions. If all turns end first, it stops before the enemy phase. Neither a declined nor an unsuccessful attempt invents a scenario resolution.

During tests, choose matching commitments from the testing investigator and at most one card from each other investigator at the location. Promise of Power adds four icons and one curse. Jacqueline can intervene once this round, including at an eligible later curse reveal after an earlier decline. The testing investigator keeps their own elder-sign effect. Patrice can draw Lucky!, gain its one-resource cost and use it during her last action; its Fast timing costs no action. Legal poor choices remain available.

## Saves and replay

Decisions save before their consequences become live. The default record is `local/experiment/party.session.json`; `--save /absolute/path/session.json` selects another location. Relaunching resumes the existing file, and `--load PATH` explicitly opens a record. Manual save preserves a pending choice. A failed save leaves the live decision and future randomness unchanged.

Undo cannot cross a committed card or token reveal. An unobserved Patrice shuffle can be undone within that boundary. Recorded replay uses stored observations with Next/Previous, automatic camera direction and manual camera control. Returning to live restores the current session. Starting another attempt archives the previous record. Tony and Jacqueline retain their existing modes and readers; records are not automatically migrated.

The player driver uses the same choices and persistence authority:

```sh
cargo run --locked --bin arkham-driver -- --experiment party --save /absolute/path/new-attempt.json
```

Send JSON lines such as `{"op":"observe"}`, then `{"op":"choose","revision":0,"id":"select:tony"}` using the current observation's revision and offered choice ID. It also accepts save, replay, next, back, live, export with a path, and quit. Use `--load` to resume an existing record. Ordinary player commands cannot select random outcomes or inspect hidden deck order.

## Artwork

Party artwork has its own prepared collection and `party.toml` preference. Existing collections and settings remain intact. Missing art leaves readable substitutes; use `--no-art` explicitly or `--art-root /absolute/path/collection` for a temporary selection. The developer prepares the collection from the unchanged archive and the verified reveal-icon input:

```sh
cargo run --locked --bin arkham-assets -- prepare-party \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /absolute/path/reveal-chaos-token.png \
  --output /absolute/path/new-party-assets
cargo run --locked --bin arkham-assets -- check-party /absolute/path/new-party-assets
```

`resources/party-asset-selection.json` binds all 43 selected images/font inputs and their transformations. A fresh output directory is required unless its complete contents already match. Runtime reads only the prepared collection. The source PDFs and deck responses remain in the declared local evidence custody; verification of their exact bytes does not establish an independent backup.

## Table layout and visual review

The scenario surface is now 18×22, extending away from Test with two-unit gaps both above and below Test. Scenario components and camera framing follow the deeper surface; card scale is unchanged. Use pan, zoom and inspection for details on a large map. The twelve-location witness below checks one finite arrangement, not arbitrary map topology or playable scenario support.

The shared layout applies the same arrangement to Party, Tony and Jacqueline: larger off-card standees; a separate Test surface between the scenario and investigator row; and compact 18×15 investigator surfaces as wide as the scenario surface. Permanent thick Damage/Horror counters sit above the upper-left investigator card, including zero totals. Party's Jacqueline starts at 2/9 horror. Deck/discard sit at upper right, a distinct upper band fits five threats, assets retain the broad central region, and centered hands use one shallow row along the near edge. Muted class tints, boundaries and labels identify the regions. Test navigation, the reveal shortcut and replay use the separate Test surface, with its bag and revealed/canceled-token sections.

Joel accepted the turn-indicator and deeper-scenario follow-up's visuals and gameplay on 2026-11-01 and authorized its publication. #55 remains open by explicit instruction. The original visual-review allowance was **one session of at most 15 minutes, including retries**; this acceptance does not renew it or #55's separate gameplay allowance. The walkthrough below remains a reference for affected checks, not a request to repeat accepted observations. From the prepared local build, launch a fresh isolated Party attempt:

```sh
cd /home/joeloverbeck/src/arkham-unfolded
ARKHAM_ART_ROOT="$HOME/.cache/agent-tmp/arkham-55-assets/party-assets-v1"
ARKHAM_LAYOUT_REVIEW="$HOME/.cache/agent-tmp/arkham-layout-owner-review-01"
scripts/tools native launch --binary "$PWD/target/debug/arkham-tabletop" \
  --experiment party --output-dir "$ARKHAM_LAYOUT_REVIEW" \
  --art-root "$ARKHAM_ART_ROOT" --initial-view overview --timeout-seconds 900
```

The output directory must not already exist; choose another unused suffix if needed. The helper stages and hashes the executable, isolates settings, records the launch and creates this review's `session.json`; it does not open a personal save. Keep the terminal running and the window visible during the graphics check. The 900-second process ceiling does not renew the cumulative review time if another launch is needed.

For the older experiments, reuse that command within the same cumulative 15 minutes with a fresh output directory and the matching arguments below. Their deliberately unsupported contents remain labeled as unrepresented.

| Experiment | Replace `--experiment` with | Replace `--art-root` with |
| --- | --- | --- |
| Tony | `tony` | `"$HOME/.local/share/arkham-unfolded/tony-assets-v4"` |
| Jacqueline | `jacqueline` | `"$HOME/.local/share/arkham-unfolded/assets-v2"` |

Replace `--art-root` and its value with `--no-art` to inspect text substitutes. These paths identify this machine's prepared collections; use the corresponding verified collection if it lives elsewhere.

- Visit Scenario, Test and each investigator focus. Inspect a card, dismiss its controls and use right-drag, WASD and zoom. Expected: clear gaps and labels, usable selection and camera travel, and no change of active investigator from navigation. After beginning a turn, its indicator and portrait badge should remain on that investigator even when focusing someone else or using a supporting ability.
- Check standee bases: the card-edge gap remains visible, neighbors nearly touch without contact, and the normal Scenario view leaves the location readable. Check names outside the investigator surfaces, distinct threat/asset/hand regions, personal harm counters and deck/discard labels. A one-card hand should sit centrally; additional cards share the center.
- Start a supported test and use cooperative commitments and Jacqueline's reaction when available. Expected: the bag, committed cards and tokens remain on the Test surface; canceled tokens move to their separate section after the choice commits. Readability should survive changing the camera and selecting tokens.
- At a pending choice, use **Menu → Save live attempt → Reopen saved attempt**. Inspect **Open recorded replay**, Next/Previous and the return to live play. Expected: the pending decision and outcomes remain intact, with the same layout and appropriate camera destinations. This in-app reopening check does not replace the separate fresh-process gameplay check below.

Stop at 15 minutes, a broken choice, a save/reopen mismatch or a material readability problem; retain the review directory and report the view and last action. Random outcomes do not require another attempt. Static native screenshots and tests of production scene entities do not prove physical input, control burden or owner acceptance. #55's gameplay acceptance remains open and separate.

### Finite crowded-layout witness

The developer witness is a standalone Rust example importing the unchanged production layout, camera, standee and harm-token renderers. It does not instrument the application library, open saves or provide gameplay. Its permanent banner reads **Presentation fixture - not a playable position**. Build and capture it in a fresh directory:

```sh
cargo build --locked --features tabletop --example layout_capacity
scripts/tools native launch --binary "$PWD/target/debug/examples/layout_capacity" \
  --output-dir "$HOME/.cache/agent-tmp/arkham-layout-fixture-01" \
  --art-root "$ARKHAM_ART_ROOT" --initial-view patrice \
  --screenshot scene.png --quit-after-screenshot --timeout-seconds 60
```

Use another fresh directory and `--initial-view tony`, `jacqueline`, `test` or `scenario` for the other finite views; `--no-art` replaces `--art-root` for named substitutes. Without capture-and-quit, Tab or 1–6 selects the six views. The three investigator surfaces show eight, nine and one hand cards respectively in single rows, twelve asset faces with an exhausted card and counters, and five threats. Selected-deck asset and threat faces repeat deliberately; unavailable threat artwork has named substitutes. The scenario contains twelve locations in three columns and four rows, alternating two card sizes, each with four standees and two clue counters, plus the act/agenda book, reference, encounter piles and victory display. The repeated standees deliberately stress clearance; the grid does not prescribe scenario connections. The Test surface contains its source, bag, six commitments and both token sections. These are geometry challenges, not legal-coexistence claims, implemented card rules or proof of full-game capacity. Developer captures do not grant an additional owner-review session.

## Remaining owner verification

Technical checks cannot judge clarity, pacing or control burden. The allowance in #55 is two sessions of at most 20 minutes each, including retries. Stop at the time limit, a broken choice, a save/reopen mismatch or a material readability problem; retain the record and report the last choice. Outcome luck does not require another run. Acceptance and publication remain separate from implementation.

1. In session 1, choose investigator order and complete an attempt with cooperative commitments and Jacqueline's reaction. Expected: actor and camera remain independent, costs and results are understandable, and all offered cancellations are selectable. Close at a meaningful pending choice and relaunch the same command. Expected: the exact choice and prior outcomes return. Finish by advancing, declining, or exhausting turns; expected: the declared endpoint and map/state are honest.
2. In session 2, change investigator order or ability allocation and inspect any declined or unsuccessful route still unobserved. Open the stored replay, move forward and back, change the camera, then return to live. Expected: understandable moments suitable for narration, unchanged live state, and no new random outcomes. Judge whether the connected party task is worth playing at this control burden.

The owning issue's candidate handoff supplies the exact commit, verified executable and runnable isolated launch command, along with completed agent checks. Repeat an accepted observation only if a later change affects it.
