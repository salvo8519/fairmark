---
title: Naming and traceability conventions
description: How issues, branches, commits, pull requests, release notes and releases are named, and how they reference each other.
---

# Naming and traceability conventions

Every change in Fairmark can be traced in both directions: from a release back to the issue
that asked for it, and from an issue forward to the code, the tests and the release that
delivered it. The conventions below follow widely adopted public standards, so that any
contributor or tool recognises them without reading this page:
[Conventional Commits 1.0](https://www.conventionalcommits.org/en/v1.0.0/),
[GitHub closing keywords](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue),
[Semantic Versioning 2.0](https://semver.org/) and the
[Developer Certificate of Origin](https://developercertificate.org/).

They are enforced by `scripts/conventions.py`, locally (commit-msg hook) and in CI
(workflow *PR hygiene*). The script is the single source of the allowed types and scopes.

## The chain

| Level | Identifier | Example | References |
| --- | --- | --- | --- |
| Milestone | `M<n> · <name>` | `M1 · Complete DCF` | release gate |
| Epic | parent issue `#<n>` | `#9 DCF method family` | milestone |
| Task | sub-issue `#<n>` | `#42 Mid-year discounting convention` | parent epic, spec |
| Specification | `SPEC-<NNNN>`, requirement `R<k>` | `SPEC-0001 R3` | issue |
| Branch | `<type>/<issue>-<slug>` | `feat/42-mid-year-discounting` | issue |
| Commit | `<type>(<scope>): <subject>` | `feat(dcf): add mid-year discounting [SPEC-0001 R3]` | spec, issue (trailer) |
| Pull request | same format as a commit | `feat(dcf): add mid-year discounting` | `Closes #42` |
| Merge commit | pull request title and number | `feat(dcf): add mid-year discounting (#57)` | pull request |
| Release note | `changes/<issue>.<type>.md` | `changes/42.feature.md` | issue |
| Release | tag `v<semver>` | `v0.2.0` | CHANGELOG and release notes |

The GitHub number (`#42`) is the code of an issue. It is unique, permanent, and GitHub links it
automatically in commits, pull requests and comments; a second, parallel coding scheme would
drift from it. Pull request numbers share the same sequence.

## Issues

- **Title**: the outcome, as a short sentence in sentence case, without a type prefix and
  without a full stop. `Mid-year discounting convention for dcf.fcff`, not `[FEAT] mid year`.
- **Type**: chosen with the issue form, which applies the label (`bug`, `method-request`,
  `task`, `numerical`).
- **Area**: one `area:*` label, plus `method:*` for methodology work. Areas decide which
  issues can run in parallel.
- **Hierarchy**: every task is a sub-issue of an epic; every epic belongs to a milestone.
  The roadmap file `roadmap/roadmap.yaml` is the source of this structure.
- **Status**: the project board columns, plus the labels `in-progress` (claimed, a branch
  exists) and `blocked` (with a comment that says on what).

## Branches

`<type>/<issue>-<slug>`, lower case, words separated by hyphens, at most five words in the
slug: `fix/61-irr-sign-change`, `docs/48-n8n-guide`. The type is the Conventional Commit type
of the main change. One branch per issue; branches are short-lived and deleted after merge.
Exempt: `dependabot/*`, `release/v*`.

## Commits

```text
<type>(<scope>)[!]: <subject> [SPEC-NNNN Rk]

<body: what and why, wrapped at 72 characters>

Refs: #42
Numerical-change: dcf.fcff v1 -> v2
Docs-impact: none
Signed-off-by: Name Surname <email>
```

- **Type**: `feat` (new capability), `fix` (wrong behaviour), `perf`, `refactor` (no behaviour
  change), `docs`, `test`, `build` (packaging, dependencies), `ci`, `chore`, `revert`.
- **Scope**: required (except for `revert`), one of: engine `core`, one scope per theme module
  (`dates`, `rates`, `tvm`, `coc`, `dcf`, `tv`, `ratios`, `analysis`), `registry`, `explain`,
  `errors`, `schemas`; adapters `cli`, `py`, `tools`, `mcp`, `n8n`, `logging`; documentation
  and project `docs`, `i18n`, `brand`, `specs`, `adr`, `agents`, `project`; build and delivery
  `ci`, `deps`, `release`, `scripts`. New scopes are added in `scripts/conventions.py` with
  this page in the same pull request.
- **Subject**: imperative, lower case first word, no full stop, header at most 72 characters.
- **Breaking change**: `!` after the scope and a `BREAKING CHANGE:` paragraph in the body.
- **Trailers**: `Signed-off-by` always (`git commit -s`); `Refs: #<n>` to link the issue;
  `Numerical-change` and `Docs-impact` when they apply (see `AGENTS.md`).

## Pull requests

- **Title**: a Conventional Commit header that summarises the whole change. Pull requests are
  squash-merged, so the title becomes the commit on `main` and the line in `CHANGELOG.md`.
- **Body**: the template sections, and the linked issue with a closing keyword:
  `Closes #42` when the pull request completes the issue, `Refs #42` when it does not.
  Trivial changes without an issue (a typo) carry the label `no-issue`.
- **Branch and issue agree**: the issue number in the branch is the one linked in the body.
- **Size**: one issue, one concern. A change to `fairmark-core` and an unrelated workflow
  change are two pull requests.
- **Labels**: area labels are applied automatically from the changed paths; the author adds
  `numerical-change`, `breaking-change`, `no-release-note` or `translation-pending` when they
  apply.

## Authorship

Authorship is recorded by git and GitHub (commit author, `Signed-off-by`, `Co-authored-by`,
pull request author), and the GitHub release lists contributors. Names or initials are not
added to titles: they duplicate that record, they would appear in the CHANGELOG, and they are
not used by the projects this one is measured against.

## Versions and releases

One version for the whole project, in `Cargo.toml` (`scripts/version.py`). Tags are
`v<MAJOR>.<MINOR>.<PATCH>[-alpha.N|-beta.N|-rc.N]`. Before 1.0.0 a minor version may break
the API; every break is a `breaking-change` and is listed first in the release notes.
