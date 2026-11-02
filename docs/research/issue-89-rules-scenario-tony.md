# Gathering transitions and Tony: source adjudication

Research for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89),
2026-11-02, against assessment commit
`c99951151863313ff3fe1416f05724e077be1628`. This is one part of the approved
eight-hour aggregate rules investigation. These are source-derived expectations,
not results from game execution or an implementation-readiness certificate.

## Conclusions

The apparent Act 2 / Agenda 3 conflict is resolved by their actual wording and
FAQ §2.2: Act 2's **when**-round-ends opportunity precedes Agenda 3's **at**-end-of-round
doom. The newly spawned Ghoul Priest therefore counts for that doom. Reaching
the threshold through this Agenda 3 effect does **not** immediately advance the
agenda; the normal next mythos check does. Ancient Evils is different because
its text expressly allows advancement.

Dark Future is an end-of-own-turn effect, not part of this round-end pair.
Dissonant Voices is a simultaneous round-end forced effect. Upkeep readying,
action reset, round-limited abilities and delayed-effect expiration must retain
their distinct timing points.

All 23 Tony identities, including his investigator, signatures and Internal
Injury, and all 26 original scenario identities were checked against original
English face images. Relevant reverses were also inspected. No named erratum
changes these printed cards. The only selected Tony taboo mutation is Quick
Thinking's once-per-round maximum. General rules corrections still apply.

One additional edge deserves an explicit source-policy disposition: Agenda 2b
can exhaust the encounter deck without finding a Ghoul. The recommended finite
result is to finish that failed discard-until effect before reshuffling; see the
final worked case and its evidence qualification below.

## Controlling sources and inspection boundary

- **RR:** [original English AHC60 Rules Reference](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf),
  SHA-256 `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`.
  Relevant locators: abilities and advancement pp2–4; campaign and defeat pp5–6;
  effects/doom pp9–10; engagement p10; health/damage p12; limits/lasting effects
  p14; simultaneous forced effects p17; spawn p19; victory p21; scenario endings
  p22; phase sequence pp23–25.
- **FAQ:** [Legacy FAQ 2.5, February 2026](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf),
  SHA-256 `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`.
  Original English/errata precedence and elimination correction p2; nested
  sequences §1.4 p8; timing of “at”/“if” §2.2 p12; Quick Thinking mutation p26.
- **CG:** [original Night of the Zealot campaign guide](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/night_of_the_zealot_campaign_guide.pdf),
  SHA-256 `132bb78a2c70245d2aa8037bab39784f42431b01c02ffa9d5b44d2edc10e13fb`,
  Gathering setup and all four ending routes pp2–3.
- **Cards:** original identities below; original numbered English images from
  the unchanged external archive. The existing encounter-image binding records
  exact variants and hashes. The fresh audit checked all 26 fronts and eleven
  relevant reverses: six act/agenda reverses and five location reverses. It also
  inspected Tony's 23 fronts and investigator reverse. The publisher's five
  Chapter One encounter PDFs independently support the shared encounter sets;
  the prior assessment records those PDFs and their hashes.
- **Taboo:** retained ArkhamDB response ID 10, effective 2026-02-19, hash
  `d63d719bd025040fd74281c123cc692ce521f305e78d5588e7c83eed31801673`, matched to
  the official FAQ's mutation. This is the selected overlay; no mixed-version
  selection is used.
- **Grimoire:** [current Grimoire 1.1](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/07/Arkham-Grimoire-v1.1_web-1.pdf),
  SHA-256 `a0ae9ebd0fc6f1455aedf95dacfd0f19a80ccb832c559e4aafbcfb437d0e9a44`.
  Its pp2/41 scope and delegation do not replace original Chapter One cards
  wholesale. Its encounter-deck clarification on p10 is separately identified
  below; Chapter Two replacement encounters and revised core cards are excluded.

The audit compared printed action/reaction icons and both sides where they bear
rules; text extraction alone was insufficient. These are inspected English
faces for the original card identities, with the archive-origin limitation already recorded
in the assessment. Hash agreement establishes inspected bytes, not independent
publisher provenance or first-printing scan status. The
[consolidated custody record](issue-89-rules-investigation.md) binds the retained
image metadata and checksums. No image payload, personal record or archive file changed.

