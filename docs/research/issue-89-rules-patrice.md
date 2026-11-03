# Patrice: cancellation, failure and retry expectations

Research for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89),
2026-11-02, against assessment commit
`c99951151863313ff3fe1416f05724e077be1628`. This is a source-derived research
proposal for the approved eight-hour investigation, not an executed game test or
an implementation activation. The issue's current handoff owns acceptance and
the aggregate allowance.

The selected Patrice list is ArkhamDB `5391062`, revision `0.7`, retained response
SHA-256 `c037ba26f8eef2fcdce2e2e39777f2f466e68df8bcb48bb09a63cbd2ffdd8c16`.
Replace its one `01000` placeholder with Joel's `05041` selection. This gives 25
distinct deck-card identities plus Patrice herself: 26 identities examined below.
No upgraded list or historical campaign state is imported.

## Findings and controlling sources

**The named cancellation and failure-order questions have supported local
answers.** A Test of Will interrupts the pending encounter revelation with its
own test. It does not put two skill tests into simultaneous execution. Live and
Learn begins after the original failure and its consequences; it does not undo
damage, recover commitments or replay an action's costs. An enemy's changed
position after the first attempt remains changed on the retry.

Sources are the inspected English card faces, [Rules Reference](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf)
(RR), [Legacy FAQ 2.5](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf)
(FAQ), the [Innsmouth investigator insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc81_rules_insert.pdf),
and the [Hemlock Vale investigator insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc76_rules_insert_web-compressed.pdf)
(HV). Existing RR/FAQ/Innsmouth identities are in
[the source manifest](issue-1-evidence.json). HV's four-page PDF is 2,229,575 bytes,
SHA-256 `ee8bc907db75241c2ab9d5d043ef08545b15e9eb1f5a5e397e267c6aa7f82b19`;
p3 supplies the previously missing explicit replenish definition needed by
Sparrow Mask. The current [Grimoire 1.1](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/07/Arkham-Grimoire-v1.1_web-1.pdf)
p2 limits its coverage to 2026-and-later products and redirects older products
to the legacy FAQ. Its newer reprints do not silently replace this deck's cards.

The important timing distinctions are:

- RR pp3,6,11,26 and FAQ §§1.4/1.7 distinguish cancellation, nested effects,
  outcome determination, result application and the end of a test.
- FAQ §1.17, p10 defers a second skill test until the first—and its containing
  action when applicable—finishes. A pending encounter revelation is not itself
  an already-running skill test.
- RR pp8,18,26 and FAQ p19 place failed-test consequences, including retaliation,
  before Live and Learn's stated play window. Delayed effects initiate with
  forced timing; an optional choice inside one remains optional.
- FAQ p2's Hidden correction and p17's Patrice answer preserve Watcher in hand
  and distinguish simultaneous card drawing from subsequent revelations.

These are rule derivations from the named cards and rules, not assertions that
the publisher printed these exact combined examples. No secondary engine output
or user-review ruling supplies their expected answers.

## Effective-text audit of the complete Patrice inventory

All 26 English faces and Patrice's reverse were visually read. No selected card
has a named-card erratum in FAQ 2.5, and none of these codes intersects the
retained taboo-10 mutation/chaining/forbidden entries. Thus each row retains its
printed card text, subject to the governing general rules and keyword
clarifications. The raw deck's taboo-9 selection is historical; this audit uses
the assessment's fresh-preparation taboo-10 proposal.

