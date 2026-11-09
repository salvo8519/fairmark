---
name: workspace-clean
description: Free disk space safely — remove merged agent worktrees and, when needed, rebuildable build caches, with a dry run first and without ever losing work or context. Use when the disk is above 80 percent, after pull requests are merged, or at the end of a session.
---

Talk to the maintainer in their preferred language. The tool is
`scripts/clean_workspace.py` (recipe `just clean-workspace`); the rules are in
`docs/en/project/workspace-cleaning.md`.

## When

- The disk is above 80 percent (`df -h .`).
- After one or more pull requests are merged: their worktrees are no longer needed.
- At the end of a session (`session-end` runs the dry run).

## Order

1. **Update.** `git fetch -q origin`, so that `origin/main` knows about recent merges. A stale
   `origin/main` only makes the tool keep more.
2. **Dry run.** `uv run just clean-workspace` (add `--caches` when the disk is full). Nothing
   is removed.
3. **Read the kept list.** Every kept worktree has a reason. Check that no worktree you expect
   to keep is marked `remove`, and tell the maintainer about worktrees kept for
   `uncommitted or untracked changes`, `unpushed commits` or `branch not merged` that look
   abandoned: they hold work, so only a person decides about them. Never work around a reason
   (no `git worktree remove --force`, no `rm -rf`, no `--idle-hours 0` on a machine where
   other agents are running).
4. **Apply.** `uv run just clean-workspace --apply` (and `--caches` if the dry run showed caches
   worth removing). Use `--caches` on worktrees of other running agents only when the disk is
   critical: it costs them a full rebuild, never work.
5. **Verify.** `df -h .` and report the space freed and the kept list to the maintainer.

## Guarantees

- Dry run unless `--apply`; caches only with `--caches`.
- A worktree is removed only when it is merged into `origin/main` (or its pull request was
  merged on GitHub with exactly its head commit), clean, pushed, not locked, not the main
  checkout, not used by a running process and idle for 12 hours. If GitHub or `/proc` cannot be
  checked, it is kept. Branches are never deleted.
- Caches are removed only when no cargo, rustc, cargo-mutants or pytest process uses their tree.
- Never removed: sources, `.git`, `.agents/journal/`, `specs/`, `docs/`, `uv.lock`, tracked
  files, symbolic links, and anything outside the worktrees and the temporary patterns.