## Effective-card dispositions

“Original” means no card-specific erratum or selected taboo mutation was found;
it does not exempt the card from the general corrected rules. The behavior notes
identify interpretation obligations rather than reproduce complete card text.
Printed costs, icons, traits, values and backs remain part of each bound card.

| Tony identity | Card | Disposition and material obligation |
| --- | --- | --- |
| `01049` | Hard Knocks | Original; separate resource-paid combat/agility boosts within the relevant test. |
| `01089` | Guts | Original; commit maximum and successful-test draw. Do not substitute revised-core Guts. |
| `02038` | Internal Injury | Original; own-turn direct damage; one activation costs two actions. |
| `02225` | “If it bleeds…” | Original; after Tony defeats a Monster, same-location horror healing; terminal timing matters. |
| `02229` | Quick Thinking | **Taboo 10 mutation:** ability maximum once per round, across copies; after-test action remains optional. |
| `03233` | “Watch this!” | Original; own-test commitment, stake paid as cost and success-margin reward. |
| `03263` | On the Hunt | Original; mythos draw replacement, bounded enemy search, spawn override, explicit no-match fallback. |
| `04037` | Backpack | Original; non-weakness Item/Supply search; attached-card play permission and empty-pack discard. |
| `04103` | Scene of the Crime | Original; first-action restriction, enemy-dependent clue count, explicit opportunity-attack exemption. |
| `04107` | Lucky Cigarette Case | Original; margin threshold and exhaust cost; do not reset by resetting action counts. |
| `04150` | Take the Initiative | Original; own test; counts completed actions by every investigator in the current phase. |
| `05111` | Intel Report | Original; independently optional extra-cost changes to clue number and range. |
| `05117` | Tennessee Sour Mash | Original; treachery-test willpower boost versus separate discard-to-fight action. |
| `05277` | Small Favor | Original; non-Elite target; independently optional damage/range surcharges. |
| `06003` | Tony Morgan | Original; restricted additional action during his turn; Elder Sign replenishes Contracts. Guardian secondary class remains bound. |
| `06010` | Bounty Contracts | Original; six initial bounties; optional placement after any enemy enters; mandatory payout after Tony defeats a marked enemy. |
| `06011` | Tony's .38 Long Colt | Original; paired play, ammo cost, bounty-dependent combat, additional damage and conditional replenishment. |
| `06012` | Tony's Quarry | Original; farthest-location spawn, Aloof, entry doom followed by a bounty; not a Ghoul. |
| `06162` | Gregory Gry | Original; wager from the ally's tokens at test initiation; required success margin. |
| `09029` | One in the Chamber | Original; last-ammo timing; next attack with that firearm in this round only. |
| `09068` | Kicking the Hornet's Nest | Original; non-Elite search/spawn; dependent clue/resource reward; no-match must not invent an enemy. |
| `60114` | “Get over here!” | Original; non-Elite locality, movement/engagement and attack; compound designated actions remain one action cost. |
| `60306` | Mauser C96 | Original; exhaust/ammo costs; succeed-by-two choice between readying and resource. |

