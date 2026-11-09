---
title: Disk use and workspace cleaning
description: Where disk space goes in a Fairmark development environment, how debug builds are kept small, and how to remove merged worktrees and build caches without losing work.
---

# Disk use and workspace cleaning

A development machine for Fairmark fills up for three reasons: each checkout has its own Rust
`target/` directory (one to several GiB), contributors and agents work in several git
worktrees at once, and mutation testing and pytest leave temporary trees behind. None of these
holds project work, but deleting them by hand risks deleting something that does.

## Smaller debug builds

The workspace `Cargo.toml` sets `debug = "line-tables-only"` for the `dev` and `test`
profiles. Panics and backtraces still show the file and line of every frame; what is dropped is
the information a debugger needs to inspect variables. Measured on a clean
`cargo test --workspace --no-run` (issue #297):

| Profile setting | `target/` size | Build time (2 cores) |
| --- | --- | --- |
| `debug = true` (Cargo default) | 2.06 GB | 1 min 52 s |
| `debug = "line-tables-only"` | 1.36 GB | 1 min 28 s |

To step through code in a debugger, set `debug = "full"` locally in `[profile.dev]` and do not
commit it.

## Cleaning merged worktrees and build caches

`uv run just clean-workspace` runs `scripts/clean_workspace.py`. It is a **dry run by
default**: it prints a table of every worktree and cache with the action it would take, the
reason and the size, and the total it would free. Nothing is removed without `--apply`.

```bash
uv run just clean-workspace                   # dry run: merged worktrees
uv run just clean-workspace --caches          # dry run: also build caches
uv run just clean-workspace --apply           # remove merged worktrees
uv run just clean-workspace --apply --caches  # and build caches
df -h .                                        # verify the space freed
```

### When a worktree is removed

A git worktree is removed only when every condition holds; otherwise it is listed as kept,
with the reason.

| Condition | How it is checked |
| --- | --- |
| Not the main checkout, not locked | `git worktree list --porcelain` |
| No uncommitted or untracked changes | `git status --porcelain --untracked-files=all` is empty |
| Branch merged | The commit is reachable from `origin/main`, or GitHub reports a merged pull request whose head is exactly that commit (squash merges). If GitHub cannot be reached, the worktree is kept |
| No unpushed work | The commit is on `origin/main`, or it is the merged pull request head and is contained in `origin/<branch>` when that branch still exists |
| Not in use | No running process has its working directory or an open file inside it (`/proc`; where `/proc` is absent, the worktree is kept) |
| Idle | Nothing in its git metadata, top level or build output changed for `--idle-hours` (default 12) |

The idle condition exists because agent shells do not keep a working directory between two
commands: an agent waiting for CI has no process in its worktree. Removal uses
`git worktree remove` without `--force`, after checking every condition again, so git itself
refuses a worktree that changed in the meantime. Branches are never deleted, so their history
stays available. `git worktree prune` runs at the end.

### Build caches

With `--caches` the script also removes rebuildable directories, inside kept worktrees and in
the main checkout too: `target/`, `mutants.out*`, `.pytest_cache/` and every `__pycache__/`,
plus `cargo-mutants-*.tmp` and `pytest-of-*` in the system temporary directory (owned by the
current user). A cache is kept while a cargo, rustc, cargo-mutants or pytest process uses its
tree. Removing the cache of a worktree where an agent is working costs that agent a full
rebuild at its next command, not any work.

### What is never removed

Sources, `.git`, the agent journal (`.agents/journal/`), `specs/`, `docs/`, `uv.lock`, any
file tracked by git and any path outside the worktrees and the temporary patterns above. Every
path is checked against an allow-list of directory names, must lie inside its root, must not
contain tracked files and must not be or pass through a symbolic link.

The agent skill `workspace-clean` describes the same procedure for agents; `session-end` runs
the dry run.
