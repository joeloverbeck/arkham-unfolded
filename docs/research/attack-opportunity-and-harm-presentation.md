# Attack-of-opportunity warnings and visible damage/horror

Research and design proposal for [GitHub #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22), requested during Joel's Tony playtest on 2026-10-28. The real case is Investigate the Study while engaged with a ready Ghoul: the attack occurred, but its risk and consequences need clearer presentation. This note does not implement or adopt a design, expand the experiment into a general rules engine, or establish human acceptance.

Joel subsequently authorized implementation of the revised recommendation below: one stage per harm event, including damage and horror together. The research observations retain their pre-implementation context. The [Tony guide](../experiments/tony-turn.md) describes the resulting controls and prepared v3 collection; native usability remains subject to his playtest. This follow-up keeps the existing #22 allocation and does not expand its reachable game rules.

## Recommendation

Use an always-visible amber warning badge and a short consequence line on actions that currently provoke. Then show each actual harm event as one brief, labeled tabletop presentation stage, displaying any damage and horror together. This incorporates Joel's revised direction on 2026-10-28: one attack is one stage, regardless of its harm amounts or kinds. Use the existing heart/brain artwork and font, with readable text when assets are absent. Keep live play automatic between genuine player decisions; retain individually stepped events in replay.

For the reported position, the action would read:

```text
Investigate the Study — Shroud 2
⚠ Attack of opportunity
Ghoul Minion will attack: 1 damage + 1 horror
```

The warning mark is an application cue, not a newly claimed Arkham rules symbol. Clicking still chooses the legal action. Hover or keyboard focus may highlight the Ghoul and Tony and explain why the warning applies, but the essential warning must already be visible.

| Approach | Trade-off | Proposed disposition |
| --- | --- | --- |
| Recolor the action or add only an icon | Small, but easy to miss or misunderstand | Insufficient alone |
| Badge plus visible source/amount | Explains the consequence without another click | Recommended first version |
| Confirmation dialog on every provoking action | Stronger protection against accidental clicks, but interrupts intentional tactics | Defer unless real use shows the inline warning is insufficient |

Color should supplement wording and shape, following the transferable design principle in [W3C's Use of Color guidance][color]. This is design guidance for a native app, not a claim of WCAG certification.

## Source findings

| Rule relevant to this proposal | Primary source |
| --- | --- |
| Ready, engaged enemies each attack when the investigator takes a non-exempt action. Fight, evade, parley and resign are exempt. The investigator chooses attack order. | [Chapter One Rules Reference, p5, Attack of Opportunity][rr] |
| Attacks occur after all action costs, before its effect. A multi-action cost provokes once per enemy; these attacks do not exhaust enemies. | [Rules Reference, p5][rr] |
| Spending or using an action is required. A free-triggered ability does not provoke merely because it has an action designator. | [Legacy FAQ 2.5, p7, §1.1][faq] |
| Fast cards cost no play action and do not provoke. Card-specific exceptions still matter. | [Rules Reference, pp2,11–12][rr] |
| Assign damage/horror first, allowing eligible assets. Resolve prevention, reduction and reassignment before applying the remainder simultaneously. Defeat is checked after application. | [Rules Reference, p7, Dealing Damage/Horror][rr] |
| Enemy attacks deal their damage and horror simultaneously. Direct damage/horror cannot be assigned or reassigned elsewhere. | [Rules Reference, pp9,25][rr] |

The [Grimoire 1.1, p2][grimoire] directs pre-2026 content to the retired FAQ. Its corresponding entries on pp5 and 8 agree on the timing points above; this agreement does not select a new baseline for the original campaign. The existing [source reconciliation](issue-1-source-reconciliation.md) remains the context for that decision.

The inspected attack-of-opportunity entries do not specify a dedicated symbol. This is not proof that no such glyph exists in the resource fonts; the font/asset inventory is a separate question. The action-cost arrow and free-triggered symbol already have other meanings and should not be relabeled as an attack-of-opportunity icon.

## Artwork and font findings

Both requested kinds of artwork exist and were visually inspected. Paths below are relative to the unchanged source archive, `/home/joeloverbeck/Pictures/Arkham Horror LCG Resources`.

| Use | Verified source | Identity |
| --- | --- | --- |
| Damage counter | `images/other/unclassified/en/sce-base-save/Damage--ImageURL--081d03144a6650cd7f9e.png` | Red anatomical heart; 185×229 RGBA PNG; SHA-256 `34156f8361c5adef67c8afe40cfd960aea36b1e69956da00046120c01a1b247d` |
| Horror counter | `images/other/unclassified/en/sce-base-save/Horror--ImageURL--d429aca0b365fe75ed12.png` | Blue outlined brain; 236×192 RGBA PNG; SHA-256 `cb7f3c7e20d1ecfcd133c605e7d6b01095b82d79acc2a18c9baee31d3b14d789` |
| Inline marks | Existing prepared `fonts/arkham-symbols.ttf`, selected by [the Jacqueline recipe](../../resources/asset-selection.json) | `arkhamicons Regular` v1.0; 144,020 bytes; SHA-256 `186176d7243c72d272a50c24e833fb77d983e7c4c6c99bc64f89c8322dbd8187` |

The font's `Q` (U+0051) visually matches the heart; `R` (U+0052) matches the outlined brain. The existing [symbol module](../../src/tabletop/symbols.rs) already identifies R as Horror. Q's identification is visual, not a named-font-metadata claim. All 108 non-control codepoints were rendered and inspected; the font supplies no glyph names. No attack-of-opportunity glyph was semantically identified. No asset name containing `opportunity` was found among 6,308 other-resource catalog entries, or in the inspected SCE save's Lua/XML. This is a bounded negative finding, not proof of universal absence.

The two selected PNG identities match the contained `Custom_Token` images in `sources/save/Arkham SCE 4.9.3.json`: Damage counter `cd2a02`, Horror counter `36be72`. The larger 768×768 Damage/Horror JPEGs are two-face supply-bag texture atlases; they are not standalone counter sprites. Alternative filled-center PNGs were also inspected. Prefer the actual contained-counter PNGs for tabletop marks and the font for crisp inline labels. The archive classifies these resources as `unclassified`; their availability does not certify publisher provenance.

Tony's current [25-image selection](../../resources/tony-asset-selection.json) includes neither these counters nor the symbol font. A future implementation must select and verify a fresh collection under [ADR 0001](../adr/0001-prepared-runtime-assets.md), preserving the current v2 collection and source archive. No preparation or copying into an active collection occurred in this research. No newly generated bitmap art is needed.

## Presentation implications and proposed safeguards

**One harm event is one presentation stage.** An attack applying 2 damage and 1 horror shows `+2 Damage` and `+1 Horror` together in one impact, with one source caption and one replay stop. A damage-only or horror-only event uses the same treatment with only its applicable amount. This matches simultaneous application without creating extra choices, rule effects or defeat checks.

Group by source occurrence, preserving each recipient's actual damage and horror within that event. A 2-damage hit is one stage; an attack dealing 1 damage and 1 horror is also one stage. Two separate attacks retain their identities even when their amounts and recipients match. Different harm effects within one player action also remain separate: Tablet damage and later Internal Injury damage are two events. If assignment eventually splits one attack across cards, show the allocations together within that attack's stage, clearly associated with their recipients; allocation support is not added by this proposal.

Resolve genuine allocation/prevention decisions at their existing rules windows. Then animate the actual result. Fully prevented damage should not look as if it landed; a brief prevention cue can explain the zero result. Preserve enough visual context to show the completed hit and defeat even when the recipient has already left authoritative play. Camera motion, animation completion, skipping and replay navigation must not own or repeat game-state mutation.

For the user's Study action, the proposed sequence is: inspect the visible risk → commit the action → show one Ghoul attack stage with Tony's damage and horror together → continue into the investigation. If a real decision or another rule effect intervenes, its governing timing takes precedence over this simple example.

Proposed visual treatment:

1. Within the attack stage, briefly outline the Ghoul and display `Ghoul Minion — attack of opportunity` as its source caption.
2. Show one compact impact on Tony's card with the red heart and `+1 Damage` alongside the blue brain and `+1 Horror`. Both amounts appear and settle into their persistent counters together. The two symbols distinguish the harm kinds without separate timings or pauses.
3. Expose the investigation's commitment choice when this stage finishes. The source cue and impact belong to the same stage and do not require separate replay steps.

For a 2-damage block, use `+2 Damage` and one grouped token movement. For combined harm, show both labeled amounts during that same movement. For two separate 1-damage events, preserve two stages and their source captions, even if the source card is the same. Use `+N Damage/Horror` because these are accumulated counters; avoid an ambiguous negative number that could mean healing. The card and its new totals remain legible after transient labels disappear.

A compact token arrival plus labeled pulse is the recommended starting treatment. Floating text alone is cheaper but less connected to the physical tabletop. Cinematic camera cuts, full-screen flashes and repeated screen shake add distraction and should not be the first version. The live camera remains under the player's control.

Initial timing could be roughly half a second of motion plus a short readable hold per harm stage; that is a tuning hypothesis, not researched evidence of ideal pacing. Live play should normally advance automatically, with a way to finish the current animation immediately. Replay's Next/Previous should visit and pause at each meaningful stage for narration. A reduced-motion form should retain the same labels, counters and discrete stages while omitting motion; this adapts [W3C's interaction-animation guidance][motion]. Skipping motion must not hide an unresolved player choice.

The warning should describe the current action's predicted attack, not prohibit a legal intentional risk. Use explicit English wording with a supplemental icon and color. State source and amounts when they are determined; do not present a guaranteed final loss when allocation, prevention or another unresolved effect can change it. Recheck after state changes, including readiness, engagement and selected action details.

## Cases that should judge an eventual implementation

- The current Ghoul/Study case: risk is legible before commitment; one attack stage clearly shows both damage and horror before investigation continues.
- Exhausted or unengaged Ghoul, fight/evade, and a fast play: avoid false attack warnings.
- One 2-damage hit versus two separate 1-damage hits: different event grouping, with no animation per point.
- One attack with both damage and horror, including lethal totals: one presentation stage and one replay stop, preserving simultaneous application and defeat timing.
- Prevention and eligible-asset allocation: show what actually landed on each recipient; keep the player's genuine allocation choice.
- Replay backward/forward and interrupted presentation: preserve source identity and amounts without reapplying consequences.

These are proposed review cases, not claims that all associated mechanics are currently implemented or newly authorized for #22. Human inspection must still judge visibility and pacing through the native experiment.

## Fit to the current implementation

Inspected code: `26f283ff07795a9b8962f7766d182a8c2176fea0`. The existing path explains why the presentation currently feels abrupt:

- [Legal choices](../../src/tony.rs#L331) expose labels without a risk preview. The rule explanation exists in [Ghoul inspection](../../src/tony_tabletop.rs#L66), away from the action being chosen.
- [Investigate](../../src/tony.rs#L467) pays the cost, applies the opportunity attack and enters commitment. The [attack](../../src/tony.rs#L707) updates damage/horror together and writes prose history.
- [Session choice](../../src/tony.rs#L397) records only one final observation after all consequences. [Resolution](../../src/tony.rs#L584) can include Tablet damage, Fight damage, defeat/bounty and end-of-turn Injury. Counter differences alone cannot distinguish their individual blocks.
- [The scene](../../src/tony_tabletop.rs#L688) immediately displays the resulting totals. [Replay](../../src/tony/continuity.rs#L16) advances by whole observations, so separate harm events inside one decision cannot currently be inspected individually.

The smallest credible implementation is local to Tony: expose a rule-derived attack preview, and record a small ordered list of actual harm applications containing source, recipient and applied damage/horror amounts. One attack's two harm kinds remain one rules application, rendered as one stage. Preserve separate occurrences even when they share a source or decision revision. Do not infer the facts from English history strings, classify action labels in the UI, or simulate random future results for the warning.

A local presentation module can own the displayed stage and timing while consuming recorded facts plus before/after views. It must never apply rules. Keep the defeated Ghoul visible only as a presentation remnant long enough to show its final damage, then its removal and bounty consequence. Do not reveal a newly drawn card early while preceding opportunity-attack stages are still playing. Its already-committed hidden-information boundary still prevents undo from changing that outcome.

The existing [save-before-install path](../../src/tony/continuity.rs#L120) must remain authoritative: save the result and facts before animation. Failed saves show no landed hit. Reopening an interrupted presentation can show the committed recap or settled state, but cannot apply effects again. Camera movement, skipping and replay remain presentation operations. During the sequence, pause new game choices while keeping camera/inspection available.

Record compatibility is the main nonvisual cost. Sessions and replays serialize their observations, and validation expects one observation per player-choice revision. Do not fabricate revisions for animation stages. New moments need explicit support for their finer presentation steps; old Tony records need a tested compatibility path preserving their honest aggregate playback or a retained compatible reader. They lack authoritative source/block facts, so filling a field with defaults is not sufficient proof of compatibility. Preserve original records and Jacqueline's existing reopening path; no general migration framework is needed.

Per-frame animation should update a few retained labels, tokens or transforms. It should not create interpolated rule observations that cause the existing scene synchronizer to despawn/rebuild the whole scene every frame. This is a design constraint, not a claim to solve the earlier intermittent NVIDIA-related lag.

The first eventual native comparison should cover the actual Study attack, successful Fight damage before enemy removal, and separate Tablet/Injury damage within one committed choice. Automated checks should cover warning presence/absence, grouping, failed-save and stale-choice refusal, replay/skip/reopen without duplicate effects, existing records and missing-art fallback. The 2-damage example can exercise the presentation seam without pretending a new two-damage action is reachable in this experiment. Allocation, prevention, multiple enemies and lethal-investigator branches inform the design's limits; they are not added mechanics for #22.

## Evidence custody and limits

The primary PDFs were read from the existing ignored `local/issue-1-evidence/sources/` copies. All three SHA-256 values were rechecked against [the existing source manifest](issue-1-evidence.json): Rules Reference `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`; FAQ `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`; Grimoire `0f932630f49d19fa05e71f34ec0cf1298db0978204bb3be41bda2c67bc1197ed`. Page numbers above are printed/PDF page numbers. Current web retrieval was attempted: the publisher landing URL redirects, the Rules Reference/FAQ fetches failed, and Grimoire exceeds the web reader's size limit. No updated bytes or changed baseline are claimed.

The source summary is deliberately short and paraphrased; the presentation behavior is a proposal. Research inspected current Rust source and actual local artwork/font bytes but did not exercise a new runtime path or establish native legibility/pacing. No application source, live save, configuration, prepared asset collection or GitHub state changed. The existing owner files were preserved. The user requested this research; implementation and adoption are not implied. The existing #22 effort and owner-session allowance carries forward without reset.

Consulted constitution revision 2026-09-30.1: PURP-1–6, WORK-1–6, EVID-1–10, SEM-1–2, REP-1–3, BOUND-1–5, INT-1–4, AGENT-1–5, OPS-1–7 and GOV-1–3. The foundations, glossary, prepared-assets ADR and tracker conventions also informed the proposal. This note supports the existing issue rather than creating another active work queue.

[rr]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf
[faq]: https://images-cdn.fantasyflightgames.com/filer_public/c1/d0/c1d0fab6-7fa6-4ce2-af6a-16416381a19b/ahc_faq_v25_february_2026-web.pdf
[grimoire]: https://images-cdn.fantasyflightgames.com/filer_public/5c/c7/5cc79eb6-3d9c-4345-a5f1-27051251a6e4/arkham_grimoire_v11_web.pdf
[color]: https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html
[motion]: https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html

## Owner playtest correction, 2026-10-28

After implementation at `58aeda6`, Joel reported plain colored harm rectangles, awkward marker placement, a flat Tony chaos token and inconsistent textual damage/horror labels. He confirmed that his old launch command selected the earlier asset collection and that the current README command displays the actual heart/brain graphics. The v3 images were also inspected and matched their manifest hashes. This establishes the old collection as the missing-art cause; no GPU repair is inferred.

The selected follow-up centers one or both counters at the landscape investigator's top edge, makes missing-art substitutes readable, shares Jacqueline's existing physical chaos-token and label rendering with Tony, and applies the same red/blue harm glyphs to visible text in both tabletops. Stored observations, rules, English history and accessible labels retain their original meanings. The shared helpers are local deduplication for these two current consumers, with no broader engine or future-consumer claim. Owner artwork, source archives, settings and saves remain outside the patch.

This corrects the existing #22 playtest path under the current effort and owner-session allowance; it does not reset either allowance. Baseline is `58aeda6`; the next inspectable increment is the corrected tabletop with focused regression checks and native render evidence. Human readability/pacing acceptance remains separate. No remote publication or issue closeout is included. Consulted rules for this bounded follow-up: AUTH-1–3, PURP-1–6, WORK-1–6, EVID-1–10, SEM-1–2, REP-1–3, BOUND-1–5, INT-1–4, AGENT-1–5, OPS-1–7 and GOV-1–3, revision 2026-09-30.1.