| Scenario identity | Card | Disposition and material obligation |
| --- | --- | --- |
| `01104` | The Gathering reference | Original Easy/Standard face; location-specific Ghoul count and conditional symbol consequences. |
| `01105` | What's Going On?! | Original 3-doom threshold; reverse retains the lead's group-discard versus horror choice. |
| `01106` | Rise of the Ghouls | Original 7-doom threshold; reverse shuffles discard, discards until Ghoul, then lead draws it. |
| `01107` | They're Getting Out! | Original 10-doom threshold; two distinct forced boundaries and act-dependent reverse. |
| `01108` | Trapped | Original 2-clue-per-starter payment; reverse relocation, enemy discard and Study removal. |
| `01109` | The Barrier | Original; optional Hallway-only 3-clue-per-starter payment **when** round ends; reverse reveals Parlor and introduces Lita/Priest. |
| `01110` | What Have You Done? | Original; **if** Priest defeated, advance; lead chooses R1/R2 on reverse. |
| `01111` | Study | Original both sides; six clues for three starters, shroud two, no outgoing connections before advancement. |
| `01112` | Hallway | Original both sides; connections to three rooms, no clues. |
| `01113` | Attic | Original both sides; entry horror, six clues, shroud one, Victory 1. |
| `01114` | Cellar | Original both sides; entry damage, six clues, shroud four, Victory 1. |
| `01115` | Parlor | Original both sides; unrevealed movement prohibition; revealed resign and Lita parley. Original Resign colon is present. |
| `01116` | Ghoul Priest | Original; fifteen health for three starters; Hunter, Retaliate, highest-combat prey, Victory 2. |
| `01117` | Lita Chantler | Original; controlled-only same-location bonuses; taking control differs from campaign acquisition. |
| `01118` | Flesh-Eater | Original; Attic spawn, Victory 1. |
| `01119` | Icy Ghoul | Original; Cellar spawn, Victory 1. |
| `01159` | Swarm of Rats | Original; Hunter; Creature rather than Monster/Ghoul. |
| `01160` | Ghoul Minion | Original; Ghoul/Monster traits, ordinary engagement and damage. |
| `01161` | Ravenous Ghoul | Original; lowest-remaining-health prey. |
| `01162` | Grasping Hands | Original; agility test and failure-margin damage. |
| `01163` | Rotting Remains | Original; willpower test and failure-margin horror. |
| `01164` | Frozen in Fear | Original; first qualifying action each round surcharge; own-turn-end willpower test. |
| `01165` | Dissonant Voices | Original; unconditional asset/event prohibition while in play; at-round-end discard. |
| `01166` | Ancient Evils | Original; doom placement with explicit permission to advance now. |
| `01167` | Crypt Chill | Original; willpower test, legal asset-loss choice or damage fallback. |
| `01168` | Obscuring Fog | Original; location attachment limit, shroud modifier and successful-investigation discard. |

## Worked expectations

These states use the actual party and original scenario. They are private
verification setups for a future implementation; they do not grant the player
control over draws, tokens or enemy placement in normal play.

### S1. Late Act 2 advancement changes the later agenda outcome

Start at the end of a round with Act 2, Agenda 3 on eight doom and no other doom.
All three investigators are in Hallway and can contribute nine clues. One Ghoul
Minion is there; the Ghoul Priest is still set aside. Assume Tony's combat is
highest and no competing effect changes these facts.

1. Act 2's **when** opportunity occurs first. The players may pay nine Hallway
   clues or decline. Paying is not forced by possessing the clues.
2. If paid, advance Act 2: reveal Parlor, put Lita there and spawn Priest in
   Hallway. Its prey selects Tony for engagement. Bounty Contracts may respond
   to that entry, even though this is outside an ordinary action window.
3. Agenda 3's later **at** effect counts both Hallway Ghouls and places two doom:
   eight becomes ten. Engagement does not remove an enemy from its location.
4. This does not flip the agenda. Finish other round-end effects, then start
   the next round. Mythos 1.2 adds one doom; 1.3 sees eleven and advances.
5. The current act is now 3, so Agenda 3b defeats each non-resigned investigator
   with one physical trauma. Once all are eliminated, resolve the guide's
   no-resolution route; this is not R3 and does not itself kill them.

Counterfactual: decline Act 2. Agenda 3 adds only one doom, reaching nine. Next
mythos adds one and advances at ten while Act 2 is still current, invoking R3.
That route kills non-resigned investigators. The implementation must preserve
this genuine choice and these different consequences.

Reject payment at turn end, payment during the preceding player window, use of
clues held outside Hallway, and Agenda 3 doom before the Act 2 opportunity.
Sources: original `01109/01107/01116/06010`; FAQ §2.2; RR pp3,9–10,24; CG pp2–3.

### S2. Specific Ghoul movement is not Hunter movement or another attack

