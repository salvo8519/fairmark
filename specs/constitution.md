# The Fairmark constitution

These principles are non-negotiable. Every spec, plan, PR and review is checked against them.
Changing one requires an ADR and the maintainer's approval.

## I. Deterministic by construction
- All calculations use exact decimal arithmetic (`rust_decimal`). No `f32`/`f64` in any
  calculation path, in any language.
- Iterative algorithms have a fixed method, starting point, tolerance and iteration cap, and
  report them in the result.
- The same input and versions always produce the same output, bit for bit, on every platform.

## II. Auditable
- Every method returns `explain`: formula, conventions and every intermediate step with the
  actual numbers substituted.
- Every convention that changes a number is explicit: an input parameter or a reported
  convention. Nothing is implicit.

## III. No silent defaults
- Unknown input fields are rejected. Defaults exist only where documented in the schema.
- Invalid inputs produce typed errors with stable codes. Never a guess, never a NaN.

## IV. One engine, thin adapters
- Financial logic lives only in `crates/fairmark-core`. Python, CLI, MCP, n8n and WASM are
  adapters that translate and never compute.
- The registry and its JSON Schemas are the single source of truth for every adapter.

## V. Private by design
- The core performs no I/O and has no network dependencies.
- No financial data ever leaves the user's machine through Fairmark.

## VI. Verified against independent sources
- Every method has golden cases whose expected values come from a source independent of the
  implementation (reference script, practitioner model, published example re-derived).
- Expected values in golden cases are changed only through a reviewed `numerical-change`.

## VII. Methodology is public
- Every method has a documentation page with formula, conventions, example and sources.
- Practice that varies across jurisdictions is modelled as explicit profiles, not hidden logic.

## VIII. Simple to use, hard to misuse
- Units are explicit in names and descriptions (rates are decimal fractions).
- Inputs map to how analysts think (financial statements, business plans), not to internals.
