# Complete Gathering assessment

Assessment for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89),
2026-11-02, initially against `a4a50c8a2ddb72cd7a2edf5d074ba5a57311a907`,
reassessed from `450e418194cfcd5c4e1f7b7f48fa68f066522f69` after Joel supplied
the actual basic weaknesses and requested stronger encounter-deck evidence.
This report is a decision proposal. The issue's current handoff owns acceptance,
allowance, checks and publication status. The accompanying
[preparation record](gathering-assessment-preparation.md) preserves the earlier
interview unchanged.

**Follow-up:** Joel approved the eight-hour rules investigation after this
reassessment. The [result and implementation brief](issue-89-rules-investigation.md)
now supply the full selected-card text audit and worked answers to the named
timing families, with two localized interpretation qualifications. Read that
return for the current source conclusions; the issue handoff owns activation
and acceptance status.

## Decision in brief

**The starting weakness scope and Gathering encounter inventory are now bound.
Drop the random-weakness preparation detour; finish the remaining named card-rule
expectations before activating the real-start gameplay increment.** Keep complete
original The Gathering on Standard as the selected playable milestone. This
reassessment removes two input uncertainties, while the existing experiments
still do not establish full-scenario implementation readiness.

The three accepted deck snapshots and taboo response are unchanged on this
inspection. The difficult work is connected behavior: the full decks can be drawn
from setup, Patrice replaces her hand every upkeep, the party separates and
rejoins, encounter effects interrupt decisions, and unsuccessful outcomes must
produce campaign records. Removing the current endpoint would expose states the
Party implementation expressly rejects.

Joel has identified Jacqueline's **Chronophobia `02039`** and Patrice's
**The 13th Vision `05041`**; Tony retains **Internal Injury `02038`**. These are
owner-supplied corrections to the starting snapshots, not agent-selected draws.
Only those three basic weaknesses need implementation for this start. No general
weakness pool, random selection, successor-weakness chain or upgraded deck import
is required. The source snapshot remains unchanged; preparation must bind the
two explicit replacements alongside it.

Encounter completeness has a direct official cross-check: **Learn to Play p8
enumerates all six sets, their copy totals and collector-number ranges**.
Together they contain 40 cards. Original setup keeps 14 outside the encounter
draw pile, leaving exactly 26. Section 3 gives the complete identity/quantity
table, publisher print-file evidence and the proposed exact-composition checks.
This establishes the inventory; future tests must still establish its execution.

There is a source identity warning worth retaining: the currently linked
Grimoire still says 1.1 but differs from the earlier retained 1.1 bytes. A version
label alone is insufficient. Existing records must retain their original source
binding; fresh preparation must deliberately bind the reconciled source set.

The proposed first gameplay checkpoint is **real setup through the first complete
round and the following mythos phase**, using all three real decks and preserving
all legal choices and any earlier resolution. It tests the boundary the prepared
Study cannot: real hand lifecycle, round limits, encounter interruptions and
continuity together. It is a checkpoint inside full-Gathering delivery, not a new
prepared experiment or a substitute acceptance target. Its reachable content is
large; one round does not mean a small card subset.

The remaining alternative is to commit immediately to full Gathering before the
named interaction expectations are complete. That buys earlier coding but hides
oracle risk. Parking is reasonable if the estimated effort below exceeds appetite;
there is no evidence here that another prepared demonstration would satisfy the
selected outcome. No follow-up issue, architecture, engine or implementation is
activated by this recommendation.

## 1. Source contract and custody

