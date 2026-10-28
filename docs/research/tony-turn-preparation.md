# Tony turn: specification preparation

Prepared during Joel's `grill-with-docs` interview on 2026-10-28 against `fc37a4fefc9e2de86a12a142a3464747095fc0f7`. The spec is published as [GitHub #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22), which owns readiness, execution, appetite, evidence and disposition. This document preserves supporting source research and interview decisions; it is not a second work queue or implementation authorization. The issue retains the exact preparation digest used for synthesis, before this publication backlink was added.

The [issue #1 assessment](issue-1-assessment.md) supplied the independently derived action-accounting obligations. [Issue #2's accepted play and replay](https://github.com/joeloverbeck/arkham-unfolded/issues/2#issuecomment-6449874991) establishes the previous bounded skill-test result. [Issue #13's accepted reveal presentation](https://github.com/joeloverbeck/arkham-unfolded/issues/13#issuecomment-6469688222) and the completed maintenance in #14/#20 do not establish further gameplay coverage. Live GitHub inspection found no open issues or PRs.

## Interview decisions

Joel accepted the recommendation to prepare one spec for a complete Tony turn from a prepared Gathering position. He then accepted the proposed unarmed Tony/Ghoul/Internal Injury position, fixed hidden deck order, genuinely random Standard chaos draws, and a prospective implementation ceiling of **24 aggregate active engineering hours**, including source preflight, implementation, verification and reviews, plus **two owner sessions of at most 20 minutes each**, including retries. No paid services. These are selected scope and appetite, not evidence of successful implementation.

The immediate consumer is Joel choosing a sequence of actions while the application administers their restrictions and consequences. The inspection should establish whether he can understand and complete the turn without doing its rules bookkeeping, resume an interrupted choice, and narrate the recorded result. This remains a decision-only experiment. It does not adopt a reusable engine or establish a playable scenario, complete campaign, recurring usefulness or long-term compatibility.

The incumbent is the accepted Jacqueline skill-test experiment plus manual administration for everything outside it. A local implementation of this one prepared turn is the least-complex credible intervention. The broader alternative, a playable Gathering segment, combines more unresolved content and scenario obligations. The narrower alternative, scripted chaos outcomes, would expose less ordinary tactical uncertainty. Neither alternative is selected.

One implementation item returns for disposition at its cap. It does not create authority for further maintenance, additional experiments or a general engine. Prior maintenance selections and their allocation history remain intact; only the particular human-inspected effect can establish its own result.

## Problem Statement

The application currently administers one scripted skill test. Joel cannot yet choose a sequence of investigator actions and have the application account for restrictions, enemy responses and end-of-turn consequences. The next decision is whether that broader playing loop is useful and manageable through the existing Rust/tabletop path.

## Solution

Offer a clearly named Tony-turn experiment alongside the existing experiment. Begin at the prepared position below, offer every legal choice reachable there, and finish after Tony's turn and mandatory end-of-turn effects. Preserve manual camera control, readable explanations, save/reopen and step-driven replay. The player chooses tactics; the application administers the rules.

## User Stories

1. As Joel, I want to see the prepared position and its boundary, so that I know what I am playing.
2. As Joel, I want to see Tony's ordinary actions and restricted additional action, so that I can plan a legal sequence.
3. As Joel, I want to fight or evade the Ghoul, so that I can choose how to manage its threat.
4. As Joel, I want to investigate, draw or gain resources while engaged, so that I can weigh their benefits against an attack of opportunity.
5. As Joel, I want to remove Internal Injury using its two-action ability, so that I can choose between immediate action cost and end-of-turn damage.
6. As Joel, I want to commit eligible cards or decline commitment, so that I control my investment in each test.
7. As Joel, I want actual random Standard chaos draws and readable symbol consequences, so that outcomes remain uncertain and understandable.
8. As Joel, I want to engage an eligible exhausted enemy or fight it directly, so that legal alternatives remain available.
9. As Joel, I want the bounty payout and remaining actions to update correctly, so that I do not have to track them manually.
10. As Joel, I want to end my turn voluntarily with remaining actions, so that the application does not force a strategy.
11. As Joel, I want illegal or stale choices refused without changing play, so that a mistaken input cannot corrupt the turn.
12. As Joel, I want to undo choices only before crossing new hidden information, so that I can correct an input without rerolling or concealing an outcome.
13. As Joel, I want to save at a pending choice and reopen offline, so that an interruption preserves costs, choices and randomness.
14. As Joel, I want a failed save to leave the live turn unchanged, so that retrying cannot duplicate an effect or change the next draw.
15. As Joel, I want to understand why damage, horror, clues and resources changed, so that I can follow the rules without administering them.
16. As Joel, I want Next/Previous replay moments and camera override, so that I can inspect and narrate the turn.
17. As Joel, I want readable substitutes when artwork is unavailable, so that presentation assets do not block play.
18. As Joel, I want existing Jacqueline saves and recordings preserved with an explicit compatible reopening path, so that the new experiment does not destroy prior work.
19. As Joel, I want a new attempt to preserve the previous record, so that restarting cannot silently replace it.
20. As an agent playtester, I want the same player observations and legal choices as the tabletop, so that automation tests the actual rules authority.

## Implementation Decisions

### Prepared position

This is a deliberately constructed position consistent with the cited local rules. It does not prove that scenario setup or a preceding campaign history reaches it.

| Element | Prepared value |
| --- | --- |
| Campaign context | Original Gathering, Standard, three investigators, Act 1 Trapped and agenda 1; no agenda threshold or pending scenario trigger. |
| Turn boundary | Investigation phase, Tony is the active investigator after turn-begin; no pending test, spawn or reaction. Stop after his end-of-turn effects and turn marker, before selecting the next investigator or advancing a phase. |
| Location and clues | Only the Study is in play, revealed; six clues on it, none held by investigators; no connecting destination is available. |
| Tony | Printed attributes, nine health/five sanity, zero damage/horror, five resources, three ordinary actions and his unused restricted additional action. No weapon or other optional asset. |
| Tony's hand | One On the Hunt, original level-zero version. |
| Tony's threat area | Internal Injury, already resolved into play. |
| Bounties | Bounty Contracts in play with five; one bounty on the Ghoul. |
| Enemy | One ready, undamaged Ghoul Minion, engaged with Tony. |
| Other investigators | Patrice and Jacqueline are at the Study with empty hands and no available response or relevant asset ability. Jacqueline's reaction is already used this round; her two Arcane Research permanents and two starting horror remain inert but preserved. Neither companion takes a turn inside this task. |
| Tony's draw pile | Preserve the pinned 35-copy slot multiset: Contracts, Injury and the hand card are outside the draw pile, leaving 32 cards. Its fixed hidden prefix is Guts, Guts, On the Hunt; the remaining 29 cards retain their identities and quantities in a stable private order. They are unreachable during this task. |
| Chaos bag | Original Standard composition: +1; two 0; three −1; two −2; −3; −4; two skull; cultist; tablet; auto-fail; elder sign. No curses or blessings. |

Fixed incidental values and private ordering are reversible fixture choices, not new campaign rules. Other unresolved campaign preparation, including the companions' basic-weakness selections, is not simulated or claimed complete. No missing input may become a silently invented live ability.

### Legal behavior and closure

- Offer basic Fight, Evade, Investigate, Draw, Resource, Engage when eligible, Internal Injury's ability and voluntary End turn. End turn is available at a legal action boundary, never to skip a pending test or mandatory consequence; saving and closing can interrupt a pending choice without advancing it. Refuse invalid targets, unaffordable costs, inappropriate commitments and stale revisions without mutation.
- The first Fight or Engage qualifying for Tony's restricted action automatically consumes it. The player does not choose ordinary-action payment instead. The extra action cannot pay for Injury, Evade, Investigate, Draw or Resource. Zero ordinary actions does not end the turn while a legal bonus action remains.
- Internal Injury costs two ordinary actions. A ready engaged Ghoul makes one attack of opportunity for that activation, not one per action spent. Removing it prevents its end-of-turn damage. Leaving it causes one direct damage when Tony ends his turn, including an early end.
- Basic Fight uses combat five against fight two and deals one damage on success. Evade uses agility two against evade two; success exhausts and disengages the Ghoul. Investigate uses intellect three against Study shroud two; success discovers one clue. Ordinary tests preserve optional commitments and their discard at test completion.
- An exhausted Ghoul can be engaged and remains exhausted. Tony can fight it without engaging first. He cannot engage an enemy already engaged with him. Bounty Contracts' payout after Tony defeats the Ghoul is mandatory.
- On the Hunt can contribute its combat/intellect icon to applicable tests; its fast event requires a mythos-phase encounter draw and is not playable here. Multiple eligible copies may be committed. Guts' willpower icons match no reachable test, so it cannot be committed and its draw effect is unreachable. No reachable card grants further actions, card draws, searches or reshuffles.
- There are at most three ordinary Draw actions, so the three-card prefix closes the reachable hand inventory. There are at most three successful investigations, insufficient for the six-clue act threshold. Movement, act advancement and deeper deck behavior are unavailable because of those rules and state, not arbitrary disabled choices.
- Every Standard face is supported. Skull counts Ghouls at the location even when exhausted; tablet applies its damage while a Ghoul remains; cultist adds horror on a failed test; Tony's elder sign adds a bounty to Contracts, not resources or the enemy. Return physical tokens at the correct test-end boundary. Six starting bounties is not a maximum.
- From this exact undamaged/unhorrified position, defeat is unreachable. Maximum damage is eight: a tablet bonus Fight, three tablet investigations each preceded by an opportunity attack, and Injury at turn end. Maximum horror is three: each ordinary action can cause opportunity horror or a failed cultist Evade, but cultist Fight/Investigate succeeds at the base values. These bounds must remain valid for the supplied state; do not give Tony immunity or suppress harmful choices.
- Another investigator could use Injury's action ability on their own turn at this location. Companions cannot spend actions during Tony's turn. Do not generalize this case into an owner-only rule or a blanket prohibition on other investigators' reactions.

### Player path, continuity and presentation

The rules authority remains observation plus revision-bound player choice, shared by native UI and driver. A local turn implementation is sufficient. Ordinary internal decomposition, algorithms, record-kind separation and fixture representation remain delegated; no generalized action engine, card interpreter or migration framework is selected.

Use session-owned randomness with unbiased selection respecting physical token multiplicities. Privileged deterministic draws support checks but are not player operations. Player observations omit future deck order and random state. Save candidate state before installing it or displaying its consequences. Failure leaves the live state and future random sequence unchanged. Retry and reopening must not reroll, repeat costs or repeat opportunity attacks.

Autosave committed decisions and permit manual saves at pending choices. Reopen the exact continuation, including held tokens, commitments, action availability, enemy state and random state. Interrupted animation has no rules authority. Replay consumes recorded observations/events rather than executing current rules or drawing randomness.

Honor the play contract's undo boundary: reverse a deterministic committed choice only when no intervening card/token revelation would be crossed. Restore the whole affected state, persist a new revision and retain an intelligible undo history. A reveal is a barrier when committed, before its animation completes. Never truncate an exported recording or reinterpret replay Back as undo.

Preserve existing Jacqueline records and an explicit tested compatible reopening path, using the retained experiment mode or a pinned compatible build. New turn records must identify their own supported input/source/application identity. Corrupt, mismatched or unsupported records fail visibly with originals intact. No automatic migration promise. New attempts preserve earlier records; saving in replay cannot overwrite live state.

Present actions, remaining action types, hand, enemy condition, bounties, clues, health/sanity and meaningful consequences in the native tabletop. Explain in ordinary game language; do not expose fixture machinery as gameplay controls. Preserve manual camera control and step-driven replay with manual override of automatic camera following. Reuse the prepared asset collection contract for exact selected new art and readable substitutes. Runtime never reads the source archive. New assets require selection, preparation and verification before dependence; image/font bytes stay outside Git.

## Testing Decisions

Use the existing highest behavioral seam: player observations and submitted legal choices through the same authority as the UI. Existing decision tests and actual driver-process save/reopen tests are the prior art. Check external available choices and consequences against the independent expectations below, rather than private type layout or output generated by the implementation under test.

| Witness | Independent expectation |
| --- | --- |
| Two successful Fights, then remove Injury | Bonus action, one ordinary action, then two ordinary actions. Ghoul defeated, one bounty paid as a resource, no opportunity damage/horror and no Injury damage. |
| Remove Injury first, then two successful Fights | Two ordinary actions and one opportunity attack (one damage/one horror), then bonus and last ordinary action. Same defeat/payout. |
| Resource three times while engaged | Gain three resources and take three damage/three horror. Bonus Fight remains. Voluntary End then adds one Injury damage; no enemy-phase attack occurs. |
| Evade, then Engage or Fight | Successful Evade exhausts/disengages. Engage can spend the bonus without readying; Fight does not require re-engaging. Both preserve the exhausted state until relevant removal. |
| Investigate with numeric −2 | Base three fails against two; one On the Hunt commitment makes four minus two succeed. Both provoke an opportunity attack first if ready/engaged. |
| Commitment and costs | Refuse Guts, On the Hunt's event timing, own-engaged Engage and Injury funded by one ordinary plus one restricted action. Permit appropriate On the Hunt subsets and decline. |
| Draw inventory | Three draws expose exactly Guts, Guts, On the Hunt. A fourth ordinary Draw is refused. Future cards never appear in player observations. |
| Every chaos face | Exercise numerical modifiers, auto-fail and each source-derived symbol effect against the applicable tests, including exhausted versus defeated Ghoul and changing Contracts bounty count. Use fixed independent token injections; statistical sampling alone is not a rules oracle. |
| Turn ending | Early end is legal; apply Injury once if present; stop at the declared turn boundary without running companions or enemy phase. |
| Undo | Reverse deterministic consequences before a reveal; refuse crossing a committed draw or token reveal, including during animation and after reopening; preserve visible history and exported records. |
| Failed-save retry | Inject persistence failure before installation, then retry: same pending state and next draw as uninterrupted play; no duplicated action cost, attack or commitment. |
| Fresh-process continuation | Save during a pending skill test, exit, reopen in a separate process and finish. Match committed choices/outcomes and unrevealed continuation against an uninterrupted control. |
| Recorded replay | Navigate actual stored moments forward/back, then return to unchanged live state. No new randomness, reconstructed outcome or live save from the past. |
| Legacy and refusal | Reopen a baseline Jacqueline save/record through the declared compatible path. Corrupt/incompatible records retain their bytes and receive a clear refusal. |

Run the repository check command for implementation. Inspect the affected native paths with isolated saves/settings using existing checked capture/input tools, including actual art and substitutes. Headless text assertions, screenshots, source fidelity and human experience retain their separate limits. Do not consume the owner allowance on avoidable infrastructure retries.

The two owner sessions inspect materially different choices: a threat-removal turn and a turn that evades or accepts opportunity costs while pursuing another benefit, with interruption and narrated replay included. Automation supplies technical evidence; only Joel judges clarity, pacing, control burden and whether the task is worth playing. Their combined ceiling is 40 minutes including retries. These runs establish only this experiment, not sustained return-use or general engine fitness.

## Out of Scope

Full scenario setup or progression; subsequent turns/phases; whole-deck random play; unresolved weaknesses/upgrades; extra weapons, Quick Thinking or other additional-action chains; Olive/Dark Prophecy; companion turns; general content support; runtime LLM rules interpretation; reusable engine/DSL/framework; automatic record migration; new native diagnostic tools; unrelated refactors; video encoding/upload; paid services. Impossible branches in this exact position are justified above, not disabled legal behavior.

The explicitly invoked `to-spec` published the bounded spec as [GitHub #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22) with `ready-for-agent`; its exact body and label were read back. No gameplay implementation, commit, push, PR or merge was performed. The publication boundary for the finished implementation remains its exact reviewed patch and evidence plus owner acceptance.

## Further Notes

### Source baseline and independent derivation

Use the [accepted legacy-source reconciliation](issue-1-source-reconciliation.md): original English card identities plus applicable errata and pinned taboo, card/scenario exceptions over general rules, Rules Reference over Learn to Play. ArkhamDB metadata is secondary transcription evidence; no Haskell implementation output was used as this turn's oracle. The pinned starting-deck projection is [here](issue-1-decks.json). This experiment does not assert a newly prepared campaign or a latest-taboo refresh.

All nine retained inputs in the [existing evidence manifest](issue-1-evidence.json) matched their recorded SHA-256 in this preparation. The relevant official Rules Reference, FAQ 2.5 and original campaign guide were read from those exact retained bytes. FAQ 2.5 was also accessible from its publisher URL; failure of the web tool to open the Rules Reference URL did not replace the verified retained PDF.

| Source locator | Obligation used |
| --- | --- |
| Rules Reference pp3–4 | Action abilities, activation access, costs and relevant player timing. |
| Rules Reference p5 | Opportunity attack timing; one attack per enemy for a multi-action activation; attacks do not exhaust the enemy. |
| Rules Reference p9 | Draw and direct damage. |
| Rules Reference p11 | Engage, Evade, exhaustion and eligible enemy targets. |
| Rules Reference pp12–13 | Fight, health/defeat thresholds, Investigate. |
| Rules Reference p17 | Resource action. |
| Rules Reference pp20–21 | Token availability and Uses; starting tokens are not a cap. |
| Rules Reference pp24–26 | Investigation turns, voluntary ending and skill-test sequence. |
| Legacy FAQ 2.5 p8, §1.10 | First qualifying action automatically uses the restricted additional action. |
| Original campaign guide pp1–2 | Standard bag and original Study-only starting topology. |
| Original card faces below | Actual abilities, icons, attributes, enemy statistics, Study/act quantities and Standard symbol effects. |

The following bindings identify exact English card sides in the unchanged source archive catalog. Card codes, variant IDs and content digests locate the intended versions without treating archive categories as publisher authentication. All ten face hashes matched; the revealed Study reverse was additionally inspected and verified. Inspected text and relevant numbers were compared against the scoped derivation. Source references and their images are outside Git.

| Card side | Code | Variant | SHA-256 |
| --- | --- | --- | --- |
| Tony Morgan | 06003 | 8549568e606cbff8 | `26780c99a7084d89c3580af12ec082b2add4f0563cac36a1a6507f41c97752d1` |
| Bounty Contracts | 06010 | b1d29e8dbb446e23 | `8050db0aca071b518bbfb9541aab74deafc01ed221a383158b673b21e590cf3e` |
| Internal Injury | 02038 | 7ba32ea06081d150 | `596d99447b67563925cb722630ad2d2f5154f2276a8dae3708773e5835a38e1b` |
| On the Hunt | 03263 | 3914f45d3bcf7d81 | `7b99463996c27142be679288af31f19b7a2cb3e0f16425c0db34cf92d8d800a8` |
| Guts | 01089 | c6da23b9ae042d18 | `4a8058f959ec5b911d77ca39f2782e750c61b399a567b657ad6de4766ab9376b` |
| Ghoul Minion | 01160 | d394ce9d19ec2cbb | `16f77cba517ecd3caf6e617f14c3968b6846f12a54f49699db6bd1868fd1fdd1` |
| Gathering Standard reference | 01104 | ee9f88be0fe1ad2b | `2c65c1d4b98f26f4268cfa4647b006ffa9a6d7030281a545e676ad41c1d9ceb5` |
| Study | 01111 | aec837f768735442 | `ea1390d9486abe7ea275b7501d87ec3a4eb73c5b27d01e3824d04b40d7b25334` |
| Study, revealed reverse | 01111 | aec837f768735442 | `f9a4874007445ac47f66275266e034ea65665722580e1672d70cc6ae9121cf10` |
| What's Going On?! | 01105 | 460aea194cbf717b | `eb8a3df46a9e1d52016872d530dee3c28a212cad3245077352fe3c6a0f6c370c` |
| Trapped | 01108 | 79184f89d96cf371 | `ea48ef97d968da6bdba80d4bb27e8fa663a881c70f6e9e51a0b216aefe58fd7f` |

The core encounter faces use the original Night of the Zealot archive occurrence. Tony/player-card faces use the selected original English SCE occurrences. The Ghoul's duplicate challenge-pack occurrence has identical bytes; its challenge context is not the source for this setup.

Custody remains bounded: downloaded PDFs/API responses are retained under the existing ignored research root, and original images in the external archive. This preparation verified retrieval and identity, not an independent backup or long-term preservation guarantee. Before dependent fidelity work relies on a source release, preserve the required exact bytes in declared durable owner-controlled custody and verify retrieval, as required by the existing source contract. A changed or missing source must be reported, not silently substituted. This is part of the bounded source preflight, not a new evidence-management product.

### Appetite, stopping and handoff

The 24-hour implementation allowance is prospective and unconsumed by implementation at this handoff. This interview performed source/spec preparation only; no new executable behavior or runtime test was produced. Close the exact state/source contract early within the allowance before expanding code. Return at the cap even if incomplete; stop sooner on unavailable governing inputs, an unresolved material source interpretation, an unapproved capability or expanding reachable content, non-convergence under OPS-4, or the owner-session ceiling. No automatic extension or fresh maintenance lane follows.

No new ADR is warranted: these are reversible, local experiment choices. The glossary records the resolved term prepared position. The published spec includes the shared player-choice seam, fresh-process continuity, separate native/human checks, source witnesses and evidence limits; the selected direction and budget remain settled.

Before execution, recheck the baseline, authoritative issue and source bindings. [GitHub #22](https://github.com/joeloverbeck/arkham-unfolded/issues/22) contains the complete spec and source bindings, without depending on this still-uncommitted document being available through GitHub. Keep subsequent readiness, active execution, appetite, evidence and disposition there.

Consulted constitution revision 2026-09-30.1: AUTH-1–AUTH-3, PURP-1–PURP-6, WORK-1–WORK-6, EVID-1–EVID-10, SEM-1–SEM-2, REP-1–REP-3, BOUND-1–BOUND-5, INT-1–INT-4, AGENT-1–AGENT-5, OPS-1–OPS-7 and GOV-1–GOV-3. The full constitution was read during preparation; AUTH, WORK-1/WORK-2, OPS-1/OPS-2 and PUB-1 were rechecked for publication. Applicable identifiers are recorded in the authoritative spec issue.
