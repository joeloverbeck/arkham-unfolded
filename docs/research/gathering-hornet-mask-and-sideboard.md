# Gathering playtest findings: Hornet's Nest, Sparrow Mask and enemy sideboard

Investigation dated 2026-11-08 for [issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89),
at `a095a2d3cd082037ceed325df5670ed98e2588cd`. Joel reported these four problems
during native play and invoked `grill-with-docs`. The investigation below records
the original findings and accepted design; the implementation result follows it.
The [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
retains the broader Gathering acceptance boundary.

## Kicking the Hornet's Nest

`src/gathering/encounter_search.rs:498–505` presents `search:finish` as the
reported reward/shuffle button. `src/gathering_tabletop/controls.rs:732–755`
automates `search:finish-event` but omits this continuation. The production
automatic dispatcher already waits for presentation and commits revision-bound
choices through the usual saving/error boundary. This is a concrete explanation
of the report, without a new native reproduction.

The selected card `09068` awards one clue at the investigator's location and
resources equal to the selected enemy's health after the successful enemy
instruction. Two resources is one case, not a constant. No available clue means
no clue transfer; it does not prevent gaining resources. An unsuccessful search
gives neither reward. The existing [resolution research](gathering-resolution-presentation.md#kicking-the-hornets-nest)
records the original card image and the search/no-match interpretation.

The investigation also found an ordering problem: the code pushes an AfterSpawn
continuation below EnemyEntered, allowing Bounty Contracts before rewards
(`encounter_search.rs:686–707`, `runtime.rs:2797–2804`).
`tests/gathering_tony.rs:87–95` explicitly expects that sequence. The governing
Rules Reference p20, **Then**, instead gives the post-then effect priority over
indirect consequences of the preceding effect, including its illustrated
"After" reaction. Bounty Contracts (`06010`) is an after-entry reaction.

The proposed sequence is spawn, automatic reward, optional bounty decision,
then automatic remaining shuffle/discard cleanup. The reward-before-reaction
conclusion follows the general Then rule. The precise placement of the final
shuffle after that reaction is a source-derived continuation interpretation,
not a located card-specific publisher ruling. Do not preserve the existing test
as the oracle merely because it passes today.

Keep actual enemy selection and eligible reactions. Do not auto-select every
single-option choice. Retain no-match disclosure. Retract the resolution tray
only after motion finishes and nothing else needs it: a resident Premonition,
for example, still owns space. Saving/reopening a pending mandatory continuation
must finish it once without repeating rewards, costs or shuffling.

## Sparrow Mask offerings and control limit

The original `10111` card uses **offerings**, a distinct type of use, not charges.
The rules model already represents it as `UseKind::Offering` and initializes two
uses (`src/gathering/cards.rs:788–801`). The missing mapping in
`src/gathering_tabletop/card_counters.rs:21–30` falls through to the generic
`Uses 2` caption in `play.rs:837–839`.

Propose one compact solid rounded-shield counter with the supplied white chalice
and drops, a fixed slate-blue body (`#40566D`) and a separate readable count.
Keep its scale/thickness consistent with existing asset counters. A continuous
backing supports the disconnected drops and narrow stem; extruding only the
individual white shapes would fragment the token. Shape and icon distinguish
it from round ammo/bounty/charge counters. The color is independent of class.
Keep the supplied image unchanged, with transparency, and the numeral clear of
the icon. Retain the counter at zero while the card is in play, since Sparrow
can replenish it. Existing `tabletop_counters.rs:107–114` needs this explicit
zero-retention case.

Remove Sparrow's redundant `Uses N` caption through the typed counter mapping;
preserve its title, status and full inspection text. Other fallback captions,
including Backpack's attachment information, are not part of this removal.
The existing [Patrice investigation](issue-89-rules-patrice.md#p6-replenishment-cancellation-and-another-tests-limit)
records the spend/replenish obligations. Existing tests exercise those rules,
but were not rerun during this investigation.

The one-Mask limit is already enforced: `src/gathering/player_cards.rs:601–607`
refuses a new Mask while that investigator controls one, before enumerating
slot replacements. Payment rechecks legal choices. Sparrow itself has no slot
symbols. Official FAQ 2.5 §2.3 supports the per-investigator play-area limit.
It is therefore misleading to create a new mechanical Mask slot or allow
slot-style replacement of the existing Mask.

Propose `Mask limit: 1/1` beside the existing printed slot strip when a Mask is
controlled, with an inspection explanation. Derive it from committed state so
exhaustion does not release the limit, other investigators remain independent,
and save/reopen and Undo agree with the actual controlled assets. A dedicated
second-Mask refusal test was not found; targeted verification should include
refusal without paying costs and legal play after the existing Mask leaves.

## Supplied icon and attribution

The inspected source is `/home/joeloverbeck/Downloads/Offerings.png`, a 512 × 512
PNG with SHA-256
`a358bb3ab1972227cc8ffc47bfc88c4bddb777e41e1867714662f544b41fb6c9`.
It is byte-for-byte identical to the freshly downloaded
[white transparent original](https://game-icons.net/icons/ffffff/transparent/1x1/lorc/chalice-drops.png).
The [source page](https://game-icons.net/1x1/lorc/chalice-drops.html) names Lorc and
[CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).

Proposed credit: **“Chalice drops” by Lorc, licensed under CC BY 3.0. Artwork
unchanged.** Link the title to its source page and the license to its deed.
The slate-blue backing is separate application geometry. Record attribution
and unchanged source identity in the selection recipe, prepared manifest and
README, following the existing Swap bag precedent.

There is no in-app Credits surface in the inspected code. To meet the requested
in-app credit, propose a small Menu → Credits view with these links, also carrying
the existing Lorc Swap bag credit. Prepare a fresh complete external collection
using the existing asset tool, retaining the earlier collection and original
download. Image bytes remain outside Git. The current SourceInput variants need
an explicit source for this owner-supplied PNG; ordinary runtime loading already
follows the recipe roles. Missing required offerings art must block 3D admission
under the existing artwork contract, not silently omit the counter.

## Enemy sideboard

`src/gathering_tabletop/enemy_board.rs:101–114` draws a 0.05-thick cuboid.
Ordinary surfaces in `src/tabletop_layout.rs:485–513` use a 0.30-thick body and
0.03-thick inset top. This explains the reported sheet appearance.

Use the existing table construction for the sideboard, retaining its navy/steel
palette, position, footprint, full-size cards and location grouping. Keep its
scene ownership correct when observations rebuild the dynamic board. This is
a local consistency repair, not a new table-layout architecture.

## Sources and evidence limits

The card-rules investigation inspected the original selected `09068`, `10111`
and `06010` images and retained official rules. Live publisher PDF retrieval
failed; retained bytes were checked against `issue-1-evidence.json`:

- Rules Reference: `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`.
- Legacy FAQ 2.5: `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`.

No gameplay, renderer or continuity tests were executed and no native window
was launched. The token design is a proposal awaiting real rendering/readability
checks. The proposed checks are successful Hornet completion with bounty choice
and zero-clue cases; resume without duplicated effects; offerings spend/zero/
replenishment; Mask refusal and printed limit; and the sideboard at ordinary
camera angles. These checks do not establish complete Gathering acceptance.

Joel confirmed shared understanding and authorized implementation on 2026-11-08:
“Yes, as you recommend. I authorize implementation once you finish with the design.”
The [activation record](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6720472016)
binds the accepted four-part repair and two-hour aggregate allowance. The design
is complete; local implementation and checks are authorized, publication remains
separate. The existing public Session decision seam and production tabletop
registration/continuity seams remain the verification boundaries accepted for #89.
Issue #89 remains the authority for scope, allowance, publication and acceptance;
this supporting note creates no separate queue. No new ADR is warranted by
these local repairs. Accepted display details belong in `tabletop-presentation.md`;
the offerings/charges distinction belongs in the glossary when recorded as a
resolved term.

Consulted for this investigation and proposal: AUTH-1–3, PURP-1–6, WORK-1–6,
EVID-1–10, SEM-1–2, REP-1–3, AGENT-5, OPS-1–7 and GOV-1–3 of constitution
2026-09-30.1. Source inspection is evidence, not acceptance of a changed rule,
implementation, publication or playing experience.

## Implementation and verification

The owner-authorized local repair was implemented on 2026-11-08 against the
baseline above. Code publication and native owner acceptance remain separate.

- Hornet selections now apply the clue/resource reward before the after-entry
  bounty reaction. The remaining mandatory continuation uses the existing
  automatic, revision-bound save path; no extra reward button is required.
  Actual enemy selection, optional bounties and no-match disclosure remain.
  The existing drawer lifecycle closes it after presentation and movement finish.
- Old recorded enemy selections retain their original meaning during private
  audit reconstruction and Undo. New Hornet selections carry a distinct decision
  identifier; a defaulted `reward_applied` continuation field prevents duplicate
  rewards. Old saves already paused before the reward complete it once. This
  preserves compatible records, without repairing previously incompatible saves.
- Offerings have the accepted slate-blue rounded-shield body and unchanged icon,
  with their count retained at zero. Sparrow no longer uses the fallback caption.
  The existing Mask refusal remains, and controlled Masks are printed as a limit
  beside slot usage. Inspection explains the distinction from equipment slots.
- The enemy sideboard calls the ordinary surface constructor: a 0.30-thick body
  and 0.03-thick inset. Both generated pieces participate in dynamic scene cleanup;
  the original tables retain their geometry, and the sideboard retains its layout.
- Menu → Credits includes the Lorc sources and CC BY 3.0 link. The recipe,
  manifest and README retain attribution. The complete v5 collection is installed
  at the ordinary default path, with its v4 predecessor retained separately.

Retained evidence is under
`local/verification/a095a2d/hornet-mask-sideboard-20261108/` (ignored, local only):

- `hornet-mask-full-check-2`: full `scripts/check`, finished with exit 0 and an
  unchanged source identity. This includes strict Clippy, tools, rules, continuity,
  tabletop registration and renderer-policy checks. The application library had
  353 passing tests and eight separately gated ignored checks. Focused additions
  cover reward priority, no-clue resources, legacy audit/pending saves, automatic
  cleanup and drawer closure in both motion modes, offerings at 2/1/0, and Mask
  refusal without costs and per-investigator ownership.
- `hornet-mask-real-art-2`: the separately selected real-collection test loaded
  every required image and the symbol font successfully. The earlier attempt
  timed out waiting for the full check's checkout lock (exit 75); its record is
  retained, and the unchanged check passed after that build finished.
- Preparation and the effective-launch check both verified 146/146 assets.
  The installed offerings PNG matches the supplied hash above. The preceding
  launch check correctly refused the v4 collection, which lacked offerings.
- `hornet-mask-build`: the ordinary tabletop and driver executables built
  successfully. `hornet-mask-owner-save` and `hornet-mask-owner-copy-resume` loaded
  the latest admitted ordinary attempt, finished its legacy pending reward on an
  isolated copy, and reopened the result without duplicating it. The original
  save's SHA-256 remained unchanged. The copied save stays outside Git.

The first full check caught an unreachable wildcard after all use types gained
counter mappings; removing that obsolete fallback fixed the strict lint. An
initial drawer assertion stopped during its closing animation; the final check
waits for the automatic presentation chain and closure, with a bounded timeout.
Review snapshots and independent Standards/Spec reports are retained alongside
the check records. No native window or browser link was opened in this repair.

For owner verification, run `target/debug/arkham-tabletop` from this checkout,
choose The Gathering → Continue, and allow the pending Hornet resolution to
finish. Confirm the discard and drawer closure; inspect Sparrow's offerings and
Mask limit, Menu → Credits, and the enemy table at normal camera angles. The
automated checks do not establish native icon/text readability, animation pacing,
browser-link behavior or complete Gathering play. If one fails, preserve the save
and report that specific observation under #89 rather than accepting the repair.
