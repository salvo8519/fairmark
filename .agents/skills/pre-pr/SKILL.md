---
name: pre-pr
description: Run the full local quality gate and both review subagents, then draft the pull request description. Use before pushing a branch or opening a PR.
---

1. `uv run just check` — must be green. Fix and re-run until it is.
2. Run the `code-reviewer` subagent on the branch diff. Fix blocking findings.
3. If any file under `crates/fairmark-core/src/`, `tests/golden/` or `docs/en/methods/` changed,
   run `numerical-auditor`. Fix discrepancies.
4. Check `git diff main...HEAD -- tests/golden`: if an existing expected value changed,
   the PR needs the `numerical-change` label and maintainer approval — say so explicitly.
5. Draft the PR:
   - Title: Conventional Commit, citing the spec (`feat(dcf): … [SPEC-0001]`).
   - Body: summary, requirements/AC covered, tests added, docs updated, reviewer verdicts,
     the PR template checklist ticked honestly.
