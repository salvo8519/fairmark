# ADR 0010 — Analysis methods compose through the registry

Status: accepted · 2026-11-07 (maintainer decision recorded in SPEC-0008, Decisions)

## Context
ADR 0009 states that methods compose through typed Rust calls: the multi-stage DCF knows at
compile time that it calls the cost of capital and the terminal value. Analysis methods
(SPEC-0008: goal seek, sensitivity, scenarios, tornado) are different: the method they
evaluate is chosen by the caller at run time. Typed composition would need one analysis type
per inner method.

## Decision
1. Methods of the `analysis` theme evaluate their inner method through the registry, on JSON
   values, inside `fairmark-core`. The inner method parses, validates and computes exactly as
   it does for any adapter; the result is the same envelope that `run` returns.
2. Composition is one level deep: an analysis method cannot evaluate another analysis method
   (`FM-E017`). This bounds recursion and cost.
3. Every analysis result records the inner method id and version actually run.
4. One request performs at most 10,000 inner evaluations (`FM-E016`), checked before any
   evaluation where the count is known in advance.
5. Every other theme keeps typed composition (ADR 0009). Run-time composition is the
   exception for generic tools, not a second way to build methods.

## Consequences
+ One generic contract for agents: any method, any decimal input, any decimal output.
+ No drift: the inner method is validated by its own schema and code, never by a copy.
+ The cost of an analysis is bounded and predictable.
− Each evaluation parses and serialises JSON and builds an explain that is then discarded;
  accepted in v1, measured by the budget test of SPEC-0008.
− The input schema of an analysis method cannot describe `base_input`: it is validated at run
  time by the inner method, and analysis schemas do not change when an inner method does.
