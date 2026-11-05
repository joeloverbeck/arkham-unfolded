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

For tabletop UI work, establish the [inherited presentation baseline](../../../docs/experiments/jacqueline-curse.md#visual-and-wording-review) before coding, including when adding a scenario or mode. Carry its affected sections, implementation paths and authorized differences into the review brief.

Use the [TDD guide](../tdd/SKILL.md) at pre-agreed seams, selecting the workflow for a behavior change or a refactor that preserves behavior.

Run typechecking regularly, single test files regularly, and the full test suite once at the end.

Before launching checks, read the [local launcher prerequisites and recovery guidance](../../../README.md#local-builds-and-checks). Run checks through `scripts/tools record-check --output-dir <fresh-task-directory> -- <check-command> [arguments...]`; for the full repository gate, the command is `scripts/check`. The parent directory must exist. This retains stdout and stderr in `check.log` and atomically records the invocation, directory, timestamps and result in `run.json`, returning the check's exit code. After interruption or context loss, read these files before repeating an unchanged check: only `status: finished` with `exit_code: 0` establishes command success. A running, interrupted or incomplete record does not. A forcibly killed recorder can leave a running record; do not infer completion from its log tail. Report the status and log path with bounded results or failure excerpts, and keep summaries separate from execution. The record establishes command completion, not that the current working tree is unchanged or semantically accepted.

At the first handoff with human verification still outstanding, give the user a runnable acceptance checklist and designate the owning issue's [current handoff](../../../docs/agents/issue-tracker.md#current-handoff). Include the exact candidate and launch command, only the remaining checks, each expected visible result, and the time limit and stopping condition. Distinguish completed agent checks from observations the human still needs to make. Update this one checklist as results arrive, linking dated evidence and owner decisions. Before a timed desktop check, prepare the [compact native handoff](../../../docs/experiments/jacqueline-curse.md#prepare-a-compact-native-handoff).

Once done, call the Skill tool with "code-review", passing the recorded review fixed point, the originating issue/spec and **working-tree review** as the candidate mode. Identify the task's changed and new files and any pre-existing edits outside its scope. Follow that skill's candidate-capture procedure before committing.

Stage the reviewed task files without disturbing unrelated staged work. After staging, run `scripts/check-index` through the recorded-check procedure above and require success before committing only the reviewed task files to the selected task branch. This covers new files that were untracked during the full gate.
