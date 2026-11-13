# Triage labels

Map each canonical skill role to the same GitHub label:

| Canonical role | GitHub label | Meaning |
| --- | --- | --- |
| `needs-triage` | `needs-triage` | Maintainer needs to evaluate this issue |
| `needs-info` | `needs-info` | Waiting on reporter for more information |
| `ready-for-agent` | `ready-for-agent` | Approved bounded work ready for an agent |
| `ready-for-human` | `ready-for-human` | Requires human implementation |
| `wontfix` | `wontfix` | Will not be actioned |

Apply `ready-for-agent` only when the bounded work satisfies `WORK-1` and `OPS-1` in `docs/principles/principles-for-repos.md`, and the `WORK-3` count in [#89's current handoff](https://github.com/joeloverbeck/arkham-unfolded/issues/89#issuecomment-6577345060) permits another item; unattended work also requires `AGENT-4`. A label does not supply missing approval or evidence.

Edit the GitHub label column if the repository later adopts different label names.
