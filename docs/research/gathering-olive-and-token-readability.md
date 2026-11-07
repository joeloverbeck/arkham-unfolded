# Gathering: Olive, reveal controls and token readability

2026-11-07. Source fixed point:
`1021b927562cadf444a1b258610a4e3fc408b235`.

This investigation supports the owner's `grill-with-docs` request under
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) and its
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060).
It records screenshot observations, current source inspection and source-qualified
rules interpretation. Joel subsequently accepted Q1–Q3, recorded in
[ADR 0002](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-harm-fractions-and-planned-token-reveals).
This accepts the presentation design, not implementation, runtime verification or
the resulting playing experience. An existing unrelated README edit was preserved.

## The reported Olive interaction

The inspected screenshots are outside Git in
`/home/joeloverbeck/Pictures/Screenshots/`:

| File | SHA-256 |
| --- | --- |
| `arkham-olive-mcbride-bug.png` | `bad80ba8523b4834a772c595d218e369f99553dda4f117affaed1a6afcbb5fd4` |
| `arkham-elder-tokens-distribution.png` | `3107e675c2ecaf361aed6d20a00f9e4ec7b967bb83a4f45553369c1f411bdb54` |

The first image's extra actions all begin **Jacqueline**. Olive appears inside
their target descriptions. The visible choices offer Jacqueline on the containing
skill-test request, on Olive's three-token request, and on individual draws inside
Olive's request. They are not labelled as a second activation of Olive, but the
repeated card names and nested draw paths make that distinction difficult to see.

The second image separates three Olive tokens from two labelled as added by
Jacqueline. That division describes physical provenance, not the cancellation
scope: `src/gathering/tokens/presentation.rs:4` explicitly distinguishes those
meanings, and `64–79` assigns Jacqueline's extra children a separate display group
even when she augments Olive's whole request. Five physical tokens alone do not
demonstrate double activation or an incorrect draw. The visible split is especially
misleading here because the saved declaration below affects all five together.

The actual current save was inspected read-only, without reopening it in the
application. Its exact retained copy is
`local/verification/1021b927562cadf444a1b258610a4e3fc408b235/olive-readability/session-observed.json`,
SHA-256 `3733e2e314426b0416a318f94dc1225fe4115c12885805fccb2f3bff52b8dc1a`.
The original `local/gathering/session-2026-11-07.json` remained byte-identical.
At revision 25 its recorded decisions include, in order:

1. `tokens:olive:0:209`: activate Olive on the original draw.
2. `tokens:jacqueline:at:0`: augment Olive's entire three-token request.
3. `test:reveal`: reveal the declared five tokens.
4. Cancel Elder Sign and -2, then confirm Jacqueline's cancellations.
5. Keep Skull and -1, then confirm Olive's selection.

Its history reports two resolved, one ignored and two cancelled. This matches the
selected composition's cardinality and the pictured five faces. It supplies
direct recorded evidence of one Olive activation followed by Jacqueline, not a
second Olive activation. It does not establish that the complete saved state can
be reopened by the current executable. The save's `source_revision` field is the
runtime contract identifier `c35be2c:issue-89-gathering-runtime-v4`, not proof of the
exact executable Git commit.

Current source supports this distinction:

- `src/gathering/tokens.rs:1323–1330` offers Olive only from the revealing
  investigator's ready assets. Applying her choice exhausts that asset at
  `1933–1938`, replaces the selected pending draw with three children at
  `1913–1949`, and leaves the reveal pending.
- Every token action is checked against current legal choices at `1846–1847`.
  Jacqueline has independent location, round-use and same-reveal guards at
  `1280–1293`; her action adds two children and marks her use at `1880–1911`.
- `src/gathering_tabletop/reveal.rs:36–40` supplies the reveal shortcut with every
  currently offered bag action. `controls.rs:412–422` routes both Olive and
  Jacqueline actions to that bag. Thus clicking **Reveal tokens** can reopen an
  ability-selection menu after Olive has already been declared.
