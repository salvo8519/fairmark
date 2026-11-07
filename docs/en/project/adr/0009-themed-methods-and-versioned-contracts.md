# ADR 0009 — Methods as typed units per theme, one registry list, versioned schemas

Status: accepted · 2026-11-07

## Context
The engine grows by themes (time value, rates, cost of capital, DCF, income and asset methods,
ratios, analysis) and will reach dozens of methods. Until now each method was a free function
registered by hand in three places of `registry.rs` (catalogue, schema lookup, dispatch); a
reviewer found entries missing in one of them. Schema snapshots kept only the latest schema of
each method, so the input contract of an earlier method version could not be shown. Future
methods (DCF, analysis) compose other methods, which needs a uniform contract.

## Decision
1. **Themes are modules.** Every method type is defined in the module of its theme (`dates`, `rates`,
   `tvm`, `coc`, `dcf`, and later `tv`, `ratios`, `analysis`). Method ids are
   `<theme>.<name>`; a test enforces the convention, uniqueness and that the type of `x.y` is
   defined in `fairmark_core::x`.
2. **Every method implements the `Method` trait** (`method.rs`): id, version, title,
   description, `Input`, `Output` and `compute`. Metadata lives next to the code it describes.
   Rust has no class inheritance; shared behaviour is expressed with traits and shared data
   with composition, which keeps each method independent and testable.
3. **One registry list.** `registry.rs` builds the catalogue, the schema lookup and the dispatch
   from a single list of method types through a macro. Adding a method is: implement `Method`,
   add one line. Registration is explicit (no link-time self-registration), so the order is
   deterministic and the full list is reviewable in one place.
4. **Versioned schemas.** `schemas/<method>/v<N>.json` stores the input schema of every method
   version, with `$id` `urn:fairmark:schema:<method>:v<N>`; `schemas/index.json` records the
   current version, each file, its SHA-256 and the release that introduced it (`unreleased`
   until `scripts/version.py bump` stamps it). "Published" means present on `main`: files of
   earlier versions are immutable, index entries are append-only, a current version never
   goes back, a published version may change only additively and a breaking change requires
   a new method version. A method is never removed (an integration may depend on it); it can
   be deprecated in its description. CI enforces these rules with no override. This amends ADR 0003,
   which kept one flat snapshot per method. Version 1 of `tvm.npv`, `tvm.irr` and `dcf.fcff`
   was reconstructed from commit b4f2867, the last state before their version 2.
5. **Domain types and theme traits come with their first consumers.** `Rate` (value,
   compounding, period basis) and `CashFlowSeries` (periodic or dated, with its time axis) are
   introduced with the multi-stage DCF, the first method that composes others; a `Ratio` trait
   (formula, numerator, denominator, balance convention, not-defined handling) is introduced
   with SPEC-0007. Introducing them before a consumer exists would fix their shape by guesswork.
   JSON inputs keep plain decimals with documented units; domain types are internal.
6. **Contract cases are data.** Extreme inputs and expected conventions of each method live
   in `tests/contracts/<id>.yaml`, read by one integration test; adding a method adds a file
   and touches no shared test.

## Consequences
+ No bottleneck when adding methods; catalogue, schemas and dispatch cannot drift.
+ Every input contract ever published remains available and verifiable by hash.
+ Composition between methods (DCF using `coc` and `tv`) goes through typed Rust calls, not JSON.
− Method metadata is spread across theme modules; the generated reference
  (`docs/en/reference/methods.md`) remains the single readable catalogue.
− A breaking input change now costs a new method version and file; this is intended.
