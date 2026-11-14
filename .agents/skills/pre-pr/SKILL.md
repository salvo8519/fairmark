---
name: pre-pr
description: Run the full local quality gate and both review subagents, then draft the pull request description. Use before pushing a branch or opening a PR.
---

1. `uv run just check` — must be green. Fix and re-run until it is.
2. Classify the pull request by tier ("Effort proportional to risk" in AGENTS.md):
   - Small: read the diff yourself; no review subagent.
   - Substantial: run the `code-reviewer` subagent once on the branch diff. Fix every finding
     and check each fix against the list; a second pass only for a blocking logic defect.
   - Numerical change: as substantial, plus `numerical-auditor`. Fix discrepancies.
3. The numerical audit of `main` is scheduled (weekly and at milestone end); do not run it for
   pull requests outside the numerical-change tier.
4. Check `git diff main...HEAD -- tests/golden`: if an existing expected value changed,
   the PR needs the `numerical-change` label and maintainer approval — say so explicitly.
5. Draft the PR:
   - Title: Conventional Commit, citing the spec (`feat(dcf): … [SPEC-0001]`).
   - Body: summary, requirements/AC covered, tests added, docs updated, reviewer verdicts,
     the PR template checklist ticked honestly.
