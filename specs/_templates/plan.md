# SPEC-NNNN — Plan

## Design
Types, functions, module layout in `crates/fairmark-core`. How it reuses existing building
blocks (tvm, explain, registry).

## Schema changes
New/changed input fields, defaults, classification (additive vs breaking), `method_version` bump.

## Numerical approach
Formulas in implementation order, exactness considerations (integer powers, divisions,
fractional exponents), solver choices.

## Test strategy
Golden cases (source of each), property tests, error cases. Which AC each test proves.

## Documentation
Methodology page, reference regeneration, examples.

## Risks and alternatives
What could go wrong; options considered and why rejected (link an ADR if structural).