Start Agenda 3's enemy-phase-end boundary with an exhausted, unengaged Ghoul
Minion in Attic and Tony in Hallway. Its ordinary enemy-phase attack opportunity
has passed. Agenda 3 moves that Ghoul one location toward Parlor, into Hallway,
despite its exhaustion and lack of Hunter. It remains exhausted and does not
engage or attack immediately. Upkeep 4.3 readies it; normal engagement then puts
it with Tony. It adds one doom at the later round-end count. Do not rerun the
enemy attack step after this movement or readying.

Sources: original `01107/01160`; RR pp10,12,23–25. The existing assessment's
ordinary-Hunter and Retaliate cases remain separate.

### S3. Ancient Evils interrupts mythos draws with a real advancement

Start Act 2 / Agenda 3 in encounter-draw step 1.4, with eight doom on the agenda
and one on Tony's Quarry. The current investigator draws Ancient Evils. Unless
legally cancelled, its added doom makes the total ten and its explicit text
advances the agenda immediately. Remove doom from all cards, including Quarry,
then resolve Agenda 3b at Act 2 and reach R3. Do not draw encounter cards for the
remaining investigators or finish an ordinary round after that resolution.

By contrast, Quarry's own entry doom or Agenda 3's round-end doom has no such
advancement permission. Quarry's Monster trait does not make it a Ghoul for
the scenario reference or Agenda 3's count.

Sources: original `01166/01107/06012`; RR pp3–4,9,22,24; CG p3. Cancellation
legality and its nested test are covered by the Patrice investigation.

### S4. Terminal act advancement precedes after-defeat rewards

Start Act 3 with a fifteen-health Priest carrying fourteen damage and at least
one bounty. Tony succeeds at an ordinary basic fight for one damage, with no
other damage modifiers. The Priest is defeated and goes to the victory display.
Act 3's **if** condition advances the act before **after**-defeat abilities.
The lead chooses burn or refuse; the selected R1/R2 ends the scenario.

There is no post-resolution Bounty Contracts payout, “If it bleeds…” play,
after-test bonus action, final turn-end Internal Injury damage or extra upkeep.
The scenario's resolution and applicable game-end instructions still run.
The fixture deliberately uses a basic fight so it does not conflate this proof
with conditional text embedded in a weapon attack's own effect.

Sources: original `01110/01116/06010/02225/02038`; FAQ §2.2; RR pp3–4,21–22.
This is a derived timing consequence, not a published card-specific FAQ answer.

### S5. Preserve simultaneous choices and distinct reset boundaries

At round end with Agenda 3 and Dissonant Voices in play, after any Act 2
**when** opportunity, the lead chooses the order of those two **at** forced
abilities. Both must resolve. Agenda 3 counts Ghouls; Voices discards and its
asset/event prohibition ceases. This order does not itself create a free player
window or cause an immediate doom-threshold check.

Upkeep 4.2 has already reset ordinary actions for the next turn; 4.3 has readied
exhausted assets. Neither step grants another use in the current round of Quick
Thinking's taboo maximum or Jacqueline's per-round ability. A new round renews
the relevant per-round allowance. One in the Chamber's unused next-attack bonus
cannot carry into that new round. RR p14 places expiration of an **until**-end
lasting effect before the corresponding **at** boundary; implementations must
not clear all duration and usage state with one generic ready/reset operation.

Frozen in Fear and Internal Injury are own-turn-end effects, not round-end
effects. Tony's extra action is restricted by his turn and target eligibility;
it is not a generic once-per-round counter that can be renewed by readying.

Sources: original `01165/01164/02038/06003/09029`; RR pp14,17,23–25;
FAQ p26 for `02229`. Dark Future's own-turn timing is adjudicated separately.

### S6. Quarry's automatic bounty precedes Contracts' optional placement

Start with enough bounties on Contracts and Quarry entering play at its legal
farthest location. Quarry's forced entry effect puts doom on itself, then adds
one bounty from the pool. Only after mandatory effects at that timing point
does Tony choose whether to use Contracts' reaction. Quarry's health is three,
so the existing bounty leaves room for at most two additional bounties from
Contracts. Damage on an enemy does not reduce its health value for this cap.
Quarry stays Aloof unless separately engaged. Neither bounty placement nor its
entry doom constitutes an attack or an immediate doom-threshold check.

