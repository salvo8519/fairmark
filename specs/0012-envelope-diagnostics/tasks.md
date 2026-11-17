# SPEC-0012 — Tasks

Status: active (SPEC-0012 approved on 2026-11-16). Issue #259.

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes. No golden file is touched before the
maintainer has approved, by name, the file list printed by T3; T11, T13, T14 and T15 then
migrate the golden keys of their own changes, and T16 adds new cases.

## Preparation
- [x] T1 Inventory script `scripts/envelope_inventory.py` (read-only) and its output compared
  with the spec inventory; any difference added to `spec.md` [R5 to R8, R10, R12, R18]
- [x] T2 [P] Golden runners (Rust `crates/fairmark-core/tests/golden.rs`, Python runner) accept
  `expect_diagnostics` and `expect_diagnostics_length`, tested on a temporary case [AC16]
- [x] T3 [P] `scripts/golden_envelope_migration.py` with `list`, `apply` and `verify`; `list`
  output attached to issue #259 for the maintainer's named approval [R18; AC16]
- [x] T4 [P] ADR 0014 G4 amendment: `*_pointer` admitted, maintainer decision of 2026-11-16
  [R12, R20]

## Tests (write first, they must fail)
- [x] T5 Envelope tests in `registry.rs`: key order, `envelope_version`, no `explain.method`,
  empty arrays present and required in every result schema [R1 to R4; AC1 to AC3]
- [x] T6 Warning tests, one per code FM-W001 to FM-W016, and the echo test [R8, R9; AC7]
- [x] T7 Not computed and checks tests for the ratios and the statements, defaults key test,
  and the precedence test of decision 14 (missing input before not defined: one entry per
  output) [R5 to R7, R10, R12; AC4 to AC6, AC8, AC9, AC17]
- [x] T8 Pointer and determinism tests: naming walk over every input and result schema, no
  exemption, pointer resolution and byte-identical diagnostics over golden inputs [R11 to R13;
  AC10, AC11]
- [x] T9 [P] Catalogue test: every emitted code documented, every documented code emitted, no
  `warning` literal outside the warning API [R16; AC14]
- [x] T10 [P] Analysis diagnostics tests (sensitivity once under `/base_input`, goal seek under
  `/value/inner_value`) [R14; AC12]

## Implementation
Each implementation task that moves golden keys runs `golden_envelope_migration.py apply` on
its approved files and `verify` in the same commit, so every commit passes `just check`.

- [ ] T11 `diagnostics.rs` types and collector; `Explain` without `method`; `Meta` with
  `envelope_version` and `defaults_applied` always written, key `pointer`; `Valuation` with
  `diagnostics`; registry assembly and FM-W001 from the reading; golden migration of the
  `expect_meta` keys `/defaults_applied/{k}/field` [R1 to R4, R8 to R11, R13; AC16]
- [ ] T12 Warnings through `ex.warn` in series, DCF engine, terminal, bridge, fcff, ratios,
  statements and the analysis preview convention (FM-W002 to FM-W016) [R8, R9]
- [ ] T13 Ratios v3: `RatioValue` with `value` only, `not_computed` into diagnostics, six
  `VERSION` = "3", `uv run just generate` (snapshots `v3.json`, `v3.output.json`, index), golden
  migration of `tests/golden/ratios.*` [R5, R6, R18; AC4, AC5, AC15, AC16]
- [ ] T14 Statements v2: checks and not computed into diagnostics with `pointer`, pointer fields
  renamed, `VERSION` = "2", `uv run just generate` (snapshots, index), golden migration of
  `tests/golden/statements.reorganise` [R5, R7, R12, R18; AC6, AC15, AC16]
- [ ] T15 Analyses report nested diagnostics once; goal seek rebases the solved evaluation;
  pointer fields renamed on the input and the output side (decision 13), the four `VERSION` =
  "2", `uv run just generate`; golden migration of the `analysis.*` cases on the list [R12,
  R14, R18; AC10, AC12, AC15, AC16]
- [ ] T16 Golden case for the envelope: one new case per bumped method family with
  `expect_diagnostics`, source noted [R5 to R7; AC4 to AC6]
- [ ] T17 Envelope snapshot `schemas/envelope/v1.json`, `envelope` entry in
  `schemas/index.json`, immutability check extended [R17; AC15]

## Adapters and docs
- [ ] T18 [P] Python `Result.diagnostics` and the properties `not_computed`, `warnings`,
  `checks`, `defaults_applied`; `python/tests/test_envelope.py` [R15; AC13]
- [ ] T19 [P] Pass-through tests: CLI `fairmark run`, function-calling dispatcher, MCP server
  equal the core envelope [R15; AC13]
- [ ] T20 [P] `docs/en/reference/warnings.md` with nav entry, envelope reference, methodology
  pages of the seven bumped methods, agent skill [R16, R17]
- [ ] T21 Italian edition via `/translate`: `docs/it/reference/warnings.md` and every page of
  T20, glossary terms [R16]
- [ ] T22 [P] Release notes `changes/259.breaking.md` and `changes/259.feature.md`

## Verification
- [ ] T23 `uv run just generate`, `uv run just check`, `uv run just docs` green; every AC ticked
  in `spec.md`
- [ ] T24 `code-reviewer` pass on the branch; `numerical-auditor` confirms AC16 (no value
  changed, golden keys moved only)
