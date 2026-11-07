# Gathering: wagers, keys and token layout

2026-11-06. Source fixed point: `be7f58e5ab4cf6a611ea60d844bc33ef44500222`.

This investigation supports Joel's `grill-with-docs` request under
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) and its
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060).
It addresses four reports from actual Gathering play. The initial working tree
was clean. Source inspection, the supplied screenshot, inspected local artwork
and existing rules tests support the findings below. No new native reproduction,
runtime repair, asset installation or complete-scenario acceptance occurred.

## Watch This!: hidden wager controls

The selected original card is `03233`. Its printed text permits commitment only
to the controller's own test, an additional commitment cost of up to three
resources, and a return of twice that amount on success by at least one.
A stake of three costs three and pays six on qualifying success, for a net gain
of three. Success by zero and failure pay nothing. The original archive card was
visually inspected and hash-verified as
`e8c71acdb5b7f513646b3d85382f0a5b7e7060e26fa5967ef2dfba87ffbb014c`;
[ArkhamDB's transcription](https://arkhamdb.com/card/03233) matches it.

The engine already implements this behavior. At the fixed point:

- `src/gathering/tony_effects.rs:144–171` offers per-copy stakes from zero to
  three, limited by resources remaining after other pending stakes.
- `src/gathering_tabletop/controls.rs:338–341` routes those choices to the
  selected card's menu. Selecting a commitment closes that menu
  (`src/gathering_tabletop.rs:657–679`), so the player must reopen the card to
  discover its wager options.
- The commitment summary lists only icon contributions
  (`src/gathering_tabletop/ui.rs:365–389`). Confirmation is immediately offered
  (`src/gathering/runtime.rs:1341–1345`), and an unspecified stake becomes zero
  (`tony_effects.rs:228–237`).

This explains the reported missing prompt; it is not missing payout logic.
Choosing or changing a pending stake does not spend resources. Confirmation pays
once; deselection removes the unpaid stake. Own-test eligibility, payment,
success margin and per-copy persistence must be preserved.

**Accepted Q1:** selecting Watch This! immediately exposes an explicit wager
choice: **0 resources — icons only**, or each affordable amount from **1–3**.
Commitment confirmation waits for that choice. Show the selected stake and allow
editing or deselecting while commitment remains pending. Do not silently select
zero. Two copies share the available resource pool but retain separate stakes.

The existing rules tests were run once through `record-check`:

```sh
scripts/tools record-check --output-dir /tmp/watch-this-existing-tests-20261106 -- \
  scripts/dev cargo test --locked --offline --test gathering_tony watch_this -- --nocapture
```

Result: **2 passed, 0 failed**. `run.json` is `finished`, exit `0`, and its before
and after source fingerprints match. Byte-identical log and manifest copies are
retained under ignored
`local/verification/be7f58e5ab4cf6a611ea60d844bc33ef44500222/wagers-keys-investigation/`.
`tests/gathering_tony.rs:1171` onward covers
stake editing/deselection, payment, payout, ties/failures, save/reopen and refusing
another investigator's test. This verifies existing headless rules, not native
wager discoverability. The new regression must exercise the real card-selection
path, explicit zero, limited resources, two copies, edits/removal and reopen.

## Enemy Inspect button

The supplied image
`/home/joeloverbeck/Pictures/Screenshots/arkham-too-much-info-enemy.png`
was inspected. Its SHA-256 is
`4d8b8baf4835a5c0da2ca20bf023078661073de32a5631b579e58b7a0e729f33`.
The Flesh-Eater menu repeats readiness, damage, doom, bounties, engagement and
location inside its Inspect button, even though the selected card, counters and
menu heading already provide that context.

`src/gathering_tabletop/controls.rs:1178–1181` unconditionally appends enemy
inspection details to the button label. The details originate in
`src/gathering_tabletop/play.rs:293–320`, which also retains each physical enemy's
identity for inspection.

**Owner-requested correction:** a single selected enemy's button says only
**Inspect**. Keep its exact physical-card target and full inspection contents.
This does not require making a multi-card chooser's different targets all say
Inspect: existing names, copy distinctions and relevant chooser information
remain available where they distinguish separate targets. The bounded correction
removes redundant information from the pictured single-target action.

## Old Keyring: mandatory bookkeeping and meaningful order

The selected level-zero card is `60507`, with two **keys**, a distinct use type.
Its investigation reduces shroud by two; success removes one key, failure does
not, and an empty Keyring is discarded. The
[card text](https://arkhamdb.com/card/60507) governs those effects.

`src/gathering/test_cards.rs:469` queues mandatory key removal after success;
`:525–533` already removes the key and automatically discards the asset at zero.
The successful investigation separately queues its result
(`src/gathering/runtime.rs:3467`). Even an ordinary success therefore has two
effects, while automatic continuation at `:4112–4122` handles only one remaining
effect. The offered wording at `:1299` makes key removal look like an ability the
player must activate. This is unnecessary ordering interaction, not an optional
key cost or absent zero-key discard implementation.

The publisher's [Rules Reference, p. 26, ST.7](https://images-cdn.fantasyflightgames.com/filer_public/50/be/50bed4be-034c-4ed5-9ce6-8509ce8d3352/ahc60_rules_reference_eng-compressed.pdf)
allows the testing investigator to choose the order of multiple test results.
The retained PDF was read and verified against SHA-256
`14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`.
Together with [Resourceful's text](https://arkhamdb.com/card/03039), this yields a
real selected-deck exception: with one key left and Resourceful committed,
removing the key first discards Old Keyring and makes it a recovery target.
Resolving Resourceful first cannot recover the still-in-play Keyring. This is a
source-derived consequence, not a separate publisher ruling about this pair.

**Accepted Q3:** ordinary successful uses apply key removal and zero-key discard
automatically. Preserve any genuine ordering decision in the test/result panel,
without requiring a visit to Patrice's contextual menu. Key removal itself is
never optional. Automate only cases whose order has no consequential difference;
preserve intervening opportunities and uncertain or consequential ordering.
This is a bounded repair to the selected card's result handling, not a general
effect-equivalence system or automatic strategy selection.

`tests/gathering_player_events.rs:171–273` already covers success, failure and
last-key/Resourceful recovery. Its helper explicitly selects key removal, so it
does not prove automatic bookkeeping. Those tests were read, not run. Future
checks must demonstrate ordinary automatic 2→1→discard, failure preserving a
key, both meaningful Resourceful orders, and exactly-once behavior after reopen.

## Local key artwork

The source archive's other-resource catalog contains 6,308 entries. A bounded
name search found 84 key-named records covering 41 distinct image asset IDs.
White, Black and Yellow Innsmouth keys and representative Circle Undone and
Edge of the Earth candidates were inspected. The latter candidates carry chaos
symbols or printed modifiers and fit this use less well.

**Accepted Q4:** use the neutral White Key image, a black key on a pale illustrated
tile, for one **Keys** counter displaying the remaining total. It changes from
two to one and leaves play with its card at zero. Preserve the whole silhouette
rather than cropping the key's ends to force a circular image. Reuse the existing
counter scale and numeric treatment, with a named Keys substitute when absent.

- Source: `images/other/official/en/the_innsmouth_conspiracy/White Key--ImageURL--cff3dc6e3390a7298f07.jpg`
  under `/home/joeloverbeck/Pictures/Arkham Horror LCG Resources`.
- Size: 170 × 170 pixels.
- SHA-256: `b74d80672dd82ee194d68b3c81e523fefca49a1e9b74dd79ebde7624a897e4aa`.
- Catalog identity: `cff3dc6e3390a7298f0784e17e130ede03db2179a106cec1010e3cf2e882611d`.
- Original: `originals/objects/b7/b74d80672dd82ee194d68b3c81e523fefca49a1e9b74dd79ebde7624a897e4aa.jpg`.

`UseKind::Key` already exists (`src/gathering/cards.rs:48,998`), but the counter
mapping at `src/gathering_tabletop/card_counters.rs:21–27` omits it. It falls back
to a generic Uses caption (`src/gathering_tabletop/play.rs:758–760`). The repair
should add the selected key presentation asset through
`resources/gathering-asset-selection.json` and existing preparation/installation.
Source bytes stay unchanged, image payloads remain outside Git, and a missing
image cannot alter rules. A fitting local asset exists; no online replacement or
image-generation service was needed or used.

## Jacqueline: cancellation scope and caption spacing

The code confirms the reported one-plus-two arrangement.
`src/gathering/tokens/presentation.rs:21–70` separates ordinary draws from
Jacqueline-added draws. `src/gathering_tabletop/play.rs:1193–1221` starts each
group on a separate row. That is provenance, not a privileged first token: all
three belong to Jacqueline's cancellation scope. Her
[card text](https://arkhamdb.com/card/60401) and
[Legacy FAQ 2.5, p. 17](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf)
support declaration before seeing faces and simultaneous revelation. Existing
qualified interpretations for nested effects remain unchanged.

Columns and rows are 1.3 world units apart
(`src/tabletop_layout.rs:274–280`), while token disks and selection rings are
0.92 and 1.16 units wide. Fixed 84-pixel captions do not shrink with projection
(`src/tabletop_tokens.rs:72,139–168`). After caption offsets, the second group's
heading sits 0.4 units beyond the first token row, beside that row's token-caption
anchor at +0.55. These competing caption bands explain the crowding risk.

Existing [native evidence](gathering-results-tokens-and-slots.md#retrospective-follow-up--activated-2026-11-06)
at `/tmp/gathering-retro-resolve-native-3/resolution.png` visibly shows overlapping
captions in the same renderer's crowded nine-token resolution tray. It supports
the shared layout concern, not an exact reproduction of the reported three-token
Test view. No new desktop session was launched for this investigation.

**Accepted Q2:** show an ordinary Jacqueline reveal as one comfortably spaced,
centered row of three tokens. Use one cancellation instruction and a visible
selection count. Proposed concise wording is **Jacqueline — choose tokens to
cancel**, followed by **Cancel 2 non-auto-fail tokens, or cancel the auto-fail
token**, adapted to the actual legal choices. Token captions need only a stable
position, face and relevant selected/canceled state. Avoid repeating availability
text under every token.

Retain source-labelled nested groups when they carry real choice structure;
do not flatten Dark Prophecy/Olive composition or confuse earlier held tokens
with a curse's additional reveal. Keep canceled tokens visibly distinct and
preserve their physical custody. The spacing repair also covers final retained
and canceled rows and the shared Resolve renderer.

Future verification must cover ordinary three-token selection, the auto-fail
branch leaving two retained tokens, protected tokens, curse follow-up, nested
effects, retained/canceled views, missing artwork and save/reopen. Existing
`src/gathering_tabletop/tests/token_guidance.rs` checks choice guidance and
buttons, not nonoverlapping physical-token captions. Projected geometry checks
and native Test/Resolve inspection are both needed before a readability claim.

## Decision record and limits

Joel accepted Q1–Q4 during this interview: explicit wagers, a unified ordinary
Jacqueline row, automatic Keyring bookkeeping with direct consequential choices,
and the local neutral key art. His original report requests the single-enemy
Inspect simplification. These decisions are recorded in
[ADR 0002](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-wagers-keys-and-chaos-token-spacing).
Keys are named separately from charges in the glossary. GitHub #89 remains the
active work and acceptance surface; this document is supporting evidence.

Joel subsequently instructed: **Proceed with the implementation.** This activates
one bounded implementation and verification increment for these four problems
through the existing Gathering paths, from `be7f58e5ab4cf6a611ea60d844bc33ef44500222`.
No adjacent card work, generic rules machinery,
save migration, publication or native-session allowance is selected here. Stop
and return on governing-rule ambiguity, a materially larger capability, or the
repository's existing appetite and non-convergence limits. Full Gathering play
and owner acceptance remain outstanding.

Consulted constitution revision 2026-09-30.1: `AUTH-2`–`AUTH-3`,
`PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`,
`SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `AGENT-5`, `OPS-1`–`OPS-7`
and `GOV-1`–`GOV-3`. Investigation establishes only the evidence described above;
accepted design does not establish implemented behavior or native readability.

## Implementation and verification

Watch this! now requires an explicit per-copy wager before commitment confirmation.
The decision panel immediately displays compact wager controls, including zero;
the full accessible labels explain the payout. Pending stakes remain editable and
share the investigator's available resources. Deselecting a copy clears its stake.

Single-enemy inspection uses **Inspect**. Multi-target menus still identify each
physical enemy, and inspection keeps the full state description.

When Old Keyring's only simultaneous result is the investigation itself, the
runtime removes the key and completes the investigation automatically. Other
simultaneous effects retain their ordering choice, now directly available in the
result panel. The key counter reuses the archive's neutral White Key unchanged;
collection v3 adds `token/key`, retaining recognition of v1 and v2 collections.

Ordinary Jacqueline reveals now form one centered three-token row. Nested reveal
groups and curse follow-ups keep their provenance. Gathering's Test and Resolve
layouts use wider spacing and concise numbered face captions, with selected,
ignored and canceled states retained. Other modes keep their existing token layout.

Development saves containing old implicit-zero Watch this! commitments can be
incompatible because continuation validation now requires an explicit wager.
Existing save bytes are preserved; this increment supplies no migration. Use the
README's fresh-start instructions if an older save is refused.

Additional consulted rules for implementation review and handoff: `AGENT-1`–`AGENT-4`.
Native review requires a fresh desktop allowance; the investigation's earlier
allowance did not authorize a later window session. No remote publication is
included in this implementation instruction.

### Local checks and installed artwork

`scripts/check` passed, including 472 library tests (five deliberately ignored),
the integration tests, formatting, Clippy, tool checks and renderer checks.
Focused tests first reproduced the missing wager prompt, manual Keyring result
and absent Keys counter. The token geometry test reproduced the one-plus-two
split and overlapping Resolve captions before the changes. Current checks cover
1280×720 and 1600×1000 at display scales 1 and 2, using real font layout and the
production UI registration. They establish the exercised geometry and control
behavior, not native readability or physical input.

The pre-native code/check fingerprint is
`41ef0d90f3763b1ee92d2a067e767a666eba5079e2f839380b79952925053493`.
Finished exit-zero records and their logs are retained under
`local/verification/be7f58e5ab4cf6a611ea60d844bc33ef44500222/wagers-keys-implementation/`:
`gathering-wagers-keys-full-check`, `gathering-wagers-keys-build`,
`gathering-wagers-keys-real-art` and `gathering-wagers-keys-resolve-preflight`.
Their original paths are the same basenames beneath `/tmp/`. Later documentation
additions report these results; they do not change the checked executable code.

The executable used for the native captures has SHA-256
`e1d1def6532c8ed29158475a2c37c40ee747c7af4f944d4d5fed440bcd06d1bb`.
The normal installed artwork root now verifies **144/144**, and Bevy loaded every
verified image and the symbol font in the real-collection check. Installation
used a verified fresh sibling before replacing the stable path; the previous
collection remains at `~/.local/share/arkham-unfolded/gathering-assets-before-keys-20261106`.
The current manifest SHA-256 is
`1571bf0fdedb183fdb2b5a2b7f373b21107e0e99ec700a7f3f59279d22e4b84a`.

The current default `local/gathering/setup.session.json` is refused by continuation
validation. Its bytes remain unchanged (SHA-256
`2c781194e470944dbe85a4b9888c9e73956780abd756141b364f6947c68af1d2`).
This check does not establish whether that save was already incompatible at the
starting revision. Fresh setup succeeds through the rebuilt driver. README's
fresh-start command uses a path confirmed unused at this handoff.

### Standards review

The independent standards reviewer found **zero confirmed violations or actionable
smells** in `/tmp/gathering-wagers-keys-review-v1`, manifest SHA-256
`64348342f01f161e3bca83a3144416460702984ec3a033a4f4a2976342c3e781`.
Shared counters add only Keys-specific branches; Tony and Party keep their
existing roles and behavior. New UI tests use native `install()` registration.

One introduced framing difference remains unverified visually: wider Resolve
rows increase tray depth, which feeds the existing camera-distance calculation.
The native comparison must check neighboring source-card readability and group
headings, not infer them from token-caption geometry.

### Specification review

The independent specification reviewer found **zero confirmed findings**, no
missing implementation requirement and no scope creep in the same snapshot.
Wager editing/confirmation/payment, both Resourceful orders, enemy identity,
Keys art and ordinary/nested/curse token provenance follow the accepted design.
Native Test/Resolve comparison, group-heading separation and final retained/
canceled layouts remain unverified; full-scenario owner acceptance remains open.

The ordinary diagnostic saves `watch-wager`, `jacqueline-three` and `keyring-two`
are exported under `/tmp/gathering-wagers-keys-ordinary-saves/`. Their builder is
shared with the passing production-startup UI checks; they contain ordinary
seeded choices, not privileged fixture state. The old nine-token Resolve save
also passes the rebuilt application's read-only startup check.

### Authorized native verification and caption correction

Joel authorized the five-minute automatic desktop verification. The interval
was 2026-11-06 22:52:55–22:57:55 UTC, including cleanup. All five captures finished
and all windows closed by 22:55:21 UTC. Each run exited with code 0; no pointer
input was used. An initial command was refused before launch because the native
helper requires a plain screenshot filename; the corrected invocation started
the first window. No desktop operation continued beyond the accepted interval.

The 3200×2000 captures, run manifests, logs and disposable saves are retained
outside the repository at
`~/.local/share/arkham-unfolded/verification/wagers-keys-20261106/`, in subfolders
`jacqueline`, `watch`, `keys`, `resolve` and `fresh`. All used the recorded
`e1d1def…` executable and the verified 144-role installation. Retained images
were compared byte-for-byte with their original `/tmp/gathering-wagers-native-*`
outputs. Personal saves and prepared input saves remained separate.

The wager controls were fully visible, the neutral key and count 2 were legible,
and fresh startup showed the ordinary lead-investigator choice. Jacqueline's
three tokens formed one clearly spaced row. However, this native check identified
two defects not covered by the earlier token-caption-only tests:

- The redundant single-group heading overlapped the committed Watch this! caption.
- Resolve's depth calculation added the side-by-side source and token column
  depths. The resulting camera distance crossed the detail-label visibility
  threshold, hiding group headings and making source cards unnecessarily small.
  The retained prior nine-token capture establishes the before/after framing.

The follow-up removes the redundant physical heading for a single skill-test
group; the decision panel keeps its source and cancellation instruction. Multiple
groups retain their physical headings. Resolve now takes the maximum of the card
and token extents, preserving card-only layouts. The legend no longer promises
physical group headings where only one test group exists.

The strengthened checks use the same ordinary committed-card checkpoint as the
native capture and include actual group/card glyph bounds and visibility at the
production camera focus. Both first failed on the observed defects and then
passed after correction (`/tmp/gathering-native-caption-red` and
`/tmp/gathering-native-caption-green`). These repairs occurred after the desktop
interval. The corrected layout has not had another native capture; no additional
desktop allowance is implied. Physical-input behavior and full-scenario owner
acceptance remain separate from this static review.

The corrected candidate passed `scripts/check` again: 472 library tests (five
ignored), integration tests, formatting, Clippy, tools and renderer checks.
The build and launch-selected 144/144 artwork check also finished with exit 0.
These records are retained alongside the initial checks as
`gathering-native-caption-full-check`, `gathering-native-caption-build` and
`gathering-native-caption-installed-art`, with matching source fingerprint
`bffeaaa58eaf94fb48d0c620bfd4b204b5aa88aaa911bb4895091ac31eb1cf69`.
The rebuilt executable SHA-256 is
`768093f74a36925c810ecfc045e7f8436535d456230f53ef45293fac45271c0a`.

The independent Standards and Spec refreshes both reported zero confirmed
findings in `/tmp/gathering-wagers-keys-review-v3`, manifest SHA-256
`73622a9c91b072300eb1b04c904c9dfac4827e1b33a0e7191665c8230531d12b`.
Both retained the explicit limit that the corrected layout has not been captured
natively. This final documentation addition reports completed evidence without
changing the checked code. At the implementation handoff, changes remained local
and uncommitted; #89's complete scenario play and owner acceptance were not claimed.

### Publication authorization — 2026-11-07

After the implementation and native-verification report, Joel explicitly
authorized publication, PR creation, merge and pulling `main`. This applies to
the four fixes and the reported caption corrections above, from the recorded
`be7f58e5ab4cf6a611ea60d844bc33ef44500222` baseline. It supersedes the earlier
local-only publication boundary. The final caption corrections still lack a
second native capture; complete Gathering play and owner acceptance remain open
under #89. The PR and current issue handoff own the resulting publication status.
Publication consultation: `AUTH-1`–`AUTH-3` and `PUB-1`.

### Accepted retrospective follow-up — 2026-11-07

After the session retrospective, Joel instructed **Proceed with recommended
changes**. From merged baseline `e4328c3a170fa30f8cad00a78830e5e3985f75e0`,
this activates the three bounded verification/tooling improvements and the
CI-wait adjustment identified in that retrospective:

- Require the committed Watch this! caption to exist in the ordinary Jacqueline
  checkpoint, alongside the existing visibility, glyph-bound and overlap checks.
- Add `review-summary` for byte-limited, digest-bound pages of snapshot revisions,
  selected paths and changes since the preceding snapshot. Keep the full context
  inventory on disk and preserve snapshot/index verification boundaries.
- Explain the native launcher's screenshot filename in CLI help and the prepared
  route guidance, retaining its existing path validation.
- Wait on the existing CI watcher for up to 45 seconds per call, within the
  communication interval, instead of alternating short polls and sleeps.

The selected verification boundaries are the existing production UI checkpoint
and tool CLI. A temporary missing-caption mutation after layout failed with the
new assertion; it is removed from the final candidate. The first mutation was
applied before layout and was recreated by redraw, so its passing result was
not evidence of the asserted missing-caption condition. Summary checks exercise
identity and delta output, pagination with Unicode filenames, byte limits,
read-only behavior and refusal of changed snapshots.

These changes support future Gathering implementation and review. They do not
alter gameplay, assets, save formats or native rendering, or establish a new
native observation. The final caption fixes above still lack a second native
capture. This follow-up authorizes local implementation and verification; the
earlier publication instruction applied to PR #120. Results are retained under
`local/verification/e4328c3a170fa30f8cad00a78830e5e3985f75e0/retro/`;
#89 retains complete-play and owner acceptance.

Consulted: `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `AGENT-5`, `OPS-1`–`OPS-7`,
`EVID-3`, `EVID-4`, `EVID-6` and `REP-2`. This is a local improvement to the
existing review path, with no wider framework or reusable-platform claim.
