---
name: release
description: Prepare a Fairmark release — version bump, changelog, tag. Use when the maintainer asks to cut a release.
---

1. Confirm `main` is green and that every numerical change since the last tag has its
   `Numerical-change:` commit trailer (`git log vPREV..HEAD --grep Numerical-change`).
2. Bump `version` in the root `Cargo.toml` (`[workspace.package]` and the
   `fairmark-core` dependency). Python's version is derived from it by maturin.
3. `uv lock`, `uv run just check`.
4. `uv run git-cliff --tag vX.Y.Z --output CHANGELOG.md`, then open a
   `chore(release): x.y.z` PR.
5. After merge, the maintainer tags `vX.Y.Z`; the Release workflow builds wheels for all
   platforms and publishes to PyPI (trusted publishing) and crates.io.
