# Gathering harm opportunities: 2026-11-14 investigation

Investigated at `b361a9ec96293e7e2961dba3ebb42e4dae99d5a3` following Joel's
report of assigning and applying three Grasping Hands damage while expecting
Deny Existence to become playable. This is diagnostic evidence for the
grill-with-docs interview, not an implemented fix or native acceptance.
The earlier [Gathering work](https://github.com/joeloverbeck/arkham-unfolded/issues/89),
[decision presentation work](https://github.com/joeloverbeck/arkham-unfolded/issues/163)
and [shared decisions](https://github.com/joeloverbeck/arkham-unfolded/issues/208)
provide the existing obligations; this note does not activate successor work.
[Issue #233](https://github.com/joeloverbeck/arkham-unfolded/issues/233) records the
accepted interview result and owns subsequent implementation and verification.

## Rules expectation

The [Rules Reference](https://images-cdn.fantasyflightgames.com/filer_public/50/be/50bed4be-034c-4ed5-9ce6-8509ce8d3352/ahc60_rules_reference_eng-compressed.pdf),
pp. 7 and 11, places prevention between damage/horror assignment and application
and permits a Fast event at its stated trigger. Deny Existence does not require
a generic player window at that point. The user's expectation of an opportunity
after assignment was correct.

The [Legacy FAQ 2.5](https://images-cdn.fantasyflightgames.com/filer_public/c1/d0/c1d0fab6-7fa6-4ce2-af6a-16416381a19b/ahc_faq_v25_february_2026-web.pdf),
section 2.23 on p. 14 and Deny clarification on p. 20, distinguishes cumulative
effects from separate repeated choices. One Deny can ignore all three damage
from failing Grasping Hands by three. The test remains failed. A later retry
does not undo already applied consequences.

The original English Deny Existence (`05032`), Grasping Hands (`01162`) and
Idol of Xanatos (`09102`) faces were visually inspected in the unchanged external
archive. Deny must still be in the affected investigator's hand, event play must
be permitted, and the consequence must have a qualifying encounter/enemy source.
Dissonant Voices would prohibit it. Direct damage/horror alone is not a
prohibition. Card/source identities follow the
[Jacqueline](issue-89-rules-jacqueline.md) and
[Patrice](issue-89-rules-patrice.md) source investigations.

## Saved attempt and reconstruction

Read-only inspection of the remembered Continue save succeeded. Its SHA-256 was
`1a0e1eeda817e231aa43cf29084cf18cb49c6e79d9e27643cc962eb33b88c2c3`;
the previous-save copy was
`0cbfbfab3f4b2464659845677cbc36695bbcd7aa3ced0aeb8bc7d485f6ee329a`.
Private paths and contents remain outside Git. The saved journal identifies:

- Jacqueline's round-three Grasping Hands test: agility 2, difficulty 3,
  tablet modifier -2, final skill value 0, failure by 3.
- An initial allocation to Robes followed by Undo, then three allocations to
  Jacqueline and `harm:confirm`.
- The resulting three damage with Deny Existence instance 211 still in hand.

Reconstructing the recorded decisions up to, but excluding, the application
choice produced these actual session options:

```text
deny:211:damage  Play Deny Existence to ignore all 3 damage from 01162
harm:confirm    Apply the assigned damage and horror simultaneously
undo
```

At that point Jacqueline had zero damage and no threat cards. Choosing Deny in
the diagnostic branch left her at zero damage and moved the event from hand to
discard. Following the recorded application choice instead produced three
damage. The original save was not edited. This establishes that the reconstructed
rules session offered and executed Deny; it does not reconstruct the historical
pointer, camera or animation state.

The temporary Rust reconstruction ran with
`scripts/dev cargo test --locked --test diagnostic_deny_session -- --nocapture`
and passed its one test (0.04 seconds of test execution). The existing
`gathering_harm` suite also passed all eight cases. These are rules-session
checks, not evidence of native discoverability or every supported timing case.
Diagnostic source and the reconstructed observation were retained privately;
the temporary test is not a maintained production feature.

## Presentation evidence

The reported `arkham-horror-apply-damage-horror.png`, SHA-256
`b4e62d45fcd1e4bf3ecbe7bb3aa1f304a767c781ce2ca16f379f8014b7f1ec2f`,
shows a different, round-two Cellar consequence: one damage assigned to Robes,
zero horror, and a button mentioning both types. The cropped image contains no
hand cards, so it cannot establish whether Deny's availability cue was visible.
A second cropped screenshot shows Deny in hand, but not its action menu or the
whole table. These screenshots do not prove a missing rules opportunity.

At the inspected revision, `controls::route` sends `deny:CARD:ASPECT` to the
physical card's menu while `harm:confirm` is global. The harm status in
`gathering_tabletop/play.rs` describes pending and assigned amounts but does not
name the available prevention cards. The generic window-opportunity summary
requires a player-window Pass option, which this harm decision does not have.
The result-opportunity summary also does not recognize the harm confirmation.
Thus Apply is exposed in the decision panel while Deny requires discovering the
card's contextual option. That presentation can explain the reported missed
opportunity without a missing rules window; the historical cue's visibility
remains unverified.

A temporary headless presentation probe confirmed the computed global choices
were Apply and Undo, with an empty window-opportunity list and no harm result
modal. Its two attempts to inspect the rendered Apply button were inconclusive:
the harness remained behind a reading modal showing Continue, so it did not
reach the intended live decision controls. Those failures are not evidence that
Apply is missing in the product, and no rendered card-menu/cue success is
claimed. The source-routing evidence and supplied screenshot remain distinct
from a completed native reproduction.

Joel's explicit zero-type display requirement is recorded in the
[presentation contract](../tabletop-presentation.md#player-decisions-and-commitments).
In the subsequent interview reply, Joel accepted showing legal prevention
directly beside the choice that proceeds with the consequence, retaining the
card menu and adding no empty pause. The presentation contract records that
decision for Deny, Idol and Deny's Agenda 1 discard opportunity at their existing
timings. This accepts the design; it does not establish an implemented fix or
native acceptance. A reminder to click the card was considered and rejected as
easier to miss.

## Comparison boundary

The closest supported analogue is Patrice's Idol of Xanatos: its optional
prevention is likewise offered after assignment and before application, with its
activation routed to the asset. Deny uses the same harm presentation for other
eligible encounter/enemy damage or horror; this is not specific to Grasping Hands.

The finite adjacent timing comparison includes Ward of Protection and A Test of
Will at treachery draw, Lucky! before failure is finalized, and Live and Learn
after failure consequences. These cannot share Deny's timing indiscriminately.
Source tracing found different presentation routes for those comparisons:

| Cards | Presentation at the inspected revision | Evidence limit |
| --- | --- | --- |
| Deny Existence; Idol of Xanatos | Card/asset menu while Apply is global; no explicit prevention-opportunity summary | Deny's actual saved decision was reconstructed; Idol shares the route |
| Deny Existence against Agenda 1 hand discard | Deny routes to its card; the proceed choice routes to the Agenda; neither opens the named opportunity summaries | Source inspection of `agenda-discard:deny` and `agenda-discard:resolve`, not a reconstructed native run |
| Ward of Protection; A Test of Will | Encounter reading exposes legal responses directly | Source inspection of `encounter::Reading::responses` |
| Lucky!; Live and Learn | Result opportunities recognize their respective decline/pass choices | Source inspection of `controls::result_opportunities` and `result_decline` |
| Dark Prophecy; Olive McBride; Jacqueline's reveal ability | Dedicated token-reveal presentation and guidance | Source inspection; not evidence that every reveal interaction is defect-free |

Sparrow Mask replenishment follows damage/horror actually taken; fully preventing
harm must not trigger it. Take Heart's failed-test reward cannot retroactively
fund an earlier Lucky! play. This list concerns cards in the selected decks, not
the full published card pool. Source inspection alone does not certify all their
runtime or native presentation paths.

The agenda-discard comparison matters independently of harm allocation: the
opportunity must precede the random discard, and it cannot be moved into the
damage-assignment step. Source tracing identifies another discoverability risk,
not proof that a player missed it or that its rules option is absent.

Consulted rules for the bounded investigation and interview: `PURP-1`–`PURP-6`,
`WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`,
`OPS-1`–`OPS-7`, `AUTH-2`–`AUTH-3`, `AGENT-5`, and `GOV-1`–`GOV-3`.
No new domain term or architectural decision was resolved.

## Implementation and rendered check

Joel subsequently instructed implementation of #233. The patch adds direct
controls for the existing Deny/Idol harm choices and Deny's Agenda 1 hand-discard
choice alongside their continuation. Their card/asset/Agenda menus remain
available. Labels and pending/assigned amounts are composed from nonzero harm
types in presentation only; rules timing, choice identities, recorded history,
save format and the partner ownership boundary remain unchanged.

Six focused tabletop tests passed after a red/green check of the missing direct
Deny control and a separate red/green check of zero-type labels. They exercise
direct activation, the retained card menu, damage-only/horror-only/mixed Apply
consequences, Idol's hand cost and aspect selection, Agenda prevention before
random discard, ineligible player-card harm without an empty pause, and human
versus partner ownership. A rules-session check also covers the Dissonant Voices
prohibition; the final repository check owns its executed result.

The ordinary recorded decision prefix was reconstructed into two isolated saves.
Both loaded through the production agent-seat startup with the installed artwork
and Vulkan on the NVIDIA GeForce RTX 2080 Ti. After the ordinary test-result
reading's Continue, the rendered decision panel showed **Play Deny Existence —
ignore 3 damage**, **Apply 3 damage**, and Undo, all on screen. The captured image
was visually inspected: both choices were readable and the pending/assigned
amounts contained no zero-horror entry.

Activating Deny through the rendered control left Jacqueline at zero damage and
moved the event from hand to discard. On the second copy, activating Apply dealt
three damage and left Deny in hand. Both seat sessions closed normally. The
original save loaded through the rebuilt read-only driver and retained its
original SHA-256 and three damage. No personal attempt was repaired or replaced.

The executable for these captures had SHA-256
`d30f16575e7fecfa506656986cea292e099bbbce89bb7373f1062a785650c6f3`.
Private snapshots, captures, protocol responses, diagnostic source and check
records are retained locally. The issue's current handoff owns the final source
candidate, repository-check result and remaining acceptance/publication status.
The windowless seat establishes actual rendered controls and activation; it does
not establish physical-input behavior or Joel's usability acceptance.
