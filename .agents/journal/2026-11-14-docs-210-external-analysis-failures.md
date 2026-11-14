# 2026-11-14 — docs-210-external-analysis-failures

- Issue: #210 Rule for external analysis failures before merge
- Pull request: none (opens from this branch after the maintainer approves the commit)
- Spec: none (documentation)

## Done
- Added the rule for failures of external analyses to the Pull requests section of
  `docs/en/project/conventions.md`: re-run first, merge waits until the failure is
  understood, and when the failure is demonstrably external (download outage, 5xx,
  "configuration not found" comparison made while the analysis was still running) and
  every required check is green, the triage review comment records what failed, the
  re-run and why the failure is external, and the merge proceeds; an analysis that is not
  a required check never blocks a merge on its own (CodeQL is not required today).
- Marked task 210 done in `roadmap/roadmap.yaml` in the same change.

## State at the end of the session
- Targeted checks green: `scripts/check_style.py`, `scripts/check_yaml.py`,
  `scripts/docs_sync.py impact --worktree`, `scripts/roadmap.py check --pr-body` with the
  pull request body.
- Full gate green: `uv run just check` (lint, 3977 Python tests, both documentation builds).
- `docs/en/project/conventions.md` is in the `english_only` list of `i18n/it.yaml`, so no
  Italian translation is required.

## Decisions taken
- The rule lives in the Pull requests section of `conventions.md` (the issue allowed that
  page or the releases page; the section already states how pull requests are titled,
  sized, labelled and merged) (decided by the orchestrator).
- The "mergeability comment" of the issue is the triage review comment of the `pr-triage`
  skill, so the rule names that comment (decided by the orchestrator).

## Open questions for the maintainer
- Implementation and wording of the one-session-one-worktree rule (#319) wait for the
  maintainer's approval.

## Collision with a second session (2026-11-14, about 21:00)
- A second agent session worked in the same checkout. At 20:56 it checked out its branch
  `chore/318-roadmap-update` in this directory; at 20:59 this session committed the #210
  work without re-checking the branch, so the commit (d5fa31c) landed on the foreign branch
  and mixed both works.
- Recovery, fully reversible and verified: tag `collision-210-recovery` at d5fa31c;
  `git reset --mixed 618cddb` of the foreign branch, which returned its roadmap edits to
  uncommitted without losing anything; rebuild of this work in a dedicated worktree
  (`../fairmark-w210`) from that tag, keeping only the #210 files; the shared checkout was
  left entirely to the other session.

## Decisions on the parallel protocol
- One agent session means one `git worktree`; no two sessions may share a checkout. The
  permanent rule, and the tooling that enforces it, are to be implemented as part of #319.

## Next steps
1. The pull request for #210 opens from the worktree `../fairmark-w210`; after the merge the
   worktree and the tag `collision-210-recovery` are removed.
