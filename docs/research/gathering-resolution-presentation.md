# Gathering exhaustion and resolution presentation

Research and accepted design, 2026-11-05, for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). Source review is bound to repository commit `4e610893604d05eb256a2dfe1e19dd43c8004d54`. Joel accepted the drawer and caption recommendations after the investigation. [ADR 0002](../adr/0002-persistent-table-areas.md#test-area-and-resolution-trays) owns the accepted presentation contract. Native usability was unverified at the investigation stage; the subsequent [owner acceptance](#owner-acceptance-and-publication) records Joel's live-app review. The implementation activation below records the separate execution boundary.

## Source findings

The existing [source reconciliation](issue-1-source-reconciliation.md) and [Gathering card adjudication](issue-89-rules-scenario-tony.md) remain the baseline: original English identities, applicable Legacy FAQ 2.5 and taboo 10. This investigation does not adopt Chapter Two replacement cards or change the pinned campaign rules.

| Source inspected for this question | Identity and relevant location |
| --- | --- |
| [Chapter One Rules Reference][rr] | AHC60; downloaded and SHA-256 rechecked as `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`; exhaustion p11, health p12, search/revelation p18, surge p19, then p20, ST.7–8 p26. The [older publisher AHC01 PDF][old-rr] independently shows the same relevant exhaustion/search wording. |
| [Revised-core Learn to Play][ltp] | AHC60, p5 diagram inspected visually; SHA-256 `f82790f81af3731f11f0a7ddc267feb0644b47d20842a973f1023de709568ab0`, matching both retained local Learn to Play PDFs. |
| [Legacy FAQ 2.5][faq] | February 2026; online sections §2.13 p13 and Scarlet Keys ruling p24 checked. The existing source record binds the selected FAQ identity. |
| Kicking the Hornet's Nest, `09068` | Original English front inspected in the unchanged external archive, variant `304055f6947160c2`, SHA-256 rechecked as `778861c274d96421bb503ba32e25466209338196629c0aeec995a015fe05e170`. [ArkhamDB transcription][hornet] agrees. Archive provenance remains subject to the existing custody limitation; its catalog label alone does not certify publisher provenance. |
| Perception, `01090` | Original English front inspected in the external archive, variant `91f4a7c5cae1b73b`; [ArkhamDB transcription][perception] agrees. |

### Exhaustion

**Direct rule:** exhaustion turns a card 90 degrees. The written rule does not specify clockwise or counterclockwise. **Direct visual evidence:** the publisher's Learn to Play illustration turns the Ghoul Priest clockwise; its title edge moves from the top to the right. [RR p11][rr]; [Learn to Play p5][ltp].

**Recommendation:** use clockwise exhaustion, measured from the card's upright orientation. This follows the illustrated convention and the owner's preference. Calling counterclockwise exhaustion a violation of an explicit written rule would overstate the evidence.

**Distinguishing example:** an upright card exhausted clockwise and one exhausted counterclockwise both represent exhaustion under the written rule; only the former matches this publisher illustration. A native inspection must still verify the app's actual direction from the intended viewing position.

### Kicking the Hornet's Nest

The printed event searches the top **nine** encounter cards for one **non-Elite enemy**, which spawns engaged with the investigator instead of at its usual spawn location. Successful completion enables discovery of one clue at that location and resources equal to the enemy's **health value**. The event ends with an encounter-deck shuffle instruction. The printed wording does not say “printed health” or “remaining health”; those must not be substituted in a general rules claim. [Inspected card/transcription][hornet]. The official FAQ p24 independently confirms the spawn override, including bypass of concealed enemies' usual shadows destination. [FAQ][faq].

**Worked no-match case:** nine searched cards contain only treacheries and Elite enemies. No enemy is selected and the dependent clue/resource reward does not occur. The recommended presentation acknowledges the unsuccessful search and shows its cleanup; it must not fabricate an eligible enemy, resolve every searched treachery, or reveal card ten. Retain the final shuffle as the card's separate cleanup instruction. This no-match sequence is derived from the card plus Search/Then, not a located card-specific publisher Q&A. [Card][hornet]; [RR pp18,20][rr].

### What a temporary surface may show

Search permits looking at the entire searched region; searched cards retain their original game area until an instruction moves them. Looking, searching and finding are distinct effects. Merely displaying a searched treachery does not draw it or trigger Revelation. [RR p18][rr]; [FAQ §2.13][faq].

**Derived presentation requirement:** show all nine inspected cards, including ineligible cards, while marking only legal enemies selectable. Moving them visually onto a tray must not make that tray an additional rules zone. Card identity, original zone, permitted visibility and legal choices still come from the rules authority. The display must not expose cards outside the permitted region or keep them inspectable after concealment/shuffling. A click menu can offer Inspect for every visible card and the relevant selection only for legal candidates.

**Worked interruption:** Perception's success draw occurs while applying test results, before test cleanup. If it draws a weakness, its revelation must resolve without losing the parent test. A search/revelation surface therefore needs coexistence with retained test presentation rather than clearing the test area when any non-test card appears. This is a derived stress case for the proposed presentation, not native evidence. [Perception][perception]; [RR pp18,26][rr].

**Worked surge:** finish the drawn encounter, then draw the next encounter required by surge. The temporary surface must preserve that sequence rather than showing an invented simultaneous batch. [RR p19][rr].

## Current implementation

GitHub readback on 2026-11-05 found #89 to be the sole open issue. The physical-card and choice-presentation follow-ups #100 and #101 are closed and their changes are present at the inspected commit. The [current #89 handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060) still retains the separate full-scenario native-play and owner-acceptance boundary.

### Exhaustion and Tony's caption

[Gathering asset rendering](../../src/gathering_tabletop/play.rs#L688) and [engaged enemy rendering](../../src/gathering_tabletop/play.rs#L929) use positive 90-degree Y rotation. The [card mesh](../../src/tabletop_pieces/card_shape.rs#L30) places the printed top at local -Z, so that angle moves the title edge left; negative 90 degrees moves it right. Enemy rendering also omits rotation when an exhausted enemy is unengaged. The proposed correction covers exhausted assets and enemies, including evaded enemies, with appropriate clearance in location piles. Landscape investigator and act/agenda orientation is a different use of rotation and must retain its intended orientation.

The exact caption is `Fight / Engage` followed by `Enemy with bounty`, emitted by [the shared action-marker renderer](../../src/tabletop_counters.rs#L378). It sits beside Tony's additional-action marker, which explains its apparent association with Threats. [ADR 0002](../adr/0002-persistent-table-areas.md#selected-artwork-and-bonus-marker-recommendation) deliberately required it: the extra disk and plus badge alone did not explain the restriction. It is neither debugging text nor a rules control.

Recommend removing the permanent caption while keeping the additional marker and all accounting. [Tony's inspection text](../../src/gathering/cards.rs#L554) and [the legal action labels](../../src/gathering/runtime.rs#L2226) already explain the restriction when relevant. Shared removal also changes the earlier Tony and Party presentations. A marker tooltip would require new picking behavior; existing inspection and action wording are the simpler sufficient explanation.

### Complete staging inventory at the inspected revision

The [current rendering block](../../src/gathering_tabletop/play.rs#L965) combines committed cards, search results and resolving cards into one count and layout. Its six-card rows put a second search row into the token region. Hornet's nine candidates plus its resolving event need ten positions; synthetic summary faces also compete for the source-card position. Changing only the caption or row spacing would leave Test's broader role intact.

| Current content in Test | Actual consumers | Recommended home |
| --- | --- | --- |
| Encounter searches | Kicking the Hornet's Nest and On the Hunt, nine searched cards plus the event | Temporary tray, full 3-by-3 candidate grid and separate source-event position |
| Investigator-deck search | Backpack, six cards and up to three selections | Tray, preserving selection and explicit finish |
| Pending revelation | Ordinary encounter draws and drawn weaknesses | Drawing investigator's tray; actual skill-test presentation moves to Test if needed |
| Other resolving cards | Event consequences, cancellation/prevention, asset placement/opportunity attacks, retry events, pending Quarry spawn and weakness resolution order | Tray, except cards directly participating in a skill test |
| Pending discard order | Ordered discards, including upkeep and hand effects | Owner's tray, retaining legal order decisions |
| Non-test chaos-token resolution | Voice of Ra, Dark Future and Premonition | Tray, with distinct revealed/ignored/canceled token positions |
| Persistent Premonition | The in-play card and its sealed token after initial resolution | Controller's investigator area, until the effect consumes/discards them |
| Actual skill-test components | Test source, commitments, skill-test tokens and associated test reactions | Test, retaining the bag, reference card and curse supply at their established homes |

This inventory follows the normal pending-view projection in [presentation.rs](../../src/gathering/presentation.rs), the [token-purpose variants](../../src/gathering/tokens.rs#L618), and the rendering block through [Premonition placement](../../src/gathering_tabletop/play.rs#L1168). Captured intermediate views have two additional producers: `draws::advance` in [draws.rs](../../src/gathering/draws.rs) adds drawn weaknesses and their owner, and `tokens::resume` in [tokens.rs](../../src/gathering/tokens.rs) includes detached token resolution and its paid cards. Both pass the supplemented view to `Game::capture_public_view`; changes to resolving cards, ownership or test routing must cover all three producers. A non-test effect can interrupt a test: route by the actual effect, not merely by whether a test exists anywhere in the pending state.

## Compared designs

The required benefit is readable special-card resolution with physical movement and the existing card menus, while Test communicates skill tests clearly. These comparisons are engineering judgments from the current consumer and renderer, not measured usability results.

| Option | Benefit | Cost or limitation |
| --- | --- | --- |
| Improve spacing on Test | Smallest patch | Preserves the conflicting uses and can still collide with an interrupted test |
| Permanent shared resolution table beside Test | Simplest credible separate surface; reuses existing scene interaction | Permanent furniture and additional travel; weak association with the investigator resolving the effect |
| Temporary drawer at the originating investigator | Physical continuity, clear ownership, clear surface after use | Needs opening/closing lifecycle, dedicated framing and motion for searched cards |
| Interactive modal | Predictable screen framing | Existing modals block scene actions/motion; continuous physical travel and card menus would need additional interaction handling and reduce visible board context |

**Selected: the temporary 3D drawer**, called a resolution tray. It extends toward the viewer from the originating investigator's area and retracts after all its temporary components reach their destinations. Only a needed tray is visible. The existing Inspect modal remains the enlarged reading view. A normal card click retains the current separate action-selection behavior.

[Current geometry](../../src/tabletop_layout.rs#L87) leaves open space in front of investigator surfaces: they end at z=30, with the hand centered at z=28. A provisional 14-by-10 surface centered at the investigator's x coordinate and z=35.6 leaves a 0.6-unit gap; a 3-by-3 grid plus source-card position fits the existing 1.35-by-1.89 cards. Exact placement and opening motion remain implementation choices. This calculation establishes available space, not legibility in the native camera.

Provide a temporary **Resolve** focus shortcut. Keep manual camera control as the default; opening a tray does not force a camera jump. Optional Follow may frame the current resolution. The current investigator view does not include the entire proposed tray comfortably, so a dedicated framing is necessary. Changing focus or dismissing a card menu never completes or cancels the pending effect.

## Lifecycle and verification consequences

- Keep a tray associated with the originating effect owner through nested choices. A response answered by another investigator must not move the tray to the responder. The selected level-0 Ward of Protection only cancels its controller's own draw; use a legal shared response when checking cross-investigator ownership. Existing typed frames carry the owners; derive only the owner/group information needed for this presentation.
- Search cards currently have selection sources but no physical motion identity. Visible deck-to-tray travel needs actual card identities and permitted origins, not only different coordinates. Show the disclosed region, mark legal choices, preserve explicit finish/decline decisions, and return/shuffle remaining cards without consuming additional randomness.
- A tray is presentation, not a new game zone. Never duplicate an authoritative physical card between tray, Test and its destination. When a nested skill test starts, preserve its parent resolution and give the test its established home; finish the entire chain before removing the tray.
- Save/reopen reconstructs the current pending resolution and selections without repeating committed effects. Keep hidden-information Undo barriers, stale-choice rejection, failed-save behavior, reduced motion and missing-art substitutes.
- A native candidate must exercise Hornet's full nine-card case, no match, Backpack selection/finish, a non-test token effect, a revelation with a nested test, and interruption/reopen. Check clockwise exhaustion for both an asset and an evaded enemy. Headless geometry and continuity checks cannot establish readable framing, comfortable menus or animation pacing.

## Accepted design, 2026-11-05

The requested clockwise correction has a source-backed visual convention and an identified implementation cause. The owner also explicitly wants Test dedicated to tests and asks for the other affected cards to be covered. Apply that boundary to the complete inventory above: this revises [ADR 0002's all-encounter-draws-through-Test convention](../adr/0002-persistent-table-areas.md#gathering-card-draws), and gives persistent Premonition a home with its controller. These consequences must be visible in the proposal, but the already-stated Test boundary does not need a duplicate approval.

Joel answered “As you recommend” to the investigator drawer and “as you recommend” to removing Tony's caption. He also requested that the Test boundary be recorded under `docs/` so future card implementations do not place unrelated content there. These decisions are settled: the [ADR refinement](../adr/0002-persistent-table-areas.md#test-area-and-resolution-trays) records the contract and supersedes the conflicting earlier staging/caption instructions, the glossary defines resolution tray, and README links the contract. The geometry remains adjustable. Native readability was untested at this design stage; the subsequent [owner acceptance](#owner-acceptance-and-publication) records the live-app result.

## Accepted implementation handoff

The next increment under #89 is one local Rust candidate applying the accepted three corrections at `4e610893604d05eb256a2dfe1e19dd43c8004d54`: clockwise exhaustion for Gathering assets and enemies, resolution trays for the complete current staging inventory, and shared removal of Tony's permanent caption. This directly addresses the owner's reported play problems; broader full-scenario acceptance remains with #89.

- **Scope:** derive the needed resolution ownership from existing rules frames; add local tray layout, visibility, physical travel and Resolve framing; route card/token presentation and existing choices; preserve nested tests, exact continuation, hidden information and the existing rules authority. No general card-effect framework or new rules-zone model is selected.
- **Verification:** focused production-path checks for the finite cases above, the repository's required Rust checks and an exact candidate review. Prepare isolated review saves and exact launch steps for an owner check of orientation, full search layout, menus, motion, nested test handling and reopening. Automated checks do not accept the native experience.
- **Accepted allowance:** up to six aggregate active engineering hours for code, tests, documentation and review. Return earlier at two active hours without an inspectable increment, a material authority/scope gap, new capability class, or the applicable non-convergence limit. Do not extend the allowance automatically.
- **Human review:** propose one prepared owner review with a cumulative ceiling of fifteen minutes, including retries, to judge readability, card interaction and motion. A desktop-control interval would still need its exact bounds before agent-driven interaction; preparation does not grant desktop control.
- **Acceptance boundary:** return the local patch, completed check results and review instructions. The design and local execution allowance are accepted. Publication, merging and #89 closure are separate. Personal saves and prepared artwork retain custody.

Joel instructed “Proceed with the implementation as you recommend.” on 2026-11-05. Local implementation began at 15:34:32 UTC from the fixed point above, retaining the four design-document changes. This activates this new six-hour increment and the proposed owner-review ceiling, not historical exhausted allowances. The agreed test seams are production Session observations/choices and save/reopen for rules continuity, and production tabletop registration/rendering/input for routing, geometry, interaction and motion. Those seams are exercised by focused regressions and the existing repository gate; they do not establish native comfort or owner acceptance. This document supports #89 rather than forming a second work queue.

## Local implementation and review route

The inherited presentation is the Party-aligned Gathering implementation at the
fixed point `4e610893604d05eb256a2dfe1e19dd43c8004d54`. The affected production
paths retain `tabletop_pieces` cards/captions, `tabletop_counters` investigator
counters, `tabletop_feedback::menu` contextual controls, `tabletop_tokens` token
faces, the existing portrait/navigation and inspection panels, and Gathering's
saved-view motion and stale-choice guards. Authorized differences are clockwise
Gathering exhaustion, the listed temporary placements and travel, Resolve
framing, Premonition's lasting home, and removal of Tony's caption in all shared
consumers (Tony, Party and Gathering). Other counters, action allowance, printed
inspection text, fixed table anchors and modal input barriers retain their
existing behavior. The new drawer is a local Gathering adapter; this is not a
shared layout redesign. Joel's live-app comparison and acceptance are recorded
below.

The candidate derives tray ownership from the existing continuation frames and
token purpose. `PendingView` identifies the original owner and test reactions;
the rules state and save format are unchanged. The tabletop renders a temporary
14-unit-wide drawer in front of that investigator, a three-column candidate
grid and a separate source rail. Individual cards retain their contextual menus
and Inspect; clicking the drawer retains the aggregate menu. **Resolve** frames
the drawer, and ordinary manual framing stays put when an effect opens it.

Search faces now have physical identities and travel from the actual deck.
Unchosen cards turn face down on return; Backpack attachments go to the asset.
All cards in an ordered discard travel to the pile, including buried cards that
the lasting pile representation no longer draws separately. Patrice orders her
upkeep discards in her drawer; those cards retain their rules-zone membership
until the discard completes and have only one physical face. The drawer
waits for departure motion before retracting. Reduced motion settles immediately;
reopening reconstructs the pending arrangement without repeating the reveal.
Revelations that initiate tests move their physical source into Test; a separate
cancellation test can coexist with the original revelation in the tray.
Premonition and sealed tokens have a lasting investigator-area position.

The native review uses only ordinary sessions generated through legal choices.
The bounded preparation found fixed prefixes for seeds 0, 1, 2, 3, 5 and 405;
the retained helper runs those fixed prefixes, without fixture constructors or
seed search. It refuses an existing output directory and round-trips every save.
The prepared files are in `/tmp/arkham-tray-review-saves-final`. To regenerate, substitute
a fresh directory:

```sh
scripts/dev cargo run --locked --example gathering_resolution_review -- \
  /tmp/arkham-tray-review-saves-final
```

Build once, then launch one checkpoint at a time. Change the save filename and
initial view using the table below. Closing and repeating a command reopens that
review attempt; **Menu → Reopen saved attempt** tests an in-place reopen.

```sh
scripts/dev cargo build --locked --features tabletop --bin arkham-tabletop
target/debug/arkham-tabletop --experiment gathering \
  --save /tmp/arkham-tray-review-saves-final/hornet.json --initial-view tony \
  --art-root /home/joeloverbeck/.cache/agent-tmp/arkham-89-art-menu/gathering-assets-v1
```

| Save filename | Initial view | Review action and expected result |
| --- | --- | --- |
| `hornet.json` | `tony` | Play Kicking the Hornet's Nest, then press Resolve. Nine separate faces and the event occupy the drawer; select a legal enemy from its card menu. Cards travel to the encounter deck or enemy destination. Resolve Bounty Contracts and finish the event; the empty drawer retracts. |
| `hornet-pending.json` | `tony` | Press Resolve, Inspect a candidate, close Inspect and reopen the session. The same nine cards and legal choices remain, with no new draw or shuffle. |
| `no-match.json` | `tony` | Resolve shows nine cards with no eligible enemy. Take the offered no-match choice; cards return and the drawer closes. |
| `backpack.json` | `tony` | Take Backpack's search reaction, press Resolve, select up to three eligible cards and finish selection. Unchosen cards return; selected cards remain inspectable through Backpack. |
| `voice-of-ra.json` | `jacqueline` | Press Resolve and reveal tokens. The event and non-test tokens use the drawer. No skill-test commitments or source appear in Test. |
| `premonition.json` | `jacqueline` | Press Resolve and reveal. The drawer closes and Premonition with its sealed token remains beside Jacqueline's assets. |
| `patrice-upkeep.json` | `patrice` | Press Resolve, order the five non-weakness cards using their menus and refill her hand. Each card appears once in her drawer, then travels to her discard before the replacement draw. |
| `exhaust-asset.json` | `patrice` | Use Patrice's Violin, discarding Old Keyring to gain a resource. Its printed top turns clockwise to the right. |
| `evaded-enemy.json` | `scenario` | Continue past the saved result reading. The evaded Swarm of Rats beside Study is clockwise, although unengaged. |
| `revelation.json` | `patrice` | Draw the offered encounter. Its revelation enters the drawer, then its source moves to Test for the skill test. |
| `revelation-pending.json` | `test` | Reopen the saved revelation test. One physical source remains in Test, with the original choices and no repeated effect. |

Keep the cumulative owner review within fifteen minutes, including retries.
Check the nine-card layout at the usual display size, click individual faces and
the drawer, try Inspect and manual camera movement, and check that Tony's extra
action marker still exists without the permanent caption. Stop at a blocked
choice, missing face, lost continuation or unreadable layout and retain that
isolated save. This prepared route does not claim that Joel executed every listed
step; his subsequent live-app acceptance is recorded below. It does not authorize
agent-driven desktop control.

Local automated verification: the full `scripts/check` gate finished with exit 0
in `/tmp/arkham-tray-full-check5/run.json` (index/tool checks, formatting, strict
Clippy, application tests and renderer regression tests). The application library
reported 423 passing tests and one intentionally ignored test. Production-path
regressions cover search layout and menus, clockwise assets/unengaged enemies
with standee clearance, deck travel and no-match cleanup, Backpack
selection/reopen, a retained revelation beside a cancellation test,
Voice of Ra/Premonition tokens, weakness draws and ordered discards. Independent
review exposed detached token/draw snapshots losing their owner, queued discard
groups using another owner's drawer, and missing Patrice upkeep staging. Each
was reproduced before correction; the completed gate includes those regressions.
The ordinary-save exporter finished with exit 0 in
`/tmp/arkham-tray-native-saves-final-check/run.json`; hashes of its eleven retained
saves are in `/tmp/arkham-tray-review-save-hashes-final.txt`. The native executable
build finished with exit 0 in `/tmp/arkham-tray-native-build-final/run.json`; its
hash is retained beside that record in `executable.sha256`.
Tests establish these observed
paths and geometry, including 1600×1000 and 3200×2000 viewports, not native
readability or physical interaction. Earlier failed attempts remain in adjacent
`/tmp/arkham-tray-*` check directories; only the finished successful records above
support these claims.

## Owner acceptance and publication

On 2026-11-05, Joel reported: "I've checked the live app and the changes are
better than the previous state, particularly the investigator drawers. I
authorize publication, creating PR, merging PR, pulling from main."
This accepts the bounded presentation candidate and authorizes its publication
through the protected-main workflow. It does not claim a complete scenario
playthrough, enumerate unreported review steps, or close #89.

The accepted source candidate is the 26-path working-tree snapshot
`/tmp/arkham-tray-review-v3` over
`4e610893604d05eb256a2dfe1e19dd43c8004d54`, manifest SHA-256
`8f5e0502810445354a00d3e3e55d01dabf06164bee9b9e0dd465cc59e363fe91`.
Both independent reviews reported zero remaining findings. The prepared
executable SHA-256 is
`a87c1e49407948560a2349c1f089bccbe1484097c604e55f1d3dea5907442f22`.
The publication refresh changes only README/ADR/research acceptance wording;
implementation and automated evidence remain unchanged. The PR enumerates the
published commit and file set, and #89's current handoff retains the resulting
publication status. Consulted for publication: `AUTH-1`–`AUTH-3`, `OPS-5`–`OPS-7`
and `PUB-1`.

## Retrospective follow-up

Joel accepted three bounded follow-up changes after PR #108: make the pre-merge checks and head-matched merge one fail-closed Rust command, correct the presentation-producer map, and reject `record-check -- scripts/dev scripts/check` before it waits on its own checkout lock. The starting commit is `dc5b7c01c9dd4870a9ea1ff668f3ecd90a006397`. The agreed verification seam is the tools CLI: a Rust `gh` fixture observes whether rejected prerequisites reach a merge, and the check-runner fixture observes whether an invalid composition starts a child. Existing detached-draw/token regressions remain the presentation evidence. This increment authorizes local implementation and verification; publication remains a separate instruction. It adds no gameplay behavior, settings changes or automatic publication authority.

The merge procedure lives in [GitHub administration](../agents/github-administration.md#merge-an-accepted-pr); this supporting note is not another work queue. Consulted for the follow-up: `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`, `AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7`, `PUB-1` and `GOV-1`–`GOV-3`.

## Evidence limits

The rules settle visibility, legal choices and timing; they do not choose a modal, drawer, table position, camera policy or animation duration. Those are product decisions requiring the repository's actual interaction path and owner judgment. Research supports clockwise convention and separation of non-test resolution from the test surface. It does not establish that any proposed layout is readable, comfortable or complete.

Consulted for this investigation and design proposal: `AUTH-1`–`AUTH-3`, `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`, `AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7` and `GOV-1`–`GOV-3`. The existing issue remains the active work-control surface; this file is supporting evidence.

[rr]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf
[old-rr]: https://images-cdn.fantasyflightgames.com/filer_public/c4/b0/c4b0d66c-d79e-411b-bdb5-b5d8c457d4bc/ahc01_rules_reference_web.pdf
[ltp]: https://images-cdn.fantasyflightgames.com/filer_public/dd/78/dd7818fe-0c9a-4a6c-b685-e32ab55b1702/ahc60_learn_to_play_web.pdf
[faq]: https://images-cdn.fantasyflightgames.com/filer_public/c1/d0/c1d0fab6-7fa6-4ce2-af6a-16416381a19b/ahc_faq_v25_february_2026-web.pdf
[hornet]: https://arkhamdb.com/card/09068
[perception]: https://arkhamdb.com/card/01090
