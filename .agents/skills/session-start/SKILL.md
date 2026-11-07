---
name: session-start
description: Open a working session on Fairmark with full context — current branch and issue, latest journal entry, open pull requests and CI, sprint progress, work in progress and issues that can be started in parallel — then propose a plan. Use at the beginning of every session, or when the maintainer asks where the project stands.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`). Do not change any
file in this skill: it only reads and proposes.

1. Collect the state, deterministically:
   `uv run python scripts/project_state.py` (add `--json` if you need to process it).
   If the GitHub CLI is not authenticated, say so and continue with git only.
2. Read the context that the script points to:
   - the latest entry in `.agents/journal/` for the current branch, otherwise the latest
     overall; its "Next steps" are the default continuation;
   - the issue linked to the branch (`gh api repos/{owner}/{repo}/issues/<n>`) and its spec,
     if any (`specs/NNNN-*/`), with `plan.md` and `tasks.md`;
   - `CHANGELOG.md` section "Unreleased" and the fragments in `changes/`, for recent changes
     in the areas you will touch;
   - ADRs referenced by the spec or the issue.
3. Check hygiene and report each problem with the command that fixes it:
   - uncommitted changes, or a branch behind `main` (propose `git rebase origin/main`);
   - work committed directly on `main`;
   - an open pull request of this branch with failing checks (propose `/fix-ci`);
   - issues labelled `in-progress` with no branch or pull request (stale claims).
4. Propose the session plan, short:
   - **Continue**: the next step from the journal or `tasks.md`, if a branch is in progress.
   - **Review queue**: pull requests waiting for the maintainer (propose `/pr-triage`).
   - **Start next**: up to three ready issues from the active sprint (or the current
     milestone), ordered by dependency. Mark which ones can run **in parallel**: different
     `area:` labels and disjoint files, no dependency between them. For each, say whether it
     needs an `Approved` spec first.
5. Wait for the maintainer's choice. When a new issue is chosen:
   - create the branch `<type>/<issue>-<slug>` from an up-to-date `main`
     (for example `feat/42-dcf-mid-year`);
   - label the issue `in-progress` and assign it:
     `gh api -X POST repos/{owner}/{repo}/issues/<n>/labels -f "labels[]=in-progress"`;
   - for parallel work, suggest one git worktree per issue
     (`git worktree add ../fairmark-<n> -b <branch> origin/main`) or `/run-sprint`.
