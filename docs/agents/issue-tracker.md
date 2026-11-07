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

When preparing a publication artifact, require a completed exit code of `0` from each producer before consuming its output. For an already-authorized sequence with no intervening decision, put the dependent commands in one shell invocation joined by `&&`, including the final mutation. Across separate tool calls, inspect the returned status before issuing the next call: a running session must reach a successful terminal result first. Awaiting a tool response alone does not establish success. Verify the completed artifact against the intended content or recorded candidate digest before the dependent GitHub write. On failure, stop that sequence, retain the error, and inspect remote state before retrying an ambiguous write.

For an exact comment-body readback followed by an authorized issue closure, replace `COMMENT_ID` with the returned comment ID and `ISSUE_NUMBER` with the selected issue number. Save the response, compare its decoded `body` with the submitted file, and close only after it matches:

```sh
gh api repos/joeloverbeck/arkham-unfolded/issues/comments/COMMENT_ID > /tmp/comment-readback.json &&
jq -e --rawfile expected /tmp/comment.md '.body == $expected' /tmp/comment-readback.json &&
gh issue close ISSUE_NUMBER --repo joeloverbeck/arkham-unfolded
```

Equality exits successfully; a failed read or mismatch prevents closure. This preserves body whitespace while avoiding the extra output newline from `gh api --jq .body`. Read back the issue state after closure. For a readback without a dependent write, end the sequence after `jq`.

For an authorized PR, identify the workflow run for the accepted head once, then
keep one watcher:

```sh
gh run watch RUN_ID --repo joeloverbeck/arkham-unfolded --compact --exit-status --interval 30
```

If the tool returns a running session, wait on that same session for up to
45 seconds per call, within the required progress-update interval. Use the wait
itself instead of alternating short polls with separate sleeps. Use its output
for intermediate status; a second watcher or repeated `gh run view` calls add no evidence while
it remains connected. After it exits, read the terminal run status and `headSha`
and verify the expected revision before the guarded merge. If the connection is
lost, inspect the run once before reconnecting; losing the watcher does not
authorize rerunning CI. A failed run needs the failure diagnosis required by
`OPS-3` before any repair or retry.

After verifying that the accepted PR merged, use the same failure propagation for the authorized local update:

```sh
git switch main &&
git pull --ff-only origin main
```

Verify the resulting branch, commit and working-tree status before reporting synchronization complete. Keep human decisions between separate command sequences; `&&` supplies failure propagation, not acceptance.

Publishing an issue or applying a readiness label follows the authority and feasibility rules in `CLAUDE.md` and the constitution. A draft spec or a closed dependency alone does not activate work.

Before changing GitHub settings or diagnosing a publication refusal, read [GitHub administration](github-administration.md) for the accepted contribution/publication policy, the verified personal-repository request and recovery procedure. Read live settings as the authority; merge and issue acceptance remain separate decisions.

## Current handoff

Designate one issue comment as the current handoff and retain its URL. It owns the current candidate, completed checks, remaining acceptance checklist, next acceptance boundary and publication status. Read that comment before updating it, edit it by identity, and verify the exact body afterward. Reuse it as results and owner decisions arrive; carry forward accepted checks unless a later change invalidates them.

Keep that comment compact: state the candidate, summarize verification, list only remaining decisions, and link the accepted brief and dated evidence. Historical bodies belong behind links; collapsed `<details>` still sends the full text to API readers. If the current comment is the only copy of the accepted brief, preserve its exact text once in a dated comment during the authorized handoff update. Verify that archive by its returned identity before replacing the original with the compact handoff and archive link. Reuse an existing preserved brief rather than posting another copy. Leave dated records unchanged; later decisions receive their own linked records.

Keep review results, native observations and authorization decisions as dated evidence records linked from the current handoff. Link each new record back to that handoff. Those records describe what happened at that revision and its evidence limits; current pending/completed status belongs in the handoff. Preserve the distinction between an owner's acceptance and observations the agent did not perform. New evidence or an owner decision updates the single current checklist without synchronizing copies of it across earlier comments. Before relying on local evidence in that handoff, follow [final evidence retention](../experiments/jacqueline-curse.md#retain-final-verification-evidence) and link the retained location.

## Bounded context reads

Start with issue metadata and the current body. Discover comments in small pages, then fetch the bodies needed to understand the decisions and current authority:

```sh
gh api --method GET repos/joeloverbeck/arkham-unfolded/issues/<number>/comments \
  -f per_page=20 -f page=1 --jq '.[] | {id,updated_at,html_url}'
gh api repos/joeloverbeck/arkham-unfolded/issues/comments/<comment-id> --jq .body
```

Follow linked decisions and continue pages as needed for the requested coverage. An index locates evidence; it does not replace reading relevant comment bodies. Read back each write using its returned issue or comment identity.

For memory or historical records, scope by repository before searching task keywords. Use bounded literal search for discovery. For a memory index with `applies_to` records, locate this repository's entries first:

