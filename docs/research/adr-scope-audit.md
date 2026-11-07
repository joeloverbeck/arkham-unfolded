# ADR scope audit

Investigation for Joel's `grill-with-docs` request, 2026-11-07, at
`6d47ae988088a28e1ea08090ce63466e1a40a5dc`. Joel accepted the recommended
documentation cleanup on 2026-11-07 with “as you recommend.” The findings below
describe that baseline; the accepted cleanup preserves product decisions and
historical evidence. GitHub remains the active work-control plane; this note
supports the documentation change and grants no publication authority.

## Finding and history

ADR 0002 mixes architectural rationale, detailed presentation requirements,
implementation briefs, verification reports, and authorization history. Its
1,484 lines exceed the other four ADRs combined (265), but the decisive problem
is their different purposes and lifecycles, not an arbitrary length limit.

1. The [first committed version, 23ddc24](https://github.com/joeloverbeck/arkham-unfolded/blob/23ddc24/docs/adr/0002-persistent-table-areas.md#L47-L65),
   dated 2026-10-29, already included implementation activation, engineering
   allowance, verification instructions, publication authority, test counts,
   executable hashes, capture paths, and desktop-input diagnosis. It was 65
   lines; a clean architectural record did not precede this mixed-purpose form.
2. The [next change, 2996f9f](https://github.com/joeloverbeck/arkham-unfolded/commit/2996f9ff4a56b7f72206f3e6969d5e078c9db998),
   added 45 lines on 2026-10-30. The agenda/act and standee refinement repeated
   the implementation-handoff, allowance, verification, and publication pattern.
   Its surviving [handoff and report](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#implementation-handoff)
   even retain exact scales, test totals, executable identities, and elapsed time.
3. `git rev-list --count HEAD -- docs/adr/0002-persistent-table-areas.md`
   reports 26 commits at this baseline. Comparing its original and current
   contents gives 1,423 insertions and four deletions: 65 to 1,484 lines.
4. The original [tracker procedure](https://github.com/joeloverbeck/arkham-unfolded/blob/23ddc24/docs/agents/issue-tracker.md#L1-L5)
   already named GitHub Issues and PRs the sole active work-control plane.
   [eb8500d, 2026-11-02](https://github.com/joeloverbeck/arkham-unfolded/commit/eb8500d2930199dcd78c4bad639db31172236eb0)
   explicitly added the single current handoff and linked dated-evidence rules.
   The ADR continued growing afterward.
5. Comparing `23ddc24` with the baseline finds no changes in the
   [domain-modeling skill](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/.claude/skills/domain-modeling/SKILL.md),
   [ADR format](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/.claude/skills/domain-modeling/ADR-FORMAT.md),
   [grill-with-docs wrapper](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/.claude/skills/grill-with-docs/SKILL.md), or
   [domain guide](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/agents/domain.md). The three-part ADR eligibility test and
   minimal context–decision–why format governed the original version too.

**Derived explanation:** the first interview's ADR became the convenient home
for its whole implementation lifecycle, then later work extended that example.
The wrapper's broad instruction to create docs as the interview proceeds and
the creation-focused wording of the eligibility test leave amendment routing
implicit. Neither requires these dumps. This is a failure to apply the existing
document boundary, with a small instruction gap that can be closed explicitly;
the Git history does not establish an author's private reasoning.

## All five ADRs

Line references below refer to the fixed baseline. Each record has a legitimate
decision worth preserving; cleanup must distinguish that decision from its logs.

| ADR | Assessment | Preserve | Remove from the ADR or relocate |
| --- | --- | --- | --- |
| [0001: prepared assets](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0001-prepared-runtime-assets.md) | Mostly focused; limited incidental detail. | Runtime independence from the source archive, portable prepared assets, provenance/custody, and maintenance trade-off. | Exact experiment payload/count at line 19 and folder-selector mechanics at line 23 can remain in preparation evidence or the existing play contract. Move the prose scope notice outside frontmatter. |
| [0002: persistent table](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md) | Severe scope drift from its first version. | Stable physical areas, camera focus independent of game ownership, the navigation/readability trade-off, and Test versus investigator-owned resolution-tray responsibility. | Detailed current UI conventions need a focused contract; implementation, checks, allowances and publication history belong with work/evidence. Superseded conventions need historical links, not continuing normative prose. |
| [0003: replay retirement](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0003-retire-replay.md) | Valid decision with implementation activation attached. | Complete replay removal, deliberate old-record incompatibility, personal-file custody and preserved live continuity. | Activation at lines 44–60, implementation/status narration at 39–42, and consultation inventory at 62–64. Retain a concise link to the existing investigation and owning issue. |
| [0004: required artwork](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0004-require-artwork-for-3d-play.md) | Valid boundary decision mixed with readiness, implementation and UI specification. | Mandatory loaded artwork, headless independence, save preservation, and why substitutes were retired. | Readiness/interview narration at 17–29, implementation summary at 31–37, detailed recovery UI at 63–88, and consultation inventory at 112–116. Move the scope notice outside frontmatter. |
| [0005: experiment retirement](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0005-retire-experiments-and-select-scenarios.md) | Valid maintenance/startup decision with interview and implementation detail. | Retire unused experiment paths, retain Gathering consumers and save compatibility, and provide scenario entry now. | Module survey at 26–32, Q5/to-spec/readiness narration at 45–53, and consultation inventory at 55–60. Current selection behavior belongs in its maintained contract. |

In 0001, 0002 and 0004, a Markdown paragraph sits between `---` and `status: accepted`.
It is misplaced frontmatter content, independently of the architectural-scope
problem. This observation is a source inspection, not a parser-test result.

## Accepted cleanup and destinations

Keep the five ADR identities and shorten them around their surviving decisions.
Do not convert every presentation preference into another ADR. A new record
would require its own meaningful reversal cost, surprising context and real
trade-off; none is automatically created by this cleanup.

| Material | Destination and limit |
| --- | --- |
| Durable architectural choice, why, alternatives and consequences | Its existing ADR, with a few evidence/supersession links. |
| Accepted, still-current tabletop behavior presently scattered through 0002 | One focused `docs/tabletop-presentation.md`, linked from the play contract. Organize by behavior, retain only current obligations, and carry no work status or dated implementation diary. Do not move the dump into a foundation or the Gathering experiment journal. |
| Current artwork recovery and scenario-entry behavior | Existing play/source contracts as appropriate; avoid copying the same requirements into both an ADR and a second specification. |
| Active scope, authorization, candidate, pending acceptance and publication | Owning GitHub issue/PR and its designated current handoff. For Gathering, the [current #89 handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060) already links dated evidence. |
| Source investigation and rationale supporting detailed behavior | Existing research notes, such as [contextual controls](tabletop-contextual-controls.md), [replay removal](replay-removal.md), [required artwork](required-artwork-for-3d-play.md), and [experiment retirement](experiment-retirement.md). Link instead of copying. |
| Superseded layouts, old execution allowances and historical checks | Existing dated issue/PR records where present, otherwise the [pinned pre-cleanup file](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md). Do not create another archive dump or manufacture present authority from old approvals. |

The strongest competing approach is simply to move the entire ADR into an
archive and begin again. That preserves text cheaply, but leaves current
requirements and superseded instructions entangled. Reorganizing current
behavior while linking existing history gives the next implementer a usable
contract without maintaining a second historical ledger.

For example, the [original physical-hand decision](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#decision)
allows additional rows, then explicitly points to a later one-row correction.
The cleanup should preserve the accepted current arrangement in the presentation
contract, the stable physical-area rationale in the ADR, and the superseded
arrangement through history. A standee-scale test count stays evidence of that
revision; it must not become an architectural requirement.

## Interview specimen for ADR 0002

The following illustrated the scope accepted in the interview. The maintained
[ADR 0002](../adr/0002-persistent-table-areas.md) now owns the concise decision;
[tabletop presentation](../tabletop-presentation.md) owns its detailed behavior.

> Use one persistent 3D scene with a shared scenario area, stable investigator
> areas and a shared test area. Non-test effects use an investigator-associated
> resolution tray, preserving investigator association and nested-effect continuity.
> Investigator hands and in-play components keep their physical homes as the
> camera moves; focusing or inspecting another
> investigator does not change the active investigator or the actor answering a
> pending decision. These presentation areas do not create rules-defined zones.
>
> Stable homes preserve the physical tabletop relationships that a changing
> selected-investigator dock would obscure. Focused views provide readable play
> while the overview provides orientation; the accepted cost is camera travel,
> especially for cross-investigator decisions. Keep the current actor, pending
> decision and test context available during ordinary navigation. A permanent
> split view would reduce the space available to the focused area. Resolution
> trays preserve investigator association and physical continuity while clearing
> unused space. Detailed presentation behavior and verification belong in their linked contracts and work records.

This distills the baseline [Decision](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#decision)
and [Why](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#why), plus the
[resolution-tray decision](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0002-persistent-table-areas.md#test-area-and-resolution-trays), without selecting new behavior.

## Prevention and verification boundary

Amend the existing domain guide and domain-modeling skill/format so the three
eligibility tests apply to additions as well as new ADRs. State the destinations
above explicitly and prohibit using ADRs for implementation diaries, test/run
logs, allowances, readiness or publication status. Keep grill-with-docs subject
to that same boundary. This needs a small prose change, not new tooling or an
automatic ADR count/length gate.

Before accepting a cleanup, compare every removed current obligation with its
destination or an explicit superseding decision; check local links and heading
anchors, frontmatter, and preserved historical references. Link repair includes
recipe provenance and source comments, not only Markdown; historical references
must pin the old revision rather than acquire revised semantics. Preserve the
[qualified Premonition interpretation](issue-89-premonition-followup.md) as qualified, not an exact publisher ruling.
Honor [0003](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0003-retire-replay.md)'s
replay retirement, [0004](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0004-require-artwork-for-3d-play.md)'s artwork
requirement, and [0005](https://github.com/joeloverbeck/arkham-unfolded/blob/6d47ae988088a28e1ea08090ce63466e1a40a5dc/docs/adr/0005-retire-experiments-and-select-scenarios.md)'s
experiment retirement. Do not reactivate historical duties or silently settle
unresolved contradictions. Semantic review must check meaning preservation;
link checks verify navigation. Neither establishes gameplay correctness or native owner acceptance.

Consulted for this investigation: `AUTH-2`–`AUTH-3`, `AGENT-3`, `AGENT-5`,
`EVID-4`, `EVID-8`, `REP-1`, `GOV-2`–`GOV-3` and `OPS-2` in the
[constitution](../principles/principles-for-repos.md). The owner accepted this
documentation boundary; the cleanup changes where existing decisions are
recorded, not their behavior or acceptance evidence.

## Local cleanup verification

The five ADRs now retain their decisions and rationale, with current tabletop
details in the linked contract. Historical research links and the two recipe
provenance references pin the original revision. The recipe is otherwise
identical after excluding its evidence text; Rust changes are two documentation
comments only. No gameplay code, private assets or saves changed.

Independent preservation review covered the four other ADRs and the complete
original ADR 0002 against the new contract. It caught two omissions, now restored:
static hover under Reduced motion and effective on-card commitment badges.
The concise ADR also clarifies Test staging during nested tray effects.
This review checks documentation preservation, not implementation fidelity.

Formatting, tracked-file hygiene, whitespace and new/affected link checks passed.
The domain-modeling skill passes its bundled structural validator. That validator
rejects grill-with-docs' unchanged Claude `disable-model-invocation` key in both
the baseline and edited skill; its explicit-only policy is retained. This is a
validator schema limitation, not evidence of a newly invalid invocation policy.
