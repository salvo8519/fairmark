# 2026-11-08 — chore/159-roadmap-page-at-build

- Issue: #159 Roadmap page generated at build time instead of committed
- Pull request: opened at the end of this session
- Spec: none (tooling)

## Done
- d6949dd The roadmap page includes `.generated/ROADMAP.md`, written by `scripts/docs.py`
  before every build (`roadmap.py page`); `roadmap.py check` requires the include;
  conventions.md updated; tests in `python/tests/test_roadmap.py`. Task #159 recorded in
  the roadmap as done.

## State at the end of the session
- `uv run just check` passes; the built page shows the same tables.

## Decisions taken
- Same pattern as the changelog and the release notes (snippet include of a generated file),
  approved by the maintainer as a fix for the conflicts between parallel pull requests.
- `roadmap/roadmap.yaml` stays committed (it is the source); two pull requests that append a
  task at the same place of the same epic can still conflict there, but the conflict is two
  independent lines, resolved by keeping both.

## Open questions for the maintainer
- None.

## Next steps
1. After merge, open pull requests that change `docs/en/project/roadmap.md` take the page from
   `main` in their next rebase.
