# Prepared Study escape attempt

[Issue #55](https://github.com/joeloverbeck/arkham-unfolded/issues/55) owns this local experiment and its acceptance. It starts from an arranged position in original The Gathering on Standard. It does not implement scenario setup or a complete scenario or campaign.

## Launch

```sh
cd /home/joeloverbeck/src/arkham-unfolded
cargo run --locked --features tabletop --bin arkham-tabletop -- --experiment party
```

The window says **Prepared Study Escape**. All three investigators start in Study, with six clues on the location and none held. Each has one turn with three ordinary actions. Choose their order, Investigate, Draw, gain a Resource or end a turn early. Camera focus does not select the acting investigator. Card inspection and history explain the available effects and consequences.

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

## Remaining owner verification

Technical checks cannot judge clarity, pacing or control burden. The allowance in #55 is two sessions of at most 20 minutes each, including retries. Stop at the time limit, a broken choice, a save/reopen mismatch or a material readability problem; retain the record and report the last choice. Outcome luck does not require another run. Acceptance and publication remain separate from implementation.

1. In session 1, choose investigator order and complete an attempt with cooperative commitments and Jacqueline's reaction. Expected: actor and camera remain independent, costs and results are understandable, and all offered cancellations are selectable. Close at a meaningful pending choice and relaunch the same command. Expected: the exact choice and prior outcomes return. Finish by advancing, declining, or exhausting turns; expected: the declared endpoint and map/state are honest.
2. In session 2, change investigator order or ability allocation and inspect any declined or unsuccessful route still unobserved. Open the stored replay, move forward and back, change the camera, then return to live. Expected: understandable moments suitable for narration, unchanged live state, and no new random outcomes. Judge whether the connected party task is worth playing at this control burden.

The owning issue's candidate handoff supplies the exact commit, verified executable and runnable isolated launch command, along with completed agent checks. Repeat an accepted observation only if a later change affects it.
