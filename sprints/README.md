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
  - name: docs
    issues: [40]
```

Lanes must touch disjoint areas of the code (for example `core` and `docs`), so that parallel
agents never edit the same files.

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
