# Arkham Unfolded

Read `docs/foundation/mission.md`, `docs/foundation/play-contract.md`, and `docs/foundation/sources-and-verification.md` before selecting project work. GitHub Issues and pull requests own active work. Consult `README.md` for implemented status and launch instructions.

<!-- BEGIN PRINCIPLES-FOR-REPOS BOOTSTRAP 2026-09-30.1 -->
## Repository constitution

This repository adopts `docs/principles/principles-for-repos.md`, revision **2026-09-30.1**, as binding. `CLAUDE.md` is the real root instruction file and `AGENTS.md` is its symlink. A namespace name below means every rule whose identifier begins with that namespace and a hyphen.

### Mandatory reading

Reasonable doubt activates a reading trigger. Memory, a previous session, an earlier reading for a different decision, and paraphrases do not satisfy it.

- Before founding, materially reorienting, or reconciling a repository; materially revising foundations; or adopting a constitutional revision: read the complete constitution.
- Before accepting or activating work, applying execution-ready or AFK status, opening an adjacent lane, expanding scope, or selecting a coordinated producer–consumer iteration: read `PURP`, `WORK`, `OPS`, and `AGENT-5`.
- Before turning uncertainty into a human request or blocker: read `AUTH-2`, `AUTH-3`, `AGENT-5`, and `OPS-2`.
- Before adopting or expanding a representation, specification, recurrent authoring path, schema, ontology, state model, simulator, corpus, evaluator, semantic gate, migration, generated surface, shared infrastructure, or general capability: read `EVID`, `SEM`, `REP`, `GOV`, and `OPS`.
- Before implementing or accepting a claim about feedback, training, ranking, search, adaptation, optimization, or repeated selection: read `PURP` and `EVID`; also read `SEM` and `REP` when semantic decisions or actions depend on it.
- Before using a test, prototype, review, simulation, score, or experience as grounds for completion, adoption, or expansion: read `EVID-4` and the applicable proof requirements in `EVID-3`, `EVID-6`, and `REP-2`.
- Before promising fidelity to external rules, standards, protocols, legacy behavior, or governed workflows: read `EVID`, `REP`, and `OPS`.
- Before adding or redesigning a capability, pipeline, skill family, repository boundary, or human/agent workflow: read `BOUND`, `AGENT`, and `INT`.
- Before cross-repository import, export, feedback, migration, shared identifiers, private-data mounting, or coordinated execution: read `INT` and `BOUND`; read `AGENT` when execution spans capabilities or repositories.
- Before unattended continuation, handoff, repeated repair or review, or continuation near an appetite or allocation limit: read `AGENT`, `WORK`, and `OPS`.
- Before merge, release, or making production work newly reachable from an authoritative path: read `AUTH` and `PUB`.
- Before a waiver, specialization, adoption reconciliation, identifier change, or amendment: read `ADOPT`.

Name the consulted rule identifiers in the existing authoritative work artifact. Do not create a compliance document. An accepted bounded task may continue without rereading or recording the same consultation until its scope, premise, authority, publication set, or triggered rule family changes.

### Universal authority chain

Human intent → constrained proposal → mechanical or semantic checks → inspectable consequences → human acceptance → authoritative state

Human selection authorizes priority and bounded effort. It does not establish truth, fitness, reusability, demand, or permission for unaccepted dependent work.

### Immediate stop-and-read signals

Stop further discretionary expansion and read the controlling rules when:

- required information, an independent oracle, dependency lifecycle evidence, or a valid no-match outcome is missing (`SEM`, `REP`, `OPS`);
- local completion excludes an essential consumer effect, repeated artifacts leave the actual handoff untested, or another implementation is offered as proof of the intended path (`PURP-3`, `PURP-5`, `EVID-3`, `EVID-4`, `INT-1`);
- experiential, recurrent, autonomous, simulation, search, generality, fidelity, or causal claims outrun their applicable evidence, or burden exceeds appetite (`PURP-2`, `EVID`, `REP`);
- presentation, enthusiasm, evaluator scores, exact replay, or compliance records stand in for a different claim; a composite score conceals a failed requirement; or transformation conceals causal credit (`EVID-4`);
- an unapproved capability, independent pipeline, uncontrolled change fan-out, non-convergence, exhausted appetite, or outstanding terminal decision appears (`BOUND`, `WORK`, `OPS`);
- acceptance requires the human to reconstruct technical work, approval is being used to repair missing evidence, or authority, publication, or governing instructions conflict (`AUTH`, `AGENT-5`, `ADOPT`).

If applicable text cannot be located or read, substantive work is not authorized. Preserve a potentially applicable safeguard while investigating. Escalate only the reserved human remainder under `AGENT-5`; ordinary implementation uncertainty and several reasonable technical options do not themselves require a stop. Authorized evidence-gathering and provisional work remain governed by the applicable rules.
<!-- END PRINCIPLES-FOR-REPOS BOOTSTRAP 2026-09-30.1 -->

## Project orientation

The owner is `joeloverbeck`. This is one English-only, personal-use Arkham Horror: The Card Game application for Ubuntu, implemented in Rust with an intended 3D tabletop. Bevy is the leading presentation candidate, subject to the bounded real-use evidence described in the foundations.

Before adding or changing executable code, tools, tests, prototypes, generated code or skill templates, apply the [implementation-language contract](docs/foundation/mission.md#implementation-language). It also governs code produced from generic skill examples.

The first complete delivery is the original Night of the Zealot campaign on Standard with the three named investigator decks and the content actually selected for their campaign. Consult the play contract for decisions about upgrades, weaknesses, taboo versions, recording, replay, controls, and asset absence.

Card and token artwork, the external resource archive, credentials, local configuration, personal saves, and recordings are outside this repository. Consult the source contract before importing or referencing external material.

## Agent skills

### Issue tracker

Use GitHub Issues and pull requests in `joeloverbeck/arkham-unfolded`. Read `docs/agents/issue-tracker.md` before working with issues or specs.

### Triage labels

Use the five default triage labels. Read `docs/agents/triage-labels.md` before triaging or changing readiness labels.

### Domain docs

Use a single-context layout: root `GLOSSARY.md` and `docs/adr/`. Read `docs/agents/domain.md` before exploring domain terminology or architectural decisions.
