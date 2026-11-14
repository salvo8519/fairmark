# 2026-11-14 — ci/211-unique-task-ids

- Issue: #211 Unique task ids in tasks.md files, checked in CI
- Pull request: #313 (replaced; a new one opens on this branch)
- Spec: none (tooling)

## Done
- Wrote scripts/check_task_ids.py: a task id is the checklist item `- [ ] T<n> ...`; mentions
  in the text of another task are references, not definitions. Checked all 10 tasks.md files:
  every id is defined once (9a64342).
- Wired the check into `just lint` and the quality job of CI (9a64342).
- Marked task 211 done in roadmap/roadmap.yaml in the same commit (9a64342).

## State at the end of the session
- All repository checks green locally: ruff, ruff format, mypy, typos, check_style,
  check_task_ids, roadmap check with the PR body, docs impact waived by Docs-impact: none,
  check-generated, docs coverage and claims, method status, version, check_yaml, pin_actions,
  cargo fmt. Rust tests and clippy not run locally (cargo was missing from that process PATH
  earlier; cargo fmt does pass now); CI will run them.
- The first attempt on branch copilot/salvo8519-make-task-ids-unique (PR #313) failed CI for
  two reasons: the script counted references as definitions, and the branch title did not
  follow the conventions. The branch of a pull request cannot be renamed: the work moved to
  ci/211-unique-task-ids. PR #313 still open, to close in favour of the new one.

## Decisions taken
- A definition is only a checklist item; the first implementation, which counted every match
  of T<number> on a line, reported eleven false duplicates (references such as "against T3")
  and failed CI on main files (decided by the orchestrator).
- No release note fragment: the change touches no user-facing source (the towncrier CI step
  only gates crates/*/src and python/fairmark).

## Open questions for the maintainer
- none

## Next steps
1. Push ci/211-unique-task-ids and open the pull request with the template body, Closes #211,
   labels area:infra and no-release-note; close PR #313 with a comment pointing to it.