Sources: original `06010/06012`; RR abilities p2, health/damage p12, doom p9.
This case protects the distinction between forced consequences and optional
player allocation during a scenario/card-created entry sequence.

### S7. The four ending routes retain different records

| Ending trigger | Expected records and consequences |
| --- | --- |
| Priest defeated; lead burns house (R1) | House burned; lead gains Lita and one mental trauma; each investigator receives victory XP plus two. |
| Priest defeated; lead refuses (R2) | House standing; each investigator receives victory XP plus two, and lead receives one further XP; no Lita-acquisition instruction. |
| All investigators eliminated without a resolution, including Agenda 3b at Act 3 | House standing; Priest alive; lead gains Lita; each investigator receives victory XP plus two. Preserve each investigator's actual resignation/defeat and trauma history. |
| Agenda 3b at Act 1/2 (R3) | Lita seeks others; house standing; Priest alive; non-resigned investigators killed; no generic victory-plus-two award. Apply the guide's replacement/continuation and Lita-recipient instructions. |

Only revealed, clue-free victory locations qualify at scenario end. For example,
with Priest (2) and eligible Attic (1), R1 gives five XP each plus the lead's
trauma/Lita; R2 gives six to the lead and five to the others. Resignation can
drop clues onto a location and make it ineligible. Elimination does not shrink
the original three-investigator scaling. The application must record the ending
and stop at the accepted successor-preparation boundary, rather than launch
Midnight Masks or replace killed investigators automatically.

Sources: CG pp2–3; RR campaign/elimination pp5–6/10 and victory p21; FAQ p2
correction runs applicable game-end weakness instructions on elimination.

### S8. Agenda 2's no-Ghoul return is finite; source-policy note

All six original ordinary Ghoul enemies can be unavailable to the deck: the three
Minions and Ravenous Ghoul may be in play while Flesh-Eater and Icy Ghoul are in
the victory display. Priest is still set aside or elsewhere. Agenda 2b first
shuffles its discard into the encounter deck, but that cannot restore those
unavailable Ghouls. Its discard-until operation can therefore find no match.

Recommended expectation: discard the available deck once, finish without drawing
a Ghoul, and then rebuild the empty encounter deck. Do not pull from the victory
display or set-aside area, invent a replacement enemy, or loop forever.

The legacy RR p10 says to rebuild an empty encounter deck but does not explicitly
state the within-effect timing in that paragraph. Its pp9–10 effects rules
support resolving as much of a bounded effect as possible. The current Grimoire
p10 explicitly defers this rebuild until the active effect completes. More
direct legacy corroboration is the [Smite the Wicked FAQ entry on ArkhamDB](https://arkhamdb.com/card/02007),
whose FAQ section is marked last updated 2017-06-22. It attributes its answers
to official FAQs/rules-form responses and specifically describes a discard-until
operation finding no enemy: finish the effect, then reshuffle. This is secondary
custody of an attributed official answer, not a newly verified publisher-hosted
original. User reviews below that FAQ are not the evidence.

The consolidated recommendation is the consistent finite result, supported by
legacy effect rules and that closely matching legacy answer. Keep its provenance
qualification visible. The current Grimoire corroborates the timing; it does not
independently authorize importing every newer rule into the legacy game.

## Evidence limits and implementation consequence

The definite cases above resolve the named act/agenda/round ordering family and
bound all selected Tony/scenario card text. The implementation needs explicit
timing points, optional decisions, nested continuations, terminal resolution and
separate duration/usage lifetimes. This does not select a generic interpreter or
new framework. Tests should inspect player-visible choices, consequences and
pending decisions through the accepted production seam, including save/resume
around clue payment and the lead's ending choice. No such tests were implemented
or executed by this research.

Source changes, a different taboo, optional encounter replacements, a different
party or newly selected cards require proportionate reassessment. Current source
coverage and these worked expectations do not prove exhaustive interactions,
full Gathering execution, presentation quality or owner gameplay acceptance.
