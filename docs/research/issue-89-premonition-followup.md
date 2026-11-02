# Premonition and an existing multiple-token request

2026-11-02. Follow-up research for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89), bounded to 20 active minutes. This challenges the whole-request interpretation and publisher-reply prerequisite in [the earlier composition note](issue-89-token-compositions.md). It changes no implementation or play-entry gate by itself.

## Recommended resolution

**Replace one pending reveal position with the complete group sealed on Premonition; retain the other requested positions.** If the original request has N positions and Premonition holds K tokens, the resulting batch contains K + N − 1 physical tokens before other declared modifiers. Premonition is consumed once. Its existing group is already selected: do not rerun the Olive choice that originally created it.

This is a source-derived interpretation, not a newly located publisher ruling for these exact pairs. It is sufficiently supported to construct the bounded implementation and independent witnesses below. A publisher reply is not a prerequisite merely because an exact example is absent. Confidence is moderate; the principal inference is replacement scope within a simultaneous batch. A contrary attributable ruling would require rechecking this conclusion.

The governing baseline remains the original selected card faces, Legacy FAQ 2.5 (February 2026), and taboo 10, as bound by [the source assessment](issue-89-rules-jacqueline.md). The 2026 Core Set reprint of Premonition is a different text and is not substituted here.

## Derivation and competing reading

