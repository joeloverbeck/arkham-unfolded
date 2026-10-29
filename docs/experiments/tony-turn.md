# Tony's prepared Gathering turn

[Issue #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22) owns this bounded experiment and its acceptance. It begins in a deliberately prepared position, not a played campaign: Tony is unarmed in the revealed Study, engaged with a Ghoul Minion carrying one bounty, with Internal Injury in his threat area. Patrice and Jacqueline remain at the Study. Play ends after Tony's turn and its mandatory effects; no next investigator or enemy phase runs.

## Launch

```sh
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony
```

The shared scenario surface holds the map and test area; separate personal surfaces give Tony, Patrice and Jacqueline stable homes. Their hands sit along the near edges. Use the investigator portraits to focus a personal area, **Scenario** for the map, **Test** for committed cards and chaos tokens, or **Overview** for orientation. The current decision, acting investigator and test remain visible. Camera focus never changes the actor.

Left-click a component to see its nearby legal actions: the Ghoul for Fight/Evade/Engage, the Study for Investigate, Tony's card for Gain resource, the deck for Draw, or Internal Injury for Remove Injury. Choose the labeled action with a separate click. Selection alone never spends an action or reveals information. Choices without a physical source remain in the decision controls, including ending the turn. Tony's first qualifying Fight/Engage automatically spends his additional action. Each skill test offers card commitments, then a genuinely random Standard token draw and its resolution. The fixed hidden draw pile begins Guts, Guts, On the Hunt. Guts has no matching icons in this turn.

Tony’s face-down deck has one card-thickness per remaining card, visible layered edges and the standard player-card back on top. Drawing lowers the stack; replay shows the recorded deck count.

The decision display explains remaining actions and the current test; contextual actions show their costs and consequences before activation. Choose **Inspect** to open the existing card modal with readable rules and available artwork. **Close** or `Esc` returns to the same pending decision. `Esc` or clicking empty space dismisses a contextual menu without choosing an action. To commit cards, select each exact copy in hand and choose **Select to commit +1**, then **Commit selected** to confirm the full set; **Unselect commitment** removes a staged copy. Alternatively choose **Commit no cards**. Opening inspection does not commit the selected cards. At **Test**, select the chaos bag for **Reveal token**, then the revealed token for **Resolve test**.

Hold unmodified `W`/`A`/`S`/`D` to pan forward/left/back/right relative to the camera, preserving tilt and zoom; release to stop. Right-drag or arrow keys orbit, the wheel or `+`/`-` zooms, and `Home` resets the camera. Camera keys do not move the table while a menu or modal captures keyboard input. Manual movement overrides replay following and dismisses an open contextual menu. Replay allows inspection only; return to the live turn before making a game choice.

Use **Menu** at the upper right for action history, motion settings, save/reopen, recorded replay and artwork settings. Replay navigation and **Return to live turn** appear in the decision controls below.

Actions that currently provoke show an amber **Attack of opportunity** warning, identifying Ghoul Minion and its damage/horror before you click. The warning changes with the enemy's readiness and engagement; taking that intentional risk remains legal.

Each harm event has one presentation stage. Ghoul Minion's attack shows **+1 Damage** and **+1 Horror** together with the heart and brain marks. Separate effects, such as Tablet damage and later Internal Injury damage, remain separate stages. Live play advances automatically; **Finish animation** completes only the current stage; subsequent harm events still appear. Toggle **Motion: reduced** or **Motion: animated** to choose a session-local presentation preference. Reduced motion retains readable stages and amounts. Camera movement and card inspection remain available during presentation.

Damage and horror in visible text use the red heart and blue horror glyphs when the prepared font is available, including warnings, card inspection and history. Both tabletops share these colors and retain English accessibility labels and absent-font substitutes. The investigator's damage/horror counters sit as a centered group along the top long edge of the landscape card; a lone counter is centered there too. Missing counter artwork uses a readable grouped label. Revealed chaos tokens use the same thick disk, separate face and backed status label as the Jacqueline experiment.

Decisions autosave before taking effect. The default file is `local/experiment/tony.session.json`; reopening the same mode resumes it. Use **Save live turn** at a pending choice. **Undo** is available only when it does not cross a committed card or token reveal. It restores the whole deterministic change and adds a new recorded revision. Replay Previous is inspection, not undo.

New attempts archive the previous run. Replay shows stored moments with Next/Previous, pausing at each recorded harm event and the settled decision result without executing rules or drawing again; return to live before saving. Earlier Tony records retain their aggregate moments because their individual harm events were not recorded. Corrupt or incompatible records are refused with originals intact. Keep the record and its `.previous` recovery copy when investigating a failure. Closing during an animation preserves its already-committed result; reopening does not apply damage again.

The original Jacqueline mode remains the default when `--experiment tony` is omitted and retains its existing compatible save/record reader. Use the [Jacqueline guide](jacqueline-curse.md) for those records. There is no automatic conversion between experiment kinds.

## Owner verification

The remaining question is whether the actual native controls, explanations, interruption and replay let you play this turn without doing its bookkeeping, and whether choosing actions feels worthwhile. Automated rules and process-continuity checks have passed; native click automation could not calibrate reliably. Your inspection supplies the interaction and experience evidence those checks cannot establish.

Allow **at most two sessions of 20 minutes each, including retries**, counting playtesting already performed; presentation follow-ups do not reset that allowance. Stop sooner if a control blocks progress or the experience is not worth continuing, and report the point where that happened. Random failures are valid outcomes; do not restart to force a successful test. The routes below are suggestions, not a required winning sequence.

Begin with a fresh turn. If the app resumes an earlier one, finish its pending test, then end the turn at an action boundary and use **Start another Tony turn**. The earlier record is archived.

### Session 1: manage the threat and resume an interrupted test

1. Launch with the command above. Confirm the in-app heading says **Tony turn**. Focus an investigator with a portrait button, then try **Scenario**, **Test** and **Overview**. Left-click a card, choose **Inspect**, scroll where necessary, and close with **Close** or `Esc`. Try WASD panning, orbit, zoom and `Home`. Check that labels and controls remain readable and navigation has not changed the acting investigator.
2. Select Tony's card and read the attack-of-opportunity warning for **Gain resource**. Choose it and observe one attack stage with both **+1 Damage** and **+1 Horror**. After it finishes, **Undo last choice** before any reveal. Resources should return to five, damage/horror to zero, and ordinary actions to three; the bonus remains available. The history should retain an intelligible undo entry.
3. Select Internal Injury, then click **Remove Injury** after reading its cost and warning. Expect one ordinary action and the bonus remaining, one combined damage/horror attack stage, and Injury removed. Try **Finish animation** or reduced motion if useful; neither should repeat the attack or change the totals.
4. Select the Ghoul and choose **Fight**. Select and confirm any card commitments, or choose **Commit no cards**, then focus **Test**, select the bag and choose **Reveal token**. Before resolving it, note the token, committed cards and counters; click **Save live turn**, close the window and relaunch with the same command. Those details should be identical, without another cost or attack. The reveal itself must not be undoable. Select the revealed token, choose **Resolve test** and finish with the available choices. The first Fight spends the bonus; later Fights spend ordinary actions. Two successful basic Fights defeat the Ghoul and pay one resource, but failures are legitimate.

### Session 2: try different tactics and narrate the record

1. After the first turn ends, click **Start another Tony turn**. Select the Ghoul and try **Evade**, then investigate or gain resources. A successful Evade leaves the Ghoul exhausted and disengaged and removes opportunity-attack warnings; a failure leaves its threat in place. Check that the choices and history explain the resulting costs. You can choose a different action sequence if it is more interesting.
2. At an action boundary, try **End Tony's turn** with actions remaining. If Injury is still present, it should deal one direct damage once. No companion turn or enemy phase should start.
3. Click **Open recorded replay**, step with **Next moment** and **Previous moment**, and explain the turn aloud. Move the camera manually to override following. **Return to live turn** should restore the unchanged live state; replay should offer no live save or gameplay choices. Judge whether the steps, explanations and camera make narration comfortable.

Report any failed control, confusing consequence or unreadable view; whether save/resume and replay preserved what you saw; and whether playing the turn was clear and worthwhile. A negative or inconclusive verdict is useful. There is no need to repeat the full automated rules matrix or certify technical internals.

## External artwork and source custody

Prepare the current collection of 27 images and one symbol font, including the player-card back and damage/horror counters, in a fresh folder; earlier Tony and Jacqueline collections are retained:

```sh
cargo run --locked --bin arkham-assets -- prepare-tony \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --output "$HOME/.local/share/arkham-unfolded/tony-assets-v3"
cargo run --locked --bin arkham-assets -- check-tony \
  "$HOME/.local/share/arkham-unfolded/tony-assets-v3"
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony
```

Use the in-app asset folder selection to save a Tony-only preference in `$XDG_CONFIG_HOME/arkham-unfolded/tony.toml` (normally `~/.config/arkham-unfolded/tony.toml`). Jacqueline keeps its existing `config.toml`. Without a saved choice, Tony selects `$XDG_DATA_HOME/arkham-unfolded/tony-assets-v3` (normally `~/.local/share/arkham-unfolded/tony-assets-v3`); the launch command need not name a version. Startup reports the experiment, resolved folder and completeness. `--art-root` is temporary; `--no-art` uses readable substitutes. The program reads verified prepared files, never the source archive. A new collection must use a fresh destination; partial/conflicting collections are preserved and reported. All third-party image/font bytes remain outside Git.

The implementation consumes the exact source contract and independent worked expectations in #22, with original English faces, Rules Reference, FAQ 2.5 and original campaign guide under the accepted precedence. It does not refresh taboo or prepare a campaign. The unchanged [source manifest](../research/issue-1-evidence.json) identifies retained PDFs and deck inputs. On 2026-10-28 all nine source files, ten selected card faces and Study's revealed reverse matched their declared hashes. Their exact bytes were also copied to owner-controlled external custody at `~/.local/share/arkham-unfolded/source-evidence/tony-turn-v1` and verified by readback. Its `manifest.json` SHA-256 is `fb3423f0efc23c2bff9b0b6755c8b5b567e6edd6b1d00ddc7173d45b1274d244`. This is retained same-machine evidence, not an independent backup; preserve it while source fidelity or recovery relies on it.

## Shared player driver and checks

```sh
cargo run --locked --bin arkham-driver -- --experiment tony --save /tmp/new-tony-turn.json
```

The JSON-line protocol supports `observe`, `choose` with current `revision` and offered `id`, `save`, `replay`, `next`, `back`, `live`, `export` with a fresh `path`, and `quit`. Use `--load PATH` in another process to resume. Observations omit future card order and random state; deterministic token injection is a privileged test facility, never a player command. The UI and driver submit to the same rules and persistence authority.

Run `scripts/check`. Decision tests use #22's independent witnesses, including additional-action spending, opportunity attacks, all Standard faces, commitment restrictions and reveal barriers. Continuity tests cover failed-save retry, record refusal, immutable exports and fresh-process continuation. Native inspection follows the existing [checked capture/input procedure](jacqueline-curse.md#developer-native-verification), using isolated saves/settings, prepared art and substitutes. Screenshots establish inspected rendering only; interaction and human acceptance require their own evidence.

### Reproduce a visual state

Use legal driver choices to reach a state, then pass its save to the immutable native launcher. This example produces an opportunity attack's combined damage/horror counters and a held random chaos token:

```sh
cargo build --locked --features tabletop --bin arkham-tabletop --bin arkham-driver
tony_review=$(mktemp -d /tmp/arkham-tony-visual.XXXXXX)
target/debug/arkham-driver --experiment tony --save "$tony_review/held.json" \
  > "$tony_review/driver.jsonl" <<'CHOICES'
{"op":"choose","revision":0,"id":"investigate"}
{"op":"choose","revision":1,"id":"commit:none"}
{"op":"choose","revision":2,"id":"reveal"}
CHOICES
python3 -B scripts/native_verify.py launch \
  --binary target/debug/arkham-tabletop --output-dir "$tony_review/render" \
  --experiment tony --load "$tony_review/held.json" \
  --art-root "$HOME/.local/share/arkham-unfolded/tony-assets-v3" \
  --screenshot held.png --quit-after-screenshot
```

Inspect the driver's returned observations for successful choices and `render/held.png` for the actual result. The launcher copies the save and preserves the original. For a lone damage counter, use a new driver save and only `{"op":"choose","revision":0,"id":"end_turn"}`; Internal Injury deals one damage. For missing-art layout, launch another fresh render directory with `--no-art` instead of `--art-root`.

Reopening captures the committed state; it does not replay an interrupted live animation. To check motion, use a fresh live run and the checked native input path, or step the saved record through replay. Compare shared objects with the [existing visual behavior](jacqueline-curse.md#visual-and-wording-review) and record any unavailable interaction separately. For intermittent camera lag, use the [bounded profiling procedure](jacqueline-curse.md#capture-a-laggy-camera) while the symptom is present.

Joel's two sessions have a combined ceiling of 40 minutes including retries: one threat-removal turn and one that evades or accepts opportunity costs for another benefit, with interruption and narrated replay. Only Joel judges clarity, pacing, control burden and whether the turn is worth playing. This experiment does not establish a playable scenario, campaign, reusable engine or recurrent fitness.
