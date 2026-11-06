# Gathering results, token choices and asset slots

2026-11-06; source baseline `918b2e2d3819f4b83970513950e7a36f034aee70`.
Research for Joel's `grill-with-docs` interview under
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89), whose
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
still owns complete Gathering play and acceptance. This note supports that work;
it is not a separate queue or implementation activation. Joel accepted all three
interface recommendations on 2026-11-06. The
[presentation ADR](../adr/0002-persistent-table-areas.md#compact-results-guided-token-groups-and-visible-asset-capacity)
records the selected design; this note retains its research and evidence limits.

The implemented increment received [owner playtest acceptance and publication
authorization](#owner-playtest-acceptance--2026-11-06). The earlier research and
agent-verification sections below retain their evidence as it stood at each stage.

The bottleneck is understanding the choices and consequences during actual play:
an oversized result dialog, unexplained nested token choices, and invisible asset
capacity. Joel supplied two screenshots and described the experience. Investigation
covered those three concerns, the governing card rules, relevant production paths,
and the matching retained attempt. During that research phase, no gameplay code,
personal save, or GitHub state was changed. Tests were inspected, not run; no
native session was launched.

Consulted for this bounded decision: `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`,
`WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`,
`BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`, `AGENT-1`–`AGENT-5`,
`OPS-1`–`OPS-7`, and `GOV-1`–`GOV-3`. Existing acceptance of full result details,
explicit optional choices and physical resolution trays remains in force.

## Sources and retained evidence

The selected legacy source set remains original card text, Legacy FAQ 2.5 and
taboo 10, as identified in the
[Jacqueline investigation](issue-89-rules-jacqueline.md#sources-and-effective-text).
Live publisher sources and card transcriptions were checked again; community
reviews were not treated as rulings. The publisher's nested-reveal guidance is
composed with the individual cards below; it is not an exact publisher example
of this particular nine-token draw. The existing
[nested-token investigation](issue-89-nested-tokens-followup.md) retains that
derivation and its qualifications. This research does not repin the active attempt
or replace its legacy rules with Chapter Two rules.

- [Legacy Rules Reference](https://images-cdn.fantasyflightgames.com/filer_public/50/be/50bed4be-034c-4ed5-9ce6-8509ce8d3352/ahc60_rules_reference_eng-compressed.pdf): pp19–20, slots and traits; p26, test timing.
- [Legacy FAQ 2.5](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf): p2, acquiring assets; p6, Katarina erratum; p17, nested reveals; p26, taboo text.
- Effective card text and publisher-FAQ transcriptions: [Jacqueline Fine](https://arkhamdb.com/card/60401), [Voice of Ra](https://arkhamdb.com/card/60416), [Dark Prophecy](https://arkhamdb.com/card/04032), [Katarina Sojka](https://arkhamdb.com/card/11069), [Sign Magick](https://arkhamdb.com/card/05112), and [Lita Chantler](https://arkhamdb.com/card/01117).

The screenshots remain in the owner's Pictures/Screenshots directory:

| Evidence | SHA-256 |
| --- | --- |
| `Screenshot From 2026-11-06 10-57-19.png` | `55e06d4fcc87f0850ebfcd70d158e7fd38d643f3f73fbf7594cf20f127f055ab` |
| `Screenshot From 2026-11-06 10-59-43.png` | `66329c1e76976a96a08787960f33cd4028139f7bc48ee08cafe71b73be121617` |
| `local/gathering/setup.session.json`, revision 31 | `2c781194e470944dbe85a4b9888c9e73956780abd756141b364f6947c68af1d2` |
| Its `.previous`, revision 30 | `c49094e868bd35c795b6bed7f922681323c0bca56ed1eb09fe2ab22fc4ccb50e` |

The saves were inspected read-only through selected fields and retained the same
hashes after inspection. Raw save and image bytes remain outside tracked content.
The saved report confirms Tony's Intellect test: base 3, Patrice's committed Ward
of Protection +1, skull +0, total 4 against difficulty 2. Ward has a wild icon.

## Result dialog

The excessive height is explained by source. `src/gathering_tabletop/results.rs:132`
uses the shared story-reading container. `src/tabletop_ui.rs:425` sets its height
to 90% of the viewport, and `:441` lets the body grow into the remaining space.
The footer consequently sits far below short content. Its ordinary controls row
does not center the button (`tabletop_ui.rs:234`). The report repeats the result
and renders empty contribution sections (`results.rs:243–250,294–311`).

The selected replacement is a compact result card in the existing dark palette:
a dominant outcome, a clear total/difficulty comparison, quieter investigator and
source context, and aligned contribution rows. Omit empty groups; retain every
meaningful contribution and every revealed token, including zero modifiers and
separate canceled/ignored states. Use words as well as color. Full detail remains
visible by default, as already accepted in
[ADR 0002](../adr/0002-persistent-table-areas.md#gathering-presentation-and-skill-test-feedback).

Example content from the actual screenshot:

```text
                    SUCCEEDED BY 2
                 Total 4 · Difficulty 2
                Tony Morgan · Intellect

Base intellect                                      3
Ward of Protection · Patrice                       +1
[skull token] Skull                                +0

                       Continue
```

Height follows content up to a viewport-relative maximum. Only overflowing body
content scrolls; the outcome and centered Continue remain visible. Long names
wrap rather than force horizontal scrolling or unreadably small type. Exact
dimensions are implementation choices requiring native inspection.

[IBM Carbon's modal guidance](https://www.carbondesignsystem.com/building-blocks/core/components/modal/guidelines)
supports sizing to content, bounding height and retaining header/footer during
vertical body overflow. Its
[typography guidance](https://www.carbondesignsystem.com/building-blocks/foundations/typography/overview)
supports a small, consistent hierarchy. The
[GOV.UK summary-list pattern](https://design-system.service.gov.uk/components/summary-list/)
provides a label/value precedent. These are design references, not evidence that
the proposed native layout is readable. Centering Continue is Joel's preference,
not a requirement imposed by those references. The
[W3C dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/)
informs focus containment and keeping the beginning of long content visible;
web ARIA guidance does not establish native accessibility.

The frozen report already contains arithmetic and contribution ownership
(`src/gathering/test_report.rs:9–43`; `runtime.rs:451–460,3350–3362`). Completed
reports can name the operation, but do not preserve its general target/location
or a consequence summary. Do not infer a target from camera focus or claim a
reward has happened while it remains pending. Continue is a presentation
acknowledgment, and optional effects retain explicit use/decline choices
(`results.rs:161–220`; `gathering_tabletop.rs:576–614`). Provisional results and
special outcomes such as auto-failure or The 13th Vision must remain explicit.

The least-complex useful change is local result geometry and composition. Preserve
the shared story container's other consumers, and keep History's report content
consistent (`src/gathering_tabletop/ui.rs:522–536`). No new dialog framework or
collapsed-details default is needed.

## Voice of Ra and nested token choices

[Voice of Ra](https://arkhamdb.com/card/60416) grants one resource plus two for
each qualifying retained symbol; taboo 10 includes elder sign and curse.
[Jacqueline](https://arkhamdb.com/card/60401) adds two reveals and then cancels
two non-auto-fail tokens or one auto-fail, subject to applicable prohibitions.
[Dark Prophecy](https://arkhamdb.com/card/04032) costs one resource and replaces
one pending reveal with five candidates. Its effective preferred category excludes
elder sign; if none qualifies, its fallback permits any candidate.
[Katarina](https://arkhamdb.com/card/11069) requires her controller's skill test.
Voice of Ra is not a test, so neither that reveal nor its nested Prophecy permits
Katarina. She is an optional reaction, not an effect that automatically activates
later. Her correct absence needs an understandable explanation.

The screenshot's “outer reveal” means the complete Voice of Ra request. “At 0”,
“at 1” and “at 2” identify its first three pending draw positions, before token
faces are known. Raw internal paths reach the labels through `path_name`
(`src/gathering/tokens.rs:1124–1128,1216–1285`). They should be replaced with
source-named, one-based targets such as “Voice of Ra — entire reveal” and
“Voice of Ra — draw 1”. Applying Jacqueline to one child is a different choice
from applying her to the whole request; clearer labels must preserve both.

For the actual declaration, physical token count is:

`3 from Voice of Ra + 2 from Jacqueline − 1 replaced draw + 5 from Prophecy = 9`.

The retained journal and revision-30 pending state establish the actual grouping:

| Source within the combined reveal | Physical tokens | Recorded choice |
| --- | --- | --- |
| Dark Prophecy replacing Voice of Ra draw 1 | 1: −2; 2: elder sign; 3: skull; 4: auto-fail; 5: −1 | Keep skull; ignore the other four |
| Voice of Ra draws 2 and 3 | 6: −1; 7: tablet | Jacqueline cancels token 6 |
| Jacqueline's two additional draws | 8: −2; 9: cultist | Jacqueline cancels token 8 |

No Dark Future is present. The offered skull/auto-fail choices are correct for
Prophecy's five candidates. After that choice, five results remain in the parent
group: skull, −1, tablet, −2 and cultist. **Jacqueline can cancel across that whole
group, not only her two additions.** Skull, tablet and cultist ultimately remain.
Voice of Ra pays seven resources; Prophecy costs one; the net change is +6,
taking Jacqueline from three to nine resources. An auto-fail retained for Voice
of Ra would be a payable symbol, not an automatic failure of a nonexistent test.
These arithmetic consequences are derived from the card rules and recorded choices.

Publisher guidance requires the replacements in this simultaneous reveal to be
declared before faces appear and preserves nested groups as parent choices.
`advance_node` resolves the inner selection before the parent's cancellation
(`tokens.rs:1508–1540`). Animation must not introduce a later opportunity to
activate a replacement after learning some of the batch's faces.

The rules retain hierarchy in `RevealNode`, but the public projection and renderer
flatten tokens (`tokens.rs:655–669,732–748`; `gathering_tabletop/play.rs:1119–1142`).
Labels expose number, face and state without source or active selection scope.
Individual actions route to token and bag; group actions route only to the bag
(`gathering_tabletop/controls.rs:412–433`). That explains the exploratory clicking.
The selected design therefore combines:

1. A visible pre-reveal plan organized by ability and named target, retaining all
   legal orders/scopes, costs, and an explicit reveal action.
2. Physical groups in the existing resolution tray, with source labels and a
   clear boundary around the current selection scope.
3. A persistent current instruction and direct controls: first Prophecy's choice,
   then Jacqueline's cancellation. Highlight every eligible target together and
   show selection progress, such as “1 of 2 selected”. Do not rely on color alone.
4. Distinct ignored/canceled presentation without hiding physical custody. For
   compositions involving Olive, a parent choice can include several surviving
   physical tokens; select and explain the whole group.
5. A short reason for relevant unavailable abilities, including Katarina's
   skill-test restriction, and a final resource breakdown separating gains/costs.

Source identity must survive the presentation lifetime: `node.event.take()`
currently removes the nested event instance at completion (`tokens.rs:1542`).
Project the needed public provenance from the rules owner rather than parsing
choice labels. Include normal and captured completion views; the three producers
are identified in `src/gathering/presentation.rs:1–6,36–55`.

This proposal refines the accepted physical tray instead of introducing a token
modal or moving non-test effects into Test. A label-only repair is cheaper but
leaves the player hunting for legal targets. A separate modal offers an explicit
list but weakens the physical continuity selected in
[ADR 0002](../adr/0002-persistent-table-areas.md#test-area-and-resolution-trays).

## Asset capacity and replacement

Ordinary capacity is per investigator: **one Ally, two Arcane, two Hand, one Body,
one Accessory**. Count slot symbols/units, not cards or traits; slotless assets do
not count and exhaustion does not release a slot. A single ordinary spell is
`Arcane: 1/2`. Katarina occupies an Ally slot. Jacqueline's selected starting deck
has no additional Ally capacity. Sign Magick occupies one Hand and adds one
Arcane slot restricted to Spell/Ritual assets. Its removal can reduce capacity.
These conclusions come from the Rules Reference and the cited card texts.

The matching save contains Katarina and two slotless Arcane Research permanents.
It has no second controlled Ally and no retained Olive play. This does not
reconstruct everything the owner perceived visually or establish that another
attempt was legal. The reported two-Ally impression remains unconfirmed.

| Situation | Current production behavior |
| --- | --- |
| New asset fits | Offers play without discarding an unrelated asset. |
| New Ally when Ally slot is full | Offers explicit minimal replacement sets using prior assets; the incoming card cannot be selected. |
| Several required discards | Offers legal sets and their discard-pile order; labels currently combine these into long play options. |
| Opportunity attack changes occupied slots | Revalidates placement after the attack; drops an unnecessary replacement or asks for a newly legal placement. |
| Crypt Chill removes Sign Magick | Recomputes Arcane capacity and pauses for mandatory excess-asset selection, with save/resume coverage. |

Evidence: `src/gathering/player_cards.rs:410–473,482–549,617–767` and
`src/gathering/crypt_chill.rs:21–45,89–111`. Relevant existing tests were read:
`tests/gathering_assets.rs:127–196,782–883` and
`tests/gathering_crypt_chill.rs:126–171`. No new execution claim is made.

For the owner's two-handed example, one existing Hand unit plus two incoming
units exceeds the ordinary capacity of two: the old asset must leave. Two
one-Hand assets must both leave to make room; one two-Hand asset releases two
units when discarded. The tally can represent repeated Hand entries, but no
selected card currently uses two. This is a rules-derived example and source
inspection, not tested support for an additional weapon.

### Defect found in Lita's gain-control path

FAQ 2.5 p2 requires discarding other controlled assets simultaneously with an
incoming asset occupying an overfull slot. The ordinary play path follows that
constraint. Lita's acquisition instead inserts her first, then offers every
controlled Ally—including Lita—as a discard target (`src/gathering/runtime.rs:3687–3695`;
`combat.rs:131–147`). `tests/gathering_scenario.rs:753–805` encodes both outcomes.

Discriminator: Patrice controls Peter Sylvestre and successfully gains Lita.
The implementation permits discarding newly gained Lita and retaining Peter.
Under the acquisition rule, the required replacement is Peter, absent a capacity
exception. Treating this as ordinary cleanup after insertion is the competing
reading; it loses to the specific acquisition rule's “other assets” requirement.
This is a source/code defect, not a newly executed reproduction or an explanation
proven to account for the owner's Katarina observation. A correction must preserve
the incoming identity and avoid exposing a stable legal two-Allies state.

### Selected strip and replacement presentation

Use centered text printed on the investigator mat between assets and hand, as
Joel requested. The quiet default is occupied types plus any type with modified
capacity, for example `Hand: 1/2 · Arcane: 2/2 · Ally: 1/1`. A full slot explanation
can remain available through the investigator's contextual inspection. The
alternative is a stable strip listing every type including zeros; it teaches
empty capacity at the cost of more permanent text. Joel selected the quieter strip
with the full capacity explanation available through investigator inspection.

Restricted additions need a qualifier, for example
`Arcane: 3/3 (+1 Spell/Ritual)`, with the supplying card available in the explanation.
Do not let a bare denominator imply unrestricted capacity. Before a new play
commits, name and highlight required discards and show the resulting usage.
Preserve genuine selection and discard-order choices without adding a redundant
confirmation to an already explicit single replacement action. After costs or
intervening attacks commit, do not offer a fictitious cost-free cancel/refund.

`PlayerView` currently has no slot summary (`src/gathering/runtime.rs:154–183`).
Derive it from the rules owner and the same snapshot as the asset cards; do not
save duplicate counters or copy capacity rules into the renderer. Existing mat
labels use projected screen-space `WorldLabel` nodes
(`src/tabletop_pieces.rs:37–61`), so reusing those would produce the floating label
the owner excluded. The new strip needs table-plane rendering. Flexible asset
rows remain intact; no equipment-slot boxes are proposed.

## Accepted design — 2026-11-06

Joel answered "As you recommend" to Q1 and Q2, and "as you recommend" to Q3.
This confirms shared understanding of the three presented decisions:

1. **Compact result card:** use the illustrated hierarchy and aligned contribution
   rows, retaining full detail, adapting height and centering Continue. Selected
   over individual contribution tiles, which need more room.
2. **Guided physical token groups:** use the existing drawer with a visible plan,
   source-labelled groups, current instruction, direct choices and unavailable-
   ability reasons. Selected over label-only repair or a separate token modal.
3. **Quiet slot strip:** show occupied and modified slot types; expose all capacities
   and modifier sources in contextual inspection. Selected over always printing
   the five zero-inclusive categories. Replacement consequences remain explicit.

The design frontier is empty. Exact dimensions, spacing, grouping geometry and
internal projection details are delegated implementation choices within these
constraints; they do not require another preference round. The accepted refinements
are recorded in the existing presentation ADR. The glossary defines asset slots,
effective capacity and reveal groups without encoding implementation details.

This closes the research/design interview, not gameplay implementation or native
acceptance. The Lita acquisition discrepancy remains a source/code finding; the
three presentation answers do not constitute verification or a separate activation
of its repair. No implementation, test run, publication or save mutation occurred
while recording these decisions. #89 remains the work-control authority.

## Local implementation activation

Joel instructed "Proceed with the implementation" on 2026-11-06 after accepting
Q1–Q3. Work starts from `918b2e2d3819f4b83970513950e7a36f034aee70` on
`fix/gathering-results-tokens-slots` at 10:55 UTC. The selected increment implements
the three accepted interfaces and corrects the investigated Lita acquisition
contradiction within asset placement. The existing player-observation and
revision-bound choice, save/reopen, and production UI layout seams own verification.
No new test seam or general rendering/rules framework is introduced.

The next acceptance surface is the local patch, recorded checks and an exact
launch handoff for Joel's visual inspection. The default two-active-hour return
and scope/non-convergence limits apply. Publication, personal-save mutation and
full-scenario acceptance are separate. The current #89 handoff was refreshed and
still points to this baseline. `PURP`, `WORK`, `AGENT-5` and `OPS` were re-read for
activation; the source and representation consultations above remain applicable
to the same accepted design.

## Evidence limits and verification consequences

Source and saved-record inspection explain the reported sequence and identify the
height/layout causes and Lita discrepancy. They do not prove the proposed layout's
clarity, token choreography or complete scenario correctness. Future checks should
exercise short/long result layout at constrained sizes, special outcomes, optional
effects, the exact nine-token choices, group-based Olive composition, restricted
capacity changes, explicit replacements, and save/reopen at a pending decision.
Native inspection remains necessary for printed-text readability and identifying
the next token action without exploratory clicking.

Existing Katarina code draws until a symbol in one operation and excludes applying
Jacqueline directly to its iterative node (`tokens.rs:1228–1231,1470–1482`). This
research establishes her ineligibility for Voice of Ra, not correctness of every
possible skill-test composition. Do not broaden a presentation acceptance into
certification or silent redesign of that separate interaction family.

## Local implementation result — 2026-11-06

The accepted increment is implemented on `fix/gathering-results-tokens-slots`.
Result reports now size to their content, keep a centered Continue button, show
the outcome and totals first, and align contribution amounts. A constrained
viewport scrolls the report body while retaining its heading and controls.
Zero-modifier tokens, ignored/canceled tokens, special outcomes and optional
effects retain their meaning.

Token decisions now show the current source, instruction and selection count,
with direct choices and confirmation controls. A source/target picker replaces
the raw nested-path terminology. Physical tokens occupy source-labelled rows;
available and selected tokens have distinct rims, including every member of a
group choice. The non-test drawer and skill-test area retain their existing
ownership. Manual camera framing remains under player control; Resolve focuses
the current resolution area. Katarina's unavailable explanation identifies the
skill-test restriction during Voice of Ra. Her absence from this event's choices
is correct and does not consume or exhaust her ability.

Slot usage comes from the same rules calculation used to validate placement.
Occupied and modified capacities are printed on the mat between assets and hand;
investigator inspection explains every capacity and restricted modifier source.
Replacement choices name the departing assets, preview them on hover or keyboard
focus, and state the resulting usage. Lita acquisition now requires discarding
other assets when the Ally slot is full. Existing restricted-capacity cleanup,
replacement ordering and post-attack revalidation remain covered. Slot units are
counted rather than cards; no two-handed card was added to the selected decks.

Joel explicitly removed the requirement to preserve existing saves during
implementation: “There's no real need to preserve existing saves. Prioritize the
best possible implementation.” Changed human-readable history labels can cause
older records to fail validation. No compatibility migration was added. Fresh
save/reopen at the pending token decision is verified. Personal saves were only
read; all native runs used isolated copies.

### Recorded verification

- `/tmp/gathering-final-check/run.json`: `scripts/check` finished with exit 0.
  Before/after source digest:
  `6140692f8fb6bf2563677b83f031a9e1d0847ea37b072b16fe4abba5190c909b`.
  This includes formatting, Clippy, the Rust suites, tool checks and renderer
  policy checks. The production UI suite exercises compact/constrained result
  layouts, direct token controls and instructions, slot-usage text and the
  presence of its physical surface, and preserved camera/selection behavior.
  Selection-count presentation and strip geometry were source-reviewed; the
  native frames below provide the stated visual evidence.
- The nine-token rules regression exercises Voice of Ra plus Jacqueline and
  Dark Prophecy, legal retained-token/cancellation choices, bag restoration,
  resource payout, Katarina remaining ready and fresh save/reopen provenance.
- `/tmp/gathering-fresh-save-startup/run.json`: the additional production-startup
  check passed on a fresh nine-token save, with visible direct controls and no
  source-save mutation.
- Independent standards and spec reviews closed with zero unresolved source
  findings on `/tmp/gathering-review-v4`, manifest SHA-256
  `6713e2102de1145c91ce3763fd0f8a5d29b365fb1ca07ecff274b62a42271d85`.
  This implementation-result section was appended after those source checks.

The checked native binary has SHA-256
`263e7bdcc9157f0ec2fa42f453e403edc79d0470ee28c1225c3513168786b470`.
Real-art validation passed in `/tmp/gathering-real-art-check/run.json`. Native
captures were inspected at `/tmp/gathering-result-native-v2/result.png`,
`/tmp/gathering-slots-native/slots.png` and
`/tmp/gathering-tokens-native/tokens.png`. These establish the compact result
layout, the printed `Hand: 1/2` strip and the visible Dark Prophecy instruction
and direct choice in those particular frames. They do not establish crowded
slot-strip readability or physical token-group interaction.

The subsequent pointer session in `/tmp/gathering-token-interaction/pointer/`
obtained consent and calibration but issued no input before its deadline. It
closed automatically at 13:24:39 UTC, retaining the 15-second cleanup reserve;
the isolated app was terminated at its 13:24:54 UTC lifetime limit. Its timeout
is not an interaction pass. No desktop-control session remains open. Native
token-group click-through and owner readability acceptance remain outstanding,
as does the complete-scenario acceptance owned by #89. No GitHub mutation,
publication or merge was performed.

### Local visual handoff

From the repository root, the following opens an isolated copy of the fresh
nine-token checkpoint for up to 30 minutes. The output directory must be new.

```sh
scripts/tools native launch --experiment gathering \
  --output-dir /tmp/gathering-owner-token-review \
  --load /tmp/gathering-fresh-review/nine-tokens.json \
  --art-root /home/joeloverbeck/.local/share/arkham-unfolded/gathering-assets \
  --initial-view jacqueline
```

Click Resolve to frame the drawer. The current instruction should identify Dark
Prophecy replacing Voice of Ra draw 1, with selection 0 of 1. This fresh random
draw has only Auto-fail among Dark Prophecy's eligible symbols; choose it and
confirm. Jacqueline's cancellation instruction should then appear. Canceling
that Auto-fail is one legal alternative to canceling two other retained tokens.
Source groups, availability rims and selected rims should remain intelligible
through both decisions. Stop on clipped controls, ambiguous source membership,
an unexpected refusal or an incorrect resource result, and retain the run folder.

For the other two checks, use the same launch command with a new output directory
and `/tmp/gathering-fresh-review/evaded-enemy.json` (compact result), or
`/tmp/gathering-fresh-review/exhaust-asset.json` with `--initial-view patrice`
(printed slot strip). Close each window when finished. The captures and temporary
checkpoints are local review artifacts, not committed game content.

## Owner playtest acceptance — 2026-11-06

After starting a fresh attempt with the README's explicit `--save` path, Joel
reported: “I've been playing the new version of the app and I don't see any issue
related to what we've been working on.” He then explicitly authorized
“publication, creating PR, merging PR, pulling from main.”

This accepts the compact results, guided token choices, visible slot capacity
and investigated replacement correction as one tested increment. It is the
owner's report of native play, separate from the agent's recorded captures and
unfinished pointer check. It does not claim that every nested card composition
or the complete Gathering scenario has been accepted. #89 retains that broader
boundary, including the earlier deferred caption-overlap correction.

Publication includes the implementation, its tests, ADR/glossary refinements,
this research/evidence note and the README fresh-save instructions. No unrelated
changes, personal saves or artwork are included. The publication pass consulted
`AUTH-1`–`AUTH-3`, `PUB-1` and `OPS-5`–`OPS-7`; prior implementation consultations
remain applicable. The PR and #89's current handoff own the exact published
commit, final review/check records and synchronization status.

## Retrospective follow-up — activated 2026-11-06

Joel instructed “Proceed with recommended changes” after the session
retrospective. Starting from `d202dfa08f3765c35542da51b4580270054b7035`, this
bounded follow-up improves the normal launch handoff, adds production UI
assertions for changing token-selection counts and slot-strip placement, and
adds diagnostic framing of the active resolution drawer through the existing
native launch path. Existing bounded readers and scoped searches remain the
tools for large references; no additional reader or global instruction file is
needed. The unrelated deferred Peter caption correction remains separate.

The accepted verification seams are the existing production UI host (visible
progress after actual choices and measured mat/strip geometry) and the existing
diagnostic startup/native-launch CLI tests (framing, argument forwarding and
explicit refusal when no resolution drawer is available). These establish their
specific behavior, not a fresh native playthrough. The local patch and recorded
checks are the next review surface; this follow-up does not inherit publication
authority for PR #114. The two-active-hour return boundary applies. Consulted
`PURP`, `WORK`, `OPS`, `AGENT-5` and `EVID-4` for the retrospective and this same
bounded follow-up; #89 remains the work-control authority.

### Follow-up implementation and verification

The native handoff guide now requires checking the owner's ordinary launch and
providing tested fresh-save recovery with the README update when compatibility
changes. The README's existing launch and incompatible-save instructions remain
applicable; this increment does not change the save format.

Production UI tests now exercise selection and deselection through Dark
Prophecy and Jacqueline's one-Auto-fail or two-other-token alternatives, checking
the displayed progress and legal confirmation. The slot test projects the
actual printed surface into the camera and checks its bounds against the gap
between the mat's asset and hand regions, plus centering and flat orientation.

Gathering accepts `--initial-view resolve`, using the existing Resolve button's
drawer framing. Tests compare those camera poses, check Home with keyboard focus
on the table, preserve the pending session, and require an explicit no-drawer
error. Resolution diagnostics reopen an existing save through a read-only path;
they cannot start a fresh attempt. Review exposed the initial create-before-refuse
ordering, and a regression reproduced it before correction. Startup-path tests
now check both unused paths (including absent parent directories) and unchanged
bytes for existing saves without a drawer. The native launcher forwards the
option and rejects it for other modes before staging. Shared camera controls
and drawer/token layout are unchanged.

Completed records (`run.json` is `finished`, `exit_code: 0`):

- `/tmp/gathering-retro-startup-green`: five UI/startup tests passed; the existing
  supplied-save check remains opt-in.
- `/tmp/gathering-retro-launch-green`: all eleven native-launch tests passed.
- `/tmp/gathering-retro-native-build-2`: normal tabletop executable built.
- `/tmp/gathering-retro-full-check-2`: complete `scripts/check` passed, including
  format, Clippy, application/tool tests and renderer checks. The library suite
  has 447 passing tests and three existing opt-in/deferred checks. This evidence
  paragraph was updated afterward; executable code and tests are unchanged.

The ordinary nine-token save at `/tmp/gathering-fresh-review/nine-tokens.json`
opened through `native launch --experiment gathering --initial-view resolve`
with the real collection. `/tmp/gathering-retro-resolve-native-3/run.json` records
`status: exited`, `returncode: 0` and executable SHA-256
`61c30e906e5a1304e4ca864267422a694a633aeed5ba22d320c9fc6f2149043a`.
The screenshot is `resolution.png` in that directory (3200×2000 pixels for the
1600×1000 logical window). Both source and staged save retained SHA-256
`a5a4e5cdb4bc81f0358784de82522fbdcedda9567264dc233fb6327ccf9e935a`.
No gameplay input was sent.

The same executable refused an unused save path before creating a session in
`/tmp/gathering-retro-missing-native` (`status: exited`, `returncode: 1`, expected
missing-save error, no `session.json`). The preceding capture attempt in
`/tmp/gathering-retro-resolve-native-2` timed out after losing foreground before
graphics admission; its record and unchanged save remain retained. The final
capture succeeded and is byte-identical to the first capture, confirming the
startup correction preserved that view.

The capture confirms drawer framing and visible selection instructions, but
also exposes overlapping token captions in this crowded state. Resolve framing
and token/label generation are unchanged from `d202dfa`; there is no native
before/after pair at this exact view. Record this as a separate visual follow-up,
not readability acceptance or a correction included in this verification patch.
The earlier Peter caption deferral and #89's full-scenario boundary also remain.
