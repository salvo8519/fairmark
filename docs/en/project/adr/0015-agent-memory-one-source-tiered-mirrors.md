# ADR 0015 — Agent memory: one canonical source, tiered memory, generated mirrors

Status: proposed · 2026-11-14 (#329; architecture of the agent loop session of 2026-11-14)

## Context

Fairmark is developed by AI agents as much as by humans, and the agents of a session need
state that survives them: the running table of a sprint, the handover to the next session,
the plan. The tools built around this in 2026-11-14 (#318 sprint state per issue, #319 worktree
lifecycle and generated-artifacts merge order) raised four architectural questions:

- **Where does a single session work?** A checkout has one HEAD and one index: two agent
  sessions in the same directory commit each other's files, demonstrated by an incident on
  2026-11-14, recovered without loss. The hard rule of #319 is "never two sessions in one
  checkout", which does not force a worktree on a session that is alone: it works in the main
  checkout, and the worktree is the tool of parallel work.
- **Database, ledger or files?** Claude Code keeps its sessions as append-only ledgers;
  Fairmark already keeps the plan as code (`roadmap/roadmap.yaml`), the handover as text
  (`.agents/journal/`) and, since #318, the per-issue state as YAML. A database (SQLite)
  would be binary, not diffable, invisible to an agent without a tool and in tension with
  the constitution's "auditable, all text".
- **Runtime state committed or git-ignored?** Committed state written at every step turns
  the repository into a database (a commit per step, conflicts between lanes); git-ignored
  state is still readable by every agent on the machine, and remote resumption happens
  through the pushed branch ("never leave work only on a laptop"), not through the state.
- **How is context indexed?** The journal is indexed by date only; an agent that needs the
  context of an issue or an area greps by hand at session start.

At the same time `.agents/` is the canonical source of the roles and skills, and `.claude/`
is one generated mirror, while `AGENTS.md` — read natively by Codex, Cursor, Zed and others —
is already the shared contract of every harness.

## Decision

1. **One canonical source, generated mirrors, never edited by hand.** `.agents/` and
   `AGENTS.md` are canonical; every agent system reads a mirror generated from them by
   `scripts/sync_agents.py` (`.claude/` today, the layouts of the other harnesses in #328),
   and `uv run just generate` keeps every mirror current or fails. Adding a harness is a
   generator change, never a copy by hand.
2. **Memory is tiered, each tier with the store that fits it.**
   - *Runtime* (seconds to hours): the per-issue state files of #318, stored since #333 in
     the git directory shared by every worktree
     (`<git common dir>/fairmark/sprints/SNN/state/`), the work-in-progress note, and a new
     append-only event ledger of the sprint loop (one JSONL line per checkpoint: issue, step,
     blockers, agent steps) beside the state files, from which the burn-down and the loop
     metrics of #320 derive. Local and never committed; readable by every agent and every
     worktree on the machine. A low-resolution metrics rollup of the ledger
     may be committed at the end of a sprint when #320 needs it.
   - *Handover* (days): `.agents/journal/`, committed with the pull request of the session,
     per the session protocol; a work-in-progress note may stay local while the work is
     in flight.
   - *Plan* (permanent): `roadmap/roadmap.yaml` with its issues, committed and checked by
     CI. Remote resumption of unfinished work goes through the pushed branch, never through
     local state alone.
3. **Isolation follows the rule of #319, not a blanket worktree policy.** A session that is
   alone works in the main checkout; the worktree is created when work runs in parallel.
   The single session that shares a checkout with nobody keeps the simplest development
   experience (the IDE opened on one directory, warm caches).
4. **Context is indexed by generated index, not by database.** An index of the journal
   (date, branch, issues, areas, one-line summary), searched by a small reader
   (`scripts/journal.py`, #327) that `session-start` cites. Text indexed, never binary;
   SQLite full-text search is the documented fallback if the journal grows past what text
   search serves, not a dependency added now. Amended by #327: the index is built from the
   entries each time it is read and never committed. Every session adds an entry, so a
   committed index would be rewritten by every pull request and make parallel lanes collide,
   the problem #335 removes for `schemas/index.json`; building it takes milliseconds.

## Consequences

- The repository stays fully textual and auditable: any agent of any harness reads the
  state, the journal, the plan and the index without a database driver.
- Harness neutrality is a generator problem: Fairmark works today with Claude Code and
  gains the others as mirrors without changing the canonical source (#328).
- The ledger makes #320 measurable rather than estimated: burn-down and loop metrics read
  events that already happen, no new bookkeeping at the edges.
- A crash or a context reset costs minutes: `project_state.py` lists the per-issue state,
  the search cites the journal, and the branch on `origin` carries the work itself.
- The maintainer accepts this ADR before implementation of #327, #328 and the ledger part
  of #320; until then its status stays `proposed`.
