# SPEC-0013 — Golden cases assert values, schema snapshots assert shape

| | |
| --- | --- |
| Status | Approved (maintainer, 2026-11-17) |
| Milestone | M3 · Agent layer and v0.1 release |
| Owner | @salvo8519 |
| Formula basis | No formula: a change of the test contract that alters no computed number and no expected value. Basis: the design in [`design.md`](design.md) and the maintainer decisions below; RFC 6901 for every pointer; JSON Schema 2020-12 for the snapshots. |
| Related | Issue #351; #259 and #361 (SPEC-0012, 131 golden files migrated); #266, #263 (the contract changes this prepares); #352 (catalogue of codes); #335 and ADR 0017 (schema history); ADR 0014 G4, G5, G8; constitution I, VI, IX |
| Commit scope | Existing scopes: `specs` (this folder), `scripts` (migration script), `core` (Rust golden runner), `py` (Python golden runner and tests), `deps` (dev group), `agents` (golden-case skill). No new scope. |

## Problem
A golden case should fail when a value changes, not when the layout of the envelope changes.
SPEC-0012 changed no number but touched 131 golden files, because 1,437 value assertions in 119
files reach their value through the layout of the envelope: the position of an entry in
`meta.defaults_applied`, `diagnostics.not_computed`, `diagnostics.checks`,
`diagnostics.warnings` or the `errors` of the error report, and the part of the envelope the
list lives in. The next contract changes (#266, #263) would repeat that cost. This spec moves
the layout to the schema snapshots and one conformance test, and lets golden cases address
values by what they are.

## Scope
**In scope:**
- Semantic sections of the golden format, read by both runners (Rust and Python):
  `expect_defaults`, `expect_not_computed`, `expect_warnings`, `expect_checks`,
  `expect_errors`, `expect_counts`.
- `null` in an expectation meaning "not computed": the value is `null` or the key is absent.
- A schema-conformance test of every golden result, success and error, against the committed
  snapshots of the versions the result declares, in closed mode, including the inner values of
  analysis results.
- A test of the documented order of every envelope list.
- `jsonschema` declared in the `dev` dependency group.
- The rule in the golden-case skill and in a new `tests/README.md`.
- Tests of `scripts/golden_values_migration.py`, then the migration of the files the maintainer
  approves by name, then the refusal of the retired keys.

**Out of scope:**
- Any change of the engine, of a schema snapshot, of a computed number or of an expected value.
- Selectors inside `value` (decision 1).
- Enumerating in the schemas the identifiers they type as free strings (decision 9, #352).
- The contract changes of #266 and #263 themselves.

## Maintainer decisions of 2026-11-17
Approved at 23:33 Europe/Rome, with the design: the ten recommendations of
[`design.md`](design.md), section 6, are decisions.

1. **Paths inside `value`.** Plain RFC 6901 pointers relative to `value`, as today; no selector
   syntax. Revisit only if a contract change reorders or inserts entries of a keyed list of
   `value`.
2. **Wording.** New cases do not assert `message`, `fix` or any `reason` text. The 47 wording
   assertions of existing cases stay; when a change of wording touches one, it is removed with
   the maintainer's approval, not updated. If the closest-field suggestion of `fix` is
   contract, it becomes a field of its own.
3. **`null` means null or absent**, adopted in this spec, before #266, with the schema check of
   keys as the guard against a typo.
4. **Completeness.** Entries of a semantic section are "contains". Completeness is stated by
   `expect_counts`, required in new cases for `errors`, `warnings` and `not_computed` whenever
   entries of that list are asserted; not required for `expect_defaults`.
5. **Identity of an error entry** is its `code` and `pointer`. If a case ever needs two entries
   with the same code and pointer, the key is extended with `reason` by a decision recorded in
   `tests/README.md`.
6. **The conformance test runs in Python only**, with `jsonschema`. The Rust runner asserts
   values only.
7. **Inner values of analysis results** are validated against the snapshot of the inner method.
8. **Plan files are committed** (`golden-migration.json`, `golden-migration.md`), as SPEC-0012
   did; they may be removed after the migration if the maintainer asks.
9. **Identifiers the schemas do not enumerate** (467 assertions) are left to #352.
10. **This spec** turns the design into requirements and acceptance criteria, so the tests are
    written first.

## Golden files: approval before any change
No golden file is changed by this spec before the maintainer has approved, by name, the list in
[`golden-migration.md`](golden-migration.md) (119 files on `main` at `3aa3eec`; the approval is
recorded on issue #351). Tasks before T9 change no file under `tests/golden/`. If `main` changes
a golden file before T9, `list` is run again and the new list is approved again.

## Requirements (EARS)

Semantic sections

- **R1** — THE GOLDEN RUNNERS (Rust `crates/fairmark-core/tests/golden.rs`, Python
  `python/tests/test_golden.py`) SHALL read the sections below, each a mapping from the key of
  one entry to a mapping of RFC 6901 pointers relative to that entry, compared like `expect`
  (decimal within `tolerance`, other text exactly, `null` by R4); an empty mapping `{}` asserts
  only that the entry exists:

  | Section | List read (envelope v1) | Key of an entry |
  | --- | --- | --- |
  | `expect_defaults` | `meta.defaults_applied` | `pointer` |
  | `expect_not_computed` | `diagnostics.not_computed` | `pointer` |
  | `expect_warnings` | `diagnostics.warnings` | `code`, one space, `pointer` (`-` when null) |
  | `expect_checks` | `diagnostics.checks` | `pointer`, one space, `id` |
  | `expect_errors` | `errors` of the error report | `code`, one space, `pointer` (`-` when null) |

- **R2** — IF a key of a semantic section names no entry, or more than one, THEN THE RUNNERS
  SHALL fail the case with a message that names the section, the key and the number of entries
  found.
- **R3** — THE RUNNERS SHALL read `expect_counts`, whose keys are either a list name (`errors`
  in an error case; `not_computed`, `warnings`, `checks` in a value case) or a pointer into
  `value`, and whose values are counts; and a pointer relative to an entry ending in `#count`
  inside a semantic section, which asserts the length of that list.
- **R4** — WHEN an expectation is `null`, THE RUNNERS SHALL pass if the value at the key is
  `null` or the last token of the key is absent, and SHALL fail if the parent of the key does
  not exist. Every other expectation still fails when its key is absent.
- **R5** — THE RUNNERS SHALL keep `expect_error` (the code raised) and `expect_error_report`
  for the top-level fields of the report (the first error, as the typed API raises it), and
  SHALL refuse a key of `expect_error_report` that starts with `/errors/`.
- **R6** — WHEN the migration of T9 is merged, THE RUNNERS AND `test_golden_case_shape` SHALL
  refuse `expect_meta`, `expect_diagnostics`, `expect_diagnostics_length` and `expect_length`.

Shape

- **R7** — THE CONFORMANCE TEST (`python/tests/test_golden_conformance.py`) SHALL validate, for
  every golden case:
  - a success: the envelope against `schemas/envelope/v<meta.envelope_version>.json` and
    `value` against `schemas/<method>/v<meta.method_version>.output.json`, both read from the
    committed files of the schema history (through `schema_history()` once ADR 0017 provides
    it), never from the engine;
  - an error: `{"error": <report>}` against the definition named by `x-fairmark-error` in the
    envelope snapshot;
  - an analysis result: each value marked `x-fairmark-output-of` against the output snapshot
    of the inner method at the version the result names;
  - in closed mode: `unevaluatedProperties: false` added at test time to every object schema
    that declares `properties` and is not a direct branch of `anyOf`, `oneOf` or `allOf`; the
    committed snapshots are not changed;
  - with formats asserted (`FormatChecker`, at least `date`).
- **R8** — THE CONFORMANCE TEST SHALL fail a golden case whose assertion key, in `expect` or
  inside an entry of a semantic section, is not declared by the schema of the part it reads
  (array indices excepted).
- **R9** (pending maintainer sign-off; wording of the code review of #394) — THE ORDER TEST
  SHALL check, on every golden result, the documented order of every envelope list:
  - `meta.defaults_applied` in input order: a depth-first walk of the input taken with its
    defaults, objects in the property order of the method's input schema, arrays by index;
  - `diagnostics.warnings`: the warnings of the method first, in the order the method emits
    them; then every `FM-W001` ordered by pointer; then every `FM-W017` ordered by pointer; no
    warning of the method after them;
  - `diagnostics.not_computed` by `pointer`, array indices compared as numbers;
  - `diagnostics.checks` by result block, then `id`;
  - the `errors` of a report by `pointer` (none first), then `code`, then `message`.

  Each check SHALL fail on a reversed list. The engine departs from the input order of
  defaults in eight known places (`/stages/N/projection` before `/stages/N/periods` in
  `dcf.fcff` and `dcf.fcfe`; six pairs in `statements.reorganise`), listed in
  `KNOWN_DEFAULT_ORDER_DEPARTURES` of `python/tests/test_golden_conformance.py`; any other
  departure fails. The maintainer decides whether the engine follows the rule (a change of
  output order, its own issue) or the rule is restated; the list is then emptied.
- **R10** — `jsonschema` SHALL be declared in the `dev` dependency group of `pyproject.toml`
  with a lower bound the locked version satisfies, and the set of packages in `uv.lock` SHALL
  not change.

Rule and tooling

- **R11** — `.agents/skills/golden-case/SKILL.md` and a new `tests/README.md` (not inside
  `tests/golden/`, whose readers take every entry as a method folder; moved in T8) SHALL state
  the rule of decisions 1 to 5: what a golden case may and may not assert, the sections of R1
  to R5, the meaning of `null`, and the identity keys.
- **R12** — `test_golden_case_shape` SHALL refuse, in a case not listed in
  `golden-migration.json`, an entry of `expect_errors`, `expect_warnings` or
  `expect_not_computed` without the matching count in `expect_counts`, and a key whose last
  token is `message` or `fix`, or `reason` outside `expect_errors` and `expect_error_report`
  (where `reason` is an enumerated code) (decisions 2 and 4).
- **R13** — `scripts/golden_values_migration.py` SHALL be covered by tests on temporary copies
  and synthetic results, as `test_golden_envelope_migration.py` covers its predecessor.
- **R14** — THE MIGRATION (T9) SHALL run `apply` only on the files approved by name, and
  `verify` against `origin/main` SHALL report every approved file checked and 0 problems: the
  same expected text at a key that reads the same node of the same result, every other block
  byte for byte, nothing dropped.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Separator inside a composite key | one space | — | Codes and ids hold no space; a pointer never starts with one (design 2.2) |
| Null pointer inside a key | `-` | — | Not a pointer (a pointer is empty or starts with `/`), so it reads one way |
| Count inside an entry | suffix `#count` on the relative pointer | — | Design 2.2 |
| Closed mode | `unevaluatedProperties: false` at test time | — | JSON Schema 2020-12; snapshots stay open (ADR 0014 G8) |

## Computability, defaults and checks
Not applicable: no method output is added or changed.

## Acceptance criteria
- **AC1** (R1, R2) — On temporary cases and results written in the test, in both runners: each
  section passes on a matching result; fails on a wrong value, on a key naming no entry, on a
  key naming two entries, and on a section read from the wrong part (an `expect_warnings` key
  present only in `not_computed`).
- **AC2** (R3) — `expect_counts` passes and fails on a list name and on a pointer into `value`;
  `/missing_any_of#count` inside an entry passes and fails; both runners.
- **AC3** (R4) — `null` passes on a `null` value and on an absent last token; fails on an absent
  parent; a non-null expectation on an absent key fails; both runners.
- **AC4** (R5) — A case with `/errors/0/pointer` in `expect_error_report` is refused; top-level
  `/pointer` is still read.
- **AC5** (R6) — After T9, a temporary case with any retired key is refused by both runners and
  by `test_golden_case_shape`; no golden file holds a retired key.
- **AC6** (R7) — Every golden case passes the conformance test; on synthetic documents the
  test fails on: a field added to `value`, a field renamed, a required envelope key missing, a
  decimal written as a JSON number, an enumerated value not in the schema, a `date` that is not
  a date, an error report with an extra field, an inner analysis value with an unknown field,
  and a result whose `meta.method_version` names a snapshot it does not match.
- **AC7** (R7) — The test reads no schema from the engine (`fm.result_schema`,
  `fm.output_schema` are not called); `test_golden_results_match_their_result_schema` is
  removed from `test_output_schemas.py`.
- **AC8** (R8) — A temporary case with a misspelt key whose expectation is `null` fails the
  conformance test; the same key spelt right passes.
- **AC9** (R9) — Every golden result passes the order test (with the known departures of R9);
  a reversed list of each kind, a method warning after the reading warnings, reading warnings
  out of pointer order and a default out of input order fail it.
- **AC10** (R10) — `pyproject.toml` declares `jsonschema` in `dev`; `uv lock --check` passes and
  the package names in `uv.lock` are the same as on `main`.
- **AC11** (R11) — The skill and `tests/README.md` state the rule; `uv run just generate`
  leaves no difference in `.claude/`.
- **AC12** (R12) — A temporary new case with an `expect_errors` entry and no `errors` count is
  refused; one asserting `/fix` is refused; a file listed in the plan is not.
- **AC13** (R13) — Tests of the script: `list` on a temporary tree; `apply` refuses a file not
  in the plan and a file whose SHA-256 differs; `apply` keeps unchanged blocks byte for byte;
  `verify` reports a changed value, a key that names another entry, swapped values between
  entries, a changed block and an added entry; `list` on a migrated tree finds 0 files.
- **AC14** (R14) — `verify --base-root <origin/main checkout>` prints `119 checked, 0
  problems` (or the count of the list approved again) and the diff of T9 touches only the
  approved files.

## Interfaces
- Contract: no change to ADR 0014; the envelope, the error report and every schema snapshot are
  unchanged.
- Method ids: none added or changed. No method version bump.
- Golden format: sections added by R1 to R3; meaning of `null` by R4; sections retired by R6.

## Open questions
None: the ten questions of the design are decided above.
