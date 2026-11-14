# GitHub administration

Use the applicable sections of this reference when monitoring CI, merging an accepted PR, synchronizing after merge, changing repository settings or diagnosing a publication refusal in `joeloverbeck/arkham-unfolded`. GitHub owns the current settings; inspect them before proposing or applying a change. This document records the policy selected in the 2026-10-28 response to [PR #23](https://github.com/joeloverbeck/arkham-unfolded/pull/23), not a continuously refreshed settings snapshot.

## Accepted publication and contribution policy

The repository stays public and accepts external PRs. All external contributors require approval before fork workflows run. Repository auto-merge stays disabled. Opening a PR grants neither permission to execute its code nor acceptance of its claims.

Publish accepted changes through a PR to `main`. Main requires an up-to-date `Rust checks` result from GitHub Actions, applies protection to administrators, and blocks force pushes and deletion. Zero approving reviews are required so the solo owner can merge their own PRs. The required check runs [scripts/check](../../scripts/check); changing its name or provider requires reconciling the live protection before publication.

Issue closure is separate from merging: close an issue only after its acceptance criteria and owner acceptance are satisfied. Joel confirmed disabling **Auto-close issues with merged linked pull requests** under **Settings → General → Issues** on 2026-10-28. That confirmation came from the owner; the APIs inspected during this work did not expose the toggle. [GitHub documents this setting](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/managing-auto-closing-issues).

## Monitor required checks

Keep the run and job IDs for the exact PR head. Start one watcher for that head, which finds its run and follows it:

```sh
scripts/tools ci-watch --head <head-sha>
```

It waits up to 10 minutes for a `Check` run of that exact head to appear, then
follows the newest such run, polling every 30 seconds, for up to an hour in all;
`--appear-timeout-seconds`, `--interval-seconds` and `--timeout-seconds` change
these. It prints each new run, attempt or status. While a run is unfinished, it
also reads that exact attempt's jobs at the same cadence, reporting changed
active jobs/steps with elapsed watch time. Unchanged progress stays quiet;
completed jobs alone never establish workflow success. It finally prints the
conclusion and the run's head. It exits 0 only when that run succeeded, 1 when
it completed otherwise, 2 when no run appeared or completed in time or GitHub's
answer failed or could not be trusted, and 130 when interrupted. Job responses
must be complete and match the run, attempt and head. For a run already
identified, `gh run watch <run-id> --repo joeloverbeck/arkham-unfolded --compact --exit-status --interval 30`
follows it alone.

If the tool returns a running session, wait on that same session for up to
45 seconds per call, within the required progress-update interval. Use the wait
itself instead of alternating short polls with separate sleeps. Use its output
for intermediate status; keep one watcher. After it exits, read the terminal
conclusion and head (`ci-watch` prints both; after `gh run watch`, read the run's
status and `headSha`) and verify the expected revision before the
[guarded merge](#merge-an-accepted-pr). If the connection is lost, inspect the
run once before reconnecting; losing the watcher does not authorize rerunning
CI. A failed run needs the failure diagnosis required by `OPS-3` before repair
or retry. If watch mode is unavailable or leaves the current phase unclear,
use the compact job-status query:

```sh
gh api repos/joeloverbeck/arkham-unfolded/actions/jobs/<job-id> \
  --jq '{status,conclusion,current:[.steps[] | select(.status == "in_progress") | .name]}'
```

Use the phase and elapsed wait in progress updates. Before merging, recheck the PR head and base and the required check's provider and conclusion. A monitoring result does not authorize a different commit or bypass protection.

Fetch detailed logs only to diagnose a failed check or unresolved status, or to satisfy an explicit audit request; the CLI may refuse logs while a job is running. Bound each retrieval to 30 seconds, for example `timeout 30s gh run view <run-id> --repo joeloverbeck/arkham-unfolded --log`. Report unavailable logs and any question they leave unresolved. When the required checks are verified successful for the exact PR head and logs are optional, unavailable logs do not block publication.

If repeated observations show no progress, and before reporting publication blocked, reconcile fresh observations of the PR head's check runs with the latest workflow run and attempt. A retry can replace the job being monitored. Read the selected run's `head_sha` and `run_attempt`, then obtain its current jobs; confirm that the required check belongs to that same head and expected provider. The relevant endpoints are `commits/<head-sha>/check-runs`, `actions/runs/<run-id>` and `actions/runs/<run-id>/attempts/<attempt>/jobs` in this repository.

Retain the local UTC observation time and the returned status timestamps. For a stalled or contradictory response, `gh api --include` also exposes the HTTP `Date` and request ID; save that diagnostic output locally and keep routine polling compact. If the observations disagree, report the disagreement and refresh the affected records before deciding whether the PR can merge. A service incident is supporting context, not evidence that this particular PR is blocked. Preserve protection and the exact-head check when the uncertainty cannot be resolved.

## Merge an accepted PR

After the exact changes have been reviewed, accepted and authorized for publication, use the repository's Rust helper with the full accepted head SHA and reviewed `main` tip:

```sh
scripts/tools merge-reviewed --pr <number> --head <accepted-head-sha> --base <reviewed-main-sha>
```

The helper requires an open, non-draft PR targeting `main`, matching head and base, `CLEAN` merge state, and exactly one successful, completed `Rust checks` result from GitHub Actions app `15368` on that head. It reads all [check-run pages](https://docs.github.com/en/rest/checks/runs#list-check-runs-for-a-git-reference) using the API's `latest` filter, rejects incomplete or ambiguous responses, and refreshes the PR after checking CI. Only then does it invoke [`gh pr merge --merge --match-head-commit`](https://cli.github.com/manual/gh_pr_merge), followed by a readback of the merged head and merge commit. Every `gh` call has a 30-second limit; prerequisite errors prevent merge, and merge is never retried automatically.

This command enforces the mechanical prerequisites; it does not grant acceptance or publication authority. GitHub's strict branch protection remains the final gate for changes racing the last read: `--match-head-commit` binds the head, but GitHub CLI supplies no atomic expected-base argument. A failed merge or readback requires inspecting the PR before another attempt. After a confirmed merge, follow [checkout synchronization](#synchronize-after-merge).

## Synchronize after merge

After verifying that the accepted PR merged, use failure propagation for the authorized local update:

```sh
git switch main &&
git pull --ff-only origin main
```

When the owner's checkout is on another branch or has uncommitted changes, another session may be using it: leave it as it is and update only the local `main` with `git fetch origin main:main`. Git refuses that update when `main` is checked out in a worktree, or when the local `main` cannot fast-forward; leave either case for the owner and report the refusal.

Verify the resulting branch, commit and working-tree status before reporting synchronization complete. Keep human decisions between separate command sequences; `&&` supplies failure propagation, not acceptance. This repository's `delete_branch_on_merge` setting deletes a merged PR's head branch on GitHub, so only the local copy remains: remove it with `git branch -d BRANCH`, which refuses an unmerged branch.

## Inspect before changing settings

Follow the target verification and write/readback conventions in [issue-tracker.md](issue-tracker.md). These commands read state:

```sh
gh api repos/joeloverbeck/arkham-unfolded --jq '{private,has_pull_requests,pull_request_creation_policy,allow_auto_merge}'
gh api repos/joeloverbeck/arkham-unfolded/actions/permissions/fork-pr-contributor-approval
gh api repos/joeloverbeck/arkham-unfolded/branches/main/protection
gh api repos/joeloverbeck/arkham-unfolded/rulesets
```

Inspect rulesets as well as classic branch protection. Preserve the before-state, accepted change and recovery plan in the active issue or PR. A permission failure is not evidence that protection is absent. Resolve an ambiguous write by reading state before retrying.

## Verified personal-repository request

[main-protection.example.json](main-protection.example.json) is the request successfully applied and independently read back on 2026-10-28. It is a replacement request, not a patch or an instruction to overwrite later settings. Compare it with live state and the approved change first. The check name and GitHub Actions app ID `15368` were verified against an actual successful check run.

For an authorized application of that exact policy:

```sh
gh api --method PUT repos/joeloverbeck/arkham-unfolded/branches/main/protection \
  --input docs/agents/main-protection.example.json
gh api repos/joeloverbeck/arkham-unfolded/branches/main/protection
```

Two request-shape details matter:

- Supply `required_status_checks.checks` without also supplying `contexts`; GitHub rejected the combination. Readback includes both fields, so do not replay a GET response as a PUT request.
- Omit organization-only user/team restriction objects for this personal repository, including empty bypass allowances. Keep `restrictions` null. A present `required_pull_request_reviews` object with count zero retains the PR requirement without requiring another reviewer.

For fork approval, the verified endpoint is `PUT /repos/joeloverbeck/arkham-unfolded/actions/permissions/fork-pr-contributor-approval` with `{"approval_policy":"all_external_contributors"}`. Read the same endpoint afterward. [Branch-protection API](https://docs.github.com/en/rest/branches/branch-protection#update-branch-protection); [fork-approval API](https://docs.github.com/en/rest/actions/permissions#set-fork-pr-contributor-approval-permissions-for-a-repository).

## Recovery

Restore the recorded before-state only within the approved recovery scope and after checking for intervening changes. Before the 2026-10-28 change, main had no protection or rulesets and fork approval was `first_time_contributors`; those are historical rollback facts, not desired defaults. Removing newly added protection restores the old state only while that remains the complete intended reversal. A direct-push refusal under the accepted policy calls for PR publication, not automatic removal of protection.

The policy applies `AUTH-2`/`AUTH-3`, `EVID-4`, `AGENT-5`, `OPS-5`–`OPS-7` and `PUB-1` from the [constitution](../principles/principles-for-repos.md). No settings-management service or automatic configuration reconciliation is required.
