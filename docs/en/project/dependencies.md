---
title: Dependency policy
description: Which dependencies Fairmark accepts, how they are updated, when, and how the project knows that an update is needed.
---

# Dependency policy

Fairmark computes valuations that people rely on, so a dependency is a liability as well as a
help. This page states what is accepted, how updates arrive, when they are applied and who
decides. Every tool used is free for public open-source repositories.

## What may be added

- **Core (`fairmark-core`)**: as few crates as possible; no network, file-system or
  async-runtime crates (`deny.toml` bans them). A new runtime dependency of the core requires
  an ADR that shows it meets every criterion below; dev-dependencies follow the Adapters and
  tooling rule and must not enable network, file-system or process features.
  - an established maintainer or organisation, with a record of maintaining crates the Rust
    ecosystem relies on;
  - no `unsafe` code, or `unsafe` code that the ADR reviews and accepts;
  - no build script;
  - no I/O and no network access, directly or through its dependencies;
  - a small, fixed scope: the crate does one thing, and the ADR states its size;
  - a permissive licence on the allow-list in `deny.toml`;
  - a release in the last twelve months, or one sentence in the ADR on why an older release
    is acceptable (for example a fixed scope that follows a crate the core already uses);
  - a fallback named in the ADR (vendoring, or our own code) for when the crate is abandoned
    or compromised.

  Upgrades of core runtime dependencies in the lock file are always reviewed and never merged
  automatically, whatever the size of the version change.
- **Adapters and tooling**: an actively maintained project with a release in the last twelve
  months, a permissive licence on the allow-list in `deny.toml` and in the *Security*
  workflow, and a reason that a few lines of our own code would not cover.
- **Optional features stay optional**: integrations such as MCP or LangChain are extras, never
  required by `pip install fairmark`.

## How the project knows an update exists

| Signal | Tool | Cadence | Cost |
| --- | --- | --- | --- |
| New versions of crates, Python packages, GitHub Actions and the container images of the Release workflow | Dependabot version updates, grouped per ecosystem | Weekly | Free |
| Known vulnerabilities in a dependency | Dependabot security alerts and updates; `cargo deny check advisories`; `pip-audit` | Immediate; also in CI and weekly | Free |
| A dependency added by a pull request with a vulnerability or a disallowed licence | Dependency review | Every pull request | Free (public repositories) |
| Vulnerable patterns in our code and workflows | CodeQL (Python, Rust, GitHub Actions) | Every push, pull request, weekly | Free (public repositories) |
| Supply-chain practices of the project | OpenSSF Scorecard | Weekly and on `main` | Free |
| Breakage coming with the next compatible versions | Weekly maintenance: tests with upgraded lock files | Weekly | Free |
| Unmaintained or yanked crates, duplicate versions | `cargo deny` | Every pull request | Free |
| Unsafe workflows: injection, excessive permissions, persisted credentials, unpinned actions | zizmor | Every pull request | Free |
| Workflow syntax, runner labels that no longer exist, shell mistakes | actionlint with shellcheck | Every pull request | Free |
| A YAML file that does not parse | `scripts/check_yaml.py` | Every pull request | Free |
| Rust dependencies declared but not used | cargo-machete | Every pull request | Free |
| Tests of the core that would not notice a wrong result | cargo-mutants 27.1.0 ([mutation testing](mutation-testing.md)) | Changed lines on every pull request; the whole core weekly | Free |

Locally: `uv run just deps-outdated` lists what can be updated, `uv run just deps-audit`
lists known advisories.

Cargo tools pinned in the workflows (`tool: <name>@<version>` of `taiki-e/install-action` and
the `cargo install --locked` fallback next to it: cargo-deny, cargo-llvm-cov, cargo-machete,
cargo-mutants) are not updated by Dependabot. `uv run just deps-outdated` compares each pin
with the latest release on crates.io; the maintainer runs it at each weekly maintenance and
updates both lines of a tool together, in one `ci(deps)` pull request.

