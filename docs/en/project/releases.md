# Releases and versions

Fairmark has one version, `[workspace.package] version` in `Cargo.toml`, shared by the Rust
crates and the Python package. Versions follow Semantic Versioning.

## Policy

| Version | When |
| --- | --- |
| `0.1.0-alpha.N`, `-beta.N`, `-rc.N` | Whenever there is something worth trying; open work is reported, not required |
| `0.1.0` | Milestones M0 to M3 complete: the first public release (gate of M3) |
| `0.2.0`, `0.3.0` | M4 and M5 complete |
| `1.0.0` | M6 complete: stable API with a published deprecation policy |
| Patch `X.Y.Z+1` | Fixes between minor releases |

Each milestone in `roadmap/roadmap.yaml` names the release that ships it, and the
[roadmap](roadmap.md) shows it. Before 1.0.0 a minor version may change an input contract; every
such change is a new method version with its own schema file, so earlier contracts stay
documented.

## Readiness, checked

`python scripts/release.py ready <version>` decides whether a version can be released:

- the version is valid and newer than the last tag;
- every milestone whose release is not later than the version is complete in the roadmap (a
  pre-release lists the open tasks instead);
- there are release notes to publish in `changes/`;
- the roadmap, the translations, the schema snapshots and the version files are current;
- `main` is green, no open pull request carries `numerical-change`, and the roadmap agrees with
  GitHub.

## Steps

1. The maintainer decides to release. An agent runs the `release` skill, which starts with
   `release.py ready` and stops if it fails.
2. The skill opens a `chore(release)` pull request: version bump, release notes for analysts
   (`RELEASE_NOTES.md`, from `changes/`) and the technical changelog (`CHANGELOG.md`).
3. The maintainer merges it and creates the tag `vX.Y.Z`, or asks an agent to.
4. The Release workflow checks the version, builds the wheels for Linux, macOS and Windows,
   creates the GitHub Release with the release notes and publishes to PyPI and crates.io.