- `src/tabletop_reveal.rs:152–180` closes/revises the offer when state changes and
  rejects stale menu generations. `src/gathering.rs:555–573` also rejects stale
  revisions and applies choices to a candidate before installation at `629–635`.

This is evidence of a confusing interaction surface, not a reproduced engine
double-use defect. No runtime probe, native launch or test was performed in this
research pass. The guards should receive a focused regression witness if this
interaction is changed; source inspection does not prove every input path.

## Governing card and rules evidence

The original English card faces were inspected in the unchanged resource archive:

| Card | Variant | Image SHA-256 |
| --- | --- | --- |
| Olive McBride `04197` | `b001e7acef79742a` | `b848477fd5113bb1e9f1dee3cd42bf661ca5056a5b72dd70995debff60634401` |
| Jacqueline Fine `60401` | `ba5b602644842ee4` | `01c1bcf5bbef4356ee2c0224ff8b7ffad87ab1584a53533da53c0857eaa7b3da` |

Their paths are under `images/cards/official/en/sce-base-save/`, respectively
`The Forgotten Age/04197--Olive McBride/b001e7acef79742a/face.png` and
`Investigator Packs/60401--Jacqueline Fine/ba5b602644842ee4/face.png`.
Olive spends exhaustion to replace one pending token with three and select two
to resolve. Jacqueline adds two to a pending reveal of any number, then cancels
two non-auto-fail results or one auto-fail result, subject to her round limit.

The publisher's [Legacy FAQ 2.5, printed page 17][faq] explicitly permits combined
replacement effects, requires their declaration before seeing the simultaneous
draw, and preserves an inner replacement group as one candidate for its parent.
Section 2.5, printed page 12, applies relevant effects across resolved survivors.
Those rules support keeping meaningful group distinctions. They do not provide
an exact official worked example for every Jacqueline/Olive composition.

The existing [nested-token interpretation][nested] records the accepted,
source-qualified derivations and their competing readings. The
[Jacqueline investigation][jacqueline] explains why equal physical draw counts
cannot establish equivalent cancellation scopes. Neither note should be upgraded
to an unqualified publisher ruling.

## Three materially different scopes

For a simple skill test already modified by Olive, the current tree offers these
different effects of an additional Jacqueline declaration:

| Proposed plain-language explanation | What changes |
| --- | --- |
| **Add Jacqueline around Olive's group** | Olive's group and two new ordinary tokens are three parent candidates. Olive chooses her survivors; Jacqueline then cancels parent candidates, possibly the whole surviving Olive group. |
| **Add Jacqueline to Olive's whole draw** | Olive's request becomes five tokens. Jacqueline cancels within that request; Olive then selects two remaining candidates. |
| **Add Jacqueline to one of Olive's draws** | One of Olive's three positions becomes a nested Jacqueline group. Jacqueline resolves its cancellation first; Olive then selects two of her three positions, with that nested group counting as one. |

These are explanations, not final button copy. The third option must not be called
“one retained token”: its target is declared before any faces or survivor choices.

A worked distinction uses physical faces `auto-fail, -1, -2, 0, +1`, without Dark
Future. With Olive's group nested under Jacqueline, choosing Olive's `-1,-2` and
then cancelling that group and `0` leaves `+1`. With Jacqueline augmenting Olive's
whole request, cancelling auto-fail and choosing `0,+1` leaves both. With
Jacqueline nested inside one Olive position, cancelling its auto-fail can leave
the pair `-1,-2` as one candidate; Olive can keep that candidate and `0`, leaving
three physical survivors. The latter result follows the existing group-preserving
interpretation; it is not a direct publisher example. These distinctions rule out
flattening all five physical tokens into identical selectable candidates.

The current implementation represents those ordering distinctions in
`src/gathering/tokens.rs:1550–1603`. Existing tests at
`tests/gathering_nested_groups.rs:150–221,295–311` exercise the first two
compositions, including save/reopen and refusal of illegal partial cancellation.
They were read, not rerun here. The third composition needs its own independently
derived witness before a simplified interface claims equivalent behavior.

