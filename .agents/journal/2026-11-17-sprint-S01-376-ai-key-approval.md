# 2026-11-17 — sprint/S01/376-ai-key-approval

- Issue: #376 AI workflow key behind an environment with approval per run
- Pull request: opens from this branch after the gate (sprint S01, process lane)
- Spec: none (CI configuration and tooling)

## Done
- `claude.yml`, `claude-review.yml`, `claude-docs.yml`: jobs in the environment `anthropic`
  (required reviewer: the maintainer). The review skips fork pull requests, which never get
  the key; the documentation sync builds the approved commit (`head.sha`), pushes with
  `HEAD:refs/heads/<branch>` and has a concurrency group.
- Claude Code hook `.claude/hooks/protect_settings.py` (PreToolUse, Bash): blocks run
  approvals and any API or `gh` command on environments, secrets, variables, rulesets and
  branch protection.
- `maintenance.yml`: weekly check of the rules of `anthropic` and `fairmark-app`.
- Tests: secrets read only inside their environment (case-insensitive names, `.yaml` files,
  workflow-level env; `secrets[...]`, `toJSON(secrets)` and `secrets: inherit` refused); the
  hook, 22 commands.
- Docs: maintainer guide (setup in order, the token agents must use, what to check before
  approving, side effects); Credentials and known gaps on the agentic development page (both
  languages).
- Security review: 2 blocking, 7 should fix, 6 nits; all fixed in this branch.

## State at the end of the session
- Nothing half done. The maintainer setup in maintainers.md is not done yet; until it is, the
  weekly maintenance check reports the environments as not protected.

## Decisions taken
- Maximum protection (the maintainer's choice): one approval per AI run.
- Environment `anthropic` without branch restriction: the approval is the protection.

## Open questions for the maintainer
- Use a fine-grained token without administration rights for local agent sessions.
- Not verified: whether the Claude GitHub App would exchange an OIDC token from a workflow
  added by a branch.

## Next steps
1. #328: generated mirrors of `.agents/` for other agent systems.
