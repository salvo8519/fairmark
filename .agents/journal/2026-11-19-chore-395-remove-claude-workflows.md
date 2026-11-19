# 2026-11-19 — chore/395-remove-claude-workflows

- Issue: #395 Remove the Claude workflows that never ran in GitHub Actions
- Pull request: cross-lane removal (workflows, CLAUDE.md, sprint docs), outside the sprint lanes
- Spec: none

## Done
- Found that `ANTHROPIC_API_KEY` was never configured: `claude-review.yml` skipped its steps on
  every run, `claude.yml` and `claude-docs.yml` never ran. #376 had protected a key that did
  not exist; GitHub created the environment `anthropic` on the first run.
- Removed the three workflows, the weekly check of `anthropic`, the AI parts of the secrets
  test, the AI section of the maintainer guide (replaced by "Agents and settings"), the remote
  mode of `run-sprint`, and the `@claude` dispatch in `sprints/README.md` and `CLAUDE.md`.
- Deleted the unused labels accessibility, documentation and enhancement on GitHub.

## State at the end of the session
- Discussions still off: the proxy refuses repository settings writes; the maintainer turns
  them on.

## Decisions taken
- Maintainer, 2026-11-19: remove the Claude workflows. The settings hook and the App
  protection stay.

## Lesson
- Verify that a credential exists before building protection around it.

## Open questions for the maintainer
- Delete the environment `anthropic` (Settings, Environments) after the merge.

## Next steps
1. Product work.
