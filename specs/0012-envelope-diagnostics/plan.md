# SPEC-0012 — Plan

Status: active (SPEC-0012 approved on 2026-11-16). Issue #259. One branch,
`feat/259-envelope-diagnostics`, one pull request in the numerical-change tier for its review
gate (method versions are bumped and golden keys move), although no computed number changes.

## Design

### Module layout in `crates/fairmark-core`
- **`diagnostics.rs` (new, scope `explain`).** The diagnostics types, all
  `Serialize + JsonSchema` with a doc comment on every field:
  - `Diagnostics { not_computed: Vec<NotComputed>, warnings: Vec<Warning>, checks: Vec<Check> }`,
    every field serialised even when empty (no `skip_serializing_if`), so the schema lists them
    as required (R3).
  - `NotComputed { pointer, reason_code: ReasonCode, reason: String, missing_any_of:
    Vec<Vec<String>>, permitted: Vec<Permitted> }` and `Permitted { pointer, values }`;
    `ReasonCode` enum `missing_input`, `choice_required`, `not_defined`, `not_applicable` (R5).
  - `Warning { code: WarningCode, pointer: Option<String>, message: String }`. `WarningCode` is a
    Rust enum serialised as its `FM-Wnnn` string, with `code()`, `root()` (input or envelope) and
    a doc comment per variant; the schema states the pattern `^FM-W[0-9]{3}$` and lists the
    known values (R8). Adding a variant is additive (G8).
  - `Check { id, status: CheckStatus, pointer, pointers }`, reusing `CheckStatus` of the
    statements, moved here (R7).
  - A collector `Diagnosed` owned by `Explain` for the duration of a computation:
    `ex.warn(code, pointer, message)` pushes the `Warning` and the echo
    `warning FM-Wnnn: <message>` into `conventions` in one call, so the two cannot diverge (R9,
    R13). `Explain::convention` refuses, in debug builds, a text starting with `warning`, and
    the test of T9 checks the sources (R16).
- **`not_computed.rs`.** `NotComputedEntry` and its `Permitted` are replaced by the types above;
  the module keeps a converter from the method-internal form (`output`, `missing`,
  `permitted`, `reason`) to `NotComputed`, so `NotComputedList` (ratios) and `Outputs`
  (statements) change only their final step. A ratio listed by `NotComputedList` keeps that entry
  (`missing_input`) and its `not_defined` text is dropped (decision 14).
- **`explain.rs`.** `Explain` loses `method` (R2) and gains the private diagnostics collector.
  `DefaultApplied.field` is renamed `pointer` (R10); `Meta` gains `envelope_version: u32`
  (constant `ENVELOPE_VERSION = 1`) and serialises `defaults_applied` always (R3, R4).
  `Valuation` gains `diagnostics` between `explain` and `meta` (R1).
- **`method.rs`.** `run_typed` returns the diagnostics with the output; methods emit through
  `Explain`, so the `Method` trait signature of `compute` does not change. A method that lists
  outputs not computed or checks hands them over through two new `Explain` calls
  (`ex.not_computed(entries)`, `ex.checks(entries)`), called at the end of the computation.
- **`registry.rs` `execute`.** The rounding warnings of `reading::rounded_on_reading` become
  `Warning` values (FM-W001) pushed before the method's own warnings; the goal-seek filter on
  `/base_input{variable}` compares pointers instead of text. The analysis preview convention
  becomes FM-W016 (`analysis/*`). The envelope is assembled with `diagnostics` (R1).
- **Statements (scope `statements`).** `YearOutput` loses `checks` and `not_computed`; `Checks`
  produces `Check` entries with `pointer` `/value/years/<i>` (R7). The pointer fields of R12 are
  renamed. `Reorganise::VERSION` = "2" (R18).
- **Ratios (scope `ratios`).** `RatioValue` keeps `value` only; `not_defined` reasons and the
  `NotComputedList` entries become `NotComputed` entries with `reason_code` `not_defined` or
  `missing_input`, pointers prefixed `/value` (R5, R6). Each output type loses `not_computed`.
  The six `VERSION` constants become "3" (R18).
- **Analyses (scope `analysis`).** `registry::evaluate` returns the inner envelope; the
  analyses read its `diagnostics`: warnings of the base evaluation once, input pointers prefixed
  `/base_input`; `analysis.goal_seek` adds the inner `not_computed` and `checks`, result pointers
  rebased from `/value/...` to `/value/inner_value/...` (R14). The pointer fields of decision 13
  are renamed in the input and output types (serde names), the four `VERSION` constants become
  "2" (R12, R18).

### Building blocks reused
`registry::execute` stays the one place that builds the envelope; `ratios::pointer_cmp` orders
`not_computed` (R13); `reading::rounded_on_reading` is reused, returning structured findings
instead of text; the schema walk of `reading.rs` is reused by the pointer-naming test (AC10).

### Adapters
- CLI `fairmark run` already prints the core JSON; a test pins it (R15).
- Python `_core.py`: `Result` gains `diagnostics: dict[str, Any]` (decoded by the result
  schema: pointers stay `str`) and the properties `not_computed`, `warnings`, `checks`,
  `defaults_applied`; `run` reads them from the envelope. `tools.py` and `mcp_server.py` return
  `run_json` output unchanged; tests pin it.

