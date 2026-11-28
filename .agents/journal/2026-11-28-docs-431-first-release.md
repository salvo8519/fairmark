# 2026-11-28 — docs/431-release-published

- Issue: #431 Documentation reflects the published 0.1.0-alpha.1
- Pull request: this branch (ready)
- Spec: none

## Done
- First public release 0.1.0-alpha.1 published: tag `v0.1.0-alpha.1` on `f49e4a6` (merge of #428),
  Release run #6 green, PyPI (7 wheels and the sdist, with provenance attestation), crates.io
  (`fairmark-core`, `fairmark-cli`) and a GitHub pre-release with the rewritten notes.
- Checked from a clean environment: `pip install fairmark` and `uv add fairmark` resolve to
  0.1.0a1 (44 methods); `cargo install fairmark-cli --version 0.1.0-alpha.1` builds; without
  `--version` cargo finds no version.
- Defects found by running the release chain, each fixed before the tag: macos-14 runner retired
  (#421, #422), `fairmark-mcp` crash without the extra (#423, #424), Prepare release without
  `issues: read` (#426, #427), schema stamping test tied to the committed index (#429, #430).
- This branch: README and installation guide describe the published alpha; the install sentence
  of the 0.1.0-alpha.1 release notes corrected (pip and uv install an alpha without a version
  while no stable version exists).

## State at the end of the session
- The GitHub Release body of `v0.1.0-alpha.1` still carries the earlier install sentence; it is
  updated from `RELEASE_NOTES.md` only if the maintainer agrees.

## Decisions taken
- License stays Apache-2.0: adoption first (maintainer, 2026-11-25).
- Release notes of a first release present what exists, not changes against unreleased code
  (maintainer, 2026-11-24).
- Agents never merge the release pull request nor create the tag (releases.md); the maintainer
  authorised merging ordinary fixes such as #430.

## Open questions for the maintainer
- Update the body of the GitHub Release `v0.1.0-alpha.1` with the corrected install sentence?

## Next steps
1. Maintainer: crates.io trusted publishing for `fairmark-core` and `fairmark-cli`, delete the
   `CARGO_REGISTRY_TOKEN` environment secret, revoke the token (releases.md, one-time setup 5).
2. Run the Release workflow by hand on `main` (dry run) to confirm the job uses trusted
   publishing without the secret (one-time setup 6).
3. Contracts before freezing: #266, #263, #352, #353.
