# 2026-11-22 — ci/399-quality-surfaces (review fixes)

- Issue: #399 Quality problems surface and are fixed without the maintainer watching
- Pull request: #400, taken over from the product session to finish it
- Spec: none

## Done
- Windows: the hook writes UTF-8 whatever the locale; the rename test compares paths with
  forward slashes; a test with a cp1252 locale fails without the fix.
- Security review (no blocking findings), every finding fixed:
  - the hook never installs (`uv run --no-sync`) nor rewrites the lock (`clippy --locked`);
  - one deadline covers the probes too; the budget is finite and at most 75 s, under the 90 s
    hook timeout; a SIGTERM stops the running check with its process group;
  - subagents keep their own blocks (state key: session, hook event, agent id);
  - `failure_issue.py` ignores a run that a newer run of the same workflow contradicts (a slow
    failure never reopens after a newer green run, a slow success never closes after a newer
    failure); the title fallback never adopts an issue marked for another workflow; the report
    jobs get `actions: read` for that check;
  - CLAUDE.md and the hook docstring say that clippy runs build scripts and procedural macros.

## State at the end of the session
- Pushed; merge when CI is green.

## Next steps
1. none in this session.
