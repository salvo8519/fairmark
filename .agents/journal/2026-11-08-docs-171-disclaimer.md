# 2026-11-08 — docs/171-disclaimer

- Issue: #171 Disclaimer: a calculation tool, decisions belong to the user
- Pull request: opened at the end of this session, waiting for the maintainer's approval of the text
- Spec: none (documentation and agent instructions)

## Done
- f88a27c DISCLAIMER.md and DISCLAIMER.it.md (tracked by scripts/i18n.py, now generalised to root
  documents), README Disclaimer section in both languages, site Disclaimer page in both
  editions, notices on the home page and methods index, footer link (Italian footer from
  i18n/it.yaml), MCP server instructions, MCP guide, agent skill, release note fragment.

## State at the end of the session
- `uv run just check` passes; `docs_sync.py impact --worktree` passes.
- Code review (code-reviewer agent): all findings fixed, including two inexact statements in
  the first draft (results do not carry the inputs, only their hash; the calculations are not
  only written by people) and two Italian translations narrower than the English.

## Decisions taken
- The maintainer's principle: the tool computes, the user decides; every decision and use of
  the results belongs exclusively to the user. The structure borrows the AI Act vocabulary
  (intended purpose, instructions, human oversight) without claiming that the AI Act applies.
- The README marketing text is unchanged at the maintainer's request; only the small print
  became a Disclaimer section.

## Open questions for the maintainer
- Approval of the wording before merge; a legal review is recommended before the 0.1.0 release.

## Next steps
1. After approval, merge; consider an ADR for a notice inside every result envelope.
