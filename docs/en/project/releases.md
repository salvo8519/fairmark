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

## What users read between releases

`RELEASE_NOTES.md` and `CHANGELOG.md` are written in the repository at release time. The
documentation site renders both at every build, with the changes not yet released on top: the
release notes from the fragments in `changes/` (towncrier, draft), the changelog from the
commits since the last tag (git-cliff). Every user-facing pull request adds its fragment (CI
checks it), so the site is current after each merge.

## Steps

1. The maintainer decides to release and starts the release preparation, in one of two ways:
   - on GitHub, the **Prepare release** workflow (Actions, Run workflow, on `main`), with the
     version and, optionally, the issue of the release task;
   - locally, an agent runs the `release` skill.

   Both start with `release.py ready` and stop if it fails; the workflow writes the report in
   the job summary.
2. Both open a `chore(release)` pull request from the branch `release/vX.Y.Z`: version bump,
   release notes for analysts (`RELEASE_NOTES.md`, from `changes/`) and the technical
   changelog (`CHANGELOG.md`). The maintainer reviews the release notes in it: they are the
   text of the GitHub Release.
3. The maintainer merges it and creates the tag `vX.Y.Z`, or asks an agent to. The workflow
   never tags and never merges.
4. The Release workflow checks the version, builds the wheels for Linux, macOS and Windows,
   creates the GitHub Release with the release notes and publishes to PyPI and crates.io.

### Setting up the Prepare release workflow

- In the repository settings (Actions, General, Workflow permissions), allow GitHub Actions
  to create pull requests.
- A pull request opened with the default workflow token does not start the pull request
  workflows; its body says so, and closing and reopening it starts them. To have the checks
  start on their own, create a GitHub App with contents and pull requests write access,
  install it on the repository, and store its id in the variable `RELEASE_APP_ID` and its
  private key in the secret `RELEASE_APP_PRIVATE_KEY`. The workflow then uses a token limited
  to those two permissions.

The documentation site is republished at every merge on `main`; the **Docs** workflow can also
be run by hand on `main` to republish it without a commit.
