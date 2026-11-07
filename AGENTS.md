# AGENTS.md — instructions for AI coding agents working on this repository

## Golden rules
- Financial logic lives only in `crates/fairmark-core`. Adapters (Python, CLI, MCP) never compute.
- Use `rust_decimal::Decimal` for every value. Never introduce `f32`/`f64` in a calculation path.
- Every new method: input type with `JsonSchema` + `deny_unknown_fields` + doc comment on every
  field, registry entry, golden cases from an independent source, methodology page.
- Never change an expected value in `tests/golden` to make a test pass. A changed result is a
  `numerical-change` and needs human review.
- No network or file I/O in `fairmark-core`.

## Commands
```bash
cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace
maturin develop && pytest && ruff check python && mypy
```

## Using Fairmark (not developing it)
See `skills/fairmark/SKILL.md`.
