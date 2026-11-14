# Measured and baseline checks

These procedures extend the [local builds and checks](../../README.md#local-builds-and-checks) when a task needs resource evidence, a comparison with a baseline revision, or one Cargo target shared by several checkouts. The launcher, lock and recorder rules there still apply.

## Resource measurements

For a retained check with resource measurements, prepare the small helper through `scripts/dev`, then run its recorder directly so that it stays outside the launcher's systemd scope:

```sh
scripts/dev cargo build --locked -p arkham-tools &&
target/debug/arkham-tools record-check --resources \
  --output-dir /absolute/path/to/new-check-record -- scripts/check
```

The output directory must be new and its parent must exist. `run.json` retains the command's status and exit code; `resources.jsonl` samples host memory, CPU counters and pressure plus the child's visible cgroup about once per second. Kernel values retain their original names and units. Each reading exposes unavailable files as errors; `resources.error` reports capture/write failures without changing the command's exit status. Sampling runs inside the recorder, so completion or interruption leaves no separate sampler process. Only a finished record with exit code zero proves check success. Resource samples can miss short spikes or the last counters before a scope disappears; an observed `memory.peak` is not a guaranteed final peak. An abruptly killed recorder leaves incomplete evidence.

Read the samples back with `scripts/tools resource-summary OUTPUT-DIR`. Each row is a window, 10 seconds unless `--window-seconds` says otherwise, timed from the first sample or from `--origin-unix-ms`, such as another record's start. Every figure is a counter difference between consecutive samples, about a second apart; an interval is shared by time among the windows it covers. Each figure after Sampled s has a mean for the window and a max: the highest single interval overlapping the window, so a spike near an edge appears in every row it overlaps. A window's columns are:
- **Sampled s:** how much of it the samples cover. Windows with none are left out.
- **Host CPU:** the busy share of all CPUs.
- **Command CPU:** the CPU used by the recorded command's cgroup, in cores. It counts only intervals whose two samples read the same cgroup. That cgroup is the scope the command ran in, so it can also hold the terminal that started it.
- **CPU pressure:** the share of time in which some task on the host waited for a CPU, from the `some total=` counter.

Host CPU rising while the command's stays level was spent outside its cgroup: by another program, or for the command by the desktop or the kernel.

## Behavior-preserving refactor baselines

When a refactor comparison needs a temporary baseline tree, record its revision and the test/helper files copied onto it.

When the dependency inputs are identical, first require `git diff --exit-code BASE CANDIDATE -- Cargo.toml Cargo.lock .cargo/config.toml rust-toolchain.toml` to exit zero with no differences, and verify the temporary tree retains those inputs. Both builds then consume the same immutable renderer source without a local path override. If any differ, or the baseline predates shared extraction, use the baseline's own dependencies and a separate target directory. Keep the toolchain, features and profile consistent with the comparison being made.

Run from the original checkout so both builds use its launcher lock. Replace the temporary baseline path and test filter with the prepared tree and focused characterizations; use fresh record directories for each attempt:

```sh
scripts/tools record-check --output-dir /tmp/baseline-checks/baseline -- \
  scripts/dev cargo test --locked --offline \
  --manifest-path /tmp/caption-baseline/Cargo.toml \
  --target-dir "$PWD/target" \
  --config 'profile.dev.package.arkham-unfolded-experiment.codegen-units=250' \
  --features tabletop --lib gathering_tabletop::tests::cards \
  --message-format=json-render-diagnostics
```

The `--config` line gives the baseline's own package separate artifacts while its dependencies stay shared. Without it, the candidate run can report that package fresh and test the baseline's build ([one target for several checkouts](#one-target-for-several-checkouts)). A codegen-unit count changes code generation, not tested behavior.

Require a finished, successful baseline record before returning to the candidate. Run the same command with a fresh output directory, without `--manifest-path` and without the `--config` line, to check the original checkout. Inspect Cargo's retained `compiler-artifact` records and their `fresh` fields for the renderer dependencies; a shared path is not a cache-hit guarantee. The first baseline build may still recompile them. Keep these large JSON records on disk and use bounded reads rather than printing whole artifact lines. This comparison establishes the selected tests' behavior and observed cache reuse; it does not replace the candidate's required checks or native acceptance.

## One target for several checkouts

Cargo identifies this repository's own packages by their path inside the workspace, which is the same in every checkout, and decides freshness by comparing the current checkout's file times with the shared artifact. Checkouts that share a target directory therefore share one artifact per package: a checkout whose files are older than the last build from another reports the package `Fresh` and runs the other checkout's code. A two-tree probe with Cargo 1.97 showed this in both directions on 2026-11-11.

Give each checkout that shares a target its own `codegen-units` value for the repository's packages; dependencies keep sharing artifacts. From the owner's checkout, create a worktree that builds into the owner's target with:

```sh
scripts/tools worktree add --from REVISION [--branch NAME] DIR
```

It creates `DIR` outside the owner's checkout, puts the checkout at `DIR/worktree`, and writes `DIR/.cargo/config.toml`, which Cargo reads when it runs inside the checkout. That file sets `[build] target-dir` to the owner's `target` and gives every workspace package one value below 250 that no Cargo configuration of another registered worktree uses. Checkouts that are not registered worktrees of this clone are invisible to that choice.

Its own value gives each checkout incremental caches of its own in the owner's target, and they outlive `worktree remove`. Before creating anything, `add` prints the free disk on the target's filesystem and the size of each incremental directory there, such as `debug/incremental`. Below 40 GiB free it refuses, naming the caches' total size. `--min-free-gib N` sets another floor. With 0 it never refuses for lack of space, but it still reads the free disk and the caches first, and an error in either still stops `add`. The owner reclaims the caches with:

```sh
scripts/tools target-prune [--older-than 7d] [--delete]
```

It lists the crate cache directories in `target/*/incremental` and `target/*/*/incremental` not modified for at least the given hours or days (default `7d`, at least `1h`), and removes them only with `--delete`. It removes nothing else.

Just before removing a cache, it checks it again. It keeps a cache modified since the listing, and one holding an `s-*.lock` file that another process has locked, as a rustc compiling that crate does; rustc's own clean-up respects the same lock. The locks it takes stay held until that removal ends, so a rustc that starts on the crate meanwhile cannot reuse the session being removed. It reports each cache as removed, already gone, kept with the reason, or failed with how much is left. An interrupt during removal stops it before the next cache, and it exits 130.

Links are not followed below the target: the listing resolves the target's own path, then reads the directories under it without following any link. A symbolic link in place of an incremental directory or a cache is refused, and the second check refuses a cache's own path that has since become a link. Removing a cache follows no link inside it. Another process that replaces a parent directory, such as `debug` or `incremental`, with a link between the listing and the removal is not guarded against. A refused link, or a cache it could not check or fully remove, makes it exit 1.

A removed cache leaves Cargo's built artifacts in place; the crate's next compilation, when Cargo needs one, starts its incremental cache afresh.

A harness whose session stays in the owner's checkout can refuse an edit to a file in `DIR/worktree` that it has not read there. Read each file with the harness's own file-read tool before editing it, and again after a formatter or another command rewrites it; a shell read such as `sed` or `cat` does not count. Send a check only once the edits it depends on have succeeded: a check sent beside a refused edit passes on unchanged code.

Once the work is merged, remove such worktrees with:

```sh
scripts/tools worktree remove --merged-into REF DIR...
```

For each `DIR` made by `add`, it requires the checkout's commit to be part of `REF`: a branch, remote branch or tag other than the checkout's own branch. It refuses ignored files outside `target/`, such as evidence under `local/`, because `git worktree remove` deletes them, and `git worktree remove` itself refuses changes or untracked files. It then deletes the checkout, its branch while that still names the checked commit, and the configuration file, and deletes `DIR` unless it holds other files. A `DIR` that fails a check keeps everything, and a branch that moves after the checks is kept and reported. Discarding unmerged work is the owner's decision.

For one command run elsewhere, `--config 'profile.dev.package.<package>.codegen-units=<n>'` does the same, as in the baseline recipe above. The owner's checkout keeps the default profile, and a worktree that builds into the owner's target uses its own value, so the owner's next build compiles the owner's code.

The final executables (`arkham-tabletop`, `arkham-driver`, `arkham-assets`, `arkham-tools`) are copied to one unhashed path per target and profile, and Cargo releases its lock before tests run. A build from another checkout can replace them while a suite that executes them through `CARGO_BIN_EXE_*` runs: the driver, agent-seat, asset-launch and artwork suites, and the `arkham-tools` suites that `scripts/check-tools` runs, which execute `arkham-tools`. Rerun such a failure alone before diagnosing it, and run an integration branch's final gate with no other build in progress. In a worktree made by `scripts/tools worktree add`, those paths are in the owner's `target`, so a README command naming `target/debug/arkham-tabletop` finds nothing inside the checkout. For a run that follows the README as written, or one that must keep a single build, copy the executable into the checkout's own `target/debug/` after building it.
