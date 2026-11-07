# Architecture

## Principles

- **One engine, thin adapters.** All financial logic lives in `fairmark-core` (Rust). Adapters
  translate, they never compute.
- **The registry is the single source of truth.** `registry.rs` lists every method with its
  version and its input type. JSON Schemas are generated from the Rust types (`schemars`), and
  every adapter is generated from them: Python helpers, CLI, MCP tools, function-calling
  definitions, documentation.
- **JSON at the boundaries.** Data crosses the Rust/Python boundary as JSON strings with
  decimals as strings. This keeps the FFI surface to three functions and makes every adapter
  testable with the same fixtures.
- **No I/O in the core.** The core is a pure function from input to result. Telemetry, files
  and network live only in adapters.

## Layout

```
crates/
  fairmark-core/    methods, decimal math, explain, registry, errors (no I/O)
  fairmark-cli/     `fairmark` binary: methods | schema | run | verify
  fairmark-py/      PyO3 bridge (3 functions), built by maturin as fairmark._engine
python/fairmark/    Pythonic API, function-calling tools, MCP server
skills/fairmark/    Agent Skill for Claude Code, Codex and compatible agents
tests/golden/       shared golden cases (Rust + Python)
docs/               architecture, determinism, errors, ADRs, methodology
roadmap/            roadmap.yaml → GitHub milestones, epics and tasks
```

## Adding a method

1. Implement it in `crates/fairmark-core/src/<area>.rs` with an `Input` struct (serde +
   `JsonSchema`, `deny_unknown_fields`, doc comment on every field) and a function returning
   `Result<(Output, Explain)>`.
2. Register it in `registry.rs` (`METHODS`, `input_schema`, `run`).
3. Add golden cases in `tests/golden/<method>/` from an independent source.
4. Add a thin Python helper in `python/fairmark/<area>.py`.
5. Write the methodology page in `docs/methods/`.

Function-calling tools, MCP tools and CLI support come for free.

## Performance

The engine is native code with no allocation-heavy abstractions; a full DCF with explain runs in
microseconds. Decimal arithmetic is slower than floating point, by design: in valuation,
exactness matters more than nanoseconds, and the engine remains orders of magnitude faster than
any LLM round trip. Batch APIs (sensitivity grids, Monte Carlo with explicit seeds) will run in
Rust in a single call to avoid per-call overhead.
