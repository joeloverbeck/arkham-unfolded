# Gathering: Jacqueline and map presentation

2026-11-06. Investigated source fixed point:
`239dfe427e3883facdaa51a395269340ff8301d1`.

This supports the owner's `grill-with-docs` request under
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) and its
[current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060).
Findings below distinguish supplied screenshot evidence, current source inspection,
publisher rules, derived interpretations and proposed design. The design choices
were subsequently accepted as recorded below. The source investigation itself
changed no application code and does not establish native pacing, repaired rules,
or complete Gathering acceptance.

## Jacqueline: the pictured controls

The supplied image is
`/home/joeloverbeck/Pictures/Screenshots/arkham-jacqueline-ability.png`, SHA-256
`8d0d21201bde61e4e7f17ccdfe0c89156841602bfce2a29b4aa13a1abbcb9d72`.
It shows Tony testing, one planned physical draw, an expanded Jacqueline chooser,
two scope buttons, and the reveal/undo footer. The chooser and its children are
slightly inset relative to that footer.

[The token-options body](../../src/gathering_tabletop/ui.rs) has four logical pixels
of padding on all sides (lines 68–81); the footer is a separate sibling without
that padding (82–102). This accounts for the pictured horizontal offset. Both the
disclosure control (153–158) and the actual offered choices (193–198) use the same
ordinary `control` builder. The first dispatches `TokenOptions`, which only opens
or closes the category; the second dispatches a revision-bound game choice.
The appearance therefore obscures a real interaction distinction.

