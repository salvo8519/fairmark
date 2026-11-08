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
  an ADR; dev-dependencies follow the Adapters and tooling rule and must not enable network,
  file-system or process features.
- **Adapters and tooling**: an actively maintained project with a release in the last twelve
  months, a permissive licence on the allow-list in `deny.toml` and in the *Security*
  workflow, and a reason that a few lines of our own code would not cover.
- **Optional features stay optional**: integrations such as MCP or LangChain are extras, never
  required by `pip install fairmark`.

## How the project knows an update exists

| Signal | Tool | Cadence | Cost |
| --- | --- | --- | --- |
| New versions of crates, Python packages and GitHub Actions | Dependabot version updates, grouped per ecosystem | Weekly | Free |
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

Locally: `uv run just deps-outdated` lists what can be updated, `uv run just deps-audit`
lists known advisories.

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

## How to review a dependency pull request

1. CI is green, including *Dependency review* and *Dependencies* (`cargo deny`).
2. For a major version: read the upstream changelog for behaviour changes that could alter a
   number (decimal parsing, rounding, serialisation). If one might, run the golden cases and
   treat any difference as a `numerical-change`.
3. Merge with the title Dependabot proposes (`build(deps): ...` or `ci(deps): ...`), which
   follows the [conventions](conventions.md).
4. A pull request that conflicts or is behind `main`: comment `@dependabot rebase`.

## Lock files

`Cargo.lock` and `uv.lock` are committed: every build of a given commit uses the same versions.
The published Python package declares ranges, not exact pins; the weekly maintenance run tests
those ranges at their newest versions so that users installing later are not surprised.
