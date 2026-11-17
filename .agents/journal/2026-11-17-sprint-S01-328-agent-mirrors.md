# 2026-11-17 — sprint/S01/328-agent-mirrors

- Issue: #328 Generated mirrors of .agents/ for other agent systems
- Pull request: opens from this branch after the gate (sprint S01, process lane)
- Spec: none (tooling)

## Done
- `scripts/sync_agents.py` generates the roles of `.agents/agents/` for Codex
  (`.codex/agents/*.toml`), Cursor (`.cursor/agents/`) and GitHub Copilot
  (`.github/agents/*.agent.md`), beside the Claude Code mirror. Read-only roles stay read-only:
  Codex read-only sandbox, Cursor `readonly: true`, Copilot without `edit`.
- An unknown tool, a missing field or a name that differs from the file name fails the
  generation. A file added by hand to any mirror fails `--check`.
- The mirror folders join the `agents` group of generated files (`scripts/sprint.py`) and the
  process lane (planning PR #379).
- Docs: the table of agent systems on the agentic development page (both languages),
  `.agents/README.md`, ADR 0015 amended.
- Tests: role formats per system (TOML parsed, front matter parsed), refusals, hand-added
  files, every repository role translates, lane group consistency.

## State at the end of the session
- Nothing half done.

## Decisions taken
- No skill mirrors outside Claude Code: Codex, Cursor, Gemini CLI and Copilot read
  `.agents/skills/` natively. No `.cursor/rules/`: Cursor reads `AGENTS.md`.
- No Gemini CLI roles yet: subagents are experimental and their tool names are not in a
  stable reference.
- Formats from each system's documentation of November 2026, research in this session; the
  generated roles have not been run in Codex, Cursor or Copilot.

## Open questions for the maintainer
- none

## Next steps
1. #335: the ADR for per-method schema manifests (stops at the ADR).
