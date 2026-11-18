---
title: Agentic development
description: How Fairmark is built by AI agents and one maintainer — roles, memory, sprint lanes and the deterministic gates that enforce every rule.
---

# Agentic development

Fairmark is developed by AI coding agents working with one maintainer. The engine is held to a
strict standard (exact decimals, golden cases from independent sources, versioned contracts),
and so is the process that builds it. One principle runs through this page: **a rule that
matters is enforced by a check that can fail, not by the goodwill of an agent.** Where a rule
is still only written down, this page says so.

```mermaid
flowchart LR
    R[Roadmap<br/>roadmap.yaml] --> S[Sprint<br/>lanes with paths]
    S --> W[One worktree<br/>per issue]
    W --> P[Spec, approved<br/>for method work]
    P --> T[Tests first]
    T --> I[Implementation]
    I --> V[Review<br/>by risk tier]
    V --> Q[Pull request]
    Q --> G[CI gates]
    G --> M[Maintainer<br/>merges]
    M --> R
    classDef plan fill:#0B1F3A,color:#F7F8FA,stroke:#0B1F3A
    classDef gate fill:#12325C,color:#F7F8FA,stroke:#12325C
    classDef work fill:#F7F8FA,color:#0B1F3A,stroke:#475569
    class R,S,M plan
    class G,V gate
    class W,P,T,I,Q work
```

## One source, generated mirrors

