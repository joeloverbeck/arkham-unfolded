# Complete Gathering assessment

Assessment for [#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89),
2026-11-02, against `a4a50c8a2ddb72cd7a2edf5d074ba5a57311a907`.
This report is a decision proposal. The issue's current handoff owns acceptance,
allowance, checks and publication status. The accompanying
[preparation record](gathering-assessment-preparation.md) preserves the earlier
interview unchanged.

## Decision in brief

**Narrow the next activation to resolving real preparation, then return with the
bound inputs before activating gameplay implementation. Keep complete original
The Gathering on Standard as the selected playable milestone.** The existing
experiments are valuable foundations, but full-scenario implementation is not
ready for an unconditional fixed estimate or activation.

The three accepted deck snapshots and taboo response are unchanged on this
inspection. The difficult work is connected behavior: the full decks can be drawn
from setup, Patrice replaces her hand every upkeep, the party separates and
rejoins, encounter effects interrupt decisions, and unsuccessful outcomes must
produce campaign records. Removing the current endpoint would expose states the
Party implementation expressly rejects.

Two preparation facts cannot be silently filled in. Patrice and Jacqueline have
unresolved basic weaknesses; their actual selections must be retained even if
unsupported. The all-official pool also needs a verified eligibility and quantity
projection, including duplicate prints and special selection restrictions.
Catalog counts alone do not provide that. Neither selection nor campaign creation
was performed in this assessment.

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

The alternative is to commit immediately to full Gathering with unresolved inputs
and a broad contingency. That buys earlier coding but hides content and oracle
risk. Parking is reasonable if the estimated effort below exceeds appetite;
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
the selected older starting-card identities. Chapter Two weaknesses could make
its current rules relevant. Do not adopt it wholesale over the legacy sources,
or exclude newer eligible weaknesses to avoid reconciliation.

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
| [Patrice 5391062](https://arkhamdb.com/api/public/deck/5391062.json) | 0.7; updated 2025-10-07 11:56:14 UTC | 45 / 25; basic weakness unresolved |
| [Jacqueline 5389418](https://arkhamdb.com/api/public/deck/5389418.json) | 0.9; updated 2025-10-07 11:55:52 UTC | 35 / 22; basic weakness unresolved, two Arcane Research permanents |

All complete live deck responses and the [taboo response](https://arkhamdb.com/api/public/taboos/)
match the hashes in `issue-1-evidence.json`. There is no observed slot or metadata
drift. The lists still specify taboo 9; the latest returned is **10**, effective
2026-02-19 (update field 2026-10-12). New preparation must bind 10 if it remains
latest when preparation runs. Quick Thinking `02229`, Dark Prophecy `04032` and
Voice of Ra `60416` have identical relevant overlays in 9 and 10. The complete
[deck projection](issue-1-decks.json) remains the identity inventory: 64 distinct
slot codes including the placeholder, 66 physical identities with investigators.
This is not a behavior-completeness count. Historical descriptions, XP and trauma
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
decks, class choices, legality, signatures and permanents; reconcile the eligible
weakness multiset; resolve and retain actual selections; close reachable behavior
support (including successors); then apply real campaign/scenario setup. A source
mismatch or unsupported behavior must identify the deficit before play, preserving
the last accepted preparation. Reopening must not redraw, charge twice or repeat
trauma. Missing art uses readable substitutes instead of blocking rules support.

**Weakness facts:** Learn to Play p19 selects weaknesses after the other deck
cards. FAQ pp16–17 bases the pool on one copy of each product, retaining supplied
quantities and removing assigned copies from a shared pool. It is not a uniform
draw over names. The Dream-Eaters insert p3 allows Multiplayer-only weaknesses
when there is more than one investigator: one human controlling three qualifies.
Campaign-only weaknesses remain eligible because this is a campaign start.
Class restrictions follow investigator class, not off-class deck access.

The inspected canonical metadata contains 65 basic-weakness identities with
summed quantity 77, excluding `01000`. The live API has 77 subtype records with
summed quantity 107: it additionally exposes eight Revised Core reprints (ten
copies), three 2026-core reprints (five copies), and the 15-count placeholder.
**Neither count is a certified eligible pool or probability distribution.**
Naively summing API rows includes placeholders and duplicate editions; filtering
only canonical subtype fields misses alias records.

Necessary handling includes these concrete cases:

- Exclude `01000`, signatures and non-basic campaign/evolution successors from
  initial draws. Preserve the three Unspeakable Oath subtitles as distinct cards.
- Include Multiplayer-only `06035`–`06038`; apply campaign eligibility to Dark
  Pact, Doomed, Unspeakable Oaths and Offer You Cannot Refuse.
- Apply Scarlet Keys class restrictions: Underprepared `09128` for Patrice,
  Ectoplasmic Horror `09127` for Jacqueline; off-class access grants no exception.
- Reserve Tony's assigned Internal Injury copy. Retain one shared remaining pool
  for later additions, with each recipient's restrictions applied.
- Support selected evolving weaknesses and their reachable successors. FAQ p17
  preserves Dark Pact/Doomed custody after replacement. Offer You Cannot Refuse
  grants XP on acquisition and can reopen preparation; do not silently choose
  upgrades, waive the effect or assert the original starting slots stay sufficient.

**Exact deficit:** a publisher-backed completeness check and a reconciled
edition/reprint quantity policy for this all-official virtual collection are not
complete. Printed restrictions and successor closure have not been visually
verified for every potential draw. Recommended proposal: collapse equivalent
reprints to original identity and count that product's quantities once; preserve
true subtitle variants and genuinely new identities, and expose changed reprint
text as a deliberate version choice. That is a proposed local interpretation of
the virtual collection, not a newly invented official rule or an already accepted
pool. It must be resolved before drawing.

Actual Patrice/Jacqueline selections must then be bound **before finalizing the
playable implementation content set and its cost**. A legal unsupported selection
stays selected and blocks launch. Recovery is implementing it or a separately
accepted change of scope, never redrawing until supported. This assessment did
not select anything, create a campaign, or alter a personal deck/record.

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
| Patrice economy/skills | `03033,04201,01093,02192,07032,01000`: spell cost reduction, failure rewards to tester, max/self-test/base-skill constraints, immediate curse insertion or committer horror; unresolved basic weakness |
| Jacqueline/signatures/preparation | `60401,60402,60403,04109,01000`: same-location optional reveal replacement and per-round limit/Arbiter exception; prohibition on cancel/ignore; end-turn non-test reveal; permanent trauma and later Spell discount; unresolved weakness |
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
Chilling Cold**. Original identities are `01104–01119` and `01159–01168`.
The starting encounter deck has 26 cards: Flesh-Eater and Icy Ghoul one each;
Rats three; Ghoul Minion three; Ravenous Ghoul one; Grasping Hands and Rotting
Remains three each; Frozen in Fear and Dissonant Voices two each; Ancient Evils
three; Crypt Chill and Obscuring Fog two each. This arithmetic checks inventory,
not semantic completeness.

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
| Preparation | [`src/party.rs`](../../src/party.rs) reads full slot inventory | Constructs one chosen hand card, zero resources and fixed three-card draw prefixes; no real setup, weakness selection or full deck support check |
| Cooperative decisions | Party decision tests exercise turn order, commits, Promise, Jacqueline, elders, Lucky! and Act closing | Only one prepared turn per investigator; Party card view has intellect icons only and validation fixes test difficulty 2 with bounded harm/locations |
| Enemy/actions | [`src/tony.rs`](../../src/tony.rs), [Tony tests](../../tests/tony_decisions.rs): one Ghoul, bounty action, opportunity attacks, Internal Injury | No integrated multi-investigator enemies, enemy phase, general allocation or scenario defeat |
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
   weaknesses can add obligations; this example does not choose them.
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

**Remaining source/oracle deficits:** all mapped original-card/errata combinations
have not been adjudicated; actual basic weaknesses are unbound; full traces are
still needed for Dark Future with multi-reveal/seal/replacement chains, nested
A Test of Will, retry/retaliation/after-test ordering, and simultaneous Act 2 /
Agenda 3 / end-round resets. These block an unqualified complete-Gathering
readiness claim. Eyes/Katarina itself has explicit official guidance and is not
an unresolved contradiction. They are bounded rules questions, not a reason to
invent a generic interpreter or ask the owner to do technical research.

The first conditional product-bearing checkpoint and its acceptance boundary are
specified in section 6. Preparation-only refusal is a useful prerequisite result;
it cannot stand in for that connected gameplay checkpoint.

## 5. Continuity, presentation and the implementation seam

The accepted seam is adequate as a **boundary of authority**, not as evidence
that the current Study state model fits a full scenario:

`observe_for_play → choose(revision, id) → persist candidate → install consequences`.

Both [player driver](../../src/bin/driver/party.rs) and
[native UI](../../src/party_tabletop.rs) call
[`party::continuity::commit_choice`](../../src/party/continuity.rs), which clones,
applies the offered choice, saves, then installs. Native animation begins after
success. Retain that ordering. Extend observations locally for preparation,
phase, decision owner, instances/zones, legal targets and allocation/order
choices as demanded by the source cases. No generic rules engine or new test
framework is justified. The implementer can choose module shapes within this
boundary without another architecture interview.

The current [Party decision tests](../../tests/party_decisions.rs) establish
selected cooperative commitments, curse continuation, Lucky!, elders and Act 1
closing. [Continuity tests](../../tests/party_continuity.rs) exercise pending
cancellation, failed saves, competing writers, corrupt/wrong-experiment refusal,
restart archival and recorded replay. [Driver tests](../../tests/party_driver.rs)
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
| Source reconciliation, independent expectations, preparation adjudication | 12–30 | Weakness pool/actual selections; nested timing and effective text |
| Real preparation, rules and selected content | 80–180 | Full deck lifecycle, interruptions, enemy/phase/scenario integration |
| Independent verification and repairs | 45–100 | Adverse branches, refusals, persisted continuations and oracle quality |
| Tabletop and offline continuity integration | 25–60 | Crowded real states, exact pending choices, replay and recovery |
| **Planning total** | **162–370** | No defensible closed upper bound before weakness identities and remaining rules gaps close |

Expect at least factor-of-two uncertainty; the range is an order-of-magnitude
judgment, not a confidence interval. Two selected weaknesses may introduce
materially different continuations. The first real-round checkpoint could
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

**Recommended next bounded proposal, not activated:** allow at most eight
aggregate engineering hours to finish the eligible pool and effective-input
reconciliation, expose the revision-bound preparation result through the
existing player observation/choice boundary, and bind actual selections only
under an explicitly authorized preparation operation. Show unsupported
selections and preserve them without redraw. Return the exact content delta,
source evidence and revised estimate. A refusal is an honest preparation result;
it does not establish playable Gathering. Stop at the first unresolved governing
semantics that cannot be localized, at the effort ceiling, or at the existing
non-convergence tripwires. Do not open implementation-ticket fan-out before this
result. If the preparation cannot close in this bound, return to narrow or park;
do not automatically extend it.

**Conditional gameplay proposal:** after those inputs and an implementation
allowance are accepted, implement original Gathering for this exact party on
Standard, with real setup, every reachable selected card, every phase, all legal
outcomes and recorded consequences. First inspectable checkpoint: the real
first round and following mythos, including legal early resolutions. Use ordinary
randomness for play; private deterministic fixtures may target the worked cases,
but cannot become player abilities or hide unsupported draws. Use the existing
player-observation/choice/persistence seam for TDD; no private-type-only oracle.

The first hostile trace should connect legal opening hands, a cooperative test,
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

### Owner review (20 minutes total, including retry)

Open this report at the commit named in the issue's current handoff. No application
launch is needed or authorized for this assessment. In the local checkout:

```sh
git show HEAD:docs/research/issue-89-gathering-assessment.md | less
```

1. Read “Decision in brief” and this effort/disposition section. Expected result:
   the next commitment and its limits are understandable without decoding source
   data. Choose whether the recommended preparation-first direction is worth its
   proposed eight-hour ceiling, or narrow/park it.
2. Inspect the source/preparation deficits and the worked adverse outcomes below
   the decision summary. Expected result: unknown weakness scope and rules gaps
   remain visible; report acceptance does not certify gameplay or resolve them.
3. Accept the assessment, request a bounded correction, narrow the next proposal,
   or park. Stop after 20 cumulative minutes or sooner if the decision is unclear;
   a retry consumes the same allowance. Acceptance alone does not activate the
   next operation unless explicitly stated.

Agent work comprises source retrieval/comparison, code and historical-evidence
inspection, independent worked expectations, report review and the repository
checks recorded in the current handoff. No new gameplay, physical-input or owner
acceptance evidence is claimed.
