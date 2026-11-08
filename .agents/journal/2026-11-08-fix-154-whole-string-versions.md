# 2026-11-08 — fix/154-whole-string-versions

- Issue: #154 Release scripts validate versions as whole strings
- Pull request: opened at the end of this session
- Spec: none (tooling)

## Done
- 4aae115 `\Z` instead of `$` in the version patterns of `scripts/release.py`,
  `scripts/version.py` and `scripts/roadmap.py`; tests in `python/tests/test_release.py`
  (they fail on the previous patterns and pass now). Task #154 recorded in the roadmap as done.

## State at the end of the session
- `uv run just check` passes locally.

## Decisions taken
- Anchor with `\Z` rather than switching every call to `fullmatch`: one change per pattern
  covers every current and future caller.

## Open questions for the maintainer
- None.

## Next steps
1. None for this issue after merge.
