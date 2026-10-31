# Prepared Study escape attempt: accepted spec preparation

Prepared on 2026-10-31 against `7083162607cc910632d1aaedbc2a82581fe54282`.
This records the requested `grill-with-docs` interview and accepted preparation
published through `to-spec` as [GitHub #55](https://github.com/joeloverbeck/arkham-unfolded/issues/55).
GitHub owns active scope, readiness, execution, evidence and acceptance. This
supporting document is not an active local queue, implementation authorization
or evidence of successful execution.

## Accepted direction and current evidence

Joel accepted Q1 on 2026-10-31: “Proceed as you recommend.” The selected direction
is one bounded connected Gathering segment with Tony, Patrice and Jacqueline,
prepared for one new gameplay spec. Joel accepted Q2 on the same date with
“Proceed as you recommend,” confirming the concrete position, endpoint and
prospective allowance below as the shared understanding for that spec. The
interview frontier is complete. The larger campaign goal is unchanged.

The assessment covered all 26 pre-existing files under `docs/`, README, GLOSSARY,
selected production code and live GitHub. Remote main matched the assessed
revision, the working tree had no pre-existing changes, and no issues or PRs
were open. The sole local addition is this preparation document.

- [#1](https://github.com/joeloverbeck/arkham-unfolded/issues/1) accepted the source
  assessment; [#2](https://github.com/joeloverbeck/arkham-unfolded/issues/2) accepted
  the Jacqueline/Patrice test experiment.
- [#22 owner acceptance](https://github.com/joeloverbeck/arkham-unfolded/issues/22#issuecomment-6536278638)
  and [closeout](https://github.com/joeloverbeck/arkham-unfolded/issues/22#issuecomment-6536383725)
  complete the Tony turn. No repeated acceptance request is warranted.
- [PR #54](https://github.com/joeloverbeck/arkham-unfolded/pull/54) supplies later
  supporting work, not a new gameplay increment. README correctly states that
  the campaign is not playable.

Full Gathering is a stronger eventual result but entails substantially wider
preparation, card, encounter, defeat and scenario behavior. Another isolated
Patrice probe would leave connected play untested. The selected intermediate
path tests cooperation and an actual act transition. No standalone maintenance,
architecture, tooling or historical-document cleanup ticket is justified by
this assessment. No new ADR or glossary term is needed for this reversible
prepared-position choice.

## Problem and selected result

The experiments cannot yet let Joel coordinate investigator turns and see their
combined decisions change the scenario. Offer a prepared **Study escape
attempt**: choose investigator order, investigate, support one another and decide
whether to spend the party's clues to escape. The program administers the rules;
Joel keeps each genuine optional decision.

This is a bounded task from a prepared position, not original scenario setup,
a complete investigation phase on every branch, or a complete scenario. Show the
endpoint before play. Success is not guaranteed or forced.

## Selected position

| Element | Value |
| --- | --- |
| Context | Original Gathering, Standard, three investigators; Act 1 Trapped and agenda 1, with no pending trigger or doom threshold. |
| Start | Start of the investigation phase before choosing its first investigator; no pending action or test. |
| Map and clues | All three investigators in the revealed Study; six clues there and zero held by the party. Only Study is in play. |
| Investigators | Printed attributes, health and sanity; zero damage; Tony and Patrice zero horror, Jacqueline two starting horror from two Arcane Research permanents. |
| Resources and discards | Zero resources and empty discard piles for all three. |
| Threats and assets | No enemies or threat-area cards, including no Internal Injury in play. Tony has Bounty Contracts with six bounties and no target. No other relevant assets beyond Jacqueline's two permanents. |
| Turns and limits | Each investigator may take one turn of three ordinary actions. Tony's restricted additional action is available but has no eligible Fight/Engage target. Jacqueline's reaction is unused this round. |
| Bag | Original Standard composition from the accepted baseline; no initial bless/curse tokens. Real random draws respecting physical multiplicities. |

| Investigator | Initial hand | Fixed hidden draw prefix |
| --- | --- | --- |
| Tony | On the Hunt `03263` ×1 | Guts `01089` ×2, then On the Hunt `03263` ×1 |
| Patrice | Promise of Power `07032` ×1 | Lucky! `01080` ×2, then Unexpected Courage `01093` ×1 |
| Jacqueline | Clairvoyance `60408` ×1 | Clairvoyance `60408` ×1, then Eyes of the Dreamer `11067` ×2 |

Preserve the pinned starting-list quantities in the remaining hidden inventory;
do not replace unknown cards with playable inert cards. Unresolved basic
weakness slots remain outside the reachable behavior and are not represented as
completed campaign preparation. This arranged state does not prove that a
preceding legal campaign reached it. Fixed private deck order is fixture input,
not player knowledge; the chaos bag remains random.

## Endpoint and legal choices

Ordinary play ends at the first fully settled action/test that leaves six clues
held by the group. Finish commitments, token cleanup, mandatory consequences and
any legal after-test choice before presenting the closing decision:

- **Advance Act 1:** choose the group clue contribution, spend six, execute the
  complete reverse, and expose Act 2. Put Hallway, Cellar, Attic and Parlor into
  play; discard any Study enemies (none in this position); place the party in
  the Hallway, reveal it, and remove the Study. Stop before further actions.
- **Finish the attempt without advancing:** preserve the resulting position and
  record the declined advancement. This is not a defeat or scenario resolution.

If all investigator turns end before six clues are gathered, finish at the
investigation-phase boundary, before the enemy phase. Voluntary early turn
ending and poor legal choices remain available. Save/close may interrupt any
pending decision without advancing it.

Ending this task does **not** end an unfinished investigator turn. Preserve
remaining actions, limits and game state. The post-action player window remains
available even when the sixth clue came from the third action. The experiment's
endpoint is not a claim that the game prohibits continuing to act after six
clues. Later continuation is outside this task, and there is no promise that
these experiment records will migrate into a future campaign implementation.

Before the endpoint, expose Investigate, Draw, Resource, eligible card
commitments, optional abilities and End turn under the actual rules. Support
Jacqueline's reaction on any investigator's eligible test, including declining
initially and using it on an eligible curse continuation. Preserve same-location
and one-card limits for other investigators' ordinary commitments. Support
Lucky!'s actual fast timing and cost; never replace optional effects with an
automatically favorable choice.

## Reachability and independent rule derivation

The finite scope follows from this position, not an unsupported-action filter:

1. Only ordinary successful Investigate tests gain clues, one at a time. Starting
   at zero means every pre-reveal test window before the sixth discovery has
   fewer than six held clues. The task ends after that test settles, before
   another begins. Thus no supported test loses its target to mid-test act
   advancement. General advancement during tests is legal, but later gameplay
   involving that case is outside this expressly selected endpoint.
2. No enemies means no legal Fight, Evade or Engage target, opportunity attack,
   friendly fire or Gathering tablet damage. Study has no destination before
   advancement; movement after it is beyond the endpoint. No Injury means no
   Injury action or turn-end damage.
3. Tony can draw at most three cards. Guts has no matching intellect icons and
   no willpower test occurs. On the Hunt can be committed to Investigate, but
   its encounter-draw event timing never occurs here.
4. Jacqueline can draw at most three cards: either three ordinary Draw actions,
   or at most two Draw actions plus one canceled-elder-sign draw during her own
   test. A reaction on someone else's test uses that investigator's elder-sign
   effect. Her selected non-Fast assets cost at least three; from zero resources
   gaining the resources and playing an asset needs at least four actions.
   Their intellect icons remain available for genuine commitments.
5. Patrice cannot draw Lucky!, gain its one-resource cost and use it during a
   test before her third action. Unexpected Courage is reached only by a third
   Draw action. Before any non-final own test, at most her initial Promise can
   be in her discard. Her elder-sign effect then cannot shuffle a nonempty
   subset while leaving one card. On a final test, an eligible shuffle must
   actually remain available, including selection of the retained card and
   persisted randomness; no later own Draw remains. The selected partners'
   cards cannot draw for her. Later commitments may change her discard without
   causing a new test for her.
6. Only one Promise is reachable. Its commitment introduces one curse; support
   real extra reveals, cancellation and token destinations. Neither a ten-curse
   overflow nor unrestricted nested replacement assets are reachable.
7. The Standard bag has one physical cultist held until test completion, even
   when Jacqueline leaves two tokens resolving. At most one cultist horror per
   test and at most three own tests gives Tony at most three horror, Patrice
   three and Jacqueline five including her starting two. No damage is reachable.
   Defeat is consequently unreachable; this is not investigator immunity.

These arguments must be challenged by implementation tests. Added card draws,
resource gifts, extra actions/tests, enemies, pre-test clue gains or further
phases invalidate the closure and require renewed scope review.

## User-facing behavior and verification

Use the existing observation → revision-bound choice → save candidate → install
boundary for both the native UI and player driver. A local party session and
its own record scope are sufficient; a general engine, DSL or migration system
is not selected. Reuse existing production helpers and shared interaction
conventions where their behavioral contracts agree. The two existing experiment
states and token resolvers are too narrow to become a party engine merely by
switching the actor.

The eventual spec should cover these user stories:

1. Choose investigator order while camera focus remains independent of the actor.
2. See each investigator's actions, hand, resources, clues and condition.
3. Investigate, draw, gain resources or end a turn with legal costs and limits.
4. Commit for the testing investigator or support another investigator legally.
5. Decide when to use Jacqueline and which tokens to cancel, including poor choices.
6. Experience Promise's curse and Lucky!'s conditional resource tradeoff.
7. Make any reachable Patrice elder-sign choice without losing it to automation.
8. Understand test arithmetic, token effects, costs and consequences in history.
9. Decide whether to spend clues and see the actual Act 1 map transition.
10. Finish without escaping when choices or random outcomes lead there.
11. Undo only before hidden information, save at pending choices and resume exactly.
12. Replay stored moments forward/back without new randomness or rewriting live play.
13. Play with readable substitutes when artwork is unavailable.
14. Keep earlier Tony/Jacqueline saves and recordings usable through their retained paths.

Independent acceptance witnesses for the same production player seam:

| Case | Expected consequence |
| --- | --- |
| Choose order / early End | Each investigator takes at most one turn; changing camera focus does not choose the actor; unused actions cannot transfer to another investigator. |
| Shared commitment | Patrice may commit her one Promise to another investigator at Study; add its icons and one curse, preserve the limit of one supporting card, discard at completion. |
| Lucky tradeoff | Patrice Draw + Resource + Investigate can pay one for Lucky on eligible failure; +2 may change the result. No icons are supplied by committing Lucky. Decline remains legal. |
| Combined tokens | Exercise every Standard symbol, curse continuation, auto-fail cancellation leaving two resolving tokens and permitted resolution order against independent expectations. |
| Investigator elder signs | Tony's effect changes Contracts; Patrice's effect applies when Patrice is testing, including when Jacqueline supplies the reaction; Jacqueline's own canceled elder can draw once. Implement each original card text, not one shared elder result. |
| Patrice final shuffle | After a final test with an eligible discard, allow retaining a chosen card and shuffle the remainder into her deck; persist exact continuation and refuse any fourth ordinary action. |
| Asset and draw bounds | Support all three prefix draws and matching commitments; refuse asset play for actual action/resource insufficiency. No hidden tail card is revealed on any supported path. |
| Sixth clue on last action | Settle the test completely, preserve the final player window, offer the closing spend/decline decision before artificial turn ending. |
| Advance / decline | Spend exactly six and perform the complete map transition, or preserve the Study and held clues. Neither branch silently continues into another turn or phase. |
| No escape | Ending all turns with fewer than six clues finishes the attempt without generating a win, defeat, enemy phase or campaign resolution. |
| Persistence failure | Failed candidate save leaves live state, pending choices and next randomness unchanged; retry duplicates no cost, token, clue or map effect. |
| Resume and replay | Fresh-process reopen at commitment, cancellation, Lucky, final shuffle and final spend choices; replay stored outcomes without drawing or recalculating under current rules. |

Keep undo barriers at committed hidden revelations, including card draws and
random reveals; do not treat presentation completion as the barrier. Reconcile
any newly reached randomization choice with the existing continuity contract.
An unobserved Patrice shuffle does not by itself reveal hidden information;
undo may restore its prior state and randomness if no intervening revelation
is crossed, but cannot cross the preceding skill-test token reveal.
Preserve original records on unsupported-version or corruption refusal. New
attempts preserve prior records. Native actor controls, commitments, token groups,
map transition, explanations and substitutes require actual-path inspection;
headless tests do not establish their usability.

Concrete arithmetic witnesses supplement the symbolic cases: Tony's intellect
three with a minus-two token fails against Study shroud two; one On the Hunt
icon makes it succeed. Patrice's intellect two with minus one fails; paid Lucky
adds two and makes it succeed. On a cultist result, Patrice's two minus one is
one; Lucky brings it to three, so the test succeeds and the failure-only horror
does not occur. Jacqueline with her Clairvoyance icon and Patrice's Promise has
intellect eight. Revealing auto-fail, curse and zero, canceling auto-fail, then
drawing minus two resolves a total minus four and succeeds at four against two.
Canceling curse and zero instead leaves auto-fail and discovers no clue. These
are source-derived expectations, not outputs from an implementation.

During spec synthesis, the original Standard reference face was rechecked:
cultist is minus one, while tablet is minus two. The cultist arithmetic above
corrects a preparation typo; the selected behavior and scope are unchanged.

Retain [ADR 0001](../adr/0001-prepared-runtime-assets.md) and
[ADR 0002](../adr/0002-persistent-table-areas.md). Prepare and verify selected new
assets outside Git, preserving the source archive and earlier collections.
No source-archive access is allowed at runtime.

## Sources and evidence limits

Use the [accepted source reconciliation](issue-1-source-reconciliation.md), the
[pinned deck projection](issue-1-decks.json), original English cards and applicable
errata/taboo precedence. This prepared position is not a newly prepared campaign
or a fresh latest-taboo selection. On 2026-10-31 all nine retained PDF/API files
matched their recorded SHA-256 in [the evidence manifest](issue-1-evidence.json).
This verifies retrieval and identity, not an independent backup guarantee.
Preserve required bytes in declared durable owner-controlled custody before
claiming dependent long-term fidelity/replay support.

The exact retained Rules Reference supplies: p3 ordinary group act advancement;
p7 clue contribution during an investigator's turn; p14 location entry/reveal;
pp23–24 investigator order and the post-action player window; p26 commitments,
token resolution and test completion. The first act has no Objective timing
restriction. The end-of-round Objective belongs to Act 2. Hallway's revealed
side has no additional triggered effect. FAQ 2.5 and the Innsmouth insert supply
the accepted multiple-token/cancellation and curse rules. New combined-token
expectations must be derived independently; neither existing resolver's output
is an oracle for their integration.

Original card images were visually inspected and their bytes hashed. The
archive catalog provides provenance, not publisher authentication. Existing
Tony, Jacqueline, Promise, Study and Standard-reference bindings remain in
[Tony preparation](tony-turn-preparation.md#source-baseline-and-independent-derivation)
and the original evidence manifest. Newly checked decision-bearing faces:

| Card | Variant | SHA-256 |
| --- | --- | --- |
| Patrice `06005` | `2961ee22e3bb9567` | `2d03069fe11626fb5055ccad9db42022b494df20462b5de6972af8aaa642dd2c` |
| Lucky! `01080` | `ae7c3042a7b21187` | `7e7d7493ad88d4e4a783298b0d1ddb33b9c919da4b1d8f4c16815fedb4e1b824` |
| Unexpected Courage `01093` | `8bef7e9dd3e8401b` | `84458fdbeb058ebe7cdf32522f83fb0406fdde2c4ca3fdd668186d4fa6ffecb7` |
| Clairvoyance `60408` | `9f216ab02abc5ae4` | `e5d7c691c81602a1fc5737c05f9182f7088061f8def927bbe3c6b3c5adcb5f8a` |
| Eyes of the Dreamer `11067` | `8a658f76fe0246fc` | `54828c35b8acd6aadc149040b433b1e2fea943fbf28421a7f822566c34a0a262` |
| Trapped front `01108` | `79184f89d96cf371` | `ea48ef97d968da6bdba80d4bb27e8fa663a881c70f6e9e51a0b216aefe58fd7f` |
| Trapped reverse `01108` | `79184f89d96cf371` | `c2a27ffffc807b7f2eea7c6da91a41808c4585ff7a41e74771be88679e606491` |
| The Barrier front `01109` | Original core face | `e2237f76b48f7ec58850739a11cb7da78255ec65455dcfeab00b9aa725191b21` |
| Hallway revealed reverse `01112` | Original core face | `5af07e38456e65821dc1ecd78e7a51728c23806167d9cf7bd3347b76b51e8bba` |

General mid-test act advancement that removes a test target remains outside this
endpoint; no unverified abort interpretation is adopted. Source search found
secondary attributed rulings but not independent primary confirmation. The
chosen first-six-clue endpoint avoids relying on that unresolved interpretation.

## Accepted prospective appetite, acceptance and exclusions

The accepted ceiling is **32 aggregate active engineering hours**, including delegated work,
source preflight, implementation, asset preparation, tests, native verification
and review; plus **two owner sessions of at most 20 minutes each**, including
retries. This is a discretionary spending ceiling, not a calibrated delivery
estimate. The allowance is prospective and unconsumed by implementation at this
handoff; this interview performed preparation only. Prior experiment allowances
do not carry over. No paid services.

The main effort is new party state, mixed investigator/token consequences,
persistence/replay and native integration. Existing shared presentation and
continuity helpers reduce supporting work, while this envelope avoids combat,
defeat, later phases and full-deck behavior. Keep the ordinary OPS-4 early stops,
including two active hours without an inspectable increment and non-convergence
limits. A large overall allowance does not waive those stops.

First demonstrate the new multi-investigator choice/test/continuity path with an
independent worked case before expanding presentation. Stop sooner if a source
cannot be verified, closure fails, unapproved capability/scope is required,
production or semantic surface approximately doubles, or any allowance expires.
No automatic extension, repair backlog or adjacent tooling lane follows.

Owner session 1 should exercise cooperation, interruption/resume and an escape
attempt; session 2 should use a materially different order/ability allocation,
include decline or unsuccessful completion as needed, and narrate recorded
replay. Technical evidence must be ready before asking Joel to judge clarity,
control burden and enjoyment. Outcome luck does not force another review run.
These planned checks can establish only the selected task; no recurrent-use or
complete-party-lifecycle claim follows.

Out of scope: scenario setup, complete Gathering, enemy/upkeep/mythos phases,
continued play after the endpoint, full deck shuffling/drawing beyond the bounded
Patrice continuation, actual random-weakness preparation, upgrades, general
engine/DSL, record migration, unrelated refactors/tools, video publication and
redistribution of source assets. Full campaign delivery remains unmet.

## Completed interview and published spec

Q1 and Q2 are accepted. There are no pending interview decisions. The selected
task is the threat-free, zero-resource prepared Study escape attempt, with real
choices, the first-six-clue endpoint, optional final advancement, fixed hidden
prefixes and random bag. Its prospective ceiling is 32 aggregate engineering
hours and two owner sessions of at most 20 minutes each, including retries.

This preserves meaningful cooperation and a real scenario transition while
containing the reachable rules. Continued play after six clues is excluded: it
adds unresolved target-removal timing and substantially more behavior before
producing this same first transition.

The selected test seam is the existing player observation/choice boundary with
fresh-process continuity and separate native/human checks. It is part of the
accepted coherent proposal; do not reopen that settled decision merely because
the generic `to-spec` procedure asks to confirm testing seams.

The owner explicitly invoked `.agents/skills/to-spec` on this document.
[GitHub #55](https://github.com/joeloverbeck/arkham-unfolded/issues/55),
“Play a prepared Study escape attempt with all three investigators,” was created
with `ready-for-agent` on 2026-10-31. The complete body, title, open state and exact
label were read back; the body matched the submitted file byte for byte.

The published issue includes the full accepted envelope, source bindings,
independent checks, appetite and stopping conditions without depending on this
uncommitted local document. It classifies the work as a decision-only gameplay
experiment for the selected playing effect, with no broader engine/campaign
adoption. The publication also includes the verified cultist arithmetic repair.

The label does not activate implementation. No gameplay implementation, runtime
session, asset preparation, personal-record change, commit, push, PR or merge
was performed. Ordinary technical choices within the contract remain delegated
when implementation is separately activated. Recheck HEAD, the authoritative
issue and source identities then; preserve separate implementation, verification,
production publication and owner-acceptance boundaries.

Consulted constitution revision 2026-09-30.1 in full during the documentation
sweep. Governing identifiers for the eventual issue: AUTH-2/3, PURP-2–5,
WORK-1–3/5/6, EVID-2–8, SEM-1/2, REP-1/2, AGENT-3/5, OPS-1/2/4 and GOV-1–3.
