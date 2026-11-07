# Retire the experiments and select actual scenarios

Read-only investigation at `9847d72791ce934f5438603eb2b38a2771674dba`,
2026-11-07. This supports the owner's `grill-with-docs` interview and the
implementation spec [#127](https://github.com/joeloverbeck/arkham-unfolded/issues/127),
published through the subsequent `to-spec` invocation. The issue owns active
scope and status; this document is supporting evidence, not a second work queue.
The [accepted decision](../adr/0005-retire-experiments-and-select-scenarios.md)
records the product boundary. No application implementation, test execution or
native session was performed for this investigation.

At final confirmation, local `main` was
`8c4e619e20f9a55dd2cbd3144ab21c26325d8af6`, the PR #126 merge of the inspected
baseline. A direct Git comparison found no tree differences from that baseline;
the source findings therefore remain applicable.

[Issue #124](https://github.com/joeloverbeck/arkham-unfolded/issues/124) is closed.
Its [completed handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/124#issuecomment-6706875824)
records artwork admission, recovery, verification and owner acceptance. It
expressly retained all four modes and excluded retiring them. This removal and
the new selection screen therefore warrant a new spec. Complete Gathering
gameplay and owner acceptance remain with
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89).

## Selected outcome

The Jacqueline, Tony and Party experiments were stepping stones. Joel no longer
wants to play or maintain them. Retire their executable interfaces and exclusively
owned implementation while retaining the investigators and every behavior The
Gathering still uses. Hiding launch choices while maintaining the old sessions,
tabletops and verification obligations would not satisfy this outcome.

The interview selected a scenario-selection screen now. Initially it offers only
The Gathering, marked as in development, with the existing Standard difficulty
and Tony Morgan, Patrice Hathaway and Jacqueline Fine starting decks. No future
scenario placeholders, new difficulty/deck choices or scenario framework are
needed for this scope.

Selecting Gathering offers **New attempt**, **Continue**, and **Open saved
attempt**. New uses a distinct unused save path; Continue reopens the most
recently selected saved attempt; Open selects an existing file. An incompatible
file is refused without modification and does not prevent starting a separate
attempt. The in-game Menu also returns to scenario selection after safely saving
the pending decision; a failed save keeps the current game open.

## Current behavior and dependency findings

Both binaries currently default to Jacqueline. Native dispatch is in
`src/bin/tabletop.rs`; driver dispatch is in `src/bin/driver.rs`. Gathering is
selected with `--experiment gathering`. There is no existing scenario chooser,
save browser or remembered-current-save selection.

Gathering defaults to `local/gathering/setup.session.json`. The same `--save`
argument resumes an existing file or starts setup when absent. Its in-game menu
offers Save live attempt, Reopen saved attempt and Restart The Gathering.
Restart archives a valid loaded predecessor and replaces the same active path;
it cannot safely serve as an overwrite mechanism for an unreadable save.

| Surface | Retirement treatment |
| --- | --- |
| `src/party.rs:10` and `src/gathering.rs:67` | Gathering reexports Party's `Investigator`. Relocate the retained identity and methods before removing Party's session. Preserve order, serialized values and displayed names. |
| `src/lib.rs`, `src/gathering.rs:68`, `src/gathering/runtime.rs:8` | Shared `Choice`, `RuleError`, `TokenFace` and `TokenStatus` live beside Jacqueline-only rules. Preserve the shared types; remove the old fixture/session/observation implementation only after separating them. |
| `src/tabletop/asset_settings.rs`, `src/tabletop/symbols.rs`, `src/tabletop.rs` | Gathering, startup recovery and shared presentation use folder selection, symbol support and readable-text conversion from the Jacqueline tabletop. Move retained support before deleting that tabletop. |
| `src/gathering_tabletop.rs:401`, `src/tabletop_camera.rs:145` | Gathering selects `CameraMode::Jacqueline`; that name controls actual camera behavior. Preserve the behavior when removing the obsolete distinction, including the layout example's consumer. |
| `src/saved_state.rs`, `src/record_replacement.rs`, `src/continuity.rs:3` | Keep generic persistence and guarded replacement. Retire Jacqueline's continuity wrapper, relocating the file-ownership contract and its Rustdoc links. |
| Standalone Tony/Party rules, continuity, native adapters and driver branches | Removal candidates after retained dependencies and tests have moved. Gathering owns its own rules session; investigator-specific Gathering modules remain. |
| `src/explanation.rs`, `src/encounter_discard.rs`, `src/tabletop_discard.rs` | Searches found no Gathering consumers. Confirm the final dependency closure before deleting these old-mode support modules. |

### Save and asset preservation

Gathering currently uses save format 3, kind `gathering-session`, application
version `0.1.0` and its pinned source revision. Module relocation does not require
changing any of those identities. Validation reconstructs state from seed and
decisions and compares the full state (`src/gathering.rs:873`): incidental changes
to history text can reject saves even when the JSON shape remains the same.

The preservation target is saves that this baseline can successfully load.
This is not a promise to repair already incompatible older saves. Preserve
pending choices, source/deck identities, randomness, legal decisions, committed
effects, safe Undo, history and stale/divergent-write protection. Unsupported
experiment saves have distinct envelope kinds and can be refused without
retaining their full readers. Leave personal records untouched.

Gathering has its own complete 144-file recipe,
`resources/gathering-asset-selection.json`, identifying
`arkham-unfolded-gathering-v3`. Retire the three experiment recipes and their
preparation/inspection entry points without shrinking or renaming Gathering's
selection. Preserve shared provenance in
`docs/experiments/{token-art,symbol-art,investigator-art}.json`.

Asset preferences are separate files: `config.toml`, `tony.toml`, `party.toml` and
`gathering.toml`. Keep Gathering's existing `gathering.toml`, selected asset root
and default `gathering-assets` location. Old personal settings and collections
can remain unused; removal does not authorize deleting or migrating them.

## Selection-screen implementation constraints

The final interview confirmation accepted the following behavior and safety
requirements:

- Ordinary native launch opens selection. Browsing, cancelling and quitting
  there do not open a rules session, create an attempt or modify a saved attempt.
  The selection screen works without scenario artwork; it is not a playable
  no-art mode.
- Explicit Gathering commands remain available for direct launch and diagnostics.
  The headless driver remains a direct, artwork-independent engineering path.
  Retired/unknown selections fail before touching saves. CLI spelling and local
  module placement remain implementation judgment.
- New reserves an unused path and only creates the initial save after the
  existing complete-artwork/loading/render-health admission. Continue/Open never
  silently reinterpret an unsupported file as a new attempt.
- Remember only a successfully opened/created attempt for Continue, preserving
  the previous usable choice on cancellation or failure. With no remembered
  attempt, explain that Continue is unavailable and offer New/Open. Existing
  custom save files remain accessible through Open and explicit commands; no
  automatic filesystem-wide discovery or save catalog is needed.
- Missing, corrupt, incompatible or wrong-scenario saves receive a readable
  error and remain untouched. New/Open remain reachable. Reopening a finished
  attempt preserves its ending; it does not silently begin another attempt.
- Returning to selection saves through the existing guarded writer before
  releasing the game. A write failure preserves the active session and offers
  existing recovery. Cancel/back navigation does not commit a gameplay choice.
- Preserve the selected scenario, attempt action and exact path through artwork
  Choose folder/Retry/Quit. Failed admission must not consume the proposed new
  attempt or create duplicate attempts. Existing in-game restart/archive and
  reopen behavior remain available.

There is a concrete native lifecycle risk: startup recovery currently closes
Bevy and `exec`s the binary (`src/tabletop_startup.rs:90`). Winit's event loop is
process-owned. A chooser followed by another independent `run()` cannot be
assumed to work. Use a bounded existing-lifecycle solution and verify actual
chooser → Gathering → chooser → Continue behavior before expanding polish.
No general scenario manager or independent launch service is justified.

## Tooling and verification that must survive

`tools/src/native.rs` and `tools/src/native/restart.rs` default to Jacqueline and
encode mode arguments. Update them for ordinary selection-screen launch and
explicit Gathering launch, preserving isolated saves, consent deadlines, process
and binary identity, shutdown and restart safeguards. Historical manifests with
an omitted selector mean Jacqueline; do not reinterpret them as a new default.
Retired or ambiguous manifests must refuse without touching their personal inputs.

`tools/src/desktop.rs` verifies exact window titles as well as process identity.
Update the recognized surviving titles without weakening ownership checks.
Gathering's capture code waits for game admission; selection-screen capture
needs UI readiness independent of starting a scenario. Existing external
desktop capture is also available. Investigator camera targets named `tony` and
`jacqueline` remain valid and are unrelated to the retired modes.

Two generic persistence suites currently run exclusively through Tony/Party:
`tests/file_replacement.rs` and `tests/file_replacement_lock.rs`. Port their
meaningful custody, installation-order, failure and lock-lifetime cases before
removing the old hosts. Adapt `tests/support/install_trace.rs`, which assumes
`--load`. Similarly preserve asset-integrity/portability/no-overwrite tests whose
fixtures currently use Jacqueline, native supervisor tests using Tony, and
Gathering tests importing `party::Investigator`.

The highest useful proof boundary is the **actual selection screen → artwork
admission → Gathering player choice → guarded persistence → reopened attempt**.
Keep the existing production driver and native-registration seams; no parallel
test-only rules path is needed.

| Verification | Evidence sought |
| --- | --- |
| Revision-bound before/after Gathering saves | Baseline-loadable setup, pending token interaction, investigator action, phase transition and later scenario checkpoints still open and continue without changed randomness or repeated effects. Reuse existing review checkpoints and independent worked expectations. |
| Real driver process | Existing `tests/gathering_driver.rs` cases retain setup, mulligans, fresh-process resume, ordinary round/mythos and privileged-operation refusal. Keep relevant investigator/token/phase/resolution/Undo tests. |
| Registered native systems | Extend `src/gathering_tabletop/tests.rs` and review-save facilities for selection, admission, safe leave/return and unchanged saves. Test the actual production registration, not only a detached UI fixture. |
| Refusal and custody | Opening/capturing/quitting selection leaves saves unchanged; unknown/retired selectors and wrong-kind saves refuse; admission failure creates no new save; invalid Continue/Open and failed save-on-leave retain recovery. |
| Existing asset checks | Keep Gathering's complete selection, decoding/font evidence, missing-art refusal, folder recovery, relocation and file identity protections. Ordinary CI stays independent of private artwork. |
| Repository gate | Run applicable focused checks and `scripts/check`; retain terminal results. Existing CI already covers Rust, tools, tabletop compilation and renderer policy. |
| Bounded native use | Exercise selection, New/Open/Continue, missing-art recovery, a real affected investigator/token decision, save/reopen and return to selection using isolated copies and authorized desktop access. Judge the visible selection and recovery flow separately from automated behavior. |

Compiler success and a reduced green test suite cannot by themselves establish
preservation. Native evidence is limited to the exercised paths; this change
does not complete or accept the unfinished Gathering scenario under #89.

## Documentation disposition

- Rewrite README around selection and Gathering. Remove the retired mode status,
  launch, settings, assets and guide links, especially the old sections at
  lines 113–220. Preserve investigator names in the supported party, current
  controls, save explanations, setup instructions and unfinished-gameplay limits.
  Correct the stale missing-art substitute statement at lines 67–68.
- Update current Gathering guidance in `docs/experiments/gathering-setup.md`:
  replace current four-mode/Party-ownership descriptions, document selection and
  attempt actions, and preserve dated evidence with its original limits.
- Move the shared native verification, consent, capture/input, performance and
  evidence-retention procedures from `docs/experiments/jacqueline-curse.md`
  (line 117 onward) into a scenario-neutral agent guide before retiring that
  runbook. Preserve safeguards while adapting examples to Gathering.
- Update live links from README, the tracker procedure, Gathering guide,
  `gathering-reveal-and-story-controls.md`, tool help, and the implementation,
  review and native-performance skill guidance. `.agents` mirrors the canonical
  `.claude` skill files; avoid duplicate edits. Apply the writing-for-agents guide
  when editing skill instructions during implementation.
- Retain the three old experiment guides as explicitly retired historical
  records, removed from current launch navigation. Preserve dated research and
  source evidence rather than rewriting it as Gathering evidence. Where useful
  provenance links target removed recipes/code, bind them to the historical
  source revision or retained equivalent.
- Narrow current foundation/ADR obligations to supported scenarios and link the
  retirement decision. Earlier replay/artwork decisions remain truthful about
  the modes they covered at their accepted revision. Preserve all independent
  rules, save, artwork and source-custody obligations.

Relevant historical research includes Tony preparation, Study preparation,
Party interaction refinements, contextual controls, action markers, attacks of
opportunity and harm, act advancement, frame pacing, startup health, replay
retirement and required artwork. Investigator names and the ordinary word
"party" are not sufficient evidence that a document or test is obsolete.

## Implementation approach and boundary

1. Bind the implementation baseline and retained Gathering regression cases.
2. Detach retained types, camera behavior, presentation/settings helpers and
   generic persistence contracts; transfer meaningful old-hosted checks.
3. Remove obsolete rules sessions, table adapters, drivers, selectors, recipes
   and mode-only tests; verify Gathering preservation.
4. Add the bounded selection/attempt flow and supervision support. Prove the
   native lifecycle and safe save/admission transitions before interface polish.
5. Reconcile current documentation, run the required checks, and present the
   bounded native evidence and candidate for owner acceptance.

No new scenarios/cards/rules, generic scenario engine, historical-save migration,
personal-file cleanup, paid services or application-code publication is selected.
The owner invoked `to-spec` to publish #127; implementation and full-Gathering
acceptance remain separate.

Joel confirmed Q5 on 2026-11-07: “Yes, ready for to-spec on that basis.” This
accepts the coherent scope and production proof boundary above, with a future
ceiling of **eight aggregate engineering hours**, including implementation,
setup, verification and review, and **one owner review of at most twenty minutes total, including
retries**. These are ceilings, not delivery estimates; this investigation does
not consume that future implementation allowance. Q1–Q5 are answered, and the
design interview is complete. This acceptance prepared the implementation
contract; the subsequent `to-spec` invocation published it with `ready-for-agent`.
Neither transition starts implementation.

Return earlier after two active hours without an inspectable increment, three
failed repair cycles at the same boundary, a material expansion beyond the
selected scope, approximately doubled expected surface, an unapproved capability,
or an exhausted engineering/owner-review ceiling. Routine internal choices and
in-scope defect repair remain delegated. At a stop, retain the exact candidate,
evidence and unresolved deficit; there is no automatic extension. This one
bounded change temporarily displaces Gathering feature work and returns to
#89's native play and owner-acceptance boundary without resetting it.

The accepted contract is now published in #127 without reopening the settled
design or production test seam. The next boundary is explicit implementation
activation. The implementing agent must recheck the source baseline and issue
before beginning work.
