# Sprints

A sprint is a time-boxed set of GitHub issues with one goal. Sprint files in this folder are
the source of truth for agents; the GitHub Project "Fairmark roadmap" shows the same sprint
as an Iteration on the Kanban board.

## Files
`sprints/SNN.yaml`, validated by `scripts/sprint.py`:

```yaml
id: S01
goal: Numerical core ready for the complete DCF
start: 2026-11-09
end: 2026-11-22
milestone: "M1 · Numerical core"
status: planned            # planned | active | done
lanes:                     # lanes run in parallel; issues within a lane run in order
  - name: core
    issues: [12, 13]
    paths: ["crates/", "python/fairmark/", "python/tests/", "tests/golden/"]
    generates: true          # the one lane that runs `uv run just generate`
  - name: docs
    issues: [40]
    paths: ["docs/en/guides/", "docs/it/guides/"]
```

Lanes must touch disjoint areas of the code (for example `core` and `docs`), so that parallel
agents never edit the same files. They must also stay disjoint in what they generate: at most
one lane may run `uv run just generate` in a sprint (see the next two sections). Both rules
are checked, not only written down (next section).

## Lane paths

Every lane declares `paths`, the files it may change, as glob patterns relative to the
repository root: `*` matches within one path segment, `**` any number of segments, `?` one
character, and a pattern that ends with `/` stands for the directory and everything below it.
`generates: true` marks the one lane that runs `uv run just generate` (default `false`).

`uv run python scripts/sprint.py validate sprints/SNN.yaml`, run by `uv run just check` and by
CI on every push, rejects a sprint when:

- a lane has no `paths`;
- two lanes have patterns that some file could match both, naming the lanes and an example
  path;
- more than one lane has `generates: true`;
- a lane without `generates` covers a generated artifact: `schemas/`,
  `docs/*/reference/methods.md`, `docs/*/methods/index.md`, `README.md`, `README.it.md`,
  `python/fairmark/tool_examples.json` or `.claude/`. The two README files and the methods
  indexes are listed whole because their generated block cannot be told apart from the rest
  of the file in a diff.

On a pull request from a sprint branch (`sprint/SNN/<issue>-<slug>`), CI runs
`scripts/sprint.py check-diff` and fails when a changed file is outside the paths of the
issue's lane. Every lane may also change `roadmap/roadmap.yaml`, `.agents/journal/` and
`changes/` (the roadmap line, the journal entry and the release note of the issue); the
generating lane may also change the generated artifacts. A file that a lane really needs
outside its paths is a planning change: update the sprint file in its own pull request, so
that the overlap check runs on the new paths before the work continues.

## One worktree per session

A git checkout gives one working tree one branch, one HEAD and one index: two agent sessions
that work in the same directory overwrite each other's branch, and one session's
`git add` commits the other's uncommitted files. Parallel work of any kind — two sessions on
two issues, or a sprint lane while the orchestrator uses the main checkout — runs in one git
worktree per session, created from an up-to-date `main`:

```bash
git fetch origin
git worktree add ../fairmark-<n> -b <type>/<n>-<slug> origin/main
```

The session lives in that directory for the whole issue; its checkout, its uncommitted files
and its build caches are its own. A pull request is opened from it and pushed from it.

### Retirement of stale worktrees

Long sprints accumulate worktrees whose branch is already merged or that nobody touched for
weeks. At the start of a session, run `git fetch origin` and then:

```bash
uv run just workspace-report
```

Its **Retirement candidates** section lists every worktree whose branch is merged into
`main` (squash merges included) or whose last activity is older than 14 days, one line each
with the reasons. The list is a proposal: propose its entries to the maintainer and remove
only the paths the maintainer explicitly approves, with `git worktree remove <path>`
(without `--force`; git refuses a worktree with changes). Nothing is ever deleted without
that approval, which is also the rule of the report script itself
(docs/en/project/workspace-report.md). After a merge, the session that created a worktree
removes it in the same way, once its pull request is pushed.

## Generated artifacts and the merge order

