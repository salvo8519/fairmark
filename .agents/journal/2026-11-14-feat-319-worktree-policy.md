# 2026-11-14 — feat-319-worktree-policy

- Issue: #319 Generated artifacts and worktree policy for long-running sprints
- Pull request: none (opens from this branch after the maintainer approves the commit)
- Spec: none (tooling and documentation)

## Done
- Extended `scripts/workspace_report.py` with the `Retirement candidates` section: every
  worktree other than the main checkout whose branch is merged into `main` (squash merges
  included) or inactive for more than `STALE_DAYS` (14, the sprint cycle) is listed with its
  reasons; read-only as before, deletion stays with the maintainer. Seven new tests in
  `python/tests/test_workspace_report.py`, written first (all 57 pass).
- Worktree lifecycle rules in `sprints/README.md`: one worktree per session (the hard rule:
  two sessions in one checkout overwrite each other's branch), creation from an up-to-date
  `main`, retirement proposal at session start from the report, removal only of the paths the
  maintainer approves.
- Generated-artifacts policy in `sprints/README.md`: at most one lane per sprint runs
  `uv run just generate`; pull requests carrying regenerated artifacts merge strictly one at
  a time, the next rebased on `main` with the step re-run in it; the alternative (a generate
  step on `main` after each method merge) is recorded as needing its own approval.
- Hard rule in `AGENTS.md` (one session, one worktree, never a shared checkout) and the
  commands table describes the report's new section.
- Skills updated: `session-start` (retirement candidates at session start), `run-sprint`
  (generate lane, merge order at end of run), agent `sprint-planner` (lanes disjoint in what
  they generate).
- Documented the section in `docs/en/project/workspace-report.md`.
- Marked task 319 done in `roadmap/roadmap.yaml` in the same change.

## State at the end of the session
- All work done in the isolated worktree `../fairmark-319` (the policy of this issue, applied
  to itself). Full gate green in it: `uv run just check` — All checks passed (clippy, Rust
  tests with golden, 3985 Python tests, both documentation builds).
- Dogfooding caught one defect after the first implementation: the candidate line of
  `Retirement candidates` proposed the worktree of an unmerged pull request (HEAD equal to
  `main`) without its 12 uncommitted paths. The line now carries the uncommitted and
  untracked counts of a candidate, and one more test covers it (58 total in the file).

## Decisions taken
- The merge-order policy was chosen over the generate-step-on-`main` alternative: it needs
  no write access to `main` from a workflow and no branch-protection change (decided by the
  orchestrator; the alternative is documented as a possible follow-up issue).
- The inactivity threshold is 14 days, the sprint cycle length (decided by the orchestrator).
- The retirement logic lives in the existing read-only report rather than a new script: it
  already knows merged state, last activity and the git safety rules (decided by the
  orchestrator).

## Open questions for the maintainer
- Whether the generate-step-on-`main` alternative should be planned as its own issue later.

## Next steps
1. Run the remaining Python-side gates in the worktree, regenerate the `.claude/` mirror,
   commit, push and open the pull request (Closes #319, label no-release-note), wait for CI.
