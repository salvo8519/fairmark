# 2026-11-16 — fix-339-workspace-report-test

- Issue: #339 Workspace report test runs without background git maintenance
- Pull request: opens from this branch after the gate
- Spec: none (test only)

## Done
- Root cause of the intermittent failure of `test_report_changes_nothing_on_disk` (seen on
  macOS in #336): after commit, merge and fetch, git runs `git maintenance run --auto`, which
  git 2.47 and later detaches in the background. The fixture ends with a `git fetch`, so the
  maintenance process could take its lock in `.git/objects` while the test compared the disk
  before and after the report. The report itself was not at fault.
- The fixture of `python/tests/test_workspace_report.py` sets `maintenance.auto=false` and
  `gc.auto=0` through `GIT_CONFIG_COUNT`; the full snapshot comparison is kept, git
  directories included. A new test asserts that the fixture disables both.
- 50 consecutive local runs pass. Note: the local git (2.43) does not detach maintenance, so
  the local runs show no regression rather than reproduce the race; the CI matrix is the check.

## State at the end of the session
- Nothing half done.

## Decisions taken
- The source of the write is removed instead of excluding `.git` from the comparison, so the
  read-only test keeps its full strength (issue criteria updated accordingly).

## Open questions for the maintainer
- none

## Next steps
1. #334: lanes declare their paths; `scripts/sprint.py validate` rejects overlaps.
