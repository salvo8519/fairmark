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
- Refusing a decimal with more than 28 places would refuse the input of the protected golden
  case tests/golden/tvm.future_value/ecma-376-monthly-rate-due.yaml (rate with 30 places, 28
  significant digits; the only such input among the golden cases). Either approve a change
  that names this case, or choose another rule (for example refusing only digits beyond what a
  Decimal holds, 28 significant digits).

## Next steps
1. Maintainer answer on the 28 decimal places; then implement it test-first with a structured
   `type` or `range` error at the field.
2. Code review and numerical audit of round 6 on #279.
