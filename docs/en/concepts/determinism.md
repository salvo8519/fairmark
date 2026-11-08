# The determinism contract

Fairmark promises that the same input produces the same output, to the last digit, on every
supported platform and Python version, for a given engine and method version. This document is
the contract; CI enforces it on every commit.

## Rules

1. **Decimal arithmetic only.** All calculations use `rust_decimal` (96-bit mantissa, 28–29
   significant digits). No `f32`/`f64` appears in any calculation path.
2. **Exact parsing.** JSON numbers are parsed directly into decimals (`arbitrary_precision`),
   never through binary floating point. Outputs serialize decimals as strings so that no
   consumer (JavaScript, spreadsheets) silently loses precision.
3. **Integer powers are exact.** Discount factors with integer exponents use repeated
   multiplication. Fractional exponents (mid-period convention) use decimal `powd`, whose
   precision is verified against an independent 50-digit reference in the golden tests.
4. **Reproducible solvers.** Iterative methods (IRR, and in the future circular WACC) use a
   fixed algorithm, starting point, tolerance and iteration cap. The solver used and the
   number of iterations are returned in the result.
5. **No silent defaults.** Defaults exist only where documented in the input schema. Unknown
   fields are rejected (`FM-E001`), so a typo never falls back to a default. A default that
   the engine fills in is reported in `meta.defaults_applied` and in `explain.conventions`.
6. **Fingerprinting.** `meta.input_hash` is the SHA-256 of the canonical parsed input, with
   its documented defaults filled in: an input that omits a field and the same input with
   the default written out have the same hash and the same value.
   `fairmark verify` re-runs a saved result and checks hash and value.
7. **Numerical versioning.** If any golden case changes result for the same input, the PR
   carries the `numerical-change` label, bumps `method_version` and a commit trailer
   `Numerical-change: <method> v<N> -> v<N+1>`, which the generated CHANGELOG lists first.

## Golden cases

`tests/golden/<method>/*.yaml` hold reviewed input/expected pairs. Expected values come from
independent sources: reference implementations (`scripts/reference_values.py`), worked examples
from practitioners, or published examples re-derived by hand. Both the Rust and the Python
test-suites run every case.

Real client data must never be committed: anonymise or reconstruct cases.
