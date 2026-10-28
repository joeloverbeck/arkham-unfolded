# GitHub administration

Use this reference when changing repository settings or diagnosing a publication refusal in `joeloverbeck/arkham-unfolded`. GitHub owns the current settings; inspect them before proposing or applying a change. This document records the policy selected in the 2026-10-28 response to [PR #23](https://github.com/joeloverbeck/arkham-unfolded/pull/23), not a continuously refreshed settings snapshot.

## Accepted publication and contribution policy

The repository stays public and accepts external PRs. All external contributors require approval before fork workflows run. Repository auto-merge stays disabled. Opening a PR grants neither permission to execute its code nor acceptance of its claims.

Publish accepted changes through a PR to `main`. Main requires an up-to-date `Rust checks` result from GitHub Actions, applies protection to administrators, and blocks force pushes and deletion. Zero approving reviews are required so the solo owner can merge their own PRs. The required check runs [scripts/check](../../scripts/check); changing its name or provider requires reconciling the live protection before publication.

Issue closure is separate from merging: close an issue only after its acceptance criteria and owner acceptance are satisfied. Joel confirmed disabling **Auto-close issues with merged linked pull requests** under **Settings → General → Issues** on 2026-10-28. That confirmation came from the owner; the APIs inspected during this work did not expose the toggle. [GitHub documents this setting](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/managing-auto-closing-issues).

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
