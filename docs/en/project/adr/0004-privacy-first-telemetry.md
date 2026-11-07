# ADR 0004 — Privacy-first, opt-in telemetry in adapters only

Status: proposed · 2026-11-07

## Decision
The core never performs network I/O (enforced by `cargo deny`). Any usage statistics are opt-in,
live in adapters, follow an explicit allow-list that excludes all financial data, and are
designed in public before implementation. See `docs/en/project/telemetry.md`.

## Consequences
+ Analysts and companies can adopt Fairmark without data-protection concerns.
− Usage insight comes mostly from public signals (PyPI, crates.io, GitHub) until opt-in data grows.
