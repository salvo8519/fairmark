# ADR 0001 — Rust core, thin adapters

Status: accepted · 2026-11-07

## Context
The engine must be deterministic, fast, and reachable from Python, a CLI, MCP, LLM function
calling, n8n (Node) and the browser (WASM). Re-implementing methods per channel would multiply
bugs and break the "same input, same output" promise.

## Decision
All financial logic lives in one Rust crate, `fairmark-core`, with no I/O. Every channel is a thin
adapter generated from the core's registry and JSON Schemas.

## Consequences
+ One implementation to test; new channels (Node, WASM) reuse the same binary logic.
+ Strong typing catches unit and convention errors at compile time.
− Cross-platform wheels must be built in CI (maturin-action).
− Analysts contributing methods need some Rust; mitigated by golden cases and methodology pages
  being contributions in their own right.