| Code | Card | Material interpretation retained |
| --- | --- | --- |
| `06005` | Patrice Hathaway | Upkeep replaces the draw; five is the refill target and normal modified hand limit. Elder sign creates an optional shuffle after the test. |
| `01065` | Ward of Protection (0) | Only its player's non-weakness treachery draw; cancellation is followed by the card's own horror. |
| `01079` | “Look what I found!” (0) | Failure-by-at-most-two window during an investigation; clues do not turn the failed test into a success. |
| `01080` | Lucky! (0) | Skill increase at the would-fail window; no automatic-success override. |
| `01093` | Unexpected Courage | Two wild icons; one copy maximum committed to a test. |
| `02033` | Peter Sylvestre (0) | Agility modifier and optional horror healing from Peter after his controller's turn. |
| `02192` | Rise to the Occasion (0) | Own test only; eligibility uses base skill, not the final modified result. |
| `03033` | Uncage the Soul (0) | Plays a Spell/Ritual from hand with the printed resource reduction; does not erase other restrictions. |
| `03039` | Resourceful | Successful-test retrieval from its owner's discard; Survivor only, excluding every Resourceful by title. A currently committed card is still in limbo. |
| `04029` | Mists of R'lyeh (0) | Willpower evasion, charge cost, optional movement on success, and the printed five-symbol discard condition; no taboo broadening. |
| `04200` | Live and Learn | A retry of the failed test after consequences; new commitments and randomness, not restored old commitments or erased consequences. |
| `04201` | Take Heart | Its reward belongs to the testing investigator and resolves as a failed-test result; one commitment maximum. |
| `04272` | Winging It | Discard play is permitted; its post-resolution shuffle also occurs on failure. |
| `05032` | Deny Existence (0) | Ignore one listed aspect from a qualifying encounter effect/enemy attack; other aspects remain. It does not ignore the horror caused by Ward of Protection itself. |
| `05041` | The 13th Vision | Local investigators fail ordinary ties; their skill and failure margin are not modified. Removal is a single two-action activation. |
| `06016` | Patrice's Violin | Exhaustion and a legal hand discard are costs; the chosen same-location investigator receives the draw or resource. |
| `06017` | Watcher from Another Dimension | Apply the corrected Hidden rule; hand fight/evade and deck-empty attack remain separate conditions. After a failed hand attempt it is an in-play enemy. |
| `06164` | Ethereal Form | Adds willpower to the evade test; successful evasion and its round-duration engagement/attack restrictions are distinct effects. |
| `06165` | Scrounge for Supplies | Level-zero retrieval does not include signatures or weaknesses with no level. |
| `06201` | Spectral Razor | Add willpower to combat; optional engagement before attacking, with the Elite-dependent damage bonus. FAQ §1.32 addresses satisfying an attack's restriction through this engagement. |
| `07032` | Promise of Power | Four wild icons; its commitment adds a curse or inflicts the stated horror if addition cannot happen. The curse can affect this same test. |
| `09102` | Idol of Xanatos | Exhaustion and discarded legal hand cards pay for a chosen amount of incoming damage/horror cancellation. Hidden Watcher is not payment. |
| `10111` | Sparrow Mask | One Mask per investigator; spend an offering for one allowed skill boost per test. Replenish stops at two offerings (HV p3). |
| `60407` | Azure Flame (0) | Willpower attack, charge, extra damage and its printed elder-sign/positive/zero-token drawback. |
| `60507` | Old Keyring (0) | Shroud reduction; a key is removed on success rather than paid as a cost, and zero keys discard it. The taboo entry for Old Keyring (3), `09117`, does not alter this card. |
| `60513` | A Test of Will (0) | One-resource fast event with a willpower(3) test; only success cancels the specified revelation. This is neither upgraded version and does not exile itself. |

The archive remains read-only. Exact inspected face paths, variants and full
hashes are retained in ignored
`local/issue-89-evidence/rules-investigation/patrice-image-bindings.json`
(SHA-256 `061109c1d22c6e41b34e64a257239689d112a96a84b24d6d031a24ebbacaa3e6`);
all 26 face hashes and the investigator-back hash matched. The new HV PDF is
retained beside it as `hemlock-investigator.pdf`. This verifies identity and the inspected text,
not original artwork provenance. In particular, the retained Spectral Razor
face bears a 2024 copyright although the selected card identity predates that
printing; no claim that every scan is a first printing is made.

## Worked expectations

All examples use ordinary legal content from the selected party and original
Gathering. Named token outcomes and deck tops specify private test fixtures,
not player powers. Unless a case names them, other modifiers and reactions are
absent and Patrice has sufficient remaining health/sanity to survive.

### P1. A pending revelation, a cancellation test and a newly drawn weakness

During a mythos phase Patrice draws Ancient Evils. She has three resources,
ready Violin, A Test of Will, Take Heart, Lucky! and an ordinary discardable
card. Vision is on top of her sufficiently large deck.

1. She pays one for A Test of Will before Ancient Evils resolves. Keep that
   exact encounter card and its drawing investigator pending. Begin her
   willpower(3) test; the agenda has not yet received Ancient Evils' doom.
