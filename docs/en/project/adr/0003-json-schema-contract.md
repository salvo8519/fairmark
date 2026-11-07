# ADR 0003 — JSON Schema as the single contract

Status: accepted · 2026-11-07

## Decision
Method inputs are Rust types deriving `JsonSchema` (JSON Schema 2020-12) with
`deny_unknown_fields` and documented fields. The registry exposes catalogue, schemas and a
`run(method, json)` entry point. Python helpers, CLI, MCP tools and LLM tool definitions are all
derived from it.

## Consequences
+ Tool definitions never drift from the implementation.
+ Field docs written once appear in IDEs, CLI, MCP clients and LLM prompts.
− Schema changes are API changes and follow semantic versioning.
