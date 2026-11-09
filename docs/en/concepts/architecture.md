# Architecture

## Principles

- **One engine, thin adapters.** All financial logic lives in `fairmark-core` (Rust). Adapters
  translate, they never compute.
- **The registry is the single source of truth.** `registry.rs` lists every method with its
  version, its input type and its output type. JSON Schemas of both are generated from the Rust
  types (`schemars`), and every adapter is generated from them: Python helpers and the
  decoding of their results, CLI, MCP tools, function-calling definitions, documentation.
- **JSON at the boundaries.** Data crosses the Rust/Python boundary as JSON strings with
  decimals as strings. This keeps the FFI surface to five functions and makes every adapter
  testable with the same fixtures.
- **No I/O in the core.** The core is a pure function from input to result. Telemetry, files
  and network live only in adapters.

## Layout

```
crates/
  fairmark-core/    methods, decimal math, explain, registry, errors (no I/O)
  fairmark-cli/     `fairmark` binary: methods | schema | run | verify
  fairmark-py/      PyO3 bridge (5 functions), built by maturin as fairmark._engine
python/fairmark/    Pythonic API, function-calling tools, MCP server
skills/fairmark/    Agent Skill for Claude Code, Codex and compatible agents
tests/golden/       shared golden cases (Rust + Python)
docs/               architecture, determinism, errors, ADRs, methodology
roadmap/            roadmap.yaml → GitHub milestones, epics and tasks
```

## Adding a method

1. Implement it in the module of its theme, `crates/fairmark-core/src/<theme>.rs`: an `Input`
   struct (serde + `JsonSchema`, `deny_unknown_fields`, doc comment with units on every field),
   an `Output` struct (`Serialize` + `JsonSchema`, doc comment with units on the type and on
   every field) and a unit type that implements the `Method` trait (id, version, title,
   description, `compute`) (ADR 0009).
2. Add one line to the list in `registry.rs`; `uv run just generate` writes its input and
   output schemas to `schemas/<method>/v1.json` and `schemas/<method>/v1.output.json` and
   updates `schemas/index.json`.
3. Add golden cases in `tests/golden/<method>/` from an independent source.
4. Add a thin Python helper in `python/fairmark/<area>.py`.
5. Write the methodology page in `docs/en/methods/`.

Function-calling tools, MCP tools and CLI support come for free.

## Performance

The engine is native code with no allocation-heavy abstractions; a full DCF with explain runs in
microseconds. Decimal arithmetic is slower than floating point, by design: in valuation,
exactness matters more than nanoseconds, and the engine remains orders of magnitude faster than
any LLM round trip. Batch APIs (sensitivity grids, Monte Carlo with explicit seeds) will run in
Rust in a single call to avoid per-call overhead.
