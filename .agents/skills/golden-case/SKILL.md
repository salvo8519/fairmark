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
   Keys of `expect` are top-level fields or JSON Pointers (`/years/0/total`) into the result
   `value`. To assert documented defaults, add `expect_meta` with JSON Pointers into `meta`,
   for example `/defaults_applied/0/field`; it is compared like `expect` and protected like it.
   To assert that a list has no entry beyond those asserted, add `expect_length` with JSON
   Pointers into `value` and the expected count, for example `/not_computed: "3"`.
   An error case (`expect_error`) may add `expect_error_report` with JSON Pointers into the
   error report (ADR 0014 G5), for example `/pointer`, `/reason`, `/pointers/1` or
   `/errors/2/pointer`; values are compared exactly (`null` for a null field) and protected
   like `expect`. In an error case, `expect_length` counts arrays of the error report, for
   example `/errors: "3"`, so that no error beyond those asserted is reported. Do not assert
   `/fix`: its wording is not part of the contract.
3. Use only anonymised or synthetic data.
4. Run `cargo test -p fairmark-core --test golden` and `uv run pytest python/tests/test_golden.py`.
5. Existing golden files are protected by a hook: never edit them; propose a
   `numerical-change` to the maintainer instead.
