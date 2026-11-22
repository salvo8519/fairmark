---
title: Mutation testing
description: How Fairmark checks that the tests of the core detect wrong results — cargo-mutants locally, on every pull request and on a weekly slice, within a measured time budget — and how missed mutants are triaged.
---

# Mutation testing

Code coverage says which lines the tests execute; it does not say whether the tests would
notice a wrong result. Mutation testing does: a tool changes the code in small ways (a `+`
becomes `-`, a `<=` becomes `<`, a function returns `Ok(Default::default())`) and runs the
tests on each change, called a *mutant*. A mutant that makes a test fail is **caught**. A
mutant that leaves every test green is **missed**: the tests cannot tell that version of the
code from the real one. For a valuation engine a missed mutant in a calculation path is a
number that could be wrong without any test noticing.

Fairmark uses [cargo-mutants](https://mutants.rs/) on `crates/fairmark-core`, the crate that
holds all financial logic. Adapters translate and never compute, so they are not mutated.

## Outcomes

| Outcome | Meaning | Action |
| --- | --- | --- |
| Caught | A test failed on the mutant | None |
| Missed | Every test passed on the mutant | Triage (below) |
| Equivalent | Missed, and recorded as equivalent after review | None, until the code changes |
| Unviable | The mutant does not compile | None: the compiler rejects it |
| Timeout | The tests did not finish in time (for example a solver that no longer converges) | Detected; check that the timeout is not too short |
| Failure | cargo-mutants could not test the mutant | Inspect the log in the artifact |
| Untested | The run stopped before the mutant: time budget reached, runner lost | The run fails with "mutation coverage incomplete" (below) |

## Where it runs

| Where | Command or workflow | Scope | Blocking |
| --- | --- | --- | --- |
| Locally, one module | `uv run just mutants tvm` (also `dcf/engine`, or a folder such as `tv`) | That module | No |
| Locally, one weekly slice | `uv run just mutants-slice 4` (no index: the slice of this week) | One slice of the rotation (below) | No |
| Locally, the whole core | `uv run just mutants` | All of `fairmark-core` (about 3,000 mutants; more than a day on 2 cores) | No |
| Locally, before a pull request | `uv run just mutants-diff` | Lines of `crates/fairmark-core/src` changed since the merge base with `origin/main` | Exit 1 on a missed mutant not recorded as equivalent |
| Every pull request | CI job *Mutation testing · changed lines of the core* | Changed lines of `crates/fairmark-core/src` (`--in-diff`), in as many shards as the time budget needs; finishes in seconds when the core is not touched | Reported: the job fails on a missed mutant not recorded as equivalent and on incomplete coverage, but it is not yet a required check. It becomes one once the triage debt is closed (the maintainer changes the ruleset) |
| Weekly (Sunday) and on demand | Workflow *Mutation testing* (`.github/workflows/mutants.yml`) | One slice of the core per week; the whole core once per cycle of 15 weeks | Report only for missed mutants; a scheduled run that fails (incomplete coverage, failing plan or baseline, tool failure) opens or updates the issue labelled `maintenance`. The v1.0 release gate on missed mutants is issue #283 |

Local runs write to `target/mutants/`, `target/mutants-slice/` and `target/mutants-diff/`. Both
CI paths use one reusable workflow, `.github/workflows/mutants-run.yml`: a plan job, the shards,
and a summary job that checks completeness and writes a table per module (caught, missed,
equivalent, unviable, timeout) and the mean time per mutant to the job summary. Each shard
uploads its `mutants.out` directory as an artifact (diffs and logs of every mutant).

## Test profile for mutants

Every mutant runs the whole test suite of the core, so the cost of one test run is paid once
per mutant. Several tests sweep the golden inputs: every golden file, or several files per
method with each field perturbed in turn (key order, two keys of a choice, declared reads,
actionable pointers, validation against computation). In normal CI they run in full. Under
mutation testing, `.cargo/mutants.toml` passes `--config env.FAIRMARK_TEST_PROFILE="mutants"`
to cargo, which sets the variable for the test processes, and those sweeps check a fixed sample
per method instead (`crates/fairmark-core/src/test_profile.rs`): two files of each method, spread
over its sorted list, or one file per method with fewer perturbed fields. The sample is
deterministic. The golden value checks, the property tests and every other test run unchanged.
Normal CI never sets the variable; any value other than `full` or `mutants` is refused.

Measured for issue #368 on the same commit, Linux with 2 cores, `--profile mutants`, and the
same 12 mutants of `error.rs` (all caught in both runs):

| Measure | Full profile | Mutants profile |
| --- | --- | --- |
| One test run of the core (`cargo test --lib --tests`) | 67 s | 27 s |
| Baseline test run of cargo-mutants | 62 s | 25 s |
| Mean time per mutant: build and test | 57 s: 10 s and 46 s | 23 s: 10 s and 13 s |
| The 12 mutants | 14 minutes | 6 minutes |

On GitHub-hosted `ubuntu-latest` (4 cores) the weekly run of 2026-11-22 took 47 to 48 s per
mutant in the full profile (14 to 15 s build, 32 to 33 s test). The summary of every run prints
the mean it measured ("Mean time per mutant").

The two slowest tests, `every_error_declares_the_fields_it_reads` and
`every_pointer_an_error_names_is_declared_and_actionable`, take 24 s each in the full profile
and 3.5 s and 5.9 s in the mutants profile (one core each); the unit tests of the core take
45 s and 10 s.

## Time budget and shards

`tests/mutants/budget.toml` holds the estimated time per mutant and the budget of each kind of
run. A run is split into shards so that each fits its budget:

- mutants per shard = (`run_minutes` − `baseline_minutes`) × 60 / `seconds_per_mutant`, rounded
  down;
- shards = mutants / mutants per shard, rounded up, at most `max_shards`. A run that needs more
  fails at the plan, before any runner time is spent: a pull request that changes too much is
  split, a weekly slice that grew is split in `budget.toml`;
- cargo-mutants is stopped after `run_minutes`; the job limit is `run_minutes` +
  `setup_minutes`, so a shard always reports what it did not test.

| Run | Run minutes per shard | Mutants per shard | Maximum shards | Job limit |
| --- | --- | --- | --- | --- |
| Pull request | 30 | 54 | 8 | 40 minutes |
| Weekly slice | 45 | 84 | 4 | 55 minutes |

With `seconds_per_mutant` = 30, `baseline_minutes` = 3 and `setup_minutes` = 10. The summary of
every run prints the mean time per mutant it measured: when it moves away from
`seconds_per_mutant`, update the estimate (and the slices if needed) in a pull request.

## Weekly rotation

The weekly workflow tests one slice of the core. The slices are listed in
`tests/mutants/budget.toml`: groups of source files of the core, each sized to fit the weekly
budget (at most 3 shards today). A large file is split into parts: with `parts = 3`
(`reading.rs`), part k tests the mutants k, k + 3, k + 6, ... of its list, round-robin. The
slice of a week is number w mod n, where w counts the ISO weeks (Monday to Sunday, UTC) since
Monday 5 January 1970 and n is the number of slices after the parts are expanded.

**Cycle length: 15 weeks.** Every mutant of the core is tested once per cycle. Every source
file of the core is in exactly one slice, which `python scripts/mutants.py check-slices` (part
of `just lint`) verifies, so a new source file is added to a slice in the pull request that
creates it. `python scripts/mutants.py slice` prints the slice of this week, `--date` or
`--index` any other; a manual run of the workflow takes the index as input. Stale equivalent
entries are reported for the files of the slice when it covers whole files.

The weekly runner time is at most 3 shards of 55 minutes, about 2.75 hours, against 16 shards of
90 minutes before (24 hours, and the run of 2026-11-22 still did not finish).

## Incomplete coverage fails explicitly

A run never ends with mutants silently untested. The plan job keeps the list of each shard.
A shard whose cargo-mutants stops at the time budget (or exits without an outcome for every
mutant) fails with **Mutation coverage incomplete** and lists, by name, the mutants it did not
test. The summary job compares the list of every shard with its outcomes and classifies each
shard:

| Result | Cause | What to do |
| --- | --- | --- |
| complete | Every mutant of the shard was tested | None |
| timeout: the time budget was reached | cargo-mutants was stopped at `run_minutes` | The estimate or the slice no longer fits: update `budget.toml` |
| timeout: the job time limit was reached | The job reached `timeout-minutes` (`run_minutes` + `setup_minutes`): GitHub ends it as cancelled and writes "The job has exceeded the maximum execution time" in its annotations, not in its log; usually a setup longer than `setup_minutes` | Raise `setup_minutes` |
| infrastructure: the runner was lost | The log of the job holds "The runner has received a shutdown signal" or "lost communication with the server" | Re-run the failed jobs; no change to the code or the budget |
| tool failure | cargo-mutants exited with another status (usage error, failing baseline) | Read the log of the shard |
| cancelled | The run was cancelled (a newer push to the pull request) | None |

The summary reads the jobs of the run, and the logs and the annotations of the failed shards,
through the GitHub REST API with the `actions: read` permission.

## Configuration

- **Tool**: cargo-mutants **27.1.0**, pinned. CI installs it with `taiki-e/install-action`,
  which checks the SHA-256 of the release binary, and falls back to
  `cargo install --locked cargo-mutants@27.1.0` (crates.io, with the published lock file).
  Locally: `cargo install --locked cargo-mutants@27.1.0`. It is a development tool: it is not a
  dependency of any crate and never ships. Updates follow the
  [dependency policy](dependencies.md); a new version can generate different mutants, so the
  update pull request records the change in the totals of one slice and in the count of mutants
  per slice (`cargo mutants --list`).
- **`.cargo/mutants.toml`**: tests are `cargo test --package fairmark-core --lib --tests`
  (unit tests, property tests and every integration test, golden cases included; the single
  doctest, a usage example, is skipped because the golden cases cover the same path with
  independent expected values), in the mutants test profile (above). The test timeout is three
  times the baseline test time and never less than 60 seconds. Builds use the `mutants` profile
  (the test profile without debug information, defined in the workspace `Cargo.toml`).
- **No file of the core is excluded.** Every caller passes `--package fairmark-core`; the
  configuration has no `examine_globs`, because cargo-mutants adds them to the module filter of
  `just mutants <module>` instead of narrowing it.
- **Sharding**: round-robin (`--sharding round-robin`, `--no-shuffle`), with the number of
  shards from the plan (above). All shards of a run use the same arguments, as cargo-mutants
  requires.

## Triage rules

Every missed mutant ends in exactly one of two ways:

1. **Killed by a new test.** Write the test that fails on the mutant and passes on the real
   code: a unit test, a property test, or a golden case with an expected value from an
   independent source (see [golden cases](../concepts/determinism.md#golden-cases) and
   `AGENTS.md`). A test whose expected value was copied from the engine's output does not
   count. Confirm with `uv run just mutants <module>` that the mutant is now caught.
2. **Recorded as equivalent.** If no observable result can change (for example a guard whose
   two branches return the same value at the boundary), add one entry to
   `tests/mutants/equivalent.toml`:

   ```toml
   [[equivalent]]
   file = "crates/fairmark-core/src/rates.rs"
   function = "inverse_pow"
   mutation = "replace match guard !g.is_zero() with true in inverse_pow"
   line = 137
   reason = "One line: why no input can produce a different output or error."
   reviewer = "github-login-of-the-maintainer"
   ```

   Every key is required. The mutation text and the line are copied exactly as cargo-mutants
   prints them, so an entry excludes one mutation and nothing else: matching is by exact text
   and line, never by pattern. `python scripts/mutants.py check-exclusions` (part of
   `just lint`) rejects an entry with a missing key, outside the core, whose mutation does not
   name its function (it ends with `in <function>`, or starts with `replace <function> ->` or
   `replace <function> with` when the whole body is replaced), or that repeats an earlier
   entry. When the code moves, the line is updated in the same pull request. The entry is
   approved by a maintainer in the pull request that adds it: `reviewer` is the GitHub login of
   that maintainer, one of the owners named in `.github/CODEOWNERS` (the one list of
   maintainers); a placeholder such as `pending` fails `check-exclusions`, so an entry cannot
   pass before its approval.

Not accepted: exclusions of whole files, functions or modules; `#[mutants::skip]` attributes
on financial code; raising the timeout to hide a timeout; deleting or weakening a test.
A missed mutant is never "for later": on a pull request it is resolved in that pull request,
and the weekly report is the triage list for the code that predates this check. The
missed mutants of the first baseline, with a provisional category each, are tracked in
`tests/mutants/triage.md` until they are resolved. Equivalent
mutants are still built and tested, so the weekly slice of their file (or a full local run) lists
the entries whose mutant is no longer missed (now caught, unviable or gone); remove them in the
next pull request on that module.

## Reading the results

`python scripts/mutants.py summary <dir> [<dir> ...]` prints the table per module for one or
more `mutants.out` directories (shards are merged) and the list of missed mutants with file,
line and function. `--gate` exits 1 when a missed mutant is not recorded as equivalent;
`--stale-in <files>` reports stale equivalent entries of those files. `coverage` (one shard)
and `completeness` (a whole run) report the mutants a run did not test.
For each mutant, `mutants.out/diff/` holds the change and `mutants.out/log/` the build and
test output.

## Security

The workflows run with `contents: read`, use no secrets, do not persist credentials and pin
every action to a commit. The summary job also has `actions: read`, to read the jobs and logs
of its own run. The only write permission is `issues: write`, on the job of the weekly workflow
that opens or updates the `maintenance` issue after a failed scheduled run. The pull-request job runs the tests of the pull request, as the
*Rust* job already does, with no more privileges than it.