## The repeated individual-draw targets

`RevealNode::declarations` (`tokens.rs:768–774`) enumerates every child. Olive
creates three children; the Jacqueline offer loop does not suppress the third.
The shortcut menu renders every supplied choice (`tabletop_reveal.rs:289–295`)
and supports scrolling (`235–272`). The screenshot ends at draw 2, so it does not
establish a missing draw-3 action; the source places that action farther down.

Before faces are drawn, three untouched ordinary Olive children have symmetric
roles. A candidate simplification may present one **one of Olive's draws** action
and choose a canonical child internally. This is a narrow equivalence proposal,
not proved by this audit. It must preserve outcomes and later legal declarations
under child relabelling, including remaining replacement abilities, groups,
protected tokens and persistence. Modified or sealed children cannot be collapsed
merely because their current physical draw counts match.

## Proposed interaction and verification boundary

Recommend one place to configure optional abilities, with a short persistent
summary such as **Olive selected: reveal 3, choose 2**. Once the player presses
**Reveal tokens**, execute that visible declaration. Do not reopen a second
overlapping ability menu. Keep an obvious route to add Jacqueline or another legal
modifier before revealing, and preserve pre-reveal Undo. Closing an options menu
should not silently undo an already committed activation.

For Olive alone, show one direct ability action when there is one meaningful
target. For compositions, explain the surviving group and cancellation effects;
do not silently choose a strategy. Keep token names available for inspection,
keyboard operation and missing-art substitutes even if permanent under-token
captions are removed. Group relationships still need a visible cue where they
affect what the player can select.

The smallest useful later checks are: Olive becomes exhausted exactly once and
cannot be reused; reveal uses the displayed plan; adding Jacqueline remains
explicit; each distinct composition preserves its allowed selections and results;
pre-reveal Undo and save/reopen preserve the same pending plan; stale actions and
failed save/install do not spend another ability. Existing Dark Future and
Premonition expectations remain binding. Native readability and the owner's
experience require their own later inspection; this note does not claim either.

## Harm-fraction readability

The third supplied image, `arkham-ally-damage-horror-labels.png` in the same
Screenshots directory, has SHA-256
`bc93a3d3eb0d6b10c84bdac946abd217a68f05ae1f2e62d8ec9f319d621c45cd`.
The white damage fraction overlaps the pale heart artwork. The horror fraction
currently has more clearance, but shares the same fragile presentation.

`src/gathering_tabletop/card_counters.rs:107–125` creates the fraction and anchors
an ordinary scene label at the lower token edge. The general label has a fixed
190-pixel container and no backing (`src/tabletop_pieces.rs:37–60`). Merely adding
a background to that container would make the wide strip the owner wants to avoid.

Recommend a near-black, nearly opaque badge fitted to the actual text, with small
padding and restrained corner rounding. Keep the light fraction at the token's
lower edge, preserving its meaning, zero counts, capacity omissions and movement
with the card. The existing counter treatment at `src/tabletop_counters.rs:156–164`
is a local precedent. Putting the fraction entirely below the token is a credible
alternative, but consumes more vertical space between token and card. A shadow
alone offers weaker separation against the pale artwork. The exact padding is
implementation judgment; its appearance still needs native inspection.

## One contained chaos-token region

The token-distribution screenshot shows an actual containment failure: the second
row extends below Test and collides with Patrice's heading. The source explains it:

- `src/tabletop_layout.rs:421–427` draws two lower rectangles, formerly Revealed
  and Canceled. Gathering hides those region labels at
  `src/gathering_tabletop/scene.rs:120–123` and places its tokens independently.
- `src/gathering_tabletop/play.rs:717–723` uses only three columns, 3.5 units apart,
  with rows 3.0 units apart. The second row's centers are at z=14.0, beyond the
  lower region's z=12.7 edge.
- `play.rs:1217–1251` starts every provenance group on a new row. Thus the exact
  three-plus-two provenance split in the screenshot necessarily overflows.

