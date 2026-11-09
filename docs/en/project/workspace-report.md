---
title: Disk use and the workspace report
description: Where disk space goes in a Fairmark development environment, how debug builds are kept small, and how to read the read-only workspace report. Deletion is manual.
---

# Disk use and the workspace report

A development machine for Fairmark fills up for three reasons: each checkout has its own Rust
`target/` directory (one to several GiB), contributors and agents work in several git
worktrees at once, and mutation testing and pytest leave temporary trees behind.

## Smaller debug builds

The workspace `Cargo.toml` sets `debug = "line-tables-only"` for the `dev` and `test`
profiles. Panics and backtraces still show the file and line of every frame; what is dropped is
the information a debugger needs to inspect variables. The `release` profile is unchanged and
`profile.mutants` keeps `debug = "none"`. Measured on a clean `cargo test --workspace --no-run`
(issue #297):

| Profile setting | `target/` size | Build time (2 cores) |
| --- | --- | --- |
| `debug = true` (Cargo default) | 2.06 GB | 1 min 52 s |
| `debug = "line-tables-only"` | 1.36 GB | 1 min 28 s |

To step through code in a debugger, set `debug = "full"` locally in `[profile.dev]` and do not
commit it.

## The workspace report

`uv run just workspace-report` runs `scripts/workspace_report.py`. It is **read-only**: it has
no option that deletes, moves or rewrites anything, and a test fails the build if deletion code
appears in the script. Git runs with `--no-optional-locks`, in an environment without the
variables that redirect it to another repository.

```bash
uv run just workspace-report            # every worktree, then the disk summary
uv run just workspace-report --offline  # without asking GitHub
```

For each worktree of the repository the report shows:

| Field | Meaning |
| --- | --- |
| Path, Branch, HEAD | From `git worktree list --porcelain -z`; `detached HEAD` when no branch is checked out; `locked` and `prunable` flags |
| Size | Disk space allocated below the worktree (other worktrees nested inside are not counted) |
| Caches | Size of the rebuildable caches inside it: `target/`, `.venv/`, `__pycache__/`, `.pytest_cache/`, `mutants.out*`, each noted "cache rebuildable" |
| Merged into main | `yes`, `no` or `unknown`, with the reason (below) |
| Uncommitted, Untracked | Paths reported by `git status --porcelain --untracked-files=all` |
| Ignored, not cache | Ignored paths that are not rebuildable caches (for example a local `.env` or a built extension), with the first names |
| Last activity | Most recent change in the git metadata, the top level and the build output |

It then lists the `cargo-mutants-*.tmp` and `pytest-of-*` directories of the current user in
the temporary directory, the total size of the worktrees and caches, and the size, use and free
space of every file system involved.

### Merged into main

- `yes` when the worktree commit is reachable from `origin/main`.
- Otherwise the GitHub REST API is asked for closed pull requests from the branch into `main`
  (the branch name is URL-encoded). A pull request counts as merged only when `merged_at` is
  set, its base is `main`, its head branch is in this repository (not a fork) and its head
  commit is exactly the worktree commit (squash merges). A merged pull request with an older
  head is reported as `no`, with commits after it.
- `unknown` when the answer cannot be established: GitHub unreachable or `--offline`, a
  malformed response, a detached HEAD outside `origin/main`, `origin/main` missing.

Before any git query inside a worktree, the report verifies that its git common directory is
the one of this repository; a directory that no longer belongs to it is reported with its size
only. Unknown values are printed as `unknown`, never guessed. Run `git fetch origin` first so
that `origin/main` knows about recent merges.

## Deleting worktrees and caches

Deletion is manual. The maintainer decides, or an agent deletes on the maintainer's explicit
request for named paths. Agents run the report, for example at the end of a session, and report
the figures; they delete nothing on their own.

Before deleting a worktree, check in the report that it is merged (`yes`), has no uncommitted,
untracked or ignored non-cache paths, and is not in use by a running agent. Then use
`git worktree remove <path>` without `--force`: git refuses a worktree with changes. Caches
marked "cache rebuildable" cost only a rebuild when deleted, but deleting the cache of a
worktree in use makes the next build or test run of that worktree start from scratch.
