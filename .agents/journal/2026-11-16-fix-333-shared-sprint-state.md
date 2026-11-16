# 2026-11-16 — fix-333-shared-sprint-state

- Issue: #333 Sprint runtime state shared by every worktree of the repository
- Pull request: opens from this branch after the gate
- Spec: none (tooling)

## Done
- `scripts/sprint.py` resolves the runtime state directory from the git directory shared by
  every worktree: `<git common dir>/fairmark/sprints/SNN/state/<issue>.yaml`. One function,
  `state_dir`, builds the path; `state_files` lists the files in issue order. Outside git the
  directory next to the sprint file is still used.
- State files of the earlier layout (`sprints/SNN/state/` in a checkout) are moved to the
  shared directory the first time they are read; when both exist, the more recent `updated`
  wins. Each write is atomic (temporary file, then rename).
- `scripts/project_state.py` reads the state through `scripts/sprint.py`. It also failed to
  start on `main`: a starred expression on the sprint section (`out += *(...)`, introduced
  in #326) was a syntax error, and no test imported the script. Fixed, and
  `python/tests/test_project_state.py` now renders the briefing.
- Six tests in `python/tests/test_sprint.py` reproduce the gap first (state written from a
  second worktree was not visible from the main checkout) and cover the shared location,
  the migration, the newer-wins rule, the atomic write and the order of the files.
- `sprints/README.md`, the `run-sprint` and `session-start` skills and ADR 0015 state the
  new location. `.claude/` regenerated with `scripts/sync_agents.py`.

## State at the end of the session
- Nothing half done. Ruff, mypy, the full Python suite (4006 tests), style, translations and
  docs impact pass locally.

## Decisions taken
- The maintainer split the work on 16 November: this session works on the agent process
  (scripts, sprints, skills, CI) and not on the product (core, Python package, schemas).

## Open questions for the maintainer
- none

## Next steps
1. #334: lanes declare their paths in `sprints/SNN.yaml`; `scripts/sprint.py validate`
   rejects overlaps, and CI checks the diff of a sprint pull request against them.
