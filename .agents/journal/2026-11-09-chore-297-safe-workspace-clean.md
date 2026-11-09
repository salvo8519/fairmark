# 2026-11-09 — chore/297-safe-workspace-clean

- Issue: #297 Safe workspace cleaning: smaller builds, worktree and build-cache cleanup
- Pull request: opened at the end of this session (ready)
- Spec: none (tooling, docs)

## Done
- 4ea6617 `[profile.dev]` and `[profile.test]` with `debug = "line-tables-only"`: clean
  `cargo test --workspace --no-run` 2.06 GB -> 1.36 GB, 1 min 52 s -> 1 min 28 s; a probe
  panic still reported file, line and column in the message and in every backtrace frame.
- 9aa10d5 `scripts/clean_workspace.py`, `just clean-workspace`, 39 tests on temporary repos.
- 4f6c32a Skill `workspace-clean`, step 10 of `session-end`, CLAUDE.md, AGENTS.md.
- Documentation page `docs/en/project/workspace-cleaning.md` and a pointer in CONTRIBUTING.

## State at the end of the session
- Nothing half done. `uv run just check` passed.

## Decisions taken
- Added an idle condition (default 12 hours) to the worktree rules: agent shells keep no
  working directory between commands, so the `/proc` check alone cannot see an agent waiting
  for CI. Proposed by the agent, within the maintainer's request for no loss of work.
- Branches are never deleted, only worktrees; `git worktree remove` is used without `--force`.
- The cargo-mutants and pytest temporary directories are shared: any running cargo-mutants or
  pytest process keeps all of them.

## Open questions for the maintainer
- The tool was only run as a dry run against the real environment. Whether to apply it, and
  with `--caches`, is the maintainer's decision.

## Rework after the maintainer's decision (2026-11-09)
- The maintainer decided that automatic deletion is too risky. The pull request now keeps only
  the `line-tables-only` debug profiles and a read-only report.
- `scripts/clean_workspace.py` became `scripts/workspace_report.py` (recipe
  `just workspace-report`) with no deletion capability; a test greps the source for deletion
  calls. Merged detection hardened: `git worktree list --porcelain -z`, common-dir check before
  any git query in a worktree, URL-encoded branch, pull request counted only with `merged_at`,
  base `main`, head in this repository and matching head commit; malformed responses and
  anything not established are `unknown`.
- Dropped: skill `workspace-clean` and its entries in CLAUDE.md, AGENTS.md and
  `.agents/README.md`. `session-end` step 10 now runs the report and reports to the maintainer.
- Documentation page renamed to `docs/en/project/workspace-report.md`.

## Next steps
1. After review and merge: `git fetch -q origin && uv run just workspace-report`; deletions are
   decided by the maintainer.