The current [accepted ADR](../adr/0002-persistent-table-areas.md#gathering-reveal-cue-and-story-panel-refinements)
deliberately retained every offered scope, including the singleton case, pending
semantic evidence. Its rationale came from the
[earlier controls investigation](gathering-reveal-and-story-controls.md#meaning-and-rules-evidence).
Any simplification below must explicitly supersede only that narrowly affected
presentation decision after proving that distinct legal continuations survive.

## What her ability means

The original English Jacqueline card `60401` was inspected directly in the
unchanged source archive, variant `ba5b602644842ee4`, image SHA-256
`01c1bcf5bbef4356ee2c0224ff8b7ffad87ab1584a53533da53c0857eaa7b3da`.
Its reaction can assist an investigator at her location before a chaos-token
reveal. It adds two tokens, then permits cancellation of two non-auto-fail tokens
or one auto-fail token, ordinarily once per round. It is not restricted to her
own skill tests. Her elder-sign card-draw exception belongs to her elder-sign
effect, not to every investigator she assists.

The [published Legacy FAQ 2.5, printed page 17][faq] requires replacement choices
before seeing the simultaneous reveal and preserves a nested replacement as one
candidate for its containing instruction. Its example uses Olive McBride and
Grotesque Statue; it does not directly specify every Jacqueline composition.
The existing [nested-token interpretation](issue-89-nested-tokens-followup.md)
retains those qualifications for Jacqueline, Olive, Dark Prophecy and Eyes.

The current code constructs an ordinary skill test as an all-results reveal group
containing one draw (`tokens.rs:1021–1027`). It enumerates both that group and its
child as possible declaration scopes (`749–755`, `1255–1285`). Consequently,
**whole reveal** means augment the containing request; **draw 1** means augment
the first position inside it. These are internal scopes, not different powers.

For the pictured plain one-token request, **if it is revealed without further
declarations**, either choice draws three tokens and offers the same cancellation
outcomes. Choosing `0`, `−1`, `−2` gives the same three legal pairs to cancel.
The extra nesting alone creates no fourth token or different ability cost.
This is a source-derived equivalence of that finite case, not an executed
equivalence test for every continuation.

The distinction matters when a request has several positions. Consider a
three-position Voice of Ra reveal, before any faces are seen:

| Declaration | Physical reveal | Jacqueline's cancellation scope |
| --- | --- | --- |
| Augment the whole request | Original A, B, C plus D, E | Any eligible results across all five |
| Augment only A | A becomes A1, A2, A3; B and C remain | Only A1, A2, A3; B and C are outside that cancellation |

This application of the general nesting rule is a derived interpretation, rather
than an exact publisher example naming Voice of Ra. The stronger concrete
compositional distinction already covered by the accepted investigation is Olive
first with Jacqueline augmenting Olive's entire request, versus Jacqueline first
with Olive replacing one of Jacqueline's positions. Both can reveal five physical
tokens while producing different grouping and selection consequences. Equal draw
counts therefore do not prove equivalence.

Likewise, one fresh physical draw does not mean there is only one meaningful
reveal: a sealed Premonition group may coexist with fresh positions. Preserve the
[accepted mandatory-replacement interpretation](issue-89-premonition-followup.md).

## Two separate rules defects exposed by the wider audit

These are source-confirmed mismatches with the selected rules, not proposals for
different game behavior. Runtime regression witnesses remain to be established
before repair verification.

### The once-per-round use never resets

`Game::jacqueline_uses` starts at zero (`runtime.rs:516,600,714`). A normal use
increments it (`tokens.rs:1878`), and every further offer requires zero
(`tokens.rs:1259`). A full literal search of `src` and `tests` finds no reset.

Both round-advancement routes increment `round` and call `begin_mythos`
(`phases.rs:581–589`; `runtime.rs:2786–2789`). That function resets Frozen in Fear
and action tracking, but leaves Jacqueline's count unchanged (`phases.rs:685–693`).
The same `Game` continues; these routes do not construct a replacement game.
As written, an ordinary use therefore disables her reaction in later rounds.
The card's round limit independently supplies the expected behavior.

Required repair: renew the ordinary allowance at the actual new-round boundary,
while retaining spent status throughout the current round, including end-of-round
effects. A continuation/reopen witness should use her in round N, refuse another
ordinary use in N, advance through the actual round transition, and offer her in
N+1 when otherwise eligible. Check both round-transition routes and a pending
save/reopen; changing a numeric field in an isolated fixture is insufficient.

### Arbiter permits reacting to the same reveal again

Choosing the Arbiter alternative exhausts Arbiter without incrementing the round
count (`tokens.rs:1870–1878`). That exemption is correct. However, the code marks
only the selected `RevealNode` as having used Jacqueline (`1881–1888`). The other
nodes, including the draws just created by her own reaction, remain eligible under
the per-node check (`1264–1272`). A normal use can consequently be offered again
within that same declaration, adding another two tokens.

The [official Rules Reference, printed page 3][rr] permits a reaction once for each
occurrence of its triggering condition. An attributed September 2023 publisher
answer, [reproduced on Arbiter's card page][arbiter], specifically says Arbiter
does not let Jacqueline react to herself and use her reaction again on that
same reveal. The hosted answer is corroborating evidence with indirect custody;
the general reaction rule is available in the publisher's own document.

Required repair: distinguish the exempted **round use** from the reaction already
used for that **reveal opportunity**. Reject a second use on the same request or
on draws created solely by that use. Do not replace this with an indiscriminate
once-per-skill-test flag: a later independent would-reveal opportunity, such as a
resolved curse's additional reveal, needs its own source-based treatment. The
existing code expressly creates that later opportunity (`tokens.rs:1773–1784`).
The minimal regression witness starts with ready Arbiter, declares its exemption,
and checks that another self-nested Jacqueline use is unavailable while the
preserved round allowance still works at a later legal reveal.

**Ordinary use before Arbiter is a different question.** Current code offers both
cost alternatives only while the ordinary round allowance is unspent. The
best-supported reading is to preserve that order dependence: Arbiter reacts when
an otherwise legal use occurs and exempts that use. It does not expressly permit
initiating Jacqueline after her round limit has already been spent. The Rules
Reference's limits rule (printed p14) restricts initiation; its reaction timing
(p3) and Arbiter's printed text support applying the exemption on the first legal
use. No exact publisher answer on that ordering was found in this bounded pass.
The competing reading would allow a second initiation because Arbiter could then
exempt it; it assumes an advance permission the reaction text does not grant.
Do not label the current ordinary-first refusal a confirmed defect. Explain the
consequence before selecting the ordinary cost when ready Arbiter is available:
**Use Arbiter now to preserve Jacqueline's use for another reveal this round.**

### Attempted runtime probe and its limit

One probe used the existing production driver and a disposable copy of
`local/gathering/setup.session.json`, which records round 1 and one ordinary
Jacqueline use. It exited **1** before play with:

> Gathering setup does not match its recorded decisions and pinned inputs.

The exact driver response, input/binary hashes, copied record and official Rules
Reference bytes/text are retained under ignored
`local/verification/239dfe427e3883facdaa51a395269340ff8301d1/jacqueline-rules-audit/`.
The original personal save and driver hashes still matched afterward. The driver
was not rebuilt or proven to correspond to the fixed source revision. This failed
load does not reproduce either rule defect, diagnose a save regression, or justify
changing the personal record. No long fresh-game substitute was run during this
bounded research pass.

## Other Jacqueline consumers checked

| Area | Current evidence and limits |
| --- | --- |
| Assisting Tony or Patrice | Gathering checks her round availability, defeat/resignation and matching location (`tokens.rs:1257–1262`); cancellation decisions belong to Jacqueline (`1223–1229`). The testing investigator remains unchanged. |
| Dark Future | The cancellation guard checks Jacqueline's actual weakness and protected faces (`1181–1193`, `1216–1220`); grouped choices protect individual members (`tokens/groups.rs:17–125`). Initial extra reveals are not prohibited. Existing finite group witnesses are in `tests/gathering_nested_groups.rs`; they were read, not rerun here. |
| Premonition | Whole scopes containing sealed material are excluded from optional Jacqueline declarations, while other fresh positions remain (`tokens.rs:1271`; `tests/gathering_premonition.rs`). This is the existing qualified interpretation, not new official certainty. |
| Canceled elder sign | Gathering checks both skill-test purpose and Jacqueline as revealer before drawing her card, and records notified tokens to avoid repeating it (`tokens.rs:1585–1605`). |
| Curse follow-up | A retained curse starts a fresh one-position declaration while keeping previous physical tokens held (`tokens.rs:1773–1784`). Unused round allowance may still be available. |
| Earlier Jacqueline experiment | Offers a direct `Use Jacqueline: reveal 3` action (`src/lib.rs:383–399`) and consumes a single-use flag (`473–476`). It has no general nested-scope chooser or multi-round scenario. |
| Party / prepared Study escape | Also exposes one direct ability action (`src/party.rs:483–490,714–724`), with its prepared-position scope. It is not evidence for full Gathering's round lifecycle. |
| Tony experiment | Its prepared observation reports Jacqueline already used (`src/tony.rs:288`); it does not supply Gathering's general ability implementation. |

This is a bounded audit of the reported interaction and directly related lifecycle,
eligibility, cancellation and nested-reveal paths. It is not an exhaustive proof of
every selected card or declaration ordering.

## Proposed Jacqueline presentation

Use a genuine action when there is one legally distinct use. For the plain
singleton case the proposed caption is **Use Jacqueline — reveal 3 tokens**, with
the short visible explanation **Then cancel 2 non-auto-fail tokens or 1 auto-fail**.
Retain the identity of the testing investigator and same-location assistance.
The action declares her use; the reveal action still reveals the bag contents.
It must not silently reveal tokens or choose later cancellations.

Where meaningful scopes or Arbiter costs remain, use an aligned disclosure row
with an explicit expanded/collapsed chevron and clear text, visually distinct from
the action buttons inside it. Name choices by their effect, such as **Affect all
3 Voice of Ra draws** versus **Affect only draw 1**, and explain what that changes
about cancellation. Keep the normal-use and exhaust-Arbiter costs distinguishable.

Apply the same disclosure distinction to Dark Prophecy, Olive and Katarina where
the current panel uses category toggles. The entire options body and footer should
share a common left edge. Exact styling and wording remain implementation judgment
within the accepted design, not a need for separate owner decisions.

Before collapsing the two singleton commands, establish equivalence at the
player-observation/choice boundary after the rules repairs. Cover ordinary and
Arbiter costs, legal later replacements, Dark Future, source-labelled groups,
Premonition's sealed/fresh distinction, curse follow-ups and save/reopen. Preserve
every distinct legal continuation; merely counting physical tokens is not an
adequate test. If a case has not been shown equivalent, keep it as an explicitly
explained choice rather than hiding it under an assumed equivalence.

The recommended initial canonicalization is deliberately narrow: an unmodified
all-results request containing exactly one ordinary unmodified draw, with no
sealed group or selection policy, maps the parent/draw aliases to the parent use.
After correcting self-retriggering, both declarations leave the same three draw
positions and subsequent legal replacements, differing only by a transparent
single-child container. Preserve normal and Arbiter costs as separate choices.
Do not apply this rule merely because `fresh_reveal_count()` returns one, to a
one-position Eyes request, or to an outer request whose sole child is already
Olive/Dark Prophecy: those parents have different later selection or cancellation
scope. This is the proposed technical boundary for the finite equivalence check;
the owner need not choose the internal tree representation.

No glossary edit is needed for this finding: the existing **testing investigator**,
**reveal group**, and **held chaos token** terms already express the distinction.
The narrow presentation update belongs with the existing table-area decision if
accepted; a new ADR would add no independent architectural trade-off.

## Location names at every player-facing destination

The selected Gathering code contains four raw-location choice-label sites:

| Choice | Source at the investigated revision |
| --- | --- |
| Ordinary movement | `src/gathering/runtime.rs:2162–2167` |
| Movement after Mists of R'lyeh | `src/gathering/spells.rs:324–337` |
| Tony's Quarry spawn, two branches | `src/gathering/runtime.rs:1263–1272,1409–1418` |

The tabletop already overrides the Mists caption with a name in
`src/gathering_tabletop/controls.rs:868–876`, but the shared label still contains
the identifier. `Session::choose_with_draws` appends that shared label to action
history (`src/gathering.rs:629–634`); ordinary movement also reuses it for granted
actions (`runtime.rs:2720–2726`). Fixing only rendered buttons would miss those
consumers. Hunter choices, Intel Report, arrival narration, the map and the enemy
sideboard already resolve names.

The owner directly requested this repair. Use the known English location title
for each player-facing destination, including shared driver choices and new
history entries. Retain machine choice IDs, saved identities and explicit source
references. Do not rewrite already-recorded historical strings. The unrevealed
Gathering locations already expose their names, so this need not reveal the
hidden face. No new naming framework is needed.

## Movement, location revelation and clue placement

### Current behavior and reusable pieces

`Game::move_investigator` reveals the destination, relocates the investigator and
engaged enemies, checks engagement, queues entry harm, and captures a named
arrival (`src/gathering/runtime.rs:2731–2766`). The scene then paints the location
face, clue count and standees directly from the shown snapshot
(`src/gathering_tabletop/play.rs:940–982`). Standees have no movement component
(`487–524`); location cards have no physical-card motion identity (`967–970`).
The existing card-motion path requires a position change, so it cannot by itself
turn a location card at its unchanged position (`motion.rs:280–293`). Both actual
location faces are available in the prepared collection (`art.rs:140–143`).

There is an existing clue **collection** animation. It detects a successful
investigation and a matching transfer from location to investigator, then carries
one counted token over 0.8 seconds (`consequences.rs:130–196,254–295`). It requires
the source to be revealed both before and after. It does not animate clues being
**placed** when the location is first revealed; those currently appear immediately.

The brass arrows already have obstacle-avoiding routes around card, caption and
reserved standee footprints (`src/tabletop_gathering_map.rs:75–84,133–136,185–210`).
Movement can follow that visible geometry in either direction. A standee includes
its whole base and holder. Current occupancy enumeration can also compact other
investigators into different slots when one leaves (`play.rs:977–981`); keep those
bystanders stationary so they do not appear to move with the actor.

### Source requirements

The publisher's [Rules Reference][rr], printed pages 14–15, says an investigator's
first entry reveals the location and places its printed clue quantity. It also
explicitly distinguishes an enemy entering an unrevealed location: that does not
reveal it. The downloaded PDF SHA-256 is
`14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`, matching the
previously pinned source; the relevant text was inspected in this investigation.
Sequential motion presents that committed consequence without creating additional
rules windows or delaying its authoritative mutation.

### Proposed physical sequence

The owner already selected the standee's lift, travel via the connecting brass
arrow and settlement into a destination location standee slot, with no rendered
hand. The proposed first-entry sequence is:

1. Lift the complete standee with restrained tilt, carry it to and along the
   actual brass route, then lower it into its destination slot.
2. Lift the destination card slightly, turn it over in place using its real
   unrevealed and revealed faces, and settle it. Match the established card-draw
   handling language without moving this card to a deck or tray.
3. Bring the existing green/gold clue counter into view just above its established
   card position, lower it with a small settling motion, and show one brief rim
   glint. Its numeric label represents the correct full quantity. This retains
   the accepted one-counter convention rather than depicting a pile of separate
   clue pieces.

Target roughly one to two seconds for the whole first-entry sequence, with a
shorter ordinary move to an already revealed location. These are starting points
for native tuning, not measured usability results. No additional Continue is
proposed. The exact lift, trajectory, easing and brief highlight are delegated
presentation choices within this proposed effect.

Stage the old face and clue visibility until their proper visual steps. Adding
movement after the current repaint would expose the revealed face and clues too
early. Keep count changes, cards and standees on a coherent displayed moment;
then expose the next entry-harm or engagement decision. Attic horror and Cellar
damage still apply on revisits. Preserve these finite cases:

- Already revealed destinations do not flip again or gain new clues.
- A zero-clue reveal has no clue counter or arrival effect. Act 2's Parlor reveal
  works even without a moving investigator (`scenario.rs:1242–1249`).
- Act 1 places investigators in Hallway while replacing Study; there is no
  Study–Hallway arrow to invent (`scenario.rs:1209–1237`).
- Hunter movement does not reveal rooms. Engaged enemies follow their investigator
  while retaining their threat-area representation.
- Fresh load shows the saved state without replaying old reveals or granting
  clues again. The existing first-reveal Undo barrier remains in force
  (`runtime.rs:2035–2060`).

Start visual transitions only after successful save/install, at the existing
`Presentation::committed` boundary (`presentation.rs:85–102`;
`gathering_tabletop.rs:655–676`). Add their motion to the existing completion wait
and redraw scheduling (`presentation.rs:151–197`;
`gathering_tabletop.rs:1225–1246,1491–1507`). Camera/menu scene rebuilds must preserve
the current pose rather than restart the sequence. Keep manual camera control;
blocking inspection pauses visual time; Reduced motion settles immediately to
the same consequence. Missing art uses readable face and token substitutes.
These requirements fit a local transient presentation extension; they do not
require a save-format change or a generalized animation framework.

## Hunter and forced enemy movement

Hunter behavior is already implemented: eligible ready, unengaged Hunters seek
nearest investigators with Prey/tie decisions (`src/gathering/scenario.rs:1395–1445`;
`phases.rs:366–398`). The [Rules Reference][rr], printed page 12, independently
defines that basic movement. Agenda 3 also moves unengaged Ghouls toward Parlor,
including exhausted ones, under a separate effect
(`scenario.rs:316–333,1602–1623`). The presentation must not infer Hunter eligibility
from every changed enemy location.

The existing **enemy-presence marker** belongs to a location, carries its stable
identity, and counts that location's sideboard group. It is not an individual
enemy. This is explicit in the [glossary](../../GLOSSARY.md) and
[accepted sideboard decision](../adr/0002-persistent-table-areas.md#enemy-sideboard),
and implemented in `enemy_board.rs:36–80`. Moving the whole marker when one Hunter
leaves would falsely imply movement of the group and carry the wrong room identity.

Propose keeping those markers stationary and using one temporary, individually
named travelling marker for the moving enemy. It follows the brass route; the
actual enemy card's transfer to another sideboard group or a threat area and the
source/destination counts settle together. Sequence several moving enemies so
their identities and destinations remain understandable. The simpler alternative
is to animate only sideboard cards, which fails to show travel along the requested
map route. Permanent individual enemy standees would introduce a larger map
redesign and clutter; they are not part of the recommendation.

## UI and motion research: evidence and design judgment

The [W3C disclosure pattern][disclosure] distinguishes a show/hide control through
an expanded state and commonly a directional chevron, while preserving button
semantics and Enter/Space operation. This supports a visibly distinct disclosure
row for real scope choices. It is web interaction guidance applied by analogy
to this native UI, not a claim that native Bevy controls implement ARIA.

Google's [Material choreography guidance][choreography] favors continuity, a clear
focal element and an understandable sequence; its [duration/easing guidance][timing]
links timing to travel and calls for smooth speed changes. These are useful
principles for following one standee, then the location and its clues. The older
Material page's desktop-interface timings are not a duration prescription for
this game's physical tabletop. Current [Material motion guidance][material-motion]
uses a spring model and distinguishes movement from color/opacity effects; there
is no need to import that system or add springs to this code merely to obtain
restrained motion.

[Apple's motion guidance][apple-motion] likewise recommends purposeful, brief,
realistic feedback and optional motion. These first-party sources support the
interaction principles, not the assertion that a particular glint will feel
rewarding. The token descent and single glint are an authored proposal to convey
newly available, tangible clues. A simple fade would cost less but communicate
less physical placement; a particle shower would compete with the card's reveal
and the established physical-tabletop style. Only the actual native result can
establish readability, pacing and the owner's desired urge to collect the clues.

## Design round 1 and safe return point

Joel accepted Q1 and Q2 with "as you recommend", then separately accepted Q3 with
"as you recommend as well". All three visible-experience decisions are settled.
Q1 and Q2 were asked while Jacqueline's independent rules investigation completed;
Q3 completed the same round.

- **Q1:** Use standee lift → brass arrow → destination slot, followed by the
  in-place location flip and one counted clue token descending/settling with a
  brief gold/green glint, roughly one to two seconds total, no extra Continue,
  revisits skipping reveal/clue placement, Reduced motion immediately settled?
  **Recommendation: yes.** Alternative: a more magical clue appearance.
- **Q2:** Keep location enemy-presence markers fixed and animate an individually
  named temporary enemy marker along the arrow, with card/count updates aligned?
  **Recommendation: yes.** Alternative: permanent individual enemy standees and
  the larger map redesign that would require.
- **Q3:** Show one genuine Jacqueline action for a proven-equivalent ordinary
  singleton reveal, retaining a chevron disclosure for distinct scopes or Arbiter
  costs, with revelation/cancellation still explicit?
  **Recommendation: yes.** Alternative: always retain an expandable ability
  section. The engine must preserve every distinct legal outcome under either UI.

The owner already requested location-name repairs and investigation of Jacqueline
through her related consumers. The two source-confirmed rule mismatches are
correctness findings, not optional game-rule choices for the owner to settle.
Implementation must establish regression witnesses before claiming either fixed.
No new glossary terms are necessary; **reveal group**, **location standee slot**
and **enemy-presence marker** already make the important distinctions. Record an
accepted refinement in the existing table-area ADR when the design settles.

The [existing table-area ADR](../adr/0002-persistent-table-areas.md#gathering-movement-location-revelation-and-jacqueline-controls)
records the accepted design. Following the original request to fix the reported
behavior and acceptance of all three design choices, local implementation proceeds
from the inspected fixed point. The existing Session observation/choice/continuity
boundary and production-registered tabletop UI remain the verification seams
accepted under #89. The default `OPS-4` inspectable-result and non-convergence stops
apply. Publication and complete-scenario owner acceptance remain separate; no
new desktop interval is opened by this design record.

Later verification should cover named shared choices/history, Jacqueline's actual
round/reveal boundaries and singleton equivalence, natural completion of the map
sequence, occupied slots/bystanders, revisits, zero-clue Parlor, a Hunter leaving
one of several enemies, engagement, forced Ghoul movement, save failure and
reopen/reduced-motion/dialog/camera behavior. Source inspection does not establish
those runtime outcomes. Native evidence must separately assess physical handling,
card faces, names/counts and pacing in the normal Scenario view with art and
substitutes; owner experience acceptance remains part of #89.

Consulted for this investigation and decision surface: `AUTH-2`–`AUTH-3`,
`PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`,
`REP-1`–`REP-3`, `AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7`, `GOV-1`–`GOV-3`,
`BOUND-1`–`BOUND-5`, and `INT-1`–`INT-4`,
constitution revision 2026-09-30.1. #89 remains the active work-control authority.

## Local implementation and verification, 2026-11-06

The working candidate implements the accepted controls and finite map sequence.
Singleton aliases are hidden only for an unmodified `All[Draw]` request with no
offered Olive, Dark Prophecy or Katarina modifier. Distinct costs retain a flat,
chevron disclosure. The direct action declares the extra tokens; the subsequent
reveal and cancellation remain explicit. Declaration detection also covers a
curse's new reveal while tokens from the preceding reveal remain held. Engine
choice identifiers and the journal format are unchanged.

The rules corrections reset Jacqueline's use at the ordinary Mythos entry and
prevent a second Jacqueline reaction anywhere in the current reveal tree, including
an Arbiter-assisted reaction. Previous curse roots do not disqualify a new reveal.
Regression witnesses first reproduced the round-reset and repeat-reaction defects.
The differential singleton witness now covers ordinary and Arbiter costs, both
cancellation alternatives, Dark Future's protected symbols and cancellable numbers,
a curse follow-up, completed results and repeated save/reopen. It compares actual
Session choices and consequences, excluding only tree provenance and history text.

Movement uses the existing map's actual arrow route and fixed investigator slots.
Each transition runs over committed observations and does not change the rules
Session or save. The initial timings are 0.85 seconds of travel, 0.55 seconds of
card turning and 0.4 seconds of clue placement. Existing harm and story reading
can follow those beats. A resident enemy's engagement waits for the investigator's
arrival; moving enemy cards finish their transfer when the displayed location and
engagement counts change. Independent enemies receive separate travel beats;
engaged companions share their investigator's beat. The scene uses actual location
faces and one counted clue token with a brief ring glint. Reduced motion settles
immediately. No new assets or saved animation state are introduced.

Ordinary movement, Mists movement and Tony's Quarry choices use location titles.
Known historical Move/Quarry labels are translated for display without rewriting
the stored journal. As with the existing development save policy in README,
old attempts are not promised compatible: validation reconstructs their decisions,
and changed rules or recorded wording can cause an older attempt to be refused.
Fresh saves produced by this candidate are exercised through save/reopen. No
personal save, settings or artwork is modified by these checks.

Retained checks are under
`local/verification/239dfe427e3883facdaa51a395269340ff8301d1/jacqueline-map/`.
`app-check-v2` passed formatting, Clippy, application/integration tests and renderer
tests before review repairs. `review-fixes` passed the expanded token matrix and
seven production-registered map tests, including resident engagement, Hunter
arrival, bystanders, menu pause/rebuild, revisits, zero-clue Parlor, Reduced motion,
fresh reopen and failed saves. `curse-ui-red-declaration` reproduced the follow-up
control defect; `curse-ui-green` passed eight guidance tests after its correction.
The earlier `curse-ui-red` failed in preparation and is not defect evidence.
The final check, review snapshot and prepared native route are retained alongside
those records; their manifests identify the exact checked candidate.

Two independent review axes found arrival synchronization and insufficient
equivalence coverage in the first snapshot; the repairs above address those
findings. Native appearance and pacing remain unverified until the agreed desktop
interval. Automated checks do not establish readable artwork/substitutes, a
convincing hand-like lift or the owner's desired appeal of newly placed clues.
The local implementation does not close #89 or authorize publication.

## Native inspection and bounded corrections, 2026-11-06

The owner authorized an eight-minute desktop interval, 20:00:47–20:08:47 UTC.
All review processes and portal sessions closed before the deadline. The inspected
v3 executable was `6bd679e31a3adc84c0ac1c170b66426e7554825a3893fd861be2724ebf3e6c6c`.
Original and copied native evidence is retained under the local verification
directory's `native-20261106T200047/`; no personal attempt was opened for play.

In the normal Scenario view, the recorded first Attic entry shows Jacqueline's
whole standee travelling over the brass connection, then the real location face
turning and the counted six-clue token descending with a glint. Tony and Patrice
remain in Hallway. The six-second, 20 fps, 3200×2000 client recording completed
with exit 0; its frames and the settled captures were inspected. An in-place
reopen preserved the settled state. A fresh process also showed Attic and six
clues with readable substitutes. The ordinary direct Jacqueline action was
readable and aligned with the explicit resolve action in a separate static run.

Two display defects were found. The bundled text font is an ASCII subset, so the
new triangle characters rendered as missing-glyph boxes. The existing shared
punctuation fallback now renders these as `>` and `v`, while preserving the full
accessible labels and disclosure behavior. Entry-harm feedback also exposed
`01113`; the finite location-name formatter now covers generated `from LOCATION`
text in prompts, choices and history, plus the pending-harm source description.
Saved journals and rules state remain unchanged by these display corrections.
A production first-visit test reproduced the raw-code caption before that repair.

The second pointer session remained awaiting portal consent and was cancelled;
it never received a grant. The Arbiter disclosure was inspected collapsed but not
physically expanded. An attempted diagnostic `--initial-view resolve` exited
because these pre-reveal checkpoints have no active drawer; `--initial-view test`
worked and is the correct future launch route. The two interactive app supervisors
were deliberately interrupted during cleanup; the two static launches exited 0.
These outcomes are not interchangeable with a fully successful interaction route.

Reduced motion, Hunter travel, zero-clue Parlor, in-motion pause and the expanded
Arbiter control remain automated evidence only. The two post-inspection display
repairs have no new native capture. The owner has not accepted motion quality or
complete Gathering gameplay, and the closed interval grants no later desktop use.

[faq]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf
[rr]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf
[arbiter]: https://arkhamdb.com/card/60402
[disclosure]: https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/
[choreography]: https://m1.material.io/motion/choreography.html
[timing]: https://m1.material.io/motion/duration-easing.html
[material-motion]: https://m3.material.io/styles/motion
[apple-motion]: https://developer.apple.com/design/human-interface-guidelines/motion
