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
| Environments | `anthropic` for the AI workflows (below); `fairmark-app`, `pypi` and `crates-io` as described in [Releases](releases.md) |

### AI workflows: one approval per run

The AI review (`claude-review.yml`), the documentation sync (`claude-docs.yml`) and the
`@claude` mentions (`claude.yml`) use the key `ANTHROPIC_API_KEY`. It is a secret of the
environment `anthropic`, never a repository secret: GitHub gives repository secrets to every
pull request workflow of a branch of this repository, and a branch can add or edit a
workflow. In the environment, the key reaches a run only after a required reviewer approves
it. That holds under two conditions: the repository secret has been deleted (step 3), and no
agent holds a token that can approve a run or change the environment (step 4).

Required reviewers on environments are available on public repositories; a private
repository needs a paid plan.

Setup, once. Until it is complete the protection is not active: a run that names an
environment that does not exist creates it without any rule, and a job reads the repository
secret when the environment has none.

1. Repository settings, Environments, `anthropic` (New environment, or open it if a run has
   already created it):
   - Required reviewers: the maintainer;
   - "Prevent self-review": off, because the maintainer opens most pull requests and must be
     able to approve their runs;
   - "Allow administrators to bypass configured protection rules": off;
   - Deployment branches and tags: No restriction. The branch name is chosen by whoever
     pushes, so a branch rule cannot tell a trusted run from another; the approval is the
     protection.
2. In the environment, Environment secrets: `ANTHROPIC_API_KEY`. Use a key of a separate
   workspace of the Anthropic Console with a monthly spending limit.
3. Delete the repository secret `ANTHROPIC_API_KEY` (Secrets and variables, Actions). Check
   that `PROJECT_TOKEN` is then the only repository secret.
4. Agent sessions on your computer use a GitHub token that cannot approve runs or change
   settings: a fine-grained token limited to this repository, with Contents, Pull requests,
   Issues and Actions (read) and without Administration, Environments, Deployments, Secrets or
   Variables. The owner's own login can do all of that. A Claude Code hook
   (`.claude/hooks/protect_settings.py`) also blocks those commands, but it guards against a
   misled agent, not against a token that allows them.
5. The ruleset "Protect main" does not use "Require deployments to succeed" for `anthropic`.

The weekly maintenance workflow checks the rules of `anthropic` (a required reviewer,
self-review allowed, no administrator bypass) and of `fairmark-app`. Repository secrets cannot
be listed with its token: check step 3 by hand after any change.

What an approval gives. The run executes the workflow and the code as the branch wrote them,
with the key and the job's permissions (`contents: write`, `pull-requests: write`,
`id-token: write`). Before approving:

- every run: the pull request does not change `.github/workflows/` or `.github/actions/`;
- the documentation sync: every file of the branch is trusted, because the job installs and
  builds the project (`pyproject.toml`, `uv.lock`, `Cargo.toml`, `Cargo.lock`, `build.rs`,
  `justfile`, `scripts/`, the docs configuration) before the agent runs. It builds the commit
  that was approved, not later pushes;
- an `@claude` mention on a pull request: the agent reads the branch, so its content is input
  to an agent that holds the key and can push.

A waiting run shows as a pending check on the pull request; "Review deployments" is on the run
page. A run that is not approved blocks no merge (none of these jobs is a required check),
fails after 30 days, and shows as failed if it is rejected. A new push cancels a waiting review
run, so only the latest commit asks for approval. Pull requests from forks are not reviewed:
they never receive the key.

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
