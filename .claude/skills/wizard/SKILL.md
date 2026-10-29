---
name: wizard
description: Guide a human through steps only they can perform, using a written procedure or a task-specific Rust helper. Use when provisioning infrastructure, setting up credentials or CI secrets, walking an unfamiliar third-party dashboard, or running a one-off migration or cutover. Don't invoke this for steps the agent can perform itself.
---

# Wizard

A **wizard** walks a human through a bounded manual procedure: which URL to open, what to click, which values to obtain, and where each value belongs. It can configure a third-party service or guide a migration or cutover.

Start with a written sequence. If automation materially helps the authorized task, implement only that task's helper in Rust under the repository's implementation-language contract. Temporary helpers are covered too. Use existing external programs through invocation-only shell commands; do not generate a Bash framework or another authored non-Rust program.

A wizard is ephemeral by default. Keep its outputs and any cleanup within the agreed scope. Commit a repeatable helper only when that authoring path is authorized.

## Process

### 1. Scope the procedure

Work out every manual step the human must take and every value that gets captured along the way. Read the repo first, don't ask cold:

- For setup: `.env`, `.env.example`, `.env.*`, `README`, `docker-compose*`, framework config, and `.github/workflows/*` (every `secrets.*` / `vars.*` reference is a value the wizard must produce).
- For a migration or transition: the current state, the target state, and the irreversible actions between them.

Then show the user the ordered list of stages and the values each produces, and confirm: they may add, drop, or reorder.

**Done when:** every stage is named in order, and for each captured value you know (a) where the human gets it, (b) where it's written (`.env`, a GitHub secret, both, or nowhere; some stages are pure actions), and (c) whether it's secret (hidden entry) or public.

### 2. Map each stage's journey

For each stage, write the precise path a human follows: which URL to open, what to do there, where a value is shown, which variable it fills: e.g. "Dashboard → Developers → API keys → Reveal test key → copy". Where you don't actually know the current UI or the exact command, say so and ask the user or check the docs: never invent steps that may not exist.

**Done when:** every stage traces to concrete instructions a stranger could follow.

### 3. Author the wizard

Write one focused stage per step, in dependency order, with progress and the remaining stages visible. Give the human the URL and precise instructions before asking for its result. A Rust helper must keep secret entry hidden, redact output, and preserve existing configuration. Prefer the service's own credential entry surface when no helper needs the secret.

Name every persistence destination and external write. Request only the values CI actually uses. Honor existing authorization and obtain any still-required approval before irreversible actions. Do not overwrite unrelated configuration or turn a narrow procedure into a reusable framework.

### 4. Verify and hand off

- Format and type-check any Rust helper; exercise its meaningful failure and configuration-preservation behavior without real secrets or external writes.
- Trace every captured value to the destination agreed in step 1. Every CI secret name must match a real `secrets.*` reference.
- Coordinate the human-only stages. Do not claim a static review proves successful setup or run an interactive or irreversible stage without its required authority.
- Give one command or the short procedure to follow. Link an authorized repeatable helper from the appropriate documentation.
