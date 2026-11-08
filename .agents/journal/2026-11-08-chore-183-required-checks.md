# 2026-11-08 — chore/183-required-checks

- Issue: #183 Required status checks recorded and kept in step with the ruleset
- Pull request: opened at the end of this session
- Spec: none (tooling)

## Done
- The maintainer created the ruleset "Protect main" (active, no bypass, default branch,
  16 required checks, squash only, linear history, no force pushes or deletions); verified
  through the API.
- 3566cc9 `.github/required-checks.txt`, `scripts/required_checks.py` (check in `just lint`,
  remote in the weekly maintenance workflow), maintainer guide, tests. Task #183 recorded in
  the roadmap as done.

## State at the end of the session
- `uv run just check` passes; `required_checks.py remote` passes against the live ruleset.

## Decisions taken
- Branches are not required to be up to date: with parallel agents it forces a rebase and a
  full run of every check after each merge.
- Not required: the Claude review (depends on API credits), coverage (measures, does not
  verify), CodeQL (to be added through the code scanning rule later).

## Open questions for the maintainer
- None.

## Next steps
1. The first weekly maintenance run will show whether the workflow token can read rulesets;
   if not, the job fails and opens the maintenance issue, and needs a token with read access.
