# .agents — the project's agent configuration

This folder is the single, tool-agnostic source of truth for AI agents working on Fairmark.
How the pieces fit together (roles, memory tiers, sprint lanes, gates) is described in
[Agentic development](../docs/en/project/agentic-development.md).

| Path | Content | Read by |
| --- | --- | --- |
| `skills/<name>/SKILL.md` | Workflows packaged as Agent Skills (open standard) | Codex, Cursor, Gemini CLI, Copilot and other agents natively; Claude Code via `.claude/skills` |
| `agents/<name>.md` | Role definitions used as subagents: front matter `name`, `description`, `tools` (Claude Code tool names), then the instructions | Claude Code via `.claude/agents`, Codex via `.codex/agents`, Cursor via `.cursor/agents`, Copilot via `.github/agents` |

`scripts/sync_agents.py` (`uv run just generate`) writes every mirror; CI fails when one is
stale or edited by hand. The table of agent systems is on the
[Agentic development](../docs/en/project/agentic-development.md) page.

Project-wide instructions live in `AGENTS.md` (all agents) and `CLAUDE.md` (Claude Code
specifics, imports `AGENTS.md`).

## Roles

| Agent | Responsibility | Writes code |
| --- | --- | --- |
| `architect` | Structural decisions, ADRs | No |
| `spec-writer` | Specs, plans, task lists | No |
| `test-engineer` | Golden cases, property and error tests, before code | Tests only |
| `core-engineer` | Rust core implementation from `tasks.md` | Yes |
| `docs-writer` | Methodology pages, guides, regenerated references | Docs only |
| `code-reviewer` | Diff review against AGENTS.md, constitution, spec | No |
| `numerical-auditor` | Independent recomputation, determinism | No |
| `security-reviewer` | Supply chain, workflows, data handling | No |
| `ci-investigator` | Root cause of red CI runs | No |
| `release-manager` | Release readiness, versioning, verification | Release files |
| `sprint-planner` | Next sprint: issues, lanes, `sprints/SNN.yaml` | Sprint files |
| `translator` | Maintained translations of docs and README, with the glossary | Translations |

## Skills

`spec-new`, `implement-spec`, `add-method`, `golden-case`, `adr-new`, `pre-pr`, `fix-ci`,
`release`, `plan-sprint`, `run-sprint`, `commit`, `explain-changes`, `translate`, `onboarding`,
`session-start`, `session-end`, `pr-triage`, `roadmap-update`.

## Journal

`journal/` holds session handover notes (see `journal/README.md`), written by `session-end`
and read by `session-start`.

Orchestration: the main session runs `/run-sprint` and delegates to the roles above, because
subagents cannot start other subagents.

## Editing

Edit files here, then run `uv run just generate` to refresh `.claude/`. CI fails if the two
diverge (`scripts/sync_agents.py --check`).
