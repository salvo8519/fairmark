---
name: release
description: Prepare a Fairmark release — version bump, changelog, tag. Use when the maintainer asks to cut a release.
---

<!-- Generated from .agents/skills/release/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

1. Confirm `main` is green and `[Unreleased]` in CHANGELOG.md is complete; numerical
   changes are listed with method versions.
2. Bump `version` in the root `Cargo.toml` (`[workspace.package]` and the
   `fairmark-core` dependency). Python's version is derived from it by maturin.
3. `uv lock`, `uv run just check`.
4. Move `[Unreleased]` to `[x.y.z] — YYYY-MM-DD` and open a `chore(release): x.y.z` PR.
5. After merge, the maintainer tags `vX.Y.Z`; the Release workflow builds wheels for all
   platforms and publishes to PyPI (trusted publishing) and crates.io.
