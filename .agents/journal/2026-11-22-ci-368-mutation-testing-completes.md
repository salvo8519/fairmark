# 2026-11-22 — ci/368-mutation-testing-completes

- Issue: #368 Mutation testing completes in CI or fails explicitly
- Pull request: #414 (ready; CI green)
- Spec: none (tooling and tests; no computed number changes)

## Done
- Mutants test profile: `crates/fairmark-core/src/test_profile.rs`, enabled by
  `additional_cargo_args = ["--config", "env.FAIRMARK_TEST_PROFILE=\"mutants\""]` in
  `.cargo/mutants.toml`. The sweeps over golden inputs in `registry.rs`, `reading.rs` and
  `tests/golden.rs` sample per method in that profile; normal CI is unchanged. One test run on
  2 cores: 67 s full, 27 s mutants profile.
- Budget and rotation: `tests/mutants/budget.toml` (estimate 30 s per mutant; pull request
  30 minutes per shard, at most 8 shards; weekly 45 minutes, at most 4 shards; 15 slices, cycle
  of 15 weeks). `scripts/mutants.py`: `check-slices`, `slice`, `plan`, `coverage`,
  `completeness`, `failed-jobs`, `summary --stale-in` and the mean time per mutant.
- Workflows: reusable `.github/workflows/mutants-run.yml` (plan, shards stopped at the budget,
  summary with completeness and runner-lost or timeout classification); `ci.yml` and
  `mutants.yml` call it. `just mutants-slice`.
- Missed mutants of run 44972766259 shard 4: 7 killed by new tests (`mutants_errors.rs`,
  `mutants_loans.rs`, `mutants_reading.rs`), 2 removed with dead code (the debt-weight and
  leverage re-checks in `coc::wacc`, the `"f32" | "f64"` arm of `reading::plain`), 1 equivalent
  (`loans.rs:1197:29`, reviewer pending).
- Documentation: `docs/en/project/mutation-testing.md`, `docs/en/project/dependencies.md`.
  Roadmap: #368 done.

- First CI runs of the new job found three defects, fixed on the branch: `--sharding` without
  `--shard` is refused by cargo-mutants 27.1.0; `CARGO_TERM_COLOR=always` coloured the names of
  `--list` (now `--colors never`, and the script strips colour codes); a single downloaded
  artifact has no directory of its own (shards now write `out/shard-<n>/mutants.out`). One more
  missed mutant on a changed line (signed integer arm of `reading::plain`) is killed by a unit
  test.
- CI of #414: plan 9 s, one shard of 6 mutants in 4 minutes, summary complete; mean time per
  mutant 29 s (build 21 s, test 8 s), baseline test run 16 to 26 s.

## State at the end of the session
- Nothing half done. The weekly workflow has not run yet with the new design: the first
  scheduled run after the merge tests the slice of that week.

## Decisions taken
- Mechanism: one environment variable set through cargo `--config env.*` by cargo-mutants only,
  read by one shared module, instead of a cargo feature (no test-only feature in the published
  crate) or nextest filters (a new tool, and the sweeps would be skipped, not sampled).
- The weekly slices are fixed groups of files, with `reading.rs` in 3 round-robin parts;
  the week index counts ISO weeks since Monday 5 January 1970, so the cycle does not restart
  at the turn of the year.
- `./` kept for the reusable workflow calls, with a zizmor suppression: the pinned actionlint
  rejects the `$/` self-repository form.

## Open questions for the maintainer
- Approve the equivalent entry for `loans.rs:1197:29` (reviewer is "pending").
- The cancelled shards of run 44972766259 also reported missed mutants in their annotations
  (shard 0, for example `Ratio::needs_positive_denominator`, `PeriodLength::years`); they are
  not part of #368 and will be listed again by the weekly slices.

## Next steps
1. After the first weekly run, read "Mean time per mutant" in its summary and update
   `seconds_per_mutant` in `tests/mutants/budget.toml` if it differs by more than 20 per cent
   (pull request #414 measured 29 s on 6 mutants).
