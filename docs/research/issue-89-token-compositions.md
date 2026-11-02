# Selected token composition: source findings

2026-11-02. Source investigation and bounded implementation evidence for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89). Ordinary play remains gated.

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

Keep outer positions, physical-token membership, declaration order, and each ignore/cancel decision separately representable. The existing J1–J4 witnesses can remain valid without proving these broader combinations.

## Bounded Eyes → Olive candidate

The implementation follows the **qualified source-derived interpretation**, not a published exact Eyes/Olive ruling. Its finite rule is: declare Olive on one Eyes position before revealing; reveal the entire batch together; choose an Eyes position while Olive's group is intact; ignore the unchosen positions; only if Olive's position survives, choose its two survivors. On success, compare every surviving member of the chosen position with ignored tokens, awarding at most one additional clue. A token ignored inside the selected group is not itself a surviving chosen token. Cancellation is distinct from ignoring.

This follows the published Statue/Olive outer-before-inner order (FAQ p17), with Eyes supplying the same outer choose-one shape. The multiple-token treatment and one-trigger limit come from §2.5 (p12). Applying Eyes' matching rule to inner ignored tokens follows the explicit Eyes/Katarina example (p22); applying that rule to Olive's surviving group is a derivation. These sources justify this reversible candidate without manufacturing an exact publisher example.

The public Session witnesses use original Study (shroud 2), Jacqueline's unmodified willpower 5, no Ghouls, no Dark Future, one spent Eyes charge, and exhausted Olive. The independent physical batch is `G=[-1,-2,skull]`, `A=[-1]`; skull is 0 here.

| Player decision | Resolved tokens | Ignored tokens | Total / clues |
| --- | --- | --- | --- |
| Choose A, with no inner Olive choice | A's `-1` | All three G members | 4 / 2 |
| Choose G, then keep `-1` and skull | G's `-1, skull` | G's `-2`, A's `-1` | 4 / 2 |
| Choose G, then keep `-2` and skull | G's `-2, skull` | Both `-1` tokens | 3 / 1 |

A second-position variant uses the same physical draw order with `A=[-1]`, `G=[-2,skull,-1]`. Choosing G and retaining `-2,-1` gives total 2 and 2 clues: only the later retained member matches A. This challenges both position identity and checking all survivors.

`tests/gathering_tokens.rs` exercises these outcomes through real Eyes activation, not a direct test-result setter. Pending outer and inner selections save/reopen with the same four held physical tokens and choices; replay reproduces the selected outcomes. Bag custody is 16 → 12 → 16; the action, charge and Olive exhaustion happen once. The outer window refuses a forged flattened token choice. A separate witness with active Dark Future refuses Olive's nested declaration without exhausting it or changing the saved state.

The implemented path does **not** settle Dark Future's protected partial groups, Dark Prophecy's mixed-group preference, Jacqueline's mixed-group cancellation, or Premonition replacing a multi-token request. Those restrictions remain. The fixed decks also do not permit simultaneous Olive/Katarina or two controlled copies of unique Olive, so those are not invented requirements for this candidate. This bounded consumer evidence is neither full selected-card coverage nor ordinary/native play acceptance.

## Remaining playable-entry dependency

The follow-up source audit found no additional attributable publisher ruling
that eliminates these differences. An exact published example for every
combination is unnecessary; general rules can establish an implementation.
Here the unresolved choices change physical draws, cancellation opportunities
or mandatory weakness resolution, so a convenient test outcome is insufficient.

The clearest mandatory case is Premonition remaining in play when Jacqueline
ends her turn with Dark Future. The latter requests five tokens. With two
numerical tokens sealed on Premonition, whole-request replacement resolves only
those two and leaves Dark Future in play. One-position replacement leaves four
bag positions, which could include elder sign and remove the weakness. The
whole-request reading remains the strongest provisional interpretation; it does
not settle the other mixed-group predicates.

This reachability is source-derived and the current refusal is code-confirmed:
the turn-end frame calls `tokens::begin_dark_future`, whose five-token request
reaches the multi-token Premonition guard. No combined public Session execution
is claimed for that mandatory case. Ordinary entry remains closed. Voice of Ra
with sealed Premonition is now withheld before payment through the shared action
eligibility path; a forged activation preserves the entire Session, while an
ordinary supported action still provokes the actual engaged enemy. This contains
the optional failure without asserting that the official composition is illegal
or that the mandatory Dark Future case has been solved.

The recommendation is to preserve this implementation and obtain the specific
publisher clarification. A build using explicit local interpretations or a
reduced interaction set would change the supported-content or fidelity promise.
The owner need not adjudicate the official rule. The Grim Rule (FAQ §1.5) is an
at-table expedient, not an independent oracle for hardcoding all such cases.

### Publisher question draft — not sent

These questions concern original versions under Legacy FAQ 2.5 and taboo 10,
including §2.5, §2.10 and the p17 Olive/Grotesque Statue nesting examples:

1. Jacqueline has Dark Future (`60403`) and Premonition (`04199`) holding two
   numerical tokens. Does Premonition replace the whole mandatory five-token
   request, or one position with four bag tokens still to reveal? Does the same
   answer govern Voice of Ra's (`60416`, taboo) three-token request and symbol
   reward? If positions remain, can their optional would-reveal reactions occur?
2. In a normal one-token test, Jacqueline (`60401`) adds two tokens and Olive
   (`04197`) replaces one position. The batch is `G=[auto-fail,-1,-2]`, `A=[0]`,
   `B=[+1]`. What is the selection/cancellation order and cancellation unit?
   Can G be cancelled as one auto-fail unit, or must physical tokens be selected?
   How does Dark Future affect a mixed protected/unprotected group?
3. Dark Prophecy (`04032`, taboo) is outermost, Olive occupies one of its five
   positions, that group is `[skull,-1,-2]` and the other positions are numerical.
   Must the group be selected for containing skull? May Olive then ignore that
   sole skull, or is eligibility checked against the eventual survivors?
4. Eyes of the Dreamer (`11067`) reveals one additional token, with Olive on one
   position and Dark Future active. The positions are `G=[skull,-1,-2]` and
   `A=[0]`. What outer/inner selections are legal when the unchosen group would
   contain a symbol that cannot be ignored?

No external inquiry has been sent. These questions isolate replacement scope,
cancellation units, symbol eligibility and protected partial groups.
