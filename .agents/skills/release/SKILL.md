---
name: release
description: Prepare a Fairmark release — readiness, version bump, release notes, changelog and the release pull request; the maintainer creates the tag. Use when the maintainer asks to cut a release.
---

1. Run `uv run python scripts/release.py ready X.Y.Z[-alpha.N]` and stop if it fails: it
   checks the version policy (`docs/en/project/releases.md`), the milestones the version
   requires, release notes, roadmap, translations, schemas, `main` and open numerical changes.
   Then confirm that every numerical change since the last tag has its `Numerical-change:`
   trailer (`git log vPREV..HEAD --grep Numerical-change`).
2. `uv run python scripts/version.py bump <major|minor|patch|X.Y.Z[-rc.N]>`: the single
   source of the version; Python's version is derived from it.
3. `uv run towncrier build --version X.Y.Z --yes` (functional release notes from `changes/`),
   then review the text with the maintainer: it is what users read.
4. `uv run just check`.
5. `uv run git-cliff --tag vX.Y.Z --output CHANGELOG.md` (technical changelog), then open a
   `chore(release): x.y.z` PR.
6. After merge, the maintainer tags `vX.Y.Z` on the merge commit with their own credentials;
   the Release workflow checks the version, builds and smoke-tests the wheels for every
   platform and the sdist, attests their provenance, publishes to PyPI and then crates.io
   (trusted publishing, each after the maintainer approves its environment) and creates the
   GitHub Release with the release notes. Agents never create, move or delete a release tag and
   never approve a publishing environment: the maintainer does both. Before the first release,
   the maintainer completes the one-time setup and a dry run (Run workflow) described in
   `docs/en/project/releases.md`.
