---
name: implement-spec
description: "Implement the result of /to-spec and /to-tickets in code."
disable-model-invocation: true
---

You have been provided a spec. This spec should have tickets associated with it, describing how to implement the spec.

The issue tracker should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

The goal is the entire spec implemented on a single **integration branch**, with every ticket resolved the way the issue tracker closes work.

The tickets are not a list of steps. They are a **task graph** with blocking relationships between them. This means there is always a **frontier** of tickets which are ready to be grabbed.

Communication to and from subagents should be sparse. Communicate primarily through **context pointers**: to the spec, tickets, research notes, and previous commits. Don't duplicate information already available via pointers.

**Implementer subagents** should be run in the background where possible for maximum concurrency.

## Steps

1. Read the spec and tickets to understand the task graph. A ticket that verifies the finished candidate, such as an agent verification round, also waits for the **frozen candidate** of step 7.

2. (optional) Use an **exploration subagent** to conduct any exploration required by the tickets - relevant codebase files or external documentation. Ensure the exploration subagent can save files - it should save its markdown notes in a directory outside the repo, accessible by all future subagents. This lets **implementer subagents** focus on implementation rather than exploration.

3. Create the integration branch. This repository publishes through a PR to `main` and closes issues separately, so open a draft PR after the first merge in step 5 (a branch with no commits ahead of main can't open one), naming the spec and tickets it implements.

4. Use **implementer subagents** to implement each ticket, each in its own worktree on its own branch. Create each worktree with `scripts/tools worktree add`, which gives it the owner's Cargo target and its own profile value ([one target for several checkouts](../../../docs/agents/measured-and-baseline-checks.md#one-target-for-several-checkouts)). Each implementer subagent:
   - confirms its worktree is based on the integration branch before starting, and resets onto it if not;
   - calls the Skill tool with `tdd` to build the ticket;
   - merges the integration branch tip into its own branch before reporting done;
   - finishes or stops every background command it started, then reports

5. Once an **implementer subagent** completes, bring its work into the integration branch: fast-forward when its branch already contains the integration tip, merge it yourself when it merges cleanly, and use a **merger subagent** for conflicts.

6. If this changes the **frontier** of available tickets, kick off more **implementer subagents** to work on the new tickets. This allows for maximum concurrency.

7. Once all other tickets are complete, call the Skill tool with `code-review` on the integration branch. Fix all issues raised by the code review in a single **implementer subagent**, then review the fix commits with `code-review` as a refresh. The reviewed tip is the **frozen candidate**: run the verifying tickets against it. A change after the freeze, other than step 8's merge of a moved `main`, is either wording their findings require, disclosed in the PR, or a new candidate that goes through the review and the verifying tickets again.

8. Mark the PR ready for review. If `main` has moved, merge it in and rerun the affected checks CI does not run, such as ignored real-table tests. Then publish and close out as [the tracker workflow](../../../docs/agents/issue-tracker.md#conventions) describes: watch CI, merge with `scripts/tools merge-reviewed`, update the local `main`, and close each ticket, then the spec, with a comment once its acceptance criteria are met. A ticket with an unmet criterion stays open and goes in your report. The PR lists the judgement calls made within the delegated scope, each rejectable on its own, and the spec's closeout comment repeats them. On 2026-11-12 Joel authorized these steps for every invocation of this skill that does not say otherwise.

9. Remove every **implementer subagent** worktree with `scripts/tools worktree remove --merged-into <integration branch> DIR...`. Report each one it refuses, with its reason, and leave those for the owner.
