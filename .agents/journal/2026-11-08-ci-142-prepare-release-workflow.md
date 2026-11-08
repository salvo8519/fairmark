# 2026-11-08 — ci/142-prepare-release-workflow

- Issue: #142 Release preparation runs as a workflow that opens the release pull request
- Pull request: opened at the end of this session
- Spec: none (tooling)

## Done
- a5355c9 `.github/workflows/prepare-release.yml`: manual workflow on main (version, optional
  issue) running release.py ready, version bump, towncrier build and git-cliff, then pushing
  release/vX.Y.Z and opening the chore(release) pull request. Optional GitHub App token
  limited to contents and pull requests write. `docs.yml`: workflow_dispatch on main and a
  concurrency group for deploys. `releases.md`: steps and setup. Task #142 recorded in the
  roadmap as done.

## State at the end of the session
- `uv run just check` passes locally; zizmor 1.32.0 and actionlint pass.
- The release steps were simulated locally in a throwaway worktree with 0.1.0-alpha.1:
  bump, towncrier, git-cliff, version check and release_notes.py all succeed. The workflow
  itself has not run on GitHub yet.

## Decisions taken
- No automatic tag on merge: releases.md keeps tagging with the maintainer.
- Input validation in the workflow with a whole-string Bash match (security review: Python
  `re.match` with `$` in release.py accepts a trailing newline).

## Open questions for the maintainer
- Allow GitHub Actions to create pull requests in the repository settings (required).
- Create the GitHub App for the release token, or keep the close-and-reopen step.
- Follow-up: use `fullmatch` in scripts/release.py and scripts/version.py (security review,
  low severity).

## Next steps
1. After merge and the settings change, run Prepare release with a pre-release version when
   the maintainer decides to publish one, and check the pull request it opens.
