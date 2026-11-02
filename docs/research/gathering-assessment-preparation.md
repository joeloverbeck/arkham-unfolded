# Complete Gathering: assessment preparation

Interview support for `grill-with-docs`, 2026-11-02. Baseline:
`bd72f441efebda420f068e4999433a4e157843e7`, matching local and remote main.
GitHub had no open issues or pull requests at the initial inspection. The accepted
assessment was subsequently published as
[issue #89](https://github.com/joeloverbeck/arkham-unfolded/issues/89) on 2026-11-02.
That issue owns active scope and status. This document preserves interview
evidence and is not a second work queue or implementation authorization.

## Selected outcome

Joel accepted this direction during the interview: target a complete original
**The Gathering on Standard**, with Tony Morgan, Patrice Hathaway and Jacqueline
Fine from actual setup, including unsuccessful outcomes; prepare a bounded
rules/content assessment as the next work item.

The full original Night of the Zealot campaign remains the delivery goal. This
scenario is the next proposed usable milestone, not completion of that goal.

## Evidence and alternatives

- [Study escape](../experiments/study-escape.md) and
  [issue 55's acceptance](https://github.com/joeloverbeck/arkham-unfolded/issues/55#issuecomment-6558284237)
  establish accepted connected party play through Act 1 advancement. The
  experiment deliberately ends there or before the enemy phase. It claims
  neither genuine setup nor continuation into a campaign.
- [Issue 73](https://github.com/joeloverbeck/arkham-unfolded/issues/73#issuecomment-6560193466)
  and [issue 77](https://github.com/joeloverbeck/arkham-unfolded/issues/77#issuecomment-6562252036)
  record subsequent owner use and acceptance of presentation refinements.
  Completed architecture work #79, #80 and #81 followed. Further enabling work
  needs a renewed scope and allocation under WORK-3; issue closure and automated
  checks alone do not establish another acceptance-bearing play experience.
- Current Party rules expose Study investigation, draw, resource and end-turn
  choices. Advancement terminates the experiment; validation assumes restricted
  cards, resources, harm and locations. See `src/party.rs` at this baseline and
  the [previous closure argument](next-playable-segment-preparation.md).
  Removing the endpoint would not produce supported ordinary play.
- A prepared post-Study phase would be cheaper and could test movement and
  split-party cooperation. It would remain another limited experiment, leaving
  setup, full rounds, encounters and scenario resolution unproved. Joel selected
  the complete Gathering target instead.
- The [original assessment](issue-1-assessment.md),
  [source reconciliation](issue-1-source-reconciliation.md), and
  [deck evidence](issue-1-inputs.md) are useful inputs. Their historical proposals
  and estimates are not current readiness or measured estimates for Gathering.

Review coverage: all Markdown under `docs/`, README, GLOSSARY, relevant skills,
and targeted production paths were reviewed by the parent and subordinate
read-only reviewers. JSON evidence/asset/configuration documents were inspected
for structure, identities and relevant semantics. Current GitHub status and
selected acceptance records were checked live. No tests or native sessions were
run; this establishes a planning recommendation, not new runtime evidence.

## Accepted next issue contract

Published title: **Bound complete Gathering play from real setup to resolution**.

The assessment must answer what remains to deliver that exact scenario, which
existing behavior can support it, and what first implementation increment and
effort are justified. Extend the existing evidence rather than repeat the
founding assessment or design a general engine.

Return one coherent report and implementation recommendation covering:

1. Exact governing sources, precedence, retrievable evidence custody and source
   changes relevant to a genuinely new campaign. Preserve the accepted starting
   deck identities; investigate revision/taboo differences explicitly rather
   than silently following a moving deck or treating an old taboo as latest.
2. Actual setup and preparation obligations, including the eligible weakness
   pool, quantity/selection rules and supported-content refusal. Identify when
   actual selections must be bound before implementation scope can be final.
   Do not silently choose weaknesses, restrict selection to implemented cards,
   change personal records or commit a campaign during this assessment.
3. A finite behavior map for the selected decks and Gathering: timing and
   player decisions, card interactions, movement, enemies, all phases and round
   limits, Patrice's hand/deck lifecycle, acts/agendas, defeat and legal scenario
   resolutions with their recorded consequences. Classify existing evidence,
   missing behavior and unresolved interpretations. Card counts alone are not
   a scope estimate.
4. Independent worked expectations for the difficult interactions that determine
   scope, and a proposed first product-bearing implementation increment through
   the actual player path. Do not label another isolated demonstration as a
   complete scenario. Any necessary narrowing returns as an explicit proposal.
5. Save/resume, offline play, replay and readable-art-substitute obligations,
   including preservation of existing experimental records without silently
   reinterpreting them as campaign state. Preserve existing accepted tabletop
   conventions and source-archive custody.
6. An evidence-backed estimate separating source/adjudication, implementation,
   verification, presentation and human review burden; a bounded implementation
   proposal with exclusions and stops; and a clear proceed, narrow or park
   recommendation. Explain uncertainty rather than promising delivery within
   the assessment allowance.

The intended implementation test boundary is the existing production player
observation and revision-bound choice path shared by the driver and native UI,
including persistence before installing consequences. Assess its adequacy and
recommend any required local extension for future implementation. Compare observed
decisions and consequences with official-source expectations, not private types
or another implementation's output. Native play and owner judgment remain
separate evidence for usability. This assessment itself is judged by traceable
coverage, worked expectations and a usable implementation decision; it does not
claim to have executed future behavior.

## Accepted allowance and stops

Four aggregate active engineering hours, including delegated research, evidence
checks, code inspection, report writing and review; one owner review of at most
20 minutes, including any retry. No paid services. Joel accepted these prospective
ceilings on 2026-11-02; they are not a full-scenario delivery estimate. This interview's
repository review is prior work and is not charged to that prospective item.

Joel's acceptance renews allocation for one assessment serving the selected
Gathering target, when that work is subsequently published and activated.
It permits no adjacent architecture/tooling lane or automatic follow-up item.
The report returns directly to the owner for the implementation decision; it
does not reset the real-use allocation streak by claiming document completion
is gameplay acceptance.

Return early if evidence cannot support a proportionate implementation proposal,
including the applicable OPS-4 stop after two active hours without an inspectable
decision-bearing result, or at the allowance limit with the best-supported
finding and exact remaining deficit. Preserve source uncertainty; no invented rulings or silent narrowed
fidelity claim. No automatic extension, implementation, asset conversion,
native/desktop session, save migration, commit, push, PR or merge. Existing Rust
tools and read-only source retrieval are sufficient for the assessment.

## Accepted interview decisions and to-spec handoff

Joel accepted Q1 (complete Gathering from actual setup, with a bounded assessment
next) and Q2 (the drafted assessment deliverables, four-hour aggregate ceiling
and one owner review of up to 20 minutes) on 2026-11-02. No interview decision
remains open. Acceptance includes the assessment evidence boundary and proposed
future player-observation/choice verification seam described above.

Joel then explicitly invoked `to-spec` with this document. The self-contained
assessment spec was published as issue #89 with `ready-for-agent`. Its exact
body, title, open state and label were read back and verified; the submitted
body's SHA-256 was
`618b355257d03d90045db32e17fd0fbdd5ad3c74869b65b394318eede715ab9c`.
An independent read-only review found no material mismatch with the accepted
handoff. No assessment execution or gameplay implementation was activated by
publication, and no implementation child tickets were created.

The label describes readiness to perform this bounded assessment, not readiness
to implement the complete scenario. Do not publish a full-scenario implementation issue as
`ready-for-agent` before the assessment establishes its feasibility. Decompose
implementation only after its actual obligation surface is known.

No new glossary term or ADR is warranted: existing prepared-position vocabulary
and ADRs remain applicable; this is a reversible work-selection decision.

Consulted constitution revision 2026-09-30.1: complete text across the review;
in particular AUTH-1–3, PURP-1–6, WORK-1–6, EVID-1–10, SEM-1–2, REP-1–3,
BOUND-1–5, INT-1–4, AGENT-1–5, OPS-1–7, PUB-1, GOV-1–3 and ADOPT-1–4.
The published issue records the relevant consultation. Its body and subsequent
current handoff own active scope and status.