The [Rules Reference](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf), pp2–3, 13 and 20, distinguishes condition eligibility, replacement scope, and timing. A/An makes a condition applicable when one or more qualifying elements are present; it does not say that a replacement absorbs every element of a containing instruction. Forced effects precede reactions to the same timing point. A replaced would-reveal occurrence cannot subsequently support reactions to that occurrence. The retained text was checked; the corresponding entries are also available in [ArkhamDB's rules transcription](https://arkhamdb.com/rules#Instead).

[Premonition 04199](https://arkhamdb.com/card/04199) replaces a token reveal with its sealed result. It does not cancel the enclosing card ability or expressly replace the entire number requested. Accordingly, the singular replacement applies to a position; the remaining positions still belong to the original instruction. This preserves both instructions to the extent they coexist.

The [official Legacy FAQ 2.5](https://images-cdn.fantasyflightgames.com/filer_public/c1/d0/c1d0fab6-7fa6-4ce2-af6a-16416381a19b/ahc_faq_v25_february_2026-web.pdf) supplies the following constraints:

- §2.5, p12: all tokens sealed by Olive are resolved when Premonition substitutes them. This expands its result, not the quantity of the request it replaces.
- Olive/Statue, p17: simultaneous revelation retains addressable positions and nested groups; simultaneous does not mean indivisible.
- Foresight, p21: a singular draw intervention can affect one card within a simultaneous multiple-card draw. This is corroborating analogy, not a token-specific ruling.
- §2.10, p13: other effects can observe the altered state during an as-if resolution. ArkhamDB's Premonition FAQ still reproduces the superseded 2020 wording, so that quotation must not suppress current reveal-dependent effects.

The strongest competing reading replaces the whole batch because the whole batch satisfies Premonition's condition, and because Premonition lacks the explicit quantity restriction printed on Olive. That conflates satisfying the condition with expanding its replacement target. It also treats simultaneity as aggregation, contrary to the position-preserving examples. The examples do not formally name Premonition plus Dark Future; nevertheless they favor one-position substitution. The lack of an exact example does not restore equal support to the competing reading.

[Favor of the Sun](https://arkhamdb.com/card/07272) provides a close textual comparison: its singular would-reveal condition substitutes a sealed token with nearly the same wording. The [August 2022 discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/wd89mt/olive_mcbride_favor_of_the_sun_blessing_of_isis/) treats it as replacing one of Olive's three positions. That is community corroboration, not publisher authority.

## Consequences and independent witnesses

Resolve Premonition's Forced replacement before offering optional reactions to its replaced position. Resolving its known group does not create another would-reveal opportunity for that group. With N greater than one, however, the N − 1 remaining bag positions still exist: offer the legal optional effects on those positions before any fresh random token is exposed. Do not suppress all reactions just because Premonition was consumed. When N is one, no original position remains, preserving the existing J4 restriction. This remaining-position timing is an inference from the replacement boundary and Forced priority, not a separate publisher example.

[Dark Future 60403](https://arkhamdb.com/card/60403) therefore supplies four fresh positions after substitution, and its elder-sign check considers the sealed group and those four tokens. Substitution itself neither cancels nor ignores a symbol. Outside a skill test, do not apply investigator elder-sign or scenario symbol effects merely because a token has that face.

[Voice of Ra 60416](https://arkhamdb.com/card/60416) supplies two fresh positions after substitution. Retain its initial resource and count every resulting qualifying symbol, including a substituted elder sign under taboo 10. The taboo text changes which symbols qualify; it does not limit the result to three physical tokens.

[Eyes of the Dreamer 11067](https://arkhamdb.com/card/11067) establishes its additional positions before revelation. Its charge expenditure remains spent. Preserve the sealed group as one selectable position: selecting it resolves all its members; selecting another position ignores the group. There is no second Olive selection inside the already-sealed group. Compare all surviving members against ignored tokens for Eyes' additional clue, awarding that bonus at most once. Applying the published group treatment to this particular pairing is a derivation.

The following are **expected outcomes, not executed evidence**. No optional modifier is used unless a row says otherwise. Use distinct physical token identities even when faces match. Eyes uses Jacqueline's unmodified willpower 5, original Study shroud 2, at least two available clues, and no active Dark Future.

| Witness | Inputs and decision | Expected consequence |
| --- | --- | --- |
| P1: mandatory five-token request | Dark Future; sealed `[-1,-2]`; fresh `[elder sign,0,+1,skull]` | Four fresh draws, six result tokens, Dark Future discarded. The alternative whole-request model incorrectly leaves the weakness in play. |
| P2: negative mandatory case | Same sealed group; fresh `[cultist,0,+1,skull]` | Four fresh draws; Dark Future remains. No skill-test symbol effects. |
| P3: taboo resource count | Voice of Ra; sealed `[skull,elder sign]`; fresh `[cultist,0]` | Four result tokens; gain 7 resources total. No investigator elder-sign card draw. |
| P4: Eyes chooses sealed group | Spend one charge; sealed `[-1,-2]`; fresh `[-1]`; choose sealed position | Resolve both sealed tokens, ignore fresh `-1`; final skill 2, success, 2 clues. |
| P5: Eyes chooses fresh position | Same inputs, choose fresh `-1` | Ignore both sealed tokens; final skill 4, success, 2 clues. A forged choice selecting only one sealed member is refused. |
| P6: zero-charge control | Spend no Eyes charge; sealed `[-1,-2]` | No fresh draw; resolve both sealed tokens; final skill 2, success, 1 clue, no ignored match. |
| P7: remaining-position opportunity | Voice of Ra with a sealed group, ready Olive and eligible Dark Prophecy/Jacqueline | Premonition is consumed first. Legal declarations can address the two remaining bag positions; none can reopen its replaced position. Passing those options gives P3's draw count. No payment/exhaustion occurs for an attempted declaration on the replaced position. |

For P1–P3, perform the real end-turn or event activation, not a direct token-result setter. For P4–P5, save and reopen at the actual outer selection; the two choices and physical identities must survive, and replay must reproduce the selected outcome. A 16-token bag with two tokens initially sealed has 14 bag tokens; after the fresh draws, bag counts are 10 for P1/P2, 12 for P3, and 13 for P4/P5, returning to 16 on cleanup. The substituted physical tokens must not be redrawn in their own simultaneous batch. Costs, Premonition disposal and weakness disposal occur once.

P7 does not settle the independently investigated mixed-group cancellation and protected-ignore predicates. It prevents this change from accidentally deleting existing legal opportunities; those other obligations retain their own source-derived witnesses.

## Secondary evidence and limits

The pinned [reference Premonition implementation](https://github.com/halogenandtoast/ArkhamHorror/blob/a6f3b870372ce239bfec3c1153b7ccbebde49d63/backend/arkham-api/library/Arkham/Event/Events/Premonition.hs) uses `ReplaceCurrentDraw` with the sealed group. Its [ChaosBag implementation](https://github.com/halogenandtoast/ArkhamHorror/blob/a6f3b870372ce239bfec3c1153b7ccbebde49d63/backend/arkham-api/library/Arkham/ChaosBag.hs#L950) represents an N-token request as N child draws and replaces a child. This corroborates the count above. It also has separate plural and singular windows, so it is not adopted as proof of Forced-before-reaction ordering. No Premonition-specific test file was found in that revision's complete tree.

[Upstream issue #371](https://github.com/halogenandtoast/ArkhamHorror/issues/371), opened 2024-08-21 and closed 2024-08-30 without a ruling comment, reports two sealed tokens followed by six tokens for Dark Future. Its complaint concerns duplicated physical elder-sign and auto-fail tokens. It is useful evidence for the custody witness, not official confirmation of either count or timing.

The [September 2020 Henry Wan discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/im1oil/) assumes two additional bag tokens for Voice of Ra, but its proposed Double, Double setup is corrected by respondents. It supplies a lead, not an independent oracle. The [December 2020 Olive/Premonition discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/k5l9vp/olive_mcbride_and_premonition/) correctly points toward Forced priority but also contains a disputed claim that Olive may exhaust without an effect. The [March 2023 as-if discussion](https://www.reddit.com/r/arkhamhorrorlcg/comments/11zf0dc/) disagrees about reveal timing and provides no attributable designer response. These threads do not constitute consensus or new official rulings.

Targeted searches covered Reddit, ArkhamDB, BoardGameGeek, Hall of Arkham and Mythos Busters references, followed by the pinned implementation and its issues. No exact publisher response was located. No external inquiry was sent. This is a bounded negative search result, not a claim that no such response exists.

The earlier publisher-question draft remains historical research. For this first blocker, the recommendation is now to implement and verify the one-position interpretation with the stated derivation. Do not wait for outreach, label the result an exact publisher ruling, or infer ordinary/native play acceptance from these witnesses.

Consulted: foundation mission, play and source contracts; `AUTH-2`, `AUTH-3`, `EVID-1`–`EVID-10`, `REP-1`–`REP-3`, `AGENT-5`, and `OPS-1`–`OPS-7`. In particular, `EVID-4` permits independently source-derived expectations, while `AGENT-5` and `OPS-2` require investigating derivability before creating a blocker.

## Local implementation evidence, 2026-11-02

The parent accepted this interpretation for the bounded local implementation. Additional activation and representation readings were `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `SEM-1`–`SEM-2`, and `GOV-1`–`GOV-3`; the Rust implementation-language contract was applied. No publisher response was made a prerequisite.

The executable expectations in [gathering_premonition.rs](../../tests/gathering_premonition.rs) now exercise P1–P6 through real card play or the mandatory turn-end effect. They check physical batches, bag custody, exact resource/clue outcomes, single disposal, saved pending decisions and replay. P7 has a bounded witness for offered fresh-position Jacqueline/Dark Prophecy and rejection of a forged declaration on the replaced position. It does not claim to execute every combination of optional modifiers. The zero-charge Eyes case resolves the entire sealed group.

[gathering_token_containment.rs](../../tests/gathering_token_containment.rs) plays fast Premonition, pays Voice's action while engaged with the Ghoul Priest, saves/reopens at the actual opportunity attack, resumes it, and checks the exact `[elder sign, skull, 0]` batch and final resource total 10 from starting resources 5. The recorded focused command was `cargo test --test gathering_premonition --test gathering_token_containment --test gathering_tokens`: 18 passed, none failed or filtered, exit 0. Full output and the atomic completion record were retained under `/tmp/arkham-89-implementation/premonition-focused-deterministic/`. Existing single-position Forced-priority and Olive/Premonition drawback controls still pass. This is local behavioral evidence, not native or owner acceptance.
