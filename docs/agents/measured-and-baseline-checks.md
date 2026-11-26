# Measured and baseline checks

These procedures extend the [local builds and checks](local-builds-and-checks.md) when a task needs resource evidence, a comparison with a baseline revision, or one Cargo target shared by several checkouts. The launcher, lock and recorder rules there still apply.

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

Its own value gives each checkout incremental caches and builds of the repository's packages of its own in the owner's target, and they outlive `worktree remove`. Earlier builds also stay: Cargo puts each build in a directory named with a hash of the build's identity, which a new `unfolded-tabletop` revision also changes, and never removes the directory of an earlier identity. Before creating anything, `add` prints the free disk on the target's filesystem and the size of each incremental directory there, such as `debug/incremental`. Below 40 GiB free it refuses, naming the caches' total size. `--min-free-gib N` sets another floor. With 0 it never refuses for lack of space, but it still reads the free disk and the caches first, and an error in either still stops `add`. The owner reclaims caches and builds with:

```sh
scripts/tools target-prune [--older-than 7d] [--delete]
```

It lists the crate cache directories in `target/*/incremental` and `target/*/*/incremental` not modified for at least the given hours or days (default `7d`, at least `1h`). In the same profile directories, such as `debug` or `agent-tools/debug`, it also lists the builds of the repository's packages not modified for that long that a newer build of the same target supersedes. The owner runs this maintenance command explicitly. It removes what it lists only with `--delete` and resolves the owner's target even when the invoking worktree builds with a private `CARGO_TARGET_DIR`.

A build is a directory `build/<package>/<hash>` in a profile directory, such as `debug/build/arkham-tools/<hash>`, as Cargo 1.100, the pinned toolchain, lays them out; the packages are those of the checkout's committed `Cargo.toml` files. Its `fingerprint` directory names the unit it builds by a `<unit>.json` file, such as `bin-arkham-driver.json` or `test-lib-arkham_tools.json`, which also records the build's features, and its `out` directory holds what it built. A target is a package's unit with one set of features, built in one of two kinds: a check, as by `cargo check` or Clippy, leaving only metadata (`.rmeta`) and dependency lists (`.d`) in `out`, or a full build, leaving a library or executable. The newest build of each target and kind in each profile directory stays, however old, so a build without the `tabletop` feature and one with it each stay, as do a check of a binary and a newer full build of it. A check and a Clippy check of the same unit and features count as one: Cargo's fingerprint does not tell them apart from builds made with another `codegen-units` value. Other packages' builds, a build script's compilation and run, a build whose fingerprint names no unit or records no features, as while Cargo writes it, or whose `out` holds no file, and the unhashed copies Cargo makes of the newest libraries and executables, such as `debug/arkham-driver`, stay. It removes nothing else. Cargo calls this layout unstable; a test of `target-prune` runs the pinned Cargo and fails when it no longer finds the builds Cargo made.

Just before removing a cache, it checks it again. It keeps a cache modified since the listing, and one holding an `s-*.lock` file that another process has locked, as a rustc compiling that crate does; rustc's own clean-up respects the same lock. The locks it takes stay held until that removal ends, so a rustc that starts on the crate meanwhile cannot reuse the session being removed. It reports each cache as removed, already gone, kept with the reason, or failed with how much is left or why that could not be measured.

It removes a profile directory's builds while it holds that directory's `.cargo-build-lock`, so a build that starts meanwhile waits. Cargo 1.100 holds that lock throughout `cargo build` and `cargo check`, and waits for it, reporting "file lock on build directory", while another process holds it: a probe on 2026-11-19 showed both. When a build holds the lock, `target-prune` keeps that directory's builds and says so. Just before removing a build, it checks the build's directory again and keeps the build if anything in it was modified since the listing. Builds are reported as caches are. A removed build is rebuilt when a build next needs it.

Links are not followed below the target: the listing resolves the target's own path, then reads the directories under it without following any link. A symbolic link in place of an incremental directory or a cache is refused, and the second check refuses a cache's own path that has since become a link. Removing a cache follows no link inside it. A link in place of a profile's `build` directory, of one of the repository's package directories in it or of one of its builds, or anywhere inside such a build, is refused, and the second check refuses a build that is no longer a real directory or now holds a link. Removing a build follows no link inside it. Three races are not guarded against:
- another process replacing a parent directory, such as `debug` or `incremental`, with a link between the listing and the removal;
- a rustc that starts on the crate during its removal, which can lose the new session it creates there, so that build may fail once, or the removal ends as failed;
- a test run in another checkout: Cargo releases its lock before tests run, so a suite running builds older than the threshold that another checkout's newer builds supersede can lose an executable it has not started yet. Rerun such a failure before diagnosing it.

It exits 1 when it refused a link, could not lock a profile directory, or could not check or fully remove a cache or build, and 2 when it could not find the owner's target, read the free disk or the packages, or scan the caches or builds. With `--delete`, it checks for an interrupt before each cache or build it removes: an interrupt received before the last of those checks, including during the scan or the listing, stops it there with exit 130. An interrupt received after that check, during a dry run, or when nothing is to be removed leaves the run's ordinary exit status, except that a run ending in an error then exits 130.

A removed cache leaves Cargo's built artifacts in place; the crate's next compilation, when Cargo needs one, starts its incremental cache afresh.

A harness whose session stays in the owner's checkout can refuse an edit to a file in `DIR/worktree` that it has not read there. Read each file with the harness's own file-read tool before editing it, and again after a formatter or another command rewrites it; a shell read such as `sed` or `cat` does not count. Send a check only once the edits it depends on have succeeded: a check sent beside a refused edit passes on unchanged code.

Once the work is merged, remove such worktrees with:

```sh
scripts/tools worktree remove --merged-into REF DIR...
```

For each `DIR` made by `add`, it requires the checkout's commit to be part of `REF`: a branch, remote branch or tag other than the checkout's own branch. It refuses ignored files outside `target/`, such as evidence under `local/`, because `git worktree remove` deletes them, and `git worktree remove` itself refuses changes or untracked files. It then deletes the checkout, its branch while that still names the checked commit, and the configuration file, and deletes `DIR` unless it holds other files. A `DIR` that fails a check keeps everything, and a branch that moves after the checks is kept and reported. Discarding unmerged work is the owner's decision.

For one command run elsewhere, `--config 'profile.dev.package.<package>.codegen-units=<n>'` does the same, as in the baseline recipe above. The owner's checkout keeps the default profile, and a worktree that builds into the owner's target uses its own value, so the owner's next build compiles the owner's code.

The final executables (`arkham-tabletop`, `arkham-driver`, `arkham-assets`, `arkham-tools`) are copied to one unhashed path per target and profile, and Cargo releases its lock before tests run. A build from another checkout can replace them while a suite that executes them through `CARGO_BIN_EXE_*` runs: the driver, agent-seat, asset-launch and artwork suites, and the `arkham-tools` suites that `scripts/check-tools` runs, which execute `arkham-tools`. Rerun such a failure alone before diagnosing it, and run an integration branch's final gate with no other build in progress. In a worktree made by `scripts/tools worktree add`, those paths are in the owner's `target`, so a README command naming `target/debug/arkham-tabletop` finds nothing inside the checkout. For a run that follows the README as written, or one that must keep a single build, copy the executable into the checkout's own `target/debug/` after building it.
