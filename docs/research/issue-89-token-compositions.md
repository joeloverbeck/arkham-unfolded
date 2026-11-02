# Selected token composition: source findings

2026-11-02. Read-only investigation for #89; no implementation or activation claim.

The sources establish group preservation and advance declaration. They do **not** supply an exact ruling for every selected composition. Eyes/Olive has a useful source-derived interpretation. Dark Prophecy's mixed-group eligibility, Jacqueline's cancellation of a mixed group, and Premonition's replacement scope on an existing multi-token request remain qualified below.

## Source binding

- [Legacy FAQ 2.5](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf), SHA-256 `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`: p12 §2.5, p13 §2.10, p17 Olive/Statue, p22 Eyes/Katarina. Local PDF hash rechecked; p17 rendered and visually inspected. Text locations: `/tmp/arkham-89-implementation/faq-2.5.txt` 711–738, 803–815, 1092–1118, 1452–1457.
- [Rules Reference](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf), SHA-256 `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`: p2 A/An, pp2–3 Forced Abilities, p13 Instead, p20 Triggering Condition. Local hash rechecked. Text locations: `/tmp/arkham-89-implementation/rules-reference.txt` 6–10, 45–76, 810–824, 1297–1310.
- Original selected faces and taboo 10 remain bound by `docs/research/issue-89-rules-jacqueline.md`; no Chapter Two text substitution. Relevant identities: [Olive 04197](https://arkhamdb.com/card/04197), [Dark Prophecy 04032](https://arkhamdb.com/card/04032), [Jacqueline 60401](https://arkhamdb.com/card/60401), [Eyes 11067](https://arkhamdb.com/card/11067), [Premonition 04199](https://arkhamdb.com/card/04199), [Voice of Ra 60416](https://arkhamdb.com/card/60416), [Dark Future 60403](https://arkhamdb.com/card/60403).

The ArkhamDB FAQ sections reproduce the publisher's group and sealed-token rulings. Their Premonition §2.10 quotation is stale: the retained current FAQ lets other effects see the altered state. Jacqueline, Voice of Ra and Dark Future currently have no FAQ entry there. Searches for the exact compositions found community answers, but no additional attributable official response settling the gaps. Those community answers are not the oracle. This is a bounded search result, not a claim that no such response exists anywhere.

## What the group ruling requires

FAQ p17 requires declaring the nesting before the simultaneous physical reveal. In its Statue-outer example, the player first selects either the ordinary outer slot or the entire three-token Olive group; only if that group is selected does Olive choose its two survivors. Thus indiscriminate inner-first resolution and flattening before every outer choice cannot implement the published example.

| Selected composition | Source-supported consequence | Remaining interpretation |
| --- | --- | --- |
| Dark Prophecy outer, Olive on one of its five positions | Seven physical tokens; five outer positions, including one Olive group. Preserve membership and the declared nesting. | The FAQ does not define whether a mixed group qualifies for Prophecy's preferred symbol category using any member, all members, or Olive's eventual survivors. Nor does it explicitly reconcile eligibility checking with the outer-before-inner order. |
| Eyes outer, Olive on one position | With one Eyes charge: four physical tokens, two outer positions. By direct structural analogy to Statue, choose the ordinary position or the whole Olive group; selecting the group then invokes Olive's keep-two choice. | This is a source-derived composition, not a published Eyes/Olive example. Applying the Eyes match condition to two selected survivors is derived from §2.5 and its single-trigger rule. |
| Jacqueline and Olive | The declared placement of Jacqueline's addition and Olive's replacement matters; both abilities must be committed before seeing the batch. | Jacqueline adds to “any number” and cancels by an auto-fail/non-auto-fail predicate. The source does not explicitly define cancellation units or that predicate on a group containing both types. Neither always flattening nor blindly treating any group containing auto-fail as a single cancellable auto-fail has been established. |

Useful distinguishing Prophecy case: its Olive group is `[skull, -1, -2]`, and all four other positions are numbers. Choosing the group and subsequently ignoring its sole skull can differ from resolving Olive first and only then checking Prophecy eligibility. A test selecting one convenient outcome would encode the unresolved interpretation rather than settle it.

For the Eyes inference, let its two positions be `G=[-1, -2, skull]` and `A=[-1]`, without Dark Future. Selecting A ignores all of G; selecting G then retaining `-1` and skull ignores `-2` and A. Either route has a matching ignored `-1`. The predicted success award is one additional clue, not one per matching pair. FAQ p22 explicitly establishes that an inner ability's ignored tokens can participate in Eyes matching; extending that to Olive is an inference. Preserve ignored-token provenance and distinguish cancellation from ignoring. Do not generalize this unqualified example over Dark Future's prohibition.

## Premonition: established priority, qualified scope

The accepted J4 single-token case stands. Premonition's mandatory replacement precedes optional reactions to that same would-reveal condition. Resolving a token **as if just revealed** does not itself recreate a future would-reveal opportunity. §2.5 expressly requires resolving every token sealed by Olive, rather than choosing only one sealed token. Current §2.10 still permits effects keyed to the substituted revealed state; it does not erase those effects.

The overlooked A/An rule matters: a condition using “a” can be satisfied by one or more elements. Therefore Premonition's singular condition cannot, by itself, prove one-position replacement. Conversely, satisfying that condition does not by itself prove the extent of the replacement.

**Leading source-derived interpretation, not certified by an exact publisher example:** Premonition replaces the entire pending simultaneous reveal with its sealed group. RR Instead replaces the triggering condition's resolution, and Premonition lacks Olive/Prophecy's express “instead of 1” limitation. Under that interpretation:

- Voice of Ra's three-token request resolves only the sealed tokens; its initial resource remains gained and its taboo award counts their symbols.
- Dark Future's five-token request resolves only the sealed tokens; its cleanup checks those tokens for elder sign.
- There is no remaining original bag request on which to declare Jacqueline, Olive or Dark Prophecy. Do not consume their costs/uses on that preempted request. A genuinely later separate reveal gets its own opportunity.
- Eyes' charge decision precedes the reveal request. Its interaction with forced substitution and an already-established choose-one instruction needs its own witness, especially if Premonition holds two tokens. The text does not establish that charges refund, that sealed tokens flatten into ordinary selectable slots, or that spending charges bypasses the forced replacement.

The competing one-position reading would leave N−1 pending positions and materially different optional opportunities. No exact Legacy FAQ or attributable official response located here chooses between those readings. **Retain an explicit unsupported boundary for the unresolved multi-token compositions; do not promote the leading interpretation to a certified oracle merely because it fits the implementation.** No owner judgment is needed to identify this source gap, and no external rules inquiry was sent.

## Implementation consequence

Keep outer positions, physical-token membership, declaration order, and each ignore/cancel decision separately representable. The existing J1–J4 witnesses can remain valid without proving these broader combinations. A later consumer test should use the distinguishing cases above only after its interpretation is explicitly justified. No code was edited in this investigation.
