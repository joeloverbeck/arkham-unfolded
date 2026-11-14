---
name: grill-with-docs
description: A relentless design interview that records resolved terms and qualifying architectural decisions.
disable-model-invocation: true
---

Start with [bounded reads](../../../docs/agents/bounded-reads.md) before gathering repository context, memory or session history. It owns source selection and the shared reading budget.

Read and apply the [grilling](../grilling/SKILL.md) and [domain-modeling](../domain-modeling/SKILL.md) guides together. Native skill invocation may load them when available; otherwise read the linked files directly.

Record each resolved result in its appropriate home under the
[document ownership guide](../../../docs/agents/domain.md#document-ownership).
An interview does not require a new ADR or an addition to an existing one.
