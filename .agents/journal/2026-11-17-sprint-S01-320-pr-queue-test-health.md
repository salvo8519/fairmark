# 2026-11-17 — sprint/S01/320-pr-queue-test-health

- Issue: #320 Sprint burn-down, loop metrics and pull request queue automation (second part)
- Pull request: opens from this branch after the gate (sprint S01, process lane)
- Spec: none (tooling)

## Done
- `scripts/pr_queue.py`: `list` puts every open pull request that is not a draft in one state
  (`conflict`, `failing`, `behind`, `ready`) and marks it stale after three idle days;
  `--fail-on-stale` exits with 1 when a stale one is in conflict or failing. `update` merges
  `main` into the pull requests that are behind: dry run by default, `--apply` to act, never
  on a fork or a conflict, and only if the head is the commit that was read
  (`expected_head_sha`). `uv run just pr-queue`.
- `scripts/test_failures.py`: `collect` reads JUnit files and counts runs and failures per
  pytest node id; `report` separates intermittent tests from tests that always fail and exits
  with 1 for either, unless the intermittent test is quarantined; `quarantine-check` validates
  `python/tests/quarantine.yaml` (issue, reason, expiry date, existing file).
- `python/tests/conftest.py`: a quarantined test runs as a non-strict expected failure, so its
  outcome stays in the report and in the JUnit file.
- CI: every Python job writes and uploads `junit-<os>-<python>`; the checks job runs
  `quarantine-check`. New `test-health.yml` (weekdays): aggregates 14 days of runs on `main`,
  reports the queue, and opens or updates one issue labelled `test-health`.
- Docs: sprints/README section "Test health and the pull request queue"; the agentic
  development page (both languages) lists the new gate and states what is still manual.
- Planning PR #366 merged first: it added the new paths to the process lane.
- Tests: 9 for the queue, 10 for the failures and the quarantine. Full suite 4256 passed;
  zizmor and actionlint clean.

## State at the end of the session
- Nothing half done.

## Decisions taken
- The scheduled workflow reports and never pushes: a push with a workflow's own token starts
  no checks, so an updated branch would look green without being tested. Updates run with a
  personal or session token.
- Only runs on `main` count for intermittent tests: failures on a pull request may be work in
  progress, not flakiness.

## Open questions for the maintainer
- A dedicated token secret would let the scheduled workflow update stale branches itself.
  Not configured; the docs say so.

## Next steps
1. #328: generated mirrors of `.agents/` for other agent systems.
