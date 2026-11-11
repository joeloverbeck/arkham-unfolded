---
name: implement
description: "Implement a piece of work based on a spec or set of tickets."
disable-model-invocation: true
---

Implement the work described by the user in the spec or tickets.

Before ticket lookup, startup-document reads or tool discovery, read the repository's [tracker procedure](../../../docs/agents/issue-tracker.md) from client selection through bounded context reads. If the user passes a ticket reference, fetch it and state its title before starting. If the reference is ambiguous, ask.

Before editing, record the entry commit (`git rev-parse HEAD`), working-tree status (`git status --short --untracked-files=all`) and entry time (`date -u +%FT%TZ`). Preserve any pre-existing edits to paths the task will touch so they remain distinguishable from this task's changes. Take every later statement about consumed effort, and every time-based stop, from the time elapsed since that record.

Choose a task branch before editing: reuse an existing topic branch when its commits and any open PR belong to the selected work; otherwise create one at the task's approved fixed point, or the entry commit when none is specified. Preserve the recorded working-tree state and identify any unmerged ancestors before publication so the publication set can be reconciled with accepted scope.

After selecting the branch, record its current commit (`git rev-parse HEAD`) as the review fixed point before making changes.

For tabletop UI work, establish the [inherited presentation baseline](../../../docs/agents/native-verification.md#visual-and-wording-review) before coding, including when adding a scenario or mode. Carry its affected sections, implementation paths and authorized differences into the review brief.

Use the [TDD guide](../tdd/SKILL.md) at pre-agreed seams, selecting the workflow for a behavior change or a refactor that preserves behavior.

Run typechecking and focused tests during implementation. Before capturing a review candidate or starting desktop verification, run `scripts/check-format` and `scripts/dev scripts/check-clippy` through the recorded-check procedure below and require both to pass. These existing checks catch mechanical repairs before review and timed human work. Run the full repository gate once the final candidate is stable; repeat it only after changes or failures invalidate that result.

Before launching checks, read the [local launcher prerequisites and recovery guidance](../../../README.md#local-builds-and-checks). Run checks through `scripts/tools record-check --output-dir <fresh-task-directory> -- <check-command> [arguments...]`; for the full repository gate, the command is `scripts/check`. The parent directory must exist. This retains stdout and stderr in `check.log` and atomically records the invocation, directory, timestamps and result in `run.json`, returning the check's exit code. After interruption or context loss, read these files before repeating an unchanged check: only `status: finished` with `exit_code: 0` establishes command success. A running, interrupted or incomplete record does not. A forcibly killed recorder can leave a running record; do not infer completion from its log tail. Report the status and log path with bounded results or failure excerpts, and keep summaries separate from execution. The record establishes command completion, not that the current working tree is unchanged or semantically accepted.

Human verification is anything the user must observe or perform beyond reading the pull request, such as a desktop check. At the first handoff with human verification still outstanding, give the user a private runnable acceptance checklist and designate the owning issue's [public current handoff](../../../docs/agents/issue-tracker.md#current-handoff). Use the [separate public and private formats](../../../docs/agents/native-verification.md#prepare-a-compact-native-handoff); keep host paths and commands in the private checklist. Distinguish completed checks from observations still needed, and update the remaining checklist as results arrive. Prepare the compact native handoff before a timed desktop check. When only the merge decision remains, the pull request body carries the evidence and no issue handoff is needed. When the spec names records that fall due only on the owner's decision, such as a glossary term on adoption, prepare them as the final commit of the same pull request, labelled for that decision and named in the decision surface, so that one authorization covers them.

Once done, call the Skill tool with "code-review", passing the recorded review fixed point, the originating issue/spec and **working-tree review** as the candidate mode. Identify the task's changed and new files and any pre-existing edits outside its scope. Follow that skill's candidate-capture procedure before committing. Adjudicate each finding before repairing it (`OPS-3`): repair confirmed implementation defects and specified missing cases, and carry unverified concerns into the pull request's stated limits as they were reported.

Stage the reviewed task files without disturbing unrelated staged work. After staging, run `scripts/check-index` through the recorded-check procedure above and require success before committing only the reviewed task files to the selected task branch. This covers new files that were untracked during the full gate.