Recommend one lower rectangle aligned with the 13.4-unit-wide commitment region
above. Start at its upper-left in the normal Test view, fill left-to-right with
small comfortable gaps, and wrap within its bounds. Account for complete token
and selection-ring footprints. A provenance change must not automatically consume
a new full row. Preserve necessary group relationships through compact brackets
or cues; current eligibility must follow the pending choice, not merely origin.

Remove routine face-name/ordinal captions when token artwork has actually decoded.
Their current source is `play.rs:704–713`; internal physical-token identities and
click targets remain necessary, including for duplicate faces. For missing or
unreadable art, retain readable English names. `art.image()` currently supplies a
handle; use actual loaded-image availability (`art.rs:134–137`) when deciding
whether a textual substitute can be omitted.

Selection, cancellation and ignored state must remain distinguishable without
depending only on color. Cancellation already has an X; selected and ignored
states need clear non-color treatment if their status captions are removed.
Preserve the persistent selection instruction and progress. Names remain available
in inspection and the decision panel. These are functional requirements, not a
reason to retain redundant identity labels under every pictured token.

Later verification must cover the exact five-token Test case, ordinary Jacqueline,
nested selection, duplicates, canceled/ignored states, curse follow-ups with held
tokens and unavailable artwork. Current geometry tests cover ordinary three-token
Test and nine-token Resolve, not this five-token Test containment failure
(`src/gathering_tabletop/tests/token_guidance.rs:66–178`). The layout helper also
serves earlier experiments; scoped presentation changes must preserve their
existing consumers. No geometry or native check was executed for this proposal.

## Accepted interview decisions

The owner explicitly requested a compact unified region, upper-left packing,
smaller gaps and removal of redundant token identities. Three design questions
were presented with recommendations during this interview:

1. Use compact backed harm fractions at the lower token edge rather than a new
   text row below it.
2. Keep only meaningful group/state cues in the unified region, plus readable
   names when artwork is absent.
3. Show the declared Olive plan and optional Jacqueline control separately; the
   main Reveal action executes the visible plan and therefore forgoes unused
   optional abilities, instead of reopening their chooser.

