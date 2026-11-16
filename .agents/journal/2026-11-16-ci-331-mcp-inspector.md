# 2026-11-16 — ci-331-mcp-inspector

- Issue: #331 MCP server verified end to end with the MCP Inspector in CI
- Pull request: opens from this branch after the gate
- Spec: none (CI and tooling)

## Done
- `scripts/mcp_protocol_check.py` starts `fairmark-mcp` over stdio and drives it with the MCP
  Inspector CLI (pinned `@modelcontextprotocol/inspector@2.15.0`): `tools/list --strict`
  (one tool per registry method, examples in every schema, no error-severity portability
  problem), `--areas` and `--compact` through the protocol, four golden cases
  (`dcf.fcff`, `tvm.npv`, `ratios.liquidity`, `statements.reorganise`) whose protocol result
  equals the in-process call as canonical JSON and meets the golden expectations, and an
  invalid input that returns the FM-E001 report with both JSON pointers as a tool result.
- `uv run just mcp-check`; CI job "MCP · protocol check with the Inspector" (Linux, Node
  22.23.4, `actions/setup-node` pinned by commit), added to `.github/required-checks.txt`.
- `python/tests/test_mcp_protocol_check.py`: 8 tests with a stand-in for the Inspector, each
  breaking one answer (different result, text and structured content apart, protocol error).
- CONTRIBUTING explains the check.

## State at the end of the session
- The ruleset "Protect main" does not yet require the new check: per
  docs/en/project/maintainers.md it is added after this pull request merges.

## Decisions taken
- Schema portability warnings are reported, not fatal: 316 warnings across 33 tools, all of
  the kind `type: ["string", "null"]` (nullable fields), which some clients that map schemas
  onto a single-type dialect (the OpenAPI subset of Gemini function declarations) may reject.
  It is a contract question for the product lane, raised with the maintainer.

## Open questions for the maintainer
- Add the new check to the ruleset after merge.
- Whether the nullable-field spelling of the schemas should change for portability.

## Next steps
1. #327: index and search of the journal (scripts/journal.py).
