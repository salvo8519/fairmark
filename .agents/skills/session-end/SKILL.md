---
name: session-end
description: Close a working session on Fairmark cleanly — run the quality gate, write the journal entry, add the release note fragment, commit, push and open or update the pull request with a complete, professional description. Use when the maintainer says the session is over, or before switching to another issue.
---

Talk to the maintainer in their preferred language; everything written to the repository or
to GitHub is in English, without icons. Never merge: merging is the maintainer's decision.

1. **Gate.** Run `uv run just check`. If it fails, fix what belongs to this session's task;
   otherwise record the failure in the journal and open the pull request as a draft.
2. **Scope.** `git status` and `git diff origin/main...HEAD --stat`. Every changed file must
   belong to the branch's issue. Unrelated changes go to a separate branch, never into this
   pull request.
3. **Roadmap.** Issues opened in this session were created with
   `uv run python scripts/roadmap.py new` (milestone, epic, labels, spec), never by hand.
   Mark the task of the issue this pull request closes as `status: done` in
   `roadmap/roadmap.yaml` and run `uv run python scripts/roadmap.py render`: merging closes the
   issue, and the file must already say so. CI checks it (`roadmap.py check --pr-body`).
4. **Release note.** If users can see the change and no fragment exists, write one in
   `changes/` (`<issue>.<type>.md`, plain language for analysts, see `changes/README.md`).
   Otherwise the pull request gets the label `no-release-note`.
5. **Journal.** Write `.agents/journal/<YYYY-MM-DD>-<branch-slug>.md` from `_template.md`:
   what is done (with hashes), what is half done and where, decisions, open questions and the
   very next step. Facts only, no client data.
6. **Commit.** Use `/commit` (English message, translation for the maintainer). Signed off.
7. **Push.** `git push -u origin <branch>`. Never force-push a branch that someone else
   reviewed without telling the maintainer.
8. **Pull request.** If none exists for the branch, create it; otherwise update its body.
   Title: the Conventional Commit summary of the whole change. Body, in this order, filling
   every section of `.github/PULL_REQUEST_TEMPLATE.md`:
   - **Summary**: two to four sentences, what and why, for a reader who has not seen the issue.
   - **Linked issue**: `Closes #<n>` (or `Refs #<n>` when the issue stays open).
   - **Specification**: spec id and requirement ids covered; acceptance criteria met.
   - **Changes**: grouped by area (core, adapters, docs, CI), one line each.
   - **Numerical impact**: "None" or the methods, old and new values, `method_version` bump,
     `Numerical-change:` trailer, and the independent source.
   - **Contracts**: schema snapshots changed (breaking or not), error codes added.
   - **Documentation**: pages added or updated, translation status (`uv run just i18n`).
   - **Testing**: golden cases, unit and property tests added; commands run and their result.
   - **Risks and follow-ups**: known limitations, issues opened for later.
   - **Checklist**: ticked honestly; leave unticked what is not done and say why.
   Use `gh api repos/{owner}/{repo}/pulls` (POST to create, PATCH to update) with the body in
   a file. Open as draft if the gate failed or tasks remain.
9. **Labels and board.** Add labels that the labeler cannot infer (`numerical-change`,
   `breaking-change`, `no-release-note`). Remove `in-progress` from the issue only when the
   pull request is ready for review.
10. **Disk.** Check disk usage with `uv run just workspace-report` (read-only) and include
    the figures in the report to the maintainer. Nothing is deleted without the maintainer's
    explicit approval.
11. **Report** to the maintainer: pull request link, check status, what remains, next step.
