# Tony's prepared Gathering turn

[Issue #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22) owns this bounded experiment and its acceptance. It begins in a deliberately prepared position, not a played campaign: Tony is unarmed in the revealed Study, engaged with a Ghoul Minion carrying one bounty, with Internal Injury in his threat area. Patrice and Jacqueline remain at the Study. Play ends after Tony's turn and its mandatory effects; no next investigator or enemy phase runs.

## Launch

```sh
cargo run --locked --features tabletop --bin arkham-tabletop -- --experiment tony
```

Use the visible legal choices to fight, evade, investigate, draw, gain a resource, engage an eligible enemy, remove Injury, or end early. Tony's first qualifying Fight/Engage automatically spends his additional action. Each skill test offers card commitments, then a genuinely random Standard token draw and its resolution. The fixed hidden draw pile begins Guts, Guts, On the Hunt. Guts has no matching icons in this turn.

The panel explains remaining actions, costs and consequences. Inspect cards for readable text and available artwork. Camera controls remain right drag/arrow keys to orbit, wheel or +/- to zoom, and Home to reset. Manual movement overrides replay following.

Decisions autosave before taking effect. The default file is `local/experiment/tony.session.json`; reopening the same mode resumes it. Use **Save live session** at a pending choice. **Undo** is available only when it does not cross a committed card or token reveal. It restores the whole deterministic change and adds a new recorded revision. Replay Previous is inspection, not undo.

New attempts archive the previous run. Replay shows stored moments with Next/Previous, without executing rules or drawing again; return to live before saving. Corrupt or incompatible records are refused with originals intact. Keep the record and its `.previous` recovery copy when investigating a failure.

The original Jacqueline mode remains the default launch and retains its existing compatible save/record reader. Use the [Jacqueline guide](jacqueline-curse.md) for those records. There is no automatic conversion between experiment kinds.

## External artwork and source custody

Prepare a separate collection; existing Jacqueline collections are retained:

```sh
cargo run --locked --bin arkham-assets -- prepare-tony \
  --source "/home/joeloverbeck/Pictures/Arkham Horror LCG Resources" \
  --output "$HOME/.local/share/arkham-unfolded/tony-assets"
cargo run --locked --bin arkham-assets -- check-tony \
  "$HOME/.local/share/arkham-unfolded/tony-assets"
cargo run --locked --features tabletop --bin arkham-tabletop -- \
  --experiment tony --art-root "$HOME/.local/share/arkham-unfolded/tony-assets"
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
