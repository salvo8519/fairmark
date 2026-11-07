---
name: fix-ci
description: Investigate and fix a failing CI run on the current branch. Use when a GitHub Actions check is red.
---

1. Delegate the diagnosis to the `ci-investigator` agent.
2. If the fix is in code: apply the minimal change, run the matching local recipe
   (`uv run just lint`, `test`, `check-generated` or `docs`), then `uv run just check`.
3. If the fix needs the maintainer (label, secret, repository setting): stop and tell them
   exactly what to do and why.
4. Commit with `fix(ci): …` or the scope of the actual fix, signed off.
