# 2026-11-08 — docs/143-release-notes-rendering

- Issue: #143 Release notes and changelog on the site: undated draft, numerical changes name the method versions
- Pull request: opened at the end of this session
- Spec: none (documentation tooling)

## Done
- 25acc10 `scripts/docs.py`: the unreleased section of the release notes is titled
  "Fairmark Unreleased" without the build date. `cliff.toml`: entries under "Numerical
  changes" list their Numerical-change trailers. Tests in
  `python/tests/test_site_release_notes.py`. Task #143 recorded in the roadmap as done.

## State at the end of the session
- `uv run just check` passes locally.

## Decisions taken
- The method versions are appended to the changelog line ("Method versions: ...") rather
  than shown as sub-items, so that each change stays one line in CHANGELOG.md.
- The changelog test skips when git-cliff is not installed; the dev dependencies install it.

## Open questions for the maintainer
- None.

## Next steps
1. After merge, check the release notes and changelog pages of the site.