2. In a test player window she exhausts Violin, discards the ordinary card and
   chooses herself to draw. Resolve Vision into her threat area. A Test of Will
   cannot cancel this weakness. Vision now affects the still-running test.
3. She commits Take Heart and resolves `−1`: willpower `4−1=3` versus 3. Vision
   makes this a failure by zero unless she changes the result at the proper
   window.
4. If she spends one on Lucky!, skill becomes 5. The cancellation succeeds,
   Ancient Evils adds no doom, and Take Heart grants nothing. She has one
   resource left; Vision remains in play and Violin remains exhausted.
5. If she declines Lucky!, Take Heart supplies two cards and two resources in
   the failed-test result step. Use two ordinary cards for this branch's fixture.
   She now has four resources. After the cancellation attempt finishes,
   Ancient Evils remains uncancelled and resolves normally, unless another
   legal cancellation is used while its original draw interruption is still
   pending. The failure of A Test of Will itself adds no doom or horror.

This is a nested **effect** sequence. If the encountered card were Rotting
Remains instead, its own willpower test would start only after the failed
cancellation attempt returned to that revelation; it does not run in parallel
with A Test of Will. Do not reuse the cancellation test's token, commitments or
result as Rotting Remains' test.

Refuse a cancellation played after the revelation has already begun, targeting
Vision/Watcher as non-weakness treacheries, remote protection by A Test of Will,
or committing Rise to the Occasion to this difficulty-3/base-4 test. Sources:
inspected `60513/06016/05041/01080/04201/02192`; RR pp3,6,11,26; FAQ §§1.4/1.7/1.17.

### P2. Failure, retaliation, elder-sign recovery and retry

After legal Act 2 advancement, Patrice is engaged with ready Ghoul Priest
(for example, she previously used Engage to take it from Tony).
Vision is absent. She makes a **basic fight** using combat 2, commits Take Heart
and reveals her elder sign. She has Live and Learn in hand and at least two
cards in her discard pile; the two Take Heart draws are ordinary cards.

The modifier gives `2+1=3` against Priest's fight 4: failure by one. Take Heart
grants two cards and two resources. After the test's results, Priest retaliates
for two damage and two horror. It remains ready. Complete these consequences
and ST.8's commitment/token cleanup before offering Live and Learn.

Patrice's elder sign created a delayed after-test effect. Its shuffle decision
is reached with forced timing before the optional after-test event: she may
shuffle all but one card from her discard into her deck, or decline. The
committed Take Heart has reached the discard at this point and can be included.
Live and Learn is still in her hand and cannot be included in that shuffle.

She can then play Live and Learn. The retry has combat `2+2=4`; with no new
commitments and a `0` token it succeeds against 4, dealing one basic-fight
damage. No second action is charged. The previous damage/horror, card draws and
resources remain. Take Heart was not committed to this second test and pays
no second reward. If the first retaliation defeated Patrice, she cannot
continue to a retry. If Priest had been exhausted, the original failure would
not have caused retaliation.

Sources: inspected `06005/04200/04201`, original `01116`; RR pp3,8,18,26;
FAQ p19's explicit failed-test/retaliate timing. The delayed-effect priority is
a derivation from RR pp3,8 and Live and Learn's after-test play condition.

### P3. A reward cannot finance an earlier window

Patrice investigates a clue-bearing Cellar with Old Keyring. Vision is absent;
Keyring has two keys. Intellect 2 against shroud `4−2=2`, with a `−2` token,
fails by two. Take Heart is committed, “Look what I found!” is in hand, and she
starts with zero resources.

She cannot pay for “Look what I found!” at its failure window. Take Heart's two
resources arrive in the subsequent result step, after that window. Gaining
them does not reopen it. The failed investigation discovers no clue, and
Keyring retains both keys.

Control branch: start with two resources and at least two clues at Cellar.
She may pay for “Look what I found!” at the failure window and discover two
clues; Take Heart then grants its reward. The test still failed, so no key is
removed. With Lucky! also in hand, she can instead pay one at the earlier
would-fail window: `0+2=2` succeeds without Vision, producing the normal
investigation clue and spending a key, with neither failure reward available.

Sources: inspected `01079/01080/04201/60507`; RR p26 and FAQ §1.7. This case
separates would-fail modification, an after-failure event and test-result text.

### P4. Repeating a charged test does not pay the activation twice

