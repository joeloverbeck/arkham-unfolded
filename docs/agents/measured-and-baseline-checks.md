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

Give each checkout that shares a target its own `codegen-units` value for the repository's packages; dependencies keep sharing artifacts. For worktrees, set the shared directory with `[build] target-dir` in a `.cargo/config.toml` above them, outside the repository, and put a distinct value in a `.cargo/config.toml` in each worktree's parent directory:

```toml
[profile.dev.package.arkham-unfolded-experiment]
codegen-units = 249
[profile.dev.package.arkham-tools]
codegen-units = 249
```

Cargo reads these files when it runs from inside the worktree. For one command run elsewhere, `--config 'profile.dev.package.<package>.codegen-units=<n>'` does the same, as in the baseline recipe above. The owner's checkout keeps the default profile, and a worktree that builds into the owner's target uses its own value, so the owner's next build compiles the owner's code.

The final executables (`arkham-tabletop`, `arkham-driver`, `arkham-assets`) are copied to one unhashed path per target and profile, and Cargo releases its lock before tests run. A build from another checkout can replace them while a suite that executes them through `CARGO_BIN_EXE_*` runs: the driver, agent-seat, asset-launch and artwork suites. Rerun such a failure alone before diagnosing it, and run an integration branch's final gate with no other build in progress.