The roles and skills of the agents are written once, in
[`.agents/`](https://github.com/salvo8519/fairmark/tree/main/.agents), in open formats (Agent
Skills, Markdown role definitions). The shared contract of every agent is
[`AGENTS.md`](https://github.com/salvo8519/fairmark/blob/main/AGENTS.md). Where an agent system
reads another layout, `scripts/sync_agents.py` generates it from the same source:

| Agent system | Instructions | Skills | Roles (subagents) |
| --- | --- | --- | --- |
| Claude Code | `CLAUDE.md`, which imports `AGENTS.md` | `.claude/skills/` (generated) | `.claude/agents/` (generated) |
| Codex | `AGENTS.md` | `.agents/skills/` (native) | `.codex/agents/*.toml` (generated) |
| Cursor | `AGENTS.md` | `.agents/skills/` (native) | `.cursor/agents/` (generated) |
| GitHub Copilot | `AGENTS.md` | `.agents/skills/` (native) | `.github/agents/*.agent.md` (generated) |
| Gemini CLI | `AGENTS.md` when configured to read it | `.agents/skills/` (native) | not generated |

A role without `Write` or `Edit` among its tools stays read-only in every system: the read-only
sandbox in Codex, `readonly: true` in Cursor, no `edit` tool in Copilot. A role the generator
cannot translate (an unknown tool, a missing field) fails the generation instead of being
guessed. Gemini CLI roles are not generated: its subagents are experimental, must be enabled
in its settings, and their tool names are not yet in a stable reference. The formats follow
each system's documentation of November 2026; the project has not yet run the generated roles
in Codex, Cursor or Copilot.

| Rule | Enforced by |
| --- | --- |
| Mirrors are never edited by hand | `scripts/sync_agents.py --check` in CI (generated files current) |
| A new agent system is a generator change, not a copy | the same check: a file in a mirror without a source fails |
| Mirrors belong to the process lane | the `agents` group of generated files in `scripts/sprint.py` |

## Roles and skills

Twelve roles, each with a narrow responsibility and the tools it needs:

| Role | Responsibility | Writes |
| --- | --- | --- |
| `architect` | Structural decisions, ADRs | Decision records |
| `spec-writer` | Specifications, plans, task lists | Specs |
| `test-engineer` | Golden cases and tests, before the code | Tests |
| `core-engineer` | The Rust core, from an approved spec | Code |
| `docs-writer` | Methodology pages, guides, references | Documentation |
| `translator` | The Italian edition, with the glossary | Translations |
| `code-reviewer` | The diff against the constitution and the spec | Nothing (read-only) |
| `numerical-auditor` | Independent recomputation of results | Nothing (read-only) |
| `security-reviewer` | Dependencies, workflows, data handling | Nothing (read-only) |
| `ci-investigator` | Root cause of a red CI run | Nothing (read-only) |
| `release-manager` | Release readiness and verification | Release files |
| `sprint-planner` | The next sprint and its lanes | Sprint files |

Skills package the workflows the roles follow: `session-start` and `session-end`, `spec-new`,
`implement-spec`, `add-method`, `golden-case`, `pre-pr`, `pr-triage`, `plan-sprint`,
`run-sprint`, `roadmap-update`, `release` and others. The main session is the orchestrator: it
delegates to the roles and keeps the overview, because a role cannot start another role.

## Memory in three tiers

Agents forget everything between sessions. What survives them is stored in plain text, in the
place that fits how long it must live ([ADR 0015](adr/0015-agent-memory-one-source-tiered-mirrors.md)):

| Tier | Lives for | Store | Shared how |
| --- | --- | --- | --- |
| Runtime | Seconds to hours | Per-issue sprint state, one YAML file per issue | In the git directory shared by every worktree; never committed |
| Handover | Days | The session journal, `.agents/journal/` | Committed with the pull request of the session |
| Plan | Permanent | [`roadmap/roadmap.yaml`](https://github.com/salvo8519/fairmark/blob/main/roadmap/roadmap.yaml): milestones, epics, tasks | Committed; synchronised with GitHub issues |

No database: every tier is readable by any agent of any system, and auditable in a diff. A crash
or a context reset costs minutes: `scripts/project_state.py` prints where the project stands,
the journal says where the last session stopped, and the branch on GitHub carries the work.
`scripts/sprint.py burndown` turns the sprint ledger and GitHub into a burn-down, cycle times,
failed check runs per lane and agent steps per issue.
`scripts/journal.py search <issue|area|date|text>` finds the entries about an issue, its own
first; `session-start` cites them for every issue it proposes. The index is built from the
entries each time it is read and never committed, so parallel lanes never collide on it.

## Sprints and lanes

A sprint groups issues into lanes that run in parallel, each agent in its own git worktree
([sprints/README.md](https://github.com/salvo8519/fairmark/blob/main/sprints/README.md)). Lanes
must not touch the same files, and this is checked, not assumed:

- each lane declares `paths`, the glob patterns of the files it may change;
- each group of generated artifacts (`registry`: schemas, methods reference, README blocks,
  tool examples; `agents`: the `.claude/` mirror) belongs to at most one lane, named in its
  `generates`;
- `scripts/sprint.py validate` rejects overlapping lanes, with the lanes and an example path;
- on a sprint pull request, CI runs `scripts/sprint.py check-diff` and fails when a changed
  file is outside the lane of the issue.

## Gates

Every pull request passes the same deterministic checks, in CI and, where it is cheap, before a
commit. The required ones block the merge.

| Rule | Enforced by |
| --- | --- |
| Branch, commit and pull request names; a linked issue | `scripts/conventions.py` (PR hygiene) |
| An issue closed by a pull request is in the roadmap; every spec is planned | `scripts/roadmap.py check` |
| Code changes come with their documentation | `scripts/docs_sync.py impact`, also a Stop hook of the agent |
| Every method has a page, a nav entry, golden cases and a Python helper | `scripts/docs_sync.py coverage` |
| Public pages describe only what exists | `scripts/docs_sync.py claims` |
| Italian pages follow their English source | `scripts/i18n.py check` |
| Earlier schema versions never change; changes are versioned | `scripts/schema_snapshots.py --check` and the contracts job |
| Existing golden cases change only through a reviewed numerical change | an edit hook in every worktree and `scripts/check_golden_changes.py` |
| The tests of the core detect wrong results | mutation testing of the changed lines |
| The MCP server works for a real client, with portable tool schemas | `scripts/mcp_protocol_check.py` with the MCP Inspector |
| Every action pinned to a commit; no icons in any file | `scripts/pin_actions.py --check`, `scripts/check_style.py` |
| Every process script has a test | `python/tests/scripts/test_every_script_is_tested.py` |
| A quarantined test has an issue, a reason and an expiry date | `scripts/test_failures.py quarantine-check` |

## Effort proportional to risk

Reviews by agents are expensive, so their number follows the risk of the change. Every pull
request states its tier ([AGENTS.md](https://github.com/salvo8519/fairmark/blob/main/AGENTS.md),
Review standards):

| Tier | Changes | Review before merge |
| --- | --- | --- |
| Small | Documentation, roadmap, tooling, CI, fixes that change no number or contract | The gates; no review agent |
| Substantial | Method logic, contracts, adapters | One `code-reviewer` pass |
| Numerical change | Results or golden cases change | `code-reviewer` and `numerical-auditor` |

Every finding of a review is fixed in the same pull request. A defect is never deferred.

## Credentials

Agents work on branches. The credentials of the development process are kept out of reach of a
branch: each lives in an environment whose rules a branch cannot change. A test guards
against a job that reads a credential outside its environment; it runs on the branch it checks, so it catches mistakes, not attacks. The
environments' settings are checked weekly by the maintenance workflow, and a Claude Code hook
stops agents from approving runs or changing secrets, variables, environments and rulesets.

- Fairmark itself uses no credential and sends no data anywhere (constitution, principle V).
- The automation that must start checks (the release pull request, the update of pull
  requests behind `main`) uses the project's GitHub App. Its key is a secret of the
  environment `fairmark-app`, which only `main` can use; a branch that changes a workflow
  cannot read it. Each job creates a token valid one hour, with only the permissions it needs,
  revoked at the end of the job. A test guards against a job that can run on content written
  by others (pull requests, issues, comments) naming that environment.
- Publishing uses trusted publishing (OIDC) from a protected tag. The only stored token is the
  crates.io token of the very first release, a secret of the `crates-io` environment; the
  maintainer deletes it after that release, and the job refuses it once the crates exist.
- The key of the AI workflows (`ANTHROPIC_API_KEY`) is a secret of the environment
  `anthropic`: every run of the AI review, the documentation sync or an `@claude` mention
  waits for the maintainer's approval
  ([Maintainer guide](maintainers.md#ai-workflows-one-approval-per-run)).
- `.env` files and `.pem` keys are ignored by git; secret scanning with push protection
  rejects a credential pushed by mistake.

Setup and rotation: [Releases](releases.md#the-projects-github-app).

## What the maintainer decides

Agents propose; the maintainer decides. The maintainer approves specs and methodology choices,
approves every change to an existing golden case, decides the roadmap, approves the sprint,
merges, and cuts releases. These decisions are recorded where agents read them: in the spec,
in the issue, in the journal or in an ADR.

## Changing the process

The process described on this page is complete and frozen since sprint S01 (maintainer
decision of 2026-11-18): the work goes to the product. The sprint loop scripts, the lanes, the
gates, the roles, the skills and the workflows change only for a measured problem, which the
request names: a defect, a failure that happened, or a figure of
`scripts/sprint.py burndown` (cycle time, failed checks, agent steps). A change that only adds
a rule waits. The process stays reusable as a template for other repositories.

## What is not enforced yet

- The cost of an issue is measured in orchestrator steps (`scripts/sprint.py burndown`), not in
  tokens: the scripts cannot see token usage.
- Intermittent tests and stale pull requests are reported every weekday, not blocked
  (`.github/workflows/test-health.yml`). Pull requests behind `main` are updated by the
  project's GitHub App once the maintainer has set it up; until then, by hand with
  `scripts/pr_queue.py update --apply`.
- Whether secret scanning with push protection is on, and whether the App key lives only in
  the environment `fairmark-app`, are repository settings that CI does not verify. The rules
  of the environments are checked weekly; the list of repository secrets and the permissions
  of the token agents use are checked by hand.
- A workflow added by a branch can request an OIDC token without any environment. Whether the
  Claude GitHub App would exchange it for a token of its own has not been verified.
- Mirrors for agent systems other than Claude Code are planned in #328.
