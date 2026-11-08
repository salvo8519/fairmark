---
title: Maintainer guide
description: The routine that keeps Fairmark consistent with one maintainer and AI agents — sessions, reviews, merges, dependencies and releases.
---

# Maintainer guide

Fairmark is maintained by one person working with AI agents. This page describes the routine
that keeps the project consistent and reviewable. The rules it relies on are in
[conventions](conventions.md), the [dependency policy](dependencies.md) and `AGENTS.md`.

## Principles

- **Machines check, people decide.** Anything that can be checked deterministically is a
  script and a CI job. Agents prepare, review and propose; only the maintainer merges, approves
  methodology and cuts releases.
- **Nothing lands on `main` without a pull request**, including the maintainer's own work.
- **Every session leaves a trace**: commits, a journal entry, a pull request.

## A working session

| Step | Command | Result |
| --- | --- | --- |
| Open | `/session-start` | Briefing: branch and issue, journal, open pull requests and CI, sprint, issues that can start in parallel |
| Choose | reply to the briefing | Branch `<type>/<issue>-<slug>`, issue labelled `in-progress` |
| Work | `/implement-spec`, `/run-sprint`, or by hand | Commits that follow the conventions, tests first |
| Close | `/session-end` | Gate passed, journal entry, release note, push, pull request with a complete description |

Without an agent, the same state is available with
`uv run python scripts/project_state.py`.

## Reviewing and merging

1. `/pr-triage` checks every open pull request, posts review comments that you approve
   first, and recommends a merge order.
2. Merge with **squash and merge** only. The pull request title becomes the commit on
   `main`, so it must be a valid Conventional Commit header (CI checks it).
3. Pull requests with `numerical-change`, `breaking-change` or `needs-maintainer` need your
   explicit decision, recorded as a review comment.

## Weekly (about thirty minutes)

- Merge the grouped Dependabot pull requests that are green; read the changelog of any major
  version ([dependency policy](dependencies.md)).
- Read the issue labelled `maintenance` if the weekly run opened one.
- Look at the Security tab: CodeQL alerts, Dependabot alerts, Scorecard.
- Triage new issues: type, area, milestone, `good first issue` where it fits.

## Per sprint and per release

- `/plan-sprint`, then `/run-sprint`.
- `/release` when the milestone gate is met: version bump, CHANGELOG, release notes, tag.
  The release workflow builds and publishes.

## Repository settings (one-time, in the GitHub web interface)

These cannot be set from the code and are listed here so that they are not forgotten.

| Setting | Value |
| --- | --- |
| General, Pull requests | Allow squash merging only; default commit message "Pull request title"; delete head branches automatically; suggest updating branches |
| Rules, Rulesets for `main` | Ruleset "Protect main", active, no bypass, target the default branch: restrict deletions; require linear history; require a pull request with 0 approvals and squash as the only merge method; require the status checks listed in `.github/required-checks.txt`, without requiring branches to be up to date; block force pushes |
| Advanced Security | Dependency graph (required by Dependency review), Dependabot alerts and security updates, secret scanning and push protection: all enabled. CodeQL runs from `security.yml`, so leave "default setup" off |
| Pages | Source: GitHub Actions |
| Actions, General | Workflow permissions: read repository contents; allow Actions to create pull requests only if needed |

### Required status checks

The ruleset requires checks by job name, and the names are listed in
`.github/required-checks.txt`. If a job is renamed, the ruleset keeps waiting for the old name
and every pull request is blocked. `scripts/required_checks.py check` (in the local gate and in
CI) fails when a listed name is no longer produced on pull requests, and the weekly maintenance
workflow compares the list with the ruleset on GitHub. When a job is renamed, change the list
in the same pull request and update the ruleset (Settings, Rules, Rulesets, Protect main) right
after merging it. Branches are not required to be up to date: with several pull requests open
in parallel, that setting would force a new rebase and a new run of every check after each
merge.

## Working on several projects

The conventions, checks and skills are plain files. To reuse them in another repository, copy
`scripts/conventions.py`, `.github/workflows/pr.yml`, `maintenance.yml`, `security.yml` and the
`session-*` and `pr-triage` skills, then adjust the scopes in `scripts/conventions.py`.
