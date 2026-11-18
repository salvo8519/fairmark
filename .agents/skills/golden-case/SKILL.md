---
name: golden-case
description: Author a golden test case for a Fairmark method with expected values from an independent source. Use when adding coverage for a method or an acceptance criterion.
---

1. Choose the source of truth, in order of preference: a practitioner model supplied by the
   maintainer; a published worked example re-derived by hand; an independent reference
   implementation in `scripts/reference_values.py` (Python `decimal`, precision 50, written
   from the formula, not from the Rust code).
2. Create `tests/golden/<method>/<kebab-case-name>.yaml`:
   ```yaml
   # Source: <where the expected values come from>
   method: <method id>
   description: <one line>
   input: { ... }            # amounts and rates as strings, e.g. "0.085"
   expect: { field: "value" }
   tolerance: "0.000000001"
   ```
   Golden cases assert values; the schema snapshots and
   `python/tests/test_golden_conformance.py` assert shape (SPEC-0013). The full rule is in
   `tests/README.md`; in short:
   - `expect`: top-level fields or JSON Pointers (`/years/0/total`) relative to `value`.
     Positions in `value` are allowed: there they are results (period, year, rank).
     `null` means not computed: the value is `null` or the key is absent; the key must be
     declared by the output schema.
   - Envelope lists are asserted by the identity of the entry, never by position, each entry
     a mapping of pointers relative to it (`{}` asserts only that it exists):
     `expect_defaults` (key: input `pointer`), `expect_not_computed` (key: result `pointer`),
     `expect_warnings` (key: `code pointer`, `-` for a null pointer), `expect_checks`
     (key: `pointer id`), `expect_errors` (key: `code pointer`). A count inside an entry is
     written `/missing_any_of#count`.
   - `expect_counts`: `errors`, `warnings`, `not_computed` or `checks`, or a pointer into
     `value`, with the expected count. Required whenever entries of `errors`, `warnings` or
     `not_computed` are asserted.
   - An error case has `expect_error` (the code), may add `expect_error_report` for the
     top-level fields of the report (`/pointer`, `/reason`, `/expected/0`) and
     `expect_errors` for its entries.
   - Do not assert wording: `message`, `fix`, or a `reason` outside an error. Do not assert
     envelope structure or `meta` bookkeeping.

3. Use only anonymised or synthetic data.
4. Run `cargo test -p fairmark-core --test golden` and `uv run pytest python/tests/test_golden.py`.
5. Existing golden files are protected by a hook: never edit them; propose a
   `numerical-change` to the maintainer instead.
