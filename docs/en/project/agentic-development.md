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
Skills, Markdown role definitions). Every agent system reads a mirror generated from that
source: `.claude/` today, the layouts of other systems as they are added (#328). The shared
contract of every agent is
[`AGENTS.md`](https://github.com/salvo8519/fairmark/blob/main/AGENTS.md), which Codex, Cursor,
Copilot and others read natively.

| Rule | Enforced by |
| --- | --- |
| Mirrors are never edited by hand | `scripts/sync_agents.py --check` in CI (generated files current) |
| A new agent system is a generator change, not a copy | the same check: a file in a mirror without a source fails |

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

## What the maintainer decides

Agents propose; the maintainer decides. The maintainer approves specs and methodology choices,
approves every change to an existing golden case, decides the roadmap, approves the sprint,
merges, and cuts releases. These decisions are recorded where agents read them: in the spec,
in the issue, in the journal or in an ADR.

## What is not enforced yet

- Sprint burn-down and loop metrics (cycle time, failed gates, agent steps per issue) are
  planned in #320; until then the cost of the process is not measured.
- Mirrors for agent systems other than Claude Code are planned in #328.
