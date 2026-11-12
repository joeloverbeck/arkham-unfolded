---
name: code-review
description: "Review the changes since a fixed point (commit, branch, tag, or merge-base) along two axes: Standards (does the code follow this repo's documented coding standards?) and Spec (does the code match what the originating issue/spec asked for?). Runs both reviews in parallel sub-agents and reports them side by side. Use when the user wants to review a branch, a PR, work-in-progress changes, or asks to \"review since X\"."
---

Two-axis review of a declared candidate against a fixed point:

- **Standards**: does the code conform to this repo's documented coding standards?
- **Spec**: does the code faithfully implement the originating issue / spec?

Both axes run as **parallel sub-agents** so they don't pollute each other's context, then this skill aggregates their findings.

The issue tracker should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

## Process

### 1. Pin the fixed point and candidate

Resolve the supplied fixed point (commit, branch, tag or merge-base) to an immutable commit. For an implementation handoff, use its recorded starting commit. Ask only when neither the request nor the handoff establishes a fixed point.

Select the candidate mode from the request or handoff:

- **Committed review** (branch or PR): `git diff --binary <fixed-point>...HEAD`. This compares the merge-base with `HEAD` and excludes uncommitted work.
- **Working-tree review** (work in progress or implementation before commit): resolve `git merge-base <fixed-point> HEAD`, then use `git diff --binary <merge-base> --` for the combined committed, staged and unstaged changes. Inventory new files with `git ls-files --others --exclude-standard` and include the full contents of those in scope; tracked diffs alone omit them.

Record the base and head identities, diff command, and commit list (`git log <fixed-point>..HEAD --oneline`). Keep unrelated pre-existing edits outside the selected candidate and name those exclusions. Capture the selected diff and in-scope new files in a temporary snapshot without staging or modifying the checkout; preserve file modes and symlink targets where relevant. Both reviewers receive that same snapshot and its content identity. Use the checkout for context, not as a moving replacement for the captured candidate. If reviewed files change, refresh the affected review before relying on it. For a small refresh delta, one reviewer may cover both axes, reporting them under separate Standards and Spec headings. A later delta that changes only wording that refresh's findings require needs no further reviewer: capture it, read its diff against the refreshed snapshot, and rerun the recorded checks. Code changed after a refresh is reviewed again.

For working-tree review, capture every changed path with the Rust helper:

```sh
scripts/tools review-snapshot --base FIXED_POINT --output-dir /tmp/review-v1 --all-changed
```

`--all-changed` includes both sides of renames, deletions and nonignored untracked
files. Name unrelated changed paths with repeatable `--exclude PATH`; exclusions
are retained in the manifest and summary. For a deliberately partial candidate,
explicit repository-relative paths after `--` remain available instead.

The helper captures whole working files, including staged and unstaged edits,
over the merge-base; unselected files retain their base contents. It creates
`base/`, `candidate/`, `changes.patch` and `manifest.json` in a fresh directory
outside the checkout. The manifest records revisions, selected paths, file modes
and SHA-256 identities; the command prints its digest. Treat this capture as
immutable review input. Supply `--previous /tmp/review-v1` with a fresh output
directory for a refresh: it verifies the previous candidate and patch against
their manifest and reports the change count. A refresh refuses omitted paths
that are still changed unless explicitly excluded. Reverted paths may disappear.
Invalid inputs and failed captures
exit nonzero without publishing a snapshot. Use `--help` for supported inputs.

Use the printed manifest digest to read the compact identity and scope summary:

```sh
scripts/tools review-summary --snapshot /tmp/review-v1 \
  --sha256 RECORDED_MANIFEST_SHA256 --max-bytes 6000
```

Keep the snapshot argument and follow each `NEXT` through `END`. The summary
verifies the recorded manifest identity, candidate files and patch, then lists
revisions, digests, selected paths and changes since the preceding snapshot.
The full context-file inventory stays in `manifest.json`; review the captured
files and patch for their contents. A summary is navigation, not a code review
or staged verification; the latter is described below.

After reading the initial complete scope, use `review-summary --delta-only` on a
refresh to inspect changes since its previous snapshot. This still verifies all
candidate files and the patch, and keeps explicit exclusions visible. Preserve
`--delta-only` when following its `NEXT` pages.

Confirm the fixed point resolves and the complete selected candidate is non-empty before spawning reviewers. A new-file-only candidate is valid even when `git diff` is empty.

### 2. Identify the spec source

Look for the originating spec, in this order:

1. The issue or spec supplied in the request or implementation handoff, fetched via the tracker workflow when applicable.
2. Issue references in the commit messages (`#123`, `Closes #45`, GitLab `!67`, etc.), fetched via the workflow in the tracker doc.
3. A spec file under `docs/`, `specs/`, or `.scratch/` matching the branch name or feature.
4. If nothing is found, ask the user where the spec is. If they say there isn't one, the **Spec** sub-agent will skip and report "no spec available".

### 3. Identify the standards sources

Search the repo for every file that documents how code should be written. When `CODING_STANDARDS.md` or `CONTRIBUTING.md` exists, it must be on the list.

On top of whatever the repo documents, the Standards axis always carries the **smell baseline** below: a fixed set of Fowler code smells (_Refactoring_, ch.3) that applies even when a repo documents nothing. Two rules bind it:

- **The repo overrides.** A documented repo standard always wins; where it endorses something the baseline would flag, suppress the smell.
- **Always a judgement call.** Each smell is a labelled heuristic ("possible Feature Envy"), never a hard violation. Like any standard here, skip anything tooling already enforces.