`uv run just generate` writes files that belong to no single lane: the schema snapshots in
`schemas/` with `schemas/index.json`, `docs/en/reference/methods.md`, the tool examples, the
Python docstrings and the `.claude/` mirror of `.agents/`. Two lanes that both add or change
methods regenerate them independently and collide when their pull requests merge.

The policy of this repository is the merge order:

1. A sprint plans at most one lane whose pull requests run `uv run just generate`: method
   changes, error-documentation changes and `.agents/` changes that rewrite the mirror all
   belong to that one lane.
2. Pull requests that carry regenerated artifacts merge strictly one at a time.
3. After one of them merges, the next one in the order is rebased on `main`, and
   `uv run just generate` is run again in it, so its artifacts supersede those of the pull
   request merged before it. CI (generated files current, docs coverage, tool examples)
   verifies every push.
4. Pull requests with no regenerated artifacts merge in any order.

The alternative policy — a generate step run on `main` after each method pull request is
merged — would remove the ordering from the lanes, but it writes to `main` from a workflow
and needs its own issue, its own approval and a change to the branch protections before it
can be planned.

## Sprint state: resume mid-issue

The orchestrator of a sprint run writes its running table on disk as it goes, one file per
issue, one write per step of the local mode. The files are runtime state, never committed,
and live in the git directory that every worktree of the repository shares:
`<git common dir>/fairmark/sprints/SNN/state/<issue>.yaml` (`.git/fairmark/...` in the main
checkout). The orchestrator in the main checkout, every lane worktree and
`scripts/project_state.py` therefore read and write the same files; `scripts/sprint.py` is the
only code that builds the path. A file is replaced atomically at each write, so a reader never
sees it half written. State files of an earlier layout, next to the sprint file in a checkout
(`sprints/SNN/state/`, still git-ignored), are moved to the shared directory the first time
they are read; when both exist, the more recent `updated` wins.

```bash
uv run python scripts/sprint.py state sprints/S01.yaml 42 \
    --branch feat/42-mid-year --step tests \
    [--pr 33] [--blocker "waiting for the reviewer"] [--clear-blockers]
```

The step is the current step of the lane loop: `spec`, `tests`, `implementation`, `review`,
`pr` or `done`. The file holds the issue, its lane, branch, pull request, current step,
blockers, `started` and `updated` (UTC, ISO 8601) and `agent_steps`, counted one per write:
the agent cost of the issue.

`uv run python scripts/sprint.py status sprints/S01.yaml` prints the state directory and, for
each issue, the state next to the
GitHub state of its issue, and marks the state file of a closed issue as stale: remove it,
and the worktree of its branch, once the issue closes. At every task switch the orchestrator
also appends one line to the work-in-progress note
`.agents/journal/<date>-sprint-SNN-wip.md`: issue, step, pull request, blockers, last commit
of the lane. A context reset or a crash then costs minutes: a new session reads the same
files in the briefing of `uv run python scripts/project_state.py`, reopens the worktree of
the branch the state names (it is still there, see the sections above) and continues from
the recorded step. The state directory and the work-in-progress note are local recovery
state; the committed record stays the journal entry and the pull request of the issue.

## Workflow
1. **Plan** — `/plan-sprint`: the `sprint-planner` agent proposes `sprints/SNN.yaml` from the
   open issues of the current milestone. The maintainer edits and approves it.
2. **Board** — in the GitHub Project, set the Iteration field of the sprint's issues (one
   bulk edit). The Iteration field is created once in the Project settings (two-week cycle).
3. **Run** — `/run-sprint SNN`: the main Claude Code session acts as orchestrator and
   delegates each issue to the specialised agents, lane by lane, or dispatches issues to
   Claude in GitHub Actions. Every issue ends as a pull request; nothing is merged without
   the maintainer.
4. **Track** — `uv run python scripts/sprint.py status sprints/SNN.yaml` shows progress from
   GitHub.
5. **Close** — set `status: done`; unfinished issues move to the next sprint.