```sh
scripts/tools search-context /path/to/MEMORY.md \
  --find 'applies_to: cwd=/home/joeloverbeck/src/arkham-unfolded;' --context 3
```

Use the returned section heading verbatim. Replace `SELECTED GROUP` below with that heading's title; the search stays inside that section, including its children:

```sh
scripts/tools search-context /path/to/MEMORY.md \
  --heading '# Task Group: SELECTED GROUP' --find 'publication' --context 2
```

Choose literal terms for the current task. Read matching passages in context and follow relevant evidence links. Widen the search only when the scoped lookup has no relevant result or the task requires another repository's history. For structured records such as saves, inspect keys/counts first and select the fields needed for the current question; retain the full record on disk.

`search-context` accepts 1–32 explicit file paths and one or more nonempty,
case-sensitive `--find` strings. A line matching any term appears once. One
shared byte budget covers every file and all metadata:

```sh
scripts/tools search-context tools/src/native.rs tools/src/interaction.rs \
  --find 'deadline' --find 'scroll' --context 3 --max-bytes 6000
```

The optional inclusive `--start`/`--end` range and exact `--heading` line apply
to each file. The command recognizes `#`–`######` headings outside
backtick/tilde fenced blocks; a section ends at the next heading of the same or
higher level. Setext headings are not section selectors. Missing or ambiguous
headings fail explicitly; narrow the line range to select a repeated heading.
`--context N` adds neighboring lines clipped to the selected scope and merges
overlapping excerpts. For different ranges per file, use `read-context-batch`.

The complete search output defaults to 6,000 bytes, adjustable with `--max-bytes` (512–12,000). Whole source lines and their original numbers are preserved; `...` marks omitted gaps. `MATCHES` counts matching lines in the whole selected scope. `NO MATCHES` with `END` is a completed empty result. An oversized next line is refused without silently shortening or skipping it.

For a search `NEXT`, repeat the same ordered paths and query/scope options and
replace the previous continuation arguments with all printed arguments,
including `--next-file` for multiple files. Its one-based index and line zero
mean start that file's results; other line numbers name the first unread source
line. The hash binds every full source (including unread files), ordered paths,
terms, heading, line bounds and context size. Changed inputs refuse continuation
before source text. Follow through `END`; `END FILE` finishes only that file.

For a whole file, omit the range; to read from a known line through EOF, supply only `--start`:

```sh
scripts/tools read-context docs/foundation/mission.md
scripts/tools read-context docs/principles/principles-for-repos.md --start 130
```

For selected sections of long files, locate headings or rule identifiers with `search-context`, then read the required inclusive range through the bounded reader. Take an explicit `--end` from discovered line numbers or a line count; a range beyond EOF is refused:

```sh
scripts/tools read-context docs/principles/principles-for-repos.md --start 130 --end 173
```

For an exploratory window, use `--lines N` instead of `--end`: it selects up to N lines from `--start`, stopping at EOF. For example, `scripts/tools read-context src/saved_state.rs --start 120 --lines 80`. The count must be positive, the start must exist (or be 1 for an empty file), and `--lines` and `--end` are mutually exclusive.

The reader prints verbatim source lines with number prefixes and caps the whole page at 6,000 UTF-8 bytes, including metadata. `NEXT` gives the exact arguments for the next page; replace the previous range options, including `--lines`, with those printed arguments and rerun the same path until `END`. A window continuation uses its resolved `--end` so later pages cannot extend the original window. The source hash rejects a continuation after the file changes. Preserve the required sections and their surrounding context; page boundaries do not reduce mandatory reading.

Use the same reader on saved issue bodies (`gh api ... --jq .body > /tmp/issue-body.md`). `--max-bytes` accepts 512–12,000; a line that cannot fit is explicitly refused, never shortened or skipped. Budget the combined output of a batch against the outer tool limit as well. If the outer tool truncates a page, lower the byte limit and reread that page before continuing. The helper reads UTF-8 files only and never modifies them.

For several excerpts, including saved check logs and decoded GitHub bodies, put 1–32 requests in a JSON file and use one shared page budget. Do this before issuing the reads; several individually bounded calls in one response can still exceed the outer limit. Use `scripts/tools read-context-batch --help` when uncertain about arguments. For example, `/tmp/context-requests.json`:

```json
[
  {"path":"docs/principles/principles-for-repos.md","start":130,"end":173},
  {"path":"docs/agents/issue-tracker.md","start":24,"lines":21}
]
```

```sh
scripts/tools read-context-batch /tmp/context-requests.json --max-bytes 6000
```

Paths are relative to the current working directory; omitted ranges select the entire file. Each request can use either an inclusive `end` or a positive `lines` window, with the same EOF behavior as an individual read. The byte limit covers all excerpts and metadata together. Follow `NEXT` with the same request file and working directory until the final `END`; `END REQUEST` ends only that excerpt. The continuation hash binds the request file and every source, including files not yet printed. Invalid requests fail before any source output. Prefer this command to combining separately bounded pages in one tool response, and leave room for any other output in the outer tool call.

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
