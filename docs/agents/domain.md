# Domain docs

This repository uses a single-context layout: `GLOSSARY.md` at the root and architectural decision records under `docs/adr/`.

## Before exploring

Read `GLOSSARY.md` and the ADRs relevant to the area being explored, when they exist.

If either is absent, proceed silently. Create entries or ADRs only when selected work resolves actual terminology or decisions; setup does not require placeholder documents.

## Document ownership

Use the document that owns the result, including during design interviews:

| Result | Home |
| --- | --- |
| Resolved domain term | `GLOSSARY.md`, as a definition without implementation detail. |
| Architectural decision and essential rationale | `docs/adr/`, after applying the [ADR eligibility tests](../../.claude/skills/domain-modeling/ADR-FORMAT.md#when-to-create-or-amend-an-adr) to new records and additions alike. |
| Current detailed product behavior | Its maintained contract; [tabletop presentation](../tabletop-presentation.md) owns the detailed tabletop conventions. |
| Source investigation and historical observations | Existing research/evidence notes, with source revisions and limits. |
| Scope, activation, allowances, implementation plans, verification, acceptance and publication | The owning GitHub issue/PR and its linked evidence, following the [tracker procedure](issue-tracker.md#current-handoff). |

ADRs explain lasting choices; implementation diaries and work status stay in the
tracker. Link supporting contracts and evidence instead of copying them into the
ADR. When consolidating documents, preserve current accepted obligations and
qualified interpretations, repair incoming links, and pin historical references
to their original revision. Git history can retain superseded text without a
second maintained archive.

## Use the glossary's vocabulary

Use the glossary's terms in issue titles, proposals, hypotheses, and tests. Avoid synonyms it explicitly rejects.

If a needed concept is missing, reconsider invented terminology or identify the gap in the current work item for later domain modeling.

## Flag ADR conflicts

If a proposal contradicts an existing ADR, identify the ADR and explain why reconsidering its decision may be warranted. Keep the accepted decision in force until its replacement is accepted.
