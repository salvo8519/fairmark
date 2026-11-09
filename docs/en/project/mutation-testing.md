---
title: Mutation testing
description: How Fairmark checks that the tests of the core detect wrong results — cargo-mutants locally, on every pull request and weekly — and how missed mutants are triaged.
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

## Where it runs

| Where | Command or workflow | Scope | Blocking |
| --- | --- | --- | --- |
| Locally, one module | `uv run just mutants tvm` (also `dcf/engine`, or a folder such as `tv`) | That module | No |
| Locally, the whole core | `uv run just mutants` | All of `fairmark-core` (about 2,000 mutants, several hours) | No |
| Locally, before a pull request | `uv run just mutants-diff` | Lines of `crates/fairmark-core/src` changed since the merge base with `origin/main` | Exit 1 on a missed mutant not recorded as equivalent |
| Every pull request | CI job *Mutation testing · changed lines of the core* | Changed lines of `crates/fairmark-core/src` (`--in-diff`); finishes in seconds when the core is not touched | Yes |
| Weekly (Sunday) and on demand | Workflow *Mutation testing* (`.github/workflows/mutants.yml`) | All of `fairmark-core`, in 16 shards | Report only; the v1.0 release gate on missed mutants is issue #283 |

Local runs write to `target/mutants/` and `target/mutants-diff/`. The workflows upload each
`mutants.out` directory as an artifact (diffs and logs of every mutant) and write a table per
module (caught, missed, equivalent, unviable, timeout) to the job summary.

## Configuration

- **Tool**: cargo-mutants **27.1.0**, pinned. CI installs it with `taiki-e/install-action`,
  which checks the SHA-256 of the release binary, and falls back to
  `cargo install --locked cargo-mutants@27.1.0` (crates.io, with the published lock file).
  Locally: `cargo install --locked cargo-mutants@27.1.0`. It is a development tool: it is not a
  dependency of any crate and never ships. Updates follow the
  [dependency policy](dependencies.md); a new version can generate different mutants, so the
  update pull request records the change in the weekly totals.
- **`.cargo/mutants.toml`**: tests are `cargo test --package fairmark-core --lib --tests`
  (unit tests, property tests and every integration test, golden cases included; the single
  doctest, a usage example, is skipped because the golden cases cover the same path with
  independent expected values). The test timeout is three times the baseline test time and
  never less than 60 seconds. Builds use the `mutants` profile (the test profile without debug
  information, defined in the workspace `Cargo.toml`).
- **No file of the core is excluded.** Every caller passes `--package fairmark-core`; the
  configuration has no `examine_globs`, because cargo-mutants adds them to the module filter of
  `just mutants <module>` instead of narrowing it.
- **Sharding**: the weekly run divides the mutants round-robin into 16 shards, at most 8 at a
  time, 90 minutes each. All shards use the same arguments, as cargo-mutants requires.

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
   line = 137                # optional: pins one occurrence
   reason = "One line: why no input can produce a different output or error."
   reviewer = "github-login-of-the-maintainer"
   ```

   The mutation text is copied exactly as cargo-mutants prints it. Matching is by exact text,
   never by pattern; `python scripts/mutants.py check-exclusions` (part of `just lint`)
   rejects an entry without a function, a reason or a reviewer, or outside the core. The
   entry is approved by a maintainer in the pull request that adds it.

Not accepted: exclusions of whole files, functions or modules; `#[mutants::skip]` attributes
on financial code; raising the timeout to hide a timeout; deleting or weakening a test.
A missed mutant is never "for later": on a pull request it is resolved in that pull request,
and the weekly report is the triage list for the code that predates this check. Equivalent
mutants are still built and tested, so a full run lists entries that no longer match a missed
mutant; remove them in the next pull request on that module.

## Reading the results

`python scripts/mutants.py summary <dir> [<dir> ...]` prints the table per module for one or
more `mutants.out` directories (shards are merged) and the list of missed mutants with file,
line and function. `--gate` exits 1 when a missed mutant is not recorded as equivalent.
For each mutant, `mutants.out/diff/` holds the change and `mutants.out/log/` the build and
test output.

## Security

The workflows run with `contents: read`, use no secrets, do not persist credentials and pin
every action to a commit. The pull-request job runs the tests of the pull request, as the
*Rust* job already does, with no more privileges than it.