The [official support page](https://www.fantasyflightgames.com/product/arkham-horror-the-card-game/)
separates Chapter One sources from Chapter Two. Fresh retrieval on 2026-11-02
found the Chapter One Rules Reference, original campaign guide, Innsmouth insert
and FAQ 2.5 byte-identical to the earlier retained inputs. All nine retained
PDF/API inputs match [their manifest](issue-1-evidence.json). Sources below are
identified by exact bytes, not dates inferred from URL paths.

| Source | Identity and controlling locators |
| --- | --- |
| [Rules Reference](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_rules_reference_eng-compressed.pdf) (RR) | AHC60 English, 32 pages; SHA-256 `14823bd23033a857efd78b5c30d2e0fa5eb0cf004675573bf6e8c6698777df4a`. p2 precedence, campaign rules pp5–6, timing pp22–26, setup p27. |
| [Original Night of the Zealot guide](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/night_of_the_zealot_campaign_guide.pdf) (CG) | Eight pages; `132bb78a2c70245d2aa8037bab39784f42431b01c02ffa9d5b44d2edc10e13fb`; pp1–3 for this assessment. Original scenario, no Return To or optional encounter substitution. |
| [Legacy FAQ 2.5](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc_faq_v25_february_2026-web-1.pdf) (FAQ) | February 2026, 30 pages; `35c82dc070332acd7863eb6d464641ea79e4657b68ff4f1873538e0bcac6b2e7`; original English/errata p2, choice/token rules §§1.5, 2.5, 2.9, 2.20; weakness guidance pp16–17; nested reveal p17; Eyes/Katarina p22; taboo pp26–27. |
| [Innsmouth investigator insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc81_rules_insert.pdf) | AHC81, two pages; `3df58b1500071a72b1e8f4dd6127f3a181e244b6e1cf8f497c648c70a6238c25`; p1 bless/curse, needed by Promise of Power despite the selected campaign. |
| [Current Grimoire 1.1](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/07/Arkham-Grimoire-v1.1_web-1.pdf) | 48 pages; `a0ae9ebd0fc6f1455aedf95dacfd0f19a80ccb832c559e4aafbcfb437d0e9a44`. Earlier 1.1 is `0f932630f49d19fa05e71f34ec0cf1298db0978204bb3be41bda2c67bc1197ed`. pp2,41 retain legacy delegation; p46 distinguishes original encounter experience. |

The current Grimoire adds whole-deck-search shuffling language on p21 and removes
the former Decoy Trap erratum on p41. These inspected edits do not directly change
the selected older starting-card identities. All three selected basic weaknesses
also predate Chapter Two. Do not adopt the Grimoire wholesale over the legacy
sources; a later separately selected input may require fresh reconciliation.

Precedence proposal: original English card/scenario identity, applicable official
errata, then the pinned taboo overlay; specific rules override general rules,
absolute prohibitions remain, and FAQ §2.20 governs encounter/player conflict.
RR overrides Learn to Play; expansion instructions supply their specific mechanics.
ArkhamDB is a transcription/index and the Haskell implementation is secondary
evidence. Neither adjudicates a conflict. Investigate conflicts against the
controlling sources; if unresolved, identify the affected behavior and a proposed
interpretation or refusal. The Grim Rule is not a license to manufacture a
permanent program ruling.

The archive's reference manifest still binds metadata commit
`d8cc0e7e9081c0171298f8b1c07c90ddc2b08662`; all 163 reference records hash-verify.
The [inspected upstream comparison](https://github.com/Kamalisk/arkhamdb-json-data/compare/d8cc0e7e9081c0171298f8b1c07c90ddc2b08662...ff531296e3e290d761a526ece5090cbe0e23cc25)
is twelve commits ahead. Selected-pack changes inspected include flavor formatting
and restoration of the Parlor Resign action colon; they do not change its printed
behavior. No archive update was made. Catalog integrity is distinct from official
completeness or full effective-text review.

### Deck identity and taboo

| Accepted deck | Current observed revision | Preparation consequence |
| --- | --- | --- |
| [Tony 5392133](https://arkhamdb.com/api/public/deck/5392133.json) | 0.5; updated 2025-10-07 11:56:28 UTC | 35 slot copies / 22 codes; Guardian secondary class, Internal Injury already specified |
| [Patrice 5391062](https://arkhamdb.com/api/public/deck/5391062.json) | 0.7; updated 2025-10-07 11:56:14 UTC | 45 / 25 in retained API snapshot; replace its `01000` copy with owner-bound The 13th Vision `05041` |
| [Jacqueline 5389418](https://arkhamdb.com/api/public/deck/5389418.json) | 0.9; updated 2025-10-07 11:55:52 UTC | 35 / 22 in retained API snapshot; replace its `01000` copy with owner-bound Chronophobia `02039`; retain two Arcane Research permanents |

All complete live deck responses and the [taboo response](https://arkhamdb.com/api/public/taboos/)
match the hashes in `issue-1-evidence.json`. There is no observed slot or metadata
drift. The lists still specify taboo 9; the latest returned is **10**, effective
2026-02-19 (update field 2026-10-12). New preparation must bind 10 if it remains
latest when preparation runs. Quick Thinking `02229`, Dark Prophecy `04032` and
Voice of Ra `60416` have identical relevant overlays in 9 and 10. The complete
[deck projection](issue-1-decks.json) remains the identity inventory: 64 distinct
slot codes including the placeholder, 66 physical identities with investigators.
Those figures describe the historical snapshot, before the two owner-bound
weakness replacements; they are not a behavior-completeness count. Historical descriptions, XP and trauma
are not imported; Jacqueline's two legitimate Arcane Research mental trauma are.

### Retention and recovery

Existing raw evidence is in ignored `local/issue-1-evidence/`, with URLs and hashes
in the tracked manifest. The original archive remains at the location documented
in the [source contract](../foundation/sources-and-verification.md). Newly consulted
bytes are retained in ignored `local/issue-89-evidence/`:

| File | Bytes | SHA-256 / recovery source |
| --- | ---: | --- |
| `grimoire-1.1-current.pdf` | 27,027,366 | Current Grimoire hash and link above |
| `dream-eaters-investigator.pdf` | 4,516,689 | `a3d41baac74ff2bb0016997b2aa722435e3ec557386ed9fea2c30efad5d4c706`; [official insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc78_rules_insert.pdf) |
| `learn-to-play.pdf` | 4,636,917 | `f82790f81af3731f11f0a7ddc267feb0644b47d20842a973f1023de709568ab0`; [official book](https://images-cdn.fantasyflightgames.com/filer_public/dd/78/dd7818fe-0c9a-4a6c-b685-e32ab55b1702/ahc60_learn_to_play_web.pdf) |
| `current-cards.json` | 2,512,558 | `36bdf57ced256a33c3145b1562eea5f8ea6146d38e6a600cb04e27d43ef5749e`; [cards API](https://arkhamdb.com/api/public/cards/) |

These copies are presently retrievable, **without verified independent backup**.
Recovery means retrieving into a fresh location and matching the recorded hash,
or restoring an exact owner-controlled copy. A changed response is a new source
candidate; it cannot replace historical bytes. Missing bytes downgrade the exact
provenance claim until restored. Long-lived replay/fidelity needs retained,
recoverable copies for its lifetime; current machine-local custody alone does
not demonstrate that. No source payload, artwork or font is included in the
commits, and no evidence-management system is proposed.

## 2. Real preparation and refusal

Prepare in this order: bind campaign/difficulty and source revisions; bind exact
starting snapshots plus the owner-supplied weakness replacements below; check
class choices, legality, signatures, permanents and reachable behavior support;
then apply real campaign/scenario setup. A source mismatch or unsupported
behavior must identify the deficit before play, preserving the accepted inputs.
Reopening must not replace a weakness, charge twice or repeat trauma. Missing
art uses readable substitutes instead of blocking rules support.

| Investigator | Fixed basic weakness (one copy) | Behavior to implement |
| --- | --- | --- |
| Tony | Internal Injury `02038` | Existing selected card: threat-area revelation, end-of-own-turn direct damage, two-action removal |
| Jacqueline | [Chronophobia `02039`](https://arkhamdb.com/card/02039) | Threat-area revelation; one direct horror at the end of her own turn; a single activation costing two actions removes it |
| Patrice | [The 13th Vision `05041`](https://arkhamdb.com/card/05041) | Threat-area revelation; investigators at her location fail skill-test ties; a single activation costing two actions removes it |

Joel supplied the latter two identities on 2026-11-02, reporting that he had
recorded them in his upgraded decks. This is the provenance of the correction;
the assessment does not claim to have inspected those upgraded lists. Apply the
correction to the starting snapshots, without importing upgrades, prior XP,
trauma or other history. Keep the raw `issue-1-decks.json` and API bytes unchanged
because the existing experiments consume that historical projection. New
preparation binds each original deck revision and its explicit replacement of
one `01000`; a placeholder surviving in the prepared playable deck is an error.

Both new original English faces were visually checked, including their two
action icons (which the plain webpage extraction omits). They introduce no
weakness evolution chain, setup XP grant or random-pool operation. Their genuine
cross-card costs remain: direct horror against Jacqueline's reduced sanity
margin, same-location help removing threats, and failure-on-tie interactions
with Lucky!, Take Heart and failure-margin consequences. Section 4 adds worked
expectations; a tie must not be implemented as a blanket skill penalty.

**Resolved scope deficit:** no all-official weakness census, edition-weighting
policy or random draw is a prerequisite for these three starting decks. The
earlier pool investigation remains historical evidence at commit `450e418`,
not a pending task. Only an actually reached later campaign requirement for a
new weakness can reopen selection/support work. That later boundary does not
authorize preemptive implementation of unselected weaknesses now. No ArkhamDB
deck, personal record or campaign state was changed by this reassessment.

Source bindings in the unchanged external archive: Chronophobia face variant
`700c94271661862d`, SHA-256
`8774151d492fd96faaaea5654bb4b8684335eb6adddddd1d8b88d95b9aeeaf5d`;
The 13th Vision variant `54ff6e0f4fbce54b`, SHA-256
`ee8fddb57f0503184c89fe118c76f125b0d7508bc8630cea141a5404d1d83bb4`.
The catalog resolves these to their original English Dunwich #39 / Circle Undone
#41 face paths. No named erratum was found for either in retained FAQ 2.5, and
neither has a taboo 10 entry. RR pp3–5,9,20,26 and FAQ §§1.2/2.1/2.9 govern the
action, threat-area, direct-horror and result interactions. The image inspection
does not authenticate the archive's publisher provenance; images remain external.

## 3. Behavior and coverage

This map starts from the selected source/deck inventory, not the current Rust
representation. Card codes below resolve through the pinned deck projection and
archive catalog. Every selected identity is included; shared cards occur in both
consumer contexts. Grouped coverage identifies obligations, not implementation
or complete adjudication. Original faces were inspected for the new worked-case
cards (Patrice, Watcher, Quarry, Arcane Research, Eyes, Katarina, Confidence, Dark
Future, Agenda 3 and Act 2); the remainder's metadata text is discovery evidence
pending complete effective-text reconciliation.

| Selected content | Codes and behavior obligations |
| --- | --- |
| Tony, signatures, fixed weakness | `06003,06010,06011,06012,02038`: restricted Fight/Engage action; bounty placement, custody, kill credit and resources; paired weapons; remote aloof Quarry spawn and doom; two-action removal and end-turn direct harm |
| Tony attacks/acquisition | `01049,05117,60306,09029,03263,60114,09068,05277`: paid boosts, uses/ammo/exhaustion, oversuccess choices, next-attack expiry, mythos search/replacement/no-match, spawn overrides, non-Elite targeting, remote engagement and damage |
| Tony information/economy/clues | `04037,04107,06162,03233,04103,05111`: Backpack search/attachment/play permission, hidden-information boundary, draw triggers, wagers, first-action restriction, opportunity exemption, paid clue quantity/range |
| Tony skills/healing | `01089,02229,04150,02225`: commitment limits, success draw, taboo round-limited extra action, party-wide completed actions in phase, Monster defeat and same-location healing |
| Patrice/signatures | `06005,06016,06017`: upkeep hand replacement, hand maximum, simultaneous draw/revelation, reshuffle/hidden retention, resource/draw gift, in-hand enemy Fight/Evade, failed spawn and deck-exhaustion attack |
| Patrice attacks/movement/clues | `04029,60407,06201,06164,01079,04272,60507`: alternate skills, charges/backlash, optional movement/arrival harm, engagement and round-duration effects, failure-margin clues, discard play/shuffle, shroud/key lifecycle |
| Patrice protection/recursion | `03039,06165,04200,01080,01065,05032,60513,09102,10111,02033`: distinct retrieval eligibility; retry after failure consequences; would-fail modifier; cancellation versus prevention; defensive nested test; discard costs/restrictions; assignment, replenishment and healing |
| Patrice economy/skills/basic weakness | `03033,04201,01093,02192,07032,05041`: spell cost reduction, failure rewards to tester, max/self-test/base-skill constraints, immediate curse insertion or committer horror; local failure-on-tie and shared two-action threat removal |
| Jacqueline/signatures/preparation/basic weakness | `60401,60402,60403,04109,02039`: same-location optional reveal replacement and per-round limit/Arbiter exception; prohibition on cancel/ignore; end-turn non-test reveal; permanent trauma and later Spell discount; own-turn direct horror and shared two-action threat removal |
| Jacqueline token composition | `04197,04199,04032,11067,11069,11097,60419,60411`: predeclared nested replacements, sealing/substitution, curse continuation/custody, taboo selection, ignored-token matches, reveal-sensitive icons, retrieval and outcome prediction |
| Jacqueline actions/assets | `60408,06117,06201,07029,03033,60412,60416,05112,01065,05032`: skill substitution/addition, clues/backlash, location-trigger ignore limits, on-play activation, discounts, non-test income, restricted arcane slot, cancellation/prevention |

Relevant original card transcriptions are in the pinned
[core](https://github.com/Kamalisk/arkhamdb-json-data/tree/d8cc0e7e9081c0171298f8b1c07c90ddc2b08662/pack/core),
[Dream-Eaters](https://github.com/Kamalisk/arkhamdb-json-data/blob/d8cc0e7e9081c0171298f8b1c07c90ddc2b08662/pack/tde/tde.json)
and [Drowned City](https://github.com/Kamalisk/arkhamdb-json-data/blob/d8cc0e7e9081c0171298f8b1c07c90ddc2b08662/pack/tdc/tdcp.json)
records; the complete pack/file identities remain in the existing manifest.
The source order above governs discrepancies with transcriptions.

### Scenario, rounds and adverse branches

CG p2 selects **The Gathering, Rats, Ghouls, Striking Fear, Ancient Evils and
Chilling Cold**. The independent publisher inventory is
[AHC60 Learn to Play p8](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_learn_to_play_web-1.pdf),
headed “The Gathering Card List”: 16 + 3 + 7 + 7 + 3 + 4 = **40 physical cards**.
It gives the complete collector-number ranges and explicitly instructs gathering
all copies using each card's printed encounter-set number (`#/#`). It also allows
other investigator selections/counts; the inventory is not limited to the
tutorial's Roland/Wendy party. CG and Learn to Play pp8–9 determine disposition.

The five [publisher Chapter One encounter PDFs](https://www.fantasyflightgames.com/product/arkham-horror-the-card-game/#support)
independently supply every numbered physical copy in the shared sets. Their
original names, printed core numbers, encounter symbols, rules text and numerical
values were checked against the original archive faces. They are not Return To,
optional refraction or Chapter Two replacements. All sixteen numbered Gathering
faces and the ten shared-set identity faces were visually inspected. The six sets
close their printed number ranges without gaps or duplicate positions.

| Set | Original identity | Card | Printed positions | Copies | Initial destination |
| --- | --- | --- | --- | ---: | --- |
| Gathering | `01104` | The Gathering reference | 1/16 | 1 | Scenario reference, Easy/Standard face |
| Gathering | `01105` | What's Going On?! | 2/16 | 1 | Agenda deck, current |
| Gathering | `01106` | Rise of the Ghouls | 3/16 | 1 | Agenda deck, second |
| Gathering | `01107` | They're Getting Out! | 4/16 | 1 | Agenda deck, third |
| Gathering | `01108` | Trapped | 5/16 | 1 | Act deck, current |
| Gathering | `01109` | The Barrier | 6/16 | 1 | Act deck, second |
| Gathering | `01110` | What Have You Done? | 7/16 | 1 | Act deck, third |
| Gathering | `01111` | Study | 8/16 | 1 | In play, starting location |
| Gathering | `01112` | Hallway | 9/16 | 1 | Set aside |
| Gathering | `01113` | Attic | 10/16 | 1 | Set aside |
| Gathering | `01114` | Cellar | 11/16 | 1 | Set aside |
| Gathering | `01115` | Parlor | 12/16 | 1 | Set aside |
| Gathering | `01116` | Ghoul Priest | 13/16 | 1 | Set aside |
| Gathering | `01117` | Lita Chantler | 14/16 | 1 | Set aside |
| Gathering | `01118` | Flesh-Eater | 15/16 | 1 | Encounter deck |
| Gathering | `01119` | Icy Ghoul | 16/16 | 1 | Encounter deck |
| Rats | `01159` | Swarm of Rats | 1–3/3 | 3 | Encounter deck |
| Ghouls | `01160` | Ghoul Minion | 1–3/7 | 3 | Encounter deck |
| Ghouls | `01161` | Ravenous Ghoul | 4/7 | 1 | Encounter deck |
| Ghouls | `01162` | Grasping Hands | 5–7/7 | 3 | Encounter deck |
| Striking Fear | `01163` | Rotting Remains | 1–3/7 | 3 | Encounter deck |
| Striking Fear | `01164` | Frozen in Fear | 4–5/7 | 2 | Encounter deck |
| Striking Fear | `01165` | Dissonant Voices | 6–7/7 | 2 | Encounter deck |
| Ancient Evils | `01166` | Ancient Evils | 1–3/3 | 3 | Encounter deck |
| Chilling Cold | `01167` | Crypt Chill | 1–2/4 | 2 | Encounter deck |
| Chilling Cold | `01168` | Obscuring Fog | 3–4/4 | 2 | Encounter deck |

The **14 non-deck cards** are one reference, three agendas, three acts, Study,
and six set-aside cards (four locations, Priest and Lita). Thus **40 − 14 = 26**
initial encounter cards. Each double-sided card is one physical card, not two.
These copy counts do not scale with investigator count. Standard selects the
reference face and chaos bag, not a different encounter-card multiset. These
personal basic weaknesses belong to investigator decks, not this encounter deck.

The source-derived expected initial multiset is:

```json
{"01118":1,"01119":1,"01159":3,"01160":3,"01161":1,"01162":3,"01163":3,"01164":2,"01165":2,"01166":3,"01167":2,"01168":2}
```

Future implementation must compare **every original identity and copy quantity**
with this expectation, and separately check all fourteen non-deck destinations.
Reject missing, extra, substituted and excess-copy cards. Include a same-total
negative case (replace an Ancient Evils with a fourth Rat), a missing-copy case,
and a premature Ghoul Priest inclusion. A total-count check or a set of twelve
unique names would miss these errors. Verify through the real preparation path,
without exposing hidden deck order in normal player observations. These are
proposed acceptance checks; no new gameplay test was implemented here.

This establishes original Gathering's inventory completeness from independent
publisher evidence, not merely agreement with ArkhamDB. It does not establish
rules execution. Archive duplicate entries may reuse a single scan (for example
Rats 1/3); those entries are not independent evidence of three copies. The
publisher PDFs supply positions 1/3, 2/3 and 3/3 separately. The Gathering image
copies retain an archive-provenance limit, but publisher p8's sixteen-card range
and the inspected numbered faces leave no unidentified inventory member.

#### Encounter evidence custody

Retrieved on 2026-11-02; all files below are retained under ignored
`local/issue-89-evidence/encounter-audit/`. The publisher labels these English
resources Chapter One; the printable cards retain original core numbers and
©2016 markings, with no separate numbered print-file revision. Face pages are
odd-numbered PDF pages, in printed-copy order; backs occupy the even pages.

| File / publisher link | Bytes | SHA-256 |
| --- | ---: | --- |
| [Learn to Play](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc60_learn_to_play_web-1.pdf), `learn-to-play.pdf` | 4,636,917 | `f82790f81af3731f11f0a7ddc267feb0644b47d20842a973f1023de709568ab0` |
| [Rats](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/rats.pdf), `rats.pdf` | 2,605,975 | `ebe274169ebfe5f0daec22ddef091b06eaaab4baaf7d055f58ea4a35c90e5929` |
| [Ghouls](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ghouls.pdf), `ghouls.pdf` | 6,414,630 | `2acb9b5c68a53c70f68b477e08c9cdf4505289d40814562a314ce2cdf136ff18` |
| [Striking Fear](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/striking_fear.pdf), `striking-fear.pdf` | 5,617,016 | `4d24cd47f4d93c293a9b2cfbde0932e18235a92b86172860c9bd11a497b5ecbd` |
| [Ancient Evils](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ancient_evils.pdf), `ancient-evils.pdf` | 2,548,907 | `ad4776223844cc4ba1669f880314134cc950d4e456edf0fcda5cca5dd999f6c2` |
| [Chilling Cold](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/chilling_cold.pdf), `chilling-cold.pdf` | 4,015,048 | `f534100759f11e7595de265c595259df7fca0286ffa5ec476a9f0ce79cbba8ff` |

The Learn to Play bytes match the earlier retained copy despite a different URL.
Scoped original-image paths, variant IDs and hashes are retained in
`encounter-image-bindings.json` beside these files (14,549 bytes; SHA-256
`635310d89a96652ca851904f52cc551e1eb2bbd5b332460f1509f99bf2f696b0`);
all 26 face files and the five
revealed-location backs were hash-checked successfully. Gathering's numbered
sequence uses `01104` face and `01105–01110` faces, `01111–01115` revealed backs,
and `01116–01119` faces. This binds the visual inspection without publishing
artwork. Custody/recovery limits in section 1 still apply: no independent backup
has been verified, and a later download must match the hash or undergo new review.

#### Required scenario behavior

| Obligation | Source-derived coverage required |
| --- | --- |
| Genuine setup | RR p27, CG pp1–2: choose lead, bind Standard bag, fresh trauma/permanents, shuffled full decks, five resources each, opening hands and optional mulligans with weakness replacement; Study alone initially, remaining rooms/Priest/Lita set aside; skip first mythos |
| Investigation | Complete turns in chosen order; costs/targets, ordinary and restricted actions, fast/reaction windows, cross-investigator commitments and genuine ordering choices; preserve a separate decision owner during interruptions |
| Enemy phase | Hunter paths and ties, engagement, attacks, evasion/exhaustion, retaliation, opportunity attacks, damage/horror allocation and defeat; distinguish player-owned weaknesses, encounter discard and victory destinations |
| Upkeep/end round | Ready, Patrice replacement versus ordinary draw, resource gain, hand limits, weakness/revelation processing, deck exhaustion, forced effects, expiry and round-limit reset; RR pp23–26 |
| Subsequent mythos | Doom placement/check across all cards, encounter draw order, replacement searches and no-match outcomes, tests, threat attachments, asset-loss fallback, shroud modifiers, play restrictions and Ancient Evils immediate advancement |
| Standard token face | `01104`: skull uses Ghouls at tester's location, cultist failure horror, tablet conditional damage; do not use the Hard face or a constant skull value |
| Locations | Connections/reveal/clues, Attic entry horror, Cellar entry damage, Parlor barrier; later Resign and Lita parley (difficulty 4), control and same-location benefits |
| Acts | Act 1: six group clues for three starters; relocate everyone, discard Study enemies, remove Study, expose four rooms. Act 2: nine clues from investigators in Hallway, optional payment only at round end; reveal Parlor/place Lita/spawn Priest. Act 3: Priest defeat and lead's burn/refuse choice. Elimination does not reduce per-investigator scaling |
| Agendas | Thresholds 3/7/10. First reverse preserves lead's choice between group random discard and lead horror. Second reshuffles encounter discard and finds a Ghoul. Third has extra unengaged-Ghoul movement at enemy-phase end and doom per Ghoul in Hallway/Parlor at round end; its reverse distinguishes Acts 1/2 from Act 3 |
| Endings | Record resolution, house/Priest/Lita flags, individual XP, trauma, death/insanity/resignation/defeat and applicable weakness consequences; all-eliminated/no-resolution differs from R3. Do not automatically begin Midnight Masks |

### Actual implementation comparison

| Family | Existing inspected evidence | Missing or unproved for this scope |
| --- | --- | --- |
| Preparation | [`src/party.rs`](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/src/party.rs) reads full slot inventory | Constructs one chosen hand card, zero resources and fixed three-card draw prefixes; no real setup, consumption of the explicit weakness bindings or full deck support check |
| Cooperative decisions | Party decision tests exercise turn order, commits, Promise, Jacqueline, elders, Lucky! and Act closing | Only one prepared turn per investigator; Party card view has intellect icons only and validation fixes test difficulty 2 with bounded harm/locations |
| Enemy/actions | [`src/tony.rs`](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/src/tony.rs), [Tony tests](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/tests/tony_decisions.rs): one Ghoul, bounty action, opportunity attacks, Internal Injury | No integrated multi-investigator enemies, enemy phase, general allocation or scenario defeat |
| Map/scenario | Act reverse reading, six-clue spend and connected map persist | Party explicitly refuses move/fight/evade/engage and stops at Act 1 or before enemy phase; map visibility is not playable movement |
| Deck lifecycle | Known commitment discard and Patrice elder-sign shuffle | Unknown tails refuse draws; no opening hands, normal upkeep, Watcher, exhaustion, searches or general retrieval |
| Continuity/UI | Shared persistence and installed production UI checks; see section 5 | No full scenario pending continuations, offline campaign/replay run or populated native gameplay evidence |

Thus **supported** means only the exercised bounded behaviors above. **Missing**
means absent production paths for setup/full rounds/scenario outcomes and most
card mechanics. **Uncertain coverage** includes recomposing existing Tony and
Jacqueline behavior into full party play, untested timing combinations and actual
native burden. **Unresolved interpretations/oracles** include the exact deficits
below; none is hidden under a global passing-test claim.

## 4. Independent worked expectations and first increment

These are source-derived proposed oracles, independently prepared without reading
Rust or a secondary implementation to choose the answers. They are not executed
full-scenario tests. Fixture randomness is private test control.

1. **Fresh start.** With the accepted lists and no old campaign history, Jacqueline
   has two mental trauma and starts with two horror from Arcane Research. Her
   permanents are outside the shuffled deck. Each investigator has five resources;
   Bounty Contracts begins with six bounties. Opening-hand/mulligan weaknesses are
   replaced without revelation and shuffled back at the prescribed boundary.
   Sources: RR pp16,27; `04109`, `06010`; fresh-deck play contract. Actual selected
   weaknesses are the fixed three above; this example does not randomize them.
2. **Patrice across exhaustion.** Healthy Patrice holds Watcher plus four ordinary
   cards; her deck has two ordinary cards and an ample discard. With no other
   relevant effect, upkeep discards four, retains Watcher and draws four. Emptying
   the deck triggers Watcher's three-damage attack; needing further draws causes
   reshuffle and one horror after the entire draw. End with Watcher plus four and
   the ordinary resource. Finish simultaneous draws before revelation processing;
   do not repeatedly refill after revelations discard cards. Reject attempts to
   discard Watcher through upkeep, Violin or Idol. Sources: `06005/06017`, RR pp9,25,
   FAQ p17. Emptying the deck and needing another draw are distinct events.
3. **Eyes/Katarina/Confidence.** Jacqueline investigates Cellar (shroud 4) with
   Eyes, ready Katarina and Confidence committed, no other modifiers or Dark
   Future. Spend a charge; predeclare Katarina on one Eyes position. Reveal `−1`
   for the ordinary position and `−1, −2, skull` for Katarina. Ignore Katarina's
   numbers and choose the ordinary `−1` with Eyes. Four physical reveals give
   Confidence four wild icons total: `5 + 4 − 1 = 8`. Success discovers two clues,
   because the ignored `−1` matches. Reject unavailable charge costs and an already
   ignored token as the outer selection. Sources: `11067/11069/11097`; FAQ p6
   corrects Katarina's timing, p17 nesting, p22 explicitly counts Katarina-ignored
   matches. A flat list of final resolved tokens loses required information.
4. **Curse and poor legal choices.** In the earlier independent Rotting Remains
   case, skill 9 versus 3, reveal auto-fail/curse/zero. Cancel auto-fail: resolve
   curse/zero and additional `−2`, skill 5, no horror. Cancel curse/zero: auto-fail
   remains, skill treated as zero, three horror. Canceling auto-fail plus another
   token is illegal. Remote commitment and a second ordinary Jacqueline use in
   the round are illegal. If Promise cannot add a curse, its committer takes two
   horror. Sources: [earlier derivation](issue-1-source-reconciliation.md), FAQ
   §§2.5/2.9, Innsmouth p1. New round-reset composition remains future work.
5. **Unavailable spawn.** Before Act 1 advances, Attic is set aside. Ordinary
   Flesh-Eater draw cannot spawn there: discard it without inventing a replacement
   draw. On the Hunt can instead override its spawn and engage Tony; a search with
   no enemy follows that card's fallback. Sources: `01118`, `03263`, RR p19. This
   legal unavailable-location result differs from unsupported implementation.
6. **Doom and immediate advancement.** Agenda 1 has one doom; Quarry enters and
   adds one, total two, without an immediate threshold check. Ancient Evils adds
   one and explicitly checks: total three advances. Remove all doom, including
   Quarry's, then preserve the lead's reverse-side choice. Sources: `06012`,
   `01166/01105`, RR pp3,9,24.
7. **Enemy state and scaling.** Priest has fifteen health for three starters even
   after an investigator is eliminated. Failed fight against ready Priest causes
   retaliation; exhaustion prevents that retaliation. Successful evade exhausts
   and disengages; normal Hunter/attack steps do not act on exhausted Priest.
   Upkeep readies it and ordinary same-location engagement applies. Agenda 3's
   specific unengaged-Ghoul move can still move it while exhausted. Sources:
   `01107/01116`, RR pp10,12,16,18,25. Other-investigator engagement does not exempt
   the attacker from retaliation.
8. **Act 2 locality/time.** Tony in Hallway has four clues, Patrice there has five,
   Jacqueline in Attic has two. At round end Tony/Patrice may pay nine or decline.
   If Patrice is in Cellar, party-wide holdings do not authorize that payment.
   Turn end is not round end. Apply Priest/Parlor/Lita changes only after legal
   advancement. Sources: `01109`, RR p3. Combined end-round forced-effect order
   requires its own worked case.
9. **Three distinct adverse paths.** All resign/defeat without a resolution gives
   house standing, Priest alive, lead's Lita reward, and victory XP plus two each.
   Agenda 3 during Act 1/2 instead invokes R3: record Lita finding others, house
   standing and Priest alive; kill every non-resigned investigator, with no generic
   victory-plus-two award. If lead was killed, the guide calls for a chosen Lita
   recipient. Replacement investigator/deck preparation is a later boundary;
   lack of available replacements can end the campaign, but death is not an
   automatic campaign-ending shortcut. Agenda 3 during Act 3 instead defeats
   non-resigned investigators with physical trauma, then takes the no-resolution
   route. Sources: `01107` both sides, CG pp2–3, RR campaign/elimination.
10. **Rewards and control.** Suppose only Priest (2) is in victory display and
    only clue-free revealed Attic (1) qualifies additionally: V=3. R1 gives five
    XP each, lead one mental trauma, house burned and the lead's Lita reward.
    R2 gives lead six XP, others five, house standing, and no Lita acquisition
    instruction. Controlling Lita during play does not award her afterward.
    Resignation drops carried clues; that can prevent a location qualifying for
    victory. Sources: CG pp2–3, RR pp10,21, `01113/01116`. Prior trauma and weakness
    effects persist separately.
11. **Shared removal costs two actions, once.** Tony and Jacqueline are together;
    Chronophobia is in Jacqueline's threat area. On his turn Tony can spend two
    ordinary actions on its removal, but cannot pool one action with hers or use
    his restricted bounty Fight/Engage action to make up the cost. Remote Tony
    cannot activate it. If engaged with a ready Ghoul Minion, he receives one
    opportunity attack before resolution, not two because of two action icons.
    Assuming he survives, discard to Jacqueline's pile. The same access/cost rule
    applies to The 13th Vision. Sources: inspected faces, RR pp3–5,20,
    FAQ §§1.2/2.1.
12. **Chronophobia's own-turn boundary.** Jacqueline starts with two horror from
    Arcane Research. With Chronophobia in her threat area and no prevention, ending
    her turn brings her to three horror directly on her investigator; an ally
    cannot absorb it through ordinary allocation. Other investigators' turn ends
    do not trigger it. Drawing it during upkeep does not retroactively charge the
    already-ended turn. Removal before her turn ends prevents that instance's
    forced effect. Reopening a completed boundary must not charge it again.
    Sources: inspected Chronophobia, RR p9/timing, FAQ §2.1. Direct horror can still
    be prevented by an applicable effect; reaching sanity invokes normal defeat.
13. **Vision fails ties by zero.** Patrice with Vision tests Rotting Remains at
    willpower 4 and resolves `−1`: 3 against 3 fails by zero. With Take Heart
    committed, she receives its two cards/two resources but takes zero
    margin-based horror. Alternatively, at the would-fail window she can pay for
    Lucky!: skill becomes 5, success by two, no Take Heart reward. Both choices
    remain hers. A same-location Jacqueline tie also fails while Vision is at
    Patrice's location; a remote test does not. Take Heart rewards the testing
    investigator, not necessarily its committer. Sources: inspected `05041`,
    `01080`, `04201`, RR p26. Difficulty+1 would produce the wrong failure and
    success margins, so it is not an equivalent implementation.
14. **Zero difficulty is an ordinary comparison.** Patrice plays Winging It from
    discard to investigate Attic. Shroud 1 becomes 0; intellect 2 with `−2` becomes
    0. Under her Vision, that 0–0 tie fails: no clue or bonus clue. Winging It's
    subsequent shuffle still happens. Without the local Vision effect the same
    ordinary tie succeeds. Auto-fail remains failure even at difficulty zero and
    cannot be overcome by Lucky!'s skill increase; do not confuse zero difficulty
    with an actual automatic-success effect. Sources: `05041`, `04272`, `01113`,
    RR p5 and FAQ §2.9/pp16–17.

**Source/oracle follow-up:** the approved
[rules investigation](issue-89-rules-investigation.md) audited every selected
card identity and added traces for the named token, cancellation, retry and
scenario/round families. It corrects the apparent Act 2/Agenda 3 simultaneity:
the act's **when** precedes the agenda's **at**. Two narrow qualifications remain
explicit there: Prescient with an ignored-only matching token and Agenda 2's
no-Ghoul exhaustion timing. Their recommended interpretations are not presented
as explicit legacy publisher rulings. The proposed Rust behavior and complete
Gathering still require execution evidence. These are not requests for the
owner to conduct technical research or reasons to invent a generic interpreter.

The first conditional product-bearing checkpoint and its acceptance boundary are
specified in section 6. Preparation-only refusal is a useful prerequisite result;
it cannot stand in for that connected gameplay checkpoint.

## 5. Continuity, presentation and the implementation seam

The accepted seam is adequate as a **boundary of authority**, not as evidence
that the current Study state model fits a full scenario:

`observe_for_play → choose(revision, id) → persist candidate → install consequences`.

Both [player driver](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/src/bin/driver/party.rs) and
[native UI](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/src/party_tabletop.rs) call
[`party::continuity::commit_choice`](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/src/party/continuity.rs), which clones,
applies the offered choice, saves, then installs. Native animation begins after
success. Retain that ordering. Extend observations locally for preparation,
phase, decision owner, instances/zones, legal targets and allocation/order
choices as demanded by the source cases. No generic rules engine or new test
framework is justified. The implementer can choose module shapes within this
boundary without another architecture interview.

The current [Party decision tests](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/tests/party_decisions.rs) establish
selected cooperative commitments, curse continuation, Lucky!, elders and Act 1
closing. [Continuity tests](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/tests/party_continuity.rs) exercise pending
cancellation, failed saves, competing writers, corrupt/wrong-experiment refusal,
restart archival and recorded replay. [Driver tests](https://github.com/joeloverbeck/arkham-unfolded/blob/8c4e619e20f9a55dd2cbd3144ab21c26325d8af6/tests/party_driver.rs)
spawn a real process and reopen pending choices without exposing hidden data.
Comparison with another run of the same implementation is useful continuity
evidence; it is not an independent rules oracle.

New Gathering records must bind source/deck/taboo/weakness identities, random
state, hidden orders, committed effects and pending continuations. Undo must stop
at new hidden information. A failed save must preserve the previous choice and
randomness; retry must not charge, harm or draw twice. A replay must use recorded
events and permit Next/back with the intended camera behavior. Future proof must
include fresh-process reopening at upkeep and nested token choices, relocated
art, and play/save/replay with network and source-archive access unavailable.
These are proposed checks, not execution completed by this assessment.

Preserve Jacqueline, Tony and Party recordings through their current supported
readers and compatible application revisions. Party's `Escaped`, `Declined` and
`TurnsEnded` endpoints are experimental endpoints, not campaign outcomes. Use a
separately identified Gathering record contract; do not reinterpret them or
promise migration. No personal save was opened or changed during this assessment.

Retain [ADR 0001](../adr/0001-prepared-runtime-assets.md) and
[ADR 0002](../adr/0002-persistent-table-areas.md): prepared assets outside Git;
readable English substitutes; persistent scenario/investigator/Test areas;
camera focus independent of actor; inspection without game actions; explicit
commitments; contextual costs and opportunity warnings; distinct hover, focus,
source and pending-selection feedback. Full decks bring more assets, threats,
enemies and token groups than the prepared scene. The registered production UI
checks establish code behavior; future native play must establish readability,
pacing and physical interaction with those actual populations. Missing art may
use substitutes; missing rules must refuse preparation. No archive fallback,
asset conversion, new layout system or native session is part of #89.

Historical acceptance was reread live:
[#55](https://github.com/joeloverbeck/arkham-unfolded/issues/55#issuecomment-6558284237)
records three-investigator prepared play through Act 1;
[#73](https://github.com/joeloverbeck/arkham-unfolded/issues/73#issuecomment-6560193466)
and [#77](https://github.com/joeloverbeck/arkham-unfolded/issues/77#issuecomment-6562252036)
accept subsequent Party presentation refinements. Those observations remain
valuable; none establishes original setup, later rounds or complete Gathering.

## 6. Effort, bounded proposal and disposition

These are planning estimates against the missing behavior families, not measured
throughput or a delivery commitment. They concern Gathering only, not the rest
of Night of the Zealot. Do not subtract earlier experiment hours from the old
campaign estimate.

| Work for complete Gathering | Aggregate engineering hours | Main uncertainty |
| --- | ---: | --- |
| Source reconciliation, independent expectations, preparation adjudication | 8–20 | Remaining selected-card timing/effective text; initial weakness selection and encounter membership now resolved |
| Real preparation, rules and selected content | 80–180 | Full deck lifecycle, interruptions, enemy/phase/scenario integration |
| Independent verification and repairs | 45–100 | Adverse branches, refusals, persisted continuations and oracle quality |
| Tabletop and offline continuity integration | 25–60 | Crowded real states, exact pending choices, replay and recovery |
| **Planning total** | **158–360** | Fixed starting content; remaining rules, verification and integration uncertainty |

Expect at least factor-of-two uncertainty; the range is an order-of-magnitude
judgment, not a confidence interval or guaranteed upper bound. Compared with the
initial 162–370-hour range, remove roughly 4–10 hours of random-pool/preparation
research and the unknown weakness-family contingency. The two known cards add
specific ordinary threat/action and result-comparison cases already represented
in this assessment; they do not remove the full-deck or round machinery. The first real-round checkpoint could
consume much of this effort because all starting cards and early scenario
advancement are reachable. Existing persistence and presentation code reduces
some integration uncertainty but does not make full card support cheap.

Human review is separate: a future initial gameplay allowance might be one
60–90-minute real-play session and one 20–30-minute recovery/adverse inspection;
full scenario acceptance may need longer. Those sessions and their cumulative
budget require future acceptance. **This assessment uses only the single
20-minute report review already allowed, including any retry.** Maintenance is
also separate: reserve roughly 4–12 engineering hours for a material source or
record-compatibility update, then re-estimate if behavior changes rather than
silently consuming that allowance.

**Subsequently activated investigation:** Joel approved the revised eight-hour
rules proposal; its [return](issue-89-rules-investigation.md) supersedes this
paragraph's original prospective status. The former random-pool/preparation
implementation proposal remains withdrawn. The approved scope was at most eight
aggregate source-adjudication hours to close the remaining named interaction
families in section 4 and complete original-card/errata reconciliation for this
fixed input set. Return worked legal/adverse expectations and an implementation
brief against the accepted player-observation/choice/persistence seam. This is
the remaining fidelity prerequisite, not a new weakness selector or preparation
UI project. No further owner selection of starting weaknesses is needed. Stop at
an unresolved governing conflict that cannot be localized, the effort ceiling,
or existing non-convergence tripwires; return exact deficits without an automatic
extension. No child-ticket fan-out is selected by this report.

**Conditional gameplay proposal:** after those inputs and an implementation
allowance are accepted, implement original Gathering for this exact party on
Standard, with real setup, every reachable selected card, every phase, all legal
outcomes and recorded consequences. First inspectable checkpoint: the real
first round and following mythos, including legal early resolutions. Use ordinary
randomness for play; private deterministic fixtures may target the worked cases,
but cannot become player abilities or hide unsupported draws. Use the existing
player-observation/choice/persistence seam for TDD; no private-type-only oracle.

The first hostile trace should connect legal opening hands, a cooperative test
under The 13th Vision with a genuine failure/Lucky! decision, shared threat removal,
Patrice's upkeep with Watcher retained, round-limit reset, a following mythos
encounter, a saved pending response/token choice, an adverse outcome and replay.
Check legal alternatives and refusals as well as success. Before claiming that
checkpoint, confirm all content legally reachable within it is supported; if
its surface doubles the planned allowance, stop and return the cost evidence.
Before complete-Gathering acceptance, add sustained rounds, all scenario outcome
branches and recorded consequences, and actual owner play. A single trace or
passing headless suite proves neither completeness nor enjoyable play.

Exclude later scenario implementation, upgrades/successor-deck workflows,
arbitrary decks, full-catalog behavior, engines/DSLs, new research infrastructure,
record migration and third-party asset publication. Retain the later preparation
boundary for replacement investigators and successor decks; no automatic campaign
termination shortcut. Full Night of the Zealot remains unmet.

For any later implementation, use the issue's early stops: two active hours
without an inspectable result, three failed repairs at one boundary, three
review passes expanding an open semantic family, approximately doubled surface,
an unapproved capability, or exhausted effort/attention. Any implementation
allowance must explicitly cover the reachable content; this report's four-hour
assessment ceiling is not that allowance.

### Original owner review (20 minutes total, including retry)

Joel has since approved the bounded rules-investigation investment, confirmed
appetite for complete Gathering and offered no further scope correction. The
following is the original review procedure, retained as context rather than a
renewed request to approve those same decisions. The current handoff owns the
remaining acceptance boundary.

Open this report at the commit named in the issue's current handoff. No application
launch is needed or authorized for this assessment. In the local checkout:

```sh
git show HEAD:docs/research/issue-89-gathering-assessment.md | less
```

1. Read “Decision in brief” and this effort/disposition section. Expected result:
   the next commitment and its limits are understandable without decoding source
   data. Judge the revised source-adjudication proposal and its eight-hour ceiling,
   or narrow/park it; the former random-pool preparation proposal is withdrawn.
2. Inspect the source/preparation deficits and the worked adverse outcomes below
   the decision summary and the encounter inventory in section 3. Expected result:
   starting weaknesses and encounter membership are resolved, while remaining
   rule-oracle gaps stay visible; report acceptance does not certify gameplay.
3. Accept the assessment, request a bounded correction, narrow the next proposal,
   or park. Stop after 20 cumulative minutes or sooner if the decision is unclear;
   a retry consumes the same allowance. Acceptance alone does not activate the
   next operation unless explicitly stated.

Agent work comprises source retrieval/comparison, code and historical-evidence
inspection, independent worked expectations, report review and the repository
checks recorded in the current handoff. No new gameplay, physical-input or owner
acceptance evidence is claimed.
