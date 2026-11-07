# CLAUDE.md

@AGENTS.md
@specs/constitution.md

## Claude Code specifics

### Subagents (`.claude/agents/`)
Delegate instead of doing everything in the main context:

| Subagent | Use it for |
| --- | --- |
| `spec-writer` | Turning a request into `spec.md`, then `plan.md` + `tasks.md` from the templates |
| `test-engineer` | Writing golden cases and property tests from acceptance criteria, before code |
| `code-reviewer` | Reviewing the current diff against AGENTS.md and the constitution |
| `numerical-auditor` | Independently recomputing results and checking conventions/determinism |
| `docs-writer` | Methodology pages, regenerated reference, CHANGELOG |

After implementing a task, always run `code-reviewer`; when numbers or golden cases changed,
also run `numerical-auditor`. Address their findings before opening a PR.

### Skills (`.claude/skills/`)
- `/spec-new` — start a new spec from the templates.
- `/implement-spec` — execute `tasks.md` test-first, one task per commit.
- `/add-method` — the end-to-end checklist for a new valuation method.
- `/golden-case` — author a golden case with an independent reference computation.
- `/pre-pr` — full local gate + both review subagents + PR description draft.
- `/release` — cut a release.

### Hooks (`.claude/settings.json`)
- Edited `.rs` / `.py` files are formatted automatically after each edit.
- Edits to **existing** files in `tests/golden/` are blocked: ask the maintainer.

### MCP
`.mcp.json` registers this repository's own `fairmark` MCP server (dogfooding): use its tools
to sanity-check numbers while developing instead of computing them yourself.

### Working style
- Plan before multi-file changes; keep diffs small and focused on one task.
- Prefer reading `specs/` and `docs/concepts/` over guessing intent.
- When a methodology choice is ambiguous, ask the maintainer instead of picking one.
