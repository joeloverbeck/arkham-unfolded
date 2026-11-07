---
status: accepted
---

# Retire the experiments and select actual scenarios

Accepted on 2026-11-07: retire the standalone Jacqueline, Tony and Party
experiments, including their executable paths and exclusively owned code. They
served their stepping-stone purpose and have no intended playing use. Hiding
launch choices alone would retain their maintenance burden. Preserve the rules,
presentation and persistence that Gathering actually consumes.

Use scenario selection as the application entry now, rather than deferring it
until another scenario exists. It needs neither a live game nor artwork to
browse. Offer only original Gathering on Standard with the three fixed decks,
visibly in development. New attempts have separate saves; Continue and Open
preserve existing attempts. Returning to selection saves the pending decision
first and keeps play active if saving fails.

Retire experiment save readers without migration or deletion of personal files.
Preserve Gathering attempts loadable at this decision's baseline, existing asset
collections/preferences, the headless driver and artwork admission/recovery.
This narrows the experiment-specific scope of ADRs 0001–0004; their independent
contracts survive. Historical experiments do not establish current playability.

The [play contract](../foundation/play-contract.md#scenario-entry) owns entry
behavior and [README](../../README.md#launch-and-resume-the-gathering) owns launch
instructions. The [investigation](../research/experiment-retirement.md) and
[issue #127](https://github.com/joeloverbeck/arkham-unfolded/issues/127) retain
implementation and verification evidence. Complete Gathering play remains the
scope of [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89).
