# Sprints

A sprint is a time-boxed set of GitHub issues with one goal. Sprints are one part of the
development system described in
[Agentic development](../docs/en/project/agentic-development.md). Sprint files in this folder are
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
    generates: [registry]    # the generator groups this lane regenerates
  - name: docs
    issues: [40]
    paths: ["docs/en/guides/", "docs/it/guides/"]
```

Lanes must touch disjoint areas of the code (for example `core` and `docs`), so that parallel
agents never edit the same files. They must also stay disjoint in what they generate: each group of
generated artifacts belongs to at most one lane of a sprint (see the next two sections). Both rules
are checked, not only written down (next section).

## Lane paths

Every lane declares `paths`, the files it may change, as glob patterns relative to the
repository root: `*` matches within one path segment, `**` any number of segments, `?` one
character, and a pattern that ends with `/` stands for the directory and everything below it.
`generates` lists the generator groups the lane regenerates (default none). Generated
artifacts come from two independent generators, so they form two groups, and each group has at
most one lane per sprint:

| Group | Generator | Artifacts |
| --- | --- | --- |
| `registry` | `schema_snapshots.py`, `gen_reference.py`, `tool_examples.py` | `schemas/`, `docs/*/reference/methods.md`, `docs/*/methods/index.md`, `README.md`, `README.it.md`, `python/fairmark/tool_examples.json` |
| `agents` | `sync_agents.py` | `.claude/` |

A product lane that changes methods and a process lane that changes skills can therefore run
in parallel: they regenerate different files.

`uv run python scripts/sprint.py validate sprints/SNN.yaml`, run by `uv run just check` and by
CI on every push, rejects a sprint when:

- a lane has no `paths`;
- two lanes have patterns that some file could match both, naming the lanes and an example
  path;
- `generates` is not a list of known groups, or two lanes generate the same group;
- a lane covers a generated artifact of a group it does not generate. The two README files and
  the methods indexes are listed whole because their generated block cannot be told apart from
  the rest of the file in a diff.

On a pull request from a sprint branch (`sprint/SNN/<issue>-<slug>`), CI runs
`scripts/sprint.py check-diff` and fails when a changed file is outside the paths of the
issue's lane. Every lane may also change `roadmap/roadmap.yaml`, `.agents/journal/` and
`changes/` (the roadmap line, the journal entry and the release note of the issue); the
lane may also change the artifacts of the groups it generates. A file that a lane really needs
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

`uv run just generate` writes files that belong to no single lane, in two groups (Lane paths):
the `registry` group (schema snapshots with `schemas/index.json`, the methods reference and
indexes, the README blocks, the tool examples) and the `agents` group (the `.claude/` mirror of
`.agents/`). Two lanes that both add or change methods regenerate the same files independently
and collide when their pull requests merge; a lane that does not change the sources of a group
regenerates its files unchanged.

The policy of this repository is the merge order:

1. A sprint plans at most one lane per generator group: method and error-documentation changes
   belong to the lane that generates `registry`, `.agents/` changes to the lane that
   generates `agents`.
2. Pull requests that carry regenerated artifacts of the same group merge strictly one at a
   time.
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
blockers, `blocked_since` (when the issue became blocked, null when it is not), `started` and
`updated` (UTC, ISO 8601) and `agent_steps`, counted one per write: the agent cost of the
issue, measured in orchestrator steps, not in tokens.

Every write also appends one event to the sprint ledger, `ledger.jsonl` beside the state
directory (shared by every worktree, never committed): one JSON line per checkpoint with the
time, issue, lane, step, pull request, number of blockers and agent steps. Lines are appended
in a single write each, so parallel lanes never interleave them.

## Burn-down and loop metrics

`uv run python scripts/sprint.py burndown sprints/SNN.yaml` (`--json` for agents) reads the
ledger and GitHub and prints:

| Figure | Meaning |
| --- | --- |
| Burn-down | Per day of the sprint, every issue in exactly one state: `done` (closed that day or before), `in review` (one of its pull requests open at the end of the day), `in progress` (a ledger checkpoint that day or before), `open` |
| Cycle time | Hours from the first ledger checkpoint of an issue to its closing; the median over the sprint. An issue closed without a checkpoint is listed as not measured, never estimated |
| Failed check runs per lane | Check runs that concluded `failure` on the commits of the issue's pull requests, each commit counted once |
| Agent steps | Orchestrator steps per issue, from the ledger |
| Blocked now | Open issues whose last checkpoint recorded blockers, with the time |

A pull request belongs to an issue when it is opened from a branch of the issue
(`<type>/<issue>-<slug>` or `sprint/SNN/<issue>-<slug>`) or recorded in its state; a pull
request that only mentions the issue (a roadmap update) does not. An issue closed as not
planned never counts as done.

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

## Test health and the pull request queue

Two scheduled reports keep the loop from stalling (`.github/workflows/test-health.yml`,
weekdays). Both warn and never block a merge; a problem opens or updates one issue labelled
`test-health`.

**Intermittent tests.** Every Python job of CI writes a JUnit file and uploads it as an
artifact (`junit-<os>-<python>`, kept 30 days). `scripts/test_failures.py collect` reads the
files of the CI runs on `main` of the last 14 days and counts, per test, the runs and the
failures; `report` lists the tests that failed at least once:

| Kind | Meaning | What to do |
| --- | --- | --- |
| Intermittent | Failed in some runs, passed in others | Fix it, or quarantine it with an issue |
| Always failing | Failed in every run where it ran | Fix it: a defect is never quarantined |

A quarantined test is listed in `python/tests/quarantine.yaml` with its pytest node id, the
issue that tracks the fix, the reason and an expiry date. It still runs; its failure is
reported as `xfailed` and does not fail the suite, and a pass is reported as `xpassed`.
`scripts/test_failures.py quarantine-check` runs in CI and fails on an entry without an issue,
without a reason, past its date or naming a file that does not exist, so a quarantine never
becomes permanent by neglect.

**The pull request queue.** `uv run just pr-queue` (or `scripts/pr_queue.py list`) puts every
open pull request that is not a draft in one state, in this order: `conflict` (a rebase by
hand), `failing` (a check run failed on its head), `behind` (`main` has commits it lacks and
it merges cleanly), `ready`. A pull request unchanged for three days is stale; the scheduled
report fails when a stale pull request is in conflict or failing.

`scripts/pr_queue.py update` merges `main` into the pull requests that are `behind`: a dry run
by default, the change only with `--apply`, never on a fork or a pull request in conflict, and
only if the head is still the commit that was read. It runs with your token or the session's,
not in the scheduled workflow: GitHub starts no workflow for a push made with a workflow's own
token, so the updated branch would never be checked again.

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
   GitHub; `scripts/sprint.py burndown sprints/SNN.yaml` the burn-down and loop metrics.
5. **Close** — set `status: done`; unfinished issues move to the next sprint.
