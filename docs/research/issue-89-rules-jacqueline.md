# Jacqueline rules investigation for #89

2026-11-02; assessment baseline `c99951151863313ff3fe1416f05724e077be1628`.
This is source research within the owner-approved eight aggregate engineering
hours, not executed gameplay evidence. The existing [assessment](issue-89-gathering-assessment.md)
and [#89 current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
own scope and acceptance. These cases are proposed implementation expectations.

## Sources and effective text

The inspected starting response is deck `5389418`, SHA-256
`adcbbefc2fb29077690595d534e0367e28ff57f69c7d9c29ceb0397d11711f54`.
Replace its single `01000` placeholder with owner-selected Chronophobia `02039`;
retain its other slots. That yields 22 distinct slot identities plus investigator
`60401`: 23 identities for this audit. No Maya, Eldritch Inspiration or Grotesque
Statue is in this list. Olive and Katarina each occupy the sole ordinary Ally
slot; the cases below do not put both under Jacqueline's control together.

Primary rule sources are:

- [Rules Reference](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf)
  (RR), hash `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`:
  pp2–3 abilities and forced priority, p6 prohibitions, p9 partial resolution,
  p13 replacements, p17 simultaneous resolution, p26 skill-test sequence.
- [Legacy FAQ 2.5](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf)
  (FAQ), hash `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`:
  p6 Katarina erratum, p12 §2.5, p13 §§2.9–2.10, p17 nested reveal guidance,
  p19 Read the Signs, p21 Rod of Carnamagos, p22 Eyes/Katarina, pp26–27 taboo.
- [Forgotten Age investigator insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc72_forgotten_age_rules_insert-compressed.pdf),
  p1 Seal: 523,314 bytes, hash
  `03cafa631baa70080244ae4de93b24229ae7d05cf5311a989efd7a2e2fae9d69`.
- [Innsmouth investigator insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc81_rules_insert.pdf),
  p1 curse effects and custody, hash
  `3df58b1500071a72b1e8f4dd6127f3a181e244b6e1cf8f497c648c70a6238c25`.
- Original-English card faces from the unchanged external archive identified in
  the source contract. All 23 fronts and Jacqueline's investigator back were
  visually inspected, including icons and activation costs. Inspection establishes
  what those retained images say; it does not authenticate the archive's origin.

The [current Grimoire 1.1](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/07/Arkham-Grimoire-v1.1_web-1.pdf),
hash `a0ae9ebd0fc6f1455aedf95dacfd0f19a80ccb832c559e4aafbcfb437d0e9a44`,
is not substituted for the legacy baseline. Its pp2/41 direct pre-2026 content
to the legacy FAQ. All these selected cards predate 2026. Its p14 distinction
between cancelled and ignored tokens is recorded as a potential comparison,
not silently applied to legacy Prescient.

The retained taboo response has hash
`d63d719bd025040fd74281c123cc692ce521f305e78d5588e7c83eed31801673`.
The latest observed record is ID 10, dated 2026-02-19. Intersecting all 23 codes
with that record produces only Dark Prophecy and Voice of Ra. The matching
publisher taboo entries confirm those two changes. Katarina's change is mandatory
errata, not taboo. Printed levels are unchanged by these text changes.

| Identity | Effective-text disposition and material obligation |
| --- | --- |
| `60401` Jacqueline Fine | Printed text; location-bound optional reveal expansion, constrained cancellation, once per round, special elder-sign draw exception. |
| `60402` Arbiter of Fates | Printed text; exhaust during a legal use to exempt that use from the round limit. |
| `60403` Dark Future | Printed text; six specified symbol types cannot be cancelled/ignored by Jacqueline; cleanup is at her own turn's end. |
| `02039` Chronophobia | Printed text; one direct horror at own-turn end; two actions in one shared-location removal activation. |
| `01065` Ward of Protection (0) | Printed text; own non-weakness treachery revelation only, followed by its horror. |
| `03033` Uncage the Soul | Printed text; Spell/Ritual from hand, resource reduction; does not turn an unrelated card into a Spell. |
| `04032` Dark Prophecy | Taboo 10 changes eligible preferred tokens to non-elder-sign symbols; original fallback and five-for-one replacement remain. |
| `04109` Arcane Research | Printed text; two selected copies impose two initial mental trauma; their subsequent upgrade effect remains a later campaign boundary. |
| `04197` Olive McBride (0) | Printed text; exhaust, replace one reveal with three, resolve two and ignore the other subject to prohibitions. |
| `04199` Premonition | Printed text; seal on play; mandatory replacement at the next relevant reveal; discard after substitution. |
| `05032` Deny Existence (0) | Printed text; ignore one eligible aspect of an encounter/enemy effect, not arbitrary player-card costs or token restrictions. |
| `05112` Sign Magick (0) | Printed text; one restricted arcane slot; supplies no Ally slot or extra activation. |
| `06117` Read the Signs (0) | Printed text; add willpower to investigation, optional location-trigger suppression and extra clue. FAQ p19 excludes constant restrictions. |
| `06201` Spectral Razor | Printed text; optional pre-attack engagement, willpower addition, different Elite/non-Elite damage. FAQ §1.32 governs its engagement permission. |
| `07029` Sword Cane | Printed text; play avoids opportunity attacks; on-play activation waives the activation cost; ordinary activation exhausts it. |
| `11067` Eyes of the Dreamer | Printed text; optional 0–3 charges after initiation, extra reveals, one selected token and match-dependent additional clue. Zero charges do not forbid its basic activation. |
| `11069` Katarina Sojka | FAQ p6 replaces the printed reaction condition with a would-reveal condition for one token; retain the own-skill-test restriction and rest of ability. |
| `11097` Confidence | Printed text; at most one per test; explicit cancelled/ignored reveal counting and maximum three additional wild icons. |
| `60408` Clairvoyance (0) | Printed text; action and charge cost, willpower substitution, extra clue and specified-token horror. |
| `60411` Crystal Pendulum | Printed text; +1 willpower; prediction made after a local test begins, before its outcome is known. |
| `60412` Robes of Endless Night (0) | Printed text; exhaust for a Spell resource discount. Do not import the upgraded Robes' additional behavior. |
| `60416` Voice of Ra | Taboo 10 counts any symbol for its resource award, including elder sign and curse; still a non-test reveal. |
| `60419` Prescient | Printed text; named token class, optional Spell return after test end, with no success requirement. Ignored-only qualification is isolated below. |

“Printed text” means no card-specific change found after inspecting the relevant
FAQ errata and taboo entries for the complete named inventory. General rules and
FAQ clarifications still apply. It is not a claim that every possible interaction
has a bespoke published ruling.

## Worked expectations

Each example uses an ordinary reachable state from the selected party. Listed
randomness is a private deterministic test witness, not a player ability. Unless
stated otherwise, no Dissonant Voices, local The 13th Vision, other modifiers,
unresolved interruption or additional chaos manipulation is active.

### J1 — Dark Future does not disable extra reveals

Jacqueline reaches the end of her turn with Dark Future and an unused reaction.
For its five-token reveal, she uses her ability to reveal seven instead. Witness:
elder sign, skull, cultist, `0`, `−1`, `−2`, `−3`. She cancels the two chosen
numerical tokens, for example `0` and `−1`. Cancelling elder sign, skull or
cultist is illegal while Dark Future remains. The elder sign survives, so the
forced effect discards Dark Future. This is not a skill test: the symbols do not
apply scenario/test modifiers or Jacqueline's elder-sign skill-test effect.

An all-protected three-token reveal during an ordinary test is a separate adverse
case: auto-fail/skull/cultist cannot be cancelled by Jacqueline. Her extra reveals
do not waive the restriction; all three remain, including auto-failure. Do not
reject initiation merely because a later cancellation might be impossible.

Sources: inspected `60401/60403`, RR pp3/6/9/26. This is a source-derived
composition of the printed prohibition and partial-resolution rule.

### J2 — A nested Dark Prophecy preserves protected symbols

At Dark Future cleanup, Jacqueline predeclares her reaction and Dark Prophecy on
one of the resulting seven reveal positions. This yields eleven physical tokens:
five for the nested position and six ordinary positions. She pays Dark Prophecy's
one resource. The nested five are elder sign, skull, cultist, `0`, `−1`; the other
six are `0`, `−1`, `−1`, `−2`, `−2`, `−3`, all available in the Standard bag.

She chooses skull for Dark Prophecy. The two nested numbers are ignored; Dark
Future prevents ignoring the other two symbols. She then uses Jacqueline's
cancellation on two ordinary numerical positions. Seven tokens remain relevant:
the three protected symbols and four numbers. Elder sign therefore clears Dark
Future. Its prohibition persists until that final discard instruction, so the
player cannot cancel it early on the theory that the weakness is about to leave.

Reject declaring the nested replacement after inspecting earlier draws. Reject
Dark Prophecy while Dissonant Voices is in Jacqueline's threat area: that card's
asset/event prohibition is unconditional until it is discarded at round end.
Katarina cannot substitute for Dark Prophecy here because this cleanup is not
Jacqueline's skill test. No Confidence or Prescient commitment exists here.

Sources: inspected `04032/60401/60403/11069`; FAQ p17 nesting and p26 taboo;
RR pp6/9; original Dissonant Voices `01165`. The exact eleven-token result is an
inference from those sources, not a publisher-provided example.

### J3 — Protected symbols survive Eyes' outer selection

Jacqueline investigates the Cellar with Eyes, ready Katarina and Confidence
committed. There are no Ghouls at her location; Dark Future is in her threat area.
Spend one Eyes charge to obtain two reveal positions; predeclare Katarina on one.
Its sequence is `−1`, `−2`, skull; the ordinary position is elder sign. Katarina
ignores its numbers and retains skull. Eyes chooses skull, but Dark Future
prevents it from ignoring elder sign. Both symbols therefore resolve.

Four physical reveals give Confidence four wild icons total. Her skill is
`5 + 4 + 0 + 1 = 10` against shroud 4: skull contributes zero with no local
Ghouls, and her elder sign contributes +1. She discovers one clue: the chosen
skull does not match either ignored number. The elder sign is not cancelled or
ignored, so its draw exception does not fire. Dark Future remains because this
is not its own cleanup effect. Reject choosing a Katarina-ignored number as
Eyes' surviving outer token. The earlier assessment's ordinary Eyes/Katarina
matching case remains a separate positive example.

Sources: inspected `11067/11069/11097/60401/60403`, original Standard reference
`01104`; FAQ p6 erratum, p12 §2.5 and p22 explicit ignored-match ruling;
RR pp6/9/26. This composition is source-derived.

### J4 — Premonition seals without cancelling a protected token

Jacqueline has Dark Future, plays Premonition in a legal player window and reveals
elder sign. Seal it; this does not cancel or ignore it. Dark Future does not
discard merely because an elder sign appeared outside its cleanup ability.
The sealed token is absent from the bag.

Her next ordinary Clairvoyance investigation has one normal reveal position.
Premonition's Forced replacement supplies the sealed elder sign and is discarded.
The mandatory replacement resolves before an optional reaction to that same
original bag-reveal condition. She cannot wait to inspect the known substitute
and then invoke Jacqueline/Olive to replace the original reveal. With no other
modifiers, her test uses willpower 5 plus the elder sign's +1. Clairvoyance's
printed condition produces one horror and a successful test discovers two clues
if available. Dark Future remains. The special elder-sign draw does not fire.

The same-location condition on Jacqueline's investigator reaction does not give
ownership of the next Premonition consumption to her: a prior relevant token
reveal by another investigator can consume it first. Source-supported separate
case: using Olive while playing Premonition can seal two retained tokens; FAQ
§2.5 explicitly requires both on its eventual substitution.

Sources: inspected `04199/04197/60408/60401/60403`, TFA insert p1, RR pp3/13,
FAQ p12 §2.5 and p13 §2.10. The old ArkhamDB Premonition FAQ's reproduced
“as if” wording is stale: current §2.10 says other abilities also see the altered
state during the indicated resolution. The current PDF governs.

### J5 — Taboo symbols and curses outside a test

With a curse previously inserted by Patrice's Promise of Power, Jacqueline plays
Voice of Ra and reveals curse, elder sign and `−1`. Taboo 10 yields five
resources: the initial one plus two for each of the two symbols. There is no
extra curse draw or −2 modifier, because this is not a test. The curse returns
to the bag rather than being spent as a test-resolved curse. Dark Future, if
present, does not discard because its own cleanup was not being resolved.

During a test, Dark Prophecy's taboo preferred category includes curse and
excludes elder sign; a retained curse then invokes its normal −2/additional
reveal and eventual token-pool destination. Dark Future does not prohibit
cancelling or ignoring curse: it lists six specific symbols, not every symbol.
Do not expand its printed prohibition to match Voice of Ra's taboo category.

Sources: inspected `04032/60403/60416`, FAQ pp26–27 taboo, Innsmouth insert p1.

### J6 — Own-turn forced order has a material consequence

At Jacqueline's turn end, both Dark Future and Frozen in Fear are in her threat
area. Her reaction is unused. The lead investigator chooses their order under
RR p17. These are not end-of-round effects.

- **Dark Future first:** its five-token witness includes elder sign, so it leaves.
  Frozen in Fear then starts its willpower-3 test. Jacqueline reveals auto-fail,
  `0`, `−1` using her reaction and cancels auto-fail. Skill `5 − 1 = 4` succeeds,
  discarding Frozen in Fear.
- **Frozen in Fear first:** for the same three-token test witness, Dark Future
  still prohibits cancelling auto-fail. Cancelling the two numbers leaves
  auto-failure, so Frozen in Fear remains. A subsequent Dark Future cleanup can
  clear Dark Future, but does not revise that failed test.

Preserve this actual ordering choice and both outcomes. The investigator's
reaction use is consumed for the round unless a legal Arbiter trigger exempted
that use. Turn end itself does not reset the round limit. Chronophobia's
end-own-turn horror, if also present, joins this own-turn forced-effect family;
it does not move to the round boundary.

Sources: inspected `60401/60402/60403/02039`, original Frozen in Fear `01164`,
RR pp3/17/25–26 and FAQ §2.5. These are independent source-derived outcomes.

### J7 — Predicted margin and ordinary Prescient qualification

Jacqueline with Crystal Pendulum tests Rotting Remains; at the start she exhausts
Pendulum and names 3. With no commitments, willpower 6 and a resolved `0` give
success by 3 against difficulty 3, so its prediction draws one card. Naming 2
would not draw. The prediction is not changed after seeing the token. A local
tie failing under The 13th Vision still has margin zero; it is not failure by one.

For a separate test, commit Prescient, name symbol, and resolve a skull. At test
end she may recover one Spell from her discard regardless of success/failure.
She may decline. Arbiter, Sword Cane and Premonition are not Spell targets;
Clairvoyance/Voice of Ra are, if actually in the discard. Empty charges alone do
not discard a Spell asset. Confidence's special counting does not itself establish
Prescient eligibility.

Sources: inspected `60411/60419` and card traits; RR p26; previous assessed
The 13th Vision case. This positive Prescient example does not settle the
ignored-only qualification below.

## Localized interpretation still needing an explicit validity label

**Prescient when its only named-class token was ignored or cancelled:** the
best-supported legacy interpretation is **no Spell return**. FAQ §2.5's Recall
the Future example has a closely analogous named-token delayed condition and
describes qualification by resolved tokens. Confidence expressly includes
cancelled/ignored tokens; Eyes has its own explicit ignored-token matching text
and ruling. These support keeping ordinary delayed qualification separate from
raw physical-reveal counting.

The concrete Olive witness below resolves two tokens, so it falls within §2.5;
the all-but-one-cancelled/ignored exception does not exclude this witness.
The remaining qualification is that the FAQ does not name Prescient or expressly
state that its positive qualification condition is an if-and-only-if rule.
For cases with only one resolved token, §2.5's own scope remains a separate limit.
FAQ p21's Rod of Carnamagos example also shows that a token can count as revealed
during a test while being ignored for other parts of that test; “ignored” cannot
be erased indiscriminately across all purposes. No direct official Prescient
answer was found in the searched primary material. The Grimoire's newer wording
cannot settle this pre-2026 case by silent substitution.

The concrete proposed witness is Prescient naming symbol, Olive revealing skull,
`0`, `−1`, and ignoring skull while resolving the two numbers: recommended result
is no Spell return. Preserve that as the **best-supported legacy adjudication**,
not an explicitly published ruling for this exact witness. A later correction affects this eligibility
predicate and related delayed reveal checks; it should not require replacing
the entire timing model. Ordinary resolved-symbol qualification and Confidence's
explicit exception are already supported. This note does not ask the owner to
research or invent a ruling.

## Custody and implementation implications

Implementation follow-up: [selected token-composition findings](issue-89-token-compositions.md)
record the FAQ's group-preservation requirement and the unresolved scope of
Premonition on an existing multi-token request. The worked witnesses above do not
establish arbitrary nested Olive/Prophecy/Eyes/Jacqueline combinations. Those
compositions remain an explicit implementation boundary; a flat list of surviving
physical tokens is insufficient evidence for their outer choices.

Exact inspected path/variant/hash bindings are retained in
`local/issue-89-evidence/rules-investigation/jacqueline-image-bindings.json`, SHA-256
`fed902b207229b3f4b98a3c86b79e4598c0fb846d90a4534a9461f7f9f93ccde`.
The TFA insert is retained in the same directory under
`forgotten-age-investigator.pdf`; its hash above permits recovery from the
publisher URL. The [consolidated investigation](issue-89-rules-investigation.md)
records custody and recovery limits, including the absence of an independent backup.
No artwork bytes or original archive records were modified.

The selected examples require distinctions among physical reveal history,
replacement group membership, selected/ignored/cancelled tokens, protected token
classes, token custody, current effect and test identity, optional choices, and
turn/round limits. They do not prescribe a generalized rules interpreter or
new shared architecture. Save/resume and replay must preserve pending decisions
and already-applied consequences across these boundaries when implemented.

This audit corrects an over-broad grouping: Dark Future and Chronophobia belong
to own-turn end, while Act 2/Agenda 3/Dissonant Voices use the round boundary.
The full scenario's independent implementation and human-play evidence remain
unperformed.
