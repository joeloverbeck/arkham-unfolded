# Measured and baseline checks

These procedures extend the [local builds and checks](../../README.md#local-builds-and-checks) when a task needs resource evidence or a comparison with a baseline revision. The launcher, lock and recorder rules there still apply.

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
  --features tabletop --lib gathering_tabletop::tests::cards \
  --message-format=json-render-diagnostics
```

Require a finished, successful baseline record before returning to the candidate. Run the same command with a fresh output directory and without `--manifest-path` to check the original checkout. Inspect Cargo's retained `compiler-artifact` records and their `fresh` fields for the renderer dependencies; a shared path is not a cache-hit guarantee. The first baseline build may still recompile them. Keep these large JSON records on disk and use bounded reads rather than printing whole artifact lines. This comparison establishes the selected tests' behavior and observed cache reuse; it does not replace the candidate's required checks or native acceptance.
