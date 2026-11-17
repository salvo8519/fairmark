# 2026-11-17 — sprint/S01/372-secrets-environments

- Issue: #372 Secrets in protected environments and branch updates through the GitHub App
- Pull request: opens from this branch after the gate (sprint S01, process lane)
- Spec: none (tooling and CI configuration)

## Done
- The project's GitHub App key moves to the environment `fairmark-app`, usable only from
  `main`. Repository variable `FAIRMARK_APP_ID`, environment secret `FAIRMARK_APP_PRIVATE_KEY`
  (the `RELEASE_APP_*` names are retired).
- `prepare-release.yml`: the job runs in `fairmark-app`; the release notes are written before
  the App token exists, and the token is created just before the push.
- `test-health.yml`: new job `update`, on `main` only, in `fairmark-app`, runs
  `pr_queue.py update --apply` with an App token (contents and pull requests write, checks
  read), with `pipefail` and a concurrency group. Skipped while the App is not set up.
- `pr_queue.py update`: a refused update is logged and the others continue (exit 1); pull
  requests into a branch other than `main` are skipped.
- `project-sync.yml`: `pull_request_target` without checkout, so `PROJECT_TOKEN` cannot be read
  from a branch.
- `.gitignore`: `.env`, `.env.*` (except `.env.example`), `.envrc`, `*.pem`.
- Test `test_workflow_environments.py`: only scheduled or manual jobs limited to `main` may
  name `fairmark-app`, and only they may read the App key.
- Docs: App setup, environment, rotation, secret scanning and the Workflows permission in
  releases.md; a Credentials section on the agentic development page (both languages), with
  the remaining exception written down.
- Security review: 2 blocking, 5 should fix, 5 nits; all fixed in this branch.

## State at the end of the session
- Nothing half done. The maintainer actions in releases.md are not done yet.

## Decisions taken
- The App id is a repository variable: a job reads its `if:` before it enters its
  environment, so an environment variable would leave the update job always skipped.
- The App gets no Workflows permission; updates that would need it are refused and reported.

## Open questions for the maintainer
- `ANTHROPIC_API_KEY` is still a repository secret, readable by a branch that edits the AI
  review workflows. Options: accept it with a key of a separate workspace and a spending limit
  (documented now), or move the review behind an environment with an approval per run.
- Do the maintainer steps: environment, App, variable and secret, removal of `RELEASE_APP_*`,
  secret scanning with push protection.

## Next steps
1. #328: generated mirrors of `.agents/` for other agent systems.
