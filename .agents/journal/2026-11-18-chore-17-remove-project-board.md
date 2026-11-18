# 2026-11-18 — chore/17-remove-project-board

- Issue: #17 Labels and Discussions (the project board dropped)
- Pull request: removal of the project board; a cross-lane change outside the sprint lanes,
  like a planning pull request (it touches `scripts/` outside the process lane)
- Spec: none

## Done
- Maintainer decision of 2026-11-18: this project uses only milestones, no project board.
- Removed `.github/workflows/project-sync.yml` (never active: `PROJECT_URL` was never set),
  `scripts/setup_github_project.py` and its test; with them `PROJECT_TOKEN`, the last
  repository secret.
- Docs: sprints/README (the board step of the workflow), roadmap and conventions pages,
  maintainer guide, the Credentials section (both languages); skills `plan-sprint` and
  `session-end`; the secrets test now admits no repository secret.

## State at the end of the session
- #17 waits for two maintainer settings: Discussions on, three unused labels deleted.

## Decisions taken
- Milestones only.

## Open questions for the maintainer
- Delete the repository secret `PROJECT_TOKEN` and the variable `PROJECT_URL` if they exist.

## Next steps
1. Product work.
