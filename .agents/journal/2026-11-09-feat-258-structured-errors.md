# 2026-11-09 — feat/258-structured-errors

- Issue: #258 Structured errors per ADR 0014 G5 (also closes #201)
- Pull request: #279 (ready, review round 6)
- Spec: none (ADR 0014 G5, contract change; tooling of the error report)

## Done
- Round 6 regression cases of review and audit round 5 (765c027, tests/error_round6.rs).
- Declared reads replace placeholder filtering: every error declares pointer, pointers and
  `Details::reads`; the reading skips an error that declares a refused field (7105450).
- Completeness property test `every_error_declares_the_fields_it_reads` (registry.rs);
  checks no longer stop at a refusal before an independent check (7105450, 4cfdfb8).
- Dependency pointers: rate rule with /compounding or /from, FM-E019 with /basis, exit
  multiple value with /metric (7105450).
- Docs and release note for declared reads, English and Italian (5543740, 3b4676e).
- Property test `every_pointer_an_error_names_is_declared_and_actionable`: every pointer
  named is declared and is in the input or is a field to give. Fixes it found: tv.gordon with
  neither flow is `missing`; a debt form conflict names only the fields given; a default
  compounding is read, never named (tvm.xnpv, tvm.xirr); FM-E024 reason `requires`
  (2ada2a5, docs a982be5). Run over every golden file with 40 leaves per file: passed.

## State at the end of the session
- origin/main (#298, #282) merged into the branch: the enterprise-basis financing error keeps
  main's message (only the fields given) with pointer, pointers and reads of the report;
  annuity_factor takes main's underflow rule, the branch keeps its periods and growth
  helpers. Regression test in tests/error_round6.rs.
- Round 6 complete except the refusal of decimals with more than 28 places (see below).
- `uv run just check` passed before the rebase on origin/main.

## Decisions taken
- FM-E024 reason changed from `conflict` to `requires` (agent, within this PR; the reason
  values are not on main). To confirm in review.
- An `FM-E026` reconciliation may name an item it took as zero, absent from the input: it is
  a field to give, as the message and fix already say (agent).

## Open questions for the maintainer
- None open. Decided on 2026-11-10 (maintainer): a decimal with more than 28 decimal places is
  accepted, rounded half away from zero at the first digit that does not fit (the parser's
  rule), and reported with a `warning: ` convention naming the pointer, the value given and
  the value used; `meta.input_hash` stays on the value used; a value too large for a decimal
  stays a `type` error; every exponent form is read as its plain digits. No golden case
  changes in a compared field (tvm.future_value/ecma-376-monthly-rate-due gains the warning
  convention only).
- Known limit, documented in the MCP and function-calling guides: the MCP library reads a
  JSON number as a binary float before Fairmark; tool schemas now declare decimals as strings
  and the MCP server reports every argument that arrived as a float.

## Next steps
1. Mutation testing of the changed lines cannot finish in CI for this pull request: the diff
   has 1263 mutants, the job has room for about 150 and was cancelled at 60 minutes. The two
   mutants it reported missed (coc.rs check_tax_rate and the country exposure bound of
   check_cost_of_equity) are killed by tests/mutants_errors.rs (checked locally with
   cargo-mutants 27.1.0). The maintainer decides how the rest is run (for example the
   scheduled mutants workflow on the branch, by module).
2. Maintainer review of #279 after the final code review (2026-11-10), every finding fixed.
   Keep a non-merge commit with the `Docs-impact: none` trailer last: docs_sync impact reads
   the latest non-merge commit.
