# SPEC-0013 — Plan

## Design
No engine code changes. The work is in the two golden runners, two Python test modules, the
golden-case skill, `tests/README.md`, `pyproject.toml` and the migration script.

**Semantic resolver, one per runner.** Both runners gain the same small function: given the
part of the result (`meta`, `diagnostics`, or the error report), the list name, the identity
fields of R1 and a key, return the one entry whose identity, joined by one space with `-` for
`null`, equals the key; zero or several entries are a failure (R2). The fields of an entry are
then read with the existing pointer lookup and compared with the existing comparison (decimal
within `tolerance`, exact text, `null`). The table of R1 is one constant in each runner: when
an envelope version moves a list, that constant changes and the golden files do not.

- Rust (`crates/fairmark-core/tests/golden.rs`): `Case` gains six `BTreeMap` fields
  (`expect_counts: BTreeMap<String, String>`, the five sections as
  `BTreeMap<String, BTreeMap<String, String>>`), still `deny_unknown_fields`. `check_result`
  and the error branch call `check_sections`; `null` handling moves into one function used by
  every comparison (R4).
- Python (`python/tests/test_golden.py`): `SECTIONS` table, `find_entry`, `check_sections`;
  `compare` gains the null-or-absent rule through `lookup` returning a sentinel for an absent
  last token. `NEW_CASE_KEYS` gains the six sections.
- `scripts/golden_values_migration.py` already holds the reference resolver
  (`semantic_paths`) used by `verify`; the runners follow it, and T2 tests it.

**Conformance test (`python/tests/test_golden_conformance.py`).** Reads the schema history
through `schema_history()` when it exists (ADR 0017, #335), else `schemas/index.json`, only to
check that the declared version exists; the files read are always
`schemas/<method>/v<N>.output.json` and `schemas/envelope/v<N>.json` with `N` taken from the
result. `closed(schema)` adds `unevaluatedProperties: false` as in R7; validators are built
once per snapshot (`functools.cache`). Inner analysis values: walk the output schema, and for
each node with `x-fairmark-output-of` resolve the relative pointer to the inner method id in
the result and validate against that method's snapshot at the version named next to it
(`inner_method.version`). The key-declared check (R8) reuses the schema walk of the migration
script (`_branches`, `tokens`, `resolve`, `schema_history`), which the test loads as a module:
one walk, one reader of the schema history.

**Order test.** In the same module: one function per list returning the sort key of R9, and a
check that each list equals itself sorted by it. Input order of defaults: the place of the
default's pointer in a depth-first walk of the input taken with its defaults, objects in the
property order of the input schema (read from the engine: the committed snapshots sort their
keys), arrays by index. Warnings: method warnings first, then `FM-W001`, then `FM-W017`, each
of the last two by pointer.

## Schema changes
None. No method version bump. The envelope snapshot is unchanged.

## Numerical approach
None: no computation. Expected values are moved verbatim; `verify` proves it (R14).

## Test strategy
Test first inside every task: the new test is run and seen to fail before the change that
makes it pass, and the task is one green commit.

| Test | Proves |
| --- | --- |
| `python/tests/test_golden_values_migration.py` (synthetic results, temporary trees) | AC13 |
| Temporary cases in `golden.rs` and `test_golden.py` | AC1 to AC5 |
| `test_golden_conformance.py` on every golden case and on synthetic documents | AC6 to AC9 |
| `uv lock --check` and a comparison of package names with `main` | AC10 |
| `test_golden_case_shape` on temporary new cases | AC12 |
| `verify` against `origin/main` | AC14 |

## Documentation
`tests/README.md` (new) and the golden-case skill; `uv run just generate` mirrors the
skill to `.claude/`. No page of `docs/` describes the golden format, so no translation is
needed. No release note: nothing a user sees changes (label `no-release-note`).

## Risks and alternatives
- **The list changes before approval.** Any golden file added or edited on `main` changes the
  plan: T9 starts by running `list` again; a different list is approved again.
- **ADR 0017 lands first.** `schemas/index.json` disappears; the script and the conformance
  test already read the history through `schema_history()` when present.
- **Closed mode too strict for a future schema** (a flattened enum whose parent declares no
  `properties`). The rule closes only objects that declare `properties`; a branch is closed by
  its parent. A failure is fixed in the snapshot generator, not by loosening the test.
- **`null` or absent weakens typo detection in the runners.** R8 moves the guard to the
  conformance test, which runs on every case.
- Rejected: selectors inside `value` (decision 1); the Rust `jsonschema` crate (decision 6,
  design 3.3); dropping wording assertions during the migration (decision 2).
