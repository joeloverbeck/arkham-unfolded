# Nested token choices: follow-up derivation

2026-11-02. Bounded research for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89), original card faces, Legacy FAQ 2.5 and taboo 10. This note supplies source-qualified expectations for the remaining selected compositions. It does not report implementation, native play, owner acceptance, or a newly obtained publisher answer. No external inquiry was sent.

The earlier [composition note](issue-89-token-compositions.md) correctly preserves groups, but its demand for further publisher clarification is too strong where the general rules and published analogous examples support a finite interpretation. The recommendations below separate those derivations from exact quotations or rulings. The interpreter is the researching agent; the controlling authority remains the publisher sources. Recheck if those sources change or an attributable contrary ruling appears.

Consulted constitution: `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `EVID-1`–`EVID-10`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `AGENT-5`, `OPS-1`–`OPS-7`. Research serves the currently blocked Gathering consumer, without adding cards, infrastructure, publication, or runtime activation.

## Evidence and the ordering decision

The controlling [Legacy FAQ 2.5][faq] remains SHA-256 `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`. Its §1.4 requires nested sequences to finish before returning to their parent. Section 2.5 applies singular token references across surviving resolved tokens and uses an existential test for symbol-dependent conditions. Page 17 preserves replacement groups and requires declarations before the simultaneous physical reveal. The [Rules Reference][rr], SHA-256 `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`, supplies absolute **cannot** (p6), partial resolution of effects (p9), and valid targets (p20). Relevant passages were read from the retained text extracts in `/tmp/arkham-89-implementation/`.

The p17 Statue-outer example narrates selecting the whole Olive group before Olive's selection. It does not establish that every outer symbol predicate must be evaluated against every physical token originally in that group. The same answer resolves an inner Statue choice before an outer Olive choice. Moreover, an [answer attributed to Matt Newman, reproduced on Dark Prophecy's page][prophecy], explicitly resolves the inner Statue choice before checking Dark Prophecy's eligible symbols. That attributed answer was present by 2018-06-05; it is useful corroboration, below the published FAQ in authority.

**Selected interpretation:** preserve each replacement group as one candidate for the parent. Where the parent has a token-type restriction, first finish the inner choice so that its surviving group has a determinate type; then evaluate that type restriction against the surviving members. A group has a specified symbol when at least one surviving member has it. A *non-auto-fail* group has no surviving auto-fail member. This applies the published group rule without treating the group as an anonymous scalar or flattening it into independent parent choices. This is a derivation, not a verbatim general rule in the FAQ.

For the already implemented unqualified Eyes choose-one operation, choosing an outer group before making its inner choice remains the direct p17 analogy. No mandatory global rewrite to one universal ordering is claimed. The finite oracles below bind the relevant ordering rather than using an implementation's existing stack as evidence.

The strongest alternative checks the original three-token group before any inner selection. It loses for the symbol-restricted parent because it can make a subsequently ignored symbol determine eligibility, contrary to the nested-sequence rule, the attributed Prophecy/Statue answer, and the general treatment of fully ignored tokens. Absence of an exact Prophecy/Olive example does not make that alternative equally well supported.

## 1. Jacqueline and Olive

[Jacqueline 60401][jacqueline] adds two tokens to a reveal of **any number**. [Olive 04197][olive] replaces one position. They therefore admit two materially different declarations before the batch; do not silently substitute one for the other.

**Olive first, Jacqueline on Olive's pending reveal:** reveal five physical tokens, apply Jacqueline's cancellation across that five-token reveal, then Olive chooses two remaining tokens to resolve. Those are individual cancellation candidates, because Jacqueline was applied to the entire three-token reveal, not to one chosen Olive position. If Jacqueline cancels only the auto-fail, Olive still chooses two survivors; the explicit choose-two instruction controls even though the printed singular description of the ignored remainder was written for an ordinary three-token reveal. This latter cardinality is a source-derived reconciliation, and community discussion records disagreement about it.

**Jacqueline first, Olive on one declared position:** reveal five physical tokens in three parent positions. Complete Olive's keep-two selection, then cancel either two non-auto-fail parent positions or one auto-fail parent position. A group counts once; cancellation applies to all its surviving members, subject to Dark Future's member-specific prohibition.

Finite witness without Dark Future: parent positions `G=[auto-fail,-1,-2]`, `A=[0]`, `B=[+1]`.

| Olive ignores | Effective G | Jacqueline's complete legal cancellation choices | Surviving tokens |
| --- | --- | --- | --- |
| auto-fail | `[-1,-2]` | `G+A`, `G+B`, or `A+B` | respectively `[+1]`, `[0]`, or `[-1,-2]` |
| `-1` | `[auto-fail,-2]` | `G` as one auto-fail candidate, or `A+B` | respectively `[0,+1]` or `[auto-fail,-2]` |
| `-2` | `[auto-fail,-1]` | `G` as one auto-fail candidate, or `A+B` | respectively `[0,+1]` or `[auto-fail,-1]` |

Refuse selecting only one numeric member of G with A, counting G as two parent cancellations, or treating an auto-fail-containing G as non-auto-fail. Deliberately retaining auto-fail is a legal player choice.

**Dark Future refinement:** apply its prohibition to physical members, not by making a mixed group wholly immune or wholly cancellable. Example: `G=[skull,-1,-2]`, `A=[0]`, `B=[+1]`; Olive chooses skull and `-1`, ignoring `-2`. G is a non-auto-fail candidate. Choosing `G+A` cancels G's `-1` and A's `0`; skull survives and resolves with B's `+1`. Choosing `G+B` leaves skull and `0`; choosing `A+B` leaves skull and `-1`. The protected skull cannot vanish merely because a parent cancels its group. This partial-group consequence is derived from group preservation plus **cannot** and partial resolution; it is not an exact published example.

Do not reject the initial reveal merely because later cancellation can be impossible. Nor may a cancellation choice select a wholly protected target solely to avoid cancelling an available legal target. Retain the existing source-derived all-protected outcome: every protected token survives when no cancellation can take effect.

## 2. Dark Prophecy containing Olive

Taboo 10 makes [Dark Prophecy's][prophecy] preferred category a non-elder-sign symbol. It still replaces one position by five. Declare Olive on one of those five positions before revealing seven physical tokens. Finish Olive's selection; retain its two survivors as one of the five parent candidates. A candidate qualifies if **any surviving member** has a preferred symbol. If any candidate qualifies, choose a qualifying candidate; otherwise the printed fallback permits any candidate.

Finite witness, no Dark Future: `G=[skull,-1,-2]`, with ordinary positions `A=[0]`, `B=[+1]`, `C=[-1]`, `D=[-2]`.

| Olive keeps | Eligible Prophecy choices | Final resolution |
| --- | --- | --- |
| skull and `-1` | G only | skull and `-1` |
| skull and `-2` | G only | skull and `-2` |
| `-1` and `-2` | G, A, B, C, or D by fallback | chosen group, or the chosen ordinary token |

Thus the player **may** ignore the sole skull with Olive, but does so before Prophecy's category check. It is not legal to use that ignored skull to force G's selection while claiming eligibility from a token that no longer participates. If C is instead a cultist, the last row must choose C; the two-number G is ineligible. This second witness distinguishes survivor-based classification from classification of the original physical group.

With Dark Future, an attempted ignore never removes a protected member. For `G=[skull,-1,-2]`, choosing both numbers with Olive leaves all three members, because skull cannot be ignored. G still qualifies. With four ordinary numerical positions, G is therefore the only eligible Prophecy choice, whatever Olive's keep-two choice. The final resolution is G's surviving two or three members; numerical outer positions are ignored. This follows the prohibition, without silently making Olive unavailable.

## 3. Eyes, Olive, and a protected partial group

[Eyes 11067][eyes] has an unqualified choose-one step. With one charge, use the published Statue-outer shape: two outer positions, one replaced by Olive's three. Selecting the ordinary position attempts to ignore the entire other group. Dark Future prevents only the protected members from being ignored; it does not make the selection illegal and does not preserve the group's unprotected numerical members. Selecting G then chooses Olive's pair, likewise applying its ignore instruction only where permitted. This distinction follows **cannot** and partial effect resolution directly.

Finite witness: original Study, no Ghouls, Jacqueline willpower 5, shroud 2, Dark Future active, no other modifiers; `G=[skull,-1,-2]`, `A=[0]`. Skull's modifier is 0.

| Choice | Resolved physical tokens | Actually ignored tokens | Skill total / clues |
| --- | --- | --- | --- |
| Eyes chooses A | `0,skull` | G's `-1,-2` | 5 / 1 |
| Eyes chooses G; Olive chooses skull and `-1` | `skull,-1` | A's `0`, G's `-2` | 4 / 1 |
| Eyes chooses G; Olive chooses skull and `-2` | `skull,-2` | A's `0`, G's `-1` | 3 / 1 |
| Eyes chooses G; Olive chooses `-1,-2` | `skull,-1,-2` | A's `0` only | 2 / 1 |

The last row is legal: Olive chooses two tokens **to resolve**, not one token as an eligible target **to ignore**. The subsequent ignore instruction fails on skull. The first row is also legal: Eyes targets A for resolution, not G for a successful full ignore. Do not recast either into a restriction requiring all protected tokens to be in the selected set.

One matching witness distinguishes an attempted ignore from an actual ignore: replace A's `0` with a second skull. Whichever outer position is selected, both skulls survive. Neither is ignored, so their equality alone does not award Eyes' additional clue. In the ordinary witness, all selections succeed but none matches an actually ignored token. Costs, exhaustions, all physical draws, and choices occur once; all tokens return normally after resolution.

## Community challenge and remaining confidence limits

These are corroborating or contrary discussions, not publisher authority:

- [Reddit, 2020-11-02][reddit-2020]: a detailed claim that Jacqueline must replace one Olive position is explicitly retracted after reading **any number**. This supports distinguishing addition from replacement and warns against copying confident nested-token explanations.
- [ArkhamDB Olive, 2020-06-30][olive]: Phoenixbadger says a mixed pair cannot qualify for Prophecy; iceysnowman corrects it using §2.5 and an any-surviving-symbol test. Both discuss Olive's choice before Prophecy. The correction supplies reasoning, not an official response.
- [Reddit, 2023-04-16][reddit-2023]: commenters disagree whether Olive keeps two or three after Jacqueline cancels only auto-fail. Prefer the card's explicit choose-two instruction; do not turn the singular remainder into permission to resolve an unchosen third token.
- [Reddit, 2025-01-23][reddit-2025]: a new claim that Jacqueline affects only one Voice of Ra position is corrected using her different text. It independently supports the additive interpretation.
- [Dark Future discussions, 2021-05-30 and 2021-07-05][future]: commenters explain that Olive/Prophecy can reveal extra tokens while the weakness prevents symbol ignoring. Some surrounding arithmetic and blanket wording are wrong; the cited comments corroborate the prohibition rather than establish a full composition oracle.
- [Reddit, 2025-12-01][reddit-dec]: the later explanation treats a surviving Olive pair as one Jacqueline cancellation candidate. Its blanket LIFO wording should not replace the published specific group examples.

Searches also targeted BoardGameGeek, Hall of Arkham, and Mythos Busters. They produced no additional attributable response for the exact mixed-group cases within this bounded pass; Hall of Arkham's attempted rules URL was inaccessible. This is a search limit, not evidence of nonexistence.

The Eyes partial-ignore result and the additive Jacqueline distinction have the strongest direct textual basis. The surviving-group type predicate, mixed-group partial cancellation, and Olive's modified-count remainder are explicit derived interpretations. They provide finite expectations for a reversible implementation; passing those expectations would establish those specified consequences, not publisher certification of every combination. The selected source-qualified claim should remain visible in #89. Nothing here justifies changing original cards to newer versions or broadening the supported inventory.

[faq]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf
[rr]: https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf
[jacqueline]: https://arkhamdb.com/card/60401
[olive]: https://arkhamdb.com/card/04197
[prophecy]: https://arkhamdb.com/card/04032
[future]: https://arkhamdb.com/card/60403
[eyes]: https://arkhamdb.com/card/11067
[reddit-2020]: https://www.reddit.com/r/arkhamhorrorlcg/comments/jmv7wf/
[reddit-2023]: https://www.reddit.com/r/arkhamhorrorlcg/comments/12o60ht/
[reddit-2025]: https://www.reddit.com/r/arkhamhorrorlcg/comments/1i874p1/
[reddit-dec]: https://www.reddit.com/r/arkhamhorrorlcg/comments/1paycwo/lore_guy_seeking_rules_person_for_jacqueline_help/

## Implementation evidence

The runtime-v4 candidate implements these grouped choices. Eight public Session
tests in `tests/gathering_nested_groups.rs` exercise both Jacqueline declaration
orders, numerical and auto-fail groups, protected partial cancellation, Prophecy's
surviving-symbol eligibility (including a competing external symbol), and four
Eyes protected-group outcomes. A combined Eyes/Jacqueline/Olive case verifies
cancellation before Eyes chooses among surviving positions. Pending grouped decisions save/reopen and replay;
all physical tokens return through ordinary cleanup. The recorded focused check
`/tmp/arkham-89-implementation/eyes-cancellation-final/run.json` exited 0 with eight
tests passed and none filtered. These are the finite source-derived cases above,
not an exhaustive-composition or native-acceptance claim.
