# Tony's prepared Gathering turn

[Issue #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22) owns this bounded experiment and its acceptance. It begins in a deliberately prepared position, not a played campaign: Tony is unarmed in the revealed Study, engaged with a Ghoul Minion carrying one bounty, with Internal Injury in his threat area. Patrice and Jacqueline remain at the Study. Play ends after Tony's turn and its mandatory effects; no next investigator or enemy phase runs.

## Launch

```sh
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony --art-root "$HOME/.local/share/arkham-unfolded/tony-assets-v3"
```

Use the visible legal choices to fight, evade, investigate, draw, gain a resource, engage an eligible enemy, remove Injury, or end early. Tony's first qualifying Fight/Engage automatically spends his additional action. Each skill test offers card commitments, then a genuinely random Standard token draw and its resolution. The fixed hidden draw pile begins Guts, Guts, On the Hunt. Guts has no matching icons in this turn.

Tony’s face-down deck has one card-thickness per remaining card, visible layered edges and the standard player-card back on top. Drawing lowers the stack; replay shows the recorded deck count.

The panel explains remaining actions, costs and consequences. Inspect cards for readable text and available artwork. Camera controls remain right drag/arrow keys to orbit, wheel or +/- to zoom, and Home to reset. Manual movement overrides replay following.

Actions that currently provoke show an amber **Attack of opportunity** warning, identifying Ghoul Minion and its damage/horror before you click. The warning changes with the enemy's readiness and engagement; taking that intentional risk remains legal.

Each harm event has one presentation stage. Ghoul Minion's attack shows **+1 Damage** and **+1 Horror** together with the heart and brain marks. Separate effects, such as Tablet damage and later Internal Injury damage, remain separate stages. Live play advances automatically; **Finish animation** completes only the current stage; subsequent harm events still appear. Toggle **Motion: reduced** or **Motion: animated** to choose a session-local presentation preference. Reduced motion retains readable stages and amounts. Camera movement and card inspection remain available during presentation.

Decisions autosave before taking effect. The default file is `local/experiment/tony.session.json`; reopening the same mode resumes it. Use **Save live turn** at a pending choice. **Undo** is available only when it does not cross a committed card or token reveal. It restores the whole deterministic change and adds a new recorded revision. Replay Previous is inspection, not undo.

New attempts archive the previous run. Replay shows stored moments with Next/Previous, pausing at each recorded harm event and the settled decision result without executing rules or drawing again; return to live before saving. Earlier Tony records retain their aggregate moments because their individual harm events were not recorded. Corrupt or incompatible records are refused with originals intact. Keep the record and its `.previous` recovery copy when investigating a failure. Closing during an animation preserves its already-committed result; reopening does not apply damage again.

The original Jacqueline mode remains the default when `--experiment tony` is omitted and retains its existing compatible save/record reader. Use the [Jacqueline guide](jacqueline-curse.md) for those records. There is no automatic conversion between experiment kinds.

## Owner verification

The remaining question is whether the actual native controls, explanations, interruption and replay let you play this turn without doing its bookkeeping, and whether choosing actions feels worthwhile. Automated rules and process-continuity checks have passed; native click automation could not calibrate reliably. Your inspection supplies the interaction and experience evidence those checks cannot establish.

Allow **at most two sessions of 20 minutes each, including retries**, counting playtesting already performed; presentation follow-ups do not reset that allowance. Stop sooner if a control blocks progress or the experience is not worth continuing, and report the point where that happened. Random failures are valid outcomes; do not restart to force a successful test. The routes below are suggestions, not a required winning sequence.

Begin with a fresh turn. If the app resumes an earlier one, finish its pending test, then end the turn at an action boundary and use **Start another Tony turn**. The earlier record is archived.

### Session 1: manage the threat and resume an interrupted test

1. Launch with the command above. Confirm the in-app heading says **Tony turn**. Inspect a card and its text, scroll where necessary, close with **Close** or `Esc`, and try orbit, zoom and `Home`. Check that labels and controls remain readable.
2. Before choosing **Gain 1 resource**, check its attack-of-opportunity warning. Choose it and observe one attack stage with both **+1 Damage** and **+1 Horror**. After it finishes, **Undo last choice** before any reveal. Resources should return to five, damage/horror to zero, and ordinary actions to three; the bonus remains available. The history should retain an intelligible undo entry.
3. Click **Remove Internal Injury**. Expect one ordinary action and the bonus remaining, one combined damage/horror attack stage, and Injury removed. Try **Finish animation** or reduced motion if useful; neither should repeat the attack or change the totals.
4. Click **Fight Ghoul Minion**, choose a commitment or **Commit no cards**, and **Reveal chaos token**. Before resolving it, note the token, committed cards and counters; click **Save live turn**, close the window and relaunch with the same command. Those details should be identical, without another cost or attack. The reveal itself must not be undoable. Click **Resolve skill test** and finish with the available choices. The first Fight spends the bonus; later Fights spend ordinary actions. Two successful basic Fights defeat the Ghoul and pay one resource, but failures are legitimate.

### Session 2: try different tactics and narrate the record

1. After the first turn ends, click **Start another Tony turn**. Try **Evade Ghoul Minion**, then investigate or gain resources. A successful Evade leaves the Ghoul exhausted and disengaged and removes opportunity-attack warnings; a failure leaves its threat in place. Check that the choices and history explain the resulting costs. You can choose a different action sequence if it is more interesting.
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
  --experiment tony --art-root "$HOME/.local/share/arkham-unfolded/tony-assets-v3"
```

Use the in-app asset folder selection to save a preference for later launches. `--art-root` is temporary; `--no-art` uses readable substitutes. The program reads verified prepared files, never the source archive. A new collection must use a fresh destination; partial/conflicting collections are preserved and reported. All third-party image/font bytes remain outside Git.

The implementation consumes the exact source contract and independent worked expectations in #22, with original English faces, Rules Reference, FAQ 2.5 and original campaign guide under the accepted precedence. It does not refresh taboo or prepare a campaign. The unchanged [source manifest](../research/issue-1-evidence.json) identifies retained PDFs and deck inputs. On 2026-10-28 all nine source files, ten selected card faces and Study's revealed reverse matched their declared hashes. Their exact bytes were also copied to owner-controlled external custody at `~/.local/share/arkham-unfolded/source-evidence/tony-turn-v1` and verified by readback. Its `manifest.json` SHA-256 is `fb3423f0efc23c2bff9b0b6755c8b5b567e6edd6b1d00ddc7173d45b1274d244`. This is retained same-machine evidence, not an independent backup; preserve it while source fidelity or recovery relies on it.

## Shared player driver and checks

```sh
cargo run --locked --bin arkham-driver -- --experiment tony --save /tmp/new-tony-turn.json
```

The JSON-line protocol supports `observe`, `choose` with current `revision` and offered `id`, `save`, `replay`, `next`, `back`, `live`, `export` with a fresh `path`, and `quit`. Use `--load PATH` in another process to resume. Observations omit future card order and random state; deterministic token injection is a privileged test facility, never a player command. The UI and driver submit to the same rules and persistence authority.

Run `scripts/check`. Decision tests use #22's independent witnesses, including additional-action spending, opportunity attacks, all Standard faces, commitment restrictions and reveal barriers. Continuity tests cover failed-save retry, record refusal, immutable exports and fresh-process continuation. Native inspection follows the existing [checked capture/input procedure](jacqueline-curse.md#developer-native-verification), using isolated saves/settings, prepared art and substitutes. Screenshots establish inspected rendering only; interaction and human acceptance require their own evidence.

Joel's two sessions have a combined ceiling of 40 minutes including retries: one threat-removal turn and one that evades or accepts opportunity costs for another benefit, with interruption and narrated replay. Only Joel judges clarity, pacing, control burden and whether the turn is worth playing. This experiment does not establish a playable scenario, campaign, reusable engine or recurrent fitness.