Each smell reads *what it is* → *how to fix*; match it against the diff:

- **Mysterious Name**: a function, variable, or type whose name doesn't reveal what it does or holds. → rename it; if no honest name comes, the design's murky.
- **Duplicated Code**: the same logic shape appears in more than one hunk or file in the change. → extract the shared shape, call it from both.
- **Feature Envy**: a method that reaches into another object's data more than its own. → move the method onto the data it envies.
- **Data Clumps**: the same few fields or params keep travelling together (a type wanting to be born). → bundle them into one type, pass that.
- **Primitive Obsession**: a primitive or string standing in for a domain concept that deserves its own type. → give the concept its own small type.
- **Repeated Switches**: the same `switch`/`if`-cascade on the same type recurs across the change. → replace with polymorphism, or one map both sites share.
- **Shotgun Surgery**: one logical change forces scattered edits across many files in the diff. → gather what changes together into one module.
- **Divergent Change**: one file or module is edited for several unrelated reasons. → split so each module changes for one reason.
- **Speculative Generality**: abstraction, parameters, or hooks added for needs the spec doesn't have. → delete it; inline back until a real need shows.
- **Message Chains**: long `a.b().c().d()` navigation the caller shouldn't depend on. → hide the walk behind one method on the first object.
- **Middle Man**: a class or function that mostly just delegates onward. → cut it, call the real target direct.
- **Refused Bequest**: a subclass or implementer that ignores or overrides most of what it inherits. → drop the inheritance, use composition.

### 4. Spawn both sub-agents in parallel

Issue both sub-agent calls together, in the foreground, and aggregate the reports they return. Start each reviewer with a fresh context (`fork_turns: "none"` when supported). Give each a self-contained brief with the repository location, captured candidate and fixed point, approved scope and requirements, relevant instruction and standards paths, and its review task. The brief and referenced artifacts must supply everything needed without the implementation conversation.

For tabletop UI changes, include the [inherited presentation baseline](../../../docs/agents/native-verification.md#visual-and-wording-review): accepted implementation and revision, affected sections, shared component paths, preserved behavior and authorized differences. For repairs after a native check, also supply the reference capture and constraints retained under that guide, including earlier uncommitted work in the same session. Both reviewers compare the actual consumer paths with these references; importing a shared module alone does not establish preservation. Identify missing native comparison evidence separately from code findings.

Require both reviewers to identify each affected production consumer, including older modes that share changed code. For each claimed regression, cite the fixed-point and candidate paths and explain the behavior before and after the change, with a reproduction or a causal trace through those consumers. Classify a finding as introduced, pre-existing, or unverified. A plausible collision or failure without baseline evidence is an unverified concern, not a confirmed regression; name the missing check and keep it separate from confirmed findings. A pre-existing spec or standards breach may still be reported, with its baseline status explicit.

**Standards sub-agent prompt** should include:

- The captured candidate's location and identity, full diff command and commit list.
- The list of standards-source files you found in step 3, **plus the smell baseline from step 3** pasted in full (the sub-agent has no other access to it).
- The spec's out-of-scope list, when it has one.
- The brief: "Report, per file/hunk where relevant, (a) every place the diff violates a documented standard: cite the standard (file + the rule); and (b) any baseline smell you spot: name it and quote the hunk. Distinguish hard violations from judgement calls: documented-standard breaches can be hard, but baseline smells are always judgement calls, and a documented repo standard overrides the baseline. Where a smell's fix would change out-of-scope code, name that conflict and leave the fix to the spec's owner. Skip anything tooling enforces. Under 400 words."

**Spec sub-agent prompt** should include:

- The captured candidate's location and identity, diff command and commit list.
- The path or fetched contents of the spec.
- The brief: "Report: (a) requirements the spec asked for that are missing or partial; (b) behaviour in the diff that wasn't asked for (scope creep); (c) requirements that look implemented but where the implementation looks wrong. Quote the spec line for each finding. Under 400 words."

If the spec is missing, skip the Spec sub-agent and note this in the final report.

### 5. Aggregate

Present the two reports under `## Standards` and `## Spec` headings, verbatim or lightly cleaned. Do **not** merge or rerank findings, because the two axes are deliberately separate (see _Why two axes_).

End with a one-line summary: total findings per axis, and the worst issue _within each axis_ (if any). Don't pick a single winner across axes: that's the reranking the separation exists to prevent.

### Verify a reviewed candidate before commit

When the authorized implementation later stages its reviewed files for a commit,
verify the index against the snapshot using the manifest digest recorded for the
review:

```sh
scripts/tools review-verify --snapshot /tmp/review-v1 \
  --sha256 RECORDED_MANIFEST_SHA256
```

Require exit 0 before committing. This read-only command checks the recorded
manifest, candidate files and patch, then compares the complete staged tree with
the captured candidate, including contents,
modes, symlink targets, additions, deletions and unreviewed staged paths. It compares
file identities, so snapshot/Git patch prefixes and ordering do not affect equality.
Unstaged working edits are outside this check. Use the recorded digest to preserve
the review binding; recomputing it from changed input would select another candidate.
An identity match establishes neither test success nor human acceptance. Review
alone does not authorize staging, committing or publication.

## Why two axes

A change can pass one axis and fail the other:

- Code that follows every standard but implements the wrong thing → **Standards pass, Spec fail.**
- Code that does exactly what the issue asked but breaks the project's conventions → **Spec pass, Standards fail.**

Reporting them separately stops one axis from masking the other.
