# Golden cases

The golden cases live in `tests/golden/<method>/<case>.yaml`. A golden case is one input of
one method and the values an independent source expects from it. Rust
(`crates/fairmark-core/tests/golden.rs`) and Python (`python/tests/test_golden.py`) run every
case. How to write one: the `golden-case` skill
(`.agents/skills/golden-case/SKILL.md`).

## Golden cases assert values, schema snapshots assert shape

Decided by the maintainer on 2026-11-17 (SPEC-0013, issue #351). A golden case fails when a
value changes, not when the layout of the envelope changes. The layout belongs to the schema
snapshots in `schemas/` and to `python/tests/test_golden_conformance.py`, which validates every
golden result, success and error, against the snapshots committed for the versions it
declares, in closed mode, and checks the documented order of every envelope list.

A golden case may assert, each value from an independent source:

- numbers, dates, codes (errors, warnings, reason codes, statuses, enumerated values, check
  ids) and pointers the user acts on;
- that an output is not computed: `null` in `expect`, or an entry of `expect_not_computed`;
- the completeness of errors, warnings, outputs not computed and checks, with `expect_counts`;
- positions inside `value`, where the order is itself a result (years, periods, schedule rows,
  ranks).

A golden case does not assert:

- where a list lives in the envelope, or the position of an entry in a list whose order is a
  presentation rule (defaults applied, diagnostics, errors);
- the presence of keys, the envelope structure or `meta` bookkeeping (`envelope_version`,
  `engine_version`, `input_hash`);
- wording: `message`, `fix`, or a `reason` outside an error entry. The cases migrated by
  SPEC-0013 (`specs/0013-golden-values-schema-shape/golden-migration.json`) keep the wording
  they asserted; when a change of wording touches one, the assertion is removed with the
  maintainer's approval, not updated.

## Sections

| Section | Reads | Keys |
| --- | --- | --- |
| `expect` | `value` | a top-level field, or an RFC 6901 pointer relative to `value` |
| `expect_defaults` | `meta.defaults_applied` | the input `pointer` of the default |
| `expect_not_computed` | `diagnostics.not_computed` | the result `pointer` of the output |
| `expect_warnings` | `diagnostics.warnings` | `code`, one space, `pointer` (`-` when null) |
| `expect_checks` | `diagnostics.checks` | `pointer`, one space, `id` |
| `expect_errors` | `errors` of the error report | `code`, one space, `pointer` (`-` when null) |
| `expect_counts` | a list | `errors`, `warnings`, `not_computed`, `checks`, or a pointer into `value` |
| `expect_error` | the error raised | its code |
| `expect_error_report` | the error report | a top-level field: the first error, as the typed API raises it |

In the five sections keyed by an entry, each key maps to RFC 6901 pointers relative to the
entry, compared like `expect`; `{}` asserts only that the entry exists. A key that names no
entry, or more than one, fails. A count inside an entry is written with the suffix `#count`,
for example `/missing_any_of#count: "2"`. Codes and ids hold no space and a pointer never starts
with one, so a key reads one way only.

```yaml
expect:
  /years/0/balance_sheet/operating/invested_capital: '1468000'
expect_defaults:
  /years/0/statutory/balance_sheet/cash: {/value: net_financial_position_cash}
expect_not_computed:
  /value/interest_cover_ebitda: {/missing_any_of/0/0: /ebitda_definition}
expect_counts:
  not_computed: '1'
```

## Rules

- `null` means not computed: it passes when the value is `null` or when the last token of the
  key is absent under an existing object. Every other expectation fails on an absent key. The
  conformance test fails a key the schema does not declare, so a typo never passes.
- A decimal is compared within `tolerance`; any other text exactly.
- A new case that asserts entries of `errors`, `warnings` or `not_computed` states their count
  in `expect_counts` (`test_golden_case_shape` checks it).
- If two error entries ever share a code and a pointer, the key is extended with `reason` by a
  decision recorded here first.
- Existing golden files are protected (`.claude/hooks/protect_golden.py`,
  `scripts/check_golden_changes.py`): their expected values change only through a reviewed
  `numerical-change`, and an approval covers only the files it names.