## When to update

| Kind of update | When | How |
| --- | --- | --- |
| Security fix | Within seven days of the alert, or immediately for a critical one | Dependabot security pull request, or a manual `fix(deps)` |
| Patch and minor versions | Weekly, as one grouped pull request per ecosystem | Merge after a green CI |
| Major versions | Within one month, after reading the upstream changelog | One pull request per dependency; `pr-triage` runs `security-reviewer` |
| Rust toolchain for the MSRV | Only on purpose, with `rust-version` in `Cargo.toml` and the CI job together | `build(deps)` pull request with the reason |
| Python versions | Support follows the [Python release status](https://devguide.python.org/versions/): add a version once its first release is out, drop it at end of life | `build(py)` pull request |

A release is never made with an open security alert, a red weekly maintenance run or a
dependency pull request older than one month.

## GitHub Actions are pinned to commits

A tag such as `v7` can be moved to other code by whoever controls the action; a commit hash
cannot. Every `uses:` names a commit with the tag as a comment
(`actions/checkout@<commit> # v7`); Dependabot updates both when the comment is a version
tag. `dtolnay/rust-toolchain` is pinned to the commit of its toolchain branch and updated by
hand (see `.github/dependabot.yml`). After adding or changing an
action, run `python scripts/pin_actions.py`; CI refuses an unpinned action. A workflow that
needs a dangerous trigger or persisted credentials says why in an inline
`# zizmor: ignore[...]` comment, which is reviewed like code.

## Container images are pinned to digests

The images that build and smoke-test the release wheels are pinned by digest for the same
reason: a tag can be moved, a digest cannot. Each one is the `FROM` line of a file in
`.github/release/images/` (`<image>:<tag>@sha256:<digest>`), which the Release workflow reads
and refuses when it is not pinned. These files are never built. Dependabot (`docker`) updates
the digest weekly when the tag points to a new image; it does not change the Python versions
of the smoke images, which follow the versions that CI tests and are changed by hand. To add
an image, add a file there with the digest that `docker buildx imagetools inspect
<image>:<tag>` prints for the tag, and check that it includes the platform of the runner that
uses it.

## How to review a dependency pull request

1. CI is green, including *Dependency review* and *Dependencies* (`cargo deny`).
2. For a major version: read the upstream changelog for behaviour changes that could alter a
   number (decimal parsing, rounding, serialisation). If one might, run the golden cases and
   treat any difference as a `numerical-change`.
3. Merge with the title Dependabot proposes (`build(deps): ...` or `ci(deps): ...`), which
   follows the [conventions](conventions.md).
4. A pull request that conflicts or is behind `main`: comment `@dependabot rebase`.

## Supported Python versions

Fairmark supports Python from 3.12 to the newest stable release. The floor is
`requires-python` in `pyproject.toml`; the wheels are `abi3` builds for that floor
(`cp312-abi3`), so one wheel per platform serves every later version.

- **Raising the floor.** The floor is raised in a minor release when its Python version
  reaches end of life, or earlier by maintainer decision. The release notes announce it as a
  breaking change.
- **Adding a version.** Every October, when a new Python version is released, it is added to
  the test matrix and to the classifiers.
- **What is tested.** The CI matrix and the release smoke test run the floor and the newest
  stable version on Linux, macOS and Windows (`ci.yml`, `SMOKE_PYTHONS` in `release.yml`, and
  the Alpine images in `.github/release/images/`). Versions in between are covered by the
  `abi3` build and are not tested separately.

When the matrix changes, the names of the required checks change with it: update
`.github/required-checks.txt` and the ruleset in the same pull request.

## Lock files

`Cargo.lock` and `uv.lock` are committed: every build of a given commit uses the same versions.
The published Python package declares ranges, not exact pins; the weekly maintenance run tests
those ranges at their newest versions so that users installing later are not surprised.
