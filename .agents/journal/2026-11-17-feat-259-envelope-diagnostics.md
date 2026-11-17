# 2026-11-17 — feat-259-envelope-diagnostics

- Issue: #259 One envelope: diagnostics block and envelope version
- Pull request: none (opened by the orchestrator after T24)
- Spec: SPEC-0012 (tasks T5 to T23 in this session)

## Done
- T5 envelope tests in `registry.rs`, ignored until T11 enables them.
- T6 warning tests FM-W001 to FM-W016 and the echo test, `tests/warnings.rs`, ignored until T11 and T12.
- T7 not computed, checks, defaults and decision 14 tests, `tests/diagnostics.rs`, ignored until T12 to T14.
- T8 pointer naming walk (AC10, any field whose description says "JSON pointer" or "RFC 6901"), pointer resolution and determinism tests in `registry.rs`, ignored until T11, T14, T15.
- T9 catalogue test `tests/warning_catalogue.rs` (text check: `Variant = "FM-Wnnn"` declared in `src/diagnostics.rs`, emitted as `WarningCode::Variant`, documented in `docs/en/reference/warnings.md`); main test ignored until T20, planted test active.
- T10 analysis diagnostics tests `tests/analysis_diagnostics.rs` (v2 field names), ignored until T15.
- T11 `diagnostics.rs` (types, `WarningCode` FM-W001 to FM-W016, pattern-only schema), `Explain` without `method` and with `warn`/`not_computed`/`checks`, `Meta.envelope_version`, `defaults_applied` always present, `Valuation.diagnostics`, FM-W001 from the reading (sorted by pointer). Helpers for T12/T15 parked in `/tmp/i259-nested.rs`, `/tmp/i259-iswarning.rs`, `/tmp/i259-tests-moved.rs` (scratch, re-create if lost: `nested` rebasing for analyses, `is_warning_text`).
- T12 every warning site through `ex.warn` (FM-W002 to FM-W016, preview as FM-W016 via `Inner::warn_if_preview`), debug assertion in `Explain::convention`, T6 tests enabled; contract expectations in `tests/contracts/` carry the codes. AC8 test stays ignored until T14 (key rename).
- T13 ratios v3: `RatioValue {value}`, `NotComputedList` writes `NotComputed` (missing_input, not_defined; decision 14), six VERSION 3, snapshots v3, 34 ratios golden files migrated (verify: 34 checked, 34 migrated, 0 problems), `ratios_partial.rs` and the reference test read the diagnostics, ratio pages en/it updated and restamped (snippets use `r.raw["diagnostics"]` until T18).
- T14 statements v2 (checks and not_computed into diagnostics, `NotComputedEntry::into_diagnostic`, pointer fields renamed by serde, VERSION 2, CheckStatus shared from diagnostics) and `DefaultApplied.pointer`; 84 golden files migrated (82 statements, 2 dcf.fcff); verify over the plan: 131 checked, 118 migrated, 0 problems (the 13 analysis files wait for T15). Tests use `year_checks`/`year_not_computed` helpers reading the diagnostics. Python docstrings of `ratios.py`/`statements.py` still name `not_computed` in `value`: fix in T18.
- T15 analyses v2: pointer fields renamed by serde (`variable_pointer`, `output_pointer`, `output_pointers`) in and out, error locations follow, base-case warnings reported once (`report_base_warnings`), goal seek reports the solved evaluation's diagnostics under `/value/inner_value` and `/base_input`; VERSION 2, snapshots; 13 analysis golden files migrated; verify over the whole plan: 131 checked, 131 migrated, 0 problems. Python helpers keep their keyword names and send the new keys. Analysis pages en/it updated and restamped. Scratch files `/tmp/i259-*.rs` no longer needed.
- T16 three new golden cases with `expect_diagnostics`, expectations written by hand from the rules (ratios.liquidity, statements.reorganise reclassified, analysis.sensitivity); resolution test accepts a defaulted field of the evaluated method under `/base_input`.

## State at the end of the session
- In progress: tasks T5 to T23 of `specs/0012-envelope-diagnostics/tasks.md`, one commit each.
- Test-first commits (T5 to T10) mark the new tests `#[ignore = "SPEC-0012: enabled by
  T<n>"]`, so that every commit passes `just check`; the implementation task removes the
  marker.

## Decisions taken
- The key `field` of `meta.defaults_applied` is renamed `pointer` in T14, not T11: 15 statements
  golden files mix that rename with the moves of T14, and `apply` migrates a file whole, so the
  rename lands with the statements migration (the two dcf.fcff files and the seven statements
  files with the rename only migrate in the same commit).

## Open questions for the maintainer
- Settled by the orchestrator on 2026-11-17: FM-W017, emitted by the core from a run option. Was: the MCP server (`python/fairmark/mcp_server.py` `call_tool`) appends its own convention
  `warning: \`<pointer>\` arrived as a binary floating-point number ...` to the core envelope.
  It is a seventeenth warning condition the spec inventory (core only) did not list, it has no
  code, and it contradicts R9 and R15 (adapter changes the envelope). Left unchanged; question
  for the maintainer: give it a code (FM-W017, emitted how?) or keep it as an adapter note.

## Next steps
1. T18: Python `Result.diagnostics` and the properties; `python/tests/test_envelope.py`; ratios.py/statements.py docstrings; doc snippets to `r.not_computed`.
