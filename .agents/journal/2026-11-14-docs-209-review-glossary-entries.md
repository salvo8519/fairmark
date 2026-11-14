# 2026-11-14 — docs-209-review-glossary-entries

- Issue: #209 Glossary entries for numerical audit and code review
- Pull request: none (opens from this branch after the maintainer approves the commit)
- Spec: none (documentation)

## Done
- Added the glossary entries "code review: revisione del codice" and
  "numerical audit: verifica numerica" to `i18n/glossary.yaml`, with a comment naming the
  agent roles that perform them: code-reviewer and numerical-auditor
  (`.agents/agents/code-reviewer.md`, `.agents/agents/numerical-auditor.md`).
- Marked task 209 done in `roadmap/roadmap.yaml` in the same change.

## State at the end of the session
- Full gate green locally: `uv run just check` passed (lint, 3977 Python tests, both
  documentation builds, i18n 63/63 pages current).
- Targeted checks green: `scripts/check_yaml.py` (733 files parse), `scripts/check_style.py`,
  `scripts/docs_sync.py impact --worktree`, `scripts/roadmap.py check --pr-body` with the
  pull request body.

## Decisions taken
- The two entries sit at the end of the terms list, next to the method-status block, as
  terms of repository process; the comment names the two agent role files (decided by the
  orchestrator).

## Open questions for the maintainer
- none

## Next steps
1. Commit, push the branch and open the pull request (Closes #209, labels area:docs and
   no-release-note), then wait for CI before merging.
