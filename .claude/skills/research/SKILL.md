---
name: research
description: Investigate a question against high-trust primary sources and capture the findings as a Markdown file in the repo. Use when the user wants a topic researched, docs or API facts gathered, or reading legwork delegated to a background agent.
---

Spin up a **background agent** to do the research, so you keep working while it reads.

Its job:

1. Investigate the question against **primary sources** (official docs, source code, specs, first-party APIs), not a secondary write-up of them. Follow every claim back to the source that owns it.
2. Write the findings to a single Markdown file, citing each claim's source.
3. Save it where the repo already keeps such notes; match the existing convention, and if there is none, put it somewhere sensible and say where.

For a disputed interpretation or a conclusion that would block work, the findings are complete when they state:

- The best-supported recommendation and strongest competing reading, distinguishing direct source statements from derived conclusions.
- A worked example that distinguishes those readings, with the supported expected result and any part the cited sources leave undecided.
- The exact remaining limitation and its consequence for the current task: implementation, source certainty/fidelity, verification, or a reserved owner judgment.

Before recommending external clarification, explain why the controlling sources, proportionate further investigation and a bounded provisional interpretation cannot resolve the current task. Apply [AGENT-5](../../../docs/principles/principles-for-repos.md#agent-5--delegated-judgment-is-the-default-escalation-is-evidence-bound) and [OPS-1](../../../docs/principles/principles-for-repos.md#ops-1--feasibility-precedes-tdd-and-execution-readiness) to that residual uncertainty. Preparing a question does not authorize sending it.