On 2026-11-07, Joel answered "as you recommend" to Q1, Q2 and Q3, accepting all
three recommendations and settling this design frontier. The
[accepted decision](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#gathering-harm-fractions-and-planned-token-reveals)
records the behavior together. Q3 supersedes Gathering's earlier Reveal-shortcut
chooser; Q2 refines routine captions and provenance-group presentation while
preserving meaningful nested choices. Existing glossary terms already describe
Test area, reveal groups and held chaos tokens; no new term or ADR is needed.
No implementation, runtime verification or publication was performed by this
documentation update. This research supports #89 and creates no independent
implementation queue.

## Local implementation activation

After accepting Q1–Q3, Joel instructed **"Proceed with the implementation and
testing."** The local candidate starts from
`1021b927562cadf444a1b258610a4e3fc408b235`, including the accepted documentation
above. The selected increment repairs the three reported readability/control
problems on the existing Gathering play path. It preserves the current rules
interpretation, exact choice IDs, saved-state contract, unrelated README edits,
personal saves and artwork custody.

Use the already accepted Session observation/choice/continuity boundary and the
production tabletop/UI harness. Check five-token containment, compact backed harm
fractions, artwork fallbacks, meaningful selection/group cues, direct reveal,
explicit ability declarations, exhaustion refusal, Undo and save/reopen. These
checks bear their exercised behavior; native appearance and owner play acceptance
retain their separate evidence requirements. Return with a local inspectable
candidate within the default two-active-hour limit, earlier for material scope or
source uncertainty, three failed repair cycles or other constitutional stops.
This activates implementation and testing, not publication, merge or issue closure.

Consulted for the combined investigation: `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`,
`WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`,
`AGENT-5`, `OPS-1`–`OPS-7`, and `GOV-1`–`GOV-3`, constitution revision
2026-09-30.1. #89 remains the active work-control authority.

## Local implementation and verification

The implementation keeps all rules choices and save identities intact. Gathering's
Reveal control now executes only `test:reveal`; optional declarations stay in the
decision panel. A single available Olive declaration is a direct control. The
active plan remains visible, and Olive-first Jacqueline choices explain their
different resolution orders without merging scopes or costs.

The Test tray has one full-width lower region. Physical tokens fill from its
upper-left; dense layouts scale only when needed to stay inside it. Loaded chaos
art replaces routine face/ordinal captions. Missing art keeps names; checks,
slashes and crosses distinguish selected, ignored and canceled tokens. Group
badges identify actual nested choice membership, independently of provenance.
Harm fractions have tightly fitted dark backing. Shared earlier tabletop modes
retain their surface arrangement and fixed-width token-caption placement.

Local evidence is retained under
`local/verification/1021b927562cadf444a1b258610a4e3fc408b235/olive-readability/`.
The recorded failing tests reproduced the overflowing fifth-token layout,
unbacked harm labels, extra Reveal chooser and redundant loaded-art captions.
`ui-second-pass` finished with exit 0: 191 Gathering tests passed, six explicit
manual/export tests skipped. This includes normal-startup validation of ten
ordinary decision checkpoints, one-time exhaustion, pre-reveal Undo, stale input,
every offered Jacqueline scope, nested membership and save/reopen. Final checks
and independent review records are retained alongside this initial result.

Independent review identified one introduced wording bug: the short "reveal 5"
Jacqueline label also matched Olive with an inner Dark Prophecy, which actually
plans nine tokens. A failing regression reproduced this. The shorthand now
requires unmodified Olive children; elaborate trees keep the accurate relative
wording. Review also found a pre-existing return-animation origin mismatch.
A production-path regression reproduced it, and settled tokens and departures
now share the same Gathering placement and scale. `review-fixes-green` finished
with exit 0: 484 library tests passed and six explicit manual/export tests skipped.
The check/build/export records retain their own terminal results. Standards and
Spec reviews are separately retained.

`tests/support/gathering_olive_review.rs` reconstructs the reported path from its
ordinary seed and decisions; it never reads or mutates the personal save. Its
ignored export test writes fresh disposable review saves and validates the exact
startup arguments, including a branch with grouped Jacqueline cancellation.
The agreed desktop checks exercised one Olive activation, separate Jacqueline
planning, direct Reveal, five-token selection through the panel and physical token
menu, and confirmation into Olive's choose-two stage. The resulting disposable
save matched the ordinary exported checkpoint. Group membership and missing-art
token names were also inspected in native captures.

The first native pass exposed overlapping no-art harm labels. The repair splits
the name and fraction across two lines and separates the paired badges, preserving
font size, physical token positions and loaded-art fraction badges. A failing
rendered-bounds regression reproduced the overlap; the final version passes at
1280x720 and 1600x1000, scales 1 and 2, before and after Olive exhausts, with and
without art. Native recaptures confirm the repaired fallback, compact art-backed
fractions and five-token tray. Physical scrolling exposes all five complete
Jacqueline labels; the initial opposite-direction wheel attempt did not establish
a panel defect, so no scroll/layout code change was needed.

Final review caught an outward offset on single missing-art damage labels: Rust's
positive-zero `signum` is positive, so the paired-label rule also moved centered
enemy and damage-only asset labels. Consumer regressions reproduced both cases.
The offset now explicitly preserves zero, leaving singleton anchors and the
captured Olive pair unchanged.

The dated observations, exact executable identities, captures and input logs are
retained in the evidence directory's `native-v3/observations.md`. These checks
establish the observed UI paths, not complete campaign correctness or owner
acceptance. The personal save remains byte-identical. No publication or issue
closure is part of this implementation authorization.

[faq]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf
[nested]: issue-89-nested-tokens-followup.md
[jacqueline]: gathering-jacqueline-and-map-presentation.md
