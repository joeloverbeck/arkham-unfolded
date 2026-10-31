---
name: implement
description: "Implement a piece of work based on a spec or set of tickets."
disable-model-invocation: true
---

Implement the work described by the user in the spec or tickets.

Before ticket lookup, startup-document reads or tool discovery, read the repository's [tracker procedure](../../../docs/agents/issue-tracker.md) from client selection through bounded context reads. If the user passes a ticket reference, fetch it and state its title before starting. If the reference is ambiguous, ask.

Before editing, record the entry commit (`git rev-parse HEAD`) and working-tree status (`git status --short --untracked-files=all`). Preserve any pre-existing edits to paths the task will touch so they remain distinguishable from this task's changes.

Choose a task branch before editing: reuse an existing topic branch when its commits and any open PR belong to the selected work; otherwise create one at the task's approved fixed point, or the entry commit when none is specified. Preserve the recorded working-tree state and identify any unmerged ancestors before publication so the publication set can be reconciled with accepted scope.

After selecting the branch, record its current commit (`git rev-parse HEAD`) as the review fixed point before making changes.

Use the [TDD guide](../tdd/SKILL.md) at pre-agreed seams, selecting the workflow for a behavior change or a refactor that preserves behavior.

Run typechecking regularly, single test files regularly, and the full test suite once at the end.

Run checks through `scripts/tools record-check --output-dir <fresh-task-directory> -- <check-command> [arguments...]`; for the full repository gate, the command is `scripts/check`. The parent directory must exist. This retains stdout and stderr in `check.log` and atomically records the invocation, directory, timestamps and result in `run.json`, returning the check's exit code. After interruption or context loss, read these files before repeating an unchanged check: only `status: finished` with `exit_code: 0` establishes command success. A running, interrupted or incomplete record does not. A forcibly killed recorder can leave a running record; do not infer completion from its log tail. Report the status and log path with bounded results or failure excerpts, and keep summaries separate from execution. The record establishes command completion, not that the current working tree is unchanged or semantically accepted.

At the first handoff with human verification still outstanding, give the user a runnable acceptance checklist and record it on the owning issue. Include the exact candidate and launch command, only the remaining checks, each expected visible result, and the time limit and stopping condition. Distinguish completed agent checks from observations the human still needs to make. Keep the checklist current as results arrive; accepted checks need repeating only when a later change affects them.

Once done, call the Skill tool with "code-review", passing the recorded review fixed point, the originating issue/spec and **working-tree review** as the candidate mode. Identify the task's changed and new files and any pre-existing edits outside its scope. Follow that skill's candidate-capture procedure before committing.

Commit the reviewed work to the selected task branch.
