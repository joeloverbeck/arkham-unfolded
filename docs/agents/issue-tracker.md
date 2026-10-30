# Issue tracker: GitHub

Issues and specs live in [joeloverbeck/arkham-unfolded](https://github.com/joeloverbeck/arkham-unfolded/issues). GitHub Issues and pull requests are the sole active work-control plane. Local documents support those work items.

Use the `gh` CLI from this clone. Before the first write, verify the target with `gh repo view --json nameWithOwner`; use `--repo joeloverbeck/arkham-unfolded` when the target could be ambiguous. File work only in this repository.

This selects the GitHub client for repository work. If a different operation needs tool discovery, filter for that operation and inspect a small set of names before requesting descriptions.

## Conventions

- **Create:** `gh issue create --title "..." --body-file /tmp/issue.md`.
- **Read:** `gh issue view <number> --json number,title,state,body,labels`. Read relevant comments separately as below.
- **List:** `gh issue list --state open --limit 100 --json number,title,state,labels`, with appropriate label and state filters. Inspect selected issue bodies separately; paginate when the task requires every matching issue.
- **Comment:** `gh issue comment <number> --body-file /tmp/comment.md`.
- **Labels:** `gh issue edit <number> --add-label "..." --remove-label "..."`, using the mapping in `triage-labels.md`.
- **Close:** post the explanation with `gh issue comment`, then use `gh issue close <number>`. Comment separately even when the issue is already closed.

Write multiline bodies to a temporary file and pass `--body-file`. Read back the affected issue, comment, or label after every write. If a write times out or its result is unclear, inspect remote state before retrying.

Publishing an issue or applying a readiness label follows the authority and feasibility rules in `CLAUDE.md` and the constitution. A draft spec or a closed dependency alone does not activate work.

Before changing GitHub settings or diagnosing a publication refusal, read [GitHub administration](github-administration.md) for the accepted contribution/publication policy, the verified personal-repository request and recovery procedure. Read live settings as the authority; merge and issue acceptance remain separate decisions.

## Bounded context reads

Start with issue metadata and the current body. Discover comments in small pages, then fetch the bodies needed to understand the decisions and current authority:

```sh
gh api --method GET repos/joeloverbeck/arkham-unfolded/issues/<number>/comments \
  -f per_page=20 -f page=1 --jq '.[] | {id,updated_at,html_url}'
gh api repos/joeloverbeck/arkham-unfolded/issues/comments/<comment-id> --jq .body
```

Follow linked decisions and continue pages as needed for the requested coverage. An index locates evidence; it does not replace reading relevant comment bodies. Read back each write using its returned issue or comment identity.

For long files, locate headings or rule identifiers with `rg -n`, then read the required inclusive range through the bounded reader:

```sh
scripts/tools read-context docs/principles/principles-for-repos.md --start 130 --end 173
```

The reader prints verbatim source lines with number prefixes and caps the whole page at 6,000 UTF-8 bytes, including metadata. `NEXT` gives the exact arguments for the next page; rerun the same path with those arguments until `END`. The source hash rejects a continuation after the file changes. Preserve the required sections and their surrounding context; page boundaries do not reduce mandatory reading.

Use the same reader on saved issue bodies (`gh api ... --jq .body > /tmp/issue-body.md`). `--max-bytes` accepts 512–12,000; a line that cannot fit is explicitly refused, never shortened or skipped. Budget the combined output of a batch against the outer tool limit as well. If the outer tool truncates a page, lower the byte limit and reread that page before continuing. The helper reads UTF-8 files only and never modifies them.

For several excerpts, put 1–32 requests in a JSON file and use one shared page budget. For example, `/tmp/context-requests.json`:

```json
[
  {"path":"docs/principles/principles-for-repos.md","start":130,"end":173},
  {"path":"docs/agents/issue-tracker.md","start":24,"end":44}
]
```

```sh
scripts/tools read-context-batch /tmp/context-requests.json --max-bytes 6000
```

Paths are relative to the current working directory; omitted ranges select the entire file. The byte limit covers all excerpts and metadata together. Follow `NEXT` with the same request file and working directory until the final `END`; `END REQUEST` ends only that excerpt. The continuation hash binds the request file and every source, including files not yet printed. Invalid requests fail before any source output. Prefer this command to combining separately bounded pages in one tool response, and leave room for any other output in the outer tool call.

## Sub-issues and dependencies

Use native GitHub relationships with `gh` 2.94 or later:

- **Parent:** `gh issue create --parent <parent> ...` or `gh issue edit <parent> --add-sub-issue <child>`.
- **Blocker:** `gh issue create --blocked-by <blocker> ...` or `gh issue edit <child> --add-blocked-by <blocker>`.
- **Open blocker count:** `gh issue view <number> --json blockedBy --jq '[.blockedBy.nodes[] | select(.state == "OPEN")] | length'`.

If the CLI lacks these flags, use the native REST endpoints with numeric issue database IDs: `POST repos/joeloverbeck/arkham-unfolded/issues/<parent>/sub_issues` with `sub_issue_id`, or `POST repos/joeloverbeck/arkham-unfolded/issues/<child>/dependencies/blocked_by` with `issue_id`. Obtain a database ID with `gh api repos/joeloverbeck/arkham-unfolded/issues/<number> --jq .id`.

Where native relationships are unavailable, put `Part of #<parent>` and `Blocked by: #<number>` at the top of the child body and maintain the parent's child task list. Check blocker states before selecting work.

## Pull requests as a triage surface

**PRs as a request surface: no.**

GitHub shares issue and PR numbers. Resolve a possible PR with `gh pr view <number>` and fall back to `gh issue view <number>`.

## Skill instructions

When a skill says "publish to the issue tracker", create a GitHub issue within the accepted scope. When it says "fetch the relevant ticket", read that GitHub issue.

## Wayfinding operations

When wayfinding is explicitly selected, use the same GitHub work-control plane:

- **Map:** one issue labelled `wayfinder:map`, with Notes, Decisions-so-far, and Fog sections.
- **Child:** a native sub-issue, or the parent/task-list fallback above, labelled `wayfinder:research`, `wayfinder:prototype`, `wayfinder:grilling`, or `wayfinder:task`.
- **Frontier:** the first open child in map order with no open blocker or assignee, within the accepted scope.
- **Claim:** `gh issue edit <number> --add-assignee @me`.
- **Resolve:** comment with the result, close the child, and add a concise result and link to the map's Decisions-so-far.

Create wayfinding labels only when that selected work requires them.
