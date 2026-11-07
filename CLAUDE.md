# CLAUDE.md

@AGENTS.md
@specs/constitution.md

## Claude Code specifics

### House style — non-negotiable
Never use icons, emoji, pictograms or decorative symbols: not in documents, README, comments,
commit messages, PR or issue text, nor in your replies about this project. Use words.
`scripts/check_style.py` enforces it in CI.

### Subagents (`.claude/agents/`, generated from `.agents/agents/`)
Delegate instead of doing everything in the main context:

| Subagent | Use it for |
| --- | --- |
| `architect` | Structural decisions and ADRs, before cross-cutting changes |
| `spec-writer` | Turning a request into `spec.md`, then `plan.md` + `tasks.md` |
| `test-engineer` | Golden cases and property tests from acceptance criteria, before code |
| `core-engineer` | Implementing Rust core tasks from `tasks.md` |
| `docs-writer` | Methodology pages, guides, regenerated reference |
| `code-reviewer` | Reviewing the current diff against AGENTS.md and the constitution |
| `numerical-auditor` | Independently recomputing results, checking conventions and determinism |
| `security-reviewer` | Dependencies, workflows, data handling |
| `ci-investigator` | Root cause of a red CI run |
| `release-manager` | Release readiness and verification |
| `sprint-planner` | Proposing the next sprint (`sprints/SNN.yaml`) with parallel lanes |
| `translator` | Updating the Italian edition from the English source, with the glossary |

Standard loop: `spec-writer` → (maintainer approves) → `test-engineer` → `core-engineer` →
`code-reviewer` (+ `numerical-auditor` when numbers changed, + `security-reviewer` when
dependencies or workflows changed) → `docs-writer` → PR.

### Skills (`.claude/skills/`, generated from `.agents/skills/`)
`/spec-new`, `/implement-spec`, `/add-method`, `/golden-case`, `/adr-new`, `/pre-pr`,
`/fix-ci`, `/release`, `/plan-sprint`, `/run-sprint`, `/commit`, `/explain-changes`,
`/translate`, `/onboarding`, `/session-start`, `/session-end`, `/pr-triage`.

### Languages
Write everything that goes into the repository in English. Talk to each person in the
language they prefer: read `CLAUDE.local.md` if it exists. Prepare commits with `/commit`
(English message plus a translation for review). After English documentation changes, run
`/translate` so the Italian edition stays current.

### Sprints and orchestration
Subagents cannot start other subagents: the main session is the orchestrator. `/run-sprint`
delegates each issue of `sprints/SNN.yaml` to the agents above (locally, lane by lane) or to
Claude in GitHub Actions (remotely, via `@claude` on the issue). Every issue ends as a PR;
nothing is merged without the maintainer.

### Documentation
A Stop hook runs `scripts/docs_sync.py impact`: if code changed without its documentation,
you are sent back to fix it (delegate to `docs-writer`).

To change an agent or a skill, edit `.agents/` and run `uv run just generate`; never edit
`.claude/agents` or `.claude/skills` directly.

### Hooks (`.claude/settings.json`)
- Edited `.rs` / `.py` files are formatted automatically after each edit.
- Edits to **existing** files in `tests/golden/` are blocked: ask the maintainer.

### MCP
`.mcp.json` registers this repository's own `fairmark` MCP server (dogfooding): use its tools
to sanity-check numbers while developing instead of computing them yourself.

### Session protocol
Begin with `/session-start`, end with `/session-end`, review the merge queue with `/pr-triage`.
The journal in `.agents/journal/` is how the next session knows where this one stopped.

### Working style
- Plan before multi-file changes; keep diffs small and focused on one task.
- Prefer reading `specs/` and `docs/en/concepts/` over guessing intent.
- When a methodology choice is ambiguous, ask the maintainer instead of picking one.
