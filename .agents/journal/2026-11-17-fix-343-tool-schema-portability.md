# 2026-11-17 — fix-343-tool-schema-portability

- Issue: #343 Tool schemas with one type per keyword, portable to every client
- Pull request: opens from this branch after the review (one code-reviewer pass, all findings fixed)
- Spec: none (adapter; no method, schema snapshot or number changes)

## Done
- `_single_types` in `python/fairmark/tools.py` rewrites every list-valued `type` of the tool
  definitions as `anyOf` with one branch per type; applied in `definitions` after the decimal
  rewrite (full) or `_portable`, so the MCP server and every provider format get it.
- The MCP Inspector in strict mode goes from 316 warnings on 33 tools to none;
  `scripts/mcp_protocol_check.py` now fails on a single warning.
- Tests: no type list in any variant or provider format; unit cases (annotations, enum with
  null, nesting); for every method, the tool schema and the registry schema agree on every
  example and on every copy with one field set to null; the protocol check fails on one
  warning and on errors.
- Function calling guide in English and Italian (stamped); release note fragment.

## Review (code-reviewer, one pass, every finding fixed)
- The null branch now keeps the keywords that constrain a null (`enum`, `const`,
  combinators): before, `{"type": ["string","null"], "enum": ["a"]}` would have accepted null
  after the rewrite. No current schema had that combination; tests now cover it with a
  validator.
- The equivalence test could not see a lost rule (a version dropping `pattern` and `minimum`
  passed it). It now adds values that break a rule (decimal grammar, minimum, type, unknown
  field), runs on the compact profile too, and has no sample cap; two injected faults fail
  61 and 4 tests.
- The protocol check also fails on a warning line when the Inspector summary is missing.
- Release note corrected: the Gemini format was already portable before this change.
- The task is marked done in the roadmap by this pull request, as #119 requires.

## State at the end of the session
- The maintainer works on the product lane in parallel: this change touches
  `python/fairmark/tools.py`, agreed with the maintainer on 17 November.

## Decisions taken
- The registry schemas keep the spelling schemars generates (279 nullable type lists); only
  the tool definitions are rewritten, so no method version changes. Changing the generation
  in the core remains a possible contract decision for v0.1.

## Open questions for the maintainer
- none

## Next steps
1. #327: index and search of the journal.
