# ADR 0007 — Structured logging in adapters, never financial data

Status: accepted · 2026-11-07

## Context
Servers (MCP over HTTP today, hosted services later) need operational logs. Valuation inputs
and outputs are confidential and must never leave the user's control (constitution, V).

## Decision
- The core does not log: it performs no I/O. Its audit trail is `explain`, returned to the
  caller.
- Adapters log to stderr, as text or JSON lines (`--log-format json`), through
  `fairmark._logging`.
- Allowed fields: event, tool or method id, outcome (`ok` or an error code), duration,
  `input_hash`, versions, transport. Forbidden: inputs, outputs, values, arguments, prompts,
  file names. `log_event` rejects forbidden field names; tests enforce it.
- Distributed tracing and metrics (OpenTelemetry) are deferred to the hosted service
  milestone and will follow the same data rules.

## Consequences
+ Operable servers with machine-readable logs and no confidentiality risk.
+ `input_hash` lets a user correlate a log line with a result they hold, without exposure.
− Debugging a wrong input requires the user to share the input: by design.
