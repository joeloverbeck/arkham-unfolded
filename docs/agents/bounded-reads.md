# Bounded reads

Use these helpers when your harness truncates tool output, and to search Codex memory. They print verbatim, line-numbered excerpts under one byte budget and refuse rather than silently shorten. Claude Code's Read (with offset and limit) and Grep tools already return such excerpts.

## Codex memory

Codex keeps its memory index in `memories/MEMORY.md` under its home directory. For memory or historical records, scope by repository before searching task keywords. Use bounded literal search for discovery. For a memory index with `applies_to` records, locate this repository's entries first:

```sh
scripts/tools search-context /path/to/MEMORY.md \
  --find 'applies_to: cwd=/home/joeloverbeck/src/arkham-unfolded;' --context 3
```

Use the returned section heading verbatim. Replace `SELECTED GROUP` below with that heading's title; the search stays inside that section, including its children:

```sh
scripts/tools search-context /path/to/MEMORY.md \
  --heading '# Task Group: SELECTED GROUP' --find 'publication' --context 2
```

Choose literal terms for the current task. Read matching passages in context and follow relevant evidence links. Widen the search only when the scoped lookup has no relevant result or the task requires another repository's history.

## Search

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

## Read

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

## Batch

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
