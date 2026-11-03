# Gathering rules investigation and implementation brief

2026-11-02, following assessment commit
`c99951151863313ff3fe1416f05724e077be1628` under
[#89](https://github.com/joeloverbeck/arkham-unfolded/issues/89).
Joel [approved up to eight aggregate engineering hours](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6578621220)
for this investigation and confirmed that complete Gathering remains worth
pursuing. The [current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060)
owns the exact candidate, checks, allowance and acceptance status.

## Result in brief

The broad remaining questions now have concrete source-derived expectations.
The audit covers **68 distinct player-card/investigator identities and all 26
scenario identities**, with **21 additional worked cases**. It preserves the
actual decks, the three fixed basic weaknesses and the independently checked
40-card scenario inventory / 26-card initial encounter deck.

Two narrow interpretations remain qualified: whether an ignored-only token
qualifies Prescient, and exactly when the encounter deck is rebuilt if Agenda 2
finds no Ghoul. The recommendations below are localized, evidence-led proposals;
neither is represented as an explicit legacy publisher ruling. They should not
be hidden inside a claim that every rules question has been definitively settled.

The owner is not being asked to investigate these rules. The useful next
commitment is bounded gameplay implementation against these expectations and
their stated limits, rather than another general card-catalog investigation.
This report supplies that implementation brief; it does not activate coding,
set its allowance, establish runtime correctness, or certify full Gathering.

## What the investigation established

| Earlier gap | Result and worked evidence |
| --- | --- |
| Effective card text | Every selected identity has a disposition in the three notes below. Katarina needs mandatory errata; Dark Prophecy, Voice of Ra and Quick Thinking need taboo-10 text changes. No other selected code has a named-card erratum or taboo change in the inspected sources. General rules/keyword corrections still apply. |
| Dark Future and token chains | It restricts six specified symbols rather than all tokens; additional reveals, legal numeric/curse cancellations and sealing remain distinct. J1–J5 specify nested groups, protected symbols, token custody and non-test reveals. |
| Nested A Test of Will | The level-zero event interrupts a pending revelation and performs its own test. It does not run two skill tests concurrently. P1 includes Violin drawing Vision during that test, failure by zero, Lucky! and the original encounter's continuation. |
| Failure and retry order | P2–P6 preserve failed-test results, retaliation, cleanup and delayed effects before retry; commitments, costs, discarded cards, enemy position and temporary boosts retain their proper lifetimes. |
| Act/agenda/round boundaries | Act 2's **when** opportunity precedes Agenda 3's **at** doom. Spawned Priest counts for that doom; normal advancement waits for mythos. S1–S7 distinguish actual simultaneous choices, Ancient Evils, endings and resets. |
| Alleged round-end Dark Future conflict | Removed: Dark Future, Frozen in Fear, Internal Injury and Chronophobia use their controller's turn boundary. J6 supplies a real own-turn ordering choice with different outcomes. |

Detailed research, including initial conditions, legal alternatives, refusals,
expected consequences and controlling page/section locators:

- [Jacqueline](issue-89-rules-jacqueline.md): 23 identities, J1–J7.
- [Patrice](issue-89-rules-patrice.md): 26 identities, P1–P6.
- [Tony and the scenario](issue-89-rules-scenario-tony.md): 23 Tony identities,
  26 scenario identities, S1–S8.

The player lists overlap in Ward of Protection, Uncage the Soul, Deny Existence
and Spectral Razor: `23 + 26 + 23 − 4 = 68`. Counts establish inventory coverage;
the worked traces and source reasoning establish the particular expectations.
Neither establishes exhaustive interaction coverage or executed gameplay.

## Source reconciliation

Use the original Chapter One Rules Reference, original Night of the Zealot guide,
English text for the original card identities, applicable Legacy FAQ 2.5 errata
and clarifications, the needed investigator inserts, and the pinned taboo-10
overlay. Specific card/expansion rules and absolute prohibitions retain their
normal precedence. Original identities are not replaced by Chapter Two reprints
or optional encounter refractions.

The current Grimoire 1.1 expressly restricts its scope to 2026-and-later content
on p2 and redirects older errata on p41. All selected identities predate 2026.
Its wording can identify an issue or corroborate an interpretation; its existence
does not silently replace the legacy baseline. This matters for its newer
Ignore definition and its more explicit encounter-deck exhaustion timing.

The audit also caught a stale secondary reproduction: the ArkhamDB Premonition
FAQ's old “as if” wording disagrees with current Legacy FAQ §2.10. The publisher
PDF governs. ArkhamDB remains an index/transcription; code or community agreement
does not supply the independent answer. No secondary engine was used as an oracle.

Two additional official inserts close specific source omissions:

| Source | Required meaning | Retained identity |
| --- | --- | --- |
| [Forgotten Age investigator insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc72_forgotten_age_rules_insert-compressed.pdf), p1 | Seal removes a token from the bag and supplies its custody lifecycle; sealing is not cancellation. | 523,314 bytes; SHA-256 `03cafa631baa70080244ae4de93b24229ae7d05cf5311a989efd7a2e2fae9d69` |
| [Hemlock Vale investigator insert](https://cdn.svc.asmodee.net/production-fantasyflightgames/uploads/2026/09/ahc76_rules_insert_web-compressed.pdf), p3 | Replenish respects the Uses value; Sparrow Mask stops at two offerings. | 2,229,575 bytes; SHA-256 `ee8bc907db75241c2ab9d5d043ef08545b15e9eb1f5a5e397e267c6aa7f82b19` |

All nine original retained PDF/API sources still match
[their existing manifest](issue-1-evidence.json). The comparison Grimoire remains
`a0ae9ebd0fc6f1455aedf95dacfd0f19a80ccb832c559e4aafbcfb437d0e9a44`, distinct
from the previously retained bytes with the same 1.1 label. Deck snapshots remain
unchanged; the two owner-selected weakness replacements are separate inputs.
Taboo 10 is the observed source used for this research, not a promise that it will
remain the latest version when a future campaign is actually prepared.

## Two localized interpretation proposals

### Prescient and an ignored-only match

**Recommendation: do not offer Spell recovery when the only token of the named
class was ignored or cancelled.** The concrete witness commits Prescient naming
symbol, uses Olive to reveal skull/0/−1, ignores skull and resolves the two
numbers. The proposed outcome is no recovery. A resolved matching token still
qualifies, and Confidence/Eyes retain their explicit exceptions.

Legacy FAQ §2.5's analogous Recall the Future example supports qualification
through resolved tokens. The concrete Olive witness resolves two tokens, so
that section applies; its all-but-one exception does not exclude the witness.
Its exact scope and the Rod of Carnamagos clarification
prevent turning this into an unsupported universal rule that every ignored token
was never revealed for every purpose. No direct official Prescient answer was
found in the inspected primary sources, and the positive condition is not
expressly an if-and-only-if rule. J7 and its qualification preserve that limit.
The strongest alternative would count the physical ignored reveal and
allow recovery; the legacy delayed-condition example favors the recommendation.
Keep this one eligibility decision explicit and replaceable if stronger evidence
changes it; do not change recorded campaigns silently.

### Agenda 2 when every eligible Ghoul is unavailable

**Recommendation: discard the available encounter deck once, draw no enemy,
finish the effect, then rebuild the empty deck.** Three Minions and Ravenous
Ghoul can be in play while Flesh-Eater and Icy Ghoul are in the victory display.
The initial shuffle cannot retrieve them; set-aside Priest is not a substitute.

Legacy RR pp9–10 support resolving as much of an effect as possible but leave
the within-effect rebuild timing less explicit. Grimoire p10 states the proposed
timing for its own scope. The [Smite the Wicked FAQ](https://arkhamdb.com/card/02007),
marked last updated 2017-06-22, reports an official answer for the directly
analogous discard-until/no-enemy case: finish the failed effect before reshuffling.
This is legacy corroboration in secondary custody, not a newly authenticated
publisher original. S8 documents that distinction. Rebuilding and restarting the same unfulfillable
discard-until operation could never produce a Ghoul; it is not a credible
termination policy. This narrow deferred-rebuild interpretation does not adopt
the Grimoire wholesale or excuse a general failure to find required cards.

These two proposals do not require a human to invent a rule. They remain visible
limits on an unqualified source-fidelity claim. An exact future implementation
proposal must carry them forward as interpretations, with their consequences,
rather than relabel them as explicit publisher facts.

## Implementation brief for the next accepted allowance

The target remains complete original Gathering on Standard for these three decks,
from genuine setup through every legal ending and recorded consequence. The
first inspectable gameplay checkpoint remains a complete real first round and
the following mythos phase, including any legal earlier resolution. It must
support every selected card reachable in that checkpoint; scripted hands and
restricted draws cannot conceal missing support.

Consume these notes as a revision-bound specification for product-specific Rust
code. Keep the existing authority direction:
`observe_for_play → choose(revision, id) → persist candidate → install consequences`.
The existing `party::continuity::commit_choice` clones, applies, saves and only
then installs the candidate. Its current implementation does not yet support
these new cases. Local module and fixture design remains engineering judgment;
no generalized interpreter, new rule language or architecture interview is needed.

1. **Bind real preparation.** Preserve deck revisions, fixed weaknesses, rules
   and taboo identities; apply fresh-campaign permanents/trauma, full deck
   shuffles, opening hands/mulligans and exact scenario destinations. Verify
   the initial encounter multiset, not merely its total. This initial invariant
   is not a rule that the deck must stay at 26 throughout play.
2. **Preserve the current decision and consequence.** Separate a pending encounter
   revelation from a currently executing test. Carry token groups and custody,
   committed cards, chosen targets, original test operations, effects already
   paid/applied, genuine ordering choices, and turn/round limits only as demanded
   by these real cases. Do not flatten every continuation into the current actor's
   ordinary action menu.
3. **Use source-derived adverse expectations.** Exercise P1's cancellation test
   with a newly drawn Vision; J6's opposite own-turn orders; P2/P4's failure,
   retaliation and retry; J3/J4's protected/ignored/sealed token distinctions;
   and S1/S3/S4's advancement and terminal boundaries. Include the two qualified
   interpretation witnesses explicitly. The deterministic fixture is a private
   test facility; the production player receives ordinary hidden information and
   randomness.
4. **Prove persistence through those same decisions.** Save at the pending Lucky!,
   token selection, simultaneous-effect order, clue payment and ending choice.
   Reopen in a fresh process and replay without repeating committed costs,
   draws, attacks, doom or awards. A failed save preserves the live state and
   randomness. Retain the existing experiment readers; do not reinterpret old
   experimental endings as campaign records or migrate personal saves.
5. **Return actual consuming-path evidence.** Tests inspect player observations,
   offered legal choices/refusals and consequences through the production seam.
   Native play later establishes readability, control and pacing; headless
   execution cannot establish those properties. Full Gathering additionally
   requires sustained rounds, all ending branches and actual owner play.

The preceding [assessment](issue-89-gathering-assessment.md) still supplies the
broader behavior map, presentation/continuity contracts and exclusions. Its
158–360-hour planning range is not measured delivery throughput or an activated
budget. This research does not establish a new implementation estimate or spend
its unused allowance on coding. Set the implementation allowance against the
reachable content and retain the existing two-hour inspectable-result and
non-convergence stops. No further general weakness-pool work is required.

## Evidence custody and recovery

The following new evidence is retained under ignored
`local/issue-89-evidence/rules-investigation/`. The source archive remains an
unchanged input. No image or font payload is added to Git.

| Retained file | SHA-256 |
| --- | --- |
| `forgotten-age-investigator.pdf` | `03cafa631baa70080244ae4de93b24229ae7d05cf5311a989efd7a2e2fae9d69` |
| `hemlock-investigator.pdf` | `ee8bc907db75241c2ab9d5d043ef08545b15e9eb1f5a5e397e267c6aa7f82b19` |
| `jacqueline-image-bindings.json` | `fed902b207229b3f4b98a3c86b79e4598c0fb846d90a4534a9461f7f9f93ccde` |
| `patrice-image-bindings.json` | `061109c1d22c6e41b34e64a257239689d112a96a84b24d6d031a24ebbacaa3e6` |
| `tony-image-bindings.jsonl` | `dd1c38ddac39669e9e29ecca49e8f4cd949d0129b3246ca91709ea67de99de86` |
| `jacqueline-face-checksums.txt` | `966aeb46cd91b2f2dac8db441f5eb8673a3410dfe0c1e7b925a6c5eabdd0def7` |
| `patrice-face-checksums.txt` | `feaf3178c2897cd212d8c5211a0a65ec3fe0d0209d912bbb5465ca7c52ec86a7` |
| `tony-image-checksums.txt` | `956f69d1846840fbb0f6f7d9ba06423501094ed9f5046baec569e612bede74b8` |
| `scenario-image-checksums.txt` | `774e6cc45cdee487631f9a36b144835baca4405a16b7095b56cffa0a8b74669e` |

The pre-existing encounter binding remains
`local/issue-89-evidence/encounter-audit/encounter-image-bindings.json`, SHA-256
`635310d89a96652ca851904f52cc551e1eb2bbd5b332460f1509f99bf2f696b0`.
It includes the act/agenda reverses freshly inspected for this investigation.
Researchers visually read the 94 distinct English fronts plus investigator and
relevant act/agenda/location reverses; recorded hashes matched the retained bytes.
The parent also repeated the four face/checksum-file checks successfully.

The archive's provenance limitation remains: these are inspected English faces
for original identities, not authenticated first-printing scans. The checked
Spectral Razor image, for example, bears a later copyright. Image hashes establish
which bytes were used; they do not authenticate the archive's origin. Publisher
PDF inventory evidence remains a separate support for encounter completeness.

No independent backup of the local evidence is claimed. Recover PDFs from the
listed publisher URLs only if their hashes match; recover selected face metadata
from the retained archive/catalog and verify the recorded bindings. If retained
evidence and a matching recoverable copy are both absent, repeat the affected
inspection instead of treating these receipts as continuing proof. Keep older
source bytes and personal records intact when newer documents change.

The issue's dated evidence records contain review and check results for the exact
committed document candidate. Those checks establish source-review findings and
repository command completion, not successful execution of the proposed game
behavior or owner acceptance of the playing experience.
