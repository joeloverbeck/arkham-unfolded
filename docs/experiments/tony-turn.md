# Tony's prepared Gathering turn

**Artwork contract amendment, 2026-11-07:** [ADR 0004](../adr/0004-require-artwork-for-3d-play.md)
requires complete artwork for 3D play and a blocking startup recovery window.
All four modes now refuse incomplete collections and wait for every required
image and the symbol font to load before play. **Choose asset folder** selects a
complete collection, **Retry** rechecks it, and **Quit** leaves saves unchanged.
The removed `--no-art` option is rejected. Dated substitute-presentation evidence
below describes earlier builds. Headless checks remain artwork-independent.
[Issue #124's handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/124#issuecomment-6706875824)
records verification and remaining native acceptance.

Replay has been retired in all four modes. Saves from before this change are unsupported and remain untouched; use `--save` with a new path to start an attempt. New-format saves retain exact continuation, action history and applicable Undo. There is no recording export or historical reader. [Decision and scope](../adr/0003-retire-replay.md). Dated evidence below describes its original builds; references to replay in those historical reports are superseded.

[Issue #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22) owns this bounded experiment and its acceptance. It begins in a deliberately prepared position, not a played campaign: Tony is unarmed in the revealed Study, engaged with a Ghoul Minion carrying one bounty, with Internal Injury in his threat area. Patrice and Jacqueline remain at the Study. Play ends after Tony's turn and its mandatory effects; no next investigator or enemy phase runs.

## Launch

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop \
  --experiment tony --art-root "$HOME/.cache/agent-tmp/arkham-73-assets/tony-assets-v5"
```

The shared scenario surface holds the map and test area; separate personal surfaces give Tony, Patrice and Jacqueline stable homes. Their hands sit along the near edges. Use the investigator portraits to focus a personal area, **Scenario** for the map, **Test** for committed cards and chaos tokens, or **Overview** for orientation. The current decision, acting investigator and test remain visible. Camera focus never changes the actor.

Left-click a component to see its nearby legal actions: the Ghoul for Fight/Evade/Engage, the Study for Investigate, Tony's card for Gain resource, the deck for Draw, or Internal Injury for Remove Injury. Choose the labeled action with a separate click. Selection alone never spends an action or reveals information. Choices without a physical source remain in the decision controls, including ending the turn. Tony's first qualifying Fight/Engage automatically spends his additional action. Each skill test offers any legal card commitments, then a genuinely random Standard token draw and its resolution. If nobody can commit, the empty commitment prompt is skipped. The fixed hidden draw pile begins Guts, Guts, On the Hunt. Guts has no matching icons in this turn; Patrice and Jacqueline are at the Study but have no cards to contribute.

Tony’s face-down deck has one card-thickness per remaining card, visible layered edges and the standard player-card back on top. Drawing lowers the stack.

The decision display explains remaining actions and the current test; contextual actions show their costs and consequences before activation. Choose **Inspect** to open the existing card modal with readable rules and available artwork. A card or component with exactly one Inspect operation opens that modal directly, including a one-card discard and the idle chaos bag's scenario reference. Several inspectable cards retain a chooser; a lone game action is never performed automatically. **Close** or `Esc` returns to the same pending decision. `Esc` or clicking empty space dismisses a contextual menu without choosing an action. To commit cards, select each exact copy in hand and choose **Select to commit +1** with the tested skill's symbol, then **Commit selected** to confirm the full set and its displayed total; **Unselect commitment** removes a staged copy. On the Hunt contributes combat during Fight or intellect during Investigate. The required symbol font supplies skill glyphs, with English accessibility labels. Alternatively choose **Commit no cards**. Selecting the last eligible card still requires confirmation, and opening inspection does not commit the selected cards. With no legal commitments, continue directly to the reveal decision.

**Reveal chaos token** appears at the lower left whenever a reveal is legal. Click it to draw through the same saved decision as the physical bag and focus **Test**; then select the revealed token for **Resolve test**. The shortcut disappears when unavailable, and respects blocking panels and unfinished consequence presentation. Its brief transition respects reduced motion. A failed save or stale click leaves the game and camera unchanged. The physical bag's contextual action remains available and retains its existing camera behavior.

The encounter discard starts empty in this prepared position. A defeated Ghoul Minion moves there after its final damage presentation, with its face and the pile count visible in the scenario area; click the one-card pile to inspect it directly. Tony still receives the bounty resource. Removed Internal Injury goes to Tony's own discard. These destinations remain visible after save/reopen. Encounter-deck and victory-display contents remain unrepresented.

Hold unmodified `W`/`A`/`S`/`D` to pan forward/left/back/right relative to the camera, preserving tilt and zoom; release to stop. Right-drag or arrow keys orbit, the wheel or `+`/`-` zooms, and `Home` resets the camera. Camera keys do not move the table while a menu or modal captures keyboard input. Manual movement dismisses an open contextual menu.

Use **Menu** at the upper right for action history, motion settings, save/reopen and artwork settings.

Actions that currently provoke show an amber **Attack of opportunity** warning, identifying Ghoul Minion and its damage/horror before you click. The warning changes with the enemy's readiness and engagement; taking that intentional risk remains legal.

Each harm event has one presentation stage. Ghoul Minion's attack shows **+1 Damage** and **+1 Horror** together with the heart and brain marks. Separate effects, such as Tablet damage and later Internal Injury damage, remain separate stages. Live play advances automatically; **Finish animation** completes only the current stage; subsequent harm events still appear. Toggle **Motion: reduced** or **Motion: animated** to choose a session-local presentation preference. Reduced motion retains readable stages and amounts. Camera movement and card inspection remain available during presentation.

Damage and horror in visible text use the red heart and blue horror glyphs from the required prepared font, including warnings, card inspection and history. Both tabletops share these colors and retain English accessibility labels. The investigator's damage/horror counters sit as a centered group along the top long edge of the landscape card; a lone counter is centered there too. Required counter artwork must load before play. Revealed chaos tokens use the same thick disk, separate face and backed status label as the Jacqueline experiment.

Decisions autosave before taking effect. The default file is `local/experiment/tony.session.json`; reopening the same mode resumes it. Use **Save live turn** at a pending choice. **Undo** is available only when it does not cross a committed card or token reveal. It restores the whole deterministic change and adds a new recorded revision.

New attempts archive the previous run. Corrupt or incompatible saves are refused with originals intact. Keep the save and its `.previous` recovery copy when investigating a failure. Closing during an animation preserves its committed result; reopening does not apply damage again.

The original Jacqueline mode remains the default when `--experiment tony` is omitted. Use the [Jacqueline guide](jacqueline-curse.md) for that mode. Each experiment has its own save kind.

## Owner verification

Joel [accepted this experiment and its final refinements on 2026-10-31](https://github.com/joeloverbeck/arkham-unfolded/issues/22#issuecomment-6536278638), reporting that the new features worked well and that #22 was satisfied. This is the owner's acceptance of the bounded playing experience; automated rules, process-continuity and rendering checks retain their separate evidential limits. The routes below are retained as a reference, not an outstanding acceptance request.

The original allowance was **at most two sessions of 20 minutes each, including retries**, counting playtesting already performed; presentation follow-ups did not reset it. No additional review session is selected here. Random failures are valid outcomes; the routes are suggestions, not a required winning sequence.

Begin with a fresh turn. If the app resumes an earlier one, finish its pending test, then end the turn at an action boundary and use **Start another Tony turn**. The earlier record is archived.

### Session 1: manage the threat and resume an interrupted test

1. Launch with the command above. Confirm the in-app heading says **Tony turn**. Focus an investigator with a portrait button, then try **Scenario**, **Test** and **Overview**. Left-click a card, choose **Inspect**, scroll where necessary, and close with **Close** or `Esc`. Try WASD panning, orbit, zoom and `Home`. Check that labels and controls remain readable and navigation has not changed the acting investigator.
2. Select Tony's card and read the attack-of-opportunity warning for **Gain resource**. Choose it and observe one attack stage with both **+1 Damage** and **+1 Horror**. After it finishes, **Undo last choice** before any reveal. Resources should return to five, damage/horror to zero, and ordinary actions to three; the bonus remains available. The history should retain an intelligible undo entry.
3. Select Internal Injury, then click **Remove Injury** after reading its cost and warning. Expect one ordinary action and the bonus remaining, one combined damage/horror attack stage, and Injury removed. Try **Finish animation** or reduced motion if useful; neither should repeat the attack or change the totals.
4. Select the Ghoul and choose **Fight**. Select and confirm any card commitments, or choose **Commit no cards**, then use **Reveal chaos token** at the lower left. The camera should focus **Test**. Before resolving it, note the token, committed cards and counters; click **Save live turn**, close the window and relaunch with the same command. Those details should be identical, without another cost or attack. The reveal itself must not be undoable. Select the revealed token, choose **Resolve test** and finish with the available choices. The first Fight spends the bonus; later Fights spend ordinary actions. Two successful basic Fights defeat the Ghoul, pay one resource and put it in the encounter discard, but failures are legitimate.

### Session 2: try different tactics and review the history

1. After the first turn ends, click **Start another Tony turn**. Select the Ghoul and try **Evade**, then investigate or gain resources. A successful Evade leaves the Ghoul exhausted and disengaged and removes opportunity-attack warnings; a failure leaves its threat in place. Check that the choices and history explain the resulting costs. You can choose a different action sequence if it is more interesting.
2. At an action boundary, try **End Tony's turn** with actions remaining. If Injury is still present, it should deal one direct damage once. No companion turn or enemy phase should start.
3. Open action history and inspect the completed turn's explanations and final counters.

Report any failed control, confusing consequence or unreadable view; whether save/resume preserved what you saw; and whether playing the turn was clear and worthwhile. A negative or inconclusive verdict is useful. There is no need to repeat the full automated rules matrix or certify technical internals.

### Reveal and discard follow-up

Joel accepted this refinement with #22 on 2026-10-31. It was offered under **one owner review of at most ten minutes total across both experiments, including retries**, judging shortcut visibility and pacing, Test navigation, discard readability and direct inspection without resetting the original 40-minute gameplay allowance. The following checks preserve that review surface for reference; no further review round is selected.

1. In Tony's live turn, reach a reveal while viewing his investigator area. Click the lower-left shortcut: expect one token, Test focus and removal of the shortcut. Check the reduced-motion setting if the transition is uncomfortable. When no reveal is legal, click the bag and expect its scenario-reference inspection directly.
2. Inspect removed Internal Injury in Tony's discard. If it is the only card, expect immediate inspection; with several cards, expect a chooser. Inspect a defeated Ghoul in the encounter discard after its final damage stage, using an already reached or supplied position. Check that reopening preserves the destination.
3. In [Jacqueline's case](jacqueline-curse.md#build-and-run), open the shortcut's normal/reaction chooser and dismiss it without revealing. Reopen it, choose a route, and check Test focus and any legal cancellations. After the revelation completes, return to the table and inspect Rotting Remains in the encounter discard. Judge whether these controls and destinations are clear; technical rule and persistence checks remain the developer's responsibility.

## External artwork and source custody

Prepare the current collection of 42 images and one symbol font, including clue/resource counters, six class icon/background pairs, the player-card back, damage/horror counters and reveal icon, in a fresh folder; earlier Tony and Jacqueline collections are retained:

```sh
curl --fail --location \
  'https://game-icons.net/icons/ffffff/transparent/1x1/lorc/swap-bag.png' \
  --output /tmp/arkham-reveal-icon.png
scripts/dev cargo run --locked --bin arkham-assets -- prepare-tony \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --reveal-icon /tmp/arkham-reveal-icon.png \
  --output "$HOME/.local/share/arkham-unfolded/tony-assets-v5"
scripts/dev cargo run --locked --bin arkham-assets -- check-tony \
  "$HOME/.local/share/arkham-unfolded/tony-assets-v5"
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop &&
target/debug/arkham-tabletop \
  --experiment tony --art-root "$HOME/.local/share/arkham-unfolded/tony-assets-v5"
```

Use the in-app asset folder selection to save a Tony-only preference in `$XDG_CONFIG_HOME/arkham-unfolded/tony.toml` (normally `~/.config/arkham-unfolded/tony.toml`). Jacqueline keeps its existing `config.toml`. Without an override or saved choice, Tony still selects `$XDG_DATA_HOME/arkham-unfolded/tony-assets-v4` (normally `~/.local/share/arkham-unfolded/tony-assets-v4`). Preparing v5 changes neither that default nor saved preferences; select the fresh folder explicitly to use its new counter artwork. The [README launch command](../../README.md#launch-and-play-the-tony-turn) uses this machine's verified v5 collection at `/home/joeloverbeck/.cache/agent-tmp/arkham-73-assets/tony-assets-v5`. Joel accepted the shared counter refinement under [#73](https://github.com/joeloverbeck/arkham-unfolded/issues/73) on 2026-11-01 after a complete prepared Party attempt through Act advancement and inspection of the recent visuals. The [acceptance record](study-escape.md#counter-acceptance-and-verification-reference-73) distinguishes that reported Party use from the separate technical checks of Tony and Jacqueline.

Startup reports the experiment, resolved folder and completeness. `--art-root` is temporary; incomplete or outdated collections block play and offer recovery. The program reads verified prepared files, never the source archive or download location. A new collection must use a fresh destination; partial/conflicting collections are preserved and reported. All third-party image/font bytes remain outside Git.

The icon is [Swap bag by Lorc](https://game-icons.net/1x1/lorc/swap-bag.html), licensed [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/). Preparation requires the selected unchanged 5,829-byte PNG with SHA-256 `54673d4cc97a84c90cab1d0a38b46183eb182c567493de676d7124ff3d70fe3e`; it rejects other bytes and retains attribution in the recipe and portable manifest. The application may tint the white silhouette. Older Tony v1/v2/v3/v4 collections remain available for diagnostic inspection, but cannot admit play. Saved choices are never silently switched to the fresh collection. An older collection's check reports incomplete new requirements without changing its contents.

The implementation consumes the exact source contract and independent worked expectations in #22, with original English faces, Rules Reference, FAQ 2.5 and original campaign guide under the accepted precedence. It does not refresh taboo or prepare a campaign. The unchanged [source manifest](../research/issue-1-evidence.json) identifies retained PDFs and deck inputs. On 2026-10-28 all nine source files, ten selected card faces and Study's revealed reverse matched their declared hashes. Their exact bytes were also copied to owner-controlled external custody at `~/.local/share/arkham-unfolded/source-evidence/tony-turn-v1` and verified by readback. Its `manifest.json` SHA-256 is `fb3423f0efc23c2bff9b0b6755c8b5b567e6edd6b1d00ddc7173d45b1274d244`. This is retained same-machine evidence, not an independent backup; preserve it while source fidelity or recovery relies on it.

## Shared player driver and checks

```sh
scripts/dev cargo build --locked --bin arkham-driver &&
target/debug/arkham-driver --experiment tony --save /tmp/new-tony-turn.json
```

The JSON-line protocol supports `observe`, `choose` with current `revision` and offered `id`, `save` and `quit`. Use `--load PATH` in another process to resume. Observations omit future card order and random state; deterministic token injection is a privileged test facility, never a player command. The UI and driver submit to the same rules and persistence authority.

Run `scripts/check`. Decision tests use #22's independent witnesses, including additional-action spending, opportunity attacks, all Standard faces, commitment restrictions and reveal barriers. Continuity tests cover failed-save retry, save refusal, restart archives and fresh-process continuation. Native inspection follows the existing [checked capture/input procedure](jacqueline-curse.md#developer-native-verification), using isolated saves/settings and complete prepared artwork. Screenshots establish inspected rendering only; interaction and human acceptance require their own evidence.

### Reproduce a visual state

Use legal driver choices to reach a state, then pass its save to the immutable native launcher. Obtain `native_approval` through the [shared desktop procedure](jacqueline-curse.md#launch-capture-and-interact) before launching. This example produces an opportunity attack's combined damage/horror counters and a held random chaos token:

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop --bin arkham-driver
tony_review=$(mktemp -d /tmp/arkham-tony-visual.XXXXXX)
target/debug/arkham-driver --experiment tony --save "$tony_review/held.json" \
  > "$tony_review/driver.jsonl" <<'CHOICES'
{"op":"choose","revision":0,"id":"investigate"}
{"op":"choose","revision":1,"id":"commit:none"}
{"op":"choose","revision":2,"id":"reveal"}
CHOICES
scripts/tools native launch \
  --approval "$native_approval" \
  --binary target/debug/arkham-tabletop --output-dir "$tony_review/render" \
  --experiment tony --load "$tony_review/held.json" \
  --art-root "$HOME/.local/share/arkham-unfolded/tony-assets-v4" \
  --screenshot held.png --quit-after-screenshot
```

Inspect the driver's returned observations for successful choices and `render/held.png` for the actual result. The launcher copies the save and preserves the original. For a lone damage counter, use a new driver save and only `{"op":"choose","revision":0,"id":"end_turn"}`; Internal Injury deals one damage. For startup refusal, use a missing folder with another isolated save and expect the recovery window without game progress.

Reopening captures the committed state; it does not resume an interrupted live animation. To check motion, use a fresh live run and the checked native input path. Compare shared objects with the [existing visual behavior](jacqueline-curse.md#visual-and-wording-review) and record any unavailable interaction separately. For intermittent camera lag, use the [bounded profiling procedure](jacqueline-curse.md#capture-a-laggy-camera) while the symptom is present.

Joel's two sessions have a combined ceiling of 40 minutes including retries: one threat-removal turn and one that evades or accepts opportunity costs for another benefit, with interruption and review of action history. Only Joel judges clarity, pacing, control burden and whether the turn is worth playing. This experiment does not establish a playable scenario, campaign, reusable engine or recurrent fitness.