Patrice has Azure Flame with exactly one charge and attacks ready Ghoul Priest
with willpower 4. Vision is absent. A `−1` produces failure against fight 4.
After the two-damage/two-horror retaliation and completion of the first test,
she plays Live and Learn. No additional action or charge is spent: the event
instructs a test retry, not a second asset activation. Azure Flame remains in
play with zero charges.

The retried test retains the originating attack's willpower/damage rules and
adds Live and Learn's +2. With `−1`, `4+2−1=5` succeeds and deals two damage.
An unrelated commitment or a boost explicitly limited to the first test does
not carry over. If the second token is `0`, Azure Flame's one-damage drawback
occurs on that second reveal and the successful attack still deals two damage.
There is no repayment of the first activation, but neither is the new reveal
immune to the originating test's consequences.

This is a local interpretation of “that test” from the printed event, the
originating attack and RR's costs/lasting-effect/test boundaries. It preserves
the same test operation and target while rebuilding test-scoped commitments,
boosts and random outcomes. Sources: inspected `60407/04200`, original `01116`;
RR pp7,14,18,26. It must be exercised by the intended Rust consumer rather than
inferred from a secondary implementation's retry method.

### P5. Watcher's failed hand evasion changes the retry's world

Patrice has Watcher in hand and Mists of R'lyeh with one charge. Vision is
absent. She spends an action and the charge to evade Watcher using willpower 4
against evade 5. A `0` fails; Watcher spawns engaged with her. It has no native
alert keyword, so this failure does not itself cause an alert attack.

After the first test/action finishes, Live and Learn retries against the same
enemy, now in play and engaged. A new `0` gives `4+2=6`, a success. Exhaust and
disengage Watcher normally, and offer Mists' optional connecting-location
movement. Do not discard Watcher by replaying the earlier hand-only result: it
is no longer in the hand from which that instruction could discard it. One
charge and one action were spent across the two attempts. Its two health remain
irrelevant to an evasion; no damage was dealt.

Refuse using Violin or Idol to discard hidden Watcher as a cost, and refuse
ordinary non-card-authorized attacks by another investigator against a card
still in Patrice's hand. Once it has spawned, ordinary enemy targeting applies.
Sources: inspected `06017/04029/04200`; RR pp11,14,26; FAQ p2 Hidden correction
and §2.10's “as if” rule. The [publisher's Patrice introduction](https://www.fantasyflightgames.com/en/news/2019/9/5/more-than-meets-the-eye/)
also confirms the hand-attempt/success-discard/failure-spawn distinction.

### P6. Replenishment, cancellation and another test's limit

Use P4's Azure Flame attack with Sparrow Mask also in play and holding one
offering. Patrice spends it for +2, leaving zero; a `−4` gives `4+2−4=2`, failing
against Priest's fight 4. Its retaliation deals both damage and horror as one
attack, with at least one point placed on her investigator. The mask replenishes
one offering, not one for damage plus another for horror. A later qualifying
instance can restore it to two; further replenishment cannot exceed two.
If all of that attack's damage/horror is cancelled before she takes it, that
instance supplies no replenishment.

On a Live and Learn retry the old +2 mask boost has expired, and she may spend
the replenished offering again because this is a new test. It still cannot be
used twice in one test. Sources: inspected `10111/04200/09102`, RR pp7,14,25
(an attack's damage and horror are simultaneous), and HV p3. Distinguish
“dealt” cancellation windows from “after you take” conditions.

## Consumer obligations and limits

The implementation brief should retain the original encounter draw while a
cancellation test resolves; distinguish outcome and result windows; preserve
committed-card limbo until cleanup; retain the originating test operation and
target through retries; and reevaluate the real card positions and legal choices
at each continuation. Saving at the Lucky!, elder-sign shuffle, allocation or
retry decision must resume that exact pending step without repeating committed
draws, costs, attacks or revelation effects.

These are behavioral requirements for the existing observation/choice/persistence
seam. They do not prescribe a generalized stack machine or card interpreter.
FAQ §1.17 is an explicit constraint against concurrent active skill tests, not a
reason to invent an unsupported real-game example.

No unresolved governing contradiction was found for the described Patrice
families. This finding is bounded to this inventory and these source-derived
cases; it does not prove every future scenario interaction or the correctness
of unimplemented code. The Root investigation integrates this note with the
other investigators, full scenario sources and independent review before any
broader readiness conclusion.