## Schema changes
- **Envelope (version 1):** `diagnostics` added; `meta.envelope_version` added;
  `meta.defaults_applied` required and renamed key; `explain.method` removed. Snapshot
  `schemas/envelope/v1.json`, generated by `uv run just generate` from the result schema of a
  method with its `value` replaced by `{"type": "object"}`; `schemas/index.json` gains an
  `envelope` entry with the same fields as a method entry. `scripts` schema immutability check
  extended to it.
- **Methods (breaking output changes, R18):** `statements.reorganise` v2, `ratios.*` v3 (six);
  `analysis.goal_seek`, `analysis.scenarios`, `analysis.sensitivity`, `analysis.tornado` v2
  (breaking input and output renames of decision 13, inputs no longer copied unchanged).
  Other input schemas copied unchanged into `v<N>.json`; output schemas `v<N>.output.json` without
  `not_computed`, `checks`, `not_defined`, with the renamed pointer fields. Commit trailers
  `Numerical-change:` are not used (no number changes); the commit bodies state
  `Breaking-change: <method> v<N> -> v<N+1>` per the release notes rules.
- **No input change outside the analyses (decision 13), no new error code.**

## Numerical approach
None: no formula changes. Every value in `value` keeps its digits; AC16 proves it.

## Test strategy
- **Inventory first (T1).** `scripts/envelope_inventory.py` (read-only) lists every
  `warning` literal, every output type field named `not_computed`, `checks`, `not_defined`, every
  `DefaultApplied::new`/`destination` call and every output field whose doc comment mentions a
  JSON pointer. Its output is compared with the inventory of the spec; differences are added to
  the spec before T4.
- **Golden migration list (T3).** `scripts/golden_envelope_migration.py list` reads every
  `tests/golden/**/*.yaml` and prints, per file, each `expect`, `expect_length` and `expect_meta`
  key that the change moves or renames, with its new key. `apply` performs the moves, only on the
  files named in the maintainer's approval (passed as a file list). The list is attached to the
  pull request and to the issue for named approval before any golden file is touched; T11,
  T13, T14 and T15 each migrate the keys of their own change in the same commit, so every
  commit passes `just check`.
- **Unit and property tests** (Rust, in the modules they test): AC1 to AC3, AC7 to AC11, AC14 in
  `registry.rs`, `diagnostics.rs` and the method modules; AC4 to AC6 in `ratios/*.rs` and
  `statements/*.rs`; AC12 in `analysis/*.rs`. AC17 (decision 14): a unit test in
  `ratios/debt_service.rs` (no debt schedule: `/value/llcr` once, `missing_input`) and one in
  `ratios/profitability.rs` (negative invested capital without leverage inputs:
  `/value/roe_leverage/roe` and `/value/roe_leverage/cost_of_debt` once each, `missing_input`,
  no `not_defined` entry with their pointer).
- **Schema tests:** AC10 (pointer naming), AC15 (snapshots, no forbidden fields in current
  output schemas) in `registry.rs` and `scripts/schemas` checks.
- **Adapters:** AC13 in `python/tests/test_envelope.py` (Python, tools, MCP) and a CLI test in
  `crates/fairmark-cli/tests/`.
- **Golden runners:** Rust `crates/fairmark-core/tests/golden.rs` and the Python runner gain
  `expect_diagnostics` and `expect_diagnostics_length` (T2), tested on a temporary case.
- **AC16:** in each migrating task, `golden_envelope_migration.py verify` compares every expected value at its
  new key with the value at its old key on `origin/main`.

## Documentation
- `docs/en/reference/warnings.md` (new, nav entry beside `errors.md`): the catalogue table of
  the spec, the pointer root rule, the echo format, "codes are never reused".
- `docs/en/reference/` page on the result (the envelope table of the spec), the agent skill
  `skills/fairmark/SKILL.md` (read `diagnostics`, not conventions), the methodology pages of
  `statements.reorganise` and the six ratios (where not computed and checks now live), the
  Python docstrings of `Result`.
- `docs/it/`: Italian editions through `/translate` with `i18n/glossary.yaml` (new terms:
  diagnostics, warning code).
- ADR 0014 G4 amendment (R20) and its Italian edition if the ADR is translated.
- Release notes: `changes/259.breaking.md` (envelope and the seven method versions) and
  `changes/259.feature.md` (coded warnings, catalogue).

## Risks and alternatives
- **Golden churn.** Many files move keys at once. Mitigation: a script, a reviewed list, and
  AC16 proving no value changed.
- **Two more bumps soon** (#263, #266). Accepted by the maintainer (decision 12); each pull
  request snapshots its own version.
- **Warning text drift.** The message is guidance, not contract; tests assert code and pointer,
  and the message of today once (AC7) so that the move changes no wording by accident.
- **Phased versioning** (diagnostics first, removal later) was rejected by the maintainer:
  it duplicated lists on main and needed a release gate.
- **A `warnings` field computed from conventions** (parse `warning: ` text) was rejected: it
  keeps the English text as the source of truth, the opposite of G4.
